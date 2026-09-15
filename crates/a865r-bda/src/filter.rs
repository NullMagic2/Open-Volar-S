use crate::{
    backend::{Session, Status, Tune},
    invalid, trace, unsupported, OBJECTS,
};
use std::{
    mem::{size_of, ManuallyDrop},
    sync::{
        atomic::{AtomicI32, Ordering},
        mpsc::Receiver,
        Arc, Mutex,
    },
    thread::JoinHandle,
};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Media::{
            DirectShow::Tv::KSDATAFORMAT_TYPE_BDA_ANTENNA,
            DirectShow::*,
            IReferenceClock,
            MediaFoundation::{
                CLSID_MemoryAllocator, FORMAT_None, FORMAT_VideoInfo, FORMAT_VideoInfo2,
                MEDIATYPE_Stream, AM_MEDIA_TYPE, VIDEOINFOHEADER, VIDEOINFOHEADER2,
            },
        },
        System::Com::*,
    },
};

pub const TUNER_CLSID: GUID = GUID::from_u128(0x2a5fc455_33c1_482c_9a83_4df863f0c601);
pub const CAPTURE_CLSID: GUID = GUID::from_u128(0x2a5fc455_33c1_482c_9a83_4df863f0c602);
const VFW_E_NOT_CONNECTED: HRESULT = HRESULT(0x80040209u32 as i32);
const VFW_E_ALREADY_CONNECTED: HRESULT = HRESULT(0x80040204u32 as i32);
const VFW_E_TYPE_NOT_ACCEPTED: HRESULT = HRESULT(0x8004022Au32 as i32);
const VFW_E_NOT_STOPPED: HRESULT = HRESULT(0x80040224u32 as i32);

