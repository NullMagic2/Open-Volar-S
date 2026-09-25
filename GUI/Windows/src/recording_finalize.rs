//! Final TS publication: selected service, CPU verification, software repair if needed.
use a865r_media::playback::Control;
use serde_json::{json, Value};
use std::{
    fs,
    os::windows::{io::AsRawHandle, process::CommandExt},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::JobObjects::*,
};
static FINISH_OWNER: std::sync::Mutex<()> = std::sync::Mutex::new(());
struct Job(HANDLE);
impl Drop for Job {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}
fn run(
    exe: &Path,
    args: &[String],
    log: &Path,
    limit: Duration,
    control: &Control,
) -> Result<bool, String> {
    let log = fs::File::create(log).map_err(|e| e.to_string())?;
    let job = unsafe {
        let h = CreateJobObjectW(None, None).map_err(|e| e.to_string())?;
        let job = Job(h);
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        SetInformationJobObject(
            h,
            JobObjectExtendedLimitInformation,
            (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            std::mem::size_of_val(&info) as u32,
        )
        .map_err(|e| e.to_string())?;
        job
    };
    let mut child = Command::new(exe)
        .args(args)
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(log)
        .spawn()
        .map_err(|e| format!("Cannot run software recording verification: {e}"))?;
    if let Err(e) = unsafe { AssignProcessToJobObject(job.0, HANDLE(child.as_raw_handle())) } {
        let _ = child.kill();
        let _ = child.wait();
        return Err(e.to_string());
    }
    let start = Instant::now();
    loop {
        if control
            .finalization_cancel
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Recording verification cancelled; broadcast capture preserved.".into());
        }
        match child.try_wait() {
            Ok(Some(s)) => return Ok(s.success()),
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e.to_string());
            }
            _ => {}
        }
        if start.elapsed() > limit {
            let _ = child.kill();
            let _ = child.wait();
            return Err(
                "Software recording verification timed out; broadcast capture preserved.".into(),
            );
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
fn strings(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}
fn verify(
    exe: &Path,
    input: &Path,
    folder: &Path,
    stage: &str,
    limit: Duration,
    control: &Control,
) -> Result<bool, String> {
    let progress = folder.join(format!("{stage}-progress.txt"));
    let mut args = strings(&[
        "-hide_banner",
        "-nostdin",
        "-v",
        "error",
        "-hwaccel",
        "none",
        "-threads",
        "2",
        "-xerror",
        "-err_detect",
        "explode",
        "-i",
    ]);
    args.push(input.to_string_lossy().into());
    args.extend(strings(&["-map", "0:v:0", "-map", "0:a?", "-progress"]));
    args.push(progress.to_string_lossy().into());
    args.extend(strings(&["-f", "null", "-"]));
    Ok(run(
        exe,
        &args,
        &folder.join(format!("{stage}.log")),
        limit,
        control,
    )? && fs::read_to_string(progress)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.strip_prefix("frame="))
        .any(|s| s.trim().parse::<u64>().unwrap_or(0) > 0))
}
pub fn finish(
    input: &Path,
    output: &Path,
    program: Option<u32>,
    exe: &Path,
    folder: &Path,
    control: &Control,
) -> Result<Value, String> {
    if control
        .finalization_cancel
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        return Err("Recording finalization cancelled; broadcast capture preserved.".into());
    }
    let _owner = FINISH_OWNER.lock().unwrap_or_else(|e| e.into_inner());
    if input == output || output.exists() {
        return Err("Recording output already exists; broadcast capture preserved.".into());
    }
    fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let mut sample = vec![0; 4 * 1024 * 1024];
    use std::io::Read;
    let n = fs::File::open(input)
        .and_then(|mut f| f.read(&mut sample))
        .map_err(|e| e.to_string())?;
    sample.truncate(n);
    let mut analyzer = a865r::TsAnalyzer::new();
    analyzer.push(&sample);
    let selected = program
        .or_else(|| {
            analyzer
                .stats()
                .streams
                .iter()
                .find(|s| matches!(s.stream_type, 0x1b | 0x02 | 0x24))
                .map(|s| s.program_number as u32)
        })
        .ok_or("No TV service found in recording; broadcast capture preserved.")?;
    let bytes = fs::metadata(input).map_err(|e| e.to_string())?.len();
    let limit = Duration::from_secs((300 + bytes / 500_000).min(86400));
    let candidate = output.with_extension("checking.ts");
    let repaired = output.with_extension("repairing.ts");
    if candidate.exists() || repaired.exists() {
        return Err(
            "Recording verification files already exist; broadcast capture preserved.".into(),
        );
    }
    control.status(crate::i18n::text("Checking recording…"));
    let mut args = strings(&["-hide_banner", "-nostdin", "-n", "-v", "warning", "-i"]);
    args.push(input.to_string_lossy().into());
    let streams: Vec<_> = analyzer
        .stats()
        .streams
        .iter()
        .filter(|s| s.program_number as u32 == selected)
        .collect();
    let video = streams
        .iter()
        .find(|s| matches!(s.stream_type, 0x1b | 0x02 | 0x24))
        .ok_or("No video in selected recording service")?;
    let mut pids = vec![video.pid];
    pids.extend(
        streams
            .iter()
            .filter(|s| matches!(s.stream_type, 0x0f | 0x11 | 0x03 | 0x04))
            .map(|s| s.pid),
    );
    let captions: std::collections::BTreeMap<u16, u16> = analyzer
        .stats()
        .caption_profiles
        .iter()
        .filter(|(pid, _)| streams.iter().any(|s| s.pid == **pid))
        .map(|(p, v)| (*p, *v))
        .collect();
    pids.extend(captions.keys().copied());
    for (i, pid) in pids.iter().enumerate() {
        args.extend([
            "-map".into(),
            format!("0:i:{pid}"),
            "-streamid".into(),
            format!("{i}:{pid}"),
        ]);
    }
    args.extend(strings(&[
        "-c",
        "copy",
        "-mpegts_flags",
        "+resend_headers+initial_discontinuity",
        "-muxdelay",
        "0",
        "-avoid_negative_ts",
        "make_zero",
    ]));
    args.push(candidate.to_string_lossy().into());
    if !run(exe, &args, &folder.join("remux.log"), limit, control)? {
        return Err("Cannot prepare recording; broadcast capture preserved.".into());
    }
    let pristine = verify(exe, &candidate, folder, "check", limit, control)?;
    let mut publish = if pristine {
        candidate.clone()
    } else {
        control.status(crate::i18n::text("Repairing recording…"));
        let mut args = strings(&[
            "-hide_banner",
            "-nostdin",
            "-n",
            "-v",
            "warning",
            "-hwaccel",
            "none",
            "-threads",
            "2",
            "-i",
        ]);
        args.push(candidate.to_string_lossy().into());
        args.extend(strings(&[
            "-map",
            "0:v:0",
            "-map",
            "0:a?",
            "-map",
            "0:s?",
            "-map",
            "0:d?",
            "-vf",
            "bwdif=mode=send_field:parity=auto:deint=interlaced",
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
            "-crf",
            "20",
            "-pix_fmt",
            "yuv420p",
            "-threads",
            "2",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-c:s",
            "copy",
            "-c:d",
            "copy",
            "-mpegts_flags",
            "+resend_headers+initial_discontinuity",
            "-muxdelay",
            "0",
            "-avoid_negative_ts",
            "make_zero",
        ]));
        for (i, pid) in pids.iter().enumerate() {
            args.extend(["-streamid".into(), format!("{i}:{pid}")]);
        }
        args.push(repaired.to_string_lossy().into());
        if !run(exe, &args, &folder.join("repair.log"), limit, control)?
            || !verify(exe, &repaired, folder, "recheck", limit, control)?
        {
            return Err(
                "Recording repair did not pass verification; broadcast capture preserved.".into(),
            );
        }
        repaired.clone()
    };
    if !captions.is_empty() {
        let tagged = output.with_extension("captions.ts");
        a865r_bda::signal::restore_caption_metadata(&publish, &tagged, &captions)?;
        let _ = fs::remove_file(&publish);
        publish = tagged;
    }
    if control
        .finalization_cancel
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        return Err("Recording finalization cancelled; broadcast capture preserved.".into());
    }
    // Same-directory rename publishes only the completed, verified recording.
    fs::rename(&publish, output).map_err(|e| e.to_string())?;
    if !pristine {
        let _ = fs::remove_file(candidate);
    }
    let report = json!({"success":true,"recording":output,"broadcast_capture":input,"software_verified":true,"software_repaired":!pristine,"program_id":selected,"hardware_decoding":false});
    fs::write(folder.join("recording-validation.json"), report.to_string())
        .map_err(|e| e.to_string())?;
    control.status(crate::i18n::text("Recording ready."));
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn existing_output_is_never_replaced() {
        let dir =
            std::env::temp_dir().join(format!("a865r-existing-recording-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let output = dir.join("keep.ts");
        fs::write(&output, b"existing recording").unwrap();
        let result = finish(
            &dir.join("missing-input.ts"),
            &output,
            None,
            Path::new("missing-ffmpeg.exe"),
            &dir.join("logs"),
            &Control::default(),
        );
        assert!(result.unwrap_err().contains("already exists"));
        assert_eq!(fs::read(&output).unwrap(), b"existing recording");
        assert!(!dir.join("logs").exists());
        let _ = fs::remove_file(output);
        let _ = fs::remove_dir(dir);
    }
    #[test]
    fn closing_cancels_before_opening_files_or_starting_a_processor() {
        let control = Control::default();
        control
            .finalization_cancel
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let result = finish(
            Path::new("unopened-input.ts"),
            Path::new("unpublished-output.ts"),
            None,
            Path::new("missing-ffmpeg.exe"),
            Path::new("unused-logs"),
            &control,
        );
        assert!(result.unwrap_err().contains("cancelled"));
    }
}
