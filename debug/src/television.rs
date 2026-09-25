use crate::playback::{Control, Options};
use a865r::{Device, Error, FirmwareImage, Result};
use serde_json::{json, Value};
use std::{fs, path::PathBuf, sync::atomic::Ordering};

pub enum Action {
    Scan(Vec<u32>),
    Discover(Vec<u32>),
    Record { frequency: u32, seconds: u32 },
    Watch { frequency: u32 },
    RecordUntilStopped { frequency: u32, path:PathBuf },
}

pub fn discover_ts(data: &[u8], frequency: u32) -> Vec<Value> {
    let mut analyzer = a865r::ts::TsAnalyzer::new();
    for chunk in data.chunks(188 * 512) {
        analyzer.push(chunk);
    }
    let stats = analyzer.finish();
    stats.programs.keys().filter_map(|id|{
        let video=stats.streams.iter().find(|s|s.program_number==*id&&matches!(s.stream_type,0x1b|0x02|0x24))?;
        let audio=stats.streams.iter().find(|s|s.program_number==*id&&matches!(s.stream_type,0x0f|0x11|0x03|0x04));
        Some(json!({"frequency_khz":frequency,"program_id":id,"channel_number":stats.channel_numbers.get(id),"name":stats.service_names.get(id).cloned().unwrap_or_else(||format!("TV service {id}")),"pcr_pid":stats.programs.get(id).and_then(|p|p.pcr_pid),"video_pid":video.pid,"video_type":video.stream_type,"audio_pid":audio.map(|s|s.pid),"audio_type":audio.map(|s|s.stream_type),"audio_tracks":stats.streams.iter().filter(|s|s.program_number==*id&&matches!(s.stream_type,0x0f|0x11|0x03|0x04)).map(|s|json!({"program_id":id,"pid":s.pid,"stream_type":s.stream_type,"language":stats.audio_languages.get(&s.pid)})).collect::<Vec<_>>()}))
    }).collect()
}

/// ISDB television services only. Generic audio/radio entries are deliberately omitted.
pub fn tv_services(metadata: &Value, frequency: u32) -> Vec<Value> {
    let Some(programs) = metadata["programs"].as_array() else {
        return vec![];
    };
    let mut seen = std::collections::HashSet::new();
    programs
        .iter()
        .filter_map(|p| {
            let id = u32::try_from(p["program_id"].as_u64()?).ok()?;
            if !p["streams"]
                .as_array()?
                .iter()
                .any(|s| s["codec_type"] == "video")
                || !seen.insert(id)
            {
                return None;
            }
            let name = p["tags"]["service_name"]
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| format!("TV service {id}"));
            Some(json!({"frequency_khz":frequency,"program_id":id,"name":name}))
        })
        .collect()
}

#[cfg(test)]
mod discovery_tests {
    use super::*;
    #[test]
    fn discovery_keeps_named_tv_and_excludes_radio_and_duplicate_services() {
        let p = json!({"programs":[
            {"program_id":1,"tags":{"service_name":"TV One"},"streams":[{"codec_type":"video"}]},
            {"program_id":2,"tags":{"service_name":"Radio"},"streams":[{"codec_type":"audio"}]},
            {"program_id":1,"streams":[{"codec_type":"video"}]},
            {"program_id":3,"streams":[{"codec_type":"video"}]}
        ]});
        let s = tv_services(&p, 521143);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0]["name"], "TV One");
        assert_eq!(s[1]["program_id"], 3);
        assert_eq!(s[1]["frequency_khz"], 521143);
    }
}

