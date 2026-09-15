//! Summary: Parses IT9175 scatter firmware, builds open images, and extracts the reference image.
//!
//! The open-source project does not redistribute AVerMedia's firmware. Instead, this module
//! recognizes the firmware metadata in the user's original `AVer857BDA.sys` version 12.6.64.12
//! and extracts only the 123 scatter records consumed by the device's cold-boot ROM.

use crate::error::{Error, Result};
use std::ops::Range;

const DRIVER_FIRMWARE_DATA_OFFSET: usize = 0x1DEB0;
const DRIVER_FIRMWARE_DESCRIPTOR_TABLE_OFFSET: usize = 0x1F5A0;
const DRIVER_FIRMWARE_DESCRIPTOR_COUNT_OFFSET: usize = 0x1C99F;
const EXPECTED_DESCRIPTOR_COUNT: usize = 123;
const EXPECTED_FIRMWARE_SIZE: usize = 5_863;
const DESCRIPTOR_SIZE: usize = 8;
const MAX_SCATTER_SIZE: usize = 58;

/// Selects one of the two MCS-51 address spaces exposed by the IT9175 scatter loader.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum FirmwareCore {
    Link = 0,
    Ofdm = 1,
}

impl TryFrom<u8> for FirmwareCore {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Self::Link),
            1 => Ok(Self::Ofdm),
            _ => Err(Error::Protocol(format!(
                "unknown IT9175 firmware core selector 0x{value:02X}"
            ))),
        }
    }
}

/// One decoded memory region inside a scatter-write record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FirmwareRegion<'a> {
    pub core: FirmwareCore,
    pub address: u16,
    pub data: &'a [u8],
}

#[derive(Clone, Debug)]
struct RegionRange {
    core: FirmwareCore,
    address: u16,
    data: Range<usize>,
}

/// A validated sequence of scatter-write records suitable for command `0x29`.
#[derive(Clone, Debug)]
pub struct FirmwareImage {
    bytes: Vec<u8>,
    segments: Vec<Range<usize>>,
    regions: Vec<RegionRange>,
}

impl FirmwareImage {
    /// Parses a complete IT9175 scatter image without relying on false-positive-prone marker scans.
    pub fn from_scatter_bytes(bytes: Vec<u8>) -> Result<Self> {
        if bytes.is_empty() {
            return Err(Error::Protocol("firmware image is empty".to_string()));
        }

        let mut segments = Vec::new();
        let mut regions = Vec::new();
        let mut offset = 0usize;
        while offset < bytes.len() {
            let record_index = segments.len();
            if offset + 4 > bytes.len() || !is_scatter_header(&bytes, offset) {
                return Err(Error::Protocol(format!(
                    "firmware scatter record {record_index} at offset 0x{offset:X} has an invalid header"
                )));
            }
            let core = FirmwareCore::try_from(bytes[offset + 1])?;
            let region_count = bytes[offset + 3] as usize;
            if region_count == 0 {
                return Err(Error::Protocol(format!(
                    "firmware scatter record {record_index} contains no regions"
                )));
            }
            let descriptor_end = offset
                .checked_add(4 + region_count * 3)
                .ok_or_else(|| Error::Protocol("scatter descriptor size overflow".to_string()))?;
            if descriptor_end > bytes.len() {
                return Err(Error::Protocol(format!(
                    "firmware scatter record {record_index} has truncated region descriptors"
                )));
            }

            let mut descriptors = Vec::with_capacity(region_count);
            let mut payload_size = 0usize;
            for index in 0..region_count {
                let descriptor = offset + 4 + index * 3;
                let address = u16::from_be_bytes([bytes[descriptor], bytes[descriptor + 1]]);
                let length = bytes[descriptor + 2] as usize;
                if length == 0 {
                    return Err(Error::Protocol(format!(
                        "firmware scatter record {record_index} region {index} is empty"
                    )));
                }
                if address as usize + length > 0x1_0000 {
                    return Err(Error::Protocol(format!("firmware record {record_index} region {index} crosses the 16-bit core address boundary")));
                }
                payload_size = payload_size
                    .checked_add(length)
                    .ok_or_else(|| Error::Protocol("scatter payload size overflow".to_string()))?;
                descriptors.push((address, length));
            }

            let record_end = descriptor_end
                .checked_add(payload_size)
                .ok_or_else(|| Error::Protocol("scatter record size overflow".to_string()))?;
            if record_end > bytes.len() {
                return Err(Error::Protocol(format!(
                    "firmware scatter record {record_index} payload is truncated"
                )));
            }
            if record_end - offset > MAX_SCATTER_SIZE {
                return Err(Error::Protocol(format!(
                    "firmware scatter record {record_index} exceeds the {MAX_SCATTER_SIZE}-byte transport limit"
                )));
            }

            let mut data_offset = descriptor_end;
            for (address, length) in descriptors {
                let data_end = data_offset + length;
                regions.push(RegionRange {
                    core,
                    address,
                    data: data_offset..data_end,
                });
                data_offset = data_end;
            }
            segments.push(offset..record_end);
            offset = record_end;
        }

        Ok(Self {
            bytes,
            segments,
            regions,
        })
    }

