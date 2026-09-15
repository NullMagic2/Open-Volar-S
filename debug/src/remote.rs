//! Bounded infrared learning sessions with raw evidence and no RF/EEPROM writes.
use crate::playback::Control;
use a865r::Device;
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

pub fn learn(seconds: u32, label: &str, folder: &Path, control: &Control) -> Value {
    let mut report = json!({"schema":1,"driver_version":env!("CARGO_PKG_VERSION"),
        "source":"tuner-infrared","label":label,"success":false,"events":[],
        "polls":0,"no_key_replies":0,"mapping_verified":false,
        "note":"Raw firmware codes only. NEC interpretations are candidates. No system keys are injected; no EEPROM or power settings are changed."});
    let result: Result<(), String> = (|| {
        if !(1..=120).contains(&seconds) {
            return Err("Learning duration must be 1–120 seconds".into());
        }
        std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
        let mut device = Device::new();
        let info = device.connect_and_probe().map_err(|e| e.to_string())?;
        report["device"] = json!({"chip":format!("{:04X}",info.chip_type),"firmware":info.firmware_version,
            "ir_mode":info.eeprom.as_ref().map(|e|e.summary.ir_mode),"ir_type":info.eeprom.as_ref().map(|e|e.summary.ir_type)});
        let mut ir = device.infrared().map_err(|e| e.to_string())?;
        let start = Instant::now();
        let mut polls = 0u32;
        let mut empty = 0u32;
        while start.elapsed() < Duration::from_secs(seconds.into())
            && !control.cancel.load(Ordering::Relaxed)
        {
            let sample = ir.poll().map_err(|e| e.to_string())?;
            polls += 1;
            report["polls"] = json!(polls);
            if let Some(code) = sample {
                report["events"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"elapsed_ms":start.elapsed().as_millis(),
                    "raw":code.raw,"key":code.key(),"nec_candidate":code.nec_candidate()}));
                control.status(format!(
                    "Remote button {label}: {} · {} seconds left",
                    code.key(),
                    seconds.saturating_sub(start.elapsed().as_secs() as u32)
                ));
            } else {
                empty += 1;
                report["no_key_replies"] = json!(empty);
                control.status(format!(
                    "Press {label} on your remote · {} seconds left · {empty} empty replies",
                    seconds.saturating_sub(start.elapsed().as_secs() as u32)
                ));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        report["cancelled"] = json!(control.cancel.load(Ordering::Relaxed));
        report["elapsed_ms"] = json!(start.elapsed().as_millis());
        Ok(())
    })();
    match result {
        Ok(()) => report["success"] = json!(true),
        Err(e) => report["error"] = json!(e),
    }
    report["received_codes"] = json!(report["events"].as_array().unwrap().len());
    if let Err(e) = std::fs::create_dir_all(folder).and_then(|_| {
        std::fs::write(
            folder.join("infrared.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
    }) {
        report["success"] = json!(false);
        report["export_error"] = json!(e.to_string());
    }
    report
}
