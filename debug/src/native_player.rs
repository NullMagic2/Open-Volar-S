//! Debug Desk reuses the native player, with an isolated profile and owned lifetime.
use crate::playback::{Control, Options};
use a865r::api::{ColorProfile, DeinterlaceMode, Resolution};
use serde_json::{json, Value};
use std::{
    fs,
    os::windows::{io::AsRawHandle, process::CommandExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::Ordering,
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::JobObjects::*,
};
struct Job(HANDLE);
impl Drop for Job {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}
fn find_player(exe: &Path) -> Option<PathBuf> {
    let dir = exe.parent()?;
    [dir.join("live-tv.exe"), dir.join("../player/live-tv.exe")]
        .into_iter()
        .find(|p| p.is_file())
}
fn settings(options: &Options, frequency: Option<u32>) -> Value {
    json!({"frequency":frequency.unwrap_or(473143),"resolution":Resolution::ALL.iter().position(|r|*r==options.resolution).unwrap_or(0),"gpu":options.gpu,"smooth":match options.deinterlacing{DeinterlaceMode::DoubleRate=>0,DeinterlaceMode::SingleRate=>1,DeinterlaceMode::Off=>2},"profile":match &options.color_profile{ColorProfile::Monitor=>json!("monitor"),ColorProfile::Disabled=>json!("off"),ColorProfile::File(p)=>json!(p)},"ffmpeg":options.ffmpeg,"volume":75,"captions_enabled":true})
}
pub fn launch(
    options: Options,
    file: Option<PathBuf>,
    frequency: Option<u32>,
    folder: PathBuf,
    control: Control,
) -> Value {
    let result = (|| -> Result<Value, String> {
        if control.cancel.load(Ordering::Relaxed) {
            return Ok(json!({"success":true,"stopped":true}));
        }
        let exe = find_player(&std::env::current_exe().map_err(|e| e.to_string())?).ok_or(
            "Native Live TV player is missing. Install the complete Open Volar S package.",
        )?;
        let profile = folder.join("native-player");
        fs::create_dir_all(&profile).map_err(|e| e.to_string())?;
        let stop = profile.join("stop.request");
        let report = profile.join("diagnostic-result.json");
        for p in [&stop, &report] {
            match fs::remove_file(p) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.to_string()),
            }
        }
        fs::write(
            profile.join("settings.json"),
            settings(&options, frequency).to_string(),
        )
        .map_err(|e| e.to_string())?;
        let mut command = Command::new(exe);
        command
            .args(["--diagnostic", "--profile-dir"])
            .arg(&profile)
            .arg("--diagnostic-control")
            .arg(&stop)
            .creation_flags(0x08000000)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Some(file) = file {
            command.arg("--play").arg(file);
        }
        if let Some(program) = options.program_id {
            command.arg("--diagnostic-program").arg(program.to_string());
        }
        if !options.gpu {
            command.arg("--software-decoder");
        }
        let job = unsafe {
            let h = CreateJobObjectW(None, None).map_err(|e| e.to_string())?;
            let j = Job(h);
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            SetInformationJobObject(
                h,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&limits) as u32,
            )
            .map_err(|e| e.to_string())?;
            j
        };
        let mut child = command.spawn().map_err(|e| e.to_string())?;
        if let Err(e) = unsafe { AssignProcessToJobObject(job.0, HANDLE(child.as_raw_handle())) } {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e.to_string());
        }
        control.status("Native diagnostic player running");
        let mut stopping = None;
        let mut refresh = Instant::now() - Duration::from_secs(1);
        loop {
            match child.try_wait() {
                Ok(Some(exit)) => {
                    if !exit.success() && !control.cancel.load(Ordering::Relaxed) {
                        return Err(format!("Native diagnostic player exited: {exit}"));
                    }
                    break;
                }
                Err(e) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(e.to_string());
                }
                _ => {}
            }
            if control.cancel.load(Ordering::Relaxed) {
                if stopping.is_none() {
                    let _ = fs::write(&stop, b"stop");
                    stopping = Some(Instant::now());
                }
                if stopping.is_some_and(|t| t.elapsed() > Duration::from_secs(5)) {
                    let _ = child.kill();
                    let _ = child.wait();
                    break;
                }
            }
            if refresh.elapsed() > Duration::from_secs(1) {
                let latest = fs::read_dir(profile.join("sessions"))
                    .ok()
                    .into_iter()
                    .flatten()
                    .filter_map(Result::ok)
                    .filter(|e| e.path().is_dir())
                    .max_by_key(|e| e.file_name());
                if let Some(latest) = latest {
                    let path = latest.path().join("native-vulkan.json");
                    if let Ok(bytes) = fs::read(path) {
                        if let Ok(v) = serde_json::from_slice(&bytes) {
                            control.set_native_diagnostic(v);
                        }
                    }
                }
                refresh = Instant::now();
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let mut value = fs::read(report)
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .unwrap_or_else(|| json!({"success":true}));
        value["presentation"] = json!("Live TV native player");
        value["diagnostic_profile"] = json!(profile);
        value["stopped"] = json!(control.cancel.load(Ordering::Relaxed));
        Ok(value)
    })();
    control.stop();
    result.unwrap_or_else(|e| json!({"success":false,"error":e}))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profile_carries_source_size_and_audio_defaults() {
        let mut o = Options::default();
        o.resolution = Resolution::ALL[0];
        o.deinterlacing = DeinterlaceMode::Off;
        let s = settings(&o, Some(521143));
        assert_eq!(s["frequency"], 521143);
        assert_eq!(s["resolution"], 0);
        assert_eq!(s["smooth"], 2);
        assert_eq!(s["captions_enabled"], true);
        assert!(s.get("receiver_startup").is_none());
    }
}
