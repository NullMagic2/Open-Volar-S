//! USB transport for the Open Volar S Linux character driver.
//!
//! The kernel driver owns the USB interface. This layer retains the same framed
//! protocol and endpoint discovery used by the Windows WinUSB backend.

use super::{BulkPipe, Transport};
use crate::error::{Error, Result};
use std::ffi::c_void;
use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

const ABI_VERSION: u32 = 1;
const MAX_ENDPOINTS: usize = 16;
const MAX_TRANSFER: usize = 16_384;
const USB_GET_INFO: u64 = ioctl_number(2, 1, std::mem::size_of::<UsbInfo>());
const USB_BULK: u64 = ioctl_number(3, 2, std::mem::size_of::<UsbBulk>());

const fn ioctl_number(direction: u64, number: u64, size: usize) -> u64 {
    (direction << 30) | ((size as u64) << 16) | ((b'V' as u64) << 8) | number
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct UsbEndpoint {
    address: u8,
    reserved: u8,
    maximum_packet_size: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct UsbInfo {
    abi_version: u32,
    vendor_id: u16,
    product_id: u16,
    interface_number: u8,
    endpoint_count: u8,
    reserved: [u8; 2],
    endpoints: [UsbEndpoint; MAX_ENDPOINTS],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct UsbBulk {
    data: u64,
    length: u32,
    timeout_ms: u32,
    actual_length: u32,
    endpoint: u8,
    reserved: [u8; 3],
}

unsafe extern "C" {
    fn ioctl(fd: i32, request: std::os::raw::c_ulong, argument: *mut c_void) -> i32;
}

fn call_ioctl<T>(file: &File, request: u64, data: &mut T) -> io::Result<()> {
    // SAFETY: the kernel ABI accepts a writable pointer to T for the full ioctl call.
    let result = unsafe { ioctl(file.as_raw_fd(), request as _, (data as *mut T).cast()) };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub struct LinuxUsbTransport {
    file: Option<File>,
    device_index: Option<u32>,
    path: String,
    pipes: Vec<BulkPipe>,
    command_out: Option<u8>,
    command_in: Option<u8>,
}

impl LinuxUsbTransport {
    pub fn new() -> Self {
        Self {
            file: None,
            device_index: None,
            path: String::new(),
            pipes: Vec::new(),
            command_out: None,
            command_in: None,
        }
    }

    pub fn for_device(index:u32)->Self {
        let mut transport=Self::new();transport.device_index=Some(index);transport
    }

    fn file(&self) -> Result<&File> {
        self.file
            .as_ref()
            .ok_or_else(|| Error::Transport("Linux USB device is not open".into()))
    }

    fn device_node_exists() -> bool {
        (0..64).any(|index| Path::new(&format!("/dev/open-volar-s{index}")).exists())
    }

    fn attach_wsl_receiver() -> std::result::Result<(), String> {
        if !Path::new("/sys/module/open_volar_s_usb").exists() {
            let distro = std::env::var("WSL_DISTRO_NAME")
                .map_err(|_| "WSL distribution name is unavailable".to_owned())?;
            let module = Command::new("wsl.exe")
                .args(["-d", &distro, "-u", "root", "--", "modprobe", "open_volar_s_usb"])
                .output().map_err(|error| format!("Cannot load WSL USB module: {error}"))?;
            if !module.status.success() {
                return Err(format!("Cannot load open_volar_s_usb for this WSL kernel: {}. Run wsl_prepare_kernel.sh as root.",
                    String::from_utf8_lossy(&module.stderr).trim()));
            }
        }
        if Self::device_node_exists() { return Ok(()); }
        let handoff = Command::new("powershell.exe")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command",
                include_str!("wsl_attach.ps1")])
            .output().map_err(|error| format!("Cannot invoke Windows usbipd: {error}"))?;
        if !handoff.status.success() {
            let detail = String::from_utf8_lossy(&handoff.stderr);
            let detail = if detail.trim().is_empty() {
                String::from_utf8_lossy(&handoff.stdout).trim().to_owned()
            } else { detail.trim().to_owned() };
            return Err(format!("Windows USB handoff failed: {detail}"));
        }
        for _ in 0..120 {
            if Self::device_node_exists() { return Ok(()); }
            thread::sleep(Duration::from_millis(100));
        }
        Err("usbipd attached the A865R, but the WSL USB driver did not create /dev/open-volar-s*".into())
    }

    fn transfer(&self, endpoint: u8, data: &mut [u8], timeout_ms: u32) -> Result<usize> {
        let length: u32 = data.len().try_into().map_err(|_| {
            Error::InvalidArgument("USB transfer exceeds the kernel ABI length".into())
        })?;
        if data.len() > MAX_TRANSFER {
            return Err(Error::InvalidArgument(format!(
                "USB transfer exceeds the {}-byte Linux driver limit",
                MAX_TRANSFER
            )));
        }
        let mut request = UsbBulk {
            data: data.as_mut_ptr() as usize as u64,
            length,
            timeout_ms: timeout_ms.max(1),
            actual_length: 0,
            endpoint,
            reserved: [0; 3],
        };
        call_ioctl(self.file()?, USB_BULK, &mut request).map_err(Error::Io)?;
        if request.actual_length > length {
            return Err(Error::Transport("Linux driver returned an invalid transfer length".into()));
        }
        Ok(request.actual_length as usize)
    }
}

impl Transport for LinuxUsbTransport {
    fn open(&mut self, vendor_id: u16, product_id: u16) -> Result<()> {
        self.file = None;
        self.path.clear();
        self.pipes.clear();
        self.command_out = None;
        self.command_in = None;
        let mut attempted_wsl_handoff = false;
        loop {
        let mut first_error = None;
        let indices=match self.device_index {Some(index)=>index..index.saturating_add(1),None=>0..64};
        for index in indices {
            let path = format!("/dev/open-volar-s{index}");
            match OpenOptions::new().read(true).write(true).open(&path) {
                Ok(file) => {
                    let mut info = UsbInfo::default();
                    call_ioctl(&file, USB_GET_INFO, &mut info).map_err(|error| {
                        Error::Transport(format!("{path}: cannot query driver: {error}"))
                    })?;
                    if info.abi_version != ABI_VERSION {
                        return Err(Error::Transport(format!(
                            "{path}: driver ABI {} does not match userspace ABI {ABI_VERSION}",
                            info.abi_version
                        )));
                    }
                    if info.vendor_id != vendor_id || info.product_id != product_id {
                        continue;
                    }
                    if info.endpoint_count as usize > MAX_ENDPOINTS {
                        return Err(Error::Transport(format!("{path}: invalid endpoint count")));
                    }
                    self.pipes = info.endpoints[..info.endpoint_count as usize]
                        .iter()
                        .map(|endpoint| BulkPipe {
                            id: endpoint.address,
                            input: endpoint.address & 0x80 != 0,
                            maximum_packet_size: endpoint.maximum_packet_size,
                        })
                        .collect();
                    self.path = path;
                    self.file = Some(file);
                    return Ok(());
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => {
                    if first_error.is_none() {
                        first_error = Some(format!("{path}: {error}"));
                    }
                }
            }
        }
        if first_error.is_none() && !attempted_wsl_handoff && std::env::var_os("WSL_DISTRO_NAME").is_some() {
            attempted_wsl_handoff = true;
            Self::attach_wsl_receiver().map_err(|error| Error::Transport(error))?;
            continue;
        }
        return Err(Error::Transport(first_error.unwrap_or_else(|| {
            "no /dev/open-volar-s* device found; attach the A865R and load open_volar_s_usb".into()
        })));
        }
    }

    fn cycle_port(&mut self) -> Result<()> {
        Err(Error::Unsupported(
            "physical USB port power cycling is unavailable through the Linux driver; unplug/replug the receiver (or detach/attach it with usbipd on WSL)".into(),
        ))
    }

    fn bulk_pipes(&self) -> &[BulkPipe] {
        &self.pipes
    }

    fn set_command_pipes(&mut self, output_pipe: u8, input_pipe: u8) -> Result<()> {
        self.file()?;
        if !self.pipes.iter().any(|pipe| pipe.id == output_pipe && !pipe.input)
            || !self.pipes.iter().any(|pipe| pipe.id == input_pipe && pipe.input)
        {
            return Err(Error::InvalidArgument("command endpoints are not active bulk pipes".into()));
        }
        self.command_out = Some(output_pipe);
        self.command_in = Some(input_pipe);
        Ok(())
    }

    fn exchange(&mut self, request: &[u8], expected_response_size: usize, timeout_ms: u32) -> Result<Vec<u8>> {
        let output = self.command_out.ok_or_else(|| Error::Transport("command endpoints have not been selected".into()))?;
        let input = self.command_in.ok_or_else(|| Error::Transport("command endpoints have not been selected".into()))?;
        if request.len() > MAX_TRANSFER || expected_response_size > MAX_TRANSFER {
            return Err(Error::InvalidArgument("command frame exceeds Linux driver transfer limit".into()));
        }
        let mut outgoing = request.to_vec();
        let written = self.transfer(output, &mut outgoing, timeout_ms)?;
        if written != request.len() {
            return Err(Error::Transport(format!("short command write: expected {} bytes, wrote {written}", request.len())));
        }
        let mut response = vec![0; expected_response_size];
        let received = self.transfer(input, &mut response, timeout_ms)?;
        response.truncate(received);
        Ok(response)
    }

    fn read_bulk(&mut self, input_pipe: u8, requested_size: usize, timeout_ms: u32) -> Result<Vec<u8>> {
        if !self.pipes.iter().any(|pipe| pipe.id == input_pipe && pipe.input) {
            return Err(Error::InvalidArgument("stream endpoint is not an active bulk IN pipe".into()));
        }
        let mut data = vec![0; requested_size.min(MAX_TRANSFER)];
        match self.transfer(input_pipe, &mut data, timeout_ms) {
            Ok(received) => {
                data.truncate(received);
                Ok(data)
            }
            Err(Error::Io(error)) if error.raw_os_error() == Some(110) => Ok(Vec::new()),
            Err(error) => Err(error),
        }
    }

    fn description(&self) -> String {
        if self.path.is_empty() {
            "Linux USB driver (not open)".into()
        } else {
            format!("Linux USB driver: {}", self.path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ioctl_abi_layout_matches_kernel_header() {
        assert_eq!(std::mem::size_of::<UsbEndpoint>(), 4);
        assert_eq!(std::mem::size_of::<UsbInfo>(), 76);
        assert_eq!(std::mem::size_of::<UsbBulk>(), 24);
        assert_eq!(USB_GET_INFO, 0x804c5601);
        assert_eq!(USB_BULK, 0xc0185602);
    }
}
