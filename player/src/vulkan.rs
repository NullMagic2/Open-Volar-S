//! Vulkan-only swapchain renderer. Decoded samples are queued without waiting
//! for the GPU, so video processing cannot hold up the demultiplexer's audio.
#[path = "vulkan_picture.rs"]
mod picture_stage;
use a865r_bda::VideoTransform;
#[path="vulkan_pipeline.rs"]
mod pipeline;
use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, Win32WindowHandle, WindowsDisplayHandle,
};
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Condvar, Mutex,
    },
    time::{Duration, Instant},
};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Media::{DirectShow::*, IReferenceClock, MediaFoundation::*},
        UI::WindowsAndMessaging::GetClientRect,
    },
};
fn error(e: impl std::fmt::Display) -> Error {
    Error::new(E_FAIL, e.to_string())
}
// Use the instance belonging to this HWND, including child video windows.
// Win32WindowHandle::new leaves hinstance unset; wgpu 29 requires it for Vulkan.
fn video_window_handle(hwnd: usize) -> Result<Win32WindowHandle> {
    use std::num::NonZeroIsize;
    use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, GWLP_HINSTANCE};
    let hwnd = NonZeroIsize::new(hwnd as isize).ok_or_else(|| error("No video window"))?;
    // SAFETY: read-only window metadata. The UI keeps its video HWND alive until
    // the presentation worker has stopped. A missing/destroyed HWND fails here.
    let instance = unsafe { GetWindowLongPtrW(HWND(hwnd.get() as *mut _), GWLP_HINSTANCE) };
    let mut handle = Win32WindowHandle::new(hwnd);
    handle.hinstance = Some(
        NonZeroIsize::new(instance)
            .ok_or_else(|| error("Cannot obtain the video window's Windows instance handle"))?,
    );
    Ok(handle)
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Format {
    display_aspect: (u32,u32),
    pitch: usize,
    height: usize,
    width: usize,
    visible_height: usize,
    interlaced: bool,
    top_first: bool,
    bt709: bool,
    full: bool,
    frame_time: i64,
}
unsafe fn format(mt: &AM_MEDIA_TYPE) -> Option<Format> {
    if mt.majortype != MEDIATYPE_Video || mt.subtype != MEDIASUBTYPE_NV12 || mt.pbFormat.is_null() {
        return None;
    }
    let (h, source, flags, interlace, period, display_aspect) = if mt.formattype == FORMAT_VideoInfo2
        && mt.cbFormat as usize >= std::mem::size_of::<VIDEOINFOHEADER2>()
    {
        let v = std::ptr::read_unaligned(mt.pbFormat.cast::<VIDEOINFOHEADER2>());
        (
            v.bmiHeader,
            v.rcSource,
            v.Anonymous.dwControlFlags,
            v.dwInterlaceFlags,
            v.AvgTimePerFrame,
            (v.dwPictAspectRatioX,v.dwPictAspectRatioY),
        )
    } else if mt.formattype == FORMAT_VideoInfo
        && mt.cbFormat as usize >= std::mem::size_of::<VIDEOINFOHEADER>()
    {
        let v = std::ptr::read_unaligned(mt.pbFormat.cast::<VIDEOINFOHEADER>());
        (v.bmiHeader, v.rcSource, 0, 0, v.AvgTimePerFrame, (0,0))
    } else {
        return None;
    };
    let pitch = usize::try_from(h.biWidth).ok()?;
    let height = h.biHeight.unsigned_abs() as usize;
    if pitch == 0
        || pitch > 8192
        || height == 0
        || height > 4320
        || pitch % 2 != 0
        || height % 2 != 0
    {
        return None;
    }
    let width = if source.right > 0 {
        (source.right as usize).min(pitch)
    } else {
        pitch
    };
    let visible_height = if source.bottom > 0 {
        (source.bottom as usize).min(height)
    } else {
        height
    };
    let matrix = (flags >> 15) & 7;
    Some(Format {
        display_aspect: if display_aspect.0>0&&display_aspect.1>0 {display_aspect}else{(width as u32,visible_height as u32)},
        pitch,
        height,
        width,
        visible_height,
        interlaced: interlace & AMINTERLACE_IsInterlaced != 0,
        top_first: interlace & AMINTERLACE_Field1First != 0,
        bt709: if flags & 0x80 != 0 && matrix > 0 {
            matrix == 1
        } else {
            width >= 1280 || height > 576
        },
        full: flags & 0x80 != 0 && (flags >> 12) & 7 == 1,
        frame_time: if period > 0 { period } else { 333667 },
    })
}
struct Clock(IReferenceClock);
unsafe impl Send for Clock {} // DirectShow reference clocks support streaming threads.
#[derive(Clone)]
struct Frame {
    bytes: Arc<[u8]>,
    texture: Option<wgpu::Texture>,
    epoch: u64,
    format: Format,
    start: i64,
    end: i64,
    id: u64,
    flags: u32,
}
impl Frame {
    fn immediately_precedes(&self, next: &Self) -> bool {
        self.epoch == next.epoch && self.format == next.format && next.id > self.id
            && next.start > self.start && self.end > self.start
            && next.start.abs_diff(self.end) <= self.end.abs_diff(self.start) / 2
    }
}