// DirectShow filters and their pins are free-threaded. All mutable state is locked;
// foreign streaming interfaces are only used by the delivery thread or during flush.
struct Connection {
    pin: IPin,
    sink: Option<IMemInputPin>,
    allocator: Option<IMemAllocator>,
}
unsafe impl Send for Connection {}
struct Running {
    session: Session,
    queue: Option<Receiver<Vec<u8>>>,
    delivery: Option<JoinHandle<()>>,
}
/// Optional application-owned sample transform; tuner/TS filters are unchanged.
pub trait VideoTransform: Send + Sync {
    fn accepts(&self, media: &AM_MEDIA_TYPE) -> bool;
    fn configure(&self, media: &AM_MEDIA_TYPE) -> Result<()>;
    fn bytes_required(&self) -> usize;
    fn allocator_bytes(&self) -> usize {self.bytes_required().max(1920 * 1088 * 4)}
    fn requires_nv12_repack(&self) -> bool {true}
    fn process(&self, bytes: &mut [u8]) -> Result<()>;
    fn terminal(&self) -> bool {
        false
    }
    fn receive(&self, _sample: &IMediaSample) -> Result<()> {
        Err(unsupported())
    }
    /// Called after the downstream renderer accepts this timestamped sample.
    fn delivered(&self, _sample:&IMediaSample) {}
    fn run(&self, _start: i64, _clock: Option<IReferenceClock>) {}
    fn pause(&self) {}
    fn stop(&self) {}
    fn end_of_stream(&self) {}
    fn begin_flush(&self) { self.stop(); }
    fn end_flush(&self) {}
}
pub type StreamObserver = Arc<dyn Fn(&[u8]) + Send + Sync>;
struct OwnedMedia {
    mt: AM_MEDIA_TYPE,
    bytes: Vec<u8>,
}
unsafe impl Send for OwnedMedia {}
impl OwnedMedia {
    unsafe fn new(mt: &AM_MEDIA_TYPE) -> Result<Self> {
        if mt.cbFormat > 65536 || mt.cbFormat > 0 && mt.pbFormat.is_null() {
            return Err(invalid());
        }
        let bytes = if mt.cbFormat > 0 {
            std::slice::from_raw_parts(mt.pbFormat, mt.cbFormat as usize).to_vec()
        } else {
            vec![]
        };
        Ok(Self {
            mt: mt.clone(),
            bytes,
        })
    }
    fn view(&self) -> AM_MEDIA_TYPE {
        let mut m = self.mt.clone();
        m.pbFormat = self.bytes.as_ptr() as *mut u8;
        m
    }
    unsafe fn export(&self) -> Result<AM_MEDIA_TYPE> {
        let mut m = self.mt.clone();
        m.pbFormat = CoTaskMemAlloc(self.bytes.len()) as *mut u8;
        if !self.bytes.is_empty() && m.pbFormat.is_null() {
            return Err(Error::from_hresult(E_OUTOFMEMORY));
        }
        std::ptr::copy_nonoverlapping(self.bytes.as_ptr(), m.pbFormat, self.bytes.len());
        Ok(m)
    }
}
unsafe fn sample_media(sample: &IMediaSample) -> Result<Option<OwnedMedia>> {
    let mt = match sample.GetMediaType() {
        Ok(mt) if !mt.is_null() => mt,
        _ => return Ok(None),
    };
    let owned = OwnedMedia::new(&*mt);
    CoTaskMemFree(Some((*mt).pbFormat.cast()));
    ManuallyDrop::drop(&mut (*mt).pUnk);
    CoTaskMemFree(Some(mt.cast()));
    owned.map(Some)
}
unsafe fn nv12_shape(mt: &AM_MEDIA_TYPE) -> Result<(usize, usize)> {
    let header = if mt.formattype == FORMAT_VideoInfo2
        && mt.cbFormat as usize >= size_of::<VIDEOINFOHEADER2>()
    {
        std::ptr::read_unaligned(mt.pbFormat.cast::<VIDEOINFOHEADER2>()).bmiHeader
    } else if mt.formattype == FORMAT_VideoInfo
        && mt.cbFormat as usize >= size_of::<VIDEOINFOHEADER>()
    {
        std::ptr::read_unaligned(mt.pbFormat.cast::<VIDEOINFOHEADER>()).bmiHeader
    } else {
        return Err(invalid());
    };
    let width = usize::try_from(header.biWidth).map_err(|_| invalid())?;
    let height = header.biHeight.unsigned_abs() as usize;
    if width == 0 || height == 0 || width % 2 != 0 || height % 2 != 0 {
        return Err(invalid());
    }
    Ok((width, height))
}
/// Expand a packed NV12 frame into EVR's padded surface without changing pixels.
fn repack_nv12(bytes: &mut [u8], iw: usize, ih: usize, ow: usize, oh: usize) -> Result<usize> {
    let total = ow
        .checked_mul(oh)
        .and_then(|v| v.checked_mul(3))
        .map(|v| v / 2)
        .ok_or_else(invalid)?;
    if iw > ow || ih > oh || bytes.len() < total {
        return Err(Error::new(
            E_FAIL,
            "Renderer surface is smaller than the decoded frame",
        ));
    }
    // Move chroma first, then luma; descending rows make overlapping moves safe.
    for row in (0..ih / 2).rev() {
        let src = iw * ih + row * iw;
        let dst = ow * oh + row * ow;
        bytes.copy_within(src..src + iw, dst);
        bytes[dst + iw..dst + ow].fill(128);
    }
    bytes[ow * oh + ih / 2 * ow..total].fill(128);
    for row in (0..ih).rev() {
        let src = row * iw;
        let dst = row * ow;
        bytes.copy_within(src..src + iw, dst);
        bytes[dst + iw..dst + ow].fill(16);
    }
    bytes[ih * ow..ow * oh].fill(16);
    Ok(total)
}
// Process in ordinary system memory, then copy once into EVR's surface.
// Reading a write-combined renderer allocation during GPU upload is expensive.
fn prepare_nv12_frame(
    source:&[u8],scratch:&mut Vec<u8>,shape:[usize;4],capacity:usize,
    process:impl FnOnce(&mut [u8])->Result<()>,
)->Result<usize> {
    let [iw,ih,ow,oh]=shape;
    let needed=ow.checked_mul(oh).and_then(|n|n.checked_mul(3)).map(|n|n/2).ok_or_else(invalid)?.max(source.len());
    if needed>capacity {return Err(invalid());}
    scratch.resize(needed,0);scratch[..source.len()].copy_from_slice(source);
    process(&mut scratch[..source.len()])?;
    repack_nv12(scratch,iw,ih,ow,oh)
}
#[cfg(test)]
mod video_tests {
    #[test] fn nv12_processing_uses_reusable_system_memory_before_renderer_padding() {
        let source=vec![100u8;4*2*3/2];let original=source.clone();let mut scratch=Vec::new();
        let n=super::prepare_nv12_frame(&source,&mut scratch,[4,2,8,4],48,|bytes|{assert_ne!(bytes.as_ptr(),source.as_ptr());bytes[0]=150;Ok(())}).unwrap();
        assert_eq!(source,original);assert_eq!(n,48);assert_eq!(scratch[0],150);assert_eq!(scratch[4],16);assert_eq!(scratch[32],100);assert_eq!(scratch[36],128);
        let allocation=scratch.as_ptr();
        super::prepare_nv12_frame(&source,&mut scratch,[4,2,8,4],48,|_|Ok(())).unwrap();
        assert_eq!(scratch.as_ptr(),allocation);assert_eq!(scratch[0],100);
        assert!(super::prepare_nv12_frame(&source,&mut scratch,[4,2,8,4],47,|_|panic!("Must reject a short output before processing")).is_err());
    }

    #[test]
    fn nv12_renderer_pitch_preserves_pixels_and_pads_both_planes() {
        let mut data = vec![0u8; 48];
        data[..12].copy_from_slice(&[20, 21, 22, 23, 24, 25, 26, 27, 120, 130, 121, 131]);
        assert_eq!(super::repack_nv12(&mut data, 4, 2, 8, 4).unwrap(), 48);
        assert_eq!(&data[..8], &[20, 21, 22, 23, 16, 16, 16, 16]);
        assert_eq!(&data[8..16], &[24, 25, 26, 27, 16, 16, 16, 16]);
        assert!(data[16..32].iter().all(|v| *v == 16));
        assert_eq!(&data[32..36], &[120, 130, 121, 131]);
        assert!(data[36..].iter().all(|v| *v == 128));
        assert!(super::repack_nv12(&mut data[..10], 4, 2, 8, 4).is_err());
    }
}
impl Drop for OwnedMedia {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.mt.pUnk);
        }
    }
}
pub(crate) struct Shared {
    pub capture: bool,
    external_signal: bool,
    video: Option<Arc<dyn VideoTransform>>,
    video_media: Mutex<Option<OwnedMedia>>,
    video_output_media: Mutex<Option<OwnedMedia>>,
    video_scratch: Mutex<Vec<u8>>,
    observer: Option<StreamObserver>,
    clock_recovery: Option<crate::clock_recovery::ClockRecoveryConfig>,
    replay: Option<crate::backend::Replay>,
    pub tune: Mutex<Tune>,
    pub committed: Mutex<Tune>,
    pub applied: Mutex<Option<Tune>>,
    pub status: Arc<Status>,
    pub state: AtomicI32,
    pins: Mutex<[Weak<IPin>; 2]>,
    connections: [Mutex<Option<Connection>>; 2],
    running: Mutex<Option<Running>>,
    transition: Mutex<()>,
}
impl Shared {
    pub fn commit(&self) -> Result<()> {
        let tune = *self.tune.lock().unwrap();
        self.start_tune(tune)?;
        *self.committed.lock().unwrap() = tune;
        Ok(())
    }
    fn ensure_stream(&self) -> Result<()> {
        let tune = *self.committed.lock().unwrap();
        self.start_tune(tune)
    }
    fn start_tune(&self, tune: Tune) -> Result<()> {
        tune.khz().map_err(|e| Error::new(E_INVALIDARG, e))?;
        if self.capture {
            return Ok(());
        }
        if *self.applied.lock().unwrap() == Some(tune) && self.running.lock().unwrap().is_some() {
            return Ok(());
        }
        let _guard = self.transition.lock().unwrap();
        self.stop_stream();
        let (session, queue) = match &self.replay {
            Some(path) => Session::replay(path.clone()),
            None if self.external_signal => Session::start_external(tune, self.status.clone()),
            None => Session::start(tune, self.status.clone()),
        }
        .map_err(|e| Error::new(E_FAIL, e))?;
        *self.running.lock().unwrap() = Some(Running {
            session,
            queue: Some(queue),
            delivery: None,
        });
        *self.applied.lock().unwrap() = Some(tune);
        if self.state.load(Ordering::Relaxed) != 0 {
            self.deliver()?;
        }
        Ok(())
    }
    pub(crate) fn stop_stream(&self) {
        let (peer, allocator) = self.connections[1]
            .lock()
            .unwrap()
            .as_ref()
            .map(|c| (Some(c.pin.clone()), c.allocator.clone()))
            .unwrap_or_default();
        if let Some(p) = &peer {
            unsafe {
                let _ = p.BeginFlush();
            }
        }
        // Release any delivery worker waiting for an allocator sample before joining it.
        if let Some(a) = allocator {
            unsafe {
                let _ = a.Decommit();
            }
        }
        if let Some(mut run) = self.running.lock().unwrap().take() {
            run.session.stop();
            if let Some(thread) = run.delivery.take() {
                let _ = thread.join();
            }
        }
        if let Some(p) = peer {
            unsafe {
                let _ = p.EndFlush();
            }
        }
        *self.applied.lock().unwrap() = None;
    }
    fn deliver(&self) -> Result<()> {
        let mut run = self.running.lock().unwrap();
        let Some(run) = run.as_mut() else {
            return Ok(());
        };
        if run.delivery.is_some() {
            return Ok(());
        }
        let (sink, allocator, peer) = {
            let connection = self.connections[1].lock().unwrap();
            let Some(c) = connection.as_ref() else {
                return Ok(());
            };
            (
                c.sink.clone().ok_or_else(unsupported)?,
                c.allocator.clone().ok_or_else(unsupported)?,
                c.pin.clone(),
            )
        };
        unsafe {
            allocator.Commit()?;
        }
        let Some(queue) = run.queue.take() else {
            return Ok(());
        };
        // DirectShow IMemInputPin is explicitly called from a source streaming thread.
        // Transfer owned references; the worker releases them before DLL unload.
        let delivery = Delivery {
            sink,
            allocator,
            peer,
            observer: self.observer.clone(),
            clock_recovery: self.clock_recovery,
        };
        let cancel = run.session.cancel.clone();
        let status = self.status.clone();
        run.delivery = Some(std::thread::spawn(move || {
            delivery.run(queue, cancel, status)
        }));
        Ok(())
    }
}
impl Drop for Shared {
    fn drop(&mut self) {
        self.stop_stream();
    }
}
struct Delivery {
    sink: IMemInputPin,
    allocator: IMemAllocator,
    peer: IPin,
    observer: Option<StreamObserver>,
    clock_recovery: Option<crate::clock_recovery::ClockRecoveryConfig>,
}
// A cumulative loss counter must mark a new gap once, not reset the demux
// on every subsequent delivery for the rest of the session.
#[derive(Default)]
struct Discontinuity(Option<u64>);
impl Discontinuity {
    fn changed(&mut self, dropped: u64) -> bool { self.0.replace(dropped) != Some(dropped) }
}
#[cfg(test)]
mod discontinuity_tests {
    #[test]
    fn one_transport_loss_does_not_poison_later_samples() {
        let mut state=super::Discontinuity::default();
        let flags=[0,0,188,188,188,376,376].map(|dropped|state.changed(dropped));
        assert_eq!(flags,[true,false,true,false,false,true,false]);
    }
}
unsafe impl Send for Delivery {}
impl Delivery {
    fn run(
        self,
        queue: Receiver<Vec<u8>>,
        cancel: Arc<std::sync::atomic::AtomicBool>,
        status: Arc<Status>,
    ) {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let mut clock_recovery = self
            .clock_recovery
            .map(crate::clock_recovery::ClockRecovery::new);
        let mut discontinuity = Discontinuity::default();
        let mut position = 0i64;
        let mut pending = Vec::new();
        while !cancel.load(Ordering::Relaxed) {
            let data = match queue.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(d) => d,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(_) => break,
            };
            if let Some(observe) = &self.observer {
                observe(&data);
            }
            let data = if let Some(clock) = &mut clock_recovery {
                let before = clock.generated;
                let processed = clock.push(&data);
                if before == 0 && clock.generated > 0 {
                    trace(format!(
                        "Playback clock recovery activated: {:?}",
                        self.clock_recovery
                    ));
                }
                processed
            } else {
                data
            };
            pending.extend_from_slice(&data);
            // Whole TS packets and disk sectors, for the standard File Writer sink.
            while pending.len() >= 188 * 256 && !cancel.load(Ordering::Relaxed) {
                let data: Vec<u8> = pending.drain(..188 * 256).collect();
                let result: Result<()> = (|| unsafe {
                    let mut sample = None;
                    self.allocator.GetBuffer(&mut sample, None, None, 0)?;
                    let sample = sample.ok_or_else(|| Error::from_hresult(E_FAIL))?;
                    if sample.GetSize() < data.len() as i32 {
                        return Err(Error::new(E_FAIL, "Allocator returned a short sample"));
                    }
                    let p = sample.GetPointer()?;
                    if p.is_null() {
                        return Err(Error::from_hresult(E_POINTER));
                    }
                    std::ptr::copy_nonoverlapping(data.as_ptr(), p, data.len());
                    sample.SetActualDataLength(data.len() as i32)?;
                    // MEDIATYPE_Stream uses byte offsets (required by the system File Writer).
                    let end = position + data.len() as i64;
                    sample.SetTime(Some(&position), Some(&end))?;
                    position = end;
                    sample.SetSyncPoint(true)?;
                    sample.SetDiscontinuity(discontinuity.changed(status.dropped.load(Ordering::Relaxed)))?;
                    self.sink.Receive(&sample)
                })();
                if let Err(e) = result {
                    if !cancel.load(Ordering::Relaxed) {
                        trace(format!("delivery error {e}"));
                        *status.error.lock().unwrap() = Some(e.to_string());
                    }
                    cancel.store(true, Ordering::Relaxed);
                    break;
                }
            }
        }
        unsafe {
            if !cancel.load(Ordering::Relaxed) {
                let _ = self.peer.EndOfStream();
            }
            let _ = self.allocator.Decommit();
            CoUninitialize();
        }
    }
}

