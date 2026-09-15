//! Summary: Provides streaming MPEG-2 transport-stream synchronization, continuity, PAT, and PMT diagnostics.

use std::collections::{BTreeMap, BTreeSet};

const TS_PACKET_SIZE: usize = 188;

/// Describes one elementary stream announced by a parsed PMT section.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct StreamInfo {
    pub program_number: u16,
    pub pid: u16,
    pub stream_type: u8,
}

/// Describes one program announced by PAT and optionally enriched by PMT.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramInfo {
    pub program_number: u16,
    pub pmt_pid: u16,
    pub pcr_pid: Option<u16>,
}

/// Aggregate diagnostics collected while parsing MPEG-TS packets.
#[derive(Clone, Debug, Default)]
pub struct TsStats {
    pub bytes_seen: u64,
    pub packets: u64,
    pub sync_losses: u64,
    pub continuity_errors: u64,
    pub transport_error_packets: u64,
    pub psi_crc_errors: u64,
    pub pid_packets: BTreeMap<u16, u64>,
    pub programs: BTreeMap<u16, ProgramInfo>,
    pub streams: BTreeSet<StreamInfo>,
    pub audio_languages: BTreeMap<u16, String>,
    pub caption_profiles: BTreeMap<u16, u16>,
    pub service_names: BTreeMap<u16, String>,
    pub channel_numbers: BTreeMap<u16, u8>,
    pub events: crate::epg::Events,
}

/// Incrementally analyzes arbitrary USB chunks while preserving packet alignment across reads.
pub struct TsAnalyzer {
    pending: Vec<u8>,
    stats: TsStats,
    continuity: BTreeMap<u16, u8>,
    sections: BTreeMap<u16, SectionAssembler>,
    last_packet: BTreeMap<u16, Vec<u8>>,
}

