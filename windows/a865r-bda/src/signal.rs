//! Persistent processing for external BDA clients. No player window is involved.
use crate::backend::Status;
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    os::windows::{io::AsRawHandle, process::CommandExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::JobObjects::*,
};

pub const CONTROL: u16 = 363;
pub const LABELS: [&str; 4] = [
    "Original",
    "Improved",
    "2K improved (upscaled)",
    "4K improved (upscaled)",
];
const MODES: [&str; 4] = ["original", "improved", "2k", "4k"];
#[derive(Clone, Debug)]
pub struct Config {
    pub mode: usize,
    pub ffmpeg: PathBuf,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            mode: 0,
            ffmpeg: PathBuf::from(r"C:\Program Files\FFmpeg\bin\ffmpeg.exe"),
        }
    }
}
pub fn config_path() -> PathBuf {
    PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default())
        .join("A865R/Compatibility/signal.json")
}
pub fn load_at(path: &Path) -> Result<Config, String> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
        Err(e) => return Err(e.to_string()),
    };
    let v: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let mode = MODES
        .iter()
        .position(|m| Some(*m) == v["mode"].as_str())
        .ok_or("Unknown AverTV signal preset")?;
    let ffmpeg = PathBuf::from(
        v["ffmpeg"]
            .as_str()
            .ok_or("Missing AverTV processor path")?,
    );
    if mode > 0 && !ffmpeg.is_absolute() {
        return Err("AverTV processing requires an absolute FFmpeg path".into());
    }
    Ok(Config { mode, ffmpeg })
}
pub fn save_at(path: &Path, config: &Config) -> Result<(), String> {
    if config.mode >= MODES.len() {
        return Err("Unknown AverTV signal preset".into());
    }
    if config.mode > 0 && (!config.ffmpeg.is_absolute() || !config.ffmpeg.is_file()) {
        return Err(
            "Choose an installed FFmpeg executable before enabling AverTV processing".into(),
        );
    }
    fs::create_dir_all(path.parent().ok_or("Missing settings directory")?)
        .map_err(|e| e.to_string())?;
    let temp = path.with_extension(format!("{}.tmp", std::process::id()));
    fs::write(
        &temp,
        json!({"version":1,"mode":MODES[config.mode],"ffmpeg":config.ffmpeg}).to_string(),
    )
    .map_err(|e| e.to_string())?;
    // Atomic replacement, shared by both 32-bit and 64-bit adapters.
    use windows::{
        core::HSTRING,
        Win32::Storage::FileSystem::{
            MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        },
    };
    unsafe {
        MoveFileExW(
            &HSTRING::from(temp.to_string_lossy().as_ref()),
            &HSTRING::from(path.to_string_lossy().as_ref()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(|e| e.to_string())
}
fn filter(mode: usize) -> String {
    let mut s="bwdif=mode=send_frame:parity=auto:deint=interlaced,fps=fps=source_fps:round=near:eof_action=pass,eq=contrast=1.035:saturation=1.04,unsharp=5:5:0.35:5:5:0".to_owned();
    if let Some((w, h)) = match mode {
        2 => Some((2560, 1440)),
        3 => Some((3840, 2160)),
        _ => None,
    } {
        s+=&format!(",scale={w}:{h}:flags=lanczos:force_original_aspect_ratio=decrease:force_divisible_by=2,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,setsar=1");
    }
    s
}
fn args(config: &Config, stats: &a865r::ts::TsStats) -> Result<Vec<String>, String> {
    if config.mode >= MODES.len() {
        return Err("Unknown AverTV signal preset".into());
    }
    let mut a: Vec<String> = [
        "-hide_banner",
        "-nostdin",
        "-v",
        "warning",
        "-hwaccel",
        "none",
        "-threads",
        "4",
        "-copyts",
        "-probesize",
        "8388608",
        "-analyzeduration",
        "2000000",
        "-i",
        "pipe:0",
        "-copy_unknown",
    ]
    .map(str::to_owned)
    .into();
    let streams: Vec<_> = stats
        .streams
        .iter()
        .filter(|s| {
            matches!(
                s.stream_type,
                0x1b | 0x0f | 0x11 | 0x03 | 0x04 | 0x02 | 0x24
            )
        })
        .collect();
    if !streams.iter().any(|s| s.stream_type == 0x1b) {
        return Err("AverTV processing requires an H.264 television service".into());
    }
    if streams.iter().any(|s| matches!(s.stream_type, 0x02 | 0x24)) {
        return Err("This AverTV processing path currently supports H.264 broadcasts only".into());
    }
    let mut seen = BTreeSet::new();
    for (i, s) in streams.iter().enumerate() {
        if !seen.insert(s.pid) {
            return Err("Shared elementary-stream PIDs are not supported by this processor".into());
        }
        a.extend([
            "-map".into(),
            format!("0:i:{}", s.pid),
            "-streamid".into(),
            format!("{i}:{}", s.pid),
        ]);
    }
    let first_pmt = (0x100u16..0x1f00)
        .find(|base| (0..stats.programs.len() as u16).all(|i| !seen.contains(&(base + i))))
        .ok_or("No free PMT PID range")?;
    a.extend(["-mpegts_pmt_start_pid".into(), first_pmt.to_string()]);
    for p in stats.programs.values() {
        let mut spec = format!("program_num={}", p.program_number);
        for (i, s) in streams
            .iter()
            .enumerate()
            .filter(|(_, s)| s.program_number == p.program_number)
        {
            let _ = s;
            spec += &format!(":st={i}");
        }
        a.extend(["-program".into(), spec]);
    }
    a.extend(
        [
            "-c",
            "copy",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-tune",
            "zerolatency",
            "-crf",
            "18",
            "-pix_fmt",
            "yuv420p",
            "-threads:v",
            "4",
            "-filter_threads",
            "2",
            "-g",
            "30",
            "-keyint_min",
            "30",
            "-sc_threshold",
            "0",
            "-vf",
        ]
        .map(str::to_owned),
    );
    a.push(filter(config.mode));
    a.extend(
        [
            "-fps_mode:v",
            "passthrough",
            "-mpegts_copyts",
            "1",
            "-avoid_negative_ts",
            "disabled",
            "-mpegts_flags",
            "+resend_headers",
            "-max_interleave_delta",
            "500000",
            "-muxdelay",
            "0",
            "-flush_packets",
            "1",
            "-f",
            "mpegts",
            "pipe:1",
        ]
        .map(str::to_owned),
    );
    Ok(a)
}
// Framing is independent of pipe read boundaries. Never discard a partial TS packet.
#[derive(Default)]
struct Packets(Vec<u8>);
impl Packets {
    fn push(&mut self, data: &[u8]) -> Vec<[u8; 188]> {
        self.0.extend_from_slice(data);
        let mut result = Vec::new();
        let mut at = 0;
        while self.0.len() - at >= 188 {
            if self.0[at] != 0x47 {
                at += 1;
                continue;
            }
            if self.0.len() - at >= 376 && self.0[at + 188] != 0x47 {
                at += 1;
                continue;
            }
            result.push(self.0[at..at + 188].try_into().unwrap());
            at += 188;
        }
        self.0.drain(..at);
        result
    }
}
fn pid(p: &[u8; 188]) -> u16 {
    ((p[1] as u16 & 31) << 8) | p[2] as u16
}
fn crc(data: &[u8]) -> u32 {
    let mut c = 0xffffffff;
    for &b in data {
        c ^= (b as u32) << 24;
        for _ in 0..8 {
            c = if c & 0x80000000 != 0 {
                (c << 1) ^ 0x04c11db7
            } else {
                c << 1
            };
        }
    }
    c
}
#[derive(Default)]
struct Sections(BTreeMap<u16, Vec<u8>>);
impl Sections {
    fn push(&mut self, p: &[u8; 188]) -> Vec<(u16, Vec<u8>)> {
        let id = pid(p);
        if p[1] & 0x80 != 0 || p[3] & 0xc0 != 0 || p[3] & 0x10 == 0 {
            self.0.remove(&id);
            return vec![];
        }
        let mut at = 4;
        if p[3] & 0x20 != 0 {
            at += 1 + p[4] as usize;
        }
        if at >= 188 {
            return vec![];
        }
        let b = self.0.entry(id).or_default();
        let mut result = Vec::new();
        if p[1] & 0x40 != 0 {
            let pointer = p[at] as usize;
            at += 1;
            if at + pointer > 188 {
                b.clear();
                return result;
            }
            if !b.is_empty() {
                b.extend_from_slice(&p[at..at + pointer]);
                Self::take(id, b, &mut result);
            }
            b.clear();
            at += pointer;
        } else if b.is_empty() {
            return result;
        }
        b.extend_from_slice(&p[at..]);
        Self::take(id, b, &mut result);
        result
    }
    fn take(id: u16, b: &mut Vec<u8>, out: &mut Vec<(u16, Vec<u8>)>) {
        while b.len() >= 3 {
            if b[0] == 0xff {
                b.clear();
                break;
            }
            let n = 3 + (((b[1] as usize & 15) << 8) | b[2] as usize);
            if n > 4096 || n < 7 {
                b.clear();
                break;
            }
            if b.len() < n {
                break;
            }
            let section: Vec<_> = b.drain(..n).collect();
            if crc(&section) == 0 {
                out.push((id, section));
            }
        }
    }
}
fn packetize(id: u16, section: &[u8]) -> Vec<[u8; 188]> {
    let mut bytes = vec![0];
    bytes.extend_from_slice(section);
    bytes
        .chunks(184)
        .enumerate()
        .map(|(i, c)| {
            let mut p = [255; 188];
            p[0] = 0x47;
            p[1] = ((id >> 8) as u8 & 31) | if i == 0 { 64 } else { 0 };
            p[2] = id as u8;
            p[3] = 0x10;
            p[4..4 + c.len()].copy_from_slice(c);
            p
        })
        .collect()
}
/// Retain broadcast PAT/PMT descriptors, SDT/NIT/EIT and clock tables. The PCR PID
/// must describe the new mux, not the old broadcast mux. A/V PIDs stay unchanged.
struct Metadata {
    sections: Sections,
    framing: Packets,
    tables: BTreeMap<(u16, u8, u16, u8), Vec<u8>>,
    pmts: BTreeMap<u16, u16>,
    elementary: BTreeSet<u16>,
    passthrough: BTreeSet<u16>,
    pending: Vec<[u8; 188]>,
    overflow: bool,
}
impl Metadata {
    fn new(stats: &a865r::ts::TsStats) -> Self {
        Self {
            sections: Sections::default(),
            framing: Packets::default(),
            tables: BTreeMap::new(),
            pmts: stats
                .programs
                .values()
                .filter_map(|p| {
                    stats
                        .streams
                        .iter()
                        .find(|s| s.program_number == p.program_number && s.stream_type == 0x1b)
                        .map(|s| (p.pmt_pid, s.pid))
                })
                .collect(),
            elementary: stats
                .streams
                .iter()
                .filter(|s| matches!(s.stream_type, 0x1b | 0x0f | 0x11 | 0x03 | 0x04))
                .map(|s| s.pid)
                .collect(),
            passthrough: stats
                .streams
                .iter()
                .filter(|s| !matches!(s.stream_type, 0x1b | 0x0f | 0x11 | 0x03 | 0x04))
                .map(|s| s.pid)
                .collect(),
            pending: Vec::new(),
            overflow: false,
        }
    }
    fn push(&mut self, data: &[u8]) {
        for p in self.framing.push(data) {
            let id = pid(&p);
            if self.passthrough.contains(&id) {
                if self.pending.len() < 32768 {
                    self.pending.push(p);
                } else {
                    self.overflow = true;
                }
                continue;
            }
            if id >= 0x30 && !self.pmts.contains_key(&id) {
                continue;
            }
            for (id, mut s) in self.sections.push(&p) {
                if s.len() < 8 {
                    continue;
                }
                if s[0] == 2 {
                    let Some(&pcr) = self.pmts.get(&id) else {
                        continue;
                    };
                    if s.len() < 16 {
                        continue;
                    }
                    s[8] = 0xe0 | ((pcr >> 8) as u8 & 31);
                    s[9] = pcr as u8;
                    let n = s.len();
                    let c = crc(&s[..n - 4]);
                    s[n - 4..].copy_from_slice(&c.to_be_bytes());
                }
                let key = (id, s[0], u16::from_be_bytes([s[3], s[4]]), s[6]);
                if self.tables.len() < 1024 || self.tables.contains_key(&key) {
                    self.tables.insert(key, s);
                }
            }
        }
    }
    fn packets(&self) -> Vec<[u8; 188]> {
        self.tables
            .iter()
            .flat_map(|((id, _, _, _), s)| packetize(*id, s))
            .collect()
    }
}
struct Job(HANDLE);
impl Drop for Job {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}
fn send(tx: &SyncSender<Vec<u8>>, mut data: Vec<u8>, cancel: &AtomicBool) -> Result<(), String> {
    let start = Instant::now();
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err("Cancelled".into());
        }
        match tx.try_send(data) {
            Ok(()) => return Ok(()),
            Err(mpsc::TrySendError::Full(d)) => data = d,
            Err(_) => return Err("AverTV disconnected".into()),
        }
        if start.elapsed() > Duration::from_secs(5) {
            return Err("AverTV is not consuming processed video".into());
        }
        thread::sleep(Duration::from_millis(5));
    }
}
fn process_service(
    config: Config,
    input: Receiver<Vec<u8>>,
    output: SyncSender<Vec<u8>>,
    cancel: Arc<AtomicBool>,
    status: Arc<Status>,
    program: u16,
) -> Result<(), String> {
    let mut analyzer = a865r::TsAnalyzer::new();
    let mut initial = Vec::new();
    let start = Instant::now();
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        match input.recv_timeout(Duration::from_millis(100)) {
            Ok(d) => {
                analyzer.push(&d);
                initial.extend_from_slice(&d);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(_) => return Err("Broadcast ended before service discovery".into()),
        }
        let stats = analyzer.stats();
        if initial.len() >= 2 * 1024 * 1024
            && !stats.programs.is_empty()
            && stats.programs.values().all(|p| p.pcr_pid.is_some())
        {
            break;
        }
        if initial.len() > 8 * 1024 * 1024 || start.elapsed() > Duration::from_secs(8) {
            return Err("AverTV processor could not discover complete service tables".into());
        }
    }
    let mut service_stats = analyzer.stats().clone();
    service_stats.programs.retain(|id, _| *id == program);
    service_stats
        .streams
        .retain(|s| s.program_number == program);
    let args = args(&config, &service_stats)?;
    let metadata = Arc::new(Mutex::new(Metadata::new(&service_stats)));
    metadata.lock().unwrap().push(&initial);
    let logdir = std::env::var_os("A865R_COMPAT_LOG")
        .map(PathBuf::from)
        .unwrap_or_else(|| config_path().parent().unwrap().to_owned());
    fs::create_dir_all(&logdir).map_err(|e| e.to_string())?;
    let log =
        fs::File::create(logdir.join(format!("processor-{}-{program}.log", std::process::id())))
            .map_err(|e| e.to_string())?;
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
    let mut child = Command::new(&config.ffmpeg)
        .args(&args)
        .creation_flags(0x08000000)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(log)
        .spawn()
        .map_err(|e| format!("Cannot start AverTV processor: {e}"))?;
    if let Err(e) = unsafe { AssignProcessToJobObject(job.0, HANDLE(child.as_raw_handle())) } {
        let _ = child.kill();
        let _ = child.wait();
        return Err(e.to_string());
    }
    crate::trace(format!(
        "AverTV signal preset={} processor_pid={} player_required=false",
        MODES[config.mode],
        child.id()
    ));
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let mi = metadata.clone();
    let ci = cancel.clone();
    let co = cancel.clone();
    let (done_tx, done_rx) = mpsc::channel();
    let input_done = done_tx.clone();
    let writer = thread::spawn(move || {
        let result = (|| -> Result<(), String> {
            stdin.write_all(&initial).map_err(|e| e.to_string())?;
            while !ci.load(Ordering::Relaxed) {
                match input.recv_timeout(Duration::from_millis(100)) {
                    Ok(d) => {
                        mi.lock().unwrap().push(&d);
                        stdin.write_all(&d).map_err(|e| e.to_string())?;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(_) => break,
                }
            }
            Ok(())
        })();
        drop(stdin);
        let _ = input_done.send((false, result));
    });
    // Track every announced video service. Audio/data output alone is not success.
    let video_watch = Arc::new(Mutex::new(
        service_stats
            .streams
            .iter()
            .filter(|s| s.stream_type == 0x1b)
            .map(|s| (s.pid, (Instant::now(), false)))
            .collect::<BTreeMap<_, _>>(),
    ));
    let video_reader = video_watch.clone();
    let last_output = Arc::new(Mutex::new(Instant::now()));
    let last = last_output.clone();
    let reader = thread::spawn(move || {
        let result = (|| -> Result<(), String> {
            let mut framing = Packets::default();
            let mut buf = [0; 188 * 256];
            let mut counters = BTreeMap::<u16, u8>::new();
            let mut received = 0u64;
            let mut tables_at = Instant::now() - Duration::from_secs(1);
            loop {
                let n = stdout.read(&mut buf).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                received += n as u64;
                let mut packets = Vec::new();
                let mut m = metadata.lock().unwrap();
                if m.overflow {
                    return Err("AverTV metadata queue overflow; choose a lower preset".into());
                }
                packets.append(&mut m.pending);
                if tables_at.elapsed() > Duration::from_millis(100) {
                    packets.extend(m.packets());
                    tables_at = Instant::now();
                }
                packets.extend(
                    framing
                        .push(&buf[..n])
                        .into_iter()
                        .filter(|p| m.elementary.contains(&pid(p))),
                );
                drop(m);
                let mut bytes = Vec::with_capacity(packets.len() * 188);
                for mut p in packets {
                    if p[1] & 0x40 != 0 && p[3] & 0x10 != 0 {
                        if let Some(v) = video_reader.lock().unwrap().get_mut(&pid(&p)) {
                            *v = (Instant::now(), true);
                        }
                    }
                    let cc = counters.entry(pid(&p)).or_insert(0);
                    p[3] = (p[3] & 0xf0) | *cc;
                    if p[3] & 0x10 != 0 {
                        *cc = (*cc + 1) & 15;
                    }
                    bytes.extend_from_slice(&p);
                }
                if !bytes.is_empty() {
                    send(&output, bytes, &co)?;
                    *last.lock().unwrap() = Instant::now();
                }
            }
            if video_reader.lock().unwrap().values().any(|v| !v.1) {
                return Err("AverTV processor omitted a video service".into());
            }
            if received == 0 {
                return Err("AverTV processor produced no video; inspect processor log".into());
            }
            Ok(())
        })();
        let _ = done_tx.send((true, result));
    });
    let mut result = Ok(());
    loop {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        if status.dropped.load(Ordering::Relaxed) > 0 {
            result = Err("AverTV processing cannot keep up; choose a lower signal preset".into());
            break;
        }
        if video_watch
            .lock()
            .unwrap()
            .values()
            .any(|v| v.0.elapsed() > Duration::from_secs(20))
        {
            result = Err("AverTV video service stalled; choose a lower preset".into());
            break;
        }
        if last_output.lock().unwrap().elapsed() > Duration::from_secs(20) {
            result =
                Err("AverTV processor stalled; choose Original or a lower signal preset".into());
            break;
        }
        match done_rx.recv_timeout(Duration::from_millis(100)) {
            Ok((true, r)) => {
                result = r;
                break;
            }
            Ok((false, Err(e))) => {
                result = Err(e);
                break;
            }
            _ => {}
        }
    }
    let stopped = cancel.load(Ordering::Relaxed);
    if !stopped && result.is_ok() {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match child.try_wait() {
                Ok(Some(exit)) => {
                    if !exit.success() {
                        result = Err(format!("AverTV processor exited with {exit}"));
                    }
                    break;
                }
                Err(e) => {
                    result = Err(e.to_string());
                    break;
                }
                _ => {}
            }
            if Instant::now() >= deadline {
                result = Err("AverTV processor did not finish cleanly".into());
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
    cancel.store(true, Ordering::Relaxed);
    let _ = child.kill();
    let _ = child.wait();
    let _ = writer.join();
    let _ = reader.join();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fragmented_pipe_packets_are_lossless() {
        let mut p = [0xff; 188];
        p[0] = 0x47;
        p[3] = 0x10;
        let data = p.repeat(7);
        let mut f = Packets::default();
        let mut out = Vec::new();
        for chunk in data.chunks(131) {
            out.extend(f.push(chunk).into_iter().flatten());
        }
        assert_eq!(data, out);
    }
    #[test]
    fn preset_dimensions_and_cpu_processing() {
        assert!(!filter(1).contains("scale="));
        assert!(filter(2).contains("2560:1440"));
        assert!(filter(3).contains("3840:2160"));
        assert!(filter(3).contains("force_original_aspect_ratio=decrease"));
    }
    #[test]
    fn psi_roundtrip_with_pointer_and_fragmentation() {
        let mut s = vec![0; 400];
        s[0] = 2;
        s[1] = 0xb1;
        s[2] = 141;
        let c = crc(&s[..396]);
        s[396..].copy_from_slice(&c.to_be_bytes());
        let mut reader = Sections::default();
        let mut got = vec![];
        for p in packetize(256, &s) {
            got.extend(reader.push(&p));
        }
        assert_eq!(got, vec![(256, s)]);
    }
    #[test]
    fn missing_settings_is_original() {
        let p = std::env::temp_dir().join(format!("no-avertv-config-{}.json", std::process::id()));
        assert_eq!(load_at(&p).unwrap().mode, 0);
    }
}

/// Exercise the exact adapter pipe/mux path using a file, without USB or GPU access.
pub fn process_file(config: Config, source: &Path, destination: &Path) -> Result<(), String> {
    let mut file = fs::File::open(source).map_err(|e| e.to_string())?;
    let mut target = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(destination)
        .map_err(|e| e.to_string())?;
    if config.mode == 0 {
        std::io::copy(&mut file, &mut target).map_err(|e| e.to_string())?;
        return Ok(());
    }
    let cancel = Arc::new(AtomicBool::new(false));
    let c = cancel.clone();
    let (itx, irx) = mpsc::sync_channel(32);
    let (otx, orx) = mpsc::sync_channel::<Vec<u8>>(32);
    let feed = thread::spawn(move || -> Result<(), String> {
        let mut b = [0; 188 * 256];
        loop {
            let n = file.read(&mut b).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            send(&itx, b[..n].to_vec(), &c)?;
        }
        Ok(())
    });
    let drain = thread::spawn(move || -> Result<(), String> {
        while let Ok(b) = orx.recv() {
            target.write_all(&b).map_err(|e| e.to_string())?;
        }
        Ok(())
    });
    let result = process(
        config,
        irx,
        otx,
        cancel.clone(),
        Arc::new(Status::default()),
    );
    cancel.store(true, Ordering::Relaxed);
    let fed = feed.join().map_err(|_| "Input worker panicked")?;
    let written = drain.join().map_err(|_| "Output worker panicked")?;
    result.and(fed).and(written)
}

/// Restore ISDB descriptors stripped when a remuxer labels caption PES as bin_data.
/// A/V packets and their timestamps are copied byte-for-byte.
pub fn restore_caption_metadata(
    input: &Path,
    output: &Path,
    profiles: &BTreeMap<u16, u16>,
) -> Result<(), String> {
    if profiles.is_empty() {
        return Err("No caption descriptors requested".into());
    }
    use std::io::{Seek, SeekFrom};
    let mut source = fs::File::open(input).map_err(|e| e.to_string())?;
    let mut initial = vec![0; 2 * 1024 * 1024];
    let n = source.read(&mut initial).map_err(|e| e.to_string())?;
    initial.truncate(n);
    let mut a = a865r::TsAnalyzer::new();
    a.push(&initial);
    let pmts: BTreeSet<_> = a.stats().programs.values().map(|p| p.pmt_pid).collect();
    source.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let mut target = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    let mut framing = Packets::default();
    let mut sections = Sections::default();
    let mut counters = BTreeMap::<u16, u8>::new();
    let mut buffer = [0; 188 * 256];
    let mut restored = BTreeSet::new();
    loop {
        let n = source.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        for p in framing.push(&buffer[..n]) {
            let id = pid(&p);
            if !pmts.contains(&id) {
                target.write_all(&p).map_err(|e| e.to_string())?;
                continue;
            }
            for (_, mut s) in sections.push(&p) {
                if s[0] != 2 || s.len() < 16 {
                    continue;
                }
                let end = s.len() - 4;
                let mut at = 12 + (((s[10] as usize & 15) << 8) | s[11] as usize);
                if at > end {
                    return Err("Malformed recording PMT".into());
                }
                let mut body = s[..at].to_vec();
                while at + 5 <= end {
                    let len = ((s[at + 3] as usize & 15) << 8) | s[at + 4] as usize;
                    let next = at + 5 + len;
                    if next > end {
                        return Err("Malformed recording descriptors".into());
                    }
                    let stream_pid = ((s[at + 1] as u16 & 31) << 8) | s[at + 2] as u16;
                    if let Some(&profile) = profiles.get(&stream_pid) {
                        let desc = [
                            0x52,
                            1,
                            0x30,
                            0xfd,
                            3,
                            (profile >> 8) as u8,
                            profile as u8,
                            0x3d,
                        ];
                        body.extend_from_slice(&[6, s[at + 1], s[at + 2], 0xf0, desc.len() as u8]);
                        body.extend_from_slice(&desc);
                        restored.insert(stream_pid);
                    } else {
                        body.extend_from_slice(&s[at..next]);
                    }
                    at = next;
                }
                let len = body.len() + 4 - 3;
                if len > 1021 {
                    return Err("Caption PMT exceeds MPEG section limit".into());
                }
                body[1] = (body[1] & 0xf0) | ((len >> 8) as u8 & 15);
                body[2] = len as u8;
                let c = crc(&body);
                body.extend_from_slice(&c.to_be_bytes());
                s = body;
                for mut packet in packetize(id, &s) {
                    let cc = counters.entry(id).or_insert(0);
                    packet[3] = (packet[3] & 0xf0) | *cc;
                    *cc = (*cc + 1) & 15;
                    target.write_all(&packet).map_err(|e| e.to_string())?;
                }
            }
        }
    }
    if !profiles.keys().all(|p| restored.contains(p)) {
        return Err("Caption PID missing from completed recording".into());
    }
    Ok(())
}

#[cfg(test)]
mod transport_regressions {
    use super::*;
    #[test]
    fn mux_tables_never_collide_with_broadcast_elementary_pids() {
        let mut stats = a865r::ts::TsStats::default();
        for (program, pmt, pid) in [(1, 4112, 4113), (2, 8136, 4097)] {
            stats.programs.insert(
                program,
                a865r::ProgramInfo {
                    program_number: program,
                    pmt_pid: pmt,
                    pcr_pid: Some(pid),
                },
            );
            stats.streams.insert(a865r::StreamInfo {
                program_number: program,
                pid,
                stream_type: 0x1b,
            });
        }
        let a = args(
            &Config {
                mode: 3,
                ..Config::default()
            },
            &stats,
        )
        .unwrap();
        let value = |key: &str| a[a.iter().position(|s| s == key).unwrap() + 1].clone();
        let pmt: u16 = value("-mpegts_pmt_start_pid").parse().unwrap();
        assert!(![pmt, pmt + 1].contains(&4097));
        assert_eq!(value("-avoid_negative_ts"), "disabled");
        assert_eq!(value("-hwaccel"), "none");
        assert_eq!(value("-fps_mode:v"), "passthrough");
        assert!(filter(3).contains("fps=fps=source_fps"));
    }
    #[test]
    fn recording_caption_metadata_is_restored_without_changing_video_packets() {
        fn section(mut s: Vec<u8>) -> Vec<u8> {
            let len = s.len() + 1;
            s[1] = (s[1] & 0xf0) | ((len >> 8) as u8);
            s[2] = len as u8;
            let c = crc(&s);
            s.extend_from_slice(&c.to_be_bytes());
            s
        }
        let pat = section(vec![0, 0xb0, 0, 0, 1, 0xc1, 0, 0, 0, 1, 0xe1, 0]);
        let pmt = section(vec![
            2, 0xb0, 0, 0, 1, 0xc1, 0, 0, 0xf0, 0x11, 0xf0, 0, 0x1b, 0xf0, 0x11, 0xf0, 0, 6, 0xe1,
            0x17, 0xf0, 0,
        ]);
        let mut video = [0xff; 188];
        video[0] = 0x47;
        video[1] = 0x10;
        video[2] = 0x11;
        video[3] = 0x10;
        let bytes: Vec<_> = packetize(0, &pat)
            .into_iter()
            .chain(packetize(256, &pmt))
            .chain([video])
            .flatten()
            .collect();
        let dir =
            std::env::temp_dir().join(format!("a865r-caption-metadata-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let input = dir.join("source.ts");
        let output = dir.join("tagged.ts");
        let _ = fs::remove_file(&output);
        fs::write(&input, &bytes).unwrap();
        restore_caption_metadata(&input, &output, &BTreeMap::from([(279, 8)])).unwrap();
        let data = fs::read(&output).unwrap();
        let mut a = a865r::TsAnalyzer::new();
        a.push(&data);
        assert_eq!(a.stats().caption_profiles.get(&279), Some(&8));
        assert_eq!(a.stats().psi_crc_errors, 0);
        assert!(data.chunks_exact(188).any(|p| p == video));
        assert_eq!(fs::read(&input).unwrap(), bytes);
        let _ = fs::remove_file(input);
        let _ = fs::remove_file(output);
        let _ = fs::remove_dir(dir);
    }
}

/// Each service has its own clock domain. A single FFmpeg interleaver must not
/// compare HD and mobile timestamps (which can differ by many hours).
pub(crate) fn process(
    config: Config,
    input: Receiver<Vec<u8>>,
    output: SyncSender<Vec<u8>>,
    cancel: Arc<AtomicBool>,
    status: Arc<Status>,
) -> Result<(), String> {
    let mut analyzer = a865r::TsAnalyzer::new();
    let mut initial = Vec::new();
    let started = Instant::now();
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        match input.recv_timeout(Duration::from_millis(100)) {
            Ok(data) => {
                analyzer.push(&data);
                initial.extend_from_slice(&data);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(_) => return Err("Broadcast ended during service discovery".into()),
        }
        let stats = analyzer.stats();
        if initial.len() >= 2 * 1024 * 1024
            && !stats.programs.is_empty()
            && stats.programs.values().all(|p| p.pcr_pid.is_some())
        {
            break;
        }
        if initial.len() > 8 * 1024 * 1024 || started.elapsed() > Duration::from_secs(8) {
            return Err("Service discovery timed out".into());
        }
    }
    let programs: Vec<_> = analyzer.stats().programs.keys().copied().collect();
    if programs.len() > 4
        || programs.iter().any(|p| {
            !analyzer
                .stats()
                .streams
                .iter()
                .any(|s| s.program_number == *p && s.stream_type == 0x1b)
        })
    {
        return Err("Processed AverTV output requires one to four H.264 TV services".into());
    }
    let (merged_tx, merged_rx) = mpsc::sync_channel::<Vec<u8>>(32);
    let mut inputs = Vec::new();
    let mut workers = Vec::new();
    let mut stops = Vec::new();
    for program in programs {
        let (tx, rx) = mpsc::sync_channel(32);
        let stop = Arc::new(AtomicBool::new(false));
        inputs.push(tx);
        stops.push(stop.clone());
        let out = merged_tx.clone();
        let options = config.clone();
        let state = status.clone();
        workers.push(thread::spawn(move || {
            process_service(options, rx, out, stop, state, program)
        }));
    }
    drop(merged_tx);
    let c = cancel.clone();
    let feed = thread::spawn(move || -> Result<(), String> {
        for chunk in initial.chunks(188 * 256) {
            for tx in &inputs {
                send(tx, chunk.to_vec(), &c)?;
            }
        }
        while !c.load(Ordering::Relaxed) {
            match input.recv_timeout(Duration::from_millis(100)) {
                Ok(data) => {
                    for tx in &inputs {
                        send(tx, data.clone(), &c)?;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(_) => break,
            }
        }
        Ok(())
    });
    let mut result = Ok(());
    let mut counters = BTreeMap::<u16, u8>::new();
    loop {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        match merged_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(mut data) => {
                // PSI is repeated by each service worker; serialize complete table
                // sections and reassign counters at the final multiplex boundary.
                for p in data.chunks_exact_mut(188) {
                    let id = ((p[1] as u16 & 31) << 8) | p[2] as u16;
                    let cc = counters.entry(id).or_insert(0);
                    p[3] = (p[3] & 0xf0) | *cc;
                    if p[3] & 0x10 != 0 {
                        *cc = (*cc + 1) & 15;
                    }
                }
                if let Err(e) = send(&output, data, &cancel) {
                    result = Err(e);
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    let externally_stopped = cancel.load(Ordering::Relaxed);
    cancel.store(true, Ordering::Relaxed);
    if externally_stopped || result.is_err() {
        for stop in stops {
            stop.store(true, Ordering::Relaxed);
        }
    }
    drop(merged_rx);
    let fed = feed.join().map_err(|_| "AverTV input worker panicked")?;
    for worker in workers {
        let completed = worker
            .join()
            .map_err(|_| "AverTV service worker panicked")?;
        if result.is_ok() && !externally_stopped {
            result = completed;
        }
    }
    if !externally_stopped {
        result = result.and(fed);
    }
    result
}
