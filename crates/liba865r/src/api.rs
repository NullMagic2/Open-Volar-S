//! Stable, dependency-free application API. Device limits and playback policy are separate.
use crate::{channel_plan, DeviceInfo, Error, Result};

pub const API_VERSION: u32 = 2;
pub const DRIVER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ColorProfile {
    #[default]
    Monitor,
    File(std::path::PathBuf),
    Disabled,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColorProfileState {
    Pending,
    Applied,
    Disabled,
    Unavailable,
    Failed,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColorProfileStatus {
    pub requested: ColorProfile,
    pub state: ColorProfileState,
    pub active_path: Option<std::path::PathBuf>,
    pub sha256: Option<String>,
    pub detail: Option<String>,
}
impl ColorProfileStatus {
    pub fn new(requested: ColorProfile) -> Self {
        Self {
            state: if requested == ColorProfile::Disabled {
                ColorProfileState::Disabled
            } else {
                ColorProfileState::Pending
            },
            requested,
            active_path: None,
            sha256: None,
            detail: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChipsetInfo {
    pub chip_type: u16,
    pub chip_revision: u8,
    pub prechip_revision: u8,
    pub tuner_id: Option<u8>,
    pub link_firmware: [u8; 4],
    pub firmware_running: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderBackend {
    Auto,
    Cpu,
    Gpu,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolution {
    Native,
    Hd,
    Qhd,
    Uhd,
}
impl Resolution {
    pub const ALL: [Self; 4] = [Self::Native, Self::Hd, Self::Qhd, Self::Uhd];
    pub fn dimensions(self) -> Option<(u32, u32)> {
        match self {
            Self::Native => None,
            Self::Hd => Some((1920, 1080)),
            Self::Qhd => Some((2560, 1440)),
            Self::Uhd => Some((3840, 2160)),
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Native => "Broadcast size",
            Self::Hd => "1080p",
            Self::Qhd => "1440p",
            Self::Uhd => "4K · 3840 × 2160",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeinterlaceMode {
    DoubleRate,
    SingleRate,
    Off,
}
impl DeinterlaceMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::DoubleRate => "Smooth motion · 50/60 fps",
            Self::SingleRate => "Standard · 25/30 fps",
            Self::Off => "Deinterlacing off",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaybackSettings {
    pub resolution: Resolution,
    pub upscaling_enabled: bool,
    pub backend: RenderBackend,
    pub cpu_threads: usize,
    pub deinterlacing: DeinterlaceMode,
    pub color_profile: ColorProfile,
}
impl Default for PlaybackSettings {
    fn default() -> Self {
        Self {
            resolution: Resolution::Uhd,
            upscaling_enabled: true,
            backend: RenderBackend::Auto,
            deinterlacing: DeinterlaceMode::DoubleRate,
            color_profile: ColorProfile::Monitor,
            cpu_threads: std::thread::available_parallelism()
                .map(|n| n.get().saturating_sub(1).clamp(1, 16))
                .unwrap_or(4),
        }
    }
}
impl PlaybackSettings {
    /// Disabled upscaling always preserves the decoded dimensions, regardless of the stored preset.
    pub fn effective_resolution(&self) -> Resolution {
        if self.upscaling_enabled {
            self.resolution
        } else {
            Resolution::Native
        }
    }
    pub fn validate(&self) -> Result<()> {
        if !(1..=64).contains(&self.cpu_threads) {
            return Err(Error::InvalidArgument("CPU workers must be 1..64".into()));
        }
        if matches!(&self.color_profile, ColorProfile::File(path) if path.as_os_str().is_empty()) {
            return Err(Error::InvalidArgument(
                "Choose an ICC/ICM profile file".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct DeviceCapabilities {
    pub api_version: u32,
    pub driver_version: &'static str,
    pub model: &'static str,
    pub standard: &'static str,
    pub bandwidth_khz: u32,
    pub min_frequency_khz: u32,
    pub max_frequency_khz: u32,
    pub custom_scan: bool,
    pub transport_stream: bool,
    pub native_resolution: Option<(u32, u32)>,
    pub maximum_scaled_resolution: (u32, u32),
    pub reference_firmware_reception_verified: bool,
    pub open_firmware_reception_verified: bool,
    pub detected_board_matches: Option<bool>,
    pub chipset: Option<ChipsetInfo>,
    pub color_profile_modes: &'static [&'static str],
}
pub fn capabilities(device: Option<&DeviceInfo>) -> DeviceCapabilities {
    DeviceCapabilities {
        api_version: API_VERSION,
        driver_version: DRIVER_VERSION,
        model: "AVerTV Volar S A865R / IT9175",
        standard: "ISDB-T / ISDB-Tb (6 MHz UHF profile; locally verified in Brazil)",
        bandwidth_khz: 6000,
        min_frequency_khz: channel_plan::MIN_FREQUENCY_KHZ,
        max_frequency_khz: channel_plan::MAX_FREQUENCY_KHZ,
        custom_scan: true,
        transport_stream: true,
        native_resolution: None,
        maximum_scaled_resolution: (3840, 2160),
        reference_firmware_reception_verified: true,
        open_firmware_reception_verified: true,
        color_profile_modes: &["monitor", "file", "disabled"],
        chipset: device.map(|d| ChipsetInfo {
            chip_type: d.chip_type,
            chip_revision: d.chip_version,
            prechip_revision: d.prechip_version,
            tuner_id: d.eeprom.as_ref().map(|e| e.summary.tuner_id),
            link_firmware: d.firmware_version,
            firmware_running: d.firmware_running,
        }),
        detected_board_matches: device.map(|d| {
            d.chip_type == 0x9175
                && d.chip_version == 1
                && d.prechip_version == 0x83
                && d.eeprom.as_ref().is_some_and(|e| {
                    e.summary.tuner_id == 0x70 && e.summary.ts_mode == 0 && !e.summary.dual_mode
                })
        }),
    }
}

#[derive(Clone, Debug)]
pub struct PlaybackStatus {
    pub requested: PlaybackSettings,
    /// None until the media pipeline actually starts. GPU presence alone does not establish usability.
    pub active_backend: Option<RenderBackend>,
    /// Dimensions come from the decoded stream, never inferred from the tuner model.
    pub source_resolution: Option<(u32, u32)>,
    pub output_resolution: Option<(u32, u32)>,
    pub fallback_reason: Option<String>,
    pub color_profile: ColorProfileStatus,
}
impl PlaybackStatus {
    pub fn new(requested: PlaybackSettings) -> Result<Self> {
        requested.validate()?;
        Ok(Self {
            color_profile: ColorProfileStatus::new(requested.color_profile.clone()),
            requested,
            active_backend: None,
            source_resolution: None,
            output_resolution: None,
            fallback_reason: None,
        })
    }
    pub fn set_settings(&mut self, requested: PlaybackSettings) -> Result<()> {
        requested.validate()?;
        self.color_profile = ColorProfileStatus::new(requested.color_profile.clone());
        self.requested = requested;
        self.active_backend = None;
        self.output_resolution = None;
        self.fallback_reason = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled_upscale_overrides_4k_preset() {
        let mut settings = PlaybackSettings::default();
        settings.upscaling_enabled = false;
        assert_eq!(settings.effective_resolution(), Resolution::Native);
    }
    #[test]
    fn capabilities_report_tested_reception_without_inventing_native_dimensions() {
        let c = capabilities(None);
        assert_eq!(c.native_resolution, None);
        assert_eq!(c.detected_board_matches, None);
        assert!(c.open_firmware_reception_verified);
    }
    #[test]
    fn invalid_settings_leave_previous_state_intact() {
        let mut state = PlaybackStatus::new(PlaybackSettings::default()).unwrap();
        let old = state.requested.clone();
        let mut invalid = old.clone();
        invalid.cpu_threads = 0;
        assert!(state.set_settings(invalid).is_err());
        assert_eq!(state.requested, old);
    }
}
