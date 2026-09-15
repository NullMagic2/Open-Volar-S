//! Summary: Generates independently authored IT9175 MCS-51 coordinated-boot probes.
//!
//! This module contains no vendor executable payload. The programs are assembled from
//! named Intel 8051 instructions and the small ROM/RAM interface contract established by hardware
//! observation. They are intentionally limited to reversible RAM-execution milestones.

mod services;
mod startup_data;
use crate::error::Result;
use crate::firmware::{FirmwareCore, FirmwareImage, FirmwareRegion};

/// ROM reset entry used by the cold IT9175 link core.
const LINK_ROM_RESET_ENTRY: u16 = 0x12BF;
/// Immutable ROM helper used by the link startup contract to publish its version bytes.
const LINK_ROM_COPY_ENTRY: u16 = 0x5413;
/// RAM hook called by the link ROM during firmware startup.
const LINK_STARTUP_HOOK: u16 = 0x4180;
/// Code-memory location of the four open version bytes inside the startup hook block.
const OPEN_LINK_VERSION_CODE_ADDRESS: u16 = 0x4186;
/// XDATA mailbox read by the ROM firmware-query command.
const OPEN_LINK_VERSION_XDATA_ADDRESS: u16 = 0x4BFC;
/// Independently authored link-startup routine.
pub const OPEN_LINK_ENTRY: u16 = 0x4193;
/// Downloaded callback address installed into the link ROM's callback table.
const OPEN_LINK_CALLBACK_ENTRY: u16 = 0x4204;
/// Version published by the independently authored link-startup probe.
pub const OPEN_LINK_VERSION: [u8; 4] = [0, 1, 4, 0];
/// Independently authored OFDM startup entry in the device's demonstrated executable-RAM window.
pub const OPEN_PROBE_ENTRY: u16 = 0x4870;
/// Shared no-op for the five ABI entries that are RET in the reference itself.
// Keep callbacks within RAM exercised by reference downloads (0x4870..0x57D1).
// The earlier 0x6100 location had no demonstrated executable-RAM evidence.
const OPEN_OFDM_CALLBACK_ENTRY: u16 = 0x4970;
/// Small open replacement for a downloaded veneer that returns zero in R7.
const OPEN_OFDM_ZERO_R7_ENTRY: u16 = 0x4971;
/// Small open replacement for a downloaded veneer that clears MMIO byte 0xEC86.
const OPEN_OFDM_CLEAR_EC86_ENTRY: u16 = 0x4974;
/// Small open replacements for downloaded interrupt-state veneers.
const OPEN_OFDM_CLEAR_EA_ENTRY: u16 = 0x497A;
const OPEN_OFDM_SET_EA_ENTRY: u16 = 0x497D;
/// Source-authored implementation of the observed serial callback contract.
const OPEN_OFDM_SERIAL_CALLBACK_ENTRY: u16 = 0x4960;
/// Fixed trampoline into the reconstructed periodic timer service.
const OPEN_OFDM_TIMER_CALLBACK_ENTRY: u16 = 0x495B;
/// First entry of the 128-element OFDM ROM dispatch veneer table.
const OFDM_VECTOR_TABLE_ADDRESS: u16 = 0x6680;
/// Downloaded veneer used by the reference OFDM reset path to enter the immutable ROM startup.
const OFDM_ROM_STARTUP_VENEER: u16 = 0x67BE;
/// Immutable OFDM ROM service that completes the coordinated two-core boot handoff.
#[cfg(test)]
const OFDM_ROM_STARTUP_ENTRY: u16 = 0xA0A6;
/// XDATA ready byte initialized immediately before the OFDM ROM startup service.
const OFDM_READY_XDATA_ADDRESS: u16 = 0xFB3A;
/// First byte of the 64-byte OFDM callback table.
const OFDM_CALLBACK_TABLE_ADDRESS: u16 = 0xFB3B;
/// Source-authored OFDM version published in the ROM-visible XDATA mailbox.
pub const OPEN_OFDM_VERSION: [u8; 4] = [0, 1, 4, 0];
/// First byte of the OFDM version mailbox.
const OPEN_OFDM_VERSION_XDATA_ADDRESS: u16 = 0x4191;
/// Shared XDATA byte written by the OFDM probe so the host can verify execution.
pub const OPEN_PROBE_MARKER_ADDRESS: u32 = 0x8046FF;
/// Value written to [`OPEN_PROBE_MARKER_ADDRESS`].
pub const OPEN_PROBE_MARKER_VALUE: u8 = 0xA5;

/// Conservative payload size for one generated scatter record.
const GENERATED_REGION_CHUNK: usize = 48;