pub fn run(
    action: Action,
    firmware: PathBuf,
    folder: PathBuf,
    options: Options,
    control: Control,
) -> Value {
    if let Action::Watch{frequency}=&action {return crate::native_player::launch(options,None,Some(*frequency),folder,control);}
    let recording_path=match &action{Action::RecordUntilStopped{path,..}=>path.clone(),_=>folder.join("recording.ts")};
    let result = (|| -> Result<Value> {
        fs::create_dir_all(&folder)?;
        // Validate before taking ownership of hardware.
        match &action {
            Action::Scan(frequencies) | Action::Discover(frequencies) => {
                if frequencies.is_empty() || frequencies.len() > 256 {
                    return Err(Error::InvalidArgument(
                        "Scan list must contain 1..256 entries".into(),
                    ));
                }
                for &frequency in frequencies {
                    a865r::channel_plan::validate_frequency(frequency)?;
                }
            }
            Action::Record { frequency, seconds } => {
                a865r::channel_plan::validate_frequency(*frequency)?;
                if !(1..=3600).contains(seconds) {
                    return Err(Error::InvalidArgument(
                        "Recording must be 1..3600 seconds".into(),
                    ));
                }
            }
            Action::Watch { frequency } | Action::RecordUntilStopped { frequency,.. } => a865r::channel_plan::validate_frequency(*frequency)?,
        }
        control.status("Connecting to the original A865R tuner…");
        let mut device = Device::new();
        let info = device.connect_and_probe()?.clone();
        if !info.firmware_running {
            control.status("Loading firmware into RAM…");
            let open = a865r::execution_probe_image()?;
            let data = if firmware.as_os_str().is_empty() {
                open.bytes().to_vec()
            } else {
                fs::read(&firmware)?
            };
            use sha2::{Digest, Sha256};
            if data != open.bytes()
                && format!("{:x}", Sha256::digest(&data)) != crate::REFERENCE_SHA
            {
                return Err(Error::Unsupported("Select the tested open firmware or your matching reference firmware; leave the path empty to use built-in open firmware.".into()));
            }
            device.load_firmware(&FirmwareImage::from_scatter_bytes(data)?)?;
        }
        if control.cancel.load(Ordering::Relaxed) {
            return Ok(json!({"stopped":true}));
        }
        let mut capability = capabilities_for(Some(device.info()?));
        capability["firmware"]["ofdm"] =
            json!(device.query_processor_firmware_version(0x80, 1000)?);
        control.set_device_info(capability.clone());
        fs::write(
            folder.join("device-capabilities.json"),
            serde_json::to_vec_pretty(&capability).map_err(std::io::Error::other)?,
        )?;
        let mut receiver = device.receiver()?;
        control.status("Initializing tuner and calibrating…");
        receiver.initialize()?;
        let discover = matches!(&action, Action::Discover(_));
        let result = (|| -> Result<Value> {
            match action {
                Action::Scan(frequencies) | Action::Discover(frequencies) => {
                    let mut stations = Vec::new();
                    for (index, frequency) in frequencies.iter().copied().enumerate() {
                        if control.cancel.load(Ordering::Relaxed) {
                            break;
                        }
                        control.status(format!(
                            "Checking frequency {} / {} · {:.3} MHz",
                            index + 1,
                            frequencies.len(),
                            frequency as f64 / 1000.0
                        ));
                        let tune = receiver.tune(frequency, 6000)?;
                        let mut station = json!({"frequency_khz":frequency,"locked":tune.mpeg_locked,"channel_found":tune.channel_found,"elapsed_ms":tune.elapsed_ms});
                        if discover && tune.mpeg_locked && !control.cancel.load(Ordering::Relaxed) {
                            let service_folder = folder.join(format!("rf-{frequency}"));
                            fs::create_dir_all(&service_folder)?;
                            let sample = service_folder.join("discovery.ts");
                            receiver.record(&sample, 3, &control.cancel)?;
                            station["services"] =
                                json!(discover_ts(&fs::read(&sample)?, frequency));
                            let _ = fs::remove_file(sample);
                        }
                        stations.push(station);
                        let services: Vec<Value> = stations
                            .iter()
                            .flat_map(|s| s["services"].as_array().cloned().unwrap_or_default())
                            .collect();
                        control.set_scan_services(json!(services));
                        control.status(format!(
                            "Checked {} / {} frequencies • {} TV channels found",
                            index + 1,
                            frequencies.len(),
                            services.len()
                        ));
                        fs::write(
                            folder.join("scan.json"),
                            serde_json::to_vec_pretty(&stations).map_err(std::io::Error::other)?,
                        )?;
                    }
                    Ok(
                        json!({"success":true,"stations":stations,"stopped":control.cancel.load(Ordering::Relaxed)}),
                    )
                }
                action => {
                    let continuous=matches!(action,Action::RecordUntilStopped{..});
                    let (frequency, seconds, watching) = match action {
                        Action::Record { frequency, seconds } => (frequency, seconds, false),
                        Action::Watch { frequency } => (frequency, 86400, true),
                        Action::RecordUntilStopped { frequency,.. } => (frequency, 0, false),
                        _ => unreachable!(),
                    };
                    control.status(format!("Tuning {:.3} MHz…", frequency as f64 / 1000.0));
                    let tune = receiver.tune(frequency, 6000)?;
                    if !tune.mpeg_locked {
                        return Err(Error::Protocol(
                            "No TV signal lock. Check the antenna or scan for another frequency."
                                .into(),
                        ));
                    }
                    if control.cancel.load(Ordering::Relaxed) {
                        return Ok(json!({"stopped":true}));
                    }
                    let playback:Option<Value> = None;
                    let capture = if let Some(program)=options.program_id {
                        control.status(format!("Recording selected TV service {program}"));
                        control.start_selected_recording(&recording_path,program)?;
                        let capture=if continuous {
                            receiver.stream_chunks_until_stopped(&control.cancel,|data|{control.record_chunk(data)?;Ok(())})
                        }else{
                            receiver.stream_chunks(seconds,&control.cancel,|data|{control.record_chunk(data)?;Ok(())})
                        };
                        let finished=control.stop_recording();
                        let capture=capture?;finished?;capture
                    } else if continuous {
                        control.status(format!("Recording {:.3} MHz",frequency as f64/1000.));
                        receiver.record_until_stopped(&recording_path,&control.cancel)?
                    } else {
                        control.status(format!(
                            "Recording {:.3} MHz · {seconds} seconds",
                            frequency as f64 / 1000.0
                        ));
                        receiver.record(&folder.join("recording.ts"), seconds, &control.cancel)?
                    };
                    Ok(
                        json!({"success":playback.as_ref().is_none_or(|p|p["success"]==true),"frequency_khz":frequency,"bytes":capture.bytes_written,"packets":capture.stats.packets,"continuity_errors":capture.stats.continuity_errors,"transport_errors":capture.stats.transport_error_packets,"playback":playback,"stopped":control.cancel.load(Ordering::Relaxed),"recording":if watching {None} else {Some(&recording_path)}}),
                    )
                }
            }
        })();
        let stop = receiver.stop();
        match (result, stop) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) => Err(error),
            (_, Err(error)) => Err(error),
        }
    })();
    control.stop();
    result.unwrap_or_else(|e| json!({"success":false,"error":e.to_string()}))
}

