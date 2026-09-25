//! H.264 elementary-stream input for Vulkan Video on the presentation device.
//! The parser and decoder stay on one worker; only compressed bytes and owned
//! GPU textures cross threads. Audio retains the DirectShow reference clock.
use a865r_bda::VideoTransform;
use serde_json::json;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, SyncSender, TrySendError},
        Arc,
    },
    time::Duration,
};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Media::{DirectShow::*, IReferenceClock, MediaFoundation::*},
    },
};
fn error(e: impl std::fmt::Display) -> Error {
    Error::new(E_FAIL, e.to_string())
}
// Allow interleaved audio to reach the demux while decoded video waits for its
// timestamps. RBI sends two compressed field pictures per output frame.
const INPUT_PACKETS: usize = 64;
const INPUT_BYTES: usize = 8 * 1024 * 1024;
struct InputBytes { used: AtomicUsize, peak: AtomicUsize }
struct Reservation { budget: Arc<InputBytes>, bytes: usize }
impl InputBytes {
    fn reserve(self: &Arc<Self>, bytes: usize) -> Option<Reservation> {
        self.used.fetch_update(Ordering::AcqRel, Ordering::Acquire, |used|
            used.checked_add(bytes).filter(|total| *total <= INPUT_BYTES)).ok().map(|used| {
                self.peak.fetch_max(used + bytes, Ordering::Relaxed);
                Reservation { budget: self.clone(), bytes }
            })
    }
}
impl Drop for Reservation {
    fn drop(&mut self) { self.budget.used.fetch_sub(self.bytes, Ordering::AcqRel); }
}
struct Packet {
    bytes: Vec<u8>,
    pts: Option<i64>,
    epoch: u64,
    eos: bool,
    reservation: Option<Reservation>,
}
pub struct Filter {
    tx: SyncSender<Packet>,
    budget: Arc<InputBytes>,
    output: Arc<crate::vulkan::Input>,
    quit: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl Filter {
    pub fn new(
        device: Arc<gpu_video::VulkanDevice>,
        output: Arc<crate::vulkan::Input>,
        folder: PathBuf,
    ) -> Result<Arc<Self>> {
        let (tx, rx) = mpsc::sync_channel::<Packet>(INPUT_PACKETS);
        let (ready_tx, ready) = mpsc::sync_channel(1);
        let quit = Arc::new(AtomicBool::new(false));
        let cancel = quit.clone();
        let budget = Arc::new(InputBytes { used: AtomicUsize::new(0), peak: AtomicUsize::new(0) });
        let queue_budget = budget.clone();
        let state = output.clone();
        let worker=std::thread::Builder::new().name("Vulkan H264 decoder".into()).spawn(move || {
            let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<()> {
                // Construct the non-Send native parser on its owning thread.
                let mut decoder=gpu_video::broadcast::Decoder::new(&device).map_err(error)?;
                state.report.lock().unwrap()["decoder_thread_id"]=json!(unsafe{windows::Win32::System::Threading::GetCurrentThreadId()});
                let _=ready_tx.send(Ok(()));
                let mut epoch=state.epoch(); let mut frames=0u64; let mut packets=0u64;
                while !cancel.load(Ordering::Relaxed) && !state.has_failed() {
                    let packet=match rx.recv_timeout(Duration::from_millis(50)) {
                        Ok(packet)=>packet,
                        Err(mpsc::RecvTimeoutError::Timeout)=>continue,
                        Err(mpsc::RecvTimeoutError::Disconnected)=>break,
                    };
                    if packet.epoch!=state.epoch() {continue;}
                    if packet.epoch!=epoch {
                        decoder=gpu_video::broadcast::Decoder::new(&device).map_err(error)?;
                        epoch=packet.epoch;
                    }
                    if !packet.eos {
                        packets+=1;
                        let mut metrics=state.report.lock().unwrap();
                        if !metrics["decoder"].is_object() {metrics["decoder"]=json!({});}
                        metrics["decoder"]["compressed_packets"]=json!(packets);
                    }
                    let decoded=if packet.eos {decoder.flush()} else {decoder.decode(&packet.bytes,packet.pts)}.map_err(error)?;
                    for frame in decoded {
                        if cancel.load(Ordering::Relaxed) || epoch!=state.epoch() {break;}
                        state.gpu_frame(frame,epoch)?; frames+=1;
                    }
                    if packet.eos && epoch==state.epoch() {state.end_of_stream();}
                    let report=json!({"backend":"Vulkan Video H.264","hardware_decoded_frames":frames,"compressed_packets":packets,
                        "input_queue_bytes":queue_budget.used.load(Ordering::Relaxed),"input_queue_peak_bytes":queue_budget.peak.load(Ordering::Relaxed),"input_queue_byte_limit":INPUT_BYTES,
                        "parsed_pictures":decoder.parsed_pictures,"skipped_startup_pictures":decoder.skipped_startup_pictures,
                        "decoded_pictures":decoder.decoded_pictures,"decoded_fields":decoder.decoded_fields,
                        "gpu_readback_per_frame":false,"cpu_nv12_bridge":false,"shared_vulkan_device":true});
                    {let mut metrics=state.report.lock().unwrap();metrics["decoder"]=report.clone();}
                    if packets==1 || packets%30==0 {let _=std::fs::write(folder.join("native-decoder.json"),report.to_string());}
                }
                Ok(())
            })).unwrap_or_else(|payload|{
                let detail=payload.downcast_ref::<String>().map(String::as_str)
                    .or_else(||payload.downcast_ref::<&str>().copied()).unwrap_or("non-text panic payload");
                Err(error(format!("Vulkan decoder failed: {detail}")))
            });
            if let Err(e)=result {
                use std::io::Write;
                let message=e.to_string();
                if let Ok(mut log)=std::fs::File::create(folder.join("native-decoder-error.txt")) {
                    let _=log.write_all(message.as_bytes());let _=log.sync_all();
                }
                let _=ready_tx.send(Err(message.clone()));state.fail(message);
            }
        }).map_err(error)?;
        match ready
            .recv_timeout(Duration::from_secs(30))
            .map_err(error)
            .and_then(|r| r.map_err(error))
        {
            Ok(()) => Ok(Arc::new(Self {
                tx,
                budget,
                output,
                quit,
                worker: Some(worker),
            })),
            Err(e) => {
                quit.store(true, Ordering::Relaxed);
                output.stop();
                let _ = worker.join();
                Err(e)
            }
        }
    }
    fn send(&self, mut packet: Packet) -> Result<()> {
        loop {
            if self.quit.load(Ordering::Relaxed)
                || !self.output.accepting()
                || packet.epoch != self.output.epoch()
            {
                return Ok(());
            }
            if packet.reservation.is_none() {
                packet.reservation = self.budget.reserve(packet.bytes.len());
                if packet.reservation.is_none() {
                    std::thread::sleep(Duration::from_millis(2));
                    continue;
                }
            }
            match self.tx.try_send(packet) {
                Ok(()) => return Ok(()),
                Err(TrySendError::Full(p)) => {
                    packet = p;
                    std::thread::sleep(Duration::from_millis(2));
                }
                Err(TrySendError::Disconnected(_)) => {
                    return Err(error("Vulkan decoder worker stopped"))
                }
            }
        }
    }
}
impl Drop for Filter {
    fn drop(&mut self) {
        self.quit.store(true, Ordering::Relaxed);
        self.output.stop();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
impl VideoTransform for Filter {
    fn accepts(&self, mt: &AM_MEDIA_TYPE) -> bool {
        mt.majortype == MEDIATYPE_Video && mt.subtype == MEDIASUBTYPE_H264
    }
    fn configure(&self, mt: &AM_MEDIA_TYPE) -> Result<()> {
        if self.accepts(mt) {
            Ok(())
        } else {
            Err(Error::from_hresult(VFW_E_TYPE_NOT_ACCEPTED))
        }
    }
    fn bytes_required(&self) -> usize {
        4 * 1024 * 1024
    }
    fn process(&self, _: &mut [u8]) -> Result<()> {
        Ok(())
    }
    fn terminal(&self) -> bool {
        true
    }
    fn receive(&self, sample: &IMediaSample) -> Result<()> {
        // SAFETY: DirectShow owns the sample throughout this synchronous call.
        // Check its actual length against the allocator capacity before copying.
        unsafe {
            let count = sample.GetActualDataLength();
            if count == 0 {
                return Ok(());
            }
            if count < 0 || count > sample.GetSize() || count > 4 * 1024 * 1024 {
                return Err(error("Invalid H264 sample length"));
            }
            let pointer = sample.GetPointer()?;
            if pointer.is_null() {
                return Err(error("Null H264 sample"));
            }
            let (mut start, mut end) = (0, 0);
            let pts = sample.GetTime(&mut start, &mut end).ok().map(|_| start);
            self.send(Packet {
                bytes: std::slice::from_raw_parts(pointer, count as usize).to_vec(),
                pts,
                epoch: self.output.epoch(),
                eos: false,
                reservation: None,
            })
        }
    }
    fn run(&self, start: i64, clock: Option<IReferenceClock>) {
        self.output.run(start, clock);
    }
    fn pause(&self) {
        self.output.pause();
    }
    fn stop(&self) {
        self.output.stop();
    }
    fn begin_flush(&self) {self.output.begin_flush();}
    fn end_flush(&self) {
        self.output.end_flush();
    }
    fn end_of_stream(&self) {
        let _ = self.send(Packet {
            bytes: Vec::new(),
            pts: None,
            epoch: self.output.epoch(),
            eos: true,
            reservation: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compressed_queue_is_byte_bounded_and_releases_cancelled_packets() {
        let budget=Arc::new(InputBytes{used:AtomicUsize::new(0),peak:AtomicUsize::new(0)});
        let a=budget.reserve(INPUT_BYTES/2).unwrap();
        let b=budget.reserve(INPUT_BYTES/2).unwrap();
        assert!(budget.reserve(1).is_none());
        drop(a);
        let (tx,rx)=mpsc::sync_channel(1);
        tx.send(Packet{bytes:vec![],pts:None,epoch:0,eos:false,reservation:budget.reserve(INPUT_BYTES/2)}).unwrap();
        drop(rx); // A cancelled decoder releases queued memory reservations.
        drop(b);
        assert_eq!(budget.used.load(Ordering::Acquire),0);
        assert_eq!(budget.peak.load(Ordering::Relaxed),INPUT_BYTES);
        assert!(budget.reserve(usize::MAX).is_none());
    }
}