/// Generates a blob-free image that preserves both cores' ROM boot ABI and probes OFDM execution.
pub fn execution_probe_image() -> Result<FirmwareImage> {
    let link_reset = long_jump(LINK_ROM_RESET_ENTRY);
    let ofdm_reset = long_jump(OPEN_PROBE_ENTRY);

    // The link ROM calls 0x4180 as its downloadable startup hook. Keep the hook layout explicit:
    // an LJMP to our source-generated routine, a harmless secondary RET hook, two reserved bytes,
    // and the open probe version. No reference-firmware payload is included.
    let mut link_hook = Vec::from(long_jump(OPEN_LINK_ENTRY));
    link_hook.extend_from_slice(&[0x22, 0x00, 0x00]); // RET; reserved; reserved
    link_hook.extend_from_slice(&OPEN_LINK_VERSION);

    let link_program = link_startup_program();
    // The link ROM's second callback-table entry targets downloaded address 0x4204. The reference
    // callback derives the bridge clock divider from the ROM-populated 0x45C7..0x45CA tuple and
    // applies the gate mask at 0xF6AF to six D8Cx bridge registers. Reimplement that contract: a
    // bare RET is sufficient for USB/link commands, but leaves the link-to-OFDM path unconfigured.
    let link_callback = link_bridge_callback_program();

    let reconstructed = services::build();
    let ofdm_program = ofdm_startup_program();
    let startup_tables = startup_data::tables();
    // Unsupported OFDM ROM patches are deliberately left absent. Installing a RET as a replacement
    // is not neutral: it redirects a working mask-ROM entry away from its implementation. Keep the
    // understood ROM-to-ROM compatibility patch plus source-authored Timer-2 and serial
    // acknowledgements.
    let ofdm_timer_callback = ofdm_timer_callback_program();
    let ofdm_serial_callback = ofdm_serial_callback_program();
    let ofdm_compatibility_handlers = ofdm_compatibility_handler_program();
    let ofdm_vector_table = ofdm_vector_table_program();

    let mut regions = vec![
        FirmwareRegion {
            core: FirmwareCore::Link,
            address: 0x4100,
            data: &link_reset,
        },
        FirmwareRegion {
            core: FirmwareCore::Link,
            address: LINK_STARTUP_HOOK,
            data: &link_hook,
        },
    ];
    for (index, chunk) in link_program.chunks(GENERATED_REGION_CHUNK).enumerate() {
        regions.push(FirmwareRegion {
            core: FirmwareCore::Link,
            address: OPEN_LINK_ENTRY + (index * GENERATED_REGION_CHUNK) as u16,
            data: chunk,
        });
    }
    for (index, chunk) in link_callback.chunks(GENERATED_REGION_CHUNK).enumerate() {
        regions.push(FirmwareRegion {
            core: FirmwareCore::Link,
            address: OPEN_LINK_CALLBACK_ENTRY + (index * GENERATED_REGION_CHUNK) as u16,
            data: chunk,
        });
    }
    regions.push(FirmwareRegion {
        core: FirmwareCore::Ofdm,
        address: 0x4100,
        data: &ofdm_reset,
    });
    for (index, chunk) in ofdm_program.chunks(GENERATED_REGION_CHUNK).enumerate() {
        regions.push(FirmwareRegion {
            core: FirmwareCore::Ofdm,
            address: OPEN_PROBE_ENTRY + (index * GENERATED_REGION_CHUNK) as u16,
            data: chunk,
        });
    }
    for (index, chunk) in startup_tables.chunks(GENERATED_REGION_CHUNK).enumerate() {
        regions.push(FirmwareRegion {
            core: FirmwareCore::Ofdm,
            address: 0x4700 + (index * GENERATED_REGION_CHUNK) as u16,
            data: chunk,
        });
    }
    regions.extend_from_slice(&[
        FirmwareRegion {
            core: FirmwareCore::Ofdm,
            address: OPEN_OFDM_TIMER_CALLBACK_ENTRY,
            data: &ofdm_timer_callback,
        },
        FirmwareRegion {
            core: FirmwareCore::Ofdm,
            address: OPEN_OFDM_SERIAL_CALLBACK_ENTRY,
            data: &ofdm_serial_callback,
        },
        FirmwareRegion {
            core: FirmwareCore::Ofdm,
            address: OPEN_OFDM_CALLBACK_ENTRY,
            data: &ofdm_compatibility_handlers,
        },
    ]);
    for (index, chunk) in ofdm_vector_table.chunks(GENERATED_REGION_CHUNK).enumerate() {
        regions.push(FirmwareRegion {
            core: FirmwareCore::Ofdm,
            address: OFDM_VECTOR_TABLE_ADDRESS + (index * GENERATED_REGION_CHUNK) as u16,
            data: chunk,
        });
    }

    for (index, chunk) in reconstructed
        .code
        .chunks(GENERATED_REGION_CHUNK)
        .enumerate()
    {
        regions.push(FirmwareRegion {
            core: FirmwareCore::Ofdm,
            address: services::ORIGIN + (index * GENERATED_REGION_CHUNK) as u16,
            data: chunk,
        });
    }

    FirmwareImage::from_regions(&regions)
}

