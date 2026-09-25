//! Native Live TV provides diagnostic playback; FFmpeg remains an optional probe tool.
//! The native diagnostic child has an isolated profile and a bounded stop deadline.
use serde_json::{json, Value};
use std::{
    fs::{self, File},
    io::{self},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

pub use a865r::api::{DeinterlaceMode, Resolution};

#[derive(Clone)]
pub struct Options {
    pub ffmpeg: PathBuf,
    pub resolution: Resolution,
    pub gpu: bool,
    pub threads: usize,
    pub program_id: Option<u32>,
    pub deinterlacing: DeinterlaceMode,
    pub color_profile: a865r::api::ColorProfile,
}
impl Default for Options {
    fn default() -> Self {
        let installed = PathBuf::from(r"C:\Program Files\FFmpeg\bin\ffmpeg.exe");
        Self {
            ffmpeg: if cfg!(target_os = "linux") {
                PathBuf::from("ffmpeg")
            } else if installed.is_file() {
                installed
            } else {
                PathBuf::from("ffmpeg.exe")
            },
            resolution: Resolution::Uhd,
            gpu: true,
            program_id: None,
            deinterlacing: DeinterlaceMode::DoubleRate,
            color_profile: a865r::api::ColorProfile::Monitor,
            threads: thread::available_parallelism()
                .map(|n| n.get().saturating_sub(1).clamp(1, 16))
                .unwrap_or(4),
        }
    }
}

impl Options {
    /// Convert the public driver API policy into an executable playback configuration.
    pub fn from_settings(
        ffmpeg: PathBuf,
        settings: &a865r::api::PlaybackSettings,
    ) -> a865r::Result<Self> {
        settings.validate()?;
        Ok(Self {
            ffmpeg,
            resolution: settings.effective_resolution(),
            gpu: settings.backend != a865r::api::RenderBackend::Cpu,
            threads: settings.cpu_threads,
            program_id: None,
            deinterlacing: settings.deinterlacing,
            color_profile: settings.color_profile.clone(),
        })
    }
}

#[derive(Clone)]
pub struct CaptionFrame {
    pub width:i32,pub height:i32,pub x:i32,pub y:i32,
    pub plane_width:i32,pub plane_height:i32,
    pub bgra:Arc<Vec<u8>>,
}
#[derive(Default)]
pub struct CaptionOverlay {
    pub revision:u64,
    pub frame:Option<CaptionFrame>,
}
#[derive(Clone, Default)]
pub struct Control {
    #[cfg(target_os="linux")]
    pub caption_stream:Arc<Mutex<crate::caption_stream::Stream>>,
    recording: Arc<Mutex<Option<Recording>>>,
    pub cancel: Arc<AtomicBool>,
    pub finalization_cancel: Arc<AtomicBool>,
    pub captions:Arc<Mutex<CaptionOverlay>>,
    pub captions_enabled:Arc<AtomicBool>,
    pub message: Arc<Mutex<String>>,
    children: Arc<Mutex<Vec<Child>>>,
    pub dropped: Arc<AtomicU64>,
    runtime: Arc<Mutex<Value>>,
    status_path: Arc<Mutex<Option<PathBuf>>>,
    presenter: Arc<Mutex<Option<(usize, String, u32)>>>,
}
struct Recording {
    file:File,
    service:Option<crate::service_stream::SelectedRecording>,
}
impl Control {
    pub fn start_recording(&self,path:&Path)->io::Result<()> {self.open_recording(path,None)}
    /// Both GUIs use this entry point. An explicit service is never replaced by
    /// another program, even when a mobile feed appears first in the multiplex.
    pub fn start_selected_recording(&self,path:&Path,program:u32)->io::Result<()> {
        let service=crate::service_stream::SelectedRecording::new(program)?;
        self.open_recording(path,Some(service))
    }
    fn open_recording(&self,path:&Path,service:Option<crate::service_stream::SelectedRecording>)->io::Result<()> {
        let mut recording=self.recording.lock().unwrap();
        if recording.is_some(){return Err(io::Error::new(io::ErrorKind::AlreadyExists,"A recording is already running"));}
        let file=File::options().write(true).create_new(true).open(path)?;
        *recording=Some(Recording{file,service});Ok(())
    }
    pub fn stop_recording(&self)->io::Result<()> {
        if let Some(recording)=self.recording.lock().unwrap().take(){
            recording.file.sync_all()?;
            if recording.service.as_ref().is_some_and(|s|!s.ready()){
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof,"Recording ended before a complete picture from the selected service arrived"));
            }
        }
        Ok(())
    }
    pub fn record_chunk(&self,data:&[u8])->io::Result<()> {
        use std::io::Write;
        if let Some(recording)=self.recording.lock().unwrap().as_mut(){
            if let Some(service)=recording.service.as_mut(){recording.file.write_all(&service.push(data)?)?;}
            else{recording.file.write_all(data)?;}
        }
        Ok(())
    }

    pub fn set_shader_state(&self,value:Value){let mut s=self.runtime.lock().unwrap();if s.is_null(){*s=json!({});}s["shader_acceleration"]=value;}
    pub fn set_native_diagnostic(&self,v:Value){let mut s=self.runtime.lock().unwrap();if s.is_null(){*s=json!({});}s["native_player"]=v;} 
    pub fn set_audio_details(&self,value:Value){let mut s=self.runtime.lock().unwrap();if s.is_null(){*s=json!({});}s["audio_details"]=value;}
    pub fn set_caption_state(&self, value:Value){let mut s=self.runtime.lock().unwrap();if s.is_null(){*s=json!({});}s["captions"]=value;}
    pub fn set_caption_tracks(&self, value:Value){let mut s=self.runtime.lock().unwrap();if s.is_null(){*s=json!({});}s["caption_tracks"]=value;}
    pub fn caption_frame(&self,frame:Option<CaptionFrame>){let mut c=self.captions.lock().unwrap();if frame.is_some()||c.frame.is_some(){c.revision=c.revision.wrapping_add(1);c.frame=frame;}}

    pub fn set_audio_tracks(&self, tracks: Value) {
        let mut state=self.runtime.lock().unwrap();
        if state.is_null(){*state=json!({});}
        state["audio_tracks"]=tracks;
    }
    pub fn set_audio_output(&self, mode: &str, pid: Option<u16>) {
        let mut state=self.runtime.lock().unwrap();
        if state.is_null(){*state=json!({});}
        state["audio_output"]=json!({"mode":mode,"pid":pid});
    }
    pub fn set_epg(&self, events: Value) {
        let mut state = self.runtime.lock().unwrap();
        if state.is_null() {
            *state = json!({});
        }
        state["epg"] = events;
    }
    pub fn set_scan_services(&self, services: Value) {
        let mut s = self.runtime.lock().unwrap();
        if s.is_null() {
            *s = json!({});
        }
        s["scan_services"] = services;
    }
    pub fn set_service(&self, service: Value) {
        let mut state = self.runtime.lock().unwrap();
        if state.is_null() {
            *state = json!({});
        }
        state["service"] = service;
    }
    pub fn set_video_size(&self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        let mut state = self.runtime.lock().unwrap();
        if state.is_null() {
            *state = json!({});
        }
        state["video_size"] = json!([width, height]);
    }
    pub fn set_timeline(&self, position: f64, duration: f64, seekable: bool, paused: bool) {
        let mut state = self.runtime.lock().unwrap();
        if state.is_null() {
            *state = json!({});
        }
        state["timeline"] =
            json!({"position":position,"duration":duration,"seekable":seekable,"paused":paused});
    }
    /// A native child HWND owned by the caller, and a session-specific mpv IPC pipe.
    /// Keep the HWND alive until the playback worker has finished.
    pub fn attach_presenter(&self, window: usize, pipe: String, volume: u32) {
        *self.presenter.lock().unwrap() = Some((window, pipe, volume.min(100)));
    }
    pub fn set_signal(&self, quality: Option<u8>) {
        let mut runtime = self.runtime.lock().unwrap();
        if runtime.is_null() {
            *runtime = json!({});
        }
        runtime["signal_quality_percent"] = json!(quality);
    }
    /// Cached state only: safe while the receiver owns USB; never opens a second device handle.
    pub fn snapshot(&self) -> Value {
        self.runtime.lock().unwrap().clone()
    }
    pub fn set_device_info(&self, value: Value) {
        self.runtime.lock().unwrap()["device"] = value;
    }
    pub fn status(&self, message: impl Into<String>) {
        *self.message.lock().unwrap() = message.into();
    }
    pub fn stop(&self) {
        let _=self.stop_recording();
        self.cancel.store(true, Ordering::Relaxed);
        self.kill_children();
        let mut state = self.runtime.lock().unwrap();
        if !state.is_null() {
            state["running"] = json!(false);
            if let Some(path) = self.status_path.lock().unwrap().as_ref() {
                if let Ok(bytes) = serde_json::to_vec_pretty(&*state) {
                    let _ = fs::write(path, bytes);
                }
            }
        }
    }
    fn kill_children(&self) {
        for mut child in self.children.lock().unwrap().drain(..) {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

}

fn hidden(_command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        _command.creation_flags(0x08000000);
    }
}

