//! Native Windows demultiplexing, Microsoft decoding and EVR presentation.
//! No external executable, codec pack, graph auto-rendering or COM registration.
use a865r_media::{playback::Control, television};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::{atomic::Ordering, mpsc},
    time::Duration,
};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        Media::{Audio::WAVEFORMATEX, DirectShow::*, MediaFoundation::*},
        System::Com::*,
    },
};

const VIDEO_DECODER: GUID = GUID::from_u128(0x212690fb_83e5_4526_8fd7_74478b7939cd);
const AUDIO_DECODER: GUID = GUID::from_u128(0xe1f1a0b8_beee_490d_ba7c_066c40b5e2b9);
const DSOUND: GUID = GUID::from_u128(0x79376820_07d0_11cf_a24d_0020afd79767);
pub fn microsoft_available() -> bool {
    static AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *AVAILABLE.get_or_init(|| std::thread::spawn(|| unsafe {
        if CoInitializeEx(None,COINIT_MULTITHREADED).is_err() {return false;}
        let available=[CLSID_FilterGraph,CLSID_MPEG2Demultiplexer,VIDEO_DECODER,AUDIO_DECODER,CLSID_EnhancedVideoRenderer]
            .iter().all(|id| CoCreateInstance::<_,IUnknown>(id,None,CLSCTX_INPROC_SERVER).is_ok());
        CoUninitialize(); available
    }).join().unwrap_or(false))
}
pub enum Command {
    Picture(crate::picture::Picture),
    VideoHdr(bool),
    AspectRatio(crate::aspect::AspectRatio),
    Pause,
    Volume(u32),
    Resize,
    Repaint,
    Snapshot(PathBuf),
    Seek(f64),
    Step(i8),
    GoLive,
    AudioMode(crate::audio::Mode),
    AudioTrack(u16),
}
unsafe fn pin(filter: &IBaseFilter, direction: PIN_DIRECTION) -> Result<IPin> {
    let e = filter.EnumPins()?;
    loop {
        let mut p = [None];
        if e.Next(&mut p, None).is_err() {
            break;
        }
        if let Some(p) = p[0].take() {
            if p.QueryDirection()? == direction {
                return Ok(p);
            }
        }
    }
    Err(Error::new(E_FAIL, "Required filter pin is missing"))
}
fn fail(stage: &str, e: Error) -> Error {
    Error::new(e.code(), format!("{stage}: {e}"))
}
unsafe fn add(graph: &IGraphBuilder, clsid: &GUID, name: &str) -> Result<IBaseFilter> {
    let f: IBaseFilter =
        CoCreateInstance(clsid, None, CLSCTX_INPROC_SERVER).map_err(|e| fail(name, e))?;
    graph.AddFilter(&f, &HSTRING::from(name))?;
    Ok(f)
}
unsafe fn connect(
    graph: &IGraphBuilder,
    a: &IBaseFilter,
    b: &IBaseFilter,
    stage: &str,
) -> Result<()> {
    graph
        .ConnectDirect(&pin(a, PINDIR_OUTPUT)?, &pin(b, PINDIR_INPUT)?, None)
        .map_err(|e| fail(stage, e))
}

fn read_sample(path: &Path) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut v = Vec::new();
    std::fs::File::open(path)?
        .take(16 * 1024 * 1024)
        .read_to_end(&mut v)?;
    Ok(v)
}
// Only complete metadata for the explicitly selected service can bypass probing.
fn saved_service(hint: Option<Value>, frequency: u32, program: Option<u32>) -> Option<Value> {
    let service = hint?;
    let id = service["program_id"].as_u64()?;
    let pid = |key: &str| {
        service[key]
            .as_u64()
            .is_some_and(|p| (32..8191).contains(&p))
    };
    if service["frequency_khz"].as_u64() != Some(frequency as u64)
        || program.map(u64::from) != Some(id)
        || !(1..=65535).contains(&id)
        || service["video_type"] != 0x1b
        || !pid("video_pid")
        || (!service["pcr_pid"].is_null() && !pid("pcr_pid"))
        || (!service["audio_pid"].is_null()
            && (!pid("audio_pid") || !matches!(service["audio_type"].as_u64(), Some(0x0f | 0x11))))
    {
        return None;
    }
    Some(service)
}

fn choose_service(services: &[Value], program: Option<u32>) -> Option<&Value> {
    services.iter().find(|s| {
        s["video_type"] == 0x1b
            && program.is_none_or(|id| s["program_id"].as_u64() == Some(id as u64))
    })
}
// A slow first keyframe is normal on some broadcasts. Reopening the receiver
// here would discard parser progress and start the same wait all over again.
fn should_probe_again(cached: bool, elapsed: Duration, video_packets: u64, failed: bool) -> bool {
    cached && elapsed >= Duration::from_secs(8) && video_packets == 0 && !failed
}

fn startup_stall(elapsed: Duration, packets: u64) -> Option<&'static str> {
    if packets == 0 && elapsed >= Duration::from_secs(15) {
        Some("Unable to start video for this service. Choose another channel or run channel discovery.")
    } else if elapsed >= Duration::from_secs(30) {
        Some("Video data arrived, but no decodable picture arrived within 30 seconds. Try this channel again.")
    } else {
        None
    }
}