/// Dispatches to the periodic timebase, status and ROM-maintenance service.
fn ofdm_timer_callback_program() -> Vec<u8> {
    long_jump(services::build().entries[&0x4A1B]).to_vec()
}

/// Reconstructs the complete OFDM ROM dispatch veneer table from derived ABI targets.
fn ofdm_vector_table_program() -> Vec<u8> {
    const TARGETS: [u16; 128] = [
        0x4DDA, 0xC7AC, 0xC71D, 0x503F, 0xB66C, 0xC9CD, 0xB570, 0x9698, 0x96D2, 0x9790, 0x95D4,
        0x86F9, 0xB35B, 0xB45B, 0xD4E5, 0xAE46, 0xB42B, 0xB555, 0x6DBB, 0x6AA8, 0x578D, 0xA3C3,
        0xA3F6, 0xA41A, 0xA436, 0xA4AB, 0xA4E9, 0x55B6, 0xB78B, 0xB7E9, 0xB848, 0xB8D9, 0xBB31,
        0xBB91, 0xBC2F, 0xBDFF, 0xD048, 0xBEFC, 0xC065, 0xC2E1, 0xC3B2, 0xC54B, 0xC5CF, 0xA9BD,
        0xA9AF, 0x8D98, 0x5153, 0xB410, 0x554E, 0xD210, 0xCE34, 0xCE85, 0xD502, 0xCE46, 0xCB24,
        0xCEE9, 0xCA9C, 0xCA33, 0x963E, 0x95FF, 0x95B6, 0x57C3, 0xCC33, 0xCCBC, 0xCD32, 0xCDC3,
        0xD615, 0xD68F, 0xCFF0, 0xCA76, 0xD0E1, 0x559B, 0x6800, 0xA058, 0xA9FD, 0x75B8, 0xABAF,
        0x7E3B, 0x8D4A, 0x8D3F, 0x37F5, 0xA9CA, 0xAAF8, 0xAB7B, 0x7BAC, 0xA1F6, 0x81FF, 0x820D,
        0x82C4, 0xA972, 0xA605, 0x362D, 0x84F0, 0x57BB, 0xD503, 0xB713, 0x57C4, 0x57C5, 0x57C6,
        0x57CC, 0x57CD, 0xD6FE, 0xD7AC, 0xD17F, 0x4EF6, 0xD819, 0xA0A6, 0x4F13, 0xB2C7, 0xAD11,
        0xDA41, 0x8402, 0x856E, 0xE12B, 0xD959, 0xB1BA, 0xA356, 0xD8AA, 0xD91C, 0xC6CC, 0xC701,
        0x9829, 0x9974, 0x99AD, 0xE1E7, 0xC165, 0x4EF0, 0x4EF3,
    ];

    let reconstructed = services::build();
    let mut table = Vec::with_capacity(TARGETS.len() * 3);
    for target in TARGETS {
        let open_target = match target {
            0x4DDA | 0x503F | 0x578D | 0x55B6 | 0x5153 | 0x554E | 0x559B | 0x4EF6 | 0x4F13 => {
                reconstructed.entries[&target]
            }
            // These five reference entries are themselves RET, not missing implementations.
            0x57C3 | 0x57C4 | 0x57C5 | 0x57CC | 0x57CD => OPEN_OFDM_CALLBACK_ENTRY,
            0x57BB => OPEN_OFDM_ZERO_R7_ENTRY,
            0x57C6 => OPEN_OFDM_CLEAR_EC86_ENTRY,
            0x4EF0 => OPEN_OFDM_CLEAR_EA_ENTRY,
            0x4EF3 => OPEN_OFDM_SET_EA_ENTRY,
            _ => target,
        };
        table.extend_from_slice(&long_jump(open_target));
    }
    table
}

/// Implements the observed serial-event acknowledgement without board-specific behavior.
fn ofdm_serial_callback_program() -> Vec<u8> {
    let mut code = Vec::new();
    code.extend_from_slice(&[0xC2, 0x99, 0xC2, 0x98]); // CLR TI; CLR RI
    emit_mov_dptr(&mut code, 0x460D);
    code.extend_from_slice(&[0xE0, 0x04, 0xF0, 0x22]); // MOVX A; INC A; MOVX; RET
    code
}

/// Packs five small source-authored compatibility handlers contiguously at 0x4970.
fn ofdm_compatibility_handler_program() -> Vec<u8> {
    let mut code = vec![0x22]; // 0x4970: RET
    code.extend_from_slice(&[0x7F, 0x00, 0x22]); // 0x4971: MOV R7,#0; RET
    emit_mov_dptr(&mut code, 0xEC86); // 0x4974
    code.extend_from_slice(&[0xE4, 0xF0, 0x22]); // CLR A; MOVX; RET
    code.extend_from_slice(&[0xC2, 0xAF, 0x22]); // 0x497A: CLR EA; RET
    code.extend_from_slice(&[0xD2, 0xAF, 0x22]); // 0x497D: SETB EA; RET
    code
}