    /// Builds a deterministic image with one conservative scatter record per supplied region.
    pub fn from_regions(regions: &[FirmwareRegion<'_>]) -> Result<Self> {
        if regions.is_empty() {
            return Err(Error::InvalidArgument(
                "open firmware requires at least one memory region".to_string(),
            ));
        }
        let mut bytes = Vec::new();
        for (index, region) in regions.iter().enumerate() {
            if region.data.is_empty() || region.data.len() > MAX_SCATTER_SIZE - 7 {
                return Err(Error::InvalidArgument(format!(
                    "open firmware region {index} must contain between 1 and {} bytes",
                    MAX_SCATTER_SIZE - 7
                )));
            }
            let end = region.address as usize + region.data.len();
            if end > 0x1_0000 {
                return Err(Error::InvalidArgument(format!(
                    "open firmware region {index} crosses the 16-bit core address boundary"
                )));
            }
            bytes.extend_from_slice(&[
                0x03,
                region.core as u8,
                0x00,
                0x01,
                (region.address >> 8) as u8,
                region.address as u8,
                region.data.len() as u8,
            ]);
            bytes.extend_from_slice(region.data);
        }
        Self::from_scatter_bytes(bytes)
    }

    /// Extracts and cross-checks firmware metadata from AVerMedia driver 12.6.64.12.
    pub fn extract_from_avermedia_driver(driver: &[u8]) -> Result<Self> {
        let table_end =
            DRIVER_FIRMWARE_DESCRIPTOR_TABLE_OFFSET + EXPECTED_DESCRIPTOR_COUNT * DESCRIPTOR_SIZE;
        if driver.len() < table_end
            || driver.get(0..2) != Some(&b"MZ"[..])
            || driver.get(DRIVER_FIRMWARE_DESCRIPTOR_COUNT_OFFSET).copied()
                != Some(EXPECTED_DESCRIPTOR_COUNT as u8)
        {
            return Err(Error::Protocol(
                "this is not the supported AVer857BDA.sys 12.6.64.12 layout".to_string(),
            ));
        }

        let mut descriptor_lengths = Vec::with_capacity(EXPECTED_DESCRIPTOR_COUNT);
        let mut firmware_size = 0usize;
        for index in 0..EXPECTED_DESCRIPTOR_COUNT {
            let descriptor = DRIVER_FIRMWARE_DESCRIPTOR_TABLE_OFFSET + index * DESCRIPTOR_SIZE;
            let kind = driver[descriptor];
            let length_bytes: [u8; 4] = driver[descriptor + 4..descriptor + 8]
                .try_into()
                .map_err(|_| Error::Protocol("truncated firmware descriptor".to_string()))?;
            let length = u32::from_le_bytes(length_bytes) as usize;
            if kind != 1 || !(7..=MAX_SCATTER_SIZE).contains(&length) {
                return Err(Error::Protocol(format!(
                    "unsupported firmware descriptor {index}: type={kind}, length={length}"
                )));
            }
            firmware_size = firmware_size.checked_add(length).ok_or_else(|| {
                Error::Protocol("firmware descriptor sizes overflowed usize".to_string())
            })?;
            descriptor_lengths.push(length);
        }

        if firmware_size != EXPECTED_FIRMWARE_SIZE
            || DRIVER_FIRMWARE_DATA_OFFSET + firmware_size > DRIVER_FIRMWARE_DESCRIPTOR_TABLE_OFFSET
        {
            return Err(Error::Protocol(format!(
                "unexpected embedded firmware size: {firmware_size} bytes"
            )));
        }

        let firmware_end = DRIVER_FIRMWARE_DATA_OFFSET + firmware_size;
        if driver[firmware_end..DRIVER_FIRMWARE_DESCRIPTOR_TABLE_OFFSET]
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(Error::Protocol(
                "embedded firmware padding is not zero-filled as expected".to_string(),
            ));
        }

        let image =
            Self::from_scatter_bytes(driver[DRIVER_FIRMWARE_DATA_OFFSET..firmware_end].to_vec())?;
        if image.segments.len() != EXPECTED_DESCRIPTOR_COUNT {
            return Err(Error::Protocol(format!(
                "descriptor table contains {EXPECTED_DESCRIPTOR_COUNT} records but firmware \
                 scan found {}",
                image.segments.len()
            )));
        }
        for (index, (range, expected_length)) in
            image.segments.iter().zip(descriptor_lengths).enumerate()
        {
            if range.len() != expected_length {
                return Err(Error::Protocol(format!(
                    "firmware record {index} length does not match its AVerMedia descriptor"
                )));
            }
        }
        Ok(image)
    }

    /// Returns the exact bytes that should be saved as the extracted firmware file.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the number of validated scatter records.
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }

    /// Iterates over complete records, including each record's seven-byte device header.
    pub fn segments(&self) -> impl ExactSizeIterator<Item = &[u8]> {
        self.segments.iter().map(|range| &self.bytes[range.clone()])
    }

    /// Iterates over decoded destination regions in upload order.
    pub fn regions(&self) -> impl ExactSizeIterator<Item = FirmwareRegion<'_>> {
        self.regions.iter().map(|region| FirmwareRegion {
            core: region.core,
            address: region.address,
            data: &self.bytes[region.data.clone()],
        })
    }
}

/// Recognizes the header pattern used by both I8051 cores in the scatter image.
fn is_scatter_header(bytes: &[u8], offset: usize) -> bool {
    bytes.get(offset).copied() == Some(0x03)
        && matches!(bytes.get(offset + 1).copied(), Some(0x00) | Some(0x01))
        && bytes.get(offset + 2).copied() == Some(0x00)
}
