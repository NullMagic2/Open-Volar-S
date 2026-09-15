//! Summary: Implements the framed ITE/AF9035-family command protocol used for A865R research.

use crate::error::{Error, Result};
use crate::transport::Transport;

// The command processor uses one 64-byte buffer. Requests have a four-byte header and two-byte
// checksum; responses have a three-byte header and two-byte checksum.
const MAX_FRAME_SIZE: usize = 64;
const REGISTER_READ_CHUNK: usize = MAX_FRAME_SIZE - 3 - 2;
const REGISTER_WRITE_CHUNK: usize = MAX_FRAME_SIZE - 4 - 2 - 6;
const FIRMWARE_SCATTER_CHUNK: usize = MAX_FRAME_SIZE - 4 - 2;
const I2C_MAX_TRANSFER: usize = 40;

/// Known command opcodes used by the ITE/Afatech command processor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Command {
    MemoryRead = 0x00,
    MemoryWrite = 0x01,
    I2cRead = 0x02,
    I2cWrite = 0x03,
    InfraredGet = 0x18,
    FirmwareDownload = 0x21,
    FirmwareQueryInfo = 0x22,
    FirmwareBoot = 0x23,
    FirmwareDownloadBegin = 0x24,
    FirmwareDownloadEnd = 0x25,
    FirmwareScatterWrite = 0x29,
    GenericI2cRead = 0x2A,
    GenericI2cWrite = 0x2B,
}

/// Encodes and validates command frames over an arbitrary USB transport.
pub struct Protocol<'a> {
    transport: &'a mut dyn Transport,
    sequence: u8,
}

impl<'a> Protocol<'a> {
    pub(crate) fn read_stream(&mut self, pipe: u8, size: usize, timeout: u32) -> Result<Vec<u8>> {
        self.transport.read_bulk(pipe, size, timeout)
    }
    /// Creates a protocol session over an already opened transport.
    pub fn new(transport: &'a mut dyn Transport) -> Self {
        Self {
            transport,
            sequence: 0,
        }
    }

    /// Calculates the 16-bit one's-complement checksum used by the command protocol.
    pub fn checksum(bytes_without_checksum: &[u8]) -> Result<u16> {
        if bytes_without_checksum.len() < 2 {
            return Err(Error::Protocol(
                "cannot checksum a frame shorter than two bytes".to_string(),
            ));
        }
        let mut sum = 0u16;
        for (index, byte) in bytes_without_checksum.iter().enumerate().skip(1) {
            let value = if index & 1 != 0 {
                (*byte as u16) << 8
            } else {
                *byte as u16
            };
            sum = sum.wrapping_add(value);
        }
        Ok(!sum)
    }

    /// Builds a request frame without performing USB I/O.
    pub fn encode_request(
        mailbox: u8,
        command: Command,
        sequence: u8,
        payload: &[u8],
    ) -> Result<Vec<u8>> {
        let total_size = 4usize + payload.len() + 2usize;
        if total_size > MAX_FRAME_SIZE {
            return Err(Error::Protocol(
                "request exceeds the one-byte frame length".to_string(),
            ));
        }

        let mut frame = Vec::with_capacity(total_size);
        frame.push((total_size - 1) as u8);
        frame.push(mailbox);
        frame.push(command as u8);
        frame.push(sequence);
        frame.extend_from_slice(payload);
        let checksum = Self::checksum(&frame)?;
        frame.push((checksum >> 8) as u8);
        frame.push(checksum as u8);
        Ok(frame)
    }

    /// Validates a response frame and returns only the device payload.
    pub fn decode_response(frame: &[u8], expected_payload_size: usize) -> Result<Vec<u8>> {
        if expected_payload_size > MAX_FRAME_SIZE - 5
            || !(5..=MAX_FRAME_SIZE).contains(&frame.len())
        {
            return Err(Error::Protocol(
                "response exceeds command bounds or is shorter than its header".to_string(),
            ));
        }
        if frame.first().copied() != Some((frame.len() - 1) as u8) {
            return Err(Error::Protocol(
                "response has an invalid length byte".to_string(),
            ));
        }

        let checksum_index = frame.len() - 2;
        let calculated = Self::checksum(&frame[..checksum_index])?;
        let received = u16::from_be_bytes([frame[checksum_index], frame[checksum_index + 1]]);
        if calculated != received {
            return Err(Error::Protocol(format!(
                "response checksum mismatch: calculated 0x{calculated:04X}, received 0x{received:04X}"
            )));
        }
        if frame[2] != 0 {
            return Err(Error::Protocol(format!(
                "device returned command status 0x{:02X}",
                frame[2]
            )));
        }
        let expected_size = 5 + expected_payload_size;
        if frame.len() != expected_size {
            return Err(Error::Protocol(format!(
                "unexpected response size: expected {expected_size}, received {}",
                frame.len()
            )));
        }
        Ok(frame[3..checksum_index].to_vec())
    }