fn sample_format(mut format: Format, flags: Option<u32>) -> Format {
    // Per-sample decoder metadata overrides the negotiated stream default.
    // Ignoring FIELD1FIRST reverses motion on streams whose header omits it.
    if let Some(flags) = flags {
        if format.interlaced && flags & AM_VIDEO_FLAG_FIELD_MASK as u32 == 0 {
            format.top_first = flags & AM_VIDEO_FLAG_FIELD1FIRST as u32 != 0;
            if flags & AM_VIDEO_FLAG_WEAVE as u32 != 0 {
                format.interlaced = false;
            }
        }
    }
    format
}
#[derive(Default)]
struct Pending {
    picture:crate::picture::Picture,
    picture_revision:u64,
    video_hdr:bool,
    hdr_active:bool,
    history:VecDeque<Frame>,
    step_frame:Option<Frame>,
    step_to:Option<(i64,i8)>,
    step_previous:Option<Frame>,
    displayed_start:i64,
    displayed_end:i64,
    stepping:bool,
    aspect: crate::aspect::AspectRatio,
    epoch: u64,
    frames: VecDeque<Frame>,
    clock: Option<Clock>,
    start: i64,
    paused: bool,
    live_paused: bool,
    eos: bool,
    snapshot: Option<PathBuf>,
    accepting: bool,
}
pub struct Input {
    format: Mutex<Format>,
    pending: Mutex<Pending>,
    space: Condvar,
    quit: AtomicBool,
    received: AtomicU64,
    presented: AtomicU64,
    dropped: AtomicU64,
    pub report: Mutex<Value>,
    error: Mutex<Option<String>>,
    finished: AtomicBool,
}
impl Input {
    pub fn epoch(&self) -> u64 {
        self.pending.lock().unwrap().epoch
    }
    pub fn accepting(&self) -> bool {
        self.pending.lock().unwrap().accepting
            && !self.quit.load(Ordering::Relaxed)
            && self.error().is_none()
    }
    pub fn has_failed(&self)->bool { self.quit.load(Ordering::Acquire) || self.error().is_some() }
    pub fn shutdown(&self) {
        self.quit.store(true, Ordering::Release);
        self.stop();
        self.space.notify_all();
    }
    pub fn fail(&self, message: String) {
        self.error.lock().unwrap_or_else(|e|e.into_inner()).get_or_insert(message);
        self.shutdown();
    }
    pub fn gpu_frame(&self, frame: gpu_video::broadcast::Frame, epoch: u64) -> Result<()> {
        let width = frame.texture.width() as usize;
        let height = frame.texture.height() as usize;
        let id = self.received.fetch_add(1, Ordering::Relaxed) + 1;
        self.enqueue(Frame {
            bytes: Arc::from([]),
            texture: Some(frame.texture),
            epoch,
            format: Format {
                display_aspect: frame.display_aspect,
                pitch: width,
                height,
                width,
                visible_height: height,
                interlaced: frame.interlaced,
                top_first: frame.top_first,
                bt709: frame.bt709,
                full: frame.full,
                frame_time: frame.duration,
            },
            start: frame.pts,
            end: frame.pts + frame.duration,
            id,
            flags: if frame.top_first { 4 } else { 0 },
        })
    }
    fn now(p: &Pending) -> i64 {
        p.clock
            .as_ref()
            .and_then(|c| unsafe { c.0.GetTime().ok() })
            .map(|time| time - p.start)
            .unwrap_or(0)
    }
    fn enqueue(&self, frame: Frame) -> Result<()> {
        let mut p = self.pending.lock().unwrap();
        if p.epoch != frame.epoch || !p.accepting || self.quit.load(Ordering::Acquire) {return Ok(());}
        // Decoder outputs already own their NV12 image. Retain leases only: no
        // additional GPU copies or CPU readbacks. Cap history by bytes and count.
        if let Some((target,direction))=p.step_to {
            if (direction<0 && frame.start<target)||(direction>=0 && frame.start<=target) {p.step_previous=Some(frame.clone());return Ok(());}
            let chosen=if direction<0 {p.step_previous.take().unwrap_or_else(||frame.clone())}else{frame.clone()};
            p.step_frame=Some(chosen);p.step_to=None;p.paused=true;p.stepping=true;
            p.picture_revision=p.picture_revision.wrapping_add(1);self.space.notify_all();
        }
        p.history.push_back(frame.clone());
        // Sum actual generations: switching from 4K to a small stream must not
        // allow the small frame size to retain 32 old 4K images.
        let frame_bytes=|f:&Frame| f.format.pitch.saturating_mul(f.format.height).saturating_mul(3)/2;
        let mut bytes=p.history.iter().fold(0usize,|n,f|n.saturating_add(frame_bytes(f)));
        while p.history.len()>32 || bytes>64*1024*1024 {
            let Some(old)=p.history.pop_front() else {break;};
            bytes=bytes.saturating_sub(frame_bytes(&old));
        }
        // Backpressure only at the bounded queue, never on a GPU readback.
        // Future frames must be retained; throwing them away freezes file playback.
        while p.frames.len() >= 12
            && p.epoch == frame.epoch
            && p.accepting
            && !p.live_paused
            && !self.quit.load(Ordering::Relaxed)
        {
            p = self
                .space
                .wait_timeout(p, Duration::from_millis(20))
                .unwrap()
                .0;
        }
        if p.epoch != frame.epoch || !p.accepting || self.quit.load(Ordering::Relaxed) {
            return Ok(());
        }
        if p.live_paused {
            // A pause requested during tuning must still show a first still picture.
            // Keep only one candidate until presentation acknowledges that picture.
            if self.presented.load(Ordering::Relaxed) == 0 && p.frames.is_empty() {
                p.frames.push_back(frame);
                self.space.notify_all();
            }
            return Ok(());
        }
        p.frames.push_back(frame);
        self.space.notify_all();
        Ok(())
    }
    /// Live reception must keep consuming packets while the displayed picture is held.
    /// Recorded/file playback uses the graph clock and bounded queue instead.
    pub fn set_live_paused(&self, paused: bool) {
        let mut p = self.pending.lock().unwrap();
        p.live_paused = paused;
        p.paused = paused;
        p.frames.clear();
        self.space.notify_all();
    }
    pub fn displayed_time(&self)->Option<(f64,f64)> {let p=self.pending.lock().unwrap();(p.displayed_end>p.displayed_start).then_some((p.displayed_start as f64/10_000_000.,(p.displayed_end-p.displayed_start) as f64/10_000_000.))}
    pub fn is_stepping(&self)->bool{self.pending.lock().unwrap().stepping}
    pub fn step(&self,direction:i8)->bool {
        let mut p=self.pending.lock().unwrap();
        let at=p.step_frame.as_ref().map(|f|f.start).unwrap_or(p.displayed_start);
        let chosen=if direction<0 {p.history.iter().filter(|f|f.start<at).max_by_key(|f|f.start)}else{p.history.iter().filter(|f|f.start>at).min_by_key(|f|f.start)}.cloned();
        if let Some(frame)=chosen {p.step_frame=Some(frame);p.paused=true;p.stepping=true;p.picture_revision=p.picture_revision.wrapping_add(1);self.space.notify_all();true}else{false}
    }
    pub fn seek_frame(&self,target:f64,direction:i8) {let mut p=self.pending.lock().unwrap();p.step_to=Some(((target*10_000_000.).round() as i64,direction));p.step_previous=None;p.frames.clear();p.stepping=true;self.space.notify_all();}
    pub fn set_video_hdr(&self,value:bool){self.pending.lock().unwrap().video_hdr=value;self.space.notify_all();}
    pub fn set_picture(&self,value:crate::picture::Picture){let mut p=self.pending.lock().unwrap();if p.picture!=value{p.picture=value;p.picture_revision=p.picture_revision.wrapping_add(1);}self.space.notify_all();}
    pub fn set_aspect_ratio(&self, aspect: crate::aspect::AspectRatio) {
        self.pending.lock().unwrap().aspect=aspect; self.space.notify_all();
    }
    pub fn snapshot(&self, path: PathBuf) {
        self.pending.lock().unwrap().snapshot = Some(path);
        self.space.notify_all();
    }
    pub fn stats(&self) -> Value {
        let mut report = self.report.lock().unwrap().clone();
        if report.is_null() {
            report = json!({});
        }
        report["received"] = json!(self.received.load(Ordering::Relaxed));
        report["presented"] = json!(self.presented.load(Ordering::Relaxed));
        {
            let pending = self.pending.lock().unwrap();
            report["paused"] = json!(pending.paused);
            report["live_paused"] = json!(pending.live_paused);
            report["queued_frames"] = json!(pending.frames.len());
        }

        report["dropped"] = json!(self.dropped.load(Ordering::Relaxed));
        report
    }
    pub fn error(&self) -> Option<String> {
        self.error.lock().unwrap().clone()
    }
    pub fn finished(&self) -> bool {
        self.finished.load(Ordering::Relaxed)
    }
}
impl VideoTransform for Input {
    fn accepts(&self, mt: &AM_MEDIA_TYPE) -> bool {
        unsafe { format(mt).is_some() }
    }
    fn configure(&self, mt: &AM_MEDIA_TYPE) -> Result<()> {
        *self.format.lock().unwrap() =
            unsafe { format(mt) }.ok_or_else(|| error("Vulkan requires decoded NV12 video"))?;
        Ok(())
    }
    fn bytes_required(&self) -> usize {
        let f = *self.format.lock().unwrap();
        f.pitch * f.height * 3 / 2
    }
    fn process(&self, _: &mut [u8]) -> Result<()> {
        Ok(())
    }
    fn terminal(&self) -> bool {
        true
    }
    fn receive(&self, sample: &IMediaSample) -> Result<()> {
        unsafe {
            let f = *self.format.lock().unwrap();
            let count = f.pitch * f.height * 3 / 2;
            if count == 0 || sample.GetActualDataLength() < count as i32 {
                return Err(error("Short decoded video sample"));
            }
            let (mut start, mut end) = (0, 0);
            let _ = sample.GetTime(&mut start, &mut end);
            if end <= start {
                end = start + f.frame_time;
            }
            let mut flags = 0;
            let mut sample_flags = None;
            if let Ok(sample2) = sample.cast::<IMediaSample2>() {
                let mut props = AM_SAMPLE2_PROPERTIES::default();
                props.cbData = std::mem::size_of_val(&props) as u32;
                let raw = std::slice::from_raw_parts_mut(
                    (&mut props as *mut AM_SAMPLE2_PROPERTIES).cast(),
                    std::mem::size_of_val(&props),
                );
                if sample2.GetProperties(raw).is_ok() {
                    flags = props.dwTypeSpecificFlags;
                    sample_flags = Some(flags);
                }
            }
            let id = self.received.fetch_add(1, Ordering::Relaxed) + 1;
            if id == 1 {
                let mut report = self.report.lock().unwrap();
                if !report.is_object() {
                    *report = json!({});
                }
                report["negotiated_top_first"] = json!(f.top_first);
                report["sample_properties_available"] = json!(sample_flags.is_some());
            }
            let frame = Frame {
                bytes: Arc::from(std::slice::from_raw_parts(sample.GetPointer()?, count)),
                texture: None,
                epoch: self.epoch(),
                format: sample_format(f, sample_flags),
                start,
                end,
                id,
                flags,
            };
            self.enqueue(frame)
        }
    }
    fn run(&self, start: i64, clock: Option<IReferenceClock>) {
        let mut p = self.pending.lock().unwrap();
        p.start = start;
        p.clock = clock.map(Clock);
        p.paused = false;
        p.live_paused = false;
        p.accepting = true;
        self.space.notify_all();
    }
    fn pause(&self) {
        let mut p = self.pending.lock().unwrap();
        p.paused = true;
        p.accepting = true;
        self.space.notify_all();
    }
    fn stop(&self) {
        let mut p = self.pending.lock().unwrap();
        p.epoch = p.epoch.wrapping_add(1);
        p.frames.clear();p.history.clear();p.step_frame=None;p.step_previous=None;
        p.paused = true;
        p.eos = false;
        p.live_paused = false;
        p.accepting = false;
        self.space.notify_all();
    }
    fn begin_flush(&self) {
        let mut p=self.pending.lock().unwrap();
        p.epoch=p.epoch.wrapping_add(1);
        p.frames.clear();p.history.clear();p.step_frame=None;p.step_previous=None;p.eos=false;p.accepting=false;
        // A demux flush is not a Pause command. Keep the running clock and
        // requested pause state so dynamic audio/caption changes can resume.
        self.space.notify_all();
    }
    fn end_flush(&self) {
        self.pending.lock().unwrap().accepting = true;
        self.space.notify_all();
    }
    fn end_of_stream(&self) {
        self.pending.lock().unwrap().eos = true;
        self.space.notify_all();
    }
}
pub struct Presenter {
    pub input: Arc<Input>,
    pub device: Option<Arc<gpu_video::VulkanDevice>>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl Drop for Presenter {
    fn drop(&mut self) {
        if self.input.error().is_some_and(|e|crate::native::graphics_lost(&json!({"error":e}))) {
            if let Some(device)=&self.device {device.mark_lost();}
        }
        self.input.shutdown();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
impl Presenter {
    pub fn new(
        hwnd: usize,
        resolution: a865r::api::Resolution,
        deinterlacing: a865r::api::DeinterlaceMode,
        profile: Option<Arc<crate::icc::Transform>>,
        folder: PathBuf,
        require_decoding:bool,
        shader:crate::backend::Shader,
    ) -> Result<Self> {
        let input = Arc::new(Input {
            format: Mutex::new(Format::default()),
            pending: Mutex::new(Pending {
                paused: true,
                accepting: true,
                ..Default::default()
            }),
            space: Condvar::new(),
            quit: AtomicBool::new(false),
            received: AtomicU64::new(0),
            presented: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            report: Mutex::new(Value::Null),
            error: Mutex::new(None),
            finished: AtomicBool::new(false),
        });
        let state = input.clone();
        let (ready_tx, ready) = std::sync::mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || {
            let _ = unsafe {
                windows::Win32::System::Com::CoInitializeEx(
                    None,
                    windows::Win32::System::Com::COINIT_MULTITHREADED,
                )
            };
            let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||->Result<()>{
                pipeline::run(state.clone(),hwnd,resolution,deinterlacing,profile.clone(),folder.clone(),&ready_tx,require_decoding,shader)
            })).unwrap_or_else(|panic| {
                let message=panic.downcast_ref::<String>().map(String::as_str)
                    .or_else(||panic.downcast_ref::<&str>().copied()).unwrap_or("Unknown renderer panic");
                let _=std::fs::write(folder.join("native-renderer-error.txt"),message);
                Err(error(format!("Vulkan renderer failed: {message}")))
            });
            if let Err(e) = result {
                let message = e.to_string();
                let _ = ready_tx.send(Err(message.clone()));
                state.fail(message);
            }
            unsafe {
                windows::Win32::System::Com::CoUninitialize();
            }
        });
        let device = match ready
            .recv_timeout(Duration::from_secs(30))
            .map_err(error)
            .and_then(|r| r.map_err(error))
        {
            Ok(device) => device,
            Err(e) => {
                input.quit.store(true, Ordering::Relaxed);
                let _ = worker.join();
                return Err(e);
            }
        };
        let presenter = Self {
            input,
            device,
            worker: Some(worker),
        };
        Ok(presenter)
    }
}
struct Textures {
    y: wgpu::Texture,
    uv: wgpu::Texture,
    previous: wgpu::Texture,
    next: wgpu::Texture,
    next_id: Option<u64>,
    bind: wgpu::BindGroup,
    shape: (usize, usize, usize, usize),
    luma_view: wgpu::TextureView,
    luma_bind: wgpu::BindGroup,
    previous_ready: bool,
}
struct Gpu {
    external: Option<picture_stage::PictureStage>,
    work: crate::gpu_work::Work,
    picture:crate::picture::Picture,
    deinterlacing: a865r::api::DeinterlaceMode,
    aspect: crate::aspect::AspectRatio,
    video_device: Option<Arc<gpu_video::VulkanDevice>>,
    current_gpu: Option<wgpu::Texture>,
    previous_gpu: Option<wgpu::Texture>,
    surface: Option<wgpu::Surface<'static>>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    hdr_capable:bool,
    pipeline: wgpu::RenderPipeline,
    luma_pipeline: wgpu::RenderPipeline,
    luma_layout: wgpu::BindGroupLayout,
    canvas: crate::canvas::Canvas,
    layout: wgpu::BindGroupLayout,
    lut: wgpu::TextureView,
    sampler: wgpu::Sampler,
    uniform: wgpu::Buffer,
    textures: Option<Textures>,
    hwnd: usize,
    resolution: a865r::api::Resolution,
    color: bool,
    adapter: String,
    viewport: [u32; 4],
    buffering: Value,
}
impl Gpu {
    async fn new(
        hwnd: usize,
        resolution: a865r::api::Resolution,
        profile: Option<Arc<crate::icc::Transform>>,
        require_decoding:bool,
    ) -> Result<Self> {
        // Microsoft decoding needs Vulkan graphics only, not Vulkan Video extensions.
        let video_instance=if require_decoding {Some(gpu_video::VulkanInstance::new().map_err(error)?)}else{None};
        let instance=if let Some(v)=&video_instance {v.wgpu_instance()}else{wgpu::Instance::new(wgpu::InstanceDescriptor{backends:wgpu::Backends::VULKAN,..wgpu::InstanceDescriptor::new_without_display_handle()})};
        let window=video_window_handle(hwnd)?;
        let surface=unsafe {instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
            raw_display_handle:Some(RawDisplayHandle::Windows(WindowsDisplayHandle::new())),raw_window_handle:RawWindowHandle::Win32(window),
        })}.map_err(error)?;
        let (video_device,adapter,device,queue)=if let Some(v)=video_instance {
            let a=v.create_adapter(&gpu_video::parameters::VulkanAdapterDescriptor{supports_decoding:true,supports_encoding:false,compatible_surface:Some(&surface)}).map_err(error)?;
            let d=a.create_device(&Default::default()).map_err(error)?;
            (Some(d.clone()),d.wgpu_adapter(),d.wgpu_device(),d.wgpu_queue())
        }else{
            let a=instance.request_adapter(&wgpu::RequestAdapterOptions{power_preference:wgpu::PowerPreference::HighPerformance,force_fallback_adapter:false,compatible_surface:Some(&surface)}).await.map_err(error)?;
            if a.get_info().device_type==wgpu::DeviceType::Cpu {return Err(error("No hardware Vulkan graphics adapter"));}
            let(d,q)=a.request_device(&wgpu::DeviceDescriptor{label:Some("Vulkan shader acceleration"),..Default::default()}).await.map_err(error)?;
            (None,a,d,q)
        };
        let info=adapter.get_info();
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| {
                matches!(
                    f,
                    wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm
                )
            })
            .ok_or_else(|| error("Vulkan surface has no SDR unorm format"))?;
        // Query actual Vulkan surface limits without owning or modifying raw handles.
        let limits = unsafe {
            let surface_hal = surface.as_hal::<wgpu::hal::api::Vulkan>();
            let adapter_hal = adapter.as_hal::<wgpu::hal::api::Vulkan>();
            use wgpu::hal::Adapter;
            adapter_hal
                .zip(surface_hal)
                .and_then(|(a, s)| a.surface_capabilities(&*s))
                .map(|c| {
                    (
                        *c.maximum_frame_latency.start() + 1,
                        *c.maximum_frame_latency.end() + 1,
                    )
                })
        };
        let buffers = limits.map(|(min, max)| 3u32.clamp(min, max)).unwrap_or(3);
        let buffering = json!({"requested_images":buffers,"minimum_images":limits.map(|v|v.0),"maximum_images":limits.map(|v|v.1),"triple_buffering_supported":limits.map(|(min,max)|min<=3&&max>=3),"mode":if buffers==3{"triple buffering"}else{"driver-supported buffering"}});
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | if caps.usages.contains(wgpu::TextureUsages::COPY_SRC) {
                    wgpu::TextureUsages::COPY_SRC
                } else {
                    wgpu::TextureUsages::empty()
                },
            format,
            width: 1,
            height: 1,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: buffers - 1,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        let validation_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let entries = (0..4)
            .map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float {
                        filterable: binding != 3,
                    },
                    view_dimension: if binding == 3 {
                        wgpu::TextureViewDimension::D3
                    } else {
                        wgpu::TextureViewDimension::D2
                    },
                    multisampled: false,
                },
                count: None,
            })
            .chain([
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ])
            .collect::<Vec<_>>();
        let mut entries = entries;
        let mut next_entry = entries[0];
        next_entry.binding = 6;
        entries.push(next_entry);
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Video textures and ICC"),
            entries: &entries,
        });
        let luma_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Reconstructed luma"),
            entries: &[wgpu::BindGroupLayoutEntry {binding:0,visibility:wgpu::ShaderStages::FRAGMENT,
                ty:wgpu::BindingType::Texture {sample_type:wgpu::TextureSampleType::Float {filterable:true},view_dimension:wgpu::TextureViewDimension::D2,multisampled:false},count:None}],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout),Some(&luma_layout)],
            immediate_size: 0,
        });
        let luma_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Source-resolution deinterlace"),bind_group_layouts:&[Some(&layout)],immediate_size:0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("NV12, adaptive deinterlace, hardware linear scale, ICC"),
            source: wgpu::ShaderSource::Wgsl(include_str!("video.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Vulkan video"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let luma_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Source-resolution deinterlace"),
            layout: Some(&luma_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_luma"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format:wgpu::TextureFormat::R16Float,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let lut = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ICC 33 cubed"),
            size: wgpu::Extent3d {
                width: 33,
                height: 33,
                depth_or_array_layers: 33,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let table = profile
            .as_ref()
            .map(|p| p.lut_rgba())
            .unwrap_or_else(|| vec![0.; 33 * 33 * 33 * 4]);
        let bytes: Vec<u8> = table.iter().flat_map(|f| f.to_le_bytes()).collect();
        queue.write_texture(
            lut.as_image_copy(),
            &bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(33 * 16),
                rows_per_image: Some(33),
            },
            lut.size(),
        );
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 80,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if let Some(e) = validation_scope.pop().await {
            return Err(error(e));
        }
        let canvas = crate::canvas::Canvas::new(&device, config.format, caps.formats.contains(&wgpu::TextureFormat::Rgba16Float));
        Ok(Self {
            external: None,
            work:Default::default(),
            canvas,
            hdr_capable:caps.formats.contains(&wgpu::TextureFormat::Rgba16Float),
            picture:Default::default(),
            deinterlacing: a865r::api::DeinterlaceMode::DoubleRate,
            video_device,
            current_gpu: None,
            previous_gpu: None,
            surface:Some(surface),
            device,
            queue,
            config,
            pipeline,
            luma_pipeline,luma_layout,
            layout,
            lut: lut.create_view(&Default::default()),
            sampler,
            uniform,
            textures: None,
            hwnd,
            resolution,
            aspect: Default::default(),
            color: profile.is_some(),
            adapter: info.name,
            viewport: [0; 4],
            buffering,
        })
    }
    fn window_size(&self) -> (u32, u32) {
        let mut r = RECT::default();
        let _ = unsafe { GetClientRect(HWND(self.hwnd as _), &mut r) };
        (r.right.max(1) as u32, r.bottom.max(1) as u32)
    }
    fn textures(&mut self, f: Format) {
        let make = |label: &str, width: u32, height: u32, format| {
            self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            })
        };
        let y = make(
            "Luma",
            f.pitch as u32,
            f.height as u32,
            wgpu::TextureFormat::R8Unorm,
        );
        let previous = make(
            "Previous luma",
            f.pitch as u32,
            f.height as u32,
            wgpu::TextureFormat::R8Unorm,
        );
        let uv = make(
            "Chroma",
            f.pitch as u32 / 2,
            f.height as u32 / 2,
            wgpu::TextureFormat::Rg8Unorm,
        );
        let next = make(
            "Next luma",
            f.pitch as u32,
            f.height as u32,
            wgpu::TextureFormat::R8Unorm,
        );
        let next_view = next.create_view(&Default::default());
        let views = [
            y.create_view(&Default::default()),
            uv.create_view(&Default::default()),
            previous.create_view(&Default::default()),
            self.lut.clone(),
        ];
        let mut entries = views
            .iter()
            .enumerate()
            .map(|(i, v)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: wgpu::BindingResource::TextureView(v),
            })
            .collect::<Vec<_>>();
        entries.push(wgpu::BindGroupEntry {
            binding: 4,
            resource: wgpu::BindingResource::Sampler(&self.sampler),
        });
        entries.push(wgpu::BindGroupEntry {
            binding: 5,
            resource: self.uniform.as_entire_binding(),
        });
        entries.push(wgpu::BindGroupEntry {
            binding: 6,
            resource: wgpu::BindingResource::TextureView(&next_view),
        });
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.layout,
            entries: &entries,
        });
        let luma=self.device.create_texture(&wgpu::TextureDescriptor {
            label:Some("Deinterlaced luma at source resolution"),
            size:wgpu::Extent3d{width:f.width as u32,height:f.visible_height as u32,depth_or_array_layers:1},
            mip_level_count:1,sample_count:1,dimension:wgpu::TextureDimension::D2,format:wgpu::TextureFormat::R16Float,
            usage:wgpu::TextureUsages::RENDER_ATTACHMENT|wgpu::TextureUsages::TEXTURE_BINDING,view_formats:&[],
        });
        let luma_view=luma.create_view(&Default::default());
        let luma_bind=self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:Some("Cached source luma"),layout:&self.luma_layout,
            entries:&[wgpu::BindGroupEntry {binding:0,resource:wgpu::BindingResource::TextureView(&luma_view)}],
        });
        self.textures = Some(Textures {
            y,
            uv,
            previous,
            next,
            next_id: None,
            bind,
            shape: (f.pitch, f.height,f.width,f.visible_height),
            luma_view,luma_bind,
            previous_ready: false,
        });
    }
    fn prepare(
        &mut self, frame:&Frame, field:u32, new:bool,
        target:Arc<crate::canvas::Canvas>, next:Option<&Frame>,
    )->Result<()> {
        if let Some(bridge) = &mut self.external {
            let changed = bridge.picture(self.picture);
            let frame = bridge.frame(frame)?;
            let next = next.map(|f| bridge.frame(f)).transpose()?;
            if changed { self.textures = None; }
            let picture = self.picture; let color = self.color;
            self.picture = Default::default(); self.color = false;
            let result = self.prepare_frame(&frame, field, new || changed, target, next.as_ref());
            self.picture = picture; self.color = color;
            result
        } else { self.prepare_frame(frame, field, new, target, next) }
    }
    fn prepare_frame(
        &mut self, frame:&Frame, field:u32, new:bool,
        target:Arc<crate::canvas::Canvas>, next:Option<&Frame>,
    )->Result<()> {
        let size = self.window_size();
        let f = frame.format;
        let cap = self
            .resolution
            .dimensions()
            .unwrap_or((f.width as u32, f.visible_height as u32));
        let (viewport, _) =
            crate::canvas::geometry(self.aspect.display((f.width as u32,f.visible_height as u32),f.display_aspect), size, cap);
        self.viewport = viewport;
        let processing=target.size;
        let processed=target.view();


        let mut encoder=self.device.create_command_encoder(&Default::default());

        let recreate = self
            .textures
            .as_ref()
            .is_none_or(|t| t.shape != (f.pitch, f.height,f.width,f.visible_height));
        if recreate {
            self.textures(f);
        }
        let t = self.textures.as_mut().unwrap();
        if frame.texture.is_none() && (new || recreate) {
            if !recreate {
                let mut copy = self.device.create_command_encoder(&Default::default());
                copy.copy_texture_to_texture(
                    t.y.as_image_copy(),
                    t.previous.as_image_copy(),
                    t.y.size(),
                );
                self.queue.submit(Some(copy.finish()));
                t.previous_ready = true;
            }
            self.queue.write_texture(
                t.y.as_image_copy(),
                &frame.bytes[..f.pitch * f.height],
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(f.pitch as u32),
                    rows_per_image: None,
                },
                t.y.size(),
            );
            self.queue.write_texture(
                t.uv.as_image_copy(),
                &frame.bytes[f.pitch * f.height..],
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(f.pitch as u32),
                    rows_per_image: None,
                },
                t.uv.size(),
            );
        }
        let next = next.filter(|next| frame.immediately_precedes(next));
        if let Some(next) = next.filter(|f| f.texture.is_none()) {
            if t.next_id != Some(next.id) {
                self.queue.write_texture(
                    t.next.as_image_copy(),
                    &next.bytes[..f.pitch * f.height],
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(f.pitch as u32),
                        rows_per_image: None,
                    },
                    t.next.size(),
                );
                t.next_id = Some(next.id);
            }
        }
        if let Some(texture) = &frame.texture {
            if new || recreate {
                self.previous_gpu = if recreate {
                    None
                } else {
                    self.current_gpu.take()
                };
                self.current_gpu = Some(texture.clone());
            }
            // Reuse views/bindings for the second field and repeated refreshes.
            if new || recreate || t.next_id != next.map(|n|n.id) {
            t.next_id=next.map(|n|n.id);
            // Bind NV12 planes directly; no decoded pixels cross system RAM.
            let plane = |texture: &wgpu::Texture, aspect, format| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    aspect,
                    format: Some(format),
                    ..Default::default()
                })
            };
            let y = plane(
                texture,
                wgpu::TextureAspect::Plane0,
                wgpu::TextureFormat::R8Unorm,
            );
            let uv = plane(
                texture,
                wgpu::TextureAspect::Plane1,
                wgpu::TextureFormat::Rg8Unorm,
            );
            let previous = plane(
                self.previous_gpu.as_ref().unwrap_or(texture),
                wgpu::TextureAspect::Plane0,
                wgpu::TextureFormat::R8Unorm,
            );
            let future = plane(
                next.and_then(|n| n.texture.as_ref()).unwrap_or(texture),
                wgpu::TextureAspect::Plane0,
                wgpu::TextureFormat::R8Unorm,
            );
            let lut = self.lut.clone();
            t.previous_ready = self.previous_gpu.is_some();
            t.bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("GPU decoded NV12 planes"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&y),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&uv),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&previous),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&lut),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: self.uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::TextureView(&future),
                    },
                ],
            });
        }
        }
        let parity = if f.top_first { field } else { 1 - field };
        let (crop_x,crop_width)=self.aspect.aperture((f.width as u32,f.visible_height as u32),f.display_aspect);
        let adjust=self.picture.uniform();
        let params: [f32; 20] = [
            f.width as f32,
            f.visible_height as f32,
            f.pitch as f32,
            if f.full { 1. } else { 0. },
            if f.bt709 { 0.2126 } else { 0.299 },
            if f.bt709 { 0.0722 } else { 0.114 },
            if f.interlaced && self.deinterlacing!=a865r::api::DeinterlaceMode::Off { 1. } else { 0. },
            parity as f32,
            if self.color { 1. } else { 0. },
            if t.previous_ready { 1. } else { 0. },
            f.height as f32,
            if next.is_some() { 1. } else { 0. },
            crop_x as f32/f.width as f32,crop_width as f32/f.width as f32,self.picture.effect_strength(),0.,
            adjust[0],adjust[1],adjust[2],adjust[3],
        ];
        self.queue.write_buffer(
            &self.uniform,
            0,
            &params
                .iter()
                .flat_map(|f| f.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        if f.interlaced && self.deinterlacing!=a865r::api::DeinterlaceMode::Off {
            let mut pass=encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label:Some("Reconstruct each source pixel once"),
                color_attachments:&[Some(wgpu::RenderPassColorAttachment {view:&t.luma_view,resolve_target:None,depth_slice:None,
                    ops:wgpu::Operations{load:wgpu::LoadOp::Clear(wgpu::Color::BLACK),store:wgpu::StoreOp::Store}})],
                depth_stencil_attachment:None,occlusion_query_set:None,timestamp_writes:None,multiview_mask:None,
            });
            pass.set_pipeline(&self.luma_pipeline);pass.set_bind_group(0,&t.bind,&[]);
            pass.set_viewport(0.,0.,f.width as f32,f.visible_height as f32,0.,1.);pass.draw(0..3,0..1);
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &processed,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &t.bind, &[]);
            pass.set_bind_group(1, &t.luma_bind, &[]);
            pass.set_viewport(0., 0., processing.0 as f32, processing.1 as f32, 0., 1.);
            pass.draw(0..3, 0..1);
        }


        let completion = self.work.reserve().map_err(error)?;
        self.queue.submit(Some(encoder.finish()));
        // The pool slot cannot be reused until this GPU write completes.
        self.queue.on_submitted_work_done(move || {drop(target);drop(completion);});
        Ok(())
    }

}