impl TsAnalyzer {
    /// Creates an empty analyzer with no assumed packet alignment.
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
            stats: TsStats::default(),
            continuity: BTreeMap::new(),
            sections: BTreeMap::new(),
            last_packet: BTreeMap::new(),
        }
    }

    /// Adds one arbitrary byte chunk and processes every complete 188-byte packet it contains.
    pub fn push(&mut self, data: &[u8]) {
        self.stats.bytes_seen += data.len() as u64;
        self.pending.extend_from_slice(data);

        loop {
            if self.pending.len() < TS_PACKET_SIZE {
                break;
            }
            if self.pending[0] != 0x47 {
                if let Some(offset) = find_sync_offset(&self.pending) {
                    if offset > 0 {
                        self.stats.sync_losses += 1;
                        self.pending.drain(..offset);
                    }
                } else {
                    let keep = self.pending.len().min(TS_PACKET_SIZE * 3);
                    let discard = self.pending.len() - keep;
                    if discard > 0 {
                        self.stats.sync_losses += 1;
                        self.pending.drain(..discard);
                    }
                    break;
                }
            }
            if self.pending.len() < TS_PACKET_SIZE || self.pending[0] != 0x47 {
                continue;
            }
            let packet: Vec<u8> = self.pending.drain(..TS_PACKET_SIZE).collect();
            self.process_packet(&packet);
        }
    }

    /// Returns the current immutable statistics snapshot.
    pub fn stats(&self) -> &TsStats {
        &self.stats
    }

    /// Consumes the analyzer and returns its final statistics.
    pub fn finish(self) -> TsStats {
        self.stats
    }

    /// Parses one aligned transport packet and updates continuity and PSI information.
    fn process_packet(&mut self, packet: &[u8]) {
        if packet.len() != TS_PACKET_SIZE || packet[0] != 0x47 {
            self.stats.sync_losses += 1;
            return;
        }
        self.stats.packets += 1;

        let transport_error = (packet[1] & 0x80) != 0;
        if transport_error {
            self.stats.transport_error_packets += 1;
        }
        let payload_unit_start = (packet[1] & 0x40) != 0;
        let pid = (((packet[1] & 0x1F) as u16) << 8) | packet[2] as u16;
        *self.stats.pid_packets.entry(pid).or_default() += 1;

        let adaptation_control = (packet[3] >> 4) & 0x03;
        let continuity_counter = packet[3] & 0x0F;
        let has_payload = adaptation_control == 1 || adaptation_control == 3;
        if transport_error || packet[3] & 0xc0 != 0 {
            self.sections.remove(&pid);
            self.continuity.remove(&pid);
            return;
        }
        if adaptation_control >= 2 && packet[4] > 0 && packet[5] & 0x80 != 0 {
            self.continuity.remove(&pid);
            self.sections.remove(&pid);
            self.last_packet.remove(&pid);
        }
        if has_payload && pid != 0x1FFF {
            if self
                .last_packet
                .get(&pid)
                .is_some_and(|last| last == packet)
            {
                return;
            }
            self.last_packet.insert(pid, packet.to_vec());
            if let Some(previous) = self.continuity.insert(pid, continuity_counter) {
                let expected = (previous + 1) & 0x0F;
                if continuity_counter != expected {
                    self.stats.continuity_errors += 1;
                    self.sections.remove(&pid);
                }
            }
        }

        let Some(payload) = packet_payload(packet, adaptation_control) else {
            return;
        };
        if pid == 0
            || pid == 0x10
            || pid == 0x11
            || pid == 0x12
            || self
                .stats
                .programs
                .values()
                .any(|program| program.pmt_pid == pid)
        {
            let sections = self
                .sections
                .entry(pid)
                .or_default()
                .push(payload, payload_unit_start);
            for section in sections {
                if crc32_mpeg(&section) != 0 {
                    self.stats.psi_crc_errors += 1;
                    continue;
                }
                if section.len() < 8 || section[5] & 1 == 0 {
                    continue;
                }
                if pid == 0 {
                    parse_pat(&section, &mut self.stats);
                } else if pid == 0x10 {
                    parse_nit(&section, &mut self.stats);
                } else if pid == 0x11 {
                    parse_sdt(&section, &mut self.stats);
                } else if pid == 0x12 {
                    crate::epg::parse(&section, &mut self.stats.events);
                } else {
                    parse_pmt(&section, pid, &mut self.stats);
                }
            }
        }
    }
}

impl Default for TsAnalyzer {
    /// Creates the same empty analyzer as `TsAnalyzer::new`.
    fn default() -> Self {
        Self::new()
    }
}

fn parse_sdt(section: &[u8], stats: &mut TsStats) {
    if section.len() < 15 || section[0] != 0x42 {
        return;
    }
    let end = section.len() - 4;
    let mut at = 11;
    while at + 5 <= end {
        let id = u16::from_be_bytes([section[at], section[at + 1]]);
        let len = (((section[at + 3] & 15) as usize) << 8) | section[at + 4] as usize;
        at += 5;
        let Some(last) = at.checked_add(len).filter(|last| *last <= end) else {
            return;
        };
        let mut d = at;
        while d + 2 <= last {
            let tag = section[d];
            let n = section[d + 1] as usize;
            d += 2;
            if d + n > last {
                return;
            }
            if tag == 0x48 && n >= 3 {
                let data = &section[d..d + n];
                let provider = data[1] as usize;
                if 2 + provider < data.len() {
                    let name_len = data[2 + provider] as usize;
                    let start = 3 + provider;
                    if start + name_len <= data.len() {
                        let raw = &data[start..start + name_len];
                        // Brazilian SDT commonly uses the Latin alphabet. Control/escape sequences
                        // are not displayed. Full ARIB gaiji/character-set support is future work.
                        let mut clean = Vec::new();
                        let mut i = 0;
                        while i < raw.len() {
                            if raw[i] == 0x1b {
                                i += 1;
                                while i < raw.len() && (0x20..=0x2f).contains(&raw[i]) {
                                    i += 1;
                                }
                                i += 1;
                                continue;
                            }
                            if raw[i] >= 0x20 && raw[i] != 0x7f {
                                clean.push(raw[i]);
                            }
                            i += 1;
                        }
                        let name = String::from_utf8(clean.clone())
                            .unwrap_or_else(|_| clean.iter().map(|b| char::from(*b)).collect());
                        if !name.trim().is_empty() {
                            stats.service_names.insert(id, name.trim().to_owned());
                        }
                    }
                }
            }
            d += n;
        }
        at = last;
    }
}