    /// Validates an IR reply. Status 1 means no key; every other error remains an error.
    pub fn decode_infrared_response(frame: &[u8], sequence: u8) -> Result<Option<[u8; 4]>> {
        if frame.get(1).copied() != Some(sequence) {
            return Err(Error::Protocol(
                "infrared response sequence mismatch".into(),
            ));
        }
        if frame.get(2) == Some(&1) {
            if !matches!(frame.len(), 5 | 9) || frame[0] as usize + 1 != frame.len() {
                return Err(Error::Protocol(
                    "invalid empty infrared response length".into(),
                ));
            }
            let end = frame.len() - 2;
            if Self::checksum(&frame[..end])? != u16::from_be_bytes([frame[end], frame[end + 1]]) {
                return Err(Error::Protocol(
                    "empty infrared response checksum mismatch".into(),
                ));
            }
            return Ok(None);
        }
        let data = Self::decode_response(frame, 4)?;
        Ok(Some(data.try_into().unwrap()))
    }

    /// Reads a decoded IR code; no EEPROM, RF, firmware or power writes are performed.
    /// The four bytes are retained verbatim until a handset's protocol is verified.
    pub fn poll_infrared(&mut self) -> Result<Option<[u8; 4]>> {
        let sequence = self.sequence;
        let request = Self::encode_request(0, Command::InfraredGet, sequence, &[])?;
        self.sequence = self.sequence.wrapping_add(1);
        let response = self.transport.exchange(&request, 9, 500)?;
        Self::decode_infrared_response(&response, sequence)
    }

    /// Sends one request and validates sequence, status, length, and checksum in the response.
    pub fn transact(
        &mut self,
        mailbox: u8,
        command: Command,
        payload: &[u8],
        expected_payload_size: usize,
        timeout_ms: u32,
    ) -> Result<Vec<u8>> {
        if expected_payload_size > MAX_FRAME_SIZE - 5 {
            return Err(Error::InvalidArgument(
                "response payload exceeds 59 bytes".to_string(),
            ));
        }
        let sequence = self.sequence;
        let request = Self::encode_request(mailbox, command, sequence, payload)?;
        self.sequence = self.sequence.wrapping_add(1);
        let expected_response_size = 3usize + expected_payload_size + 2usize;
        let response = self
            .transport
            .exchange(&request, expected_response_size, timeout_ms)?;
        if response.get(1).copied() != Some(sequence) {
            return Err(Error::Protocol(format!(
                "response sequence mismatch: expected 0x{sequence:02X}, received {:?}",
                response.get(1)
            )));
        }
        Self::decode_response(&response, expected_payload_size)
    }

    /// Queries the four-byte running firmware version tuple.
    pub fn query_firmware_version(&mut self, timeout_ms: u32) -> Result<[u8; 4]> {
        self.query_processor_firmware_version(0x00, timeout_ms)
    }

    /// Queries one processor through the vendor protocol's processor/mailbox selector.
    ///
    /// The original Windows host driver uses selector `0x00` for LINK and `0x08` for OFDM. The
    /// command framing shifts that selector into the mailbox high nibble, yielding USB mailbox
    /// byte `0x80` for the OFDM processor.
    pub fn query_processor_firmware_version(
        &mut self,
        mailbox: u8,
        timeout_ms: u32,
    ) -> Result<[u8; 4]> {
        let payload = self.transact(mailbox, Command::FirmwareQueryInfo, &[1], 4, timeout_ms)?;
        payload.try_into().map_err(|_| {
            Error::Protocol("firmware query returned an invalid payload length".to_string())
        })
    }

    /// Reads an arbitrary contiguous range from the 24-bit device register space in safe chunks.
    pub fn read_registers(&mut self, address: u32, length: usize) -> Result<Vec<u8>> {
        validate_register_range(address, length)?;
        if length == 0 {
            return Ok(Vec::new());
        }

        let mut output = Vec::with_capacity(length);
        let mut remaining = length;
        let mut current = address;
        while remaining > 0 {
            let chunk = remaining
                .min(REGISTER_READ_CHUNK)
                .min(0x1_0000 - (current as usize & 0xffff));
            let chunk_u8 = chunk as u8;
            let payload = [
                chunk_u8,
                2,
                0,
                0,
                ((current >> 8) & 0xFF) as u8,
                (current & 0xFF) as u8,
            ];
            let data = self.transact(
                ((current >> 16) & 0xFF) as u8,
                Command::MemoryRead,
                &payload,
                chunk,
                2_000,
            )?;
            output.extend_from_slice(&data);
            current += chunk as u32;
            remaining -= chunk;
        }
        Ok(output)
    }

    /// Writes an arbitrary contiguous range to the 24-bit device register space in safe chunks.
    pub fn write_registers(&mut self, address: u32, data: &[u8]) -> Result<()> {
        validate_register_range(address, data.len())?;
        if data.is_empty() {
            return Err(Error::InvalidArgument(
                "register write requires at least one byte".to_string(),
            ));
        }

        let mut offset = 0usize;
        while offset < data.len() {
            let current = address + offset as u32;
            let chunk = (data.len() - offset)
                .min(REGISTER_WRITE_CHUNK)
                .min(0x1_0000 - (current as usize & 0xffff));
            let mut payload = Vec::with_capacity(6 + chunk);
            payload.extend_from_slice(&[
                chunk as u8,
                2,
                0,
                0,
                ((current >> 8) & 0xFF) as u8,
                (current & 0xFF) as u8,
            ]);
            payload.extend_from_slice(&data[offset..offset + chunk]);
            let _ = self.transact(
                ((current >> 16) & 0xFF) as u8,
                Command::MemoryWrite,
                &payload,
                0,
                2_000,
            )?;
            offset += chunk;
        }
        Ok(())
    }