#[cfg(test)]
mod tests {
    #[test]
    fn video_handle_carries_the_windows_instance_without_gpu_initialization() {
        use windows::Win32::{System::LibraryLoader::GetModuleHandleW, UI::WindowsAndMessaging::*};
        assert!(video_window_handle(0).is_err());
        unsafe {
            let instance = GetModuleHandleW(None).unwrap();
            // A message-only window stays invisible and does not create a GPU surface.
            let window = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                w!("Handle regression"),
                WINDOW_STYLE(0),
                0,
                0,
                16,
                16,
                HWND_MESSAGE,
                None,
                instance,
                None,
            )
            .unwrap();
            let handle = video_window_handle(window.0 as usize);
            DestroyWindow(window).unwrap();
            let handle = handle.unwrap();
            assert_eq!(handle.hwnd.get(), window.0 as isize);
            assert_eq!(handle.hinstance.unwrap().get(), instance.0 as isize);
            assert!(video_window_handle(window.0 as usize).is_err());
        }
    }
    #[test]
    fn sample_field_order_overrides_stream_default() {
        let base = super::Format {
            interlaced: true,
            top_first: false,
            ..Default::default()
        };
        assert!(
            super::sample_format(base, Some(super::AM_VIDEO_FLAG_FIELD1FIRST as u32)).top_first
        );
        assert!(!super::sample_format(base, None).top_first);
        assert!(!super::sample_format(base, Some(super::AM_VIDEO_FLAG_WEAVE as u32)).interlaced);
    }
    #[test]
    #[ignore = "Requires real GPU execution; excluded after reported driver instability"]
    fn stationary_detail_does_not_bob_between_gpu_fields() {
        use windows::Win32::{System::LibraryLoader::GetModuleHandleW, UI::WindowsAndMessaging::*};
        unsafe {
            let window = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                w!("Deinterlace regression"),
                WS_OVERLAPPEDWINDOW,
                0,
                0,
                160,
                160,
                None,
                None,
                GetModuleHandleW(None).unwrap(),
                None,
            )
            .unwrap();
            let mut gpu = pollster::block_on(Gpu::new(
                window.0 as usize,
                a865r::api::Resolution::Native,
                None,true,
            ))
            .unwrap();
            let mut bytes = vec![128u8; 64 * 64 * 3 / 2];
            for y in 0..64 {
                for x in 0..64 {
                    bytes[y * 64 + x] = if y % 2 == 0 {
                        40 + x as u8
                    } else {
                        200 - x as u8
                    };
                }
            }
            let mut frame = Frame {
                bytes: bytes.into(),
                texture: None,
                epoch: 0,
                format: Format {
                    pitch: 64,
                    height: 64,
                    width: 64,
                    visible_height: 64,
                    interlaced: true,
                    top_first: true,
                    bt709: true,
                    frame_time: 333667,
                    ..Default::default()
                },
                start: 0,
                end: 333667,
                id: 1,
                flags: 4,
            };
            let mut fixture=pipeline::Fixture::new(&mut gpu);
            let state=input();
            fixture.draw(&mut gpu,&frame,0,true,None,None,&state).unwrap();
            frame.id = 2;
            let mut next = frame.clone();
            next.id = 3;
            next.start = frame.end;
            next.end = next.start + 333667;
            let dir =
                std::env::temp_dir().join(format!("a865r-field-regression-{}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            let first = dir.join("first.bmp");
            let second = dir.join("second.bmp");
            fixture.draw(&mut gpu,&frame,0,true,Some(first.clone()),Some(&next),&state)
                .unwrap();
            fixture.draw(&mut gpu,&frame,1,false,Some(second.clone()),Some(&next),&state)
                .unwrap();
            assert_eq!(
                std::fs::read(&first).unwrap(),
                std::fs::read(&second).unwrap(),
                "Stationary detail moved between fields"
            );
            drop(fixture);
            drop(gpu);
            DestroyWindow(window).unwrap();
            std::fs::remove_file(first).unwrap();
            std::fs::remove_file(second).unwrap();
            std::fs::remove_dir(dir).unwrap();
        }
    }
    #[test]
    fn shader_is_valid() {
        let module = naga::front::wgsl::parse_str(include_str!("video.wgsl")).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
    }
    use super::*;
    fn input() -> Arc<Input> {
        Arc::new(Input {
            format: Mutex::new(Format::default()),
            pending: Mutex::new(Pending {
                paused: true,
                accepting: true,
                ..Default::default()
            }),
            space: Condvar::new(),
            quit: AtomicBool::new(false),
            received: AtomicU64::new(0),
            presented: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            report: Mutex::new(Value::Null),
            error: Mutex::new(None),
            finished: AtomicBool::new(false),
        })
    }
    #[test]
    fn hdr_effect_requests_a_paused_repaint_without_flushing_frames(){
        let state=input();state.enqueue(frame(1)).unwrap();
        let before=state.pending.lock().unwrap().picture_revision;
        let effect=crate::picture::Picture{hdr_effect:true,..Default::default()};
        state.set_picture(effect);
        {let p=state.pending.lock().unwrap();assert!(p.paused);assert!(!p.video_hdr);assert_eq!(p.frames.len(),1);assert_eq!(p.picture_revision,before+1);assert!(p.picture.hdr_effect);}
        state.set_picture(effect);assert_eq!(state.pending.lock().unwrap().picture_revision,before+1);
        state.set_picture(Default::default());let p=state.pending.lock().unwrap();assert_eq!(p.picture_revision,before+2);assert_eq!(p.frames.len(),1);
    }
    fn frame(id: u64) -> Frame {
        Frame {
            bytes: Arc::from([]),
            texture: None,
            epoch: 0,
            format: Format::default(),
            start: id as i64 * 333667,
            end: (id + 1) as i64 * 333667,
            id,
            flags: 0,
        }
    }
    #[test]
    fn frame_steps_choose_adjacent_frames_and_pause_without_readback(){
        let state=input();for id in 0..6{state.enqueue(frame(id)).unwrap();}
        {let mut p=state.pending.lock().unwrap();p.displayed_start=3*333667;p.displayed_end=4*333667;}
        assert!(state.step(-1));assert_eq!(state.pending.lock().unwrap().step_frame.as_ref().unwrap().id,2);
        assert!(state.step(1));let p=state.pending.lock().unwrap();assert_eq!(p.step_frame.as_ref().unwrap().id,3);assert!(p.paused);assert!(p.stepping);
    }
    #[test]
    fn backward_decode_stops_before_the_exact_current_timestamp(){
        let state=input();state.seek_frame(3.*333667./10_000_000.,-1);
        for id in 0..=3{state.enqueue(frame(id)).unwrap();}
        let p=state.pending.lock().unwrap();assert!(p.step_to.is_none());assert_eq!(p.step_frame.as_ref().unwrap().id,2);assert!(p.paused);
    }
    #[test]
    fn resolution_change_and_stopped_delivery_do_not_retain_old_images(){
        let state=input();
        for id in 0..5 {let mut f=frame(id);f.format.pitch=3840;f.format.height=2160;state.enqueue(f).unwrap();state.pending.lock().unwrap().frames.clear();}
        for id in 5..15 {let mut f=frame(id);f.format.pitch=1280;f.format.height=720;state.enqueue(f).unwrap();state.pending.lock().unwrap().frames.clear();}
        {let p=state.pending.lock().unwrap();let bytes:usize=p.history.iter().map(|f|f.format.pitch*f.format.height*3/2).sum();assert!(bytes<=64*1024*1024);}
        state.shutdown();let f=frame(51);let weak=std::sync::Arc::downgrade(&f.bytes);
        state.enqueue(f).unwrap();assert!(state.pending.lock().unwrap().history.is_empty());assert!(weak.upgrade().is_none());
    }
    #[test]
    fn history_is_bounded_by_bytes_and_flush_drops_old_frames(){
        let state=input();
        for id in 0..100{let mut f=frame(id);f.format.pitch=3840;f.format.height=2160;state.enqueue(f).unwrap();state.pending.lock().unwrap().frames.clear();}
        assert!(state.pending.lock().unwrap().history.len()<=5);
        state.begin_flush();assert!(state.pending.lock().unwrap().history.is_empty());
    }
    #[test]
    fn failure_cancels_blocked_delivery_and_preserves_first_error() {
        let state=input();state.run(0,None);
        state.presented.store(1,Ordering::Relaxed);
        for id in 0..12 {state.enqueue(frame(id)).unwrap();}
        let pending=state.clone();let (tx,rx)=std::sync::mpsc::channel();
        let worker=std::thread::spawn(move || {let _=pending.enqueue(frame(13));tx.send(()).unwrap();});
        assert!(rx.recv_timeout(Duration::from_millis(30)).is_err());
        state.fail("primary decoder failure".into());state.fail("secondary cleanup failure".into());
        rx.recv_timeout(Duration::from_secs(1)).unwrap();worker.join().unwrap();
        assert_eq!(state.error().as_deref(),Some("primary decoder failure"));
        assert!(!state.accepting());assert!(state.has_failed());
        state.begin_flush();state.end_flush();state.run(0,None);
        assert!(!state.accepting(),"A graph restart must not revive a failed input");
    }
    #[test]
    fn lookahead_rejects_discontinuities_and_other_channels() {
        let current = frame(1);
        let mut next = frame(2);
        assert!(current.immediately_precedes(&next));
        next.start += 10;
        assert!(current.immediately_precedes(&next));
        next.epoch += 1;
        assert!(!current.immediately_precedes(&next));
        next = frame(4);
        assert!(!current.immediately_precedes(&next));
        next = frame(2);
        next.format.width = 1920;
        assert!(!current.immediately_precedes(&next));
        assert!(!current.immediately_precedes(&current));
    }
    #[test]
    fn live_pause_before_first_picture_retains_one_preview_then_resumes() {
        let state = input();
        state.run(1234, None);
        state.set_live_paused(true);
        for id in 0..40 {
            state.enqueue(frame(id)).unwrap();
        }
        assert_eq!(state.pending.lock().unwrap().frames.len(), 1);
        assert_eq!(state.pending.lock().unwrap().frames.front().unwrap().id, 0);
        state.pending.lock().unwrap().frames.pop_front();
        state.presented.store(1, Ordering::Relaxed);
        state.enqueue(frame(41)).unwrap();
        assert!(state.pending.lock().unwrap().frames.is_empty());
        state.set_live_paused(false);
        state.enqueue(frame(42)).unwrap();
        assert_eq!(state.pending.lock().unwrap().frames.front().unwrap().id, 42);
        assert_eq!(state.epoch(), 0);
    }
    #[test]
    fn live_pause_unblocks_full_queue_and_resumes_without_resetting_decoder() {
        let state = input();
        state.run(1234, None);
        state.presented.store(1, Ordering::Relaxed);
        for id in 0..12 {
            state.enqueue(frame(id)).unwrap();
        }
        let worker_state = state.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            worker_state.enqueue(frame(12)).unwrap();
            tx.send(()).unwrap();
        });
        assert!(rx.recv_timeout(Duration::from_millis(30)).is_err());
        state.set_live_paused(true);
        rx.recv_timeout(Duration::from_secs(1)).unwrap();
        worker.join().unwrap();
        for id in 13..100 {
            state.enqueue(frame(id)).unwrap();
        }
        assert!(state.pending.lock().unwrap().frames.is_empty());
        assert_eq!(state.epoch(), 0);
        state.set_live_paused(false);
        state.enqueue(frame(100)).unwrap();
        let pending = state.pending.lock().unwrap();
        assert!(!pending.paused);
        assert_eq!(pending.start, 1234);
        assert_eq!(pending.frames.front().unwrap().id, 100);
    }

    #[test]
    fn bounded_future_frames_and_flush_recovery() {
        let state = input();
        for id in 0..12 {
            state.enqueue(frame(id)).unwrap();
        }
        let worker_state = state.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            worker_state.enqueue(frame(12)).unwrap();
            tx.send(()).unwrap();
        });
        assert!(rx.recv_timeout(Duration::from_millis(30)).is_err());
        assert_eq!(state.pending.lock().unwrap().frames.front().unwrap().id, 0);
        state.stop();
        rx.recv_timeout(Duration::from_secs(1)).unwrap();
        worker.join().unwrap();
        assert!(state.pending.lock().unwrap().frames.is_empty());
        state.end_flush();
        let mut fresh = frame(20);
        fresh.epoch = state.epoch();
        state.enqueue(fresh).unwrap();
        assert_eq!(state.pending.lock().unwrap().frames.front().unwrap().id, 20);
    }
    #[test]
    fn dynamic_stream_flush_preserves_clock_and_requested_pause_state() {
        let state=input();
        state.run(12345,None);
        let epoch=state.epoch();
        state.begin_flush();state.end_flush();
        {let p=state.pending.lock().unwrap();assert!(!p.paused);assert!(p.accepting);assert_eq!(p.start,12345);assert_ne!(p.epoch,epoch);}
        state.pause();state.begin_flush();state.end_flush();
        assert!(state.pending.lock().unwrap().paused);
        state.set_live_paused(true);state.begin_flush();state.end_flush();
        assert!(state.pending.lock().unwrap().live_paused);
    }
    #[test]
    fn decoded_padding_interlace_and_color_metadata() {
        unsafe {
            let mut vi = VIDEOINFOHEADER2::default();
            vi.bmiHeader.biWidth = 2048;
            vi.bmiHeader.biHeight = -1088;
            vi.rcSource.right = 1920;
            vi.rcSource.bottom = 1080;
            vi.AvgTimePerFrame = 333667;
            vi.dwInterlaceFlags = AMINTERLACE_IsInterlaced | AMINTERLACE_Field1First;
            vi.Anonymous.dwControlFlags = AMCONTROL_COLORINFO_PRESENT | (1 << 15) | (1 << 12);
            let mut mt = AM_MEDIA_TYPE {
                majortype: MEDIATYPE_Video,
                subtype: MEDIASUBTYPE_NV12,
                formattype: FORMAT_VideoInfo2,
                cbFormat: std::mem::size_of_val(&vi) as u32,
                pbFormat: (&mut vi as *mut VIDEOINFOHEADER2).cast(),
                ..Default::default()
            };
            let f = format(&mt).unwrap();
            assert_eq!(
                (f.width, f.visible_height, f.pitch, f.height),
                (1920, 1080, 2048, 1088)
            );
            assert!(f.interlaced && f.top_first && f.full && f.bt709);
            mt.cbFormat = 1;
            assert!(format(&mt).is_none());
        }
    }
}
