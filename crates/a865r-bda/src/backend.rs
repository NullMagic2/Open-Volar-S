use a865r::{Device, FirmwareImage};
use sha2::{Digest, Sha256};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tune {
    pub frequency: u32,
    pub multiplier: u32,
    pub bandwidth: u32,
}
impl Default for Tune {
    fn default() -> Self {
        Self {
            frequency: 473143,
            multiplier: 1000,
            bandwidth: 6,
        }
    }
}
impl Tune {
    pub fn khz(self) -> std::result::Result<u32, String> {
        let hz = u64::from(self.frequency) * u64::from(self.multiplier);
        if hz % 1000 != 0 {
            return Err("Frequency must resolve to a whole kHz".into());
        }
        let khz = u32::try_from(hz / 1000).map_err(|_| "Frequency overflow")?;
        if self.bandwidth != 6 {
            return Err("This board supports 6 MHz ISDB-T".into());
        }
        a865r::channel_plan::validate_frequency(khz).map_err(|e| e.to_string())?;
        Ok(khz)
    }
}
#[derive(Default)]
pub struct Status {
    pub quality: Mutex<Option<u8>>,
    pub locked: AtomicBool,
    pub present: AtomicBool,
    pub dropped: AtomicU64,
    pub bytes: AtomicU64,
    pub error: Mutex<Option<String>>,
}
pub struct Session {
    pub cancel: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}
#[derive(Clone)]
pub struct Replay {
    pub path: std::path::PathBuf,
    pub offset: u64,
    /// True only while a writer is appending. EOF then waits, without losing data.
    pub growing: Arc<AtomicBool>,
}
impl Session {
    pub fn start_external(tune:Tune,status:Arc<Status>)->std::result::Result<(Self,Receiver<Vec<u8>>),String>{
        let config=crate::signal::load_at(&crate::signal::config_path())?;
        if config.mode==0 {return Self::start(tune,status);}
        let (mut raw,input)=Self::start(tune,status.clone())?;
        let cancel=raw.cancel.clone();let c=cancel.clone();let (tx,rx)=mpsc::sync_channel(32);
        let thread=thread::spawn(move||{
            if let Err(e)=crate::signal::process(config,input,tx,c.clone(),status.clone()) {
                if !c.load(Ordering::Relaxed) || e!="Cancelled" {crate::trace(&e);*status.error.lock().unwrap()=Some(e);}
            }
            raw.stop();
        });
        Ok((Self{cancel,thread:Some(thread)},rx))
    }

    pub fn replay(replay: Replay) -> std::result::Result<(Self, Receiver<Vec<u8>>), String> {
        use std::io::{Read, Seek, SeekFrom};
        let mut file = std::fs::File::open(&replay.path).map_err(|e| e.to_string())?;
        file.seek(SeekFrom::Start(replay.offset))
            .map_err(|e| e.to_string())?;
        let cancel = Arc::new(AtomicBool::new(false));
        let c = cancel.clone();
        let (tx, rx) = mpsc::sync_channel(4);
        let thread = thread::spawn(move || {
            let mut buf = vec![0; 188 * 256];
            while !c.load(Ordering::Relaxed) {
                let n = match file.read(&mut buf) {
                    Ok(0) if replay.growing.load(Ordering::Acquire) => {
                        thread::sleep(Duration::from_millis(20));
                        continue;
                    }
                    Ok(0) | Err(_) => break,
                    Ok(n) => n,
                };
                let mut data = buf[..n].to_vec();
                loop {
                    if c.load(Ordering::Relaxed) {
                        return;
                    }
                    match tx.try_send(data) {
                        Ok(()) => break,
                        Err(mpsc::TrySendError::Full(d)) => {
                            data = d;
                            thread::sleep(Duration::from_millis(5));
                        }
                        Err(_) => return,
                    }
                }
            }
        });
        Ok((
            Self {
                cancel,
                thread: Some(thread),
            },
            rx,
        ))
    }
    pub fn start(
        tune: Tune,
        status: Arc<Status>,
    ) -> std::result::Result<(Self, Receiver<Vec<u8>>), String> {
        let frequency = tune.khz()?;
        let cancel = Arc::new(AtomicBool::new(false));
        let c = cancel.clone();
        let (tx, rx) = mpsc::sync_channel(256);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        status.dropped.store(0, Ordering::Relaxed);
        status.bytes.store(0, Ordering::Relaxed);
        status.locked.store(false, Ordering::Relaxed);
        *status.quality.lock().unwrap() = None;
        status.present.store(false, Ordering::Relaxed);
        *status.error.lock().unwrap() = None;
        let handle = thread::spawn(move || {
            let result = receive(frequency, &c, &status, tx, ready_tx.clone());
            if let Err(e) = result {
                let _ = ready_tx.try_send(Err(e.clone()));
                crate::trace(format!("receiver error: {e}"));
                *status.error.lock().unwrap() = Some(e);
            }
            status.locked.store(false, Ordering::Relaxed);
            *status.quality.lock().unwrap() = None;
            status.present.store(false, Ordering::Relaxed);
        });
        let mut session = Self {
            cancel,
            thread: Some(handle),
        };
        match ready_rx.recv_timeout(Duration::from_secs(20)) {
            Ok(Ok(())) => Ok((session, rx)),
            other => {
                session.stop();
                Err(match other {
                    Ok(Err(e)) => e,
                    _ => "Tuner initialization timed out".into(),
                })
            }
        }
    }
    pub fn stop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
    }
}