fn cancellable_probe(
    control: &Control,
    probe: &Control,
    job: impl FnOnce() -> Value + Send,
) -> Value {
    if control.cancel.load(Ordering::Relaxed) {
        return json!({"stopped":true});
    }
    std::thread::scope(|scope| {
        let worker = scope.spawn(job);
        while !worker.is_finished() {
            if control.cancel.load(Ordering::Relaxed) {
                probe.cancel.store(true, Ordering::Relaxed);
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        worker
            .join()
            .unwrap_or_else(|_| json!({"success":false,"error":"Service discovery worker stopped"}))
    })
}

fn probe_services(frequency: u32, folder: &Path, control: &Control) -> Result<Vec<u8>> {
    control.status("Reading broadcast TV services…");
    let probe = Control::default();
    let child = probe.clone();
    let probe_folder = folder.join("probe");
    let result = cancellable_probe(control, &probe, move || {
        television::run(
            television::Action::Record {
                frequency,
                seconds: 3,
            },
            PathBuf::new(),
            probe_folder,
            Default::default(),
            child,
        )
    });
    if control.cancel.load(Ordering::Relaxed) {
        return Ok(Vec::new());
    }
    if result["success"] != true {
        return Err(Error::new(E_FAIL, result.to_string()));
    }
    let recorded = folder.join("probe/recording.ts");
    let data = read_sample(&recorded).map_err(|e| Error::new(E_FAIL, e.to_string()))?;
    let _ = std::fs::rename(recorded, folder.join("native-probe.ts"));
    Ok(data)
}

pub fn run(
    file: Option<PathBuf>,
    frequency: u32,
    program: Option<u32>,
    surface: usize,
    folder: PathBuf,
    control: Control,
    commands: &mpsc::Receiver<Command>,
    volume: u32,
    resolution: a865r::api::Resolution,
    deinterlacing: a865r::api::DeinterlaceMode,
    audio_mode: crate::audio::Mode,
    color: a865r::api::ColorProfile,
    growing: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    service_hint: Option<Value>,
    backend: crate::backend::Backend,
    shader: crate::backend::Shader,
) -> Value {
    if a865r::transport::wine_bridge::is_wine() {
        return crate::wine_playback::run(file,frequency,program,surface,folder,control,commands,volume,deinterlacing,growing.is_some(),service_hint,resolution,color);
    }
    let _owner=PLAYBACK_OWNER.lock().unwrap_or_else(|e|e.into_inner());
    let mut resume=Resume {position:0.,paused:false,volume,audio_mode,aspect:crate::aspect::AspectRatio::Auto,picture:Default::default(),video_hdr:false};
    recover_playback(&control,&folder,backend==crate::backend::Backend::Microsoft || std::env::args().any(|s|s=="--windows-renderer"),|windows,attempt_folder| {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_once(file.clone(),frequency,program,surface,attempt_folder.to_owned(),
                control.clone(),commands,resolution,deinterlacing,color.clone(),growing.clone(),
                service_hint.clone(),windows,shader,&mut resume)
        })).unwrap_or_else(|payload| {
            let detail=payload.downcast_ref::<String>().map(String::as_str)
                .or_else(||payload.downcast_ref::<&str>().copied()).unwrap_or("Unknown native playback panic");
            json!({"success":false,"error":format!("Native playback stopped: {detail}")})
        })
    })
}
// Keep Vulkan as the normal decoder. Explicit software/Windows diagnostics
// take precedence regardless of argument order; recovery behavior is unchanged.
fn use_vulkan_decoder(args: impl IntoIterator<Item = impl AsRef<str>>) -> bool {
    !args.into_iter().any(|arg| matches!(arg.as_ref(), "--software-decoder" | "--windows-renderer"))
}
static PLAYBACK_OWNER: std::sync::Mutex<()> = std::sync::Mutex::new(());
struct Resume {
    position:f64, paused:bool, volume:u32, audio_mode:crate::audio::Mode,
    aspect:crate::aspect::AspectRatio,
    picture:crate::picture::Picture,
    video_hdr:bool,
}
pub(crate) fn graphics_lost(result:&Value)->bool {
    let e=result["error"].as_str().unwrap_or("").to_ascii_lowercase();
    // Errors cross the native graph boundary as text. Only explicit device-loss
    // reports justify retiring the device and attempting a fresh Vulkan device.
    ["device has been lost","device is lost","device lost","devicelost","error_device_lost","dxgi_error_device_removed","dxgi_error_device_reset","0x887a0005","0x887a0007"]
        .iter().any(|needle|e.contains(needle))
}
fn write_recovery(folder:&Path,report:&Value) {
    use std::io::Write;
    if let Ok(mut file)=std::fs::File::create(folder.join("native-recovery.json")) {
        let _=file.write_all(report.to_string().as_bytes());let _=file.sync_all();
    }
}
fn recover_playback(
    control:&Control,folder:&Path,initial_windows:bool,
    mut attempt:impl FnMut(bool,&Path)->Value,
)->Value {
    const RETRIES: usize = 5;
    if control.cancel.load(Ordering::Relaxed) {return json!({"stopped":true});}
    let mut result=attempt(initial_windows,folder);
    if !graphics_lost(&result) {return result;}
    let backend=if initial_windows {"Microsoft"} else {"Vulkan"};
    let mut report=json!({"trigger":result,"backend":backend,"attempts":0,"state":"cancelled","results":[]});
    for retry in 1..=RETRIES {
        // Each attempt returned and released its graph before a fresh device opens.
        // Stop/channel change interrupts the short backoff without consuming retries.
        control.status("Graphics device reset — reconnecting playback…");
        for _ in 0..20 {
            if control.cancel.load(Ordering::Relaxed) {write_recovery(folder,&report);return json!({"stopped":true});}
            std::thread::sleep(Duration::from_millis(50));
        }
        if control.cancel.load(Ordering::Relaxed) {write_recovery(folder,&report);return json!({"stopped":true});}
        let recovery=folder.join(format!("recovery-{}-{retry}",backend.to_ascii_lowercase()));
        if let Err(e)=std::fs::create_dir_all(&recovery) {result=json!({"error":e.to_string()});break;}
        report["state"]=json!("reconnecting");report["attempts"]=json!(retry);write_recovery(folder,&report);
        result=attempt(initial_windows,&recovery);
        report["results"].as_array_mut().unwrap().push(result.clone());
        if !graphics_lost(&result) {break;}
    }
    report["state"]=json!(if result["error"].is_string() {"failed"} else if result["stopped"]==true {"stopped"} else {"completed"});
    report["result"]=result.clone();write_recovery(folder,&report);
    if let Some(detail)=result["error"].as_str() {
        result["error"]=json!(format!("{backend} recovery failed. Playback stopped: {detail}"));
    }
    result["graphics_recovery"]=report;
    result
}

