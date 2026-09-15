//! Use the window's display vblank, rather than a polling sleep, to pace Vulkan.
use std::time::{Duration, Instant};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::{Dxgi::*, Gdi::*},
        System::Threading::*,
    },
};

/// Pick the closest source sample to the upcoming display refresh. A half-field
/// margin prevents small clock/wakeup variations from repeating then skipping it.
pub fn sample_margin(start: i64, end: i64, interlaced: bool) -> i64 {
    end.saturating_sub(start).max(0) / if interlaced { 4 } else { 2 }
}

#[cfg(test)]
mod tests {
    #[test]
    fn nearest_fields_remain_sequential_with_submillisecond_jitter() {
        let frame = 333_667i64;
        let margin = super::sample_margin(0, frame, true);
        for field in 2..1200i64 {
            let jitter = if field % 2 == 0 { -8_000 } else { 8_000 };
            let target = field * frame / 2 + jitter;
            let selected_frame = (target + margin) / frame;
            let selected_field = i64::from(target >= selected_frame * frame + margin);
            assert_eq!(selected_frame * 2 + selected_field, field);
        }
    }
    #[test]
    fn progressive_frames_have_one_temporal_sample() {
        assert_eq!(super::sample_margin(0, 400_000, false), 200_000);
        assert_eq!(super::sample_margin(20, 10, true), 0);
    }
}

pub struct DisplayPacer {
    hwnd: HWND,
    monitor: HMONITOR,
    output: Option<IDXGIOutput>,
    timer: Option<HANDLE>,
    mmcss: Option<HANDLE>,
    last: Option<Instant>,
    refreshed: Instant,
    pub period: Duration,
    pub hardware_wait: bool,
    pub missed_refreshes: u64,
}
impl DisplayPacer {
    pub fn new(hwnd: usize) -> Self {
        unsafe {
            let mut index = 0;
            let mmcss = AvSetMmThreadCharacteristicsW(w!("Playback"), &mut index).ok();
            if let Some(handle) = mmcss {
                let _ = AvSetMmThreadPriority(handle, AVRT_PRIORITY_HIGH);
            }
            let timer = CreateWaitableTimerExW(
                None,
                PCWSTR::null(),
                CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
                TIMER_ALL_ACCESS.0,
            )
            .ok();
            let mut this = Self {
                hwnd: HWND(hwnd as _),
                monitor: HMONITOR::default(),
                output: None,
                timer,
                mmcss,
                last: None,
                refreshed: Instant::now() - Duration::from_secs(2),
                period: Duration::from_secs_f64(1. / 60.),
                hardware_wait: false,
                missed_refreshes: 0,
            };
            this.refresh();
            this
        }
    }
    fn refresh(&mut self) {
        unsafe {
            if self.refreshed.elapsed() < Duration::from_secs(1) {
                return;
            }
            self.refreshed = Instant::now();
            let monitor = MonitorFromWindow(self.hwnd, MONITOR_DEFAULTTONEAREST);
            if monitor == self.monitor && self.output.is_some() {
                return;
            }
            self.monitor = monitor;
            self.output = None;
            self.last = None;
            if let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory1>() {
                let mut ai = 0;
                while let Ok(adapter) = factory.EnumAdapters1(ai) {
                    ai += 1;
                    let mut oi = 0;
                    while let Ok(output) = adapter.EnumOutputs(oi) {
                        oi += 1;
                        if let Ok(desc) = output.GetDesc() {
                            if desc.Monitor == monitor {
                                let mut mode = DEVMODEW::default();
                                mode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
                                if EnumDisplaySettingsW(
                                    PCWSTR(desc.DeviceName.as_ptr()),
                                    ENUM_CURRENT_SETTINGS,
                                    &mut mode,
                                )
                                .as_bool()
                                    && mode.dmDisplayFrequency > 1
                                {
                                    self.period = Duration::from_secs_f64(
                                        1. / mode.dmDisplayFrequency as f64,
                                    );
                                }
                                self.output = Some(output);
                                return;
                            }
                        }
                    }
                }
            }
        }
    }
    pub fn fullscreen_surface(&mut self)->bool {
        self.refresh();
        unsafe {
            let mut monitor=MONITORINFO{cbSize:std::mem::size_of::<MONITORINFO>() as u32,..Default::default()};
            let mut client=RECT::default();
            if !GetMonitorInfoW(self.monitor,&mut monitor).as_bool() || windows::Win32::UI::WindowsAndMessaging::GetClientRect(self.hwnd,&mut client).is_err(){return false;}
            client.right>=monitor.rcMonitor.right-monitor.rcMonitor.left && client.bottom>=monitor.rcMonitor.bottom-monitor.rcMonitor.top
        }
    }
    pub fn wait(&mut self) {
        self.refresh();
        self.hardware_wait = self
            .output
            .as_ref()
            .is_some_and(|o| unsafe { o.WaitForVBlank().is_ok() });
        if !self.hardware_wait {
            let remaining = self
                .last
                .map(|last| self.period.saturating_sub(last.elapsed()))
                .unwrap_or(self.period);
            if let Some(timer) = self.timer {
                let due = -((remaining.as_nanos() / 100).max(1) as i64);
                unsafe {
                    if SetWaitableTimerEx(timer, &due, 0, None, None, None, 0).is_ok() {
                        WaitForSingleObject(timer, 100);
                    }
                }
            } else {
                std::thread::sleep(remaining);
            }
        }
        let now = Instant::now();
        if let Some(last) = self.last {
            let dt = now.duration_since(last).as_secs_f64();
            let period = self.period.as_secs_f64();
            if self.hardware_wait && dt > period * 0.75 && dt < period * 1.25 {
                self.period = Duration::from_secs_f64(period * 0.95 + dt * 0.05);
            } else if dt > period * 1.5 {
                self.missed_refreshes += (dt / period).round() as u64 - 1;
            }
        }
        self.last = Some(now);
    }
    pub fn lead_100ns(&self) -> i64 {
        self.period.as_nanos() as i64 / 100
    }
}
impl Drop for DisplayPacer {
    fn drop(&mut self) {
        unsafe {
            if let Some(timer) = self.timer {
                let _ = CloseHandle(timer);
            }
            if let Some(mmcss) = self.mmcss {
                let _ = AvRevertMmThreadCharacteristics(mmcss);
            }
        }
    }
}

#[derive(Default)]
pub struct Intervals {
    last: Option<Instant>,
    samples: Vec<f64>,
}
impl Intervals {
    pub fn tick(&mut self) {
        let now = Instant::now();
        if let Some(last) = self.last {
            self.samples
                .push(now.duration_since(last).as_secs_f64() * 1000.);
        }
        self.last = Some(now);
    }
    pub fn report(&mut self) -> serde_json::Value {
        let mut samples = std::mem::take(&mut self.samples);
        if samples.is_empty() {
            return serde_json::Value::Null;
        }
        samples.sort_by(f64::total_cmp);
        let n = samples.len();
        let avg = samples.iter().sum::<f64>() / n as f64;
        serde_json::json!({"count":n,"mean_ms":avg,"p95_ms":samples[((n-1)as f64*0.95).round()as usize],"max_ms":samples[n-1],"stddev_ms":(samples.iter().map(|x|(x-avg).powi(2)).sum::<f64>()/n as f64).sqrt()})
    }
}