pub fn capabilities_json() -> Value {
    capabilities_for(None)
}

pub fn capabilities_for(info: Option<&a865r::DeviceInfo>) -> Value {
    let c = a865r::api::capabilities(info);
    json!({"api_version":c.api_version,"driver_version":c.driver_version,"model":c.model,"usb":{"vendor_id":0x07ca,"product_id":0xb865},
        "device_detected":info.is_some(),"chipset":c.chipset.as_ref().map(|s|json!({"name":"IT9175","id":s.chip_type,"revision":s.chip_revision,"prechip_revision":s.prechip_revision,"tuner_id":s.tuner_id})),
        "firmware":{"link":info.map(|i|i.firmware_version),"ofdm":null,"running":info.map(|i|i.firmware_running),"tested_open_version":[0,1,4,0]},
        "board_matches":c.detected_board_matches,"standard":c.standard,"bandwidth_khz":c.bandwidth_khz,
        "frequency_range_khz":[c.min_frequency_khz,c.max_frequency_khz],"manual_frequencies":true,"custom_scan":true,
        "native_resolution":null,"maximum_upscaled_resolution":if cfg!(windows) { json!(c.maximum_scaled_resolution) } else { Value::Null },"rendering_backends":if cfg!(windows) { json!(["auto","cpu","gpu"]) } else { json!([]) },
        "upscaling_toggle":cfg!(windows),"cpu_threads_range":if cfg!(windows) { json!([1,64]) } else { Value::Null },"color_profile_modes":if cfg!(windows) { json!(c.color_profile_modes) } else { json!([]) },
        "gpu_availability":if cfg!(windows) { "Measured when playback starts; use runtime status for the active backend" } else { "Linux native renderer is unavailable" },
        "reference_firmware_reception_verified":c.reference_firmware_reception_verified,"open_firmware_reception_verified":c.open_firmware_reception_verified})
}

pub fn query_device() -> Value {
    let mut device = Device::new();
    match device.connect_and_probe() {
        Ok(info) => {
            let mut report = capabilities_for(Some(&info));
            match device.query_processor_firmware_version(0x80, 1000) {
                Ok(version) => report["firmware"]["ofdm"] = json!(version),
                Err(error) => report["firmware"]["query_error"] = json!(error.to_string()),
            }
            report
        }
        Err(error) => {
            let mut report = capabilities_json();
            report["query_error"] = json!(error.to_string());
            report
        }
    }
}