fn run_once(
    file:Option<PathBuf>,frequency:u32,program:Option<u32>,surface:usize,folder:PathBuf,
    control:Control,commands:&mpsc::Receiver<Command>,resolution:a865r::api::Resolution,
    deinterlacing:a865r::api::DeinterlaceMode,color:a865r::api::ColorProfile,
    growing:Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,service_hint:Option<Value>,
    microsoft_decoder:bool,shader:crate::backend::Shader,resume:&mut Resume,
)->Value {
    let hardware_decoder = !microsoft_decoder && use_vulkan_decoder(std::env::args());
    let startup = std::time::Instant::now();
    let result = (|| -> Result<Value> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
            struct ComGuard;
            impl Drop for ComGuard {
                fn drop(&mut self) {
                    unsafe {
                        CoUninitialize();
                    }
                }
            }
            let _com = ComGuard;
            if control.cancel.load(Ordering::Relaxed) {
                return Ok(json!({"stopped":true}));
            }
            let cached = if file.is_none() {
                saved_service(service_hint, frequency, program)
            } else {
                None
            };
            let cache_used = cached.is_some();
            let sample = if let Some(path) = &file {
                read_sample(path).map_err(|e| Error::new(E_FAIL, e.to_string()))?
            } else if cache_used {
                Vec::new()
            } else {
                probe_services(frequency, &folder, &control)?
            };
            if control.cancel.load(Ordering::Relaxed) {
                return Ok(json!({"stopped":true}));
            }
            let services = if let Some(service) = cached {
                vec![service]
            } else {
                television::discover_ts(&sample, frequency)
            };
            std::fs::write(
                folder.join("native-services.json"),
                json!(services).to_string(),
            )
            .ok();
            let mut service=choose_service(&services,program).cloned().ok_or_else(||
                Error::new(E_FAIL,"The selected H.264 TV service was not found. Run channel discovery to refresh the channel list."))?;
            control.set_service(service.clone());
            let mut startup_report = json!({"service_source":if cache_used {"saved_scan"}else{"broadcast_probe"},
                "frequency_khz":frequency,"program_id":service["program_id"],"service_ready_ms":startup.elapsed().as_millis()});
            let epg_observer = crate::epg_source::observe(&sample, frequency, control.clone());
            crate::parental::check(&control.snapshot()["epg"],frequency,service["program_id"].as_u64().unwrap_or(0) as u32).map_err(|e|Error::new(E_ACCESSDENIED,e))?;
            let mut index = if let Some(path) = &file {
                Some(
                    crate::timeline::Index::open(
                        path,
                        service["pcr_pid"]
                            .as_u64()
                            .unwrap_or(service["video_pid"].as_u64().unwrap())
                            as u16,
                    )
                    .map_err(|e| Error::new(E_FAIL, e.to_string()))?,
                )
            } else {
                None
            };
            if let Some(index) = &mut index {
                index
                    .update()
                    .map_err(|e| Error::new(E_FAIL, e.to_string()))?;
            }
            let profile = match &color {
                a865r::api::ColorProfile::File(path) => Some(path.clone()),
                a865r::api::ColorProfile::Monitor => {
                    crate::icc::monitor_profile(HWND(surface as _))
                }
                a865r::api::ColorProfile::Disabled => None,
            };
            let mut color_transform = profile
                .as_ref()
                .map(|path| crate::icc::Transform::load(path).map(std::sync::Arc::new))
                .transpose()
                .map_err(|e| fail("ICC profile", e))?;
            let mut point = index.as_ref().map(|i|i.seek(resume.position)).unwrap_or_default();
            let aspect_ratio=std::cell::Cell::new(resume.aspect);
            let mut frame_target:Option<(f64,i8)>=None;
            let mut paused = resume.paused;
            let mut volume = resume.volume;
            let mut audio_mode=resume.audio_mode;
            'playback: loop {
                if control.cancel.load(Ordering::Relaxed) {
                    break;
                }
                // Reverse declaration order keeps the device alive until the graph
                // and decoder filters have stopped and joined their workers.
                use crate::backend::Shader;
                let shader=shader.compatible(if hardware_decoder {crate::backend::Backend::Vulkan}else{crate::backend::Backend::Microsoft},crate::backend::capabilities());
                let force_windows=std::env::args().any(|s|s=="--windows-renderer");
                let want_vulkan=hardware_decoder || (!force_windows && (matches!(shader,Shader::Auto|Shader::Vulkan) || (matches!(shader,Shader::Dx12|Shader::Dx11) && crate::backend::capabilities().vulkan)));
                let presenter=if want_vulkan {
                    match crate::vulkan::Presenter::new(surface,resolution,deinterlacing,color_transform.clone(),folder.clone(),hardware_decoder,shader) {
                        Ok(p)=>Some(p),
                        Err(e) if hardware_decoder || shader==Shader::Vulkan || graphics_lost(&json!({"error":e.to_string()}))=>return Err(e),
                        Err(e)=>{let _=std::fs::write(folder.join("shader-selection.json"),json!({"vulkan_unavailable":e.to_string(),"next":"DirectX 12 / DirectX 11"}).to_string());None}
                    }
                } else {None};
                let windows_renderer=presenter.is_none();
                if windows_renderer {
                    let transform=color_transform.get_or_insert_with(||std::sync::Arc::new(crate::icc::Transform::identity()));
                    transform.set_shader(shader);transform.set_picture(resume.picture);
                }
                let graph: IGraphBuilder =
                    CoCreateInstance(&CLSID_FilterGraph, None, CLSCTX_INPROC_SERVER)?;
                let media: IMediaControl = graph.cast()?;
                struct Stop(IMediaControl,Option<std::sync::Arc<crate::vulkan::Input>>);
                impl Drop for Stop {
                    fn drop(&mut self) {
                        unsafe {
                            if let Some(input)=&self.1 {input.shutdown();}
                            let _ = self.0.Stop();
                        }
                    }
                }
                let _stop = Stop(media.clone(),presenter.as_ref().map(|p|p.input.clone()));
                let replay = if let Some(path) = file.clone() {
                    Some(a865r_bda::Replay {
                        path,
                        offset: point.offset,
                        growing: growing.clone().unwrap_or_default(),
                    })
                } else {
                    None
                };
                let clock_recovery = service["pcr_pid"].as_u64().and_then(|pcr| {
                    a865r_bda::clock_recovery::ClockRecoveryConfig::new(
                        service["video_pid"].as_u64()? as u16,
                        u16::try_from(pcr).ok()?,
                    )
                });
                let source =
                    a865r_bda::create_clocked_source(replay, epg_observer.clone(), clock_recovery);
                graph.AddFilter(&source, w!("A865R Rust stream source"))?;
                let demux = add(
                    &graph,
                    &CLSID_MPEG2Demultiplexer,
                    "Windows transport demultiplexer",
                )?;
                connect(&graph, &source, &demux, "Transport source to demultiplexer")?;
                // The source already captures on its own bounded worker. Begin reception
                // before renderer setup so a keyframe arriving during setup is retained.
                if file.is_none() {
                    let tuner: IBDA_FrequencyFilter = source.cast()?;
                    let changes: IBDA_DeviceControl = source.cast()?;
                    changes.StartChanges()?;
                    tuner.SetFrequencyMultiplier(1000)?;
                    tuner.SetFrequency(frequency)?;
                    tuner.SetBandwidth(6)?;
                    changes.CheckChanges()?;
                    changes.CommitChanges()?;
                }
                startup_report["receiver_ready_ms"] = json!(startup.elapsed().as_millis());
                let map: IMpeg2Demultiplexer = demux.cast()?;
                let mut vi = VIDEOINFOHEADER2::default();
                vi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                vi.bmiHeader.biWidth = 1920;
                vi.bmiHeader.biHeight = 1080;
                vi.bmiHeader.biPlanes = 1;
                vi.bmiHeader.biBitCount = 24;
                vi.bmiHeader.biCompression = u32::from_le_bytes(*b"H264");
                vi.AvgTimePerFrame = 333667;
                vi.dwPictAspectRatioX = 16;
                vi.dwPictAspectRatioY = 9;
                let vmt = AM_MEDIA_TYPE {
                    majortype: MEDIATYPE_Video,
                    subtype: MEDIASUBTYPE_H264,
                    bTemporalCompression: true.into(),
                    formattype: FORMAT_VideoInfo2,
                    cbFormat: std::mem::size_of_val(&vi) as u32,
                    pbFormat: (&mut vi as *mut VIDEOINFOHEADER2).cast(),
                    ..Default::default()
                };
                let vpin = map.CreateOutputPin(&vmt, w!("H264 television"))?;
                let pidmap: IMPEG2PIDMap = vpin.cast()?;
                pidmap.MapPID(
                    1,
                    &(service["video_pid"].as_u64().unwrap() as u32),
                    MEDIA_ELEMENTARY_STREAM,
                )?;
                if control.cancel.load(Ordering::Relaxed) {
                    break;
                }
                startup_report["renderer_ready_ms"] = json!(startup.elapsed().as_millis());
                let mut evr_quality:Option<IQualProp>=None;
                let mut evr_repaints = 0u64;
                let display = if let Some(presenter) = &presenter {
                    if !hardware_decoder {
                        let decoder =
                            add(&graph, &VIDEO_DECODER, "Microsoft DTV-DVD Video Decoder")?;
                        graph.ConnectDirect(&vpin, &pin(&decoder, PINDIR_INPUT)?, None)?;
                        let renderer = a865r_bda::create_video_filter(presenter.input.clone());
                        graph.AddFilter(&renderer, w!("A865R Vulkan video renderer"))?;
                        connect(
                            &graph,
                            &decoder,
                            &renderer,
                            "Windows software decoder to Vulkan",
                        )?;
                        let report=json!({"backend":"Microsoft DTV-DVD Video Decoder",
                            "hardware_decoding":false,"presentation":"Vulkan",
                            "selection":"Microsoft decoder with Vulkan shader acceleration"});
                        presenter.input.report.lock().unwrap()["decoder"]=report.clone();
                        let _=std::fs::write(folder.join("native-decoder.json"),report.to_string());
                    } else {
                        let hardware = crate::vkdecode::Filter::new(
                            presenter.device.clone().ok_or_else(||Error::new(E_FAIL,"Missing Vulkan decoding device"))?,
                            presenter.input.clone(),
                            folder.clone(),
                        )
                        .map_err(|e| fail("Vulkan Video H.264 decoder", e))?;
                        let renderer = a865r_bda::create_video_filter(hardware);
                        graph.AddFilter(
                            &renderer,
                            w!("A865R Vulkan Video H.264 decoder and Vulkan renderer"),
                        )?;
                        graph
                            .ConnectDirect(&vpin, &pin(&renderer, PINDIR_INPUT)?, Some(&vmt))
                            .map_err(|e| fail("H264 to Vulkan decoder", e))?;
                    }
                    None
                } else {
                    let decoder = add(&graph, &VIDEO_DECODER, "Microsoft DTV-DVD Video Decoder")?;
                    graph.ConnectDirect(&vpin, &pin(&decoder, PINDIR_INPUT)?, None)?;
                    let evr = add(
                        &graph,
                        &CLSID_EnhancedVideoRenderer,
                        "Windows GPU video renderer",
                    )?;
                    evr_quality=evr.cast().ok();
                    let service_get: IMFGetService = evr.cast()?;
                    let display: IMFVideoDisplayControl =
                        service_get.GetService(&MR_VIDEO_RENDER_SERVICE)?;
                    display.SetVideoWindow(HWND(surface as _))?;
                    display.SetAspectRatioMode(MFVideoARMode_None.0 as u32)?;
                    if let Some(transform) = &color_transform {
                        let color_filter = a865r_bda::create_video_filter(transform.clone());
                        graph.AddFilter(&color_filter, w!("A865R Direct3D picture and ICC transform"))?;
                        connect(
                            &graph,
                            &decoder,
                            &color_filter,
                            "Microsoft decoder to ICC transform",
                        )?;
                        connect(&graph, &color_filter, &evr, "ICC transform to GPU renderer")?;
                    } else {
                        connect(&graph, &decoder, &evr, "Microsoft decoder to GPU renderer")?;
                    }
                    Some(display)
                };
                let mut captions=crate::captions::Engine::new(control.clone());
                let caption_type=AM_MEDIA_TYPE{majortype:MEDIATYPE_Stream,formattype:FORMAT_None,..Default::default()};
                let caption_pin=map.CreateOutputPin(&caption_type,w!("ISDB closed captions"))?;
                let caption_map:IMPEG2PIDMap=caption_pin.cast()?;
                let caption_filter=a865r_bda::create_video_filter(captions.sink.clone());
                graph.AddFilter(&caption_filter,w!("ISDB caption sink"))?;
                graph.ConnectDirect(&caption_pin,&pin(&caption_filter,PINDIR_INPUT)?,None)?;
                let mut caption_pid:Option<u16>=None;
                let mixer=std::sync::Arc::new(crate::audio::Mixer::new(audio_mode));
                control.set_audio_output(audio_mode.name(),service["audio_pid"].as_u64().map(|p|p as u16));
                let frame_only=frame_target.is_some();
                let mut audio_codec:Option<ICodecAPI>=None;
                let mut multichannel_configured=false;
                if let Some(pid) = service["audio_pid"].as_u64().filter(|_|!frame_only) {
                    let latm = service["audio_type"] == 0x11;
                    let mut wave = WAVEFORMATEX {
                        wFormatTag: if latm { 0x1602 } else { 0x1600 },
                        nChannels: 2,
                        nSamplesPerSec: 48000,
                        nAvgBytesPerSec: 16000,
                        nBlockAlign: 1,
                        wBitsPerSample: 0,
                        cbSize: 0,
                    };
                    let amt = AM_MEDIA_TYPE {
                        majortype: MEDIATYPE_Audio,
                        subtype: if latm {
                            MEDIASUBTYPE_MPEG_LOAS
                        } else {
                            MEDIASUBTYPE_MPEG_ADTS_AAC
                        },
                        bTemporalCompression: true.into(),
                        formattype: FORMAT_WaveFormatEx,
                        cbFormat: std::mem::size_of_val(&wave) as u32,
                        pbFormat: (&mut wave as *mut WAVEFORMATEX).cast(),
                        ..Default::default()
                    };
                    let apin = map.CreateOutputPin(&amt, w!("AAC television audio"))?;
                    let p: IMPEG2PIDMap = apin.cast()?;
                    p.MapPID(1, &(pid as u32), MEDIA_ELEMENTARY_STREAM)?;
                    let adec = add(&graph, &AUDIO_DECODER, "Microsoft DTV-DVD Audio Decoder")?;
                    multichannel_configured=crate::audio::configure_decoder(&adec).is_ok();
                    audio_codec=pin(&adec,PINDIR_INPUT)?.cast::<ICodecAPI>().ok();
                    graph
                        .ConnectDirect(&apin, &pin(&adec, PINDIR_INPUT)?, None)
                        .map_err(|e| fail("AAC demultiplexer to Microsoft decoder", e))?;
                    let audio = add(&graph, &DSOUND, "Windows audio output")?;
                    let mix_filter=a865r_bda::create_video_filter(mixer.clone());
                    graph.AddFilter(&mix_filter,w!("A865R PCM sound modes"))?;
                    connect(&graph,&adec,&mix_filter,"Audio decoder to sound mode")?;
                    connect(&graph,&mix_filter,&audio,"Sound mode to speakers")?;
                    let clock: windows::Win32::Media::IReferenceClock = audio.cast()?;
                    graph.cast::<IMediaFilter>()?.SetSyncSource(&clock)?;
                }
                let audio: IBasicAudio = graph.cast()?;
                let set_volume = |v: u32| {
                    if frame_only {return Ok(());}
                    audio.SetVolume(if v == 0 {
                        -10000
                    } else {
                        (2000. * (v as f64 / 100.).log10()) as i32
                    })
                };
                set_volume(volume)?;
                let evr_format=std::cell::Cell::new(None);
                let resize = || -> Result<()> {
                    let Some(display) = &display else {
                        return Ok(());
                    };
                    let mut r = RECT::default();
                    windows::Win32::UI::WindowsAndMessaging::GetClientRect(
                        HWND(surface as _),
                        &mut r,
                    )?;
                    let mut video_size = SIZE { cx: 1920, cy: 1080 };
                    let mut aspect = SIZE::default();
                    if display.GetNativeVideoSize(&mut video_size, &mut aspect).is_ok() {
                        evr_format.set(Some((video_size.cx,video_size.cy,aspect.cx,aspect.cy)));
                    }
                    control
                        .set_video_size(video_size.cx.max(1) as u32, video_size.cy.max(1) as u32);
                    let source=(video_size.cx.max(1) as u32,video_size.cy.max(1) as u32);
                    let auto=if aspect.cx>0&&aspect.cy>0 {(aspect.cx as u32,aspect.cy as u32)}else{source};
                    let (cap_w,cap_h)=resolution.dimensions().unwrap_or(source);
                    let ([left,top,width,height],_)=crate::canvas::geometry(aspect_ratio.get().display(source,auto),(r.right.max(1) as u32,r.bottom.max(1) as u32),(cap_w,cap_h));
                    let (left,top,width,height)=(left as i32,top as i32,width as i32,height as i32);
                    let target = RECT {
                        left,
                        top,
                        right: left + width,
                        bottom: top + height,
                    };
                    let (crop,aperture_width)=aspect_ratio.get().aperture(source,auto);
                    let aperture=MFVideoNormalizedRect {left:crop as f32/source.0 as f32,top:0.,
                        right:(crop+aperture_width) as f32/source.0 as f32,bottom:1.};
                    display.SetVideoPosition(&aperture, &target)?;
                    let _=std::fs::write(folder.join("native-output.json"),json!({"source":[video_size.cx,video_size.cy],"requested_maximum":[cap_w,cap_h],"presented":[width,height],"backend":"Windows EVR GPU scaling","native_size_no_enlargement":resolution==a865r::api::Resolution::Native}).to_string());
                    Ok(())
                };
                if control.cancel.load(Ordering::Relaxed) {
                    break;
                }
                if let Some(presenter)=&presenter { presenter.input.set_aspect_ratio(aspect_ratio.get());presenter.input.set_picture(resume.picture);presenter.input.set_video_hdr(resume.video_hdr); }
                resize()?;
                control.status(if growing.is_some() {
                    ""
                } else {
                    if hardware_decoder && !windows_renderer {"Playing • Vulkan decoding and presentation"} else if windows_renderer {"Playing • Microsoft decoding and presentation"} else {"Playing • native Windows decoding and GPU presentation"}
                });
                if let (Some(presenter),Some((target,direction)))=(&presenter,frame_target) {presenter.input.seek_frame(target-point.seconds,direction);set_volume(0)?;}
                if paused && frame_target.is_none() {
                    media.Pause()?;
                } else {
                    media.Run()?;
                }
                startup_report["graph_running_ms"] = json!(startup.elapsed().as_millis());
                let _ = std::fs::write(
                    folder.join("native-startup.json"),
                    startup_report.to_string(),
                );
                let mut first_frame = false;
                let mut first_packet = false;
                let seeking: IMediaSeeking = graph.cast()?;
                let mut fallback_position = 0.;
                let mut last_tick = std::time::Instant::now();
                let mut last_index = std::time::Instant::now();
                let mut color_report = std::time::Instant::now() - Duration::from_secs(1);
                let mut sized = false;
                let size_ready = std::time::Instant::now();
                let mut seek_to = None;
                std::fs::write(folder.join("native-pipeline.json"),json!({"backend":if presenter.is_some(){if hardware_decoder {"Vulkan Video decoding / Vulkan presentation"}else{"Windows H.264 decoding / Vulkan presentation"}}else{"Windows DirectShow / EVR"},"external_processes":false,"service":service,"filters":["A865R Rust source","Windows MPEG2 demultiplexer",if presenter.is_some()&&hardware_decoder{ "Vulkan Video H.264" }else{ "Microsoft DTV-DVD Video Decoder" },"Microsoft DTV-DVD Audio Decoder","A865R PCM sound modes",if presenter.is_some(){"Vulkan"}else{"EVR"},"DirectSound"]}).to_string()).ok();
                let events: IMediaEvent = graph.cast()?;
                let mut health =
                    crate::audio_health::Health::new(folder.join("native-health.jsonl"));
                let stats: IBDA_SignalStatistics = source.cast()?;
                while !control.cancel.load(Ordering::Relaxed) {
                    if !first_frame {
                        if let Some(presenter) = &presenter {
                            let input_stats = presenter.input.stats();
                            let packets = input_stats["decoder"]["compressed_packets"]
                                .as_u64()
                                .unwrap_or(0);
                            if !first_packet && packets > 0 {
                                first_packet = true;
                                startup_report["first_video_packet_ms"] =
                                    json!(startup.elapsed().as_millis());
                                let _ = std::fs::write(
                                    folder.join("native-startup.json"),
                                    startup_report.to_string(),
                                );
                            }
                            if input_stats["received"].as_u64().unwrap_or(0) > 0 {
                                first_frame = true;
                                startup_report["first_decoded_frame_ms"] =
                                    json!(startup.elapsed().as_millis());
                                let _ = std::fs::write(
                                    folder.join("native-startup.json"),
                                    startup_report.to_string(),
                                );
                            } else if should_probe_again(
                                cache_used,
                                size_ready.elapsed(),
                                packets,
                                presenter.input.error().is_some(),
                            ) {
                                // Missing video packets can indicate stale PIDs. Receiving packets
                                // while waiting for an IDR is not a reason to throw away the stream.
                                media.Stop()?;
                                return Ok(
                                    json!({"retry_without_cache":true,"reason":"Saved service produced no video"}),
                                );
                            } else if file.is_none() {
                                if let Some(message) = startup_stall(size_ready.elapsed(), packets)
                                {
                                    return Err(Error::new(E_FAIL, message));
                                }
                            }
                        }
                    }
                    let runtime=control.snapshot();
                    let track=runtime["caption_tracks"].as_array().and_then(|a|a.iter().find(|t|t["program_id"]==service["program_id"]));
                    let selected_pid=track.and_then(|t|t["pid"].as_u64()).map(|n|n as u16);
                    if selected_pid!=caption_pid {
                        if let Some(old)=caption_pid{let _=caption_map.UnmapPID(1,&(old as u32));}
                        caption_pid=selected_pid.filter(|p|caption_map.MapPID(1,&(*p as u32),MEDIA_ELEMENTARY_STREAM).is_ok());
                        captions.select(caption_pid,track.and_then(|t|t["profile"].as_u64()).unwrap_or(8) as u16);

                    }
                    let mut bounds=RECT::default();let _=windows::Win32::UI::WindowsAndMessaging::GetClientRect(HWND(surface as _),&mut bounds);
                    let mut area=[0,0,bounds.right.max(1),bounds.bottom.max(1)];
                    if let Some(v)=presenter.as_ref().map(|p|p.input.stats()){
                        if let Some(a)=v["output"].as_array().filter(|a|a.len()==4){for (i,n) in a.iter().enumerate(){area[i]=n.as_i64().unwrap_or(area[i] as i64) as i32;}}
                    }
                    if let Some(display)=&display {
                        let mut source_rect=MFVideoNormalizedRect::default();let mut target=RECT::default();
                        if display.GetVideoPosition(&mut source_rect,&mut target).is_ok() && target.right>target.left && target.bottom>target.top {
                            area=[target.left,target.top,target.right-target.left,target.bottom-target.top];
                        }
                    }
                    if let Err(message)=crate::parental::check(&control.snapshot()["epg"],frequency,service["program_id"].as_u64().unwrap_or(0) as u32){control.status(&message);control.cancel.store(true,Ordering::Release);media.Stop()?;return Err(Error::new(E_ACCESSDENIED,message));}
                    // Microsoft+Vulkan and Vulkan decoding both expose the presented timestamp.
                    // Before their first frame, captions remain at the start instead of racing ahead.
                    let caption_position=if let Some(p)=&presenter {Some(p.input.displayed_time().map(|(t,_)|(t*1000.).round() as i64).unwrap_or(0))}else{color_transform.as_ref().map(|t|t.caption_position(captions.graph_position()))};
                    captions.tick(area,paused,caption_position);
                    health.sample(paused, presenter.as_ref().map(|p| p.input.stats()));
                    let mut restart_audio=false;
                    while let Ok(cmd) = commands.try_recv() {
                        match cmd {
                            Command::VideoHdr(value)=>{resume.video_hdr=value;if let Some(presenter)=&presenter{presenter.input.set_video_hdr(value);}else if value{control.status("Video HDR requires the Vulkan renderer.");}}
                            Command::Picture(value)=>{
                                resume.picture=value;
                                if let Some(presenter)=&presenter {presenter.input.set_picture(value);}else if let Some(transform)=&color_transform {transform.set_picture(value);}
                            }
                            Command::AspectRatio(value) => {
                                aspect_ratio.set(value);resume.aspect=value;
                                if let Some(presenter)=&presenter { presenter.input.set_aspect_ratio(value); }
                                resize()?;
                            }
                            Command::Step(direction)=>{
                                if let Some(p)=&presenter {
                                    media.Pause()?;paused=true;resume.paused=true;set_volume(0)?;
                                    if !p.input.step(direction) {
                                        if let Some(index)=&index {
                                            let at=p.input.displayed_time().map(|t|point.seconds+t.0).unwrap_or(resume.position);
                                            frame_target=Some((at,direction));seek_to=Some((at-10.).max(0.).min(index.duration));
                                        }else{control.status("No earlier buffered frame is available.");}
                                    }
                                }else{control.status("Frame stepping requires the Vulkan renderer.");}
                            }
                            Command::Pause => {
                                if paused && presenter.as_ref().is_some_and(|p|p.input.is_stepping()) && index.is_some(){
                                    let at=presenter.as_ref().and_then(|p|p.input.displayed_time()).map(|t|point.seconds+t.0).unwrap_or(resume.position);
                                    frame_target=None;paused=false;resume.paused=false;seek_to=Some(at);continue;
                                }
                                paused = !paused;resume.paused=paused;
                                if let Some(presenter) =
                                    presenter.as_ref().filter(|_| file.is_none())
                                {
                                    // Keep the live audio clock and decoder moving. Pausing the
                                    // graph lets demux/audio backpressure block the resume call.
                                    presenter.input.set_live_paused(paused);
                                    set_volume(if paused { 0 } else { volume })?;
                                } else if paused {
                                    media.Pause()?;
                                } else {
                                    media.Run()?;
                                }
                                last_tick = std::time::Instant::now();
                            }
                            Command::Volume(v) => {
                                volume = v;resume.volume=v;
                                set_volume(if paused && file.is_none() && presenter.is_some() {
                                    0
                                } else {
                                    v
                                })?;
                            }
                            Command::AudioMode(mode)=>{
                                audio_mode=mode;resume.audio_mode=mode;mixer.set_mode(mode);
                                control.set_audio_output(mode.name(),service["audio_pid"].as_u64().map(|p|p as u16));
                            }
                            Command::AudioTrack(pid)=>{
                                let runtime=control.snapshot();
                                if let Some(track)=runtime["audio_tracks"].as_array().and_then(|a|a.iter().find(|t|
                                    t["program_id"]==service["program_id"] && t["pid"].as_u64()==Some(pid as u64) && matches!(t["stream_type"].as_u64(),Some(0x0f|0x11)))) {
                                    if service["audio_pid"].as_u64()!=Some(pid as u64) {
                                        service["audio_pid"]=json!(pid);service["audio_type"]=track["stream_type"].clone();
                                        control.set_service(service.clone());restart_audio=true;
                                    }
                                }
                            }
                            Command::GoLive=>{
                                frame_target=None;
                                if growing.as_ref().is_some_and(|g|g.load(Ordering::Acquire)) {
                                    if let Some(index)=&mut index {
                                        index.update().map_err(|e|Error::new(E_FAIL,e.to_string()))?;
                                        paused=false;resume.paused=false;seek_to=Some(crate::timeline::live_position(index.duration));
                                    }
                                }
                            }
                            Command::Seek(seconds) => {
                                frame_target=None;
                                if index.is_some() {
                                    seek_to = Some(seconds);
                                }
                            }
                            Command::Resize => {
                                resize()?;
                                if let Some(display) = &display { let _ = display.RepaintVideo(); }
                            }
                            Command::Repaint => {
                                // Before the first frame EVR may return MF_E_INVALIDREQUEST.
                                // Do not turn a normal early WM_PAINT into playback failure.
                                if let Some(display) = &display {
                                    if display.RepaintVideo().is_ok() { evr_repaints += 1; }
                                }
                            }
                            Command::Snapshot(path) => {
                                if let Some(presenter) = &presenter {
                                    presenter.input.snapshot(path);
                                    continue;
                                }
                                let display = display.as_ref().unwrap();
                                let mut header = BITMAPINFOHEADER {
                                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                                    ..Default::default()
                                };
                                let mut data = std::ptr::null_mut();
                                let mut bytes = 0;
                                let mut stamp = 0;
                                let image_result = display.GetCurrentImage(
                                    &mut header,
                                    &mut data,
                                    &mut bytes,
                                    &mut stamp,
                                );
                                if let Err(e) = &image_result {
                                    let _ = std::fs::write(
                                        path.with_extension("error.txt"),
                                        e.to_string(),
                                    );
                                }
                                if image_result.is_ok() && !data.is_null() {
                                    let mut bmp = Vec::new();
                                    bmp.extend_from_slice(b"BM");
                                    bmp.extend_from_slice(&(14 + 40 + bytes).to_le_bytes());
                                    bmp.extend_from_slice(&[0; 4]);
                                    bmp.extend_from_slice(&54u32.to_le_bytes());
                                    bmp.extend_from_slice(std::slice::from_raw_parts(
                                        (&header as *const BITMAPINFOHEADER).cast::<u8>(),
                                        40,
                                    ));
                                    bmp.extend_from_slice(std::slice::from_raw_parts(
                                        data,
                                        bytes as usize,
                                    ));
                                    if let Err(e)=crate::snapshots::save_bmp(&path,&bmp) {
                                        let _=std::fs::write(path.with_extension("error.txt"),e);
                                    }
                                }
                                if !data.is_null() {CoTaskMemFree(Some(data.cast()));}
                            }
                        }
                    }
                    if let Some(presenter) = &presenter {
                        if let Some(e) = presenter.input.error() {
                            return Err(Error::new(E_FAIL, e));
                        }
                        if presenter.input.finished() {
                            break;
                        }
                    }
                    if color_report.elapsed() >= Duration::from_secs(1) {
                        // EVR may announce new display-aspect metadata without a window resize.
                        // Vulkan follows the format carried by each frame; keep EVR current too.
                        if let Some(display)=&display {
                            let mut size=SIZE::default();let mut aspect=SIZE::default();
                            if display.GetNativeVideoSize(&mut size,&mut aspect).is_ok()
                                && evr_format.get()!=Some((size.cx,size.cy,aspect.cx,aspect.cy)) {
                                resize()?;
                            }
                        }
                        if windows_renderer {let q=evr_quality.as_ref();let _=std::fs::write(folder.join("native-evr.json"),json!({"repaints":evr_repaints,"frames_drawn":q.and_then(|q|q.FramesDrawn().ok()),"frames_dropped":q.and_then(|q|q.FramesDroppedInRenderer().ok()),"graph_clock_ms":captions.graph_position(),"video_delivered_until_ms":color_transform.as_ref().map(|t|t.backend()["delivered_until_ms"].clone())}).to_string());}

                        let mut audio_report=mixer.report();
                        audio_report["pid"]=service["audio_pid"].clone();
                        audio_report["multichannel_configured"]=json!(multichannel_configured);
                        audio_report["broadcast_channels"]=json!(audio_codec.as_ref().and_then(|c|c.GetValue(&CODECAPI_AVAudioChannelCount).ok()).and_then(|v|u32::try_from(&v).ok()));
                        control.set_audio_details(audio_report.clone());
                        let _=std::fs::write(folder.join("native-audio.json"),audio_report.to_string());
                        if let Some(presenter) = &presenter {
                            let report = presenter.input.stats();
                            if let (Some(w), Some(h)) =
                                (report["source"][0].as_u64(), report["source"][1].as_u64())
                            {
                                control.set_video_size(w as u32, h as u32);
                            }
                            let _ = std::fs::write(
                                folder.join("native-vulkan.json"),
                                report.to_string(),
                            );
                        }
                        let report = if let Some(t) = &color_transform {
                            let frames = t.frames.load(Ordering::Relaxed);
                            json!({"state":if frames>0{"applied"}else{"waiting_for_frame"},"path":t.path,"sha256":t.sha256,"frames_transformed":frames,"average_ms":t.micros.load(Ordering::Relaxed)as f64/frames.max(1)as f64/1000.,"engine":if t.path.as_os_str().is_empty(){"Picture processing"}else{"Windows ICC CMM"},"backend":if let Some(p)=&presenter {let processing=p.input.stats()["shader_processing"].clone();if processing.is_null(){json!("Vulkan ICC 3D LUT")}else{processing}}else{t.backend()},"intent":"relative colorimetric","source":"SDR RGB with sRGB primaries and display encoding"})
                        } else {
                            json!({"state":if color==a865r::api::ColorProfile::Disabled{"disabled"}else{"no_monitor_profile"}})
                        };
                        control.set_shader_state(if let Some(p)=&presenter {let state=p.input.stats()["shader_processing"].clone();if state.is_null(){json!({"enabled":shader!=Shader::Off,"name":shader.label()})}else{state}}else{report["backend"].clone()});
                        let _ =
                            std::fs::write(folder.join("native-color.json"), report.to_string());
                        color_report = std::time::Instant::now();
                    }
                    if !sized && size_ready.elapsed() >= Duration::from_secs(1) {
                        resize()?;
                        sized = true;
                    }
                    if restart_audio {
                        if let Some(index)=&mut index {
                            index.update().map_err(|e|Error::new(E_FAIL,e.to_string()))?;
                            point=index.seek(control.snapshot()["timeline"]["position"].as_f64().unwrap_or(point.seconds));
                        }
                        media.Stop()?;continue 'playback;
                    }
                    if let Some(seconds) = seek_to.take() {
                        if let Some(index) = &mut index {
                            index
                                .update()
                                .map_err(|e| Error::new(E_FAIL, e.to_string()))?;
                            point = index.seek(seconds);
                            let _=std::fs::write(folder.join("native-seek.json"),json!({"requested":seconds,"actual":point.seconds,"offset":point.offset,"paused":paused}).to_string());
                            resume.position=point.seconds;
                            control.set_timeline(point.seconds, index.duration, true, paused);
                            media.Stop()?;
                            continue 'playback;
                        }
                    }
                    if let Some(index) = &mut index {
                        if last_index.elapsed() >= Duration::from_millis(300) {
                            index
                                .update()
                                .map_err(|e| Error::new(E_FAIL, e.to_string()))?;
                            last_index = std::time::Instant::now();
                        }
                        if !paused {
                            fallback_position += last_tick.elapsed().as_secs_f64();
                        }
                        last_tick = std::time::Instant::now();
                        let position = seeking
                            .GetCurrentPosition()
                            .ok()
                            .filter(|p| *p > 0)
                            .map(|p| p as f64 / 10_000_000.)
                            .unwrap_or(fallback_position);
                        let position=if presenter.as_ref().is_some_and(|p|p.input.is_stepping()) {presenter.as_ref().and_then(|p|p.input.displayed_time()).map(|t|t.0).unwrap_or(position)}else{position};
                        resume.position=(point.seconds + position).min(index.duration);
                        if paused && frame_target.is_some() && presenter.as_ref().is_some_and(|p|p.input.displayed_time().is_some()) {media.Pause()?;frame_target=None;}
                        control.set_timeline(
                            resume.position,
                            index.duration,
                            index.duration >= 0.5,
                            paused,
                        );
                    } else {
                        // Live viewing still has an elapsed clock without a seek index.
                        // Resume retains this value across graph recovery; pausing freezes it.
                        if !paused {resume.position+=last_tick.elapsed().as_secs_f64();}
                        last_tick=std::time::Instant::now();
                        control.set_timeline(resume.position,0.,false,paused);
                    }
                    if file.is_none() {
                        let mut quality = -1;
                        if stats.SignalQuality(&mut quality).is_ok() {
                            control.set_signal(u8::try_from(quality).ok().filter(|n| *n <= 100));
                        }
                    }
                    let mut code = 0;
                    let mut p1 = 0;
                    let mut p2 = 0;
                    if events.GetEvent(&mut code, &mut p1, &mut p2, 0).is_ok() {
                        events.FreeEventParams(code, p1, p2)?;
                        if code == EC_COMPLETE as i32 {
                            break;
                        }
                        if code == EC_ERRORABORT as i32 {
                            return Err(Error::new(
                                HRESULT(p1 as i32),
                                "Native playback graph stopped with an error",
                            ));
                        }
                    }
                    std::thread::sleep(Duration::from_millis(30));
                }
                media.Stop()?;
                break;
            }
            Ok(
                json!({"success":true,"native":true,"stopped":control.cancel.load(Ordering::Relaxed)}),
            )
        }
    })();
    result.unwrap_or_else(|e| json!({"success":false,"error":e.to_string()}))
}

