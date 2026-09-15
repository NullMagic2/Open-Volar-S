//! Measure this player's output session only; no loopback recording.
use std::{
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    time::{Duration, Instant},
};
use windows::{
    core::*,
    Win32::{
        Media::Audio::{Endpoints::IAudioMeterInformation, *},
        System::Com::*,
    },
};
pub struct Health {
    meter: Option<IAudioMeterInformation>,
    path: PathBuf,
    start: Instant,
    last: Instant,
    peak: f32,
    samples: u32,
}
impl Health {
    pub fn new(path: PathBuf) -> Self {
        Self {
            meter: None,
            path,
            start: Instant::now(),
            last: Instant::now() - Duration::from_secs(1),
            peak: 0.,
            samples: 0,
        }
    }
    unsafe fn meter() -> Result<IAudioMeterInformation> {
        let devices: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_INPROC_SERVER)?;
        let endpoint = devices.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
        let manager: IAudioSessionManager2 = endpoint.Activate(CLSCTX_INPROC_SERVER, None)?;
        let sessions = manager.GetSessionEnumerator()?;
        for i in 0..sessions.GetCount()? {
            let session: IAudioSessionControl2 = sessions.GetSession(i)?.cast()?;
            if session.GetProcessId()? == std::process::id() {
                return session.cast();
            }
        }
        Err(Error::from_hresult(windows::Win32::Foundation::E_FAIL))
    }
    pub fn sample(&mut self, paused: bool, video: Option<serde_json::Value>) {
        unsafe {
            if let Some(meter) = &self.meter {
                if let Ok(peak) = meter.GetPeakValue() {
                    self.peak = self.peak.max(peak);
                    self.samples += 1;
                }
            }
            if self.last.elapsed() < Duration::from_secs(1) {
                return;
            }
            if self.meter.is_none() {
                self.meter = Self::meter().ok();
            }
            let row = serde_json::json!({"elapsed_seconds":self.start.elapsed().as_secs_f64(),"paused":paused,"audio_peak":if self.samples>0{Some(self.peak)}else{None},"audio_samples":self.samples,"video":video});
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
            {
                let _ = writeln!(file, "{row}");
            }
            self.peak = 0.;
            self.samples = 0;
            self.last = Instant::now();
        }
    }
}
