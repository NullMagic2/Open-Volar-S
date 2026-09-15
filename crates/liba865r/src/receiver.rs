// SPDX-License-Identifier: GPL-3.0-only
// IT9175 host algorithms adapted from recfsusb2i (c) 2015-2016 trinity19683.
// Rust adaptation, board checks, bounded state transitions and recording: 2026.
//! Original-board UHF ISDB-T reception. See docs/RECEPTION.md for provenance and limits.

use crate::{
    protocol::Protocol, receiver_tables::*, CaptureReport, DeviceInfo, Error, Result, Transport,
    TsAnalyzer,
};
use std::{
    fs::OpenOptions,
    io::Write,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceiverState {
    Uninitialized,
    Ready,
    Tuned,
    Locked,
    Recording,
    Suspended,
    Faulted,
}

#[derive(Clone, Debug)]
pub struct Calibration {
    pub clock_mode: u8,
    pub xtal: u32,
    pub divider: u32,
    pub boundaries: [u32; 8],
    pub calibration_word: u16,
}

#[derive(Clone, Debug)]
pub struct TuneReport {
    pub frequency_khz: u32,
    pub channel_found: bool,
    pub mpeg_locked: bool,
    pub elapsed_ms: u128,
    pub channel_status: u8,
    pub mpeg_status: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignalReport {
    pub mpeg_locked: bool,
    pub quality_percent: Option<u8>,
}

/// One borrowed USB command owner, with a persistent sequence through init/tune/stream operations.
pub struct Receiver<'a> {
    protocol: Protocol<'a>,
    state: ReceiverState,
    calibration: Option<Calibration>,
    experimental: bool,
}

pub fn brazil_uhf_frequency(channel: u8) -> Result<u32> {
    if !(14..=51).contains(&channel) {
        return Err(Error::InvalidArgument(
            "Brazil UHF channel must be 14..51".into(),
        ));
    }
    Ok(473_143 + u32::from(channel - 14) * 6000)
}

use crate::channel_plan::validate_frequency;

fn oscillator_word(frequency: u32, cal: &Calibration) -> Result<(u16, u8, u8)> {
    validate_frequency(frequency)?;
    let index = cal
        .boundaries
        .iter()
        .position(|&upper| frequency <= upper)
        .unwrap_or(8);
    let n = [48u32, 32, 24, 16, 12, 8, 6, 4, 2][index];
    let value = (((u64::from(frequency) * 2 * u64::from(n) * u64::from(cal.divider))
        / u64::from(cal.xtal))
        + 1)
        >> 1;
    if index < 2 || index > 7 || value > 0x1fff {
        return Err(Error::Unsupported(
            "Calibration produced an unsupported UHF divider".into(),
        ));
    }
    let lna = [
        444000, 484000, 533000, 587000, 645000, 710000, 782000, 860000,
    ]
    .iter()
    .position(|&f| frequency <= f)
    .unwrap_or(7);
    Ok(((value as u16) | ((index as u16) << 13), n as u8, lna as u8))
}

impl<'a> Receiver<'a> {
    pub(crate) fn new(transport: &'a mut dyn Transport, info: &DeviceInfo) -> Result<Self> {
        if info.chip_type != 0x9175 || info.chip_version != 1 || info.prechip_version != 0x83 {
            return Err(Error::Unsupported(
                "Reception profile requires IT9175 9175:8301".into(),
            ));
        }
        if info.eeprom.as_ref().is_none_or(|e| {
            e.summary.tuner_id != 0x70 || e.summary.ts_mode != 0 || e.summary.dual_mode
        }) {
            return Err(Error::Unsupported(
                "Reception profile requires the identified single-tuner 0x70 USB board".into(),
            ));
        }
        if !info.firmware_running {
            return Err(Error::Unsupported(
                "Load the supplied reference firmware first".into(),
            ));
        }
        if !info
            .bulk_pipes
            .iter()
            .any(|p| p.id == 0x84 && p.input && p.maximum_packet_size == 512)
        {
            return Err(Error::Unsupported(
                "Required high-speed TS endpoint 0x84 is absent".into(),
            ));
        }
        Ok(Self {
            protocol: Protocol::new(transport),
            state: ReceiverState::Uninitialized,
            calibration: None,
            experimental: false,
        })
    }
    pub fn state(&self) -> ReceiverState {
        self.state
    }
    pub fn calibration(&self) -> Option<&Calibration> {
        self.calibration.as_ref()
    }
    fn r(&mut self, address: u32) -> Result<u8> {
        Ok(self.protocol.read_registers(address, 1)?[0])
    }
    fn w(&mut self, address: u32, value: u8) -> Result<()> {
        self.protocol.write_registers(address, &[value])
    }
    fn mask(&mut self, address: u32, value: u8, mask: u8) -> Result<()> {
        if mask == 0 || mask == 255 {
            return self.w(address, value);
        }
        let old = self.r(address)?;
        self.w(address, (old & !mask) | (value & mask))
    }
    fn table(&mut self, table: &[(u32, u8, u8)]) -> Result<()> {
        for &(r, v, m) in table {
            self.mask(r, v, m)?;
        }
        Ok(())
    }
    fn pause(ms: u64) {
        thread::sleep(Duration::from_millis(ms));
    }

    /// Applies the known-board profile, failing closed on unsupported clocks or stalled calibration.
    pub fn initialize(&mut self) -> Result<()> {
        self.initialize_profile(false)
    }

    /// Explicit development-only path. Boot health does not imply reception compatibility.
    pub fn initialize_open_firmware_experiment(&mut self) -> Result<()> {
        self.initialize_profile(true)
    }

    fn initialize_profile(&mut self, experimental: bool) -> Result<()> {
        if self.state != ReceiverState::Uninitialized {
            return Err(Error::InvalidArgument(
                "Receiver was already initialized or faulted; create a new session".into(),
            ));
        }
        self.state = ReceiverState::Faulted;
        self.experimental = experimental;
        let link = self.protocol.query_firmware_version(1000)?;
        let ofdm = self.protocol.query_processor_firmware_version(0x80, 1000)?;
        let supported_reference = (matches!(link, [3, 0, 3, 0] | [0, 1, 1, 0] | [0, 1, 2, 0])
            || link == crate::OPEN_LINK_VERSION)
            && ofdm == [3, 0, 4, 5];
        // 0.1.4.0 received RF22 and recovered through repeated standby/resume.
        // Keep version matching exact; future images need their own hardware validation.
        let supported_open = link == [0, 1, 4, 0] && ofdm == [0, 1, 4, 0];
        let supported_experiment = supported_open
            || (experimental
                && link == crate::OPEN_LINK_VERSION
                && ofdm == crate::OPEN_OFDM_VERSION);
        if !supported_reference && !supported_experiment {
            return Err(Error::Unsupported(format!("Unsupported reception firmware LINK {link:?}, OFDM {ofdm:?}; open firmware needs the explicit experimental path")));
        }
        if self.r(0xd800)? & 0xf != 0 {
            return Err(Error::Unsupported(
                "The A865R profile requires a 12 MHz device clock".into(),
            ));
        }
        // Wake only the demonstrated demodulator/tuner path, never command 0x23 on warm firmware.
        self.w(0x80ec40, 1)?;
        self.mask(0x80fb24, 0, 8)?;
        self.w(0x80fba8, 0)?;
        self.mask(0x80fbb9, 0, 0x20)?;
        self.w(0xe00c, 0)?;
        self.w(0x80f84f, 0)?;
        self.w(0x80f84f, 1)?;
        self.w(0xf103, 0x1a)?;
        self.w(0x4bfb, 0)?;
        self.w(0xcfff, 0)?;
        self.w(0xf641, 0x70)?;
        self.table(&[
            (0x80ec4c, 0xa8, 0),
            (0xd8c8, 1, 1),
            (0xd8c9, 1, 1),
            (0xd8bc, 1, 1),
            (0xd8bd, 1, 1),
            (0xe00c, 0, 0),
            (0xd864, 0, 0),
            (0x80f5ca, 1, 1),
            (0x80f715, 1, 1),
        ])?;
        self.protocol
            .write_registers(0x800025, &(12u32 << 19).to_le_bytes())?;
        let adc = ((20_250_000u64 << 19) / 1_000_000) as u32;
        self.protocol
            .write_registers(0x80f1cd, &adc.to_le_bytes()[..3])?;
        for &(address, values) in INITTAB_1.iter().chain(INITTAB_2) {
            self.protocol.write_registers(address, values)?;
        }
        self.w(0x80004e, 0)?;
        self.w(0x800000, 1)?;
        Self::pause(30);
        self.w(0xd827, 0)?;
        self.w(0xd829, 0)?;
        let clock_mode = self.r(0x80ec86)?;
        let (xtal, divider, iq) = match clock_mode {
            0 => (2000, 3, 16),
            1 => (20480, 18, 6),
            _ => {
                return Err(Error::Unsupported(format!(
                    "Unknown demodulator clock mode {clock_mode}"
                )))
            }
        };
        let count = self.r(0x80ed03)?;
        if count > 8 {
            return Err(Error::Protocol(format!(
                "Invalid calibration boundary count {count}"
            )));
        }
        Self::pause(10);
        let mut word = 0u16;
        for _ in 0..15 {
            let bytes = self.protocol.read_registers(0x80ed23, 2)?;
            word = u16::from_le_bytes([bytes[0], bytes[1]]);
            if word != 0 {
                break;
            }
            Self::pause(5);
        }
        if word == 0 {
            return Err(Error::Protocol(
                "Tuner calibration ED23 did not become ready".into(),
            ));
        }
        let mut boundaries = [
            78200, 117300, 156400, 234600, 312800, 469200, 625600, 950000,
        ];
        let scale = (u32::from(word) * 4 * xtal) / divider;
        for i in 0..usize::from(count) {
            boundaries[i] = (scale / [32, 24, 16, 12, 8, 6, 4, 2][i]) / 4;
        }
        if boundaries.windows(2).any(|p| p[0] >= p[1]) {
            return Err(Error::Protocol(
                "Calibration boundaries are not increasing".into(),
            ));
        }
        Self::pause(20);
        let mut ready = false;
        for _ in 0..10 {
            if self.r(0x80ec82)? != 0 {
                ready = true;
                break;
            }
            Self::pause(10);
        }
        if !ready {
            return Err(Error::Protocol(
                "Tuner calibration EC82 did not become ready".into(),
            ));
        }
        self.w(0x80ed81, iq)?;
        if self.r(0x8001dc)? != 1 {
            return Err(Error::Protocol(
                "Demodulator configuration 01DC is not 1".into(),
            ));
        }
        self.table(&[
            (0xd8d0, 1, 1),
            (0xd8d1, 1, 1),
            (0xf41f, 4, 4),
            (0x80f990, 0, 1),
            (0xf41a, 1, 1),
            (0x80f985, 0, 1),
            (0x80f986, 0, 1),
            (0xd91c, 0, 1),
            (0xd91b, 0, 1),
            (0x80cfff, 0, 3),
            (0xcfff, 0, 3),
            (0x80f99d, 1, 1),
            (0x80f9a4, 1, 1),
            (0xdd11, 0, 0x60),
            (0xdd13, 0, 0x60),
        ])?;
        self.protocol
            .write_registers(0xdd88, &(305u16 * 188 / 4).to_le_bytes())?;
        self.w(0xdd0c, 128)?;
        self.table(&[
            (0x80f985, 0, 1),
            (0x80f986, 0, 1),
            (0x80f9a3, 0, 1),
            (0x80f9cd, 0, 1),
            (0x80f9a4, 0, 1),
            (0xd8fd, 1, 0),
            (0xd833, 1, 0),
            (0xd830, 0, 0),
            (0x80ec57, 0, 0),
            (0x80ec58, 0, 0),
            (0x80ec40, 1, 0),
            (0xd8b8, 1, 0),
            (0xd8b9, 1, 0),
            (0xd831, 0, 0),
            (0xd832, 0, 0),
            (0x80f992, 1, 1),
            (0x80f993, 0, 1),
        ])?; // receive the full multiplex, including null packets
        self.calibration = Some(Calibration {
            clock_mode,
            xtal,
            divider,
            boundaries,
            calibration_word: word,
        });
        self.state = ReceiverState::Ready;
        Ok(())
    }

    fn stream(&mut self, on: bool) -> Result<()> {
        if on {
            self.mask(0xdd11, 0x20, 0x60)?;
            self.mask(0x80f99d, 0, 1)?;
        } else {
            self.mask(0x80f99d, 1, 1)?;
            self.mask(0xdd11, 0, 0x60)?;
        }
        Ok(())
    }

    /// Programs a UHF frequency in kHz, then waits for channel and MPEG synchronization.
    pub fn tune(&mut self, frequency: u32, timeout_ms: u32) -> Result<TuneReport> {
        validate_frequency(frequency)?;
        if !(200..=15000).contains(&timeout_ms) {
            return Err(Error::InvalidArgument(
                "Tune timeout must be 200..15000 ms".into(),
            ));
        }
        if !matches!(
            self.state,
            ReceiverState::Ready | ReceiverState::Tuned | ReceiverState::Locked
        ) {
            return Err(Error::InvalidArgument(
                "Initialize the receiver before tuning".into(),
            ));
        }
        let cal = self.calibration.clone().ok_or(Error::NotConnected)?;
        let (word, ndiv, lna) = oscillator_word(frequency, &cal)?;
        self.state = ReceiverState::Faulted;
        self.stream(false)?;
        self.mask(0x80cfff, 0, 3)?;
        self.protocol.write_registers(
            0x80011b,
            &[
                0x5e, 0x45, 0x5b, 0x7f, 0x56, 0x6a, 0x4d, 0xc5, 0x2c, 0xf9, 0x2b, 0xa7, 0x29, 0x3a,
                0x25, 0x1b, 0x17, 1, 0x16, 0x53, 0x15, 0x16, 0x12, 0xfa,
            ],
        )?;
        let mut coefficients = [
            0x0335edd7u32,
            0x019af6eb,
            0x00cd81e2,
            0x00cd7b76,
            0x00cd750a,
            0x0066bdbb,
            0x019af6eb,
            0x00cd7b76,
        ];
        if self.r(0x800045)? == 1 {
            for c in &mut coefficients {
                *c >>= 1;
            }
        }
        let mut packed: Vec<u8> = coefficients.iter().flat_map(|c| c.to_be_bytes()).collect();
        packed.extend_from_slice(&[0x7e, 2, 0x9b, 1]);
        self.protocol.write_registers(0x800001, &packed)?;
        self.w(0x800040, 0)?;
        self.w(0x800047, 0)?;
        self.mask(0x80f999, 0, 1)?;
        self.w(0x80004b, 1)?;
        if self.r(0x8001c6)? != 0 {
            // Same UHF selection as vendor table at file offset 0x1F978: clock = 80 - entry.
            let clocks = [
                64u16, 66, 66, 67, 67, 79, 80, 61, 62, 71, 62, 65, 66, 64, 67, 60, 68, 79, 80, 62,
                63, 64, 65, 66, 66, 66, 67, 60, 74, 61, 79, 64, 65, 65, 66, 60, 73, 60, 68, 62, 69,
                64, 80, 79, 79, 60, 60, 60, 63, 61,
            ];
            let index = (frequency.saturating_sub(470143) / 6000) as usize;
            let clock = clocks[index.min(clocks.len() - 1)] << 3;
            self.table(&[
                (0x80fb25, 0x65, 0),
                (0x80fbb5, ((clock >> 8) as u8) | 0x30, 0),
                (0x80fbb6, clock as u8, 0),
                (0x80fbb7, 4, 0),
                (0x80fbb9, 0x4a, 0),
                (0x80fbb9, 0x48, 0),
                (0x80fbba, 0x40, 0),
            ])?;
        }
        if self.r(0x8001dc)? != 1 {
            return Err(Error::Protocol(
                "Demodulator configuration changed before tune".into(),
            ));
        }
        let band = self.r(0x80ec4c)?;
        let iq = self.r(0x80ed81)?;
        let signed = (i32::from(iq & 31) - i32::from(iq & 32)) * i32::from(ndiv);
        let correction = if cal.clock_mode == 0 {
            (signed * 9) >> 5
        } else {
            signed >> 1
        };
        let corrected = word.wrapping_add(correction as u16);
        self.w(0x800160, lna)?;
        self.w(0x80ec56, 2)?;
        self.w(0x80ec4c, (band & 0xe7) | 8)?;
        self.protocol
            .write_registers(0x80ec4d, &corrected.to_le_bytes())?;
        self.protocol
            .write_registers(0x80015e, &word.to_le_bytes())?;
        self.mask(0xd8cf, 1, 1)?;
        self.w(0x8001e3, 0)?;
        self.protocol
            .write_registers(0x8001e1, &word.wrapping_sub(2 << 13).to_le_bytes())?;
        self.w(0x800000, 0)?;
        self.stream(true)?;
        let start = Instant::now();
        let deadline = Duration::from_millis(u64::from(timeout_ms));
        let mut previous = 0u8;
        let mut status = 0u8;
        let mut found = false;
        let mut mpeg = 0u8;
        while start.elapsed() < deadline {
            status = self.r(0x800047)?;
            mpeg = self.r(0x80f999)?;
            if status == 1 && previous == 1 {
                found = true;
                if mpeg & 1 != 0 {
                    break;
                }
            }
            if status == 2 && previous == 2 {
                break;
            }
            previous = status;
            Self::pause(40);
        }
        let locked = found && mpeg & 1 != 0;
        self.state = if locked {
            ReceiverState::Locked
        } else {
            ReceiverState::Tuned
        };
        Ok(TuneReport {
            frequency_khz: frequency,
            channel_found: found,
            mpeg_locked: locked,
            elapsed_ms: start.elapsed().as_millis(),
            channel_status: status,
            mpeg_status: mpeg,
        })
    }

    /// Records a locked multiplex to a new file; stop is checked at most every USB timeout.
    pub fn record(
        &mut self,
        path: &Path,
        seconds: u32,
        cancel: &AtomicBool,
    ) -> Result<CaptureReport> {
        let mut output = OpenOptions::new().write(true).create_new(true).open(path)?;
        let report = self.stream_chunks(seconds, cancel, |data| {
            output.write_all(data)?;
            Ok(())
        })?;
        output.flush()?;
        Ok(report)
    }

    /// Interactive recording continues until cancellation or an I/O error.
    pub fn record_until_stopped(&mut self, path:&Path, cancel:&AtomicBool)->Result<CaptureReport> {
        let mut output=OpenOptions::new().write(true).create_new(true).open(path)?;
        let report=self.stream_chunks_impl(None,cancel,|data|{output.write_all(data)?;Ok(())},None)?;
        output.flush()?;
        Ok(report)
    }

    /// Streams a locked multiplex to a caller-owned sink. The sink must return promptly.
    pub fn stream_chunks(
        &mut self,
        seconds: u32,
        cancel: &AtomicBool,
        sink: impl FnMut(&[u8]) -> Result<()>,
    ) -> Result<CaptureReport> {
        self.stream_chunks_impl(Some(seconds), cancel, sink, None)
    }

    /// Refreshes measured signal information every 500 ms on the USB owner thread.
    /// Monitoring errors are reported separately and do not discard received video.
    pub fn stream_chunks_monitored(
        &mut self,
        seconds: u32,
        cancel: &AtomicBool,
        sink: impl FnMut(&[u8]) -> Result<()>,
        mut signal: impl FnMut(Result<SignalReport>),
    ) -> Result<CaptureReport> {
        self.stream_chunks_impl(Some(seconds), cancel, sink, Some(&mut signal))
    }

    fn stream_chunks_impl(
        &mut self,
        seconds: Option<u32>,
        cancel: &AtomicBool,
        mut sink: impl FnMut(&[u8]) -> Result<()>,
        mut signal: Option<&mut dyn FnMut(Result<SignalReport>)>,
    ) -> Result<CaptureReport> {
        if self.state != ReceiverState::Locked {
            return Err(Error::InvalidArgument(
                "No MPEG lock; tune before receiving".into(),
            ));
        }
        if seconds.is_some_and(|s| !(1..=86400).contains(&s)) {
            return Err(Error::InvalidArgument(
                "Session duration must be 1..86400 seconds".into(),
            ));
        }
        self.state = ReceiverState::Recording;
        let start = Instant::now();
        let mut analyzer = TsAnalyzer::new();
        let mut bytes = 0;
        let mut next_signal = Instant::now();
        let result = (|| -> Result<()> {
            while seconds.is_none_or(|s| start.elapsed() < Duration::from_secs(u64::from(s)))
                && !cancel.load(Ordering::Relaxed)
            {
                if Instant::now() >= next_signal {
                    if let Some(observer) = signal.as_mut() {
                        observer(self.signal_status());
                    }
                    next_signal = Instant::now() + Duration::from_millis(500);
                }
                let data = self.protocol.read_stream(0x84, 188 * 305, 500)?;
                if data.is_empty() {
                    continue;
                }
                sink(&data)?;
                bytes += data.len() as u64;
                analyzer.push(&data);
            }
            Ok(())
        })();
        self.state = if result.is_ok() {
            ReceiverState::Locked
        } else {
            ReceiverState::Faulted
        };
        result?;
        if bytes == 0 && !cancel.load(Ordering::Relaxed) {
            return Err(Error::Protocol(
                "MPEG lock was reported, but no TS bytes arrived".into(),
            ));
        }
        Ok(CaptureReport {
            bytes_written: bytes,
            elapsed: start.elapsed(),
            stats: analyzer.finish(),
        })
    }
    /// Stops stream submission in hardware without resetting or suspending either firmware core.
    pub fn stop(&mut self) -> Result<()> {
        if matches!(
            self.state,
            ReceiverState::Suspended | ReceiverState::Uninitialized
        ) {
            return Ok(());
        }
        let faulted = self.state == ReceiverState::Faulted;
        let result = self.stream(false);
        self.state = if result.is_ok() && !faulted {
            ReceiverState::Ready
        } else {
            ReceiverState::Faulted
        };
        result
    }

    /// Explicit RF/demodulator standby, adapted from recfsusb2i it9175_sleep.
    /// Keeps the LINK command processor and RAM firmware alive. This is not USB D3.
    pub fn suspend(&mut self) -> Result<()> {
        if self.state == ReceiverState::Suspended {
            return Ok(());
        }
        if !matches!(
            self.state,
            ReceiverState::Ready | ReceiverState::Tuned | ReceiverState::Locked
        ) {
            return Err(Error::InvalidArgument(
                "Suspend requires an initialized, idle receiver".into(),
            ));
        }
        self.state = ReceiverState::Faulted;
        self.stream(false)?;
        self.mask(0x80fbb9, 0, 0x20)?;
        self.w(0xe00c, 1)?;
        self.mask(0x80fbb9, 0x20, 0x20)?;
        self.w(0x80004c, 1)?;
        self.w(0x800000, 0)?;
        Self::pause(30);
        let mut acknowledged = false;
        for _ in 0..20 {
            if self.r(0x80004c)? == 0 {
                acknowledged = true;
                break;
            }
            Self::pause(25);
        }
        if !acknowledged {
            return Err(Error::Protocol(
                "Demodulator did not acknowledge suspend; tuner power was left on".into(),
            ));
        }
        self.w(0x80fb24, 8)?;
        self.w(0x80fba8, 0)?;
        self.w(0x80ec40, 0)?;
        let mut power = [0u8; 15];
        power[1] = 12;
        self.protocol.write_registers(0x80ec02, &power)?;
        self.protocol.write_registers(0x80ec12, &[0; 4])?;
        self.protocol.write_registers(0x80ec17, &[0; 9])?;
        self.protocol.write_registers(0x80ec22, &[0; 10])?;
        self.w(0x80ec20, 0)?;
        self.w(0x80ec3f, 1)?;
        self.calibration = None;
        self.state = ReceiverState::Suspended;
        Ok(())
    }

    /// Reinitializes and recalibrates after explicit standby. Caller must tune again.
    pub fn resume(&mut self) -> Result<()> {
        if self.state != ReceiverState::Suspended {
            return Err(Error::InvalidArgument(
                "Resume requires a suspended receiver".into(),
            ));
        }
        self.state = ReceiverState::Uninitialized;
        self.initialize_profile(self.experimental)
    }

    /// Public reference ABI quality output (recfsusb2i readStatistic, 0x800049).
    /// Internal 0x80446B is a working accumulator, not the host quality register.
    /// This is a relative firmware percentage, not an independently calibrated RF level.
    pub fn signal_quality(&mut self) -> Result<Option<u8>> {
        Ok(self.signal_status()?.quality_percent)
    }

    pub fn signal_status(&mut self) -> Result<SignalReport> {
        let unlocked = SignalReport {
            mpeg_locked: false,
            quality_percent: None,
        };
        if !matches!(self.state, ReceiverState::Locked | ReceiverState::Recording) {
            return Ok(unlocked);
        }
        if self.r(0x80f999)? & 1 == 0 {
            return Ok(unlocked);
        }
        let quality = self.r(0x800049)?;
        if self.r(0x80f999)? & 1 == 0 {
            return Ok(unlocked);
        }
        Ok(SignalReport {
            mpeg_locked: true,
            quality_percent: (quality <= 100).then_some(quality),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::BulkPipe;
    use std::collections::{HashMap, VecDeque};

    #[derive(Default)]
    struct Registers {
        values: HashMap<u32, u8>,
        reads: HashMap<u32, VecDeque<u8>>,
        writes: Vec<(u32, u8)>,
    }
    impl Transport for Registers {
        fn open(&mut self, _: u16, _: u16) -> Result<()> {
            Ok(())
        }
        fn cycle_port(&mut self) -> Result<()> {
            unreachable!()
        }
        fn bulk_pipes(&self) -> &[BulkPipe] {
            &[]
        }
        fn set_command_pipes(&mut self, _: u8, _: u8) -> Result<()> {
            Ok(())
        }
        fn description(&self) -> String {
            "register model".into()
        }
        fn read_bulk(&mut self, _: u8, _: usize, _: u32) -> Result<Vec<u8>> {
            unreachable!()
        }
        fn exchange(&mut self, request: &[u8], _: usize, _: u32) -> Result<Vec<u8>> {
            let address = (u32::from(request[1]) << 16)
                | (u32::from(request[8]) << 8)
                | u32::from(request[9]);
            let count = usize::from(request[4]);
            let mut payload = Vec::new();
            match request[2] {
                0 => {
                    for i in 0..count {
                        let a = address + i as u32;
                        payload.push(
                            self.reads
                                .get_mut(&a)
                                .and_then(|q| q.pop_front())
                                .unwrap_or(*self.values.get(&a).unwrap_or(&0)),
                        );
                    }
                }
                1 => {
                    for (i, value) in request[10..10 + count].iter().copied().enumerate() {
                        let a = address + i as u32;
                        self.values.insert(a, value);
                        self.writes.push((a, value));
                    }
                }
                _ => return Err(Error::Transport("Unexpected command in power test".into())),
            }
            let mut response = vec![(payload.len() + 4) as u8, request[3], 0];
            response.extend(payload);
            response.extend(Protocol::checksum(&response)?.to_be_bytes());
            Ok(response)
        }
    }
    fn model_receiver(transport: &mut Registers, state: ReceiverState) -> Receiver<'_> {
        Receiver {
            protocol: Protocol::new(transport),
            state,
            calibration: None,
            experimental: false,
        }
    }
    #[test]
    fn suspend_waits_for_ack_and_never_powers_down_on_timeout() {
        for ack in [false, true] {
            let mut registers = Registers::default();
            if ack {
                registers.reads.insert(0x80004c, [1, 0].into());
            }
            let mut receiver = model_receiver(&mut registers, ReceiverState::Ready);
            assert_eq!(receiver.suspend().is_ok(), ack);
            assert_eq!(
                receiver.state(),
                if ack {
                    ReceiverState::Suspended
                } else {
                    ReceiverState::Faulted
                }
            );
            if ack {
                receiver.stop().unwrap();
                assert_eq!(receiver.state(), ReceiverState::Suspended);
                assert!(receiver.tune(521143, 6000).is_err());
            }
            assert_eq!(registers.writes.contains(&(0x80ec40, 0)), ack);
            assert_eq!(registers.writes.contains(&(0x80ec3f, 1)), ack);
        }
    }
    #[test]
    fn quality_uses_public_register_and_rechecks_lock() {
        for (value, final_lock, expected) in [(73, 1, Some(73)), (255, 1, None), (73, 0, None)] {
            let mut registers = Registers::default();
            registers.values.insert(0x800049, value);
            registers.values.insert(0x80446b, 99);
            registers.reads.insert(0x80f999, [1, final_lock].into());
            let mut receiver = model_receiver(&mut registers, ReceiverState::Locked);
            assert_eq!(receiver.signal_quality().unwrap(), expected);
        }
        let mut registers = Registers::default();
        registers.values.insert(0x800049, 255);
        registers.values.insert(0x80f999, 1);
        let mut receiver = model_receiver(&mut registers, ReceiverState::Recording);
        assert_eq!(
            receiver.signal_status().unwrap(),
            SignalReport {
                mpeg_locked: true,
                quality_percent: None
            }
        );
    }
    #[test]
    fn brazil_channels_and_bounds() {
        assert_eq!(brazil_uhf_frequency(14).unwrap(), 473143);
        assert_eq!(brazil_uhf_frequency(22).unwrap(), 521143);
        assert_eq!(brazil_uhf_frequency(51).unwrap(), 695143);
        assert!(brazil_uhf_frequency(13).is_err());
        assert!(brazil_uhf_frequency(52).is_err());
    }
    #[test]
    fn oscillator_matches_integer_reference() {
        let c = Calibration {
            clock_mode: 1,
            xtal: 20480,
            divider: 18,
            boundaries: [
                78200, 117300, 156400, 234600, 312800, 469200, 625600, 950000,
            ],
            calibration_word: 1,
        };
        let (word, n, lna) = oscillator_word(521143, &c).unwrap();
        assert_eq!(n, 6);
        assert_eq!(lna, 2);
        assert_eq!(word, 0xcabc);
        assert!(oscillator_word(0, &c).is_err());
    }
}