/// Owns cancellation and joining even when playback unwinds before returning.
struct RecordingWorker {
    control: Control,
    worker: Option<std::thread::JoinHandle<Value>>,
}
impl RecordingWorker {
    fn finish(&mut self) -> Value {
        self.control.stop();
        self.worker.take().map(|worker| worker.join().unwrap_or_else(|_|
            json!({"success":false,"error":"Recording worker ended unexpectedly"})))
            .unwrap_or_else(|| json!({"stopped":true}))
    }
}
impl Drop for RecordingWorker {
    fn drop(&mut self) { let _ = self.finish(); }
}
struct GrowingRecording(std::sync::Arc<std::sync::atomic::AtomicBool>);
impl Drop for GrowingRecording {
    fn drop(&mut self) { self.0.store(false, Ordering::Release); }
}

/// A single tuner writer owns reception; replay reads the growing recording independently.
pub fn record(
    path:PathBuf,
    frequency: u32,
    program: Option<u32>,
    surface: usize,
    folder: PathBuf,
    control: Control,
    commands: mpsc::Receiver<Command>,
    volume: u32,
    resolution: a865r::api::Resolution,
    deinterlacing: a865r::api::DeinterlaceMode,
    audio_mode: crate::audio::Mode,
    color: a865r::api::ColorProfile,
    ffmpeg:PathBuf,
    backend:crate::backend::Backend,
    shader:crate::backend::Shader,
    picture:crate::picture::Picture,
) -> Value {
    use std::sync::{atomic::AtomicBool, Arc};
    if let Some(parent)=path.parent(){if let Err(e)=std::fs::create_dir_all(parent){return json!({"success":false,"error":format!("Cannot create recording folder: {e}")});}}
    // Check before starting a raw recording writer, not after media is saved.
    if crate::parental::active(){
        let result=probe_services(frequency,&folder,&control).map_err(|e|e.to_string()).and_then(|sample|{
            let mut analyzer=a865r::TsAnalyzer::new();analyzer.push(&sample);
            let selected=program.or_else(||analyzer.stats().streams.iter().find(|s|s.stream_type==0x1b).map(|s|s.program_number as u32)).unwrap_or(0);
            crate::parental::check(&crate::epg_source::events(analyzer.stats(),frequency),frequency,selected)
        });
        if let Err(e)=result{return json!({"success":false,"error":e});}
    }
    if program.is_none(){return json!({"success":false,"error":"Select a TV service before recording"});}
    let capture_path=path.with_extension("broadcast.ts");
    let writer_path=capture_path.clone();
    let export_settings=a865r_media::recording_export::Settings{program:program.unwrap(),resolution,deinterlacing,picture:picture.json()};
    if let Err(e)=std::fs::write(capture_path.with_extension("settings.json"),export_settings.json().to_string()){
        return json!({"success":false,"error":format!("Cannot save recording settings: {e}")});
    }
    let capture = Control::default();
    let worker_control = capture.clone();
    let growing = Arc::new(AtomicBool::new(true));
    let worker_growing = growing.clone();
    let writer_folder = folder.clone();
    let worker = std::thread::spawn(move || {
        let _growing = GrowingRecording(worker_growing);
        let result = television::run(
            television::Action::RecordUntilStopped { frequency, path:writer_path },
            PathBuf::new(),
            writer_folder,
            a865r_media::playback::Options{program_id:program,..Default::default()},
            worker_control,
        );
        result
    });
    let mut writer = RecordingWorker { control: capture.clone(), worker: Some(worker) };
    let started = std::time::Instant::now();
    while !control.cancel.load(Ordering::Relaxed) && growing.load(Ordering::Acquire) {
        control.status(format!(
            "Starting recording… {}",
            capture.message.lock().unwrap()
        ));
        if started.elapsed() >= Duration::from_secs(4)
            && std::fs::metadata(&capture_path)
                .map(|m| m.len() > 1_000_000)
                .unwrap_or(false)
        {
            break;
        }
        if started.elapsed() > Duration::from_secs(30) {
            return json!({"success":false,"error":"Recording did not produce enough data within 30 seconds.","recording":path});
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let mut playback = if control.cancel.load(Ordering::Relaxed) {
        json!({"stopped":true})
    } else if !growing.load(Ordering::Acquire) {
        json!({})
    } else {
        run(
            Some(capture_path.clone()),
            frequency,
            program,
            surface,
            folder.clone(),
            control.clone(),
            &commands,
            volume,
            resolution,
            deinterlacing,
            audio_mode,
            color,
            Some(growing),
            None,
            backend,shader,
        )
    };
    let recording = writer.finish();
    if recording["success"] == false {
        return recording;
    }
    match a865r_media::recording_export::finish(&capture_path,&path,
        &a865r_media::recording_export::Settings{program:program.unwrap(),resolution,deinterlacing,picture:picture.json()},&ffmpeg,&control) {
        Ok(report)=>{
            // This temporary capture was created by this recording operation;
            // the verified rendered recording is now the library file.
            let _=std::fs::remove_file(&capture_path);
            let _=std::fs::remove_file(capture_path.with_extension("settings.json"));
            playback["recording"]=json!(path);playback["recording_validation"]=report;
        }
        Err(e)=>{playback["success"]=json!(false);playback["error"]=json!(e);playback["broadcast_capture"]=json!(capture_path);}
    }
    playback
}

#[cfg(test)]
mod startup_tests {
    use super::*;
    #[test]
    fn normal_playback_keeps_vulkan_and_respects_explicit_diagnostics() {
        assert!(use_vulkan_decoder(["live-tv.exe"]));
        assert!(use_vulkan_decoder(["live-tv.exe", "--play", "recording.ts"]));
        assert!(use_vulkan_decoder(["live-tv.exe", "--verify-record"]));
        assert!(use_vulkan_decoder(["live-tv.exe", "--diagnostic"]));
        assert!(use_vulkan_decoder(["live-tv.exe", "--vulkan-decoder"]));
        assert!(!use_vulkan_decoder(["--vulkan-decoder", "--software-decoder"]));
        assert!(!use_vulkan_decoder(["--software-decoder", "--vulkan-decoder"]));
        assert!(!use_vulkan_decoder(["--vulkan-decoder", "--windows-renderer"]));
    }

    fn recovery_folder(name:&str)->PathBuf {
        let p=std::env::temp_dir().join(format!("live-tv-recovery-{}-{name}",std::process::id()));
        std::fs::create_dir_all(&p).unwrap();p
    }
    #[test]
    fn recoverable_errors_stop_without_retry_and_leave_vulkan_available() {
        for message in ["OutOfMemory", "out of memory", "VK_ERROR_OUT_OF_DEVICE_MEMORY",
            "GPU completion timed out; playback stopped", "The wait timed out",
            "swapchainAcquireSemaphore validation failure", "swapchainPresentSemaphores validation failure",
            "Invalid H.264 NAL unit"] {
            assert!(!graphics_lost(&json!({"error":message})), "{message}");
            let control=Control::default();let folder=recovery_folder("recoverable");let mut calls=0;
            let result=recover_playback(&control,&folder,false,|windows,_| {
                assert!(!windows);calls+=1;json!({"error":message})
            });
            assert_eq!(calls,1);assert_eq!(result["error"],message);
            recover_playback(&control,&folder,false,|windows,_| {
                assert!(!windows);json!({"success":true})
            });
            let _=std::fs::remove_dir_all(folder);
        }
    }
    #[test]
    fn device_loss_recreates_vulkan_once_after_old_attempt_is_released() {
        let control=Control::default();let folder=recovery_folder("fresh-vulkan");
        let mut calls=Vec::new();let released=std::cell::Cell::new(false);
        struct Attempt<'a>(&'a std::cell::Cell<bool>);
        impl Drop for Attempt<'_> {fn drop(&mut self){self.0.set(true);}}
        let result=recover_playback(&control,&folder,false,|windows,path| {
            assert!(!windows,"Recovery must never select the Microsoft codec");
            if !calls.is_empty() {assert!(released.get());}
            calls.push(path.to_owned());
            let _resources=Attempt(&released);
            if calls.len()==1 {json!({"error":"Parent device is lost"})} else {json!({"success":true})}
        });
        assert_eq!(calls.len(),2);assert_ne!(calls[0],calls[1]);assert_eq!(result["success"],true);
        assert_eq!(result["graphics_recovery"]["backend"],"Vulkan");
        assert_eq!(result["graphics_recovery"]["attempts"],1);
        let _=std::fs::remove_dir_all(folder);
    }
    #[test]
    fn failed_vulkan_refresh_returns_error_without_fallback_or_lockout() {
        let control=Control::default();let folder=recovery_folder("failed-refresh");let mut calls=0;
        let result=recover_playback(&control,&folder,false,|windows,_| {
            assert!(!windows);calls+=1;json!({"error":"VK_ERROR_DEVICE_LOST"})
        });
        assert_eq!(calls,6);assert_eq!(result["graphics_recovery"]["attempts"],5);assert!(result["error"].as_str().unwrap().contains("Vulkan recovery failed"));
        assert_eq!(result["graphics_recovery"]["state"],"failed");
        assert!(result["graphics_recovery"].get("fallback").is_none());
        recover_playback(&control,&folder,false,|windows,_| {assert!(!windows);json!({"success":true})});
        let _=std::fs::remove_dir_all(folder);
    }
    #[test]
    fn fifth_retry_can_succeed_and_preserves_the_selected_backend() {
        for windows in [false,true] {
            let control=Control::default();let folder=recovery_folder(if windows {"ms-five"}else{"vk-five"});let mut paths=Vec::new();
            let result=recover_playback(&control,&folder,windows,|selected,path| {
                assert_eq!(selected,windows);paths.push(path.to_owned());
                if paths.len()<=5 {json!({"error":"DeviceLost"})} else {json!({"success":true})}
            });
            assert_eq!(paths.len(),6);assert_eq!(paths.iter().collect::<std::collections::HashSet<_>>().len(),6);
            assert_eq!(result["success"],true);assert_eq!(result["graphics_recovery"]["attempts"],5);
            let _=std::fs::remove_dir_all(folder);
        }
    }
    #[test]
    fn a_different_failure_during_recovery_stops_further_retries() {
        let control=Control::default();let folder=recovery_folder("new-error");let mut calls=0;
        let result=recover_playback(&control,&folder,false,|_,_| {
            calls+=1;if calls==1 {json!({"error":"DeviceLost"})}else{json!({"error":"Invalid H.264 NAL unit"})}
        });
        assert_eq!(calls,2);assert!(result["error"].as_str().unwrap().contains("Invalid H.264 NAL unit"));
        let _=std::fs::remove_dir_all(folder);
    }
    #[test]
    fn stop_or_channel_change_cancels_vulkan_refresh() {
        let control=Control::default();let folder=recovery_folder("cancel");let mut calls=0;
        let result=recover_playback(&control,&folder,false,|windows,_| {
            assert!(!windows);calls+=1;control.cancel.store(true,Ordering::Relaxed);json!({"error":"DeviceLost"})
        });
        assert_eq!(calls,1);assert_eq!(result["stopped"],true);
        recover_playback(&control,&folder,false,|_,_|panic!("Cancelled playback cannot open a device"));
        let _=std::fs::remove_dir_all(folder);
    }
    #[test]
    fn channel_change_during_recovery_backoff_interrupts_retry() {
        let control=Control::default();let cancel=control.cancel.clone();
        let worker=std::thread::spawn(move || {std::thread::sleep(Duration::from_millis(100));cancel.store(true,Ordering::Relaxed);});
        let folder=recovery_folder("backoff");let mut calls=0;
        let result=recover_playback(&control,&folder,false,|windows,_|{assert!(!windows);calls+=1;json!({"error":"DeviceLost"})});
        worker.join().unwrap();assert_eq!(calls,1);assert_eq!(result["stopped"],true);
        let _=std::fs::remove_dir_all(folder);
    }
    #[test]
    fn unavailable_services_and_missing_keyframes_have_distinct_timeouts() {
        assert!(startup_stall(Duration::from_secs(14), 0).is_none());
        assert!(startup_stall(Duration::from_secs(15), 0)
            .unwrap()
            .contains("Unable to start video"));
        assert!(startup_stall(Duration::from_secs(15), 1).is_none());
        assert!(startup_stall(Duration::from_secs(29), 200).is_none());
        assert!(startup_stall(Duration::from_secs(30), 200)
            .unwrap()
            .contains("no decodable picture"));
    }
    #[test]
    fn arriving_video_keeps_waiting_for_keyframe_without_retuning() {
        assert!(should_probe_again(true, Duration::from_secs(8), 0, false));
        assert!(!should_probe_again(true, Duration::from_secs(7), 0, false));
        assert!(!should_probe_again(true, Duration::from_secs(60), 1, false));
        assert!(!should_probe_again(true, Duration::from_secs(60), 0, true));
        assert!(!should_probe_again(
            false,
            Duration::from_secs(60),
            0,
            false
        ));
    }

    fn service() -> Value {
        json!({"frequency_khz":521143,"program_id":1025,"video_type":27,"video_pid":257,"pcr_pid":257,"audio_pid":258,"audio_type":15})
    }
    #[test]
    fn services_on_one_frequency_keep_the_requested_program() {
        let first = service();
        let mut second = first.clone();
        second["program_id"] = json!(1026);
        second["video_pid"] = json!(513);
        let services = vec![first, second];
        assert_eq!(
            choose_service(&services, Some(1026)).unwrap()["video_pid"],
            513
        );
        assert!(choose_service(&services, Some(999)).is_none());
        assert_eq!(choose_service(&services, None).unwrap()["program_id"], 1025);
    }
    #[test]
    fn probe_cancellation_is_forwarded_but_normal_completion_is_isolated() {
        let parent = Control::default();
        let probe = Control::default();
        let child = probe.clone();
        assert_eq!(
            cancellable_probe(&parent, &probe, move || {
                child.stop();
                json!({"success":true})
            })["success"],
            true
        );
        assert!(!parent.cancel.load(Ordering::Relaxed));
        let probe = Control::default();
        let child = probe.clone();
        let cancel = parent.cancel.clone();
        let caller = std::thread::current().id();
        let result = cancellable_probe(&parent, &probe, move || {
            assert_ne!(caller, std::thread::current().id());
            cancel.store(true, Ordering::Relaxed);
            let start = std::time::Instant::now();
            while !child.cancel.load(Ordering::Relaxed) && start.elapsed() < Duration::from_secs(2)
            {
                std::thread::sleep(Duration::from_millis(1));
            }
            json!({"cancelled":child.cancel.load(Ordering::Relaxed)})
        });
        assert_eq!(result["cancelled"], true);
        let result = cancellable_probe(&parent, &probe, || {
            panic!("Already-cancelled probe must not run")
        });
        assert_eq!(result["stopped"], true);
    }
    #[test]
    fn saved_service_skips_probe_only_for_matching_complete_tv_metadata() {
        assert!(saved_service(Some(service()), 521143, Some(1025)).is_some());
        assert!(saved_service(Some(service()), 527143, Some(1025)).is_none());
        assert!(saved_service(Some(service()), 521143, Some(1026)).is_none());
        assert!(saved_service(Some(service()), 521143, None).is_none());
        assert!(saved_service(None, 521143, Some(1025)).is_none());
        for (key, value) in [
            ("video_pid", json!(null)),
            ("video_pid", json!(8191)),
            ("pcr_pid", json!(1)),
            ("video_type", json!(2)),
            ("audio_type", json!(129)),
        ] {
            let mut s = service();
            s[key] = value;
            assert!(
                saved_service(Some(s), 521143, Some(1025)).is_none(),
                "{key}"
            );
        }
        let mut silent = service();
        silent["audio_pid"] = Value::Null;
        silent["audio_type"] = Value::Null;
        assert!(saved_service(Some(silent), 521143, Some(1025)).is_some());
    }
}

#[cfg(test)] mod recording_cleanup_tests {
    use super::*;
    #[test] fn unwinding_playback_stops_and_joins_recording() {
        let control=Control::default();let child=control.clone();
        let stopped=std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));let done=stopped.clone();
        let worker=std::thread::spawn(move || {
            while !child.cancel.load(Ordering::Acquire) {std::thread::sleep(Duration::from_millis(1));}
            done.store(true,Ordering::Release);json!({"success":true})
        });
        let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _writer=RecordingWorker {control,worker:Some(worker)};
            panic!("simulated playback failure");
        }));
        assert!(result.is_err());assert!(stopped.load(Ordering::Acquire));
    }
    #[test] fn writer_panic_releases_growing_file_reader() {
        let growing=std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));let flag=growing.clone();
        let _=std::panic::catch_unwind(move || {let _guard=GrowingRecording(flag);panic!("writer failure");});
        assert!(!growing.load(Ordering::Acquire));
    }
}