#[implement(
    IBaseFilter,
    IAMFilterMiscFlags,
    IBDA_DeviceControl,
    IBDA_Topology,
    IBDA_FrequencyFilter,
    IBDA_DigitalDemodulator,
    IBDA_SignalStatistics,
    windows::Win32::Media::KernelStreaming::IKsPropertySet
)]
pub(crate) struct Filter {
    pub shared: Arc<Shared>,
    name: Mutex<String>,
    graph: Mutex<usize>,
    clock: Mutex<Option<IReferenceClock>>,
}
// DirectShow's graph owns its filters. Keep the back-reference non-owning to avoid
// a graph/filter cycle; the graph clears it with JoinFilterGraph(NULL) on removal.
unsafe impl Send for Filter {}
unsafe impl Sync for Filter {}
impl Drop for Filter {
    fn drop(&mut self) {
        trace("filter released");
        OBJECTS.fetch_sub(1, Ordering::SeqCst);
    }
}
pub(crate) fn create_external_filter(capture:bool)->IBaseFilter {create_filter_source(capture,None,None,None,None,true)}
pub fn create_filter(capture: bool) -> IBaseFilter {
    create_filter_source(capture, None, None, None, None, false)
}
/// Independent push source for native transport-stream playback; never opens USB.
pub fn create_replay_filter(path: std::path::PathBuf) -> IBaseFilter {
    create_replay_source(crate::backend::Replay {
        path,
        offset: 0,
        growing: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    })
}
pub fn create_replay_source(replay: crate::backend::Replay) -> IBaseFilter {
    create_filter_source(false, Some(replay), None, None, None, false)
}
pub fn create_video_filter(transform: Arc<dyn VideoTransform>) -> IBaseFilter {
    create_filter_source(true, None, Some(transform), None, None, false)
}
pub fn create_observed_source(
    replay: Option<crate::backend::Replay>,
    observer: StreamObserver,
) -> IBaseFilter {
    create_filter_source(false, replay, None, Some(observer), None, false)
}
/// Playback source with optional missing-PCR recovery. USB recordings remain raw.
pub fn create_clocked_source(
    replay: Option<crate::backend::Replay>,
    observer: StreamObserver,
    clock: Option<crate::clock_recovery::ClockRecoveryConfig>,
) -> IBaseFilter {
    create_filter_source(false, replay, None, Some(observer), clock, false)
}
fn create_filter_source(
    capture: bool,
    replay: Option<crate::backend::Replay>,
    video: Option<Arc<dyn VideoTransform>>,
    observer: Option<StreamObserver>,
    clock_recovery: Option<crate::clock_recovery::ClockRecoveryConfig>,
    external_signal: bool,
) -> IBaseFilter {
    trace(format!("create filter capture={capture}"));
    OBJECTS.fetch_add(1, Ordering::SeqCst);
    Filter {
        shared: Arc::new(Shared {
            capture,
            external_signal,
            replay,
            video,
            video_media: Mutex::new(None),
            video_output_media: Mutex::new(None),
            video_scratch: Mutex::new(Vec::new()),
            observer,
            clock_recovery,
            tune: Mutex::new(Tune::default()),
            committed: Mutex::new(Tune::default()),
            applied: Mutex::new(None),
            status: Arc::new(Status::default()),
            state: AtomicI32::new(0),
            pins: Mutex::new([Weak::new(), Weak::new()]),
            connections: [Mutex::new(None), Mutex::new(None)],
            running: Mutex::new(None),
            transition: Mutex::new(()),
        }),
        name: Mutex::new("AVer865 BDA Filter".into()),
        graph: Mutex::new(0),
        clock: Mutex::new(None),
    }
    .into()
}
impl Filter_Impl {
    fn pins(&self) -> Result<Vec<IPin>> {
        let parent: IBaseFilter = self.to_interface();
        let mut cache = self.shared.pins.lock().unwrap();
        let mut pins = Vec::new();
        let count = if self.shared.video.as_ref().is_some_and(|v| v.terminal()) {
            1
        } else {
            2
        };
        for i in 0..count {
            let pin = match cache[i].upgrade() {
                Some(p) => p,
                None => {
                    let p: IPin = Pin {
                        parent: parent.clone(),
                        shared: self.shared.clone(),
                        index: i,
                        allocator: Mutex::new(None),
                    }
                    .into();
                    cache[i] = p.downgrade()?;
                    p
                }
            };
            pins.push(pin);
        }
        Ok(pins)
    }
}
impl IPersist_Impl for Filter_Impl {
    fn GetClassID(&self) -> Result<GUID> {
        Ok(if self.shared.capture {
            CAPTURE_CLSID
        } else {
            TUNER_CLSID
        })
    }
}
impl IMediaFilter_Impl for Filter_Impl {
    fn Stop(&self) -> Result<()> {
        trace("Stop");
        if let Some(video) = &self.shared.video {
            video.stop();
        }
        self.shared.state.store(0, Ordering::Relaxed);
        let _g = self.shared.transition.lock().unwrap();
        self.shared.stop_stream();
        Ok(())
    }
    fn Pause(&self) -> Result<()> {
        trace("Pause");
        if let Some(video) = &self.shared.video {
            video.pause();
        }
        self.shared.state.store(1, Ordering::Relaxed);
        if self.shared.capture {
            if let Some(a) = self.shared.connections[1]
                .lock()
                .unwrap()
                .as_ref()
                .and_then(|c| c.allocator.clone())
            {
                unsafe {
                    a.Commit()?;
                }
            }
        }
        self.shared.ensure_stream()?;
        self.shared.deliver()
    }
    fn Run(&self, start: i64) -> Result<()> {
        trace("Run");
        if let Some(video) = &self.shared.video {
            video.run(start, self.clock.lock().unwrap().clone());
        }
        self.shared.state.store(2, Ordering::Relaxed);
        self.shared.ensure_stream()?;
        self.shared.deliver()
    }
    fn GetState(&self, _: u32) -> Result<FILTER_STATE> {
        Ok(FILTER_STATE(self.shared.state.load(Ordering::Relaxed)))
    }
    fn SetSyncSource(&self, c: Option<&IReferenceClock>) -> Result<()> {
        *self.clock.lock().unwrap() = c.cloned();
        Ok(())
    }
    fn GetSyncSource(&self) -> Result<IReferenceClock> {
        self.clock
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| Error::from_hresult(E_FAIL))
    }
}
impl IAMFilterMiscFlags_Impl for Filter_Impl {
    fn GetMiscFlags(&self) -> u32 {
        if self.shared.video.as_ref().is_some_and(|v| v.terminal()) {
            1
        } else {
            0
        }
    }
}
impl IBaseFilter_Impl for Filter_Impl {
    fn EnumPins(&self) -> Result<IEnumPins> {
        trace("EnumPins");
        Ok(Pins {
            items: self.pins()?,
            cursor: Mutex::new(0),
        }
        .into())
    }
    fn FindPin(&self, id: &PCWSTR) -> Result<IPin> {
        let s = unsafe { id.to_string()? };
        match s.as_str() {
            "0" | "Input" => Ok(self.pins()?[0].clone()),
            "1" | "Output" => self.pins()?.get(1).cloned().ok_or_else(invalid),
            _ => Err(invalid()),
        }
    }
    fn QueryFilterInfo(&self, info: *mut FILTER_INFO) -> Result<()> {
        if info.is_null() {
            return Err(Error::from_hresult(E_POINTER));
        }
        let mut v = FILTER_INFO::default();
        copy_name(&mut v.achName, &self.name.lock().unwrap());
        let graph = self.graph.lock().unwrap();
        let raw = *graph as *mut std::ffi::c_void;
        v.pGraph = ManuallyDrop::new(unsafe { IFilterGraph::from_raw_borrowed(&raw) }.cloned());
        unsafe {
            info.write(v);
        }
        Ok(())
    }
    fn JoinFilterGraph(&self, g: Option<&IFilterGraph>, name: &PCWSTR) -> Result<()> {
        trace("JoinFilterGraph");
        *self.graph.lock().unwrap() = g.map(|g| g.as_raw() as usize).unwrap_or(0);
        if !name.is_null() {
            *self.name.lock().unwrap() = unsafe { name.to_string()? };
        }
        Ok(())
    }
    fn QueryVendorInfo(&self) -> Result<PWSTR> {
        alloc_string("A865R Open Driver Project")
    }
}
fn copy_name(out: &mut [u16], name: &str) {
    let n = out.len().saturating_sub(1);
    for (o, c) in out.iter_mut().take(n).zip(name.encode_utf16()) {
        *o = c;
    }
}
fn alloc_string(s: &str) -> Result<PWSTR> {
    let bytes: Vec<u16> = s.encode_utf16().chain(Some(0)).collect();
    unsafe {
        let p = CoTaskMemAlloc(bytes.len() * 2) as *mut u16;
        if p.is_null() {
            return Err(Error::from_hresult(E_OUTOFMEMORY));
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len());
        Ok(PWSTR(p))
    }
}
fn media(antenna: bool) -> AM_MEDIA_TYPE {
    AM_MEDIA_TYPE {
        majortype: if antenna {
            KSDATAFORMAT_TYPE_BDA_ANTENNA
        } else {
            MEDIATYPE_Stream
        },
        subtype: if antenna {
            windows::Win32::Media::MediaFoundation::MEDIASUBTYPE_None
        } else {
            MEDIASUBTYPE_MPEG2_TRANSPORT
        },
        bFixedSizeSamples: BOOL(0),
        bTemporalCompression: BOOL(0),
        lSampleSize: 188,
        formattype: FORMAT_None,
        ..Default::default()
    }
}
fn accepts(p: *const AM_MEDIA_TYPE, antenna: bool) -> bool {
    if p.is_null() {
        return false;
    }
    let m = unsafe { &*p };
    let expected = media(antenna);
    m.majortype == expected.majortype
        && (m.subtype == expected.subtype || (antenna && m.subtype == GUID::zeroed()))
        && m.cbFormat == 0
}