/// Reimplements the link ROM callback that configures the bridge clock divider and gate bits.
///
/// The routine is assembled from named MCS-51 operations. It deliberately touches only the
/// register set used by this callback contract and contains no copied firmware payload.
fn link_bridge_callback_program() -> Vec<u8> {
    const ZERO_DIVIDER: usize = 0;
    const MODE_ZERO: usize = 1;
    const LARGE_DIVISOR: usize = 2;
    const TINY_DIVISOR: usize = 3;
    const STORE_DIVIDER: usize = 4;
    const CLEAR_D8C3: usize = 5;
    const D8C3_DONE: usize = 6;
    const CLEAR_D8C7: usize = 7;
    const CLEAR_ALL_GATES: usize = 8;
    const DONE: usize = 9;
    const MASTER_GATE_ENABLED: usize = 10;
    const SET_GATE_GROUP: usize = 11;

    let mut asm = Mcs51Assembler::<12>::new(OPEN_LINK_CALLBACK_ENTRY);

    asm.bytes(&[0xC2, 0xE9]); // CLR link callback interrupt gate
    asm.read_xdata_to_register(0x45C7, 7);
    asm.read_xdata_to_register(0x45C8, 6);
    asm.read_xdata_to_register(0x45C9, 5);
    asm.read_xdata_to_register(0x45CA, 4);
    asm.bytes(&[0xD2, 0xE9]); // SETB link callback interrupt gate

    // A zero fourth tuple byte or mode >= 3 selects a disabled divider.
    asm.bytes(&[0xEC]); // MOV A,R4
    asm.rel(0x60, ZERO_DIVIDER); // JZ
    asm.bytes(&[0xED, 0xD3, 0x94, 0x02]); // MOV A,R5; SETB C; SUBB A,#2
    asm.rel(0x50, ZERO_DIVIDER); // JNC: R5 >= 3
    asm.bytes(&[0xED]); // MOV A,R5
    asm.rel(0x60, MODE_ZERO); // JZ

    // Modes 1 and 2: 0x37 - (mode << 4) - high_nibble(byte1).
    asm.bytes(&[0xED, 0xC4, 0x54, 0xF0, 0xFC]); // SWAP; ANL #F0; MOV R4,A
    asm.bytes(&[0x74, 0x37, 0xC3, 0x9C, 0xFC]); // MOV A,#37; CLR C; SUBB R4; save
    asm.bytes(&[0xEE, 0xC4, 0x54, 0x0F, 0xFB]); // high nibble byte1 -> R3
    asm.bytes(&[0xEC, 0xC3, 0x9B]); // subtract R3
    asm.abs(0x02, STORE_DIVIDER); // LJMP

    asm.label(MODE_ZERO);
    asm.bytes(&[0xEE, 0xD3, 0x94, 0x0C]); // byte1 - 13
    asm.rel(0x50, LARGE_DIVISOR); // JNC: byte1 >= 13
    asm.bytes(&[0xEE, 0xD3, 0x94, 0x02]); // byte1 - 3
    asm.rel(0x40, TINY_DIVISOR); // JC: byte1 <= 2

    // byte1 in 3..12: 0x5F - 2*byte1 - one selector bit from byte0.
    asm.bytes(&[0xEE, 0x25, 0xE0, 0xFC]); // MOV A,R6; ADD A,ACC; MOV R4,A
    asm.bytes(&[0x74, 0x5F, 0xC3, 0x9C, 0xFC]);
    asm.bytes(&[0xEF, 0xC4, 0xC3, 0x13, 0x13, 0x13, 0x54, 0x01, 0xFB]);
    asm.bytes(&[0xEC, 0xC3, 0x9B]);
    asm.abs(0x02, STORE_DIVIDER);

    asm.label(LARGE_DIVISOR);
    asm.bytes(&[0xEE, 0xC3, 0x13, 0x13, 0x13, 0x54, 0x1F, 0xFC]);
    asm.bytes(&[0x74, 0x49, 0xC3, 0x9C]);
    asm.abs(0x02, STORE_DIVIDER);

    asm.label(TINY_DIVISOR);
    asm.bytes(&[0x74, 0x64]);
    asm.abs(0x02, STORE_DIVIDER);

    asm.label(ZERO_DIVIDER);
    asm.bytes(&[0xE4]); // CLR A

    asm.label(STORE_DIVIDER);
    asm.write_accumulator_to_xdata(0xF6A5);

    // F6AF bit 0 is the master gate. Bit 3 selects set-vs-clear for the bridge group, while bits
    // 1 and 2 independently control D8C3 and D8C7.
    asm.read_xdata(0xF6AF);
    asm.bit_rel(0x20, 0xE0, MASTER_GATE_ENABLED); // JB ACC.0
    asm.abs(0x02, DONE);
    asm.label(MASTER_GATE_ENABLED);
    asm.bit_rel(0x20, 0xE3, SET_GATE_GROUP); // JB ACC.3
    asm.abs(0x02, CLEAR_ALL_GATES);
    asm.label(SET_GATE_GROUP);
    asm.set_xdata_bit_zero(0xD8C5);
    asm.set_xdata_bit_zero(0xD8C4);
    asm.read_xdata(0xF6AF);
    asm.bit_rel(0x30, 0xE1, CLEAR_D8C3);
    asm.set_xdata_bit_zero(0xD8C3);
    asm.abs(0x02, D8C3_DONE);
    asm.label(CLEAR_D8C3);
    asm.clear_xdata_bit_zero(0xD8C3);
    asm.label(D8C3_DONE);
    asm.set_xdata_bit_zero(0xD8C9);
    asm.set_xdata_bit_zero(0xD8C8);
    asm.read_xdata(0xF6AF);
    asm.bit_rel(0x30, 0xE2, CLEAR_D8C7);
    asm.set_xdata_bit_zero(0xD8C7);
    asm.abs(0x02, DONE);
    asm.label(CLEAR_D8C7);
    asm.clear_xdata_bit_zero(0xD8C7);
    asm.abs(0x02, DONE);

    asm.label(CLEAR_ALL_GATES);
    for address in [0xD8C5, 0xD8C4, 0xD8C3, 0xD8C9, 0xD8C8, 0xD8C7] {
        asm.clear_xdata_bit_zero(address);
    }

    asm.label(DONE);
    asm.bytes(&[0x22]); // RET
    asm.finish()
}

