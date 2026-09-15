//! Summary: Public entry point for the open-source AVerMedia A865R hardware-control library.
//!
//! The crate intentionally keeps hardware protocol knowledge in user mode. Windows builds use
//! Microsoft's inbox WinUSB driver while the separate `bda/` project is reserved for a future
//! native Broadcast Driver Architecture compatibility layer.

pub mod api;
pub mod channel_plan;
pub mod device;
pub mod eeprom;
pub mod epg;
pub mod error;
pub mod firmware;
pub mod open_firmware;
pub mod protocol;
pub mod receiver;
mod receiver_tables;
pub mod remote;
pub mod transport;
pub mod ts;

pub use device::{CaptureReport, Device, DeviceInfo, FirmwareLoadReport, OpenFirmwareProbeReport};
pub use eeprom::{EepromInfo, EepromLayout, EepromSummary};
pub use error::{Error, Result};
pub use firmware::{FirmwareCore, FirmwareImage, FirmwareRegion};
pub use open_firmware::{
    execution_probe_image, OPEN_LINK_ENTRY, OPEN_LINK_VERSION, OPEN_OFDM_VERSION, OPEN_PROBE_ENTRY,
    OPEN_PROBE_MARKER_ADDRESS, OPEN_PROBE_MARKER_VALUE,
};
pub use transport::{BulkPipe, Transport};
pub use ts::{ProgramInfo, StreamInfo, TsAnalyzer, TsStats};

/// USB vendor identifier assigned to AVerMedia.
pub const AVERMEDIA_VENDOR_ID: u16 = 0x07CA;

/// USB product identifier used by the AVerMedia A865R / AVerTV Volar S.
pub const A865R_PRODUCT_ID: u16 = 0xB865;