pub fn filter(resolution: Resolution, gpu: bool) -> String {
    filter_with_deinterlacing(resolution, gpu, DeinterlaceMode::DoubleRate)
}

pub fn filter_with_deinterlacing(
    resolution: Resolution,
    gpu: bool,
    mode: DeinterlaceMode,
) -> String {
    let deint = if mode == DeinterlaceMode::Off {
        String::new()
    } else {
        format!(
            "{}=mode={}:parity=auto:deint=interlaced,",
            if gpu { "bwdif_vulkan" } else { "bwdif" },
            if mode == DeinterlaceMode::DoubleRate {
                "send_field"
            } else {
                "send_frame"
            }
        )
    };
    match (resolution.dimensions(),gpu) {
        (Some((w,h)),true)=>format!("{deint}libplacebo=w={w}:h={h}:force_original_aspect_ratio=decrease:force_divisible_by=2:normalize_sar=true:upscaler=ewa_lanczos:format=yuv420p,hwdownload,format=yuv420p,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,setsar=1"),
        (Some((w,h)),false)=>format!("{deint}scale=w='min({w},trunc({h}*dar/2)*2)':h='min({h},trunc({w}/dar/2)*2)':flags=lanczos,setsar=1,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,format=yuv420p"),
        (None,true)=>format!("{deint}libplacebo=format=yuv420p,hwdownload,format=yuv420p"),
        (None,false)=>format!("{deint}format=yuv420p"),
    }
}