/// Tiny label-aware assembler used to keep source-authored branch logic readable.
struct Mcs51Assembler<const LABELS: usize> {
    origin: u16,
    code: Vec<u8>,
    labels: [Option<usize>; LABELS],
    relative_fixups: Vec<(usize, usize)>,
    absolute_fixups: Vec<(usize, usize)>,
}

impl<const LABELS: usize> Mcs51Assembler<LABELS> {
    fn new(origin: u16) -> Self {
        Self {
            origin,
            code: Vec::new(),
            labels: [None; LABELS],
            relative_fixups: Vec::new(),
            absolute_fixups: Vec::new(),
        }
    }

    fn bytes(&mut self, bytes: &[u8]) {
        self.code.extend_from_slice(bytes);
    }

    fn label(&mut self, label: usize) {
        assert!(self.labels[label].replace(self.code.len()).is_none());
    }

    fn rel(&mut self, opcode: u8, label: usize) {
        self.code.push(opcode);
        let displacement = self.code.len();
        self.code.push(0);
        self.relative_fixups.push((displacement, label));
    }

    fn bit_rel(&mut self, opcode: u8, bit: u8, label: usize) {
        self.code.extend_from_slice(&[opcode, bit]);
        let displacement = self.code.len();
        self.code.push(0);
        self.relative_fixups.push((displacement, label));
    }

    fn abs(&mut self, opcode: u8, label: usize) {
        self.code.push(opcode);
        let address = self.code.len();
        self.code.extend_from_slice(&[0, 0]);
        self.absolute_fixups.push((address, label));
    }

    fn read_xdata(&mut self, address: u16) {
        self.code
            .extend_from_slice(&[0x90, (address >> 8) as u8, address as u8, 0xE0]);
    }

    fn read_xdata_to_register(&mut self, address: u16, register: u8) {
        assert!(register < 8);
        self.read_xdata(address);
        self.code.push(0xF8 + register); // MOV Rn,A
    }

    fn write_accumulator_to_xdata(&mut self, address: u16) {
        self.code
            .extend_from_slice(&[0x90, (address >> 8) as u8, address as u8, 0xF0]);
    }

    fn set_xdata_bit_zero(&mut self, address: u16) {
        self.code.extend_from_slice(&[
            0x90,
            (address >> 8) as u8,
            address as u8,
            0xE0,
            0x54,
            0xFE,
            0x44,
            0x01,
            0xF0,
        ]);
    }

    fn clear_xdata_bit_zero(&mut self, address: u16) {
        self.code.extend_from_slice(&[
            0x90,
            (address >> 8) as u8,
            address as u8,
            0xE0,
            0x54,
            0xFE,
            0xF0,
        ]);
    }

    fn finish(mut self) -> Vec<u8> {
        for (displacement, label) in self.relative_fixups {
            let target = self.labels[label].expect("undefined MCS-51 label");
            let delta = target as isize - (displacement + 1) as isize;
            assert!((-128..=127).contains(&delta));
            self.code[displacement] = delta as i8 as u8;
        }
        for (address, label) in self.absolute_fixups {
            let target = self.labels[label].expect("undefined MCS-51 label");
            let target = self.origin.wrapping_add(target as u16);
            self.code[address] = (target >> 8) as u8;
            self.code[address + 1] = target as u8;
        }
        self.code
    }
}