#[implement(
    IPin,
    IMemInputPin,
    IBDA_PinControl,
    windows::Win32::Media::KernelStreaming::IKsPin
)]
pub(crate) struct Pin {
    pub parent: IBaseFilter,
    pub shared: Arc<Shared>,
    pub index: usize,
    pub allocator: Mutex<Option<IMemAllocator>>,
}
unsafe impl Send for Pin {}
unsafe impl Sync for Pin {}
impl Pin_Impl {
    fn accepts_type(&self, p: *const AM_MEDIA_TYPE) -> bool {
        if p.is_null() {
            return false;
        }
        if let Some(video) = &self.shared.video {
            video.accepts(unsafe { &*p })
        } else {
            accepts(p, self.antenna())
        }
    }
    fn antenna(&self) -> bool {
        self.index == 0 && !self.shared.capture
    }
}
impl IPin_Impl for Pin_Impl {
    fn Connect(&self, peer: Option<&IPin>, pmt: *const AM_MEDIA_TYPE) -> Result<()> {
        trace(format!("pin {} Connect", self.index));
        if self.index != 1 {
            return Err(invalid());
        }
        if self.shared.state.load(Ordering::Relaxed) != 0 {
            return Err(Error::from_hresult(VFW_E_NOT_STOPPED));
        }
        if self.shared.connections[1].lock().unwrap().is_some() {
            return Err(Error::from_hresult(VFW_E_ALREADY_CONNECTED));
        }
        let peer = peer.ok_or_else(invalid)?;
        let mt = if self.shared.video.is_some() {
            self.shared
                .video_media
                .lock()
                .unwrap()
                .as_ref()
                .ok_or_else(invalid)?
                .view()
        } else {
            media(false)
        };
        if !pmt.is_null() && !self.accepts_type(pmt) {
            return Err(Error::from_hresult(VFW_E_TYPE_NOT_ACCEPTED));
        }
        let me: IPin = self.to_interface();
        unsafe {
            peer.ReceiveConnection(&me, &mt)?;
            if self.shared.video.is_some() {
                *self.shared.video_output_media.lock().unwrap() = Some(OwnedMedia::new(&mt)?);
            }
        }
        let setup = (|| -> Result<Connection> {
            unsafe {
                let sink: IMemInputPin = peer.cast()?;
                let allocator: IMemAllocator = sink.GetAllocator().or_else(|_| {
                    CoCreateInstance(&CLSID_MemoryAllocator, None, CLSCTX_INPROC_SERVER)
                })?;
                let requirements = sink.GetAllocatorRequirements().unwrap_or_default();
                let wanted = ALLOCATOR_PROPERTIES {
                    cBuffers: requirements.cBuffers.max(8),
                    cbBuffer: requirements.cbBuffer.max(
                        self.shared
                            .video
                            .as_ref()
                            .map(|v| v.allocator_bytes() as i32)
                            .unwrap_or(188 * 305),
                    ),
                    cbAlign: requirements.cbAlign.max(1),
                    cbPrefix: requirements.cbPrefix.max(0),
                };
                let actual = allocator.SetProperties(&wanted)?;
                if actual.cbBuffer < wanted.cbBuffer || actual.cBuffers < 1 {
                    return Err(Error::from_hresult(E_FAIL));
                }
                sink.NotifyAllocator(&allocator, false)?;
                Ok(Connection {
                    pin: peer.clone(),
                    sink: Some(sink),
                    allocator: Some(allocator),
                })
            }
        })();
        match setup {
            Ok(c) => {
                *self.shared.connections[1].lock().unwrap() = Some(c);
                Ok(())
            }
            Err(e) => {
                unsafe {
                    let _ = peer.Disconnect();
                }
                Err(e)
            }
        }
    }
    fn ReceiveConnection(&self, peer: Option<&IPin>, mt: *const AM_MEDIA_TYPE) -> Result<()> {
        trace(format!("pin {} ReceiveConnection", self.index));
        if self.index != 0 {
            return Err(invalid());
        }
        let peer=peer.ok_or_else(invalid)?;
        let mut c=self.shared.connections[0].lock().unwrap();
        if let Some(existing)=c.as_ref() {
            // DirectShow decoders renegotiate surface pitch/size through
            // ReceiveConnection on the SAME output pin. Terminal renderers can
            // accept this; a different peer or passthrough graph cannot replace
            // an established connection or mutate its active format.
            let same_peer=existing.pin.cast::<IUnknown>()? == peer.cast::<IUnknown>()?;
            if !same_peer || !self.shared.video.as_ref().is_some_and(|v|v.terminal()) {
                return Err(Error::from_hresult(VFW_E_ALREADY_CONNECTED));
            }
        }
        if !self.accepts_type(mt) {return Err(Error::from_hresult(VFW_E_TYPE_NOT_ACCEPTED));}
        if let Some(video)=&self.shared.video {
            unsafe {
                let owned=OwnedMedia::new(&*mt)?;
                video.configure(&owned.view())?;
                *self.shared.video_media.lock().unwrap()=Some(owned);
            }
        }
        if c.is_none() {
            *c=Some(Connection{pin:peer.clone(),sink:None,allocator:None});
        }
        Ok(())
    }
    fn Disconnect(&self) -> Result<()> {
        if self.shared.state.load(Ordering::Relaxed) != 0 {
            return Err(Error::from_hresult(VFW_E_NOT_STOPPED));
        }
        self.shared.connections[self.index].lock().unwrap().take();
        Ok(())
    }
    fn ConnectedTo(&self) -> Result<IPin> {
        self.shared.connections[self.index]
            .lock()
            .unwrap()
            .as_ref()
            .map(|c| c.pin.clone())
            .ok_or_else(|| Error::from_hresult(VFW_E_NOT_CONNECTED))
    }
    fn ConnectionMediaType(&self, p: *mut AM_MEDIA_TYPE) -> Result<()> {
        self.ConnectedTo()?;
        if p.is_null() {
            return Err(Error::from_hresult(E_POINTER));
        }
        unsafe {
            p.write(if self.shared.video.is_some() {
                (if self.index == 0 {
                    &self.shared.video_media
                } else {
                    &self.shared.video_output_media
                })
                .lock()
                .unwrap()
                .as_ref()
                .ok_or_else(invalid)?
                .export()?
            } else {
                media(self.antenna())
            });
        }
        Ok(())
    }
    fn QueryPinInfo(&self, p: *mut PIN_INFO) -> Result<()> {
        if p.is_null() {
            return Err(Error::from_hresult(E_POINTER));
        }
        let mut v = PIN_INFO::default();
        v.pFilter = ManuallyDrop::new(Some(self.parent.clone()));
        v.dir = PIN_DIRECTION(self.index as i32);
        copy_name(
            &mut v.achName,
            if self.index == 0 { "Input" } else { "Output" },
        );
        unsafe { p.write(v) };
        Ok(())
    }
    fn QueryDirection(&self) -> Result<PIN_DIRECTION> {
        trace(format!(
            "QueryDirection capture={} pin={}",
            self.shared.capture, self.index
        ));
        Ok(PIN_DIRECTION(self.index as i32))
    }
    fn QueryId(&self) -> Result<PWSTR> {
        alloc_string(if self.index == 0 { "0" } else { "1" })
    }
    fn QueryAccept(&self, p: *const AM_MEDIA_TYPE) -> HRESULT {
        if self.accepts_type(p) {
            S_OK
        } else {
            S_FALSE
        }
    }
    fn EnumMediaTypes(&self) -> Result<IEnumMediaTypes> {
        trace(format!("EnumMediaTypes antenna={}", self.antenna()));
        Ok(Types::create(
            self.antenna(),
            if self.shared.video.is_some() { 1 } else { 0 },
        ))
    }
    fn QueryInternalConnections(&self, _: *mut Option<IPin>, _: *mut u32) -> Result<()> {
        Err(unsupported())
    }
    fn EndOfStream(&self) -> Result<()> {
        if let Some(video) = &self.shared.video {
            if video.terminal() {
                video.end_of_stream();
                return Ok(());
            }
        }
        if self.index == 0 && self.shared.capture {
            if let Some(c) = self.shared.connections[1].lock().unwrap().as_ref() {
                unsafe { c.pin.EndOfStream()? }
            }
        }
        Ok(())
    }
    fn BeginFlush(&self) -> Result<()> {
        if let Some(video) = &self.shared.video {
            if video.terminal() {
                video.begin_flush();
                return Ok(());
            }
        }
        if self.index == 0 && self.shared.capture {
            if let Some(c) = self.shared.connections[1].lock().unwrap().as_ref() {
                unsafe { c.pin.BeginFlush()? }
            }
        }
        Ok(())
    }
    fn EndFlush(&self) -> Result<()> {
        if let Some(video) = &self.shared.video {
            if video.terminal() {
                video.end_flush();
                return Ok(());
            }
        }
        if self.index == 0 && self.shared.capture {
            if let Some(c) = self.shared.connections[1].lock().unwrap().as_ref() {
                unsafe { c.pin.EndFlush()? }
            }
        }
        Ok(())
    }
    fn NewSegment(&self, a: i64, b: i64, r: f64) -> Result<()> {
        if self.index == 0 && self.shared.capture {
            if let Some(c) = self.shared.connections[1].lock().unwrap().as_ref() {
                unsafe { c.pin.NewSegment(a, b, r)? }
            }
        }
        Ok(())
    }
}
impl IMemInputPin_Impl for Pin_Impl {
    fn GetAllocator(&self) -> Result<IMemAllocator> {
        let mut a = self.allocator.lock().unwrap();
        if a.is_none() {
            *a = Some(unsafe {
                CoCreateInstance(&CLSID_MemoryAllocator, None, CLSCTX_INPROC_SERVER)?
            });
        }
        Ok(a.as_ref().unwrap().clone())
    }
    fn NotifyAllocator(&self, a: Option<&IMemAllocator>, _: BOOL) -> Result<()> {
        *self.allocator.lock().unwrap() = Some(a.ok_or_else(invalid)?.clone());
        Ok(())
    }
    fn GetAllocatorRequirements(&self) -> Result<ALLOCATOR_PROPERTIES> {
        Ok(ALLOCATOR_PROPERTIES {
            cBuffers: 8,
            cbBuffer: self
                .shared
                .video
                .as_ref()
                .map(|v| v.allocator_bytes() as i32)
                .unwrap_or(188 * 305),
            cbAlign: 1,
            cbPrefix: 0,
        })
    }
    fn Receive(&self, s: Option<&IMediaSample>) -> Result<()> {
        if self.index != 0 || !self.shared.capture {
            return Err(unsupported());
        }
        if let Some(video) = &self.shared.video {
            if video.terminal() {
                let sample = s.ok_or_else(invalid)?;
                unsafe {
                    if let Some(mt) = sample_media(sample)? {
                        video.configure(&mt.view())?;
                        *self.shared.video_media.lock().unwrap() = Some(mt);
                    }
                }
                return video.receive(sample);
            }
        }
        let (sink, allocator) = {
            let c = self.shared.connections[1].lock().unwrap();
            let c = c
                .as_ref()
                .ok_or_else(|| Error::from_hresult(VFW_E_NOT_CONNECTED))?;
            (
                c.sink.clone().ok_or_else(unsupported)?,
                c.allocator.clone().ok_or_else(unsupported)?,
            )
        };
        unsafe {
            let source = s.ok_or_else(invalid)?;
            let n = source.GetActualDataLength();
            if n < 0 || n > source.GetSize() {
                return Err(invalid());
            }
            let mut sample = None;
            allocator.GetBuffer(&mut sample, None, None, 0)?;
            let sample = sample.ok_or_else(unsupported)?;
            if sample.GetSize() < n {
                return Err(invalid());
            }
            let input_ptr = source.GetPointer()?;
            let output_ptr = sample.GetPointer()?;
            if input_ptr.is_null() || output_ptr.is_null() { return Err(invalid()); }
            let mut actual = n;
            if let Some(video) = &self.shared.video {
                // Input and EVR output formats are independent: EVR announces
                // its aligned surface pitch on the first allocator sample.
                if !video.requires_nv12_repack() { sample.SetMediaType(std::ptr::null())?; }
                if let Some(mt) = sample_media(source)? {
                    video.configure(&mt.view())?;
                    // PCM can switch between stereo and 5.1 after AAC headers arrive.
                    // Forward the format with the same sample, before its bytes reach the renderer.
                    if !video.requires_nv12_repack() {
                        sample.SetMediaType(&mt.view())?;
                        *self.shared.video_output_media.lock().unwrap() = Some(OwnedMedia::new(&mt.view())?);
                    }
                    *self.shared.video_media.lock().unwrap() = Some(mt);
                }
                if let Some(mt) = sample_media(&sample)? {
                    *self.shared.video_output_media.lock().unwrap() = Some(mt);
                }
                if video.requires_nv12_repack() {
                    let input = self.shared.video_media.lock().unwrap();
                    let output = self.shared.video_output_media.lock().unwrap();
                    let (iw,ih)=nv12_shape(&input.as_ref().ok_or_else(invalid)?.view())?;
                    let (ow,oh)=nv12_shape(&output.as_ref().ok_or_else(invalid)?.view())?;
                    let mut scratch=self.shared.video_scratch.lock().unwrap();
                    actual=prepare_nv12_frame(std::slice::from_raw_parts(input_ptr,n as usize),&mut scratch,
                        [iw,ih,ow,oh],sample.GetSize() as usize,|bytes|video.process(bytes))? as i32;
                    std::ptr::copy_nonoverlapping(scratch.as_ptr(),output_ptr,actual as usize);
                } else {
                    std::ptr::copy_nonoverlapping(input_ptr,output_ptr,n as usize);
                    video.process(std::slice::from_raw_parts_mut(output_ptr,n as usize))?;
                }
            } else {
                std::ptr::copy_nonoverlapping(input_ptr,output_ptr,n as usize);
            }
            sample.SetActualDataLength(actual)?;
            let (mut start, mut end) = (0, 0);
            if source.GetTime(&mut start, &mut end).is_ok() {sample.SetTime(Some(&start), Some(&end))?;} else {sample.SetTime(None,None)?;}
            sample.SetSyncPoint(source.IsSyncPoint() == S_OK)?;
            sample.SetDiscontinuity(source.IsDiscontinuity() == S_OK)?;
            sink.Receive(&sample)?;
            if let Some(video)=&self.shared.video {video.delivered(&sample);}
            Ok(())
        }
    }
    fn ReceiveMultiple(&self, s: *const Option<IMediaSample>, n: i32) -> Result<i32> {
        if s.is_null() || n < 0 {
            return Err(invalid());
        }
        for i in 0..n {
            self.Receive(unsafe { (*s.add(i as usize)).as_ref() })?;
        }
        Ok(n)
    }
    fn ReceiveCanBlock(&self) -> Result<()> {
        Ok(())
    }
}
#[implement(IEnumPins)]
struct Pins {
    items: Vec<IPin>,
    cursor: Mutex<usize>,
}
impl IEnumPins_Impl for Pins_Impl {
    fn Next(&self, n: u32, out: *mut Option<IPin>, f: *mut u32) -> HRESULT {
        if out.is_null() || (n != 1 && f.is_null()) {
            return E_POINTER;
        }
        let mut pos = self.cursor.lock().unwrap();
        let count = (n as usize).min(self.items.len().saturating_sub(*pos));
        unsafe {
            for i in 0..count {
                out.add(i).write(Some(self.items[*pos + i].clone()));
            }
            if !f.is_null() {
                *f = count as u32
            }
        }
        *pos += count;
        if count == n as usize {
            S_OK
        } else {
            S_FALSE
        }
    }
    fn Skip(&self, n: u32) -> Result<()> {
        let mut p = self.cursor.lock().unwrap();
        *p = (*p + n as usize).min(self.items.len());
        Ok(())
    }
    fn Reset(&self) -> Result<()> {
        *self.cursor.lock().unwrap() = 0;
        Ok(())
    }
    fn Clone(&self) -> Result<IEnumPins> {
        Ok(Pins {
            items: self.items.clone(),
            cursor: Mutex::new(*self.cursor.lock().unwrap()),
        }
        .into())
    }
}
#[implement(IEnumMediaTypes)]
struct Types {
    antenna: bool,
    cursor: Mutex<usize>,
}
impl Types {
    fn create(antenna: bool, cursor: usize) -> IEnumMediaTypes {
        OBJECTS.fetch_add(1, Ordering::SeqCst);
        Self {
            antenna,
            cursor: Mutex::new(cursor),
        }
        .into()
    }
}
impl Drop for Types {
    fn drop(&mut self) {
        OBJECTS.fetch_sub(1, Ordering::SeqCst);
    }
}
impl IEnumMediaTypes_Impl for Types_Impl {
    fn Next(&self, n: u32, out: *mut *mut AM_MEDIA_TYPE, f: *mut u32) -> HRESULT {
        if out.is_null() || (n != 1 && f.is_null()) {
            return E_POINTER;
        }
        let mut pos = self.cursor.lock().unwrap();
        unsafe {
            if !f.is_null() {
                *f = 0
            }
            if n == 0 {
                return S_OK;
            }
            if *pos != 0 {
                return S_FALSE;
            }
            let p = CoTaskMemAlloc(size_of::<AM_MEDIA_TYPE>()) as *mut AM_MEDIA_TYPE;
            if p.is_null() {
                return E_OUTOFMEMORY;
            }
            p.write(media(self.antenna));
            *out = p;
            if !f.is_null() {
                *f = 1
            }
        }
        *pos = 1;
        if n == 1 {
            S_OK
        } else {
            S_FALSE
        }
    }
    fn Skip(&self, n: u32) -> Result<()> {
        if n > 0 {
            *self.cursor.lock().unwrap() = 1;
        }
        Ok(())
    }
    fn Reset(&self) -> Result<()> {
        *self.cursor.lock().unwrap() = 0;
        Ok(())
    }
    fn Clone(&self) -> Result<IEnumMediaTypes> {
        Ok(Types::create(self.antenna, *self.cursor.lock().unwrap()))
    }
}

