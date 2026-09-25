//! Summary: Implements A865R endpoint discovery, safe identification, EEPROM probing, and raw TS capture.

use crate::eeprom::{
    it9135_eeprom_present, layout_for_chip, read_eeprom, EepromInfo, EepromLayout,
};
use crate::error::{Error, Result};
use crate::firmware::FirmwareImage;
use crate::open_firmware::{
    execution_probe_image, OPEN_LINK_VERSION, OPEN_OFDM_VERSION, OPEN_PROBE_MARKER_ADDRESS,
};
use crate::protocol::Protocol;
use crate::transport::{default_transport, BulkPipe, Transport};
use crate::ts::{TsAnalyzer, TsStats};
use crate::{A865R_PRODUCT_ID, AVERMEDIA_VENDOR_ID};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

/// Information obtained without intentionally changing tuner or demodulator state.
#[derive(Clone, Debug)]
pub struct DeviceInfo {
    pub transport: String,
    pub chip_type: u16,
    pub chip_version: u8,
    pub prechip_version: u8,
    pub firmware_version: [u8; 4],
    pub firmware_running: bool,
    pub bulk_pipes: Vec<BulkPipe>,
    pub command_out_pipe: u8,
    pub command_in_pipe: u8,
    pub transport_stream_pipe: Option<u8>,
    pub eeprom: Option<EepromInfo>,
    pub eeprom_probe_note: Option<String>,
}

/// Results returned after copying USB stream bytes into a `.ts` file.
#[derive(Clone, Debug)]
pub struct CaptureReport {
    pub bytes_written: u64,
    pub elapsed: Duration,
    pub stats: TsStats,
}

/// Results returned after uploading and booting the embedded IT9175 firmware.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirmwareLoadReport {
    pub bytes_uploaded: usize,
    pub segments_uploaded: usize,
    pub firmware_version: [u8; 4],
}

/// Results from the reversible, independently authored MCS-51 execution probe.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenFirmwareProbeReport {
    pub bytes_uploaded: usize,
    pub records_uploaded: usize,
    pub boot_acknowledged: bool,
    pub marker_address: u32,
    pub marker_value: u8,
    pub firmware_query: [u8; 4],
    pub ofdm_firmware_query: [u8; 4],
    pub scheduler_ticks: Option<(u16, u16)>,
}

/// Owns one host transport and all state discovered for one connected A865R.
pub struct Device {
    transport: Box<dyn Transport>,
    info: Option<DeviceInfo>,
}

impl Device {
    /// Borrows the connection exclusively for one serialized reception session.
    pub fn capabilities(&self) -> crate::api::DeviceCapabilities {
        crate::api::capabilities(self.info().ok())
    }

