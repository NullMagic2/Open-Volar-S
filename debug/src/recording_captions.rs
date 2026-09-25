//! Shared ISDB caption metadata preservation after GPU export.
use std::{collections::{BTreeMap,BTreeSet},fs,io::{Read,Write},path::Path};
#[derive(Default)]
struct Packets(Vec<u8>);
impl Packets {
    fn push(&mut self, data: &[u8]) -> Vec<[u8; 188]> {
        self.0.extend_from_slice(data);
        let mut result = Vec::new();
        let mut at = 0;
        while self.0.len() - at >= 188 {
            if self.0[at] != 0x47 {
                at += 1;
                continue;
            }
            if self.0.len() - at >= 376 && self.0[at + 188] != 0x47 {
                at += 1;
                continue;
            }
            result.push(self.0[at..at + 188].try_into().unwrap());
            at += 188;
        }
        self.0.drain(..at);
        result
    }
}
fn pid(p: &[u8; 188]) -> u16 {
    ((p[1] as u16 & 31) << 8) | p[2] as u16
}
fn crc(data: &[u8]) -> u32 {
    let mut c = 0xffffffff;
    for &b in data {
        c ^= (b as u32) << 24;
        for _ in 0..8 {
            c = if c & 0x80000000 != 0 {
                (c << 1) ^ 0x04c11db7
            } else {
                c << 1
            };
        }
    }
    c
}
#[derive(Default)]
struct Sections(BTreeMap<u16, Vec<u8>>);
impl Sections {
    fn push(&mut self, p: &[u8; 188]) -> Vec<(u16, Vec<u8>)> {
        let id = pid(p);
        if p[1] & 0x80 != 0 || p[3] & 0xc0 != 0 || p[3] & 0x10 == 0 {
            self.0.remove(&id);
            return vec![];
        }
        let mut at = 4;
        if p[3] & 0x20 != 0 {
            at += 1 + p[4] as usize;
        }
        if at >= 188 {
            return vec![];
        }
        let b = self.0.entry(id).or_default();
        let mut result = Vec::new();
        if p[1] & 0x40 != 0 {
            let pointer = p[at] as usize;
            at += 1;
            if at + pointer > 188 {
                b.clear();
                return result;
            }
            if !b.is_empty() {
                b.extend_from_slice(&p[at..at + pointer]);
                Self::take(id, b, &mut result);
            }
            b.clear();
            at += pointer;
        } else if b.is_empty() {
            return result;
        }
        b.extend_from_slice(&p[at..]);
        Self::take(id, b, &mut result);
        result
    }
    fn take(id: u16, b: &mut Vec<u8>, out: &mut Vec<(u16, Vec<u8>)>) {
        while b.len() >= 3 {
            if b[0] == 0xff {
                b.clear();
                break;
            }
            let n = 3 + (((b[1] as usize & 15) << 8) | b[2] as usize);
            if n > 4096 || n < 7 {
                b.clear();
                break;
            }
            if b.len() < n {
                break;
            }
            let section: Vec<_> = b.drain(..n).collect();
            if crc(&section) == 0 {
                out.push((id, section));
            }
        }
    }
}
fn packetize(id: u16, section: &[u8]) -> Vec<[u8; 188]> {
    let mut bytes = vec![0];
    bytes.extend_from_slice(section);
    bytes
        .chunks(184)
        .enumerate()
        .map(|(i, c)| {
            let mut p = [255; 188];
            p[0] = 0x47;
            p[1] = ((id >> 8) as u8 & 31) | if i == 0 { 64 } else { 0 };
            p[2] = id as u8;
            p[3] = 0x10;
            p[4..4 + c.len()].copy_from_slice(c);
            p
        })
        .collect()
}
/// Restore ISDB descriptors stripped when a remuxer labels caption PES as bin_data.
/// A/V packets and their timestamps are copied byte-for-byte.
pub fn restore_caption_metadata(
    input: &Path,
    output: &Path,
    profiles: &BTreeMap<u16, u16>,
) -> Result<(), String> {
    if profiles.is_empty() {
        return Err("No caption descriptors requested".into());
    }
    use std::io::{Seek, SeekFrom};
    let mut source = fs::File::open(input).map_err(|e| e.to_string())?;
    let mut initial = vec![0; 2 * 1024 * 1024];
    let n = source.read(&mut initial).map_err(|e| e.to_string())?;
    initial.truncate(n);
    let mut a = a865r::TsAnalyzer::new();
    for chunk in initial.chunks(188*256){a.push(chunk);}
    let pmts: BTreeSet<_> = a.stats().programs.values().map(|p| p.pmt_pid).collect();
    source.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let target = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    let mut target=std::io::BufWriter::with_capacity(1024*1024,target);
    let mut framing = Packets::default();
    let mut sections = Sections::default();
    let mut counters = BTreeMap::<u16, u8>::new();
    let mut buffer = [0; 188 * 256];
    let mut restored = BTreeSet::new();
    loop {
        let n = source.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        for p in framing.push(&buffer[..n]) {
            let id = pid(&p);
            if !pmts.contains(&id) {
                target.write_all(&p).map_err(|e| e.to_string())?;
                continue;
            }
            for (_, mut s) in sections.push(&p) {
                if s[0] != 2 || s.len() < 16 {
                    continue;
                }
                let end = s.len() - 4;
                let mut at = 12 + (((s[10] as usize & 15) << 8) | s[11] as usize);
                if at > end {
                    return Err("Malformed recording PMT".into());
                }
                let mut body = s[..at].to_vec();
                while at + 5 <= end {
                    let len = ((s[at + 3] as usize & 15) << 8) | s[at + 4] as usize;
                    let next = at + 5 + len;
                    if next > end {
                        return Err("Malformed recording descriptors".into());
                    }
                    let stream_pid = ((s[at + 1] as u16 & 31) << 8) | s[at + 2] as u16;
                    if let Some(&profile) = profiles.get(&stream_pid) {
                        let desc = [
                            0x52,
                            1,
                            0x30,
                            0xfd,
                            3,
                            (profile >> 8) as u8,
                            profile as u8,
                            0x3d,
                        ];
                        body.extend_from_slice(&[6, s[at + 1], s[at + 2], 0xf0, desc.len() as u8]);
                        body.extend_from_slice(&desc);
                        restored.insert(stream_pid);
                    } else {
                        body.extend_from_slice(&s[at..next]);
                    }
                    at = next;
                }
                let len = body.len() + 4 - 3;
                if len > 1021 {
                    return Err("Caption PMT exceeds MPEG section limit".into());
                }
                body[1] = (body[1] & 0xf0) | ((len >> 8) as u8 & 15);
                body[2] = len as u8;
                let c = crc(&body);
                body.extend_from_slice(&c.to_be_bytes());
                s = body;
                for mut packet in packetize(id, &s) {
                    let cc = counters.entry(id).or_insert(0);
                    packet[3] = (packet[3] & 0xf0) | *cc;
                    *cc = (*cc + 1) & 15;
                    target.write_all(&packet).map_err(|e| e.to_string())?;
                }
            }
        }
    }
    target.flush().map_err(|e|e.to_string())?;
    target.get_ref().sync_all().map_err(|e|e.to_string())?;
    if !profiles.keys().all(|p| restored.contains(p)) {
        return Err("Caption PID missing from completed recording".into());
    }
    Ok(())
}