/// ISDB TS information descriptor: announced remote-control channel, never the RF index.
fn parse_nit(s: &[u8], stats: &mut TsStats) {
    if s.len() < 16 || s[0] != 0x40 {
        return;
    }
    let end = s.len() - 4;
    let mut p = 10 + (((s[8] & 15) as usize) << 8) + s[9] as usize;
    if p + 2 > end {
        return;
    }
    let last = p + 2 + (((s[p] & 15) as usize) << 8) + s[p + 1] as usize;
    p += 2;
    if last > end {
        return;
    }
    while p + 6 <= last {
        let stop = p + 6 + (((s[p + 4] & 15) as usize) << 8) + s[p + 5] as usize;
        p += 6;
        if stop > last {
            return;
        }
        while p + 2 <= stop {
            let tag = s[p];
            let len = s[p + 1] as usize;
            p += 2;
            if p + len > stop {
                return;
            }
            let d = &s[p..p + len];
            if tag == 0xcd && len >= 2 && d[0] > 0 {
                let mut at = 2 + (d[1] >> 2) as usize;
                for _ in 0..(d[1] & 3) {
                    if at + 2 > len {
                        break;
                    }
                    let count = d[at + 1] as usize;
                    at += 2;
                    if at + count * 2 > len {
                        break;
                    }
                    for _ in 0..count {
                        let service = u16::from_be_bytes([d[at], d[at + 1]]);
                        at += 2;
                        stats.channel_numbers.insert(service, d[0]);
                    }
                }
            }
            p += len;
        }
        p = stop;
    }
}

/// Finds a plausible sync byte that repeats at 188-byte intervals.
fn find_sync_offset(data: &[u8]) -> Option<usize> {
    // A USB capture can start with more than one packet of acquisition noise.
    // Search the whole buffered block, preserving the last two packet intervals.
    let max_offset = data.len().saturating_sub(TS_PACKET_SIZE * 2);
    for offset in 0..max_offset {
        let mut probes = 0usize;
        let mut position = offset;
        while position < data.len() && probes < 4 {
            if data[position] != 0x47 {
                break;
            }
            probes += 1;
            position += TS_PACKET_SIZE;
        }
        if probes >= 3 {
            return Some(offset);
        }
    }
    None
}

/// Returns the payload slice after accounting for adaptation-field bytes.
fn packet_payload(packet: &[u8], adaptation_control: u8) -> Option<&[u8]> {
    match adaptation_control {
        1 => Some(&packet[4..]),
        3 => {
            let adaptation_length = *packet.get(4)? as usize;
            let payload_start = 5usize.checked_add(adaptation_length)?;
            packet.get(payload_start..)
        }
        _ => None,
    }
}