    /// Reads bytes from a 7-bit I2C peripheral through the firmware-managed I2C bridge.
    pub fn i2c_read(&mut self, address_7bit: u8, length: usize) -> Result<Vec<u8>> {
        validate_i2c_address(address_7bit)?;
        validate_i2c_length(length, "I2C read")?;
        if length == 0 {
            return Ok(Vec::new());
        }

        let payload = [length as u8, address_7bit << 1, 0, 0, 0];
        self.transact(0, Command::I2cRead, &payload, length, 2_000)
    }

    /// Writes bytes to a 7-bit I2C peripheral through the firmware-managed I2C bridge.
    pub fn i2c_write(&mut self, address_7bit: u8, data: &[u8]) -> Result<()> {
        validate_i2c_address(address_7bit)?;
        validate_i2c_length(data.len(), "I2C write")?;
        if data.is_empty() {
            return Err(Error::InvalidArgument(
                "I2C write requires at least one byte".to_string(),
            ));
        }

        let mut payload = Vec::with_capacity(5 + data.len());
        payload.extend_from_slice(&[data.len() as u8, address_7bit << 1, 0, 0, 0]);
        payload.extend_from_slice(data);
        let _ = self.transact(0, Command::I2cWrite, &payload, 0, 2_000)?;
        Ok(())
    }

    /// Performs a repeated-start style write-then-read transaction used by many tuner register maps.
    pub fn i2c_write_read(
        &mut self,
        address_7bit: u8,
        write_prefix: &[u8],
        read_length: usize,
    ) -> Result<Vec<u8>> {
        validate_i2c_address(address_7bit)?;
        validate_i2c_length(write_prefix.len(), "I2C write prefix")?;
        validate_i2c_length(read_length, "I2C read")?;
        if read_length == 0 {
            return Err(Error::InvalidArgument(
                "I2C write-read requires a non-zero read length".to_string(),
            ));
        }

        let mut payload = Vec::with_capacity(5 + write_prefix.len());
        payload.extend_from_slice(&[read_length as u8, address_7bit << 1, 0, 0, 0]);
        payload.extend_from_slice(write_prefix);
        self.transact(0, Command::I2cRead, &payload, read_length, 2_000)
    }

    /// Writes a firmware scatter segment using the public ITE-family command opcode.
    pub fn firmware_scatter_write(&mut self, segment: &[u8]) -> Result<()> {
        if segment.is_empty() || segment.len() > FIRMWARE_SCATTER_CHUNK {
            return Err(Error::InvalidArgument(format!(
                "firmware scatter segments must contain between 1 and \
                     {FIRMWARE_SCATTER_CHUNK} bytes"
            )));
        }
        let _ = self.transact(0, Command::FirmwareScatterWrite, segment, 0, 5_000)?;
        Ok(())
    }

    /// Requests firmware boot after a separately validated firmware download sequence.
    pub fn firmware_boot(&mut self) -> Result<()> {
        self.firmware_boot_with_timeout(5_000)
    }

    /// Requests firmware boot with a caller-selected acknowledgement timeout.
    pub fn firmware_boot_with_timeout(&mut self, timeout_ms: u32) -> Result<()> {
        let _ = self.transact(0, Command::FirmwareBoot, &[], 0, timeout_ms)?;
        Ok(())
    }
}

/// Verifies that a register range remains inside the protocol's 24-bit address space.
fn validate_register_range(address: u32, length: usize) -> Result<()> {
    if address > 0x00FF_FFFF {
        return Err(Error::InvalidArgument(
            "register address exceeds 24 bits".to_string(),
        ));
    }
    let end = address as u64 + length.saturating_sub(1) as u64;
    if length > 0 && end > 0x00FF_FFFF {
        return Err(Error::InvalidArgument(
            "register range crosses the 24-bit address boundary".to_string(),
        ));
    }
    Ok(())
}
/// Verifies that an I2C address fits the ordinary seven-bit address space.
fn validate_i2c_address(address_7bit: u8) -> Result<()> {
    if address_7bit > 0x7F {
        return Err(Error::InvalidArgument(
            "I2C address must be a 7-bit value between 0x00 and 0x7F".to_string(),
        ));
    }
    Ok(())
}

/// Applies the conservative 40-byte transaction limit used by the public AF9035 implementation.
fn validate_i2c_length(length: usize, operation: &str) -> Result<()> {
    if length > I2C_MAX_TRANSFER {
        return Err(Error::InvalidArgument(format!(
            "{operation} exceeds the conservative {I2C_MAX_TRANSFER}-byte limit"
        )));
    }
    Ok(())
}