/// Builds the minimum independently authored OFDM reset path that preserves the ROM boot ABI.
fn ofdm_startup_program() -> Vec<u8> {
    let mut code = Vec::new();

    // Reset ABI observed independently from control flow: clear internal RAM 0x01..0x7F and use
    // the same conservative stack position as the reference core startup.
    code.extend_from_slice(&[0x78, 0x7F, 0xE4, 0xF6, 0xD8, 0xFD]);
    code.extend_from_slice(&[0x75, 0x81, 0x26]); // MOV SP,#0x26

    for (offset, value) in OPEN_OFDM_VERSION.iter().copied().enumerate() {
        emit_write_xdata(
            &mut code,
            OPEN_OFDM_VERSION_XDATA_ADDRESS + offset as u16,
            value,
        );
    }

    // ROM owns the dispatch mechanism, but downloaded code owns this 16-pair table. Initialize
    // all slots to the unused 0xFFFF value before enabling the understood reset-time patches.
    emit_mov_dptr(&mut code, OFDM_CALLBACK_TABLE_ADDRESS);
    code.extend_from_slice(&[0x7F, 0x40, 0x74, 0xFF]); // MOV R7,#64; MOV A,#0xFF
    code.extend_from_slice(&[0xF0, 0xA3, 0xDF, 0xFC]); // MOVX; INC DPTR; DJNZ R7,loop

    // The two 16-word halves form an original-entry -> replacement-entry patch map. Slots whose
    // downloaded replacement has not been reimplemented must stay 0xFFFF so the immutable ROM
    // entry remains reachable. Earlier probes incorrectly mapped four such entries to a RET stub,
    // suppressing ROM services rather than providing a harmless fallback.
    let reconstructed = services::build();
    let callback_patches = [
        (0usize, 0x7353, reconstructed.entries[&0x4B1B]),
        (1usize, 0x30F6, reconstructed.entries[&0x4AF1]),
        (3usize, 0xDC0F, reconstructed.entries[&0x4954]),
        (2usize, 0x99F5, OPEN_OFDM_TIMER_CALLBACK_ENTRY),
        (4usize, 0x0C8D, 0x4123),
        (5usize, 0x9F97, OPEN_OFDM_SERIAL_CALLBACK_ENTRY),
    ];
    for (slot, rom_callback, replacement_callback) in callback_patches {
        emit_write_xdata_u16(
            &mut code,
            OFDM_CALLBACK_TABLE_ADDRESS + (slot * 2) as u16,
            rom_callback,
        );
        emit_write_xdata_u16(
            &mut code,
            OFDM_CALLBACK_TABLE_ADDRESS + 0x20 + (slot * 2) as u16,
            replacement_callback,
        );
    }

    // Preserve the serial/interrupt state required by the immutable OFDM ROM scheduler.
    code.extend_from_slice(&[0xC2, 0xAC, 0xC2, 0x8E]);
    code.extend_from_slice(&[0x43, 0x8E, 0x10]);
    code.extend_from_slice(&[0x43, 0x87, 0x80]);
    code.extend_from_slice(&[0x75, 0x98, 0x50]);
    code.extend_from_slice(&[0x53, 0x89, 0x0F]);
    code.extend_from_slice(&[0x43, 0x89, 0x20]);
    code.extend_from_slice(&[0x75, 0x8B, 0xBC, 0x75, 0x8D, 0xBC]);
    code.extend_from_slice(&[0xD2, 0x8E, 0xC2, 0x99, 0xD2, 0xAC]);

    emit_write_xdata(&mut code, OFDM_READY_XDATA_ADDRESS, 1);
    code.extend_from_slice(&[
        0x12,
        (OFDM_ROM_STARTUP_VENEER >> 8) as u8,
        OFDM_ROM_STARTUP_VENEER as u8,
    ]); // LCALL veneer -> immutable ROM startup -> RET

    // Publish success only after the ROM handoff returns, then return to the CMD_FW_BOOT caller.
    emit_write_xdata(
        &mut code,
        OPEN_PROBE_MARKER_ADDRESS as u16,
        OPEN_PROBE_MARKER_VALUE,
    );
    code.push(0x22); // RET
    code
}

