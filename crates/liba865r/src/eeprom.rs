//! Summary: Reads and interprets the memory-mapped EEPROM layout used by AF9035/IT9135-family devices.
//!
//! The public Linux driver documents these EEPROM windows as read-only memory mappings, making this
//! probe appropriate for diagnostics. Unknown chip layouts are kept explicit rather than guessed.

use crate::error::{Error, Result};
use crate::protocol::Protocol;

const EEPROM_SIZE: usize = 256;
const EEPROM_BASE_AF9035: u32 = 0x42F5;
const EEPROM_BASE_IT9135: u32 = 0x4994;
const EEPROM_TS_MODE: usize = 0x31;
const EEPROM_2ND_DEMOD_ADDR: usize = 0x32;
const EEPROM_IR_MODE: usize = 0x18;
const EEPROM_IR_TYPE: usize = 0x34;
const EEPROM_1_IF_L: usize = 0x38;
const EEPROM_1_IF_H: usize = 0x39;
const EEPROM_1_TUNER_ID: usize = 0x3C;

/// Identifies which public EEPROM memory-map layout was selected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EepromLayout {
    Af9035,
    It9135,
}

impl EepromLayout {
    /// Returns the memory-mapped base address for this layout.
    pub fn base_address(self) -> u32 {
        match self {
            Self::Af9035 => EEPROM_BASE_AF9035,
            Self::It9135 => EEPROM_BASE_IT9135,
        }
    }
}

/// Summarizes fields that are useful when identifying the tuner configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EepromSummary {
    pub ts_mode: u8,
    pub dual_mode: bool,
    pub second_demod_address_8bit: u8,
    pub ir_mode: u8,
    pub ir_type: u8,
    pub tuner_id: u8,
    pub tuner_if_khz: u16,
}

/// Contains a complete EEPROM snapshot plus its parsed summary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EepromInfo {
    pub layout: EepromLayout,
    pub base_address: u32,
    pub bytes: Vec<u8>,
    pub summary: EepromSummary,
}

impl EepromInfo {
    /// Parses known fields from a complete 256-byte EEPROM snapshot.
    pub fn from_bytes(layout: EepromLayout, bytes: Vec<u8>) -> Result<Self> {
        if bytes.len() != EEPROM_SIZE {
            return Err(Error::Protocol(format!(
                "EEPROM snapshot must be {EEPROM_SIZE} bytes, received {}",
                bytes.len()
            )));
        }
        let ts_mode = bytes[EEPROM_TS_MODE];
        let dual_mode =
            matches!(ts_mode, 1 | 3) || (layout == EepromLayout::Af9035 && ts_mode == 5);
        let tuner_if_khz = u16::from_le_bytes([bytes[EEPROM_1_IF_L], bytes[EEPROM_1_IF_H]]);
        Ok(Self {
            layout,
            base_address: layout.base_address(),
            summary: EepromSummary {
                ts_mode,
                dual_mode,
                second_demod_address_8bit: bytes[EEPROM_2ND_DEMOD_ADDR],
                ir_mode: bytes[EEPROM_IR_MODE],
                ir_type: bytes[EEPROM_IR_TYPE],
                tuner_id: bytes[EEPROM_1_TUNER_ID],
                tuner_if_khz,
            },
            bytes,
        })
    }
}

/// Selects a public EEPROM layout for a detected chip type.
pub fn layout_for_chip(chip_type: u16) -> Option<EepromLayout> {
    match chip_type {
        // The original AVerMedia driver handles IT9175 identically to IT9135 for this path.
        0x9135 | 0x9175 => Some(EepromLayout::It9135),
        0x9306 => None,
        // The public AF9035 driver uses the AF9035 map for non-IT9135/non-IT930x members.
        _ => Some(EepromLayout::Af9035),
    }
}

/// Checks the IT9135 EEPROM-present register documented by the public Linux driver.
pub fn it9135_eeprom_present(protocol: &mut Protocol<'_>, chip_version: u8) -> Result<bool> {
    let presence_register = if chip_version == 0x02 {
        0x00461D
    } else {
        0x00461B
    };
    Ok(protocol.read_registers(presence_register, 1)?[0] != 0)
}

/// Reads a complete 256-byte EEPROM snapshot from the selected memory-mapped window.
pub fn read_eeprom(protocol: &mut Protocol<'_>, layout: EepromLayout) -> Result<EepromInfo> {
    let bytes = protocol.read_registers(layout.base_address(), EEPROM_SIZE)?;
    EepromInfo::from_bytes(layout, bytes)
}
