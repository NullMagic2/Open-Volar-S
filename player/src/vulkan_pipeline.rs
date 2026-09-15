//! Application-owned GPU images between preparation and Windows presentation.
use super::*;
use crate::frame_pool::{Pool, Ready, Timed};
use std::io::Write;

struct Prepared {
    image: Arc<crate::canvas::Canvas>,
    epoch: u64,
    id: u64,
    field: u32,
    picture_revision:u64,
    start: i64,
    end: i64,
    flags: u32,
    format: Format,
    prepared_at: Instant,
}
struct Handoff {
    ready: Mutex<Ready<Prepared>>,
    period: AtomicU64,
    displayed_epoch: AtomicU64,
    displayed_picture: AtomicU64,
    end_frame: AtomicU64,
}
impl Handoff {
    fn new() -> Self {
        Self {
            ready: Mutex::new(Ready::default()),
            period: AtomicU64::new(166_833),
            displayed_epoch: AtomicU64::new(u64::MAX),
            displayed_picture: AtomicU64::new(u64::MAX),
            end_frame: AtomicU64::new(0),
        }
    }
}
struct JoinedWorker {
    state: Arc<Input>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl Drop for JoinedWorker {
    fn drop(&mut self) {
        self.state.quit.store(true, Ordering::Relaxed);
        self.state.space.notify_all();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn failure(state: &Input, folder: &std::path::Path, message: String) {
    // Preserve the actual validation/driver error before shutting down workers.
    // sync_all makes a recoverable error less likely to vanish during a later reset.
    if let Ok(mut file)=std::fs::File::create(folder.join("native-renderer-error.txt")) {
        let _=file.write_all(message.as_bytes());
        let _=file.sync_all();
    }
    state.fail(message);
    state.quit.store(true, Ordering::Relaxed);
    state.space.notify_all();
}
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    let detail=payload.downcast_ref::<String>().map(String::as_str)
        .or_else(||payload.downcast_ref::<&str>().copied()).unwrap_or("non-text panic payload");
    format!("Vulkan presentation worker panicked: {detail}")
}
#[cfg(test)]
mod diagnostics_tests {
    #[test]
    fn presentation_panic_retains_the_underlying_error() {
        assert!(super::panic_message(&String::from("Validation Error: invalid texture")).ends_with("invalid texture"));
        assert!(super::panic_message(&"device lost").ends_with("device lost"));
        assert!(super::panic_message(&42u32).ends_with("non-text panic payload"));
    }
}
fn pause_worker(state: &Input, duration: Duration) {
    let p = state.pending.lock().unwrap();
    if !state.quit.load(Ordering::Relaxed) {
        drop(state.space.wait_timeout(p, duration).unwrap());
    }
}
fn window_size(hwnd: usize) -> (u32, u32) {
    let mut r = RECT::default();
    let _ = unsafe { GetClientRect(HWND(hwnd as _), &mut r) };
    (r.right.max(1) as u32, r.bottom.max(1) as u32)
}
fn aspect(format: Format, selected: crate::aspect::AspectRatio) -> (u32, u32) {
    selected.display((format.width as u32,format.visible_height as u32),format.display_aspect)
}
pub(super) fn run(
    state: Arc<Input>,
    hwnd: usize,
    resolution: a865r::api::Resolution,
    deinterlacing: a865r::api::DeinterlaceMode,
    profile: Option<Arc<crate::icc::Transform>>,
    folder: PathBuf,
    ready_tx: &std::sync::mpsc::SyncSender<
        std::result::Result<Option<Arc<gpu_video::VulkanDevice>>, String>,
    >,
    require_decoding:bool,
    shader:crate::backend::Shader,
) -> Result<()> {
    let mut gpu = pollster::block_on(Gpu::new(hwnd, resolution, profile.clone(),require_decoding))?;
    gpu.deinterlacing = deinterlacing;
    if matches!(shader, crate::backend::Shader::Dx12 | crate::backend::Shader::Dx11 | crate::backend::Shader::Off) {
        gpu.external = Some(super::picture_stage::PictureStage::new(profile.clone(), shader));
    }
    let handoff = Arc::new(Handoff::new());
    *state.report.lock().unwrap() = json!({"backend":"Vulkan","adapter":gpu.adapter,
        "present_mode":"FIFO","vsync":true,"buffering":gpu.buffering,"gpu_readback_per_frame":false,
        "pipeline":"separate GPU preparation and presentation workers","processed_image_slots":3,
        "preparation_thread_id":unsafe{windows::Win32::System::Threading::GetCurrentThreadId()}});
    let stage = SurfaceStage {
        work:Default::default(),
        surface: gpu.surface.take().unwrap(),
        device: gpu.device.clone(),
        queue: gpu.queue.clone(),
        config: gpu.config.clone(),
        configured: false,
        sdr_format:gpu.config.format,hdr_capable:gpu.hdr_capable,hdr_active:false,white:1.,hdr_poll:Instant::now()-Duration::from_secs(2),hdr_requested:false,
        hwnd,
        snapshot_worker: None,
    };
    let ps = state.clone();
    let ph = handoff.clone();
    let pf = folder.clone();
    let worker = std::thread::Builder::new()
        .name("a865r-present".into())
        .spawn(move || {
            let _ = unsafe {
                windows::Win32::System::Com::CoInitializeEx(
                    None,
                    windows::Win32::System::Com::COINIT_MULTITHREADED,
                )
            };
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                present(stage, &ps, &ph, &pf)
            }));
            match result {
                Ok(Ok(())) => {}
                Ok(Err(e)) => failure(&ps, &pf, e.to_string()),
                Err(payload) => failure(&ps, &pf, panic_message(payload.as_ref())),
            }
            unsafe {
                windows::Win32::System::Com::CoUninitialize();
            }
        })
        .map_err(error)?;
    // This guard also joins the presentation thread during panic unwinding.
    let _joined = JoinedWorker {
        state: state.clone(),
        worker: Some(worker),
    };
    let _ = ready_tx.send(Ok(gpu.video_device.clone()));
    let result=prepare(&mut gpu, &state, &handoff, &folder, profile);
    // Stop/join the only other submitter before draining completion callbacks.
    // Those callbacks own image leases: abandoning them can keep GPU resources
    // alive across recording stops and channel changes.
    drop(_joined);
    let drained=gpu.device.poll(wgpu::PollType::Wait {
        submission_index:None,timeout:Some(Duration::from_secs(2)),
    }).map_err(|e|error(format!("GPU completion timed out or failed during shutdown: {e}")));
    if let Err(e)=&drained {state.fail(e.to_string());}
    result.and(drained.map(|_|()))
}
fn prepare(
    gpu: &mut Gpu,
    state: &Arc<Input>,
    handoff: &Arc<Handoff>,
    folder: &std::path::Path,
    profile: Option<Arc<crate::icc::Transform>>,
) -> Result<()> {
    let mut pool: Pool<crate::canvas::Canvas> = Pool::default();
    let mut current: Option<Frame> = None;
    let mut epoch = state.epoch();
    let mut last_key = None;
    let mut last_source = None;
    let mut last_report = Instant::now();
    let mut prepared = 0u64;
    let mut pool_waits = 0u64;
    let start = Instant::now();
    let mut trace = if std::env::args().any(|s| s == "--trace-pacing") {
        std::fs::File::create(folder.join("native-prepare.jsonl")).ok()
    } else {
        None
    };
    while !state.quit.load(Ordering::Relaxed) {
        if !gpu.work.ready(&gpu.device).map_err(error)? {
            pause_worker(state,Duration::from_millis(4));continue;
        }
        let period = handoff.period.load(Ordering::Relaxed) as i64;
        let (now, target, paused, next, selected, picture, revision, hdr_active) = {
            let mut p = state.pending.lock().unwrap();
            if p.epoch != epoch {
                epoch = p.epoch;
                current = None;
                last_key = None;
                last_source = None;
                handoff.end_frame.store(0, Ordering::Relaxed);
                gpu.current_gpu = None;
                gpu.previous_gpu = None;
                gpu.textures = None;
                handoff.ready.lock().unwrap().clear();
            }
            let now = Input::now(&p);
            if p.step_to.is_some(){drop(p);pause_worker(state,Duration::from_millis(4));continue;}
            if let Some(frame)=p.step_frame.take(){current=Some(frame);last_key=None;last_source=None;}
            if !p.accepting
                || (p.paused && handoff.displayed_epoch.load(Ordering::Relaxed) == epoch && handoff.displayed_picture.load(Ordering::Relaxed)==p.picture_revision)
            {
                drop(p);
                pause_worker(state, Duration::from_millis(30));
                continue;
            }
            // At most one source-frame interval of preparation lead. The display
            // selects nearer the clock, so preparation jitter does not choose the
            // displayed field and the queue cannot build up seconds of latency.
            let duration = current
                .as_ref()
                .map(|f| f.end - f.start)
                .unwrap_or(333_667)
                .max(1);
            let target = now + (2 * period).min(duration);
            while !p.stepping && p.frames.front().is_some_and(|f| {
                current.is_none()
                    || (!p.paused
                        && f.start
                            - crate::pacing::sample_margin(
                                f.start,
                                f.end,
                                f.format.interlaced
                                    && gpu.deinterlacing == a865r::api::DeinterlaceMode::DoubleRate,
                            )
                            <= target)
            }) {
                current = p.frames.pop_front();
                state.space.notify_all();
                if p.paused {
                    break;
                }
            }
            (now, target, p.paused, p.frames.front().cloned(), p.aspect,p.picture,p.picture_revision,p.hdr_active)
        };
        let Some(frame) = current.as_ref() else {
            pause_worker(state, Duration::from_millis(8));
            continue;
        };
        let interlaced =
            frame.format.interlaced && gpu.deinterlacing == a865r::api::DeinterlaceMode::DoubleRate;
        let margin = crate::pacing::sample_margin(frame.start, frame.end, interlaced);
        let field = u32::from(
            interlaced && !paused && target >= frame.start + (frame.end - frame.start) / 2 - margin,
        );
        let cap = gpu.resolution.dimensions().unwrap_or((
            frame.format.width as u32,
            frame.format.visible_height as u32,
        ));
        let processing =
            crate::canvas::geometry(aspect(frame.format, selected), gpu.window_size(), cap).1;
        let key = (epoch, frame.id, field, processing.0, processing.1, selected, revision);
        if last_key == Some(key) {
            let p = state.pending.lock().unwrap();
            if p.epoch == epoch && p.eos && p.frames.is_empty() && (!interlaced || field == 1) {
                handoff.end_frame.store(frame.id, Ordering::Relaxed);
            }
            drop(p);
            pause_worker(state, Duration::from_millis(4));
            continue;
        }
        handoff
            .ready
            .lock()
            .unwrap()
            .discard_obsolete(epoch, now + period);
        let slot = pool.acquire(
            |c| c.size == processing,
            || {
                let mut c = gpu.canvas.empty_slot();
                c.ensure(&gpu.device, &gpu.sampler, processing);
                c
            },
        );
        let Some(slot) = slot else {
            // Submission and the presentation worker already progress callbacks.
            // Poll explicitly only when outstanding leases prevent preparation.
            gpu.device.poll(wgpu::PollType::Poll).map_err(error)?;
            pool_waits += 1;
            pause_worker(state, Duration::from_millis(4));
            continue;
        };
        if state.has_failed() {break;}
        gpu.aspect = selected;gpu.picture=picture;gpu.color=profile.is_some() && !hdr_active;
        let began = Instant::now();
        gpu.prepare(
            frame,
            field,
            last_source != Some((epoch, frame.id)),
            slot.clone(),
            next.as_ref(),
        )?;
        last_source = Some((epoch, frame.id));
        prepared += 1;
        let p = state.pending.lock().unwrap();
        if p.epoch == epoch
            && !(p.paused && handoff.displayed_epoch.load(Ordering::Relaxed) == epoch && handoff.displayed_picture.load(Ordering::Relaxed)==p.picture_revision)
        {
            last_key = Some(key);
            let due = frame.start
                + if field == 1 {
                    (frame.end - frame.start) / 2
                } else {
                    0
                }
                - margin;
            handoff.ready.lock().unwrap().publish(Timed {
                epoch,
                due,
                value: Prepared {
                    image: slot,
                    epoch,
                    id: frame.id,
                    field,
                    picture_revision:revision,
                    start: frame.start,
                    end: frame.end,
                    flags: frame.flags,
                    format: frame.format,
                    prepared_at: Instant::now(),
                },
            });
            if p.eos && p.frames.is_empty() && (!interlaced || field == 1) {
                handoff.end_frame.store(frame.id, Ordering::Relaxed);
            }
            state.space.notify_all();
        }
        drop(p);
        if let Some(t) = &mut trace {
            let _ = writeln!(
                t,
                "{}",
                json!({"wall_ms":start.elapsed().as_secs_f64()*1000.,
            "frame":frame.id,"field":field,"audio_clock":now,"target":target,"prepare_ms":began.elapsed().as_secs_f64()*1000.})
            );
        }
        if let Some(p) = profile.as_ref().filter(|_| gpu.external.is_none()) {
            p.frames.fetch_add(1, Ordering::Relaxed);
            p.micros
                .fetch_add(began.elapsed().as_micros() as u64, Ordering::Relaxed);
        }
        if last_report.elapsed() > Duration::from_secs(1) {
            let mut r = state.report.lock().unwrap();
            r["shader_processing"] = gpu.external.as_ref().map(|b| b.report()).unwrap_or_else(|| json!({"enabled":true,"name":"Vulkan"}));
            r["prepared_fields"] = json!(prepared);
            r["picture"] = picture.json();
            r["hdr_effect_strength"] = json!(picture.effect_strength());
            r["picture_revision"] = json!(revision);
            r["processed_fields"] = json!(prepared);
            r["pool_waits"] = json!(pool_waits);
            r["processing_output"] = json!(processing);
            r["deinterlacing"] = json!(gpu.deinterlacing.label());
            r["upscaling"] = json!("GPU hardware linear filtering");
            r["lookahead"] = json!("one source frame of preparation lead; bounded GPU pool");
            last_report = Instant::now();
        }
    }
    Ok(())
}
struct SurfaceStage {
    work: crate::gpu_work::Work,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    configured: bool,
    sdr_format:wgpu::TextureFormat,hdr_capable:bool,hdr_active:bool,white:f32,hdr_poll:Instant,hdr_requested:bool,
    hwnd: usize,
    snapshot_worker: Option<std::thread::JoinHandle<()>>,
}
impl Drop for SurfaceStage {
    fn drop(&mut self) {
        if let Some(worker) = self.snapshot_worker.take() {
            let _ = worker.join();
        }
    }
}
impl SurfaceStage {
    fn update_hdr(&mut self,state:&Input)->bool {
        let requested=state.pending.lock().unwrap().video_hdr;
        if requested==self.hdr_requested && self.hdr_poll.elapsed()<Duration::from_secs(1){return false;}
        self.hdr_requested=requested;self.hdr_poll=Instant::now();
        let display=unsafe{crate::display_hdr::query(HWND(self.hwnd as _))}.ok();
        let active=requested && self.hdr_capable && display.as_ref().is_some_and(|s|s.enabled&&s.supported&&!s.limited);
        let white=display.as_ref().map_or(1.,|s|s.white_scale);
        let changed=active!=self.hdr_active || (active && (white-self.white).abs()>0.001);
        if active!=self.hdr_active {
            self.hdr_active=active;self.config.format=if active{wgpu::TextureFormat::Rgba16Float}else{self.sdr_format};self.configured=false;
            let mut p=state.pending.lock().unwrap();p.hdr_active=active;p.picture_revision=p.picture_revision.wrapping_add(1);state.space.notify_all();
        }
        self.white=white;
        let mut report=state.report.lock().unwrap();report["video_hdr_requested"]=json!(requested);report["video_hdr_active"]=json!(active);report["video_hdr_capable"]=json!(self.hdr_capable);report["output_color_space"]=json!(if active{"scRGB linear"}else{"SDR sRGB"});report["sdr_white_scale"]=json!(white);
        changed
    }