/// Builds the link hook from named 8051 operations instead of an opaque firmware byte array.
fn link_startup_program() -> Vec<u8> {
    let mut code = Vec::new();

    // Preserve the two interrupt-enable bits changed while rebuilding the ROM callback table.
    code.extend_from_slice(&[0xE4, 0xA2, 0xAF, 0x33]); // CLR A; MOV C,EA; RLC A
    emit_mov_dptr(&mut code, 0x4800);
    code.push(0xF0); // MOVX @DPTR,A
    code.extend_from_slice(&[0xA3, 0xE4, 0xA2, 0xDD, 0x33, 0xF0]);

    // Publish a distinct open-firmware version through the immutable ROM's startup-copy service.
    // Calling convention observed at the ROM boundary: R6:R7 is XDATA destination, R4:R5 is code
    // source, and R3 is byte count.
    emit_mov_register_immediate(&mut code, 6, (OPEN_LINK_VERSION_XDATA_ADDRESS >> 8) as u8);
    emit_mov_register_immediate(&mut code, 7, OPEN_LINK_VERSION_XDATA_ADDRESS as u8);
    emit_mov_register_immediate(&mut code, 4, (OPEN_LINK_VERSION_CODE_ADDRESS >> 8) as u8);
    emit_mov_register_immediate(&mut code, 5, OPEN_LINK_VERSION_CODE_ADDRESS as u8);
    emit_mov_register_immediate(&mut code, 3, OPEN_LINK_VERSION.len() as u8);
    code.extend_from_slice(&[
        0x12,
        (LINK_ROM_COPY_ENTRY >> 8) as u8,
        LINK_ROM_COPY_ENTRY as u8,
    ]); // LCALL LINK_ROM_COPY_ENTRY

    // Initialize the ROM callback table to its unused value.
    emit_mov_dptr(&mut code, 0xF53B);
    code.extend_from_slice(&[0x7F, 0x40, 0x74, 0xFF]); // MOV R7,#64; MOV A,#0xFF
    code.extend_from_slice(&[0xF0, 0xA3, 0xDF, 0xFC]); // loop: MOVX; INC DPTR; DJNZ R7,loop

    code.extend_from_slice(&[0xC2, 0xDD, 0xC2, 0xAF]); // CLR secondary interrupt; CLR EA

    // Install one immutable-ROM callback and one downloaded callback. The table stores high byte
    // first: 0x2C58 at 0xF53B and OPEN_LINK_CALLBACK_ENTRY (0x4204) at 0xF55B.
    emit_write_xdata(&mut code, 0xF53B, 0x2C);
    emit_write_xdata(&mut code, 0xF53C, 0x58);
    emit_write_xdata(&mut code, 0xF55B, 0x42);
    emit_write_xdata(&mut code, 0xF55C, 0x04);
    emit_set_xdata_bit_zero(&mut code, 0xF53A);

    // Restore the saved interrupt state through ACC.0.
    emit_mov_dptr(&mut code, 0x4801);
    code.extend_from_slice(&[0xE0, 0xA2, 0xE0, 0x92, 0xDD]);
    emit_mov_dptr(&mut code, 0x4800);
    code.extend_from_slice(&[0xE0, 0xA2, 0xE0, 0x92, 0xAF]);

    // Enable the two link-ROM command-service gates.
    emit_set_xdata_bit_zero(&mut code, 0xE009);
    emit_set_xdata_bit_zero(&mut code, 0xE00A);
    code.push(0x22); // RET to the immutable ROM startup path

    code
}

fn emit_mov_dptr(code: &mut Vec<u8>, address: u16) {
    code.extend_from_slice(&[0x90, (address >> 8) as u8, address as u8]);
}

fn emit_mov_a_immediate(code: &mut Vec<u8>, value: u8) {
    code.extend_from_slice(&[0x74, value]);
}

fn emit_mov_register_immediate(code: &mut Vec<u8>, register: u8, value: u8) {
    debug_assert!(register < 8);
    code.extend_from_slice(&[0x78 + register, value]);
}

fn emit_write_xdata(code: &mut Vec<u8>, address: u16, value: u8) {
    emit_mov_dptr(code, address);
    emit_mov_a_immediate(code, value);
    code.push(0xF0); // MOVX @DPTR,A
}

fn emit_write_xdata_u16(code: &mut Vec<u8>, address: u16, value: u16) {
    emit_write_xdata(code, address, (value >> 8) as u8);
    emit_write_xdata(code, address + 1, value as u8);
}

fn emit_set_xdata_bit_zero(code: &mut Vec<u8>, address: u16) {
    emit_mov_dptr(code, address);
    code.extend_from_slice(&[0xE0, 0x44, 0x01, 0xF0]); // MOVX A,@DPTR; ORL A,#1; MOVX @DPTR,A
}

