//! Playback-only PCR recovery for a service that advertises a clock PID but omits it.
//! Payload bytes and PES timestamps stay untouched. Recorders use the raw receiver.
const PACKET: usize = 188;
const MASK: u64 = (1 << 33) - 1;
const GRACE: u64 = 90_000; // Observe one second of this service's timestamps first.
const LEAD: u64 = 27_000; // 300 ms of decode lead, shared by audio/video via PCR.
#[derive(Clone, Copy, Debug)]
pub struct ClockRecoveryConfig {
    pub video_pid: u16,
    pub pcr_pid: u16,
}
impl ClockRecoveryConfig {
    pub fn new(video_pid: u16, pcr_pid: u16) -> Option<Self> {
        ((32..8191).contains(&video_pid) && (32..8191).contains(&pcr_pid))
            .then_some(Self { video_pid, pcr_pid })
    }
}
pub struct ClockRecovery {
    config: ClockRecoveryConfig,
    pending: Vec<u8>,
    real_clock: bool,
    first: Option<u64>,
    last: Option<u64>,
    recovering: bool,
    pub generated: u64,
}
impl ClockRecovery {
    pub fn new(config: ClockRecoveryConfig) -> Self {
        Self {
            config,
            pending: Vec::new(),
            real_clock: false,
            first: None,
            last: None,
            recovering: false,
            generated: 0,
        }
    }
    /// Accept arbitrary USB/file chunk boundaries. Forward original bytes, including
    /// acquisition noise; only insert clock packets at confirmed TS boundaries.
    pub fn push(&mut self, bytes: &[u8]) -> Vec<u8> {
        self.pending.extend_from_slice(bytes);
        let mut out = Vec::with_capacity(self.pending.len() + PACKET * 4);
        let mut at = 0;
        while self.pending.len() - at >= PACKET * 3 {
            if self.pending[at] != 0x47
                || self.pending[at + PACKET] != 0x47
                || self.pending[at + PACKET * 2] != 0x47
            {
                out.push(self.pending[at]);
                at += 1;
                continue;
            }
            let mut packet: [u8; PACKET] = self.pending[at..at + PACKET].try_into().unwrap();
            if let Some(clock) = self.inspect(&mut packet) {
                out.extend_from_slice(&clock);
            }
            out.extend_from_slice(&packet);
            at += PACKET;
        }
        self.pending.drain(..at);
        out
    }
    /// Retain the final short tail verbatim at EOF; live input has no such tail.
    pub fn finish(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.pending)
    }
    fn inspect(&mut self, p: &mut [u8; PACKET]) -> Option<[u8; PACKET]> {
        if p[1] & 0x80 != 0 || p[3] & 0xc0 != 0 {
            return None;
        }
        let pid = ((p[1] as u16 & 31) << 8) | p[2] as u16;
        let adaptation = (p[3] >> 4) & 3;
        if adaptation == 0 || (adaptation & 2 != 0 && p[4] as usize + 5 > PACKET) {
            return None;
        }
        let has_pcr = adaptation & 2 != 0
            && p[4] >= 7
            && p[5] & 0x10 != 0
            && p[10] & 0x7e == 0x7e
            && (((p[10] & 1) as u16) << 8 | p[11] as u16) < 300;
        if pid == self.config.pcr_pid && has_pcr {
            // Return immediately to a real clock. Mark the change of clock origin
            // so the Windows demux can translate it without a long timestamp jump.
            if self.recovering {
                p[5] |= 0x80;
            }
            self.real_clock = true;
            self.recovering = false;
            return None;
        }
        if self.real_clock
            || pid != self.config.video_pid
            || p[1] & 0x40 == 0
            || adaptation & 1 == 0
        {
            return None;
        }
        let offset = if adaptation & 2 != 0 {
            5 + p[4] as usize
        } else {
            4
        };
        let pes = p.get(offset..)?;
        if pes.len() < 14
            || pes[..3] != [0, 0, 1]
            || !(0xe0..=0xef).contains(&pes[3])
            || pes[6] & 0xc0 != 0x80
        {
            return None;
        }
        let flags = pes[7] >> 6;
        // Prefer decode timestamps for reordered video. Never infer clocks from
        // arbitrary payload bytes or corrupt marker/header fields.
        let timestamp = match flags {
            2 if pes[8] >= 5 => read_stamp(pes.get(9..14)?, 2),
            3 if pes[8] >= 10 => read_stamp(pes.get(14..19)?, 1),
            _ => None,
        }?;
        if let Some(last) = self.last {
            let delta = timestamp.wrapping_sub(last) & MASK;
            if delta == 0 || delta > MASK / 2 {
                return None;
            } // Reordered PTS or duplicate.
            if delta > 900_000 {
                self.first = None;
                self.recovering = false;
            }
        }
        self.last = Some(timestamp);
        let first = *self.first.get_or_insert(timestamp);
        if timestamp.wrapping_sub(first) & MASK < GRACE {
            return None;
        }
        let base = timestamp.wrapping_sub(LEAD) & MASK;
        let mut clock = [0xff; PACKET];
        clock[..6].copy_from_slice(&[
            0x47,
            (self.config.pcr_pid >> 8) as u8,
            self.config.pcr_pid as u8,
            0x20,
            183,
            if self.recovering { 0x10 } else { 0x90 },
        ]);
        clock[6..12].copy_from_slice(&[
            (base >> 25) as u8,
            (base >> 17) as u8,
            (base >> 9) as u8,
            (base >> 1) as u8,
            (((base & 1) as u8) << 7) | 0x7e,
            0,
        ]);
        self.recovering = true;
        self.generated += 1;
        Some(clock)
    }
}
fn read_stamp(p: &[u8], prefix: u8) -> Option<u64> {
    if p.len() != 5 || p[0] >> 4 != prefix || p[0] & 1 == 0 || p[2] & 1 == 0 || p[4] & 1 == 0 {
        return None;
    }
    Some(
        ((p[0] as u64 & 14) << 29)
            | ((p[1] as u64) << 22)
            | ((p[2] as u64 & 254) << 14)
            | ((p[3] as u64) << 7)
            | (p[4] as u64 >> 1),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    fn video(time: u64) -> [u8; 188] {
        let mut p = [0xff; 188];
        p[..13].copy_from_slice(&[0x47, 0x50, 1, 0x10, 0, 0, 1, 0xe0, 0, 0, 0x84, 0x80, 5]);
        p[13..18].copy_from_slice(&[
            0x21 | ((((time >> 30) & 7) as u8) << 1),
            (time >> 22) as u8,
            ((time >> 14) as u8 & 0xfe) | 1,
            (time >> 7) as u8,
            ((time as u8) << 1) | 1,
        ]);
        p
    }
    fn recovery() -> ClockRecovery {
        ClockRecovery::new(ClockRecoveryConfig::new(4097, 4098).unwrap())
    }
    #[test]
    fn missing_clock_recovers_only_after_grace_and_wraps_33_bits() {
        let mut r = recovery();
        assert!(r.inspect(&mut video(MASK - 45_000)).is_none());
        assert!(r.inspect(&mut video(44_998)).is_none());
        let p = r.inspect(&mut video(45_000)).unwrap();
        assert_eq!(&p[..6], &[0x47, 0x10, 2, 0x20, 183, 0x90]);
        let base = ((p[6] as u64) << 25)
            | ((p[7] as u64) << 17)
            | ((p[8] as u64) << 9)
            | ((p[9] as u64) << 1)
            | (p[10] as u64 >> 7);
        assert_eq!(base, 18_000);
    }
    #[test]
    fn real_clock_and_chunk_boundaries_preserve_original_bytes() {
        let mut producer = recovery();
        producer.inspect(&mut video(0));
        let clock = producer.inspect(&mut video(90_000)).unwrap();
        let mut data = vec![3, 7, 99];
        data.extend_from_slice(&clock);
        for i in 0..80 {
            data.extend_from_slice(&video(i * 6_000));
        }
        for chunk_size in [1, 187, 188, 301, 4096] {
            let mut r = recovery();
            let mut result = Vec::new();
            for chunk in data.chunks(chunk_size) {
                result.extend(r.push(chunk));
            }
            result.extend(r.finish());
            assert_eq!(result, data);
            assert_eq!(r.generated, 0);
        }
    }
    #[test]
    fn recovery_hands_back_to_real_clock_and_rejects_bad_or_reordered_stamps() {
        let mut r = recovery();
        r.inspect(&mut video(0));
        let mut real = r.inspect(&mut video(90_000)).unwrap();
        assert!(r.inspect(&mut video(89_000)).is_none());
        let mut bad = video(100_000);
        bad[17] &= !1;
        assert!(r.inspect(&mut bad).is_none());
        real[5] = 0x10;
        assert!(r.inspect(&mut real).is_none());
        assert_ne!(real[5] & 0x80, 0);
        assert!(r.inspect(&mut video(180_000)).is_none());
        assert_eq!(r.generated, 1);
    }
    #[test]
    fn selected_service_and_dts_take_priority() {
        let mut r = recovery();
        let mut other = video(0);
        other[2] = 3;
        assert!(r.inspect(&mut other).is_none());
        assert!(r.first.is_none());
        let mut p = video(90_000);
        p[11] = 0xc0;
        p[12] = 10;
        p[13] = (p[13] & 15) | 0x30;
        let mut dts = video(0);
        dts[13] = (dts[13] & 15) | 0x10;
        p[18..23].copy_from_slice(&dts[13..18]);
        assert!(r.inspect(&mut p).is_none());
        assert_eq!(r.first, Some(0));
        let mut tei = video(200_000);
        tei[1] |= 0x80;
        assert!(r.inspect(&mut tei).is_none());
    }
}