/// Test actual device/filter execution, not just the build's advertised accelerator list.
pub fn probe(options: &Options, folder: &Path, control: &Control) -> io::Result<bool> {
    fs::create_dir_all(folder)?;
    if !options.gpu {
        return Ok(false);
    }
    control.status("Checking GPU decoding and scaling support…");
    let mut command = Command::new(&options.ffmpeg);
    command
        .args([
            "-hide_banner",
            "-loglevel",
            "verbose",
            "-nostdin",
            "-init_hw_device",
            "vulkan=gpu",
            "-filter_hw_device",
            "gpu",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=320x180:rate=1",
            "-vf",
        ])
        .arg(format!(
            "format=yuv420p,setfield=tff,hwupload,{}",
            filter_with_deinterlacing(options.resolution, true, options.deinterlacing)
        ))
        .args(["-frames:v", "1", "-f", "null", "-"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(File::create(folder.join("gpu-probe.log"))?);
    hidden(&mut command);
    let mut child = command.spawn()?;
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status.success());
        }
        if control.cancel.load(Ordering::Relaxed) || start.elapsed() > Duration::from_secs(10) {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(false);
        }
        thread::sleep(Duration::from_millis(50));
    }
}

/// Probe actual media metadata with a bounded child process. Largest video service wins;
/// audio is then selected from that same program, independent of PID/stream discovery order.
pub fn select_program(
    options: &Options,
    path: &Path,
    folder: &Path,
    control: &Control,
) -> io::Result<Option<u32>> {
    fs::create_dir_all(folder)?;
    let ffprobe = if options
        .ffmpeg
        .parent()
        .is_some_and(|p| !p.as_os_str().is_empty())
    {
        options.ffmpeg.with_file_name(if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" })
    } else {
        PathBuf::from(if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" })
    };
    let mut command = Command::new(ffprobe);
    command
        .args([
            "-v",
            "error",
            "-analyzeduration",
            "2000000",
            "-probesize",
            "4000000",
            "-show_programs",
            "-show_streams",
            "-of",
            "json",
        ])
        .arg(path)
        .stdin(Stdio::null())
        .stdout(File::create(folder.join("source-media.json"))?)
        .stderr(File::create(folder.join("source-probe.log"))?);
    hidden(&mut command);
    let mut child = command.spawn()?;
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            if !status.success() {
                return Ok(None);
            }
            break;
        }
        if control.cancel.load(Ordering::Relaxed) || start.elapsed() > Duration::from_secs(5) {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(None);
        }
        thread::sleep(Duration::from_millis(25));
    }
    let metadata: Value = serde_json::from_slice(&fs::read(folder.join("source-media.json"))?)?;
    Ok(choose_program(&metadata))
}
/// Source dimensions and frame rate measured by FFprobe. Unknown values remain null.
pub fn media_properties(folder: &Path) -> io::Result<Value> {
    let metadata: Value = serde_json::from_slice(&fs::read(folder.join("source-media.json"))?)?;
    let program = choose_program(&metadata);
    let streams = program
        .and_then(|id| {
            metadata["programs"]
                .as_array()?
                .iter()
                .find(|p| p["program_id"].as_u64() == Some(id as u64))
                .and_then(|p| p["streams"].as_array())
        })
        .or_else(|| metadata["streams"].as_array());
    let video = streams.and_then(|streams| {
        streams
            .iter()
            .filter(|s| s["codec_type"] == "video")
            .max_by_key(|s| s["width"].as_u64().unwrap_or(0) * s["height"].as_u64().unwrap_or(0))
    });
    Ok(
        json!({"program_id":program,"width":video.and_then(|s|s["width"].as_u64()),"height":video.and_then(|s|s["height"].as_u64()),"frame_rate":video.and_then(|s|s["r_frame_rate"].as_str()),"field_order":video.and_then(|s|s["field_order"].as_str())}),
    )
}