#[derive(Default)]
struct SectionAssembler {
    pending: Vec<u8>,
}
impl SectionAssembler {
    fn feed(&mut self, mut bytes: &[u8], allow_new: bool, output: &mut Vec<Vec<u8>>) {
        while !bytes.is_empty() {
            if self.pending.is_empty() && (!allow_new || bytes[0] == 0xff) {
                break;
            }
            let needed = if self.pending.len() < 3 {
                3 - self.pending.len()
            } else {
                let total = 3 + (((self.pending[1] & 15) as usize) << 8) + self.pending[2] as usize;
                if !(8..=1024).contains(&total) {
                    self.pending.clear();
                    break;
                }
                total - self.pending.len()
            };
            let take = needed.min(bytes.len());
            self.pending.extend_from_slice(&bytes[..take]);
            bytes = &bytes[take..];
            if self.pending.len() >= 3 {
                let total = 3 + (((self.pending[1] & 15) as usize) << 8) + self.pending[2] as usize;
                if !(8..=1024).contains(&total) {
                    self.pending.clear();
                    break;
                }
                if self.pending.len() == total {
                    output.push(std::mem::take(&mut self.pending));
                    if !allow_new {
                        break;
                    }
                }
            }
        }
    }
    fn push(&mut self, payload: &[u8], start: bool) -> Vec<Vec<u8>> {
        let mut output = Vec::new();
        if start {
            let Some((&pointer, rest)) = payload.split_first() else {
                return output;
            };
            if pointer as usize > rest.len() {
                self.pending.clear();
                return output;
            }
            if !self.pending.is_empty() {
                self.feed(&rest[..pointer as usize], false, &mut output);
            }
            self.pending.clear();
            self.feed(&rest[pointer as usize..], true, &mut output);
        } else if !self.pending.is_empty() {
            self.feed(payload, false, &mut output);
        }
        output
    }
}
fn crc32_mpeg(bytes: &[u8]) -> u32 {
    let mut crc = 0xffffffffu32;
    for &byte in bytes {
        crc ^= (byte as u32) << 24;
        for _ in 0..8 {
            crc = if crc & 0x80000000 != 0 {
                (crc << 1) ^ 0x04c11db7
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// Parses a reassembled, CRC-checked PAT section and records program-to-PMT PID mappings.
fn parse_pat(section: &[u8], stats: &mut TsStats) {
    if section.first().copied() != Some(0x00) || section.len() < 12 {
        return;
    }
    let section_end_without_crc = section.len().saturating_sub(4);
    let mut offset = 8usize;
    while offset + 4 <= section_end_without_crc {
        let program_number = u16::from_be_bytes([section[offset], section[offset + 1]]);
        let pid = (((section[offset + 2] & 0x1F) as u16) << 8) | section[offset + 3] as u16;
        if program_number != 0 {
            stats.programs.entry(program_number).or_insert(ProgramInfo {
                program_number,
                pmt_pid: pid,
                pcr_pid: None,
            });
        }
        offset += 4;
    }
}

/// Parses a reassembled, CRC-checked PMT section and records PCR and elementary stream PIDs.
fn parse_pmt(section: &[u8], pmt_pid: u16, stats: &mut TsStats) {
    if section.first().copied() != Some(0x02) || section.len() < 16 {
        return;
    }
    let program_number = u16::from_be_bytes([section[3], section[4]]);
    let pcr_pid = (((section[8] & 0x1F) as u16) << 8) | section[9] as u16;
    let program_info_length = (((section[10] & 0x0F) as usize) << 8) | section[11] as usize;
    let mut offset = 12usize.saturating_add(program_info_length);
    let section_end_without_crc = section.len().saturating_sub(4);

    let program = stats.programs.entry(program_number).or_insert(ProgramInfo {
        program_number,
        pmt_pid,
        pcr_pid: Some(pcr_pid),
    });
    program.pmt_pid = pmt_pid;
    program.pcr_pid = Some(pcr_pid);

    for stream in stats.streams.iter().filter(|s|s.program_number==program_number) {
        stats.audio_languages.remove(&stream.pid);
        stats.caption_profiles.remove(&stream.pid);
    }
    stats.streams.retain(|s|s.program_number!=program_number);
    while offset + 5 <= section_end_without_crc {
        let stream_type = section[offset];
        let elementary_pid =
            (((section[offset + 1] & 0x1F) as u16) << 8) | section[offset + 2] as u16;
        let es_info_length =
            (((section[offset + 3] & 0x0F) as usize) << 8) | section[offset + 4] as usize;
        let descriptors_end=offset.saturating_add(5+es_info_length);
        if descriptors_end>section_end_without_crc {break;}
        let mut at=offset+5;
        while at+2<=descriptors_end {
            let tag=section[at];let len=section[at+1] as usize;at+=2;
            if at+len>descriptors_end {break;}
            let data=&section[at..at+len];
            if stream_type==6 && tag==0xfd && len>=2 {
                let profile=u16::from_be_bytes([data[0],data[1]]);
                if matches!(profile,0x0008|0x0012){stats.caption_profiles.insert(elementary_pid,profile);}
            }
            let language=if tag==0x0a && len>=4 {Some(&data[..3])}
                else if tag==0xc4 && len>=9 {Some(&data[6..9])}else{None};
            if let Some(language)=language.filter(|l|l.iter().all(u8::is_ascii_alphabetic)) {
                stats.audio_languages.insert(elementary_pid,String::from_utf8_lossy(language).into_owned());
            }
            at+=len;
        }
        stats.streams.insert(StreamInfo {
            program_number,
            pid: elementary_pid,
            stream_type,
        });
        offset = offset.saturating_add(5 + es_info_length);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pmt_audio_languages_follow_track_updates_and_ignore_truncated_descriptors() {
        fn pmt(streams:&[u8])->Vec<u8> {
            let mut s=vec![2,0xb0,0,0,1,0xc1,0,0,0xe1,1,0xf0,0];
            s.extend_from_slice(streams);s.extend_from_slice(&[0;4]);s
        }
        let mut stats=TsStats::default();
        let s=pmt(&[
            0x0f,0xe1,2,0xf0,6,0x0a,4,b'p',b'o',b'r',0,
            0x11,0xe1,3,0xf0,11,0xc4,9,0,0,0,0,0,0,b'e',b'n',b'g',
        ]);
        parse_pmt(&s,0x100,&mut stats);
        assert_eq!(stats.streams.len(),2);
        assert_eq!(stats.audio_languages.get(&0x102).map(String::as_str),Some("por"));
        assert_eq!(stats.audio_languages.get(&0x103).map(String::as_str),Some("eng"));
        parse_pmt(&pmt(&[0x0f,0xe1,2,0xf0,0]),0x100,&mut stats);
        assert_eq!(stats.streams.len(),1);
        assert!(stats.audio_languages.is_empty());
        parse_pmt(&pmt(&[0x0f,0xe1,2,0xf0,5,0x0a,4,b'p',b'o',b'r']),0x100,&mut stats);
        assert!(stats.audio_languages.is_empty());
        for n in 0..s.len() {parse_pmt(&s[..n],0x100,&mut TsStats::default());}
    }
    #[test]
    fn caption_components_follow_pmt_replacement_and_reject_invalid_descriptors() {
        fn pmt(streams:&[u8])->Vec<u8>{let mut s=vec![2,0xb0,0,0,1,0xc1,0,0,0xe1,1,0xf0,0];s.extend_from_slice(streams);s.extend_from_slice(&[0;4]);s}
        let mut stats=TsStats::default();
        let s=pmt(&[6,0xe1,22,0xf0,5,0xfd,3,0,8,0x3d,6,0xe2,18,0xf0,4,0xfd,2,0,0x12]);
        parse_pmt(&s,0x100,&mut stats);assert_eq!(stats.caption_profiles.get(&278),Some(&8));assert_eq!(stats.caption_profiles.get(&530),Some(&0x12));
        for descriptors in [&[6,0xe1,22,0xf0,0][..],&[6,0xe1,22,0xf0,3,0xfd,2,0][..],&[6,0xe1,22,0xf0,4,0xfd,2,0,1][..]] {
            parse_pmt(&pmt(descriptors),0x100,&mut stats);assert!(stats.caption_profiles.is_empty());
        }
        for n in 0..s.len(){parse_pmt(&s[..n],0x100,&mut TsStats::default());}
    }
    #[test]
    fn nit_channel_number_is_announced_by_broadcast_not_rf_frequency() {
        let mut s = vec![
            0x40, 0xb0, 0, 0, 1, 0xc1, 0, 0, 0xf0, 0, 0xf0, 14, 0, 1, 0, 1, 0xf0, 8, 0xcd, 6, 4, 1,
            0, 1, 0x42, 0x40, 0, 0, 0, 0,
        ];
        let mut stats = TsStats::default();
        parse_nit(&s, &mut stats);
        assert_eq!(stats.channel_numbers.get(&16960), Some(&4));
        for n in 0..s.len() {
            parse_nit(&s[..n], &mut TsStats::default());
        }
        s[17] = 255;
        let mut stats = TsStats::default();
        parse_nit(&s, &mut stats);
        assert!(stats.channel_numbers.is_empty());
    }
    #[test]
    fn acquisition_noise_does_not_discard_valid_tables_later_in_the_block() {
        fn packet(mut section: Vec<u8>, pid: u16) -> Vec<u8> {
            let crc = crc32_mpeg(&section);
            section.extend_from_slice(&crc.to_be_bytes());
            let mut p = vec![0xff; 188];
            p[..5].copy_from_slice(&[0x47, 0x40 | ((pid >> 8) as u8), pid as u8, 0x10, 0]);
            p[5..5 + section.len()].copy_from_slice(&section);
            p
        }
        let pat = packet(vec![0, 0xb0, 0x0d, 0, 1, 0xc1, 0, 0, 0, 1, 0xe1, 0], 0);
        let pmt = packet(
            vec![
                2, 0xb0, 0x12, 0, 1, 0xc1, 0, 0, 0xe1, 1, 0xf0, 0, 0x1b, 0xe1, 1, 0xf0, 0,
            ],
            0x100,
        );
        let mut bytes = vec![0x55; 4096];
        bytes.extend_from_slice(&pat);
        bytes.extend_from_slice(&pmt);
        bytes.extend_from_slice(&pat);
        bytes.extend_from_slice(&pmt);
        let mut a = TsAnalyzer::new();
        a.push(&bytes);
        let stats = a.finish();
        assert_eq!(stats.programs.len(), 1);
        assert!(stats
            .streams
            .iter()
            .any(|s| s.stream_type == 0x1b && s.pid == 0x101));
    }
    fn section(length: usize) -> Vec<u8> {
        let mut data = vec![0u8; length - 4];
        data[0] = 2;
        data[1] = 0xb0 | (((length - 3) >> 8) as u8);
        data[2] = (length - 3) as u8;
        let crc = crc32_mpeg(&data);
        data.extend_from_slice(&crc.to_be_bytes());
        data
    }
    #[test]
    fn multipacket_section_and_pointer_completion() {
        let a = section(240);
        let b = section(24);
        let mut assembler = SectionAssembler::default();
        let mut first = vec![0];
        first.extend_from_slice(&a[..183]);
        assert!(assembler.push(&first, true).is_empty());
        let mut second = vec![57];
        second.extend_from_slice(&a[183..]);
        second.extend_from_slice(&b);
        assert_eq!(assembler.push(&second, true), vec![a, b]);
    }
    #[test]
    fn crc_detects_corruption_and_reassembly_waits_for_start() {
        let mut valid = section(240);
        assert_eq!(crc32_mpeg(&valid), 0);
        valid[42] ^= 1;
        assert_ne!(crc32_mpeg(&valid), 0);
        let mut assembler = SectionAssembler::default();
        assert!(assembler.push(&valid, false).is_empty());
        assert!(assembler.push(&[250, 0, 1], true).is_empty());
        assert!(assembler.pending.is_empty());
    }
}
