//! CPU-only AAC surround negotiation probe. No GPU or tuner is opened.
#[path = "../src/audio.rs"]
#[allow(dead_code)]
mod audio;
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
        Media::Audio::WAVEFORMATEX,
        Media::{DirectShow::*, MediaFoundation::*},
        System::Com::*,
    },
};
#[derive(Default)]
struct Sink {
    samples: AtomicU64,
    bytes: AtomicU64,
    times: Mutex<Vec<i64>>,
    channels: AtomicU64,
    pcm: Mutex<Vec<u8>>,
    done: AtomicBool,
    switch: Mutex<Option<(Arc<audio::Mixer>,audio::Mode)>>,
}
impl VideoTransform for Sink {
    fn accepts(&self, m: &AM_MEDIA_TYPE) -> bool {
        m.majortype == MEDIATYPE_Audio
    }
    fn configure(&self, m: &AM_MEDIA_TYPE) -> Result<()> {
        if !m.pbFormat.is_null() && m.cbFormat >= 18 {
            self.channels.store(
                unsafe { u16::from_le_bytes([*m.pbFormat.add(2), *m.pbFormat.add(3)]) } as u64,
                Ordering::Relaxed,
            );
            println!("PCM format {:?}", unsafe {
                std::slice::from_raw_parts(m.pbFormat, m.cbFormat.min(40) as usize)
            });
        }
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
            if s.GetActualDataLength() > 0 && s.GetActualDataLength() <= s.GetSize() {
                let p = s.GetPointer()?;
                if !p.is_null() {
                    self.pcm
                        .lock()
                        .unwrap()
                        .extend_from_slice(std::slice::from_raw_parts(
                            p,
                            s.GetActualDataLength() as usize,
                        ));
                }
            }
            self.bytes
                .fetch_add(s.GetActualDataLength().max(0) as u64, Ordering::Relaxed);
            if count==20 {if let Some((mixer,mode))=self.switch.lock().unwrap().take(){
                println!("switch_after_bytes={}",self.pcm.lock().unwrap().len());mixer.set_mode(mode);
            }}
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
        let pid: u32 = args.get(2).expect("audio PID").parse().unwrap();
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
        let mut wave = WAVEFORMATEX {
            wFormatTag: if args.iter().any(|s|s=="--latm"){0x1602}else{0x1600},
            nChannels: 2,
            nSamplesPerSec: 48000,
            nAvgBytesPerSec: 16000,
            nBlockAlign: 1,
            ..Default::default()
        };
        let mt = AM_MEDIA_TYPE {
            majortype: MEDIATYPE_Audio,
            subtype: if args.iter().any(|s|s=="--latm"){MEDIASUBTYPE_MPEG_LOAS}else{MEDIASUBTYPE_MPEG_ADTS_AAC},
            bTemporalCompression: true.into(),
            formattype: FORMAT_WaveFormatEx,
            cbFormat: std::mem::size_of_val(&wave) as u32,
            pbFormat: (&mut wave as *mut WAVEFORMATEX).cast(),
            ..Default::default()
        };
        let out = demux
            .cast::<IMpeg2Demultiplexer>()?
            .CreateOutputPin(&mt, w!("AAC"))?;
        out.cast::<IMPEG2PIDMap>()?
            .MapPID(1, &pid, MEDIA_ELEMENTARY_STREAM)?;
        let sink = Arc::new(Sink::default());
        let speakers=args.iter().any(|s|s=="--speakers");
        let filter = if speakers {CoCreateInstance(&GUID::from_u128(0x79376820_07d0_11cf_a24d_0020afd79767),None,CLSCTX_INPROC_SERVER)?} else {a865r_bda::create_video_filter(sink.clone())};
        graph.AddFilter(&filter, w!("Compressed counter - no GPU"))?;
        let decoder: IBaseFilter = CoCreateInstance(
            &GUID::from_u128(0xe1f1a0b8_beee_490d_ba7c_066c40b5e2b9),
            None,
            CLSCTX_INPROC_SERVER,
        )?;
        graph.AddFilter(&decoder, w!("Windows AAC decoder"))?;
        println!("multichannel {:?}", audio::configure_decoder(&decoder));
        graph.ConnectDirect(&out, &pin(&decoder, PINDIR_INPUT)?, None)?;
        let mode=args.iter().position(|s|s=="--mode").and_then(|i|args.get(i+1)).and_then(|s|s.parse::<usize>().ok()).map(audio::Mode::from_index).unwrap_or(audio::Mode::Surround);
        let mixer = Arc::new(audio::Mixer::new(mode));
        if let Some(mode)=args.iter().position(|s|s=="--switch-mode").and_then(|i|args.get(i+1)).and_then(|s|s.parse::<usize>().ok()){
            *sink.switch.lock().unwrap()=Some((mixer.clone(),audio::Mode::from_index(mode)));
        }
        let transform = a865r_bda::create_video_filter(mixer.clone());
        graph.AddFilter(&transform, w!("Surround PCM transform"))?;
        graph.ConnectDirect(
            &pin(&decoder, PINDIR_OUTPUT)?,
            &pin(&transform, PINDIR_INPUT)?,
            None,
        )?;
        graph.ConnectDirect(
            &pin(&transform, PINDIR_OUTPUT)?,
            &pin(&filter, PINDIR_INPUT)?,
            None,
        )?;
        if speakers{graph.cast::<IMediaFilter>()?.SetSyncSource(&filter.cast::<windows::Win32::Media::IReferenceClock>()?)?;}else{graph.cast::<IMediaFilter>()?.SetSyncSource(None)?;}
        let media: IMediaControl = graph.cast()?;
        media.Run()?;
        let start = Instant::now();
        let events:IMediaEvent=graph.cast()?;
        while start.elapsed() < Duration::from_secs(if speakers{12}else{3}) && !sink.done.load(Ordering::Relaxed) {
            let (mut code,mut p1,mut p2)=(0,0,0);
            if events.GetEvent(&mut code,&mut p1,&mut p2,0).is_ok(){println!("event {code} {p1:x} {p2:x}");events.FreeEventParams(code,p1,p2)?;}
            std::thread::sleep(Duration::from_millis(20));
        }
        if let Ok(api) = pin(&decoder, PINDIR_INPUT)?.cast::<ICodecAPI>() {
            println!(
                "broadcast channels {:?}",
                api.GetValue(&CODECAPI_AVAudioChannelCount)
                    .and_then(|v| u32::try_from(&v))
            );
        }
        println!("mixer {}", mixer.report());
        if !speakers {assert_eq!(
            Some(sink.channels.load(Ordering::Relaxed)),
            mixer.report()["channels"].as_u64()
        );}
        if let Some(path) = args.get(3).filter(|s|!s.starts_with("--")) {
            std::fs::write(path, &*sink.pcm.lock().unwrap()).unwrap();
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