    // A successfully acquired image must consume its acquisition semaphore even
    // if a seek/channel change invalidates the selected picture. Dropping it is
    // not enough: the Vulkan backend has no discard implementation.
    fn clear(&self, output: wgpu::SurfaceTexture, state:&Input) {
        if state.error().is_some() {return;}
        let view = output.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Retire obsolete surface image"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
        }
        if state.error().is_some() {return;}
        let Ok(completion)=self.work.reserve() else {state.fail("GPU submission limit reached".into());return;};
        self.queue.submit(Some(encoder.finish()));
        self.queue.on_submitted_work_done(move || drop(completion));
        output.present();
    }
    fn snapshot_busy(&mut self) -> bool {
        if self
            .snapshot_worker
            .as_ref()
            .is_some_and(|w| w.is_finished())
        {
            if let Some(w) = self.snapshot_worker.take() {
                let _ = w.join();
            }
        }
        self.snapshot_worker.is_some()
    }
    fn acquire(&mut self) -> Result<Option<wgpu::SurfaceTexture>> {
        let size = window_size(self.hwnd);
        if !self.configured || (self.config.width, self.config.height) != size {
            self.config.width = size.0;
            self.config.height = size.1;
            self.surface.configure(&self.device, &self.config);
            self.configured = true;
        }
        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t)
            | wgpu::CurrentSurfaceTexture::Suboptimal(t) => Ok(Some(t)),
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.configured = false;
                Ok(None)
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                Ok(None)
            }
            other => Err(error(format!("Vulkan presentation stopped: {other:?}"))),
        }
    }
    fn draw(
        &mut self,
        frame: &Prepared,
        output: wgpu::SurfaceTexture,
        viewport: [u32; 4],
        snapshot: Option<PathBuf>,
        state: &Arc<Input>,
    ) -> Result<bool> {
        if state.error().is_some() {return Ok(false);}
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let view = output.texture.create_view(&Default::default());
        frame.image.set_white(&self.queue,self.white);
        frame.image.present(&mut encoder, &view, viewport,self.hdr_active);
        let size = (self.config.width, self.config.height);
        let capture =
            if snapshot.is_some() {
                // Keep PNG snapshots SDR regardless of the display swapchain format.
                let capture=self.device.create_texture(&wgpu::TextureDescriptor{label:Some("SDR snapshot"),size:output.texture.size(),mip_level_count:1,sample_count:1,dimension:wgpu::TextureDimension::D2,format:self.sdr_format,usage:wgpu::TextureUsages::RENDER_ATTACHMENT|wgpu::TextureUsages::COPY_SRC,view_formats:&[]});
                let capture_view=capture.create_view(&Default::default());frame.image.present(&mut encoder,&capture_view,viewport,false);
                let pitch = (size.0 * 4).div_ceil(256) * 256;
                let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Requested snapshot only"),
                    size: pitch as u64 * size.1 as u64,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                encoder.copy_texture_to_buffer(
                    capture.as_image_copy(),
                    wgpu::TexelCopyBufferInfo {
                        buffer: &buffer,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(pitch),
                            rows_per_image: None,
                        },
                    },
                    output.texture.size(),
                );
                Some((buffer, pitch))
            } else {
                None
            };
        if state.epoch() != frame.epoch {
            if let Some(path) = snapshot {
                let mut p = state.pending.lock().unwrap();
                if p.snapshot.is_none() {
                    p.snapshot = Some(path);
                }
            }
            drop(encoder);
            self.clear(output,state);
            return Ok(false);
        }
        if state.error().is_some() {return Ok(false);}
        let completion=self.work.reserve().map_err(error)?;
        let submitted = self.queue.submit(Some(encoder.finish()));
        // Holds the slot even if a channel change drops the displayed descriptor.
        let lease = frame.image.clone();
        let wake = state.clone();
        self.queue.on_submitted_work_done(move || {
            drop(lease);drop(completion);
            wake.space.notify_all();
        });
        output.present();
        if let Some(path) = snapshot {
            if let Some((buffer, pitch)) = capture {
                let device = self.device.clone();
                let format = self.sdr_format;
                let state = state.clone();
                self.snapshot_worker = Some(
                    std::thread::Builder::new()
                        .name("a865r-snapshot".into())
                        .spawn(move || {
                            if let Err(e) =
                                save_snapshot(device, buffer, submitted, size, pitch, format, path.clone())
                            {
                                let _=std::fs::write(path.with_extension("error.txt"),e.to_string());
                                state.report.lock().unwrap()["snapshot_error"] =
                                    json!(e.to_string());
                            }
                        })
                        .map_err(error)?,
                );
            } else {
                let _=std::fs::write(path.with_extension("error.txt"),"Vulkan surface does not support snapshots");
                state.report.lock().unwrap()["snapshot_error"] =
                    json!("Vulkan surface does not support snapshots");
            }
        }
        Ok(true)
    }
}
fn save_snapshot(
    device: wgpu::Device,
    buffer: wgpu::Buffer,
    submitted: wgpu::SubmissionIndex,
    size: (u32, u32),
    pitch: u32,
    format: wgpu::TextureFormat,
    path: PathBuf,
) -> Result<()> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submitted),
            timeout: Some(Duration::from_secs(5)),
        })
        .map_err(error)?;
    rx.recv_timeout(Duration::from_secs(5))
        .map_err(error)?
        .map_err(error)?;
    let pixels = buffer.slice(..).get_mapped_range();
    let mut bmp = Vec::with_capacity((54 + size.0 * size.1 * 4) as usize);
    bmp.extend(b"BM");
    bmp.extend((54 + size.0 * size.1 * 4).to_le_bytes());
    bmp.extend([0; 4]);
    bmp.extend(54u32.to_le_bytes());
    bmp.extend(40u32.to_le_bytes());
    bmp.extend(size.0.to_le_bytes());
    bmp.extend((-(size.1 as i32)).to_le_bytes());
    bmp.extend(1u16.to_le_bytes());
    bmp.extend(32u16.to_le_bytes());
    bmp.extend([0; 24]);
    for row in pixels.chunks(pitch as usize) {
        for px in row[..size.0 as usize * 4].chunks_exact(4) {
            if format == wgpu::TextureFormat::Rgba8Unorm {
                bmp.extend([px[2], px[1], px[0], 255]);
            } else {
                bmp.extend_from_slice(px);
            }
        }
    }
    drop(pixels);
    buffer.unmap();
    crate::snapshots::save_bmp(&path,&bmp).map_err(error)
}
fn present(
    mut gpu: SurfaceStage,
    state: &Arc<Input>,
    handoff: &Arc<Handoff>,
    folder: &std::path::Path,
) -> Result<()> {
    state.report.lock().unwrap()["presentation_thread_id"]=json!(unsafe{windows::Win32::System::Threading::GetCurrentThreadId()});
    let mut pacer = crate::pacing::DisplayPacer::new(gpu.hwnd);
    let mut current: Option<Prepared> = None;
    let mut last = None;
    let mut last_size = (0, 0);
    let mut last_aspect = crate::aspect::AspectRatio::default();
    let mut intervals = crate::pacing::Intervals::default();
    let mut last_report = Instant::now();
    let start = Instant::now();
    let mut repeats = 0u64;
    let mut acquisition_started: Option<Instant> = None;
    let mut acquisition_retries = 0u64;
    let mut trace = if std::env::args().any(|s| s == "--trace-pacing") {
        std::fs::File::create(folder.join("native-present.jsonl")).ok()
    } else {
        None
    };
    while !state.quit.load(Ordering::Relaxed) {
        if !gpu.work.ready(&gpu.device).map_err(error)? {
            pause_worker(state,Duration::from_millis(4));continue;
        }
        // queue.submit performs completion maintenance during active playback.
        let hdr_changed=gpu.update_hdr(state);
        let snapshot_busy = gpu.snapshot_busy();
        let (epoch, paused, snapshot_pending, selected, revision) = {
            let p = state.pending.lock().unwrap();
            (p.epoch, p.paused, p.snapshot.is_some(), p.aspect,p.picture_revision)
        };
        if current.as_ref().is_some_and(|f| f.epoch != epoch) {
            current = None;
            last = None;
            handoff.displayed_epoch.store(u64::MAX, Ordering::Relaxed);
        }
        if current.is_none() && handoff.ready.lock().unwrap().len() == 0 {
            gpu.device.poll(wgpu::PollType::Poll).map_err(error)?;
            pause_worker(state, Duration::from_millis(8));
            continue;
        }
        if paused && current.as_ref().is_some_and(|f|f.picture_revision==revision) {
            handoff.ready.lock().unwrap().clear();
        }
        if paused
            && !hdr_changed
            && current.is_some()
            && (!snapshot_pending || snapshot_busy)
            && last_size == window_size(gpu.hwnd)
            && last_aspect == selected
            && current.as_ref().is_some_and(|f|f.picture_revision==revision)
        {
            gpu.device.poll(wgpu::PollType::Poll).map_err(error)?;
            pause_worker(state, Duration::from_millis(50));
            continue;
        }
        let began = Instant::now();
        if acquisition_started.is_none() && !pacer.fullscreen_surface() {
            pacer.wait();
        }
        let vblank_ms = began.elapsed().as_secs_f64() * 1000.;
        if state.quit.load(Ordering::Relaxed) {
            break;
        }
        let acquisition_start = *acquisition_started.get_or_insert_with(Instant::now);
        let Some(output) = gpu.acquire()? else {
            acquisition_retries += 1;
            pause_worker(state, Duration::from_millis(1));
            continue;
        };
        if state.error().is_some() {drop(output);break;}
        let acquire_ms = acquisition_start.elapsed().as_secs_f64() * 1000.;
        acquisition_started = None;
        let (epoch, now, target, paused, eos, selected) = {
            let p = state.pending.lock().unwrap();
            let now = Input::now(&p);
            (
                p.epoch,
                now,
                now + pacer.lead_100ns(),
                p.paused,
                p.eos && p.frames.is_empty(),
                p.aspect,
            )
        };
        handoff
            .period
            .store(pacer.lead_100ns().max(1) as u64, Ordering::Relaxed);
        if current.as_ref().is_some_and(|f| f.epoch != epoch) {
            current = None;
            last = None;
        }
        if !paused || current.is_none() || current.as_ref().is_some_and(|f|f.picture_revision!=revision) {
            if let Some(frame) = handoff
                .ready
                .lock()
                .unwrap()
                .select(epoch, if paused { i64::MAX } else { target })
            {
                current = Some(frame.value);
                state.space.notify_all();
            }
        }
        let Some(frame) = &current else {
            gpu.clear(output,state);
            pause_worker(state, Duration::from_millis(4));
            continue;
        };
        let snapshot = if !snapshot_busy {
            state.pending.lock().unwrap().snapshot.take()
        } else {
            None
        };
        let viewport = crate::canvas::geometry(
            aspect(frame.format, selected),
            (gpu.config.width, gpu.config.height),
            frame.image.size,
        )
        .0;
        let draw_started = Instant::now();
        if !gpu.draw(frame, output, viewport, snapshot, state)? {
            continue;
        }
        let key = (frame.epoch, frame.id, frame.field);
        if last == Some(key) {
            repeats += 1;
        }
        last = Some(key);
        last_size = (gpu.config.width, gpu.config.height);
        last_aspect = selected;
        handoff.displayed_epoch.store(epoch, Ordering::Relaxed);
        handoff.displayed_picture.store(frame.picture_revision,Ordering::Relaxed);
        {let mut p=state.pending.lock().unwrap();p.displayed_start=frame.start;p.displayed_end=frame.end;}
        state.presented.fetch_add(1, Ordering::Relaxed);
        intervals.tick();
        if let Some(trace) = &mut trace {
            let _ = writeln!(
                trace,
                "{}",
                json!({"wall_ms":start.elapsed().as_secs_f64()*1000.,
            "frame":frame.id,"field":frame.field,"start":frame.start,"end":frame.end,"flags":frame.flags,"audio_clock":now,"target":target,
            "draw_ms":draw_started.elapsed().as_secs_f64()*1000.,"vblank_ms":vblank_ms,"acquire_ms":acquire_ms,
            "prepared_age_ms":frame.prepared_at.elapsed().as_secs_f64()*1000.})
            );
        }
        let queued = handoff.ready.lock().unwrap().len();
        if eos
            && handoff.end_frame.load(Ordering::Relaxed) == frame.id
            && queued == 0
            && !paused
            && now >= frame.end
        {
            state.finished.store(true, Ordering::Relaxed);
        }
        if last_report.elapsed() > Duration::from_secs(1) {
            let mut r = state.report.lock().unwrap();
            r["source"] = json!([frame.format.width, frame.format.visible_height]);
            r["display_aspect"] = json!(frame.format.display_aspect);
            r["effective_display_aspect"] = json!(aspect(frame.format,selected));
            r["horizontal_aperture"] = json!(selected.aperture((frame.format.width as u32,frame.format.visible_height as u32),frame.format.display_aspect));
            r["interlaced"] = json!(frame.format.interlaced);
            r["top_field_first"] = json!(frame.format.top_first);
            r["sample_flags"] = json!(frame.flags);
            r["frame_duration_ms"] = json!(frame.format.frame_time as f64 / 10000.);
            r["color_matrix"] = json!(if frame.format.bt709 {
                "BT.709"
            } else {
                "BT.601"
            });
            r["nominal_range"] = json!(if frame.format.full { "full" } else { "limited" });
            r["clock_lateness_ms"] = json!((now - frame.start) as f64 / 10000.);
            r["prepared_age_ms"] = json!(frame.prepared_at.elapsed().as_secs_f64() * 1000.);
            r["output"] = json!(viewport);
            r["aspect_ratio"] = json!(selected.name());
            r["handoff_frames"] = json!(queued);
            r["presentation_intervals"] = intervals.report();
            r["display_refresh_hz"] = json!(1. / pacer.period.as_secs_f64());
            r["vblank_wait"] = json!(pacer.hardware_wait);
            r["missed_refreshes"] = json!(pacer.missed_refreshes);
            r["pacing"] = json!(if pacer.fullscreen_surface() {
                "Vulkan FIFO"
            } else {
                "display vblank"
            });
            r["acquisition_retries"] = json!(acquisition_retries);
            r["repeat_presentations"] = json!(repeats);
            last_report = Instant::now();
        }
    }
    Ok(())
}