#[cfg(test)] mod reconnect_tests {
    use super::*;
    struct Format {size:Mutex<u32>,terminal:bool}
    impl VideoTransform for Format {
        fn accepts(&self,mt:&AM_MEDIA_TYPE)->bool{mt.lSampleSize>0 && mt.lSampleSize<=4096}
        fn configure(&self,mt:&AM_MEDIA_TYPE)->Result<()>{*self.size.lock().unwrap()=mt.lSampleSize;Ok(())}
        fn bytes_required(&self)->usize{*self.size.lock().unwrap() as usize}
        fn process(&self,_:&mut[u8])->Result<()>{Ok(())}
        fn terminal(&self)->bool{self.terminal}
    }
    unsafe fn pin(filter:&IBaseFilter,dir:PIN_DIRECTION)->IPin {
        let e=filter.EnumPins().unwrap();loop{let mut p=[None];assert!(e.Next(&mut p,None).is_ok());let p=p[0].take().unwrap();if p.QueryDirection().unwrap()==dir{return p;}}
    }
    #[test] fn terminal_reconnect_updates_format_only_for_its_existing_peer(){unsafe{
        for terminal in [true,false] {
            let transform=Arc::new(Format{size:Mutex::new(0),terminal});
            let filter=create_video_filter(transform.clone());let input=pin(&filter,PINDIR_INPUT);
            let source=create_filter(false);let peer=pin(&source,PINDIR_OUTPUT);
            let other=create_filter(false);let stranger=pin(&other,PINDIR_OUTPUT);
            let mut mt=AM_MEDIA_TYPE::default();mt.lSampleSize=128;
            input.ReceiveConnection(&peer,&mt).unwrap();assert_eq!(transform.bytes_required(),128);
            mt.lSampleSize=512;
            let result=input.ReceiveConnection(&peer,&mt);
            if terminal {result.unwrap();assert_eq!(transform.bytes_required(),512);}else{assert_eq!(result.unwrap_err().code(),VFW_E_ALREADY_CONNECTED);}
            let before=transform.bytes_required();mt.lSampleSize=1024;
            assert_eq!(input.ReceiveConnection(&stranger,&mt).unwrap_err().code(),VFW_E_ALREADY_CONNECTED);
            assert_eq!(transform.bytes_required(),before);
            mt.lSampleSize=8192;assert!(input.ReceiveConnection(&peer,&mt).is_err());assert_eq!(transform.bytes_required(),before);
            assert_eq!(input.ConnectedTo().unwrap().cast::<IUnknown>().unwrap(),peer.cast::<IUnknown>().unwrap());
            input.Disconnect().unwrap();
        }
    }}
}
