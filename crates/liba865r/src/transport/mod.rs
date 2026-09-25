//! Summary: Declares the platform-neutral USB transport and selects WinUSB on Windows.

use crate::error::Result;

#[cfg(any(windows, target_os="linux"))]
pub mod wine_bridge;

#[cfg(windows)]
#[path = "../../../../windows/transport/winusb.rs"]
mod winusb;
#[cfg(target_os = "linux")]
#[path = "../../../../linux/transport/linux.rs"]
mod linux;

/// Describes one USB bulk endpoint discovered on the active interface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BulkPipe {
    /// USB endpoint address, including the direction bit.
    pub id: u8,
    /// True for device-to-host endpoints.
    pub input: bool,
    /// Maximum USB packet size reported by the endpoint descriptor.
    pub maximum_packet_size: u16,
}

/// Abstracts USB I/O so the ITE-family command protocol remains independent of WinUSB.
pub trait Transport {
    /// Opens the first device matching the requested USB vendor and product identifiers.
    fn open(&mut self, vendor_id: u16, product_id: u16) -> Result<()>;

    /// Cycles the physical USB port through its parent hub without using device firmware.
    fn cycle_port(&mut self) -> Result<()>;

    /// Returns the bulk endpoints available on the active interface.
    fn bulk_pipes(&self) -> &[BulkPipe];

    /// Selects the bulk OUT and IN endpoints used for framed command transactions.
    fn set_command_pipes(&mut self, output_pipe: u8, input_pipe: u8) -> Result<()>;

    /// Sends one framed command and reads its complete expected response.
    fn exchange(
        &mut self,
        request: &[u8],
        expected_response_size: usize,
        timeout_ms: u32,
    ) -> Result<Vec<u8>>;

    /// Reads one chunk from a bulk IN endpoint, returning an empty vector on an ordinary timeout.
    fn read_bulk(
        &mut self,
        input_pipe: u8,
        requested_size: usize,
        timeout_ms: u32,
    ) -> Result<Vec<u8>>;

    /// Returns a human-readable description of the selected backend and device path.
    fn description(&self) -> String;
}

/// Creates the default host transport.
pub fn default_transport() -> Box<dyn Transport> {
    #[cfg(windows)]
    {
        if let Some(path) = std::env::var_os("OPEN_VOLAR_S_WINE_BRIDGE") {
            return Box::new(wine_bridge::WineTransport::new(path.into()));
        }
        if wine_bridge::is_wine() {
            return Box::new(wine_bridge::WineTransport::new(r"C:\open-volar-s-bridge.txt".into()));
        }
        Box::new(winusb::WinUsbTransport::new())
    }

    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxUsbTransport::new())
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    {
        Box::new(UnsupportedTransport::new())
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
struct UnsupportedTransport {
    pipes: Vec<BulkPipe>,
}

#[cfg(not(any(windows, target_os = "linux")))]
impl UnsupportedTransport {
    /// Creates a backend that explains that hardware access is currently Windows-only.
    fn new() -> Self {
        Self { pipes: Vec::new() }
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
impl Transport for UnsupportedTransport {
    /// Rejects device opening on hosts for which no USB backend has been implemented yet.
    fn open(&mut self, _vendor_id: u16, _product_id: u16) -> Result<()> {
        Err(crate::Error::Unsupported(
            "the current build has no USB backend; build on Windows to use WinUSB".to_string(),
        ))
    }

    /// Rejects hub-port cycling on hosts for which no USB backend is active.
    fn cycle_port(&mut self) -> Result<()> {
        Err(crate::Error::Unsupported(
            "USB hub port cycling is currently implemented only on Windows".to_string(),
        ))
    }

    /// Returns an empty endpoint list because the unsupported backend never opens hardware.
    fn bulk_pipes(&self) -> &[BulkPipe] {
        &self.pipes
    }

    /// Rejects command-pipe selection because no hardware is open.
    fn set_command_pipes(&mut self, _output_pipe: u8, _input_pipe: u8) -> Result<()> {
        Err(crate::Error::Unsupported(
            "no USB backend is active".to_string(),
        ))
    }

    /// Rejects command exchange because no hardware is open.
    fn exchange(
        &mut self,
        _request: &[u8],
        _expected_response_size: usize,
        _timeout_ms: u32,
    ) -> Result<Vec<u8>> {
        Err(crate::Error::Unsupported(
            "no USB backend is active".to_string(),
        ))
    }

    /// Rejects streaming because no hardware is open.
    fn read_bulk(
        &mut self,
        _input_pipe: u8,
        _requested_size: usize,
        _timeout_ms: u32,
    ) -> Result<Vec<u8>> {
        Err(crate::Error::Unsupported(
            "no USB backend is active".to_string(),
        ))
    }

    /// Describes the intentionally unavailable backend.
    fn description(&self) -> String {
        "unsupported host transport".to_string()
    }
}

/// Open a specific Linux character device, keeping each DVB adapter tied to its USB receiver.
#[cfg(target_os = "linux")]
pub fn linux_device_transport(index:u32)->Box<dyn Transport> {
    Box::new(linux::LinuxUsbTransport::for_device(index))
}