fn choose_program(metadata: &Value) -> Option<u32> {
    metadata
        .get("programs")?
        .as_array()?
        .iter()
        .filter_map(|program| {
            let pixels = program["streams"]
                .as_array()?
                .iter()
                .filter(|s| s["codec_type"] == "video")
                .filter_map(|s| Some(s["width"].as_u64()?.checked_mul(s["height"].as_u64()?)?))
                .max()?;
            Some((pixels, u32::try_from(program["program_id"].as_u64()?).ok()?))
        })
        .max_by_key(|&(pixels, _)| pixels)
        .map(|(_, id)| id)
}

pub fn play_file(options:Options,path:PathBuf,folder:PathBuf,control:Control)->Value {
    crate::native_player::launch(options,Some(path),None,folder,control)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_dimensions_are_actual_scaling_not_window_hints() {
        assert!(filter(Resolution::Uhd, true).contains("libplacebo=w=3840:h=2160"));
        let cpu = filter(Resolution::Uhd, false);
        assert!(
            cpu.contains("scale=") && cpu.contains("pad=3840:2160") && !cpu.contains("hwupload")
        );
        assert!(!filter(Resolution::Native, false).contains("scale="));
    }
    #[test]
    fn service_selection_ignores_one_seg_discovery_order() {
        let metadata = json!({"programs":[{"program_id":9,"streams":[{"codec_type":"video","width":320,"height":180}]},{"program_id":123,"streams":[{"codec_type":"video","width":1920,"height":1080}]}]});
        assert_eq!(choose_program(&metadata), Some(123));
        assert_eq!(choose_program(&json!({"programs":[]})), None);
    }
    #[test]
    fn double_rate_deinterlaces_fields_without_forcing_progressive_frame_rate() {
        for gpu in [false, true] {
            let f = filter_with_deinterlacing(Resolution::Uhd, gpu, DeinterlaceMode::DoubleRate);
            assert!(
                f.contains("send_field") && f.contains("deint=interlaced") && !f.contains("fps=")
            );
            assert!(
                filter_with_deinterlacing(Resolution::Hd, gpu, DeinterlaceMode::SingleRate)
                    .contains("send_frame")
            );
            assert!(
                !filter_with_deinterlacing(Resolution::Native, gpu, DeinterlaceMode::Off)
                    .contains("bwdif")
            );
        }
    }
    #[test]
    fn default_worker_count_is_bounded() {
        assert!((1..=16).contains(&Options::default().threads));
    }
}