#[cfg(test)]
pub(super) struct Fixture {
    stage: SurfaceStage,
}
#[cfg(test)]
impl Fixture {
    pub(super) fn new(gpu: &mut Gpu) -> Self {
        Self {
            stage: SurfaceStage {
                work:Default::default(),
                surface: gpu.surface.take().unwrap(),
                device: gpu.device.clone(),
                queue: gpu.queue.clone(),
                config: gpu.config.clone(),
                configured: false,
        sdr_format:gpu.config.format,hdr_capable:gpu.hdr_capable,hdr_active:false,white:1.,hdr_poll:Instant::now()-Duration::from_secs(2),hdr_requested:false,
                hwnd: gpu.hwnd,
                snapshot_worker: None,
            },
        }
    }
    pub(super) fn draw(
        &mut self,
        gpu: &mut Gpu,
        frame: &Frame,
        field: u32,
        new: bool,
        snapshot: Option<PathBuf>,
        next: Option<&Frame>,
        state: &Arc<Input>,
    ) -> Result<()> {
        let mut image = gpu.canvas.empty_slot();
        image.ensure(&gpu.device, &gpu.sampler, window_size(gpu.hwnd));
        let image = Arc::new(image);
        gpu.prepare(frame, field, new, image.clone(), next)?;
        let prepared = Prepared {
            image,
            epoch: frame.epoch,
            id: frame.id,
            field,
            picture_revision:0,
            start: frame.start,
            end: frame.end,
            flags: frame.flags,
            format: frame.format,
            prepared_at: Instant::now(),
        };
        let output = self
            .stage
            .acquire()?
            .ok_or_else(|| error("No test surface image"))?;
        let viewport = [0, 0, self.stage.config.width, self.stage.config.height];
        self.stage
            .draw(&prepared, output, viewport, snapshot, state)?;
        if let Some(w) = self.stage.snapshot_worker.take() {
            w.join().unwrap();
        }
        Ok(())
    }
}