/// Encodes an absolute MCS-51 `LJMP` instruction.
const fn long_jump(address: u16) -> [u8; 3] {
    [0x02, (address >> 8) as u8, address as u8]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinated_probe_contains_both_resets_and_rom_veneer() {
        let image = execution_probe_image().unwrap();
        assert!(image.bytes().len() < 5863);
        assert!(image.segment_count() > 28);
        let regions: Vec<_> = image.regions().collect();
        let expected_link_reset = long_jump(LINK_ROM_RESET_ENTRY);
        let expected_ofdm_reset = long_jump(OPEN_PROBE_ENTRY);
        let expected_ofdm_veneer = long_jump(OFDM_ROM_STARTUP_ENTRY);
        assert!(regions.iter().any(|region| {
            region.core == FirmwareCore::Link
                && region.address == 0x4100
                && region.data == &expected_link_reset[..]
        }));
        assert!(regions.iter().any(|region| {
            region.core == FirmwareCore::Ofdm
                && region.address == 0x4100
                && region.data == &expected_ofdm_reset[..]
        }));
        let vector_table = ofdm_vector_table_program();
        let startup_offset = (OFDM_ROM_STARTUP_VENEER - OFDM_VECTOR_TABLE_ADDRESS) as usize;
        assert_eq!(
            &vector_table[startup_offset..startup_offset + 3],
            &expected_ofdm_veneer[..]
        );
    }

    #[test]
    fn ofdm_program_calls_rom_before_publishing_success() {
        let program = ofdm_startup_program();
        assert_eq!(program.len(), 235);
        assert!(
            OPEN_PROBE_ENTRY as usize + program.len() <= OPEN_OFDM_TIMER_CALLBACK_ENTRY as usize
        );
        let rom_call = [
            0x12,
            (OFDM_ROM_STARTUP_VENEER >> 8) as u8,
            OFDM_ROM_STARTUP_VENEER as u8,
        ];
        let marker = [
            0x90,
            (OPEN_PROBE_MARKER_ADDRESS >> 8) as u8,
            OPEN_PROBE_MARKER_ADDRESS as u8,
            0x74,
            OPEN_PROBE_MARKER_VALUE,
            0xF0,
        ];
        let rom_position = program
            .windows(rom_call.len())
            .position(|item| item == &rom_call[..])
            .unwrap();
        let marker_position = program
            .windows(marker.len())
            .position(|item| item == &marker[..])
            .unwrap();
        assert!(rom_position < marker_position);
        assert_eq!(program.last(), Some(&0x22));
    }

    #[test]
    fn all_six_reconstructed_rom_patches_are_registered() {
        let program = ofdm_startup_program();
        // Check complete callback-map writes, not an unrelated immediate byte
        // (the relocated serial entry now legitimately has low byte 0x30).
        for slot in 0u16..6 {
            for offset in [0, 1, 0x20, 0x21] {
                let address = OFDM_CALLBACK_TABLE_ADDRESS + slot * 2 + offset;
                let prefix = [0x90, (address >> 8) as u8, address as u8, 0x74];
                assert!(program.windows(4).any(|w| w == prefix));
            }
        }
    }

    #[test]
    fn ofdm_timer_dispatches_to_reconstructed_maintenance() {
        let services = services::build();
        assert_eq!(
            ofdm_timer_callback_program(),
            long_jump(services.entries[&0x4A1B])
        );
        assert!(services::ORIGIN as usize + services.code.len() <= 0x57CE);
        assert_eq!(services.entries.len(), 13);
        for address in services.entries.values() {
            assert!(*address >= services::ORIGIN);
            assert_ne!(services.code[(*address - services::ORIGIN) as usize], 0x22);
        }
    }

    #[test]
    fn ofdm_dispatch_table_has_all_128_veneers() {
        let table = ofdm_vector_table_program();
        assert_eq!(table.len(), 128 * 3);
        assert!(table.chunks_exact(3).all(|entry| entry[0] == 0x02));
        let startup_offset = (OFDM_ROM_STARTUP_VENEER - OFDM_VECTOR_TABLE_ADDRESS) as usize;
        assert_eq!(
            &table[startup_offset..startup_offset + 3],
            &long_jump(OFDM_ROM_STARTUP_ENTRY)[..]
        );
    }

    #[test]
    fn link_bridge_callback_fits_the_observed_download_window() {
        let callback = link_bridge_callback_program();
        assert_eq!(callback.len(), 268);
        assert!(OPEN_LINK_CALLBACK_ENTRY as usize + callback.len() <= 0x4335);
        assert_eq!(callback.first(), Some(&0xC2)); // CLR callback interrupt gate
        assert_eq!(callback.last(), Some(&0x22)); // RET
    }

    #[test]
    fn coordinated_probe_regions_do_not_overlap() {
        let image = execution_probe_image().unwrap();
        let regions: Vec<_> = image.regions().collect();
        for (index, left) in regions.iter().enumerate() {
            let left_start = left.address as usize;
            let left_end = left_start + left.data.len();
            for right in &regions[index + 1..] {
                if left.core != right.core {
                    continue;
                }
                let right_start = right.address as usize;
                let right_end = right_start + right.data.len();
                assert!(left_end <= right_start || right_end <= left_start);
            }
        }
    }
}