    pub fn receiver(&mut self) -> Result<crate::receiver::Receiver<'_>> {
        let info = self.info()?.clone();
        crate::receiver::Receiver::new(self.transport.as_mut(), &info)
    }

    /// Exclusively borrows the command transport for infrared diagnostics.
    pub fn infrared(&mut self) -> Result<crate::remote::Infrared<'_>> {
        let info = self.info()?;
        if !info.firmware_running {
            return Err(Error::Unsupported(
                "infrared polling requires running firmware".into(),
            ));
        }
        Ok(crate::remote::Infrared::new(self.transport.as_mut()))
    }
    /// Creates a device using the default host backend.
    pub fn new() -> Self {
        Self::with_transport(default_transport())
    }

    /// Creates a device around a caller-supplied transport, primarily for tests and future backends.
    pub fn with_transport(transport: Box<dyn Transport>) -> Self {
        Self {
            transport,
            info: None,
        }
    }

    /// Opens only the USB transport and asks the parent hub to cycle the A865R's physical port.
    ///
    /// This deliberately does not send an A865R command, so it remains usable when an experimental
    /// firmware image has stopped both command endpoints. The caller must create a fresh `Device`
    /// after re-enumeration before attempting another probe.
    pub fn cycle_usb_port(&mut self) -> Result<()> {
        self.transport.open(AVERMEDIA_VENDOR_ID, A865R_PRODUCT_ID)?;
        self.transport.cycle_port()?;
        self.info = None;
        Ok(())
    }

    /// Opens the hardware, discovers command endpoints, and performs only read-oriented identification probes.
    pub fn connect_and_probe(&mut self) -> Result<DeviceInfo> {
        self.transport.open(AVERMEDIA_VENDOR_ID, A865R_PRODUCT_ID)?;
        let pipes = self.transport.bulk_pipes().to_vec();
        let inputs: Vec<BulkPipe> = pipes.iter().copied().filter(|pipe| pipe.input).collect();
        let outputs: Vec<BulkPipe> = pipes.iter().copied().filter(|pipe| !pipe.input).collect();
        if inputs.is_empty() || outputs.is_empty() {
            return Err(Error::Transport(
                "active USB interface does not expose both bulk IN and bulk OUT endpoints"
                    .to_string(),
            ));
        }

        let mut selected = None;
        let mut failures = Vec::new();
        'outer: for output in &outputs {
            for input in &inputs {
                if let Err(error) = self.transport.set_command_pipes(output.id, input.id) {
                    failures.push(format!("0x{:02X}->0x{:02X}: {error}", output.id, input.id));
                    continue;
                }
                let probe = {
                    let mut protocol = Protocol::new(self.transport.as_mut());
                    protocol.query_firmware_version(400)
                };
                match probe {
                    Ok(version) => {
                        selected = Some((output.id, input.id, version));
                        break 'outer;
                    }
                    Err(error) => {
                        failures.push(format!("0x{:02X}->0x{:02X}: {error}", output.id, input.id))
                    }
                }
            }
        }

        let Some((command_out, command_in, firmware_version)) = selected else {
            return Err(Error::Protocol(format!(
                "could not identify the command endpoint pair; probe failures:\n  {}",
                failures.join("\n  ")
            )));
        };
        self.transport.set_command_pipes(command_out, command_in)?;

        let (chip_version, chip_type, prechip_version, eeprom, eeprom_probe_note) = {
            let mut protocol = Protocol::new(self.transport.as_mut());
            let chip = protocol.read_registers(0x001222, 3)?;
            let chip_version = chip[0];
            let chip_type = u16::from_le_bytes([chip[1], chip[2]]);
            let prechip_version = protocol.read_registers(0x00384F, 1)?[0];
            let (eeprom, note) = probe_eeprom(&mut protocol, chip_type, chip_version);
            (chip_version, chip_type, prechip_version, eeprom, note)
        };

        let mut stream_candidates: Vec<BulkPipe> = inputs
            .iter()
            .copied()
            .filter(|pipe| pipe.id != command_in)
            .collect();
        stream_candidates.sort_by_key(|pipe| std::cmp::Reverse(pipe.maximum_packet_size));
        let transport_stream_pipe = stream_candidates.first().map(|pipe| pipe.id);

        let info = DeviceInfo {
            transport: self.transport.description(),
            chip_type,
            chip_version,
            prechip_version,
            firmware_running: firmware_version.iter().any(|byte| *byte != 0),
            firmware_version,
            bulk_pipes: pipes,
            command_out_pipe: command_out,
            command_in_pipe: command_in,
            transport_stream_pipe,
            eeprom,
            eeprom_probe_note,
        };
        self.info = Some(info.clone());
        Ok(info)
    }

    /// Returns information from the most recent successful probe.
    pub fn info(&self) -> Result<&DeviceInfo> {
        self.info.as_ref().ok_or(Error::NotConnected)
    }

    /// Reads bytes from the 24-bit device register space after probing the device.
    pub fn read_registers(&mut self, address: u32, length: usize) -> Result<Vec<u8>> {
        self.ensure_connected()?;
        let mut protocol = Protocol::new(self.transport.as_mut());
        protocol.read_registers(address, length)
    }

    /// Queries a specific firmware processor using an already encoded mailbox byte.
    ///
    /// Re-establishing the proven command pipes first aborts a stale timed-out read without issuing
    /// a physical endpoint reset. This makes it possible to compare candidate processor encodings
    /// independently on a known-good running firmware image.
    pub fn query_processor_firmware_version(
        &mut self,
        mailbox: u8,
        timeout_ms: u32,
    ) -> Result<[u8; 4]> {
        let current = self.info()?.clone();
        self.transport
            .set_command_pipes(current.command_out_pipe, current.command_in_pipe)?;
        let mut protocol = Protocol::new(self.transport.as_mut());
        protocol.query_processor_firmware_version(mailbox, timeout_ms)
    }

    /// Writes bytes to the 24-bit device register space after probing the device.
    pub fn write_registers(&mut self, address: u32, data: &[u8]) -> Result<()> {
        self.ensure_connected()?;
        let mut protocol = Protocol::new(self.transport.as_mut());
        protocol.write_registers(address, data)
    }

    /// Reads bytes from a 7-bit I2C peripheral after probing the device.
    pub fn i2c_read(&mut self, address_7bit: u8, length: usize) -> Result<Vec<u8>> {
        self.ensure_connected()?;
        let mut protocol = Protocol::new(self.transport.as_mut());
        protocol.i2c_read(address_7bit, length)
    }

    /// Writes bytes to a 7-bit I2C peripheral after probing the device.
    pub fn i2c_write(&mut self, address_7bit: u8, data: &[u8]) -> Result<()> {
        self.ensure_connected()?;
        let mut protocol = Protocol::new(self.transport.as_mut());
        protocol.i2c_write(address_7bit, data)
    }

    /// Performs a firmware-managed I2C write-then-read transaction after probing the device.
    pub fn i2c_write_read(
        &mut self,
        address_7bit: u8,
        write_prefix: &[u8],
        read_length: usize,
    ) -> Result<Vec<u8>> {
        self.ensure_connected()?;
        let mut protocol = Protocol::new(self.transport.as_mut());
        protocol.i2c_write_read(address_7bit, write_prefix, read_length)
    }

    /// Reads a complete EEPROM snapshot using an explicit memory-map layout.
    pub fn read_eeprom_with_layout(&mut self, layout: EepromLayout) -> Result<EepromInfo> {
        self.ensure_connected()?;
        let mut protocol = Protocol::new(self.transport.as_mut());
        read_eeprom(&mut protocol, layout)
    }

    /// Uploads a validated scatter image to a cold IT9175 and verifies that its firmware started.
    pub fn load_firmware(&mut self, image: &FirmwareImage) -> Result<FirmwareLoadReport> {
        let current = self.info()?.clone();
        if current.firmware_running {
            return Ok(FirmwareLoadReport {
                bytes_uploaded: 0,
                segments_uploaded: 0,
                firmware_version: current.firmware_version,
            });
        }
        if current.chip_type != 0x9175 {
            return Err(Error::Unsupported(format!(
                "this A865R firmware loader targets IT9175 only, not chip 0x{:04X}",
                current.chip_type
            )));
        }

        let mut protocol = Protocol::new(self.transport.as_mut());
        for (index, segment) in image.segments().enumerate() {
            protocol.firmware_scatter_write(segment).map_err(|error| {
                Error::Protocol(format!(
                    "firmware scatter record {index}/{} failed: {error}",
                    image.segment_count()
                ))
            })?;
        }
        protocol.firmware_boot()?;

        // AVerMedia's driver waits 10 ms. Allow a little more time and retry the harmless query so
        // slower controllers are not reported as failed merely because the first poll was early.
        let mut running_version = None;
        let mut last_query_error = None;
        for _ in 0..10 {
            thread::sleep(Duration::from_millis(20));
            match protocol.query_firmware_version(1_000) {
                Ok(version) if version.iter().any(|byte| *byte != 0) => {
                    running_version = Some(version);
                    break;
                }
                Ok(_) => {}
                Err(error) => last_query_error = Some(error.to_string()),
            }
        }
        let firmware_version = running_version.ok_or_else(|| {
            Error::Protocol(match last_query_error {
                Some(error) => {
                    format!("firmware did not start; last version query failed: {error}")
                }
                None => "firmware boot completed but its version remained 0.0.0.0".to_string(),
            })
        })?;

        let (eeprom, eeprom_probe_note) =
            probe_eeprom(&mut protocol, current.chip_type, current.chip_version);
        if let Some(info) = self.info.as_mut() {
            info.firmware_version = firmware_version;
            info.firmware_running = true;
            info.eeprom = eeprom;
            info.eeprom_probe_note = eeprom_probe_note;
        }

        Ok(FirmwareLoadReport {
            bytes_uploaded: image.bytes().len(),
            segments_uploaded: image.segment_count(),
            firmware_version,
        })
    }

    /// Boots the independently authored execution probe and verifies its shared-memory marker.
    ///
    /// The probe is deliberately useful only on a cold device. It changes RAM until the next USB
    /// power cycle and never touches tuner, demodulator, EEPROM, GPIO, or TS configuration.
    pub fn probe_open_firmware_execution(&mut self) -> Result<OpenFirmwareProbeReport> {
        self.probe_boot_image(
            execution_probe_image()?,
            OPEN_LINK_VERSION,
            OPEN_OFDM_VERSION,
            true,
        )
    }

    /// Component-isolation experiment. The reference half remains proprietary and is never exported.
    /// This is not an open-firmware reception image. Requires a physically cold original IT9175.
    pub fn probe_firmware_components(
        &mut self,
        reference: &FirmwareImage,
        open_link: bool,
    ) -> Result<OpenFirmwareProbeReport> {
        let open = execution_probe_image()?;
        let selected = if open_link {
            crate::FirmwareCore::Link
        } else {
            crate::FirmwareCore::Ofdm
        };
        let regions: Vec<_> = open
            .regions()
            .filter(|r| r.core == selected)
            .chain(reference.regions().filter(|r| r.core != selected))
            .collect();
        let image = FirmwareImage::from_regions(&regions)?;
        self.probe_boot_image(
            image,
            if open_link {
                OPEN_LINK_VERSION
            } else {
                [3, 0, 3, 0]
            },
            if open_link {
                [3, 0, 4, 5]
            } else {
                OPEN_OFDM_VERSION
            },
            !open_link,
        )
    }

    fn probe_boot_image(
        &mut self,
        image: FirmwareImage,
        expected_link: [u8; 4],
        expected_ofdm: [u8; 4],
        require_marker: bool,
    ) -> Result<OpenFirmwareProbeReport> {
        let current = self.info()?.clone();
        if current.firmware_running {
            return Err(Error::Unsupported(
                "the open-firmware execution probe requires a cold device; physically unplug and reconnect the A865R before retrying"
                    .to_string(),
            ));
        }
        if current.chip_type != 0x9175 {
            return Err(Error::Unsupported(format!(
                "the open-firmware execution probe targets IT9175, not chip 0x{:04X}",
                current.chip_type
            )));
        }

        // A failed upload leaves the device state unknown. Never let a cached cold flag authorize
        // another upload into a possibly warm core without a fresh identification query.
        self.info = None;
        let mut protocol = Protocol::new(self.transport.as_mut());
        for (index, record) in image.segments().enumerate() {
            protocol.firmware_scatter_write(record).map_err(|error| {
                Error::Protocol(format!(
                    "open firmware scatter record {index}/{} failed: {error}",
                    image.segment_count()
                ))
            })?;
        }
        let (boot_acknowledged, boot_error) = match protocol.firmware_boot_with_timeout(1_000) {
            Ok(()) => (true, None),
            Err(error) => (false, Some(error.to_string())),
        };
        drop(protocol);

        // A boot reset can interrupt the synchronous read for CMD_FW_BOOT. Cancel stale host I/O
        // without resetting either endpoint, then determine whether the link processor restarted.
        thread::sleep(Duration::from_millis(100));
        self.transport
            .set_command_pipes(current.command_out_pipe, current.command_in_pipe)?;
        let mut protocol = Protocol::new(self.transport.as_mut());
        let mut firmware_query = None;
        let mut link_error = None;
        for _ in 0..10 {
            match protocol.query_firmware_version(400) {
                Ok(version) => {
                    firmware_query = Some(version);
                    break;
                }
                Err(error) => link_error = Some(error.to_string()),
            }
            thread::sleep(Duration::from_millis(50));
        }
        let Some(firmware_query) = firmware_query else {
            let boot = boot_error.unwrap_or_else(|| "boot was acknowledged".to_string());
            let link = link_error.unwrap_or_else(|| "no response received".to_string());
            return Err(Error::Protocol(format!(
                "open image upload completed, but the link command processor did not recover after CMD_FW_BOOT (boot result: {boot}; recovery result: {link}). OFDM execution cannot yet be observed over USB; physically unplug and reconnect the tuner"
            )));
        };
        let mut running = current.clone();
        running.firmware_version = firmware_query;
        running.firmware_running = firmware_query != [0, 0, 0, 0];
        self.info = Some(running);
        if firmware_query != expected_link {
            return Err(Error::Protocol(format!(
                "the link command processor responded after the open upload, but reported unexpected version {}.{}.{}.{} instead of the open marker version {}.{}.{}.{}; unplug and reconnect to restore the cold state",
                firmware_query[0],
                firmware_query[1],
                firmware_query[2],
                firmware_query[3],
                expected_link[0],
                expected_link[1],
                expected_link[2],
                expected_link[3]
            )));
        }

        // The vendor driver queries processor 0x00 (LINK) and processor 0x08 (OFDM) separately.
        // In USB framing, OFDM selector 0x08 becomes mailbox byte 0x80. Test that scheduler before
        // asking it to execute CMD_MEM_RD so a timeout identifies the failing core unambiguously.
        let mut ofdm_firmware_query = None;
        let mut ofdm_query_error = None;
        for _ in 0..5 {
            thread::sleep(Duration::from_millis(20));
            match protocol.query_processor_firmware_version(0x80, 400) {
                Ok(version) => {
                    ofdm_firmware_query = Some(version);
                    break;
                }
                Err(error) => ofdm_query_error = Some(error.to_string()),
            }
        }
        let Some(ofdm_firmware_query) = ofdm_firmware_query else {
            let error = ofdm_query_error.unwrap_or_else(|| "no response received".to_string());
            return Err(Error::Protocol(format!(
                "the LINK command processor recovered as firmware {}.{}.{}.{}, but the OFDM processor did not answer CMD_FW_QUERYINFO on mailbox 0x80: {error}; the failure is before OFDM register access; physically unplug and reconnect the tuner",
                firmware_query[0], firmware_query[1], firmware_query[2], firmware_query[3]
            )));
        };
        if ofdm_firmware_query != expected_ofdm {
            return Err(Error::Protocol(format!(
                "the OFDM processor answered after open boot, but reported unexpected version {}.{}.{}.{} instead of {}.{}.{}.{}; unplug and reconnect to restore the cold state",
                ofdm_firmware_query[0],
                ofdm_firmware_query[1],
                ofdm_firmware_query[2],
                ofdm_firmware_query[3],
                expected_ofdm[0],
                expected_ofdm[1],
                expected_ofdm[2],
                expected_ofdm[3]
            )));
        }

        if !require_marker {
            return Ok(OpenFirmwareProbeReport {
                bytes_uploaded: image.bytes().len(),
                records_uploaded: image.segment_count(),
                boot_acknowledged,
                marker_address: 0,
                marker_value: 0,
                firmware_query,
                ofdm_firmware_query,
                scheduler_ticks: None,
            });
        }
        let ticks = read_scheduler_progress(&mut protocol)?;
        let marker_value = protocol.read_registers(OPEN_PROBE_MARKER_ADDRESS, 1)?[0];
        Ok(OpenFirmwareProbeReport {
            bytes_uploaded: image.bytes().len(),
            records_uploaded: image.segment_count(),
            boot_acknowledged,
            marker_address: OPEN_PROBE_MARKER_ADDRESS,
            marker_value,
            firmware_query,
            ofdm_firmware_query,
            scheduler_ticks: Some(ticks),
        })
    }

    /// Read-only health check for an already running open image. No RAM writes or firmware upload.
    pub fn check_open_firmware_health(&mut self) -> Result<([u8; 4], [u8; 4], (u16, u16))> {
        self.info()?;
        let mut protocol = Protocol::new(self.transport.as_mut());
        let link = protocol.query_firmware_version(1000)?;
        let ofdm = protocol.query_processor_firmware_version(0x80, 1000)?;
        if link != OPEN_LINK_VERSION || ofdm != OPEN_OFDM_VERSION {
            return Err(Error::Unsupported(
                "Both processors must run the current open firmware version".into(),
            ));
        }
        let ticks = read_scheduler_progress(&mut protocol)?;
        Ok((link, ofdm, ticks))
    }

    /// Captures raw bytes from the likely MPEG-TS endpoint and performs streaming TS diagnostics.
    pub fn capture_transport_stream(
        &mut self,
        output_path: &Path,
        seconds: u32,
        pipe_override: Option<u8>,
    ) -> Result<CaptureReport> {
        if seconds == 0 {
            return Err(Error::InvalidArgument(
                "capture duration must be greater than zero".to_string(),
            ));
        }
        let info = self.info()?.clone();
        let pipe = pipe_override
            .or(info.transport_stream_pipe)
            .ok_or_else(|| {
                Error::Transport(
                    "no separate bulk IN endpoint was identified; specify a stream pipe explicitly"
                        .to_string(),
                )
            })?;

        let mut output = File::create(output_path)?;
        let started = Instant::now();
        let deadline = started + Duration::from_secs(seconds as u64);
        let mut bytes_written = 0u64;
        let mut analyzer = TsAnalyzer::new();

        while Instant::now() < deadline {
            let data = self.transport.read_bulk(pipe, 188 * 512, 1_000)?;
            if data.is_empty() {
                continue;
            }
            output.write_all(&data)?;
            bytes_written += data.len() as u64;
            analyzer.push(&data);
        }
        output.flush()?;

        Ok(CaptureReport {
            bytes_written,
            elapsed: started.elapsed(),
            stats: analyzer.finish(),
        })
    }

    /// Ensures that endpoint and chip discovery completed before a stateful operation.
    fn ensure_connected(&self) -> Result<()> {
        if self.info.is_some() {
            Ok(())
        } else {
            Err(Error::NotConnected)
        }
    }
}

