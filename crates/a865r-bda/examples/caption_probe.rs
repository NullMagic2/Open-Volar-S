//! CPU-only regression harness: no decoder, renderer, GPU device, or tuner is opened.
use a865r_bda::VideoTransform;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Media::{DirectShow::*, MediaFoundation::*},
        System::Com::*,
    },
};
#[derive(Default)]
struct Sink {
    samples: AtomicU64,
    bytes: AtomicU64,
    times: Mutex<Vec<i64>>,
    done: AtomicBool,
}
impl VideoTransform for Sink {
    fn accepts(&self, m: &AM_MEDIA_TYPE) -> bool {
        m.majortype == MEDIATYPE_Stream
    }
    fn configure(&self, _: &AM_MEDIA_TYPE) -> Result<()> {
        Ok(())
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
    fn receive(&self, s: &IMediaSample) -> Result<()> {
        unsafe {
            let count = self.samples.fetch_add(1, Ordering::Relaxed);
            if count < 3 && s.GetActualDataLength() > 0 && s.GetActualDataLength() <= s.GetSize() {
                let p = s.GetPointer()?;
                if !p.is_null() {
                    println!(
                        "payload {:?}",
                        std::slice::from_raw_parts(p, (s.GetActualDataLength() as usize).min(24))
                    );
                }
            }
            self.bytes
                .fetch_add(s.GetActualDataLength().max(0) as u64, Ordering::Relaxed);
            let (mut a, mut b) = (0, 0);
            if s.GetTime(&mut a, &mut b).is_ok() {
                self.times.lock().unwrap().push(a);
            }
            Ok(())
        }
    }
    fn end_of_stream(&self) {
        self.done.store(true, Ordering::Relaxed);
    }
}
unsafe fn pin(f: &IBaseFilter, d: PIN_DIRECTION) -> Result<IPin> {
    let e = f.EnumPins()?;
    loop {
        let mut p = [None];
        if e.Next(&mut p, None) != S_OK {
            return Err(Error::from_hresult(E_FAIL));
        }
        let p = p[0].take().unwrap();
        if p.QueryDirection()? == d {
            return Ok(p);
        }
    }
}
fn main() -> Result<()> {
    unsafe {
        let args: Vec<String> = std::env::args().collect();
        let path = args.get(1).expect("TS file");
        let pid: u32 = args.get(2).expect("caption PID").parse().unwrap();
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        let graph: IGraphBuilder =
            CoCreateInstance(&CLSID_FilterGraph, None, CLSCTX_INPROC_SERVER)?;
        let source = a865r_bda::create_replay_filter(path.into());
        graph.AddFilter(&source, w!("Recorded transport"))?;
        let demux: IBaseFilter =
            CoCreateInstance(&CLSID_MPEG2Demultiplexer, None, CLSCTX_INPROC_SERVER)?;
        graph.AddFilter(&demux, w!("Windows demultiplexer"))?;
        graph.ConnectDirect(
            &pin(&source, PINDIR_OUTPUT)?,
            &pin(&demux, PINDIR_INPUT)?,
            None,
        )?;
        let mt = AM_MEDIA_TYPE {
            majortype: MEDIATYPE_Stream,
            formattype: FORMAT_None,
            ..Default::default()
        };
        let out = demux
            .cast::<IMpeg2Demultiplexer>()?
            .CreateOutputPin(&mt, w!("ISDB captions"))?;
        out.cast::<IMPEG2PIDMap>()?
            .MapPID(1, &pid, MEDIA_ELEMENTARY_STREAM)?;
        let sink = Arc::new(Sink::default());
        let filter = a865r_bda::create_video_filter(sink.clone());
        graph.AddFilter(&filter, w!("Compressed counter - no GPU"))?;
        graph.ConnectDirect(&out, &pin(&filter, PINDIR_INPUT)?, None)?;
        graph.cast::<IMediaFilter>()?.SetSyncSource(None)?;
        let media: IMediaControl = graph.cast()?;
        media.Run()?;
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(3) && !sink.done.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(20));
        }
        media.Stop()?;
        let t = sink.times.lock().unwrap();
        println!(
            "samples={} bytes={} timestamps={} first={:?} last={:?} eos={}",
            sink.samples.load(Ordering::Relaxed),
            sink.bytes.load(Ordering::Relaxed),
            t.len(),
            t.first(),
            t.last(),
            sink.done.load(Ordering::Relaxed)
        );
        Ok(())
    }
}