fn receive(
    frequency: u32,
    cancel: &AtomicBool,
    status: &Status,
    tx: SyncSender<Vec<u8>>,
    ready: SyncSender<std::result::Result<(), String>>,
) -> std::result::Result<(), String> {
    let mut device = Device::new();
    let info = device
        .connect_and_probe()
        .map_err(|e| e.to_string())?
        .clone();
    if !info.firmware_running {
        let open = a865r::execution_probe_image().map_err(|e| e.to_string())?;
        let bytes = match std::env::var_os("A865R_FIRMWARE") {
            Some(path) => std::fs::read(path).map_err(|e| e.to_string())?,
            None => open.bytes().to_vec(),
        };
        if bytes != open.bytes()
            && format!("{:x}", Sha256::digest(&bytes))
                != "4b066157d0eb1a088e55daaf339418fc6596b5f76c4583d06d5eddb3a272e921"
        {
            return Err("The selected firmware is neither the tested open image nor the matching reference image".into());
        }
        device
            .load_firmware(&FirmwareImage::from_scatter_bytes(bytes).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    }
    let mut receiver = device.receiver().map_err(|e| e.to_string())?;
    let result = (|| {
        receiver.initialize().map_err(|e| e.to_string())?;
        if cancel.load(Ordering::Relaxed) {
            return Err("Cancelled".into());
        }
        let tune = receiver.tune(frequency, 6000).map_err(|e| e.to_string())?;
        status.present.store(tune.channel_found, Ordering::Relaxed);
        status.locked.store(tune.mpeg_locked, Ordering::Relaxed);
        crate::trace(format!("tune {frequency} kHz, lock={}", tune.mpeg_locked));
        // A successfully executed tune without RF lock is not a driver failure.
        let _ = ready.send(Ok(()));
        if !tune.mpeg_locked {
            while !cancel.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(50));
            }
            return Ok(());
        }
        let report = receiver
            .stream_chunks_monitored(
                86400,
                cancel,
                |data| {
                    match tx.try_send(data.to_vec()) {
                        Ok(()) => {
                            status.bytes.fetch_add(data.len() as u64, Ordering::Relaxed);
                        }
                        Err(mpsc::TrySendError::Full(data)) => {
                            status
                                .dropped
                                .fetch_add(data.len() as u64, Ordering::Relaxed);
                        }
                        Err(mpsc::TrySendError::Disconnected(_)) => {
                            return Err(a865r::Error::Protocol(
                                "DirectShow consumer disconnected".into(),
                            ))
                        }
                    }
                    Ok(())
                },
                |measurement| {
                    let (locked, quality) = match measurement {
                        Ok(signal) => (signal.mpeg_locked, signal.quality_percent),
                        Err(_) => (false, None),
                    };
                    let mut previous = status.quality.lock().unwrap();
                    if *previous != quality || status.locked.load(Ordering::Relaxed) != locked {
                        crate::trace(format!(
                            "signal locked={locked} quality_percent={quality:?}"
                        ));
                    }
                    *previous = quality;
                    status.locked.store(locked, Ordering::Relaxed);
                },
            )
            .map_err(|e| e.to_string())?;
        crate::trace(format!(
            "stream bytes={} queue_dropped={}",
            report.bytes_written,
            status.dropped.load(Ordering::Relaxed)
        ));
        Ok(())
    })();
    let stop = receiver.stop().map_err(|e| e.to_string());
    result.and(stop)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bda_frequency_units() {
        assert_eq!(
            Tune {
                frequency: 521143,
                ..Tune::default()
            }
            .khz()
            .unwrap(),
            521143
        );
        assert_eq!(
            Tune {
                frequency: 521143000,
                multiplier: 1,
                ..Tune::default()
            }
            .khz()
            .unwrap(),
            521143
        );
        assert!(Tune {
            frequency: 521143001,
            multiplier: 1,
            ..Tune::default()
        }
        .khz()
        .is_err());
        assert!(Tune {
            multiplier: u32::MAX,
            ..Tune::default()
        }
        .khz()
        .is_err());
        assert!(Tune {
            bandwidth: 8,
            ..Tune::default()
        }
        .khz()
        .is_err());
    }
}