impl Default for Device {
    /// Creates the same default WinUSB-backed device as `Device::new`.
    fn default() -> Self {
        Self::new()
    }
}

/// Attempts the public family EEPROM probe and converts failures into diagnostics instead of failing device discovery.
fn probe_eeprom(
    protocol: &mut Protocol<'_>,
    chip_type: u16,
    chip_version: u8,
) -> (Option<EepromInfo>, Option<String>) {
    let Some(layout) = layout_for_chip(chip_type) else {
        return (
            None,
            Some("public family data marks this chip as having no EEPROM layout".to_string()),
        );
    };

    if layout == EepromLayout::It9135 {
        match it9135_eeprom_present(protocol, chip_version) {
            Ok(false) => {
                return (
                    None,
                    Some("IT9135 EEPROM-present register reports no EEPROM".to_string()),
                )
            }
            Err(error) => return (None, Some(format!("EEPROM presence probe failed: {error}"))),
            Ok(true) => {}
        }
    }

    match read_eeprom(protocol, layout) {
        Ok(info) => (Some(info), None),
        Err(error) => (
            None,
            Some(format!(
                "EEPROM read failed without aborting the main probe: {error}"
            )),
        ),
    }
}

/// The ROM handoff runs the scheduler and need not return to the downloaded reset routine.
/// Health therefore requires live command responses and timebase progress, not a post-return marker.
fn read_scheduler_progress(protocol: &mut Protocol<'_>) -> Result<(u16, u16)> {
    let first = protocol.read_registers(0x80441d, 2)?;
    let first = u16::from_be_bytes([first[0], first[1]]);
    for _ in 0..5 {
        thread::sleep(Duration::from_millis(20));
        let next = protocol.read_registers(0x80441d, 2)?;
        let next = u16::from_be_bytes([next[0], next[1]]);
        if next != first {
            return Ok((first, next));
        }
    }
    Err(Error::Protocol(
        "Open firmware answers commands, but the OFDM scheduler timebase did not advance".into(),
    ))
}
