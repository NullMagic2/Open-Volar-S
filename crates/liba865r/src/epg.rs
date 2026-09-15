//! ISDB event information, decoded from CRC-checked EIT sections.
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub service_id: u16,
    pub transport_id: u16,
    pub network_id: u16,
    pub event_id: u16,
    /// MJD/BCD broadcast clock as civil seconds from 1970, not assumed UTC.
    pub start: i64,
    pub duration: u32,
    pub name: String,
    pub description: String,
    pub language: String,
    pub running: bool,
    pub following: bool,
    pub minimum_age:Option<u8>,
}
/// ISDB-Tb rating descriptor: low nibble is age; high nibble is content.
pub fn minimum_age(country:&[u8],rating:u8)->Option<u8>{
    if country==b"BRA" {match rating&15 {1=>Some(0),2=>Some(10),3=>Some(12),4=>Some(14),5=>Some(16),6=>Some(18),_=>None}}
    else if country==b"JPN" {(1..=15).contains(&rating).then_some(rating.saturating_add(3))}else{None}
}
#[cfg(test)]mod rating_tests{#[test]fn brazil_content_bits_do_not_change_age(){for content in 0..16{for (code,age)in [(1,0),(2,10),(3,12),(4,14),(5,16),(6,18)]{assert_eq!(super::minimum_age(b"BRA",content*16+code),Some(age));}}assert_eq!(super::minimum_age(b"BRA",0),None);assert_eq!(super::minimum_age(b"BRA",15),None);assert_eq!(super::minimum_age(b"XXX",6),None);}}
pub type Events = BTreeMap<(u16, u16, u16, u16), Event>;
fn bcd(v: u8) -> Option<u32> {
    (v >> 4 <= 9 && v & 15 <= 9).then_some((v >> 4) as u32 * 10 + (v & 15) as u32)
}
fn hms(v: &[u8], clock: bool) -> Option<u32> {
    let h = bcd(*v.first()?)?;
    let m = bcd(*v.get(1)?)?;
    let s = bcd(*v.get(2)?)?;
    (m < 60 && s < 60 && (!clock || h < 24)).then_some(h * 3600 + m * 60 + s)
}
pub fn text(v: &[u8]) -> String {
    if v.first() == Some(&0x15) {
        return String::from_utf8_lossy(&v[1..]).trim().to_owned();
    }
    // ISDB-Tb Latin character set. Skip designation/control codes, preserve
    // Latin accents; Japanese multibyte/gaiji is outside these country profiles.
    let mut result = String::new();
    let mut at = 0;
    while at < v.len() {
        let b = v[at];
        at += 1;
        if b == 0x1b {
            while at < v.len() && (0x20..=0x2f).contains(&v[at]) {
                at += 1;
            }
            at = (at + 1).min(v.len());
            continue;
        }
        if b == 0x0d || b == 0x8a {
            result.push('\n');
            continue;
        }
        if b < 0x20 || (0x7f..0xa0).contains(&b) {
            continue;
        }
        result.push(match b {
            0xa4 => '€',
            0xa6 => 'Š',
            0xa8 => 'š',
            0xb4 => 'Ž',
            0xb8 => 'ž',
            0xbc => 'Œ',
            0xbd => 'œ',
            0xbe => 'Ÿ',
            _ => char::from(b),
        });
    }
    result.trim().to_owned()
}
pub(crate) fn parse(s: &[u8], events: &mut Events) {
    if s.len() < 18 || !(s[0] == 0x4e || (0x50..=0x5f).contains(&s[0])) {
        return;
    }
    let service_id = u16::from_be_bytes([s[3], s[4]]);
    let transport_id = u16::from_be_bytes([s[8], s[9]]);
    let network_id = u16::from_be_bytes([s[10], s[11]]);
    let mut at = 14;
    let end = s.len() - 4;
    while at + 12 <= end {
        let event_id = u16::from_be_bytes([s[at], s[at + 1]]);
        let mjd = u16::from_be_bytes([s[at + 2], s[at + 3]]);
        let time = hms(&s[at + 4..at + 7], true);
        let duration = hms(&s[at + 7..at + 10], false);
        let flags = s[at + 10];
        let length = (((flags & 15) as usize) << 8) | s[at + 11] as usize;
        at += 12;
        let last = at + length;
        if last > end {
            return;
        }
        let (Some(time), Some(duration)) = (time, duration) else {
            at = last;
            continue;
        };
        if mjd == 65535 {
            at = last;
            continue;
        }
        let mut event = Event {
            minimum_age:None,
            service_id,
            transport_id,
            network_id,
            event_id,
            start: (mjd as i64 - 40587) * 86400 + time as i64,
            duration,
            name: String::new(),
            description: String::new(),
            language: String::new(),
            running: flags >> 5 == 4,
            following: s[0] == 0x4e && s[6] == 1,
        };
        let mut extended = BTreeMap::new();
        let mut descriptor = at;
        while descriptor + 2 <= last {
            let tag = s[descriptor];
            let size = s[descriptor + 1] as usize;
            descriptor += 2;
            if descriptor + size > last {
                return;
            }
            let d = &s[descriptor..descriptor + size];
            descriptor += size;
            if tag==0x55 {
                event.minimum_age=d.chunks_exact(4).filter_map(|v|minimum_age(&v[..3],v[3])).max();
            } else if tag == 0x4d && d.len() >= 5 {
                let n = d[3] as usize;
                if 4 + n < d.len() {
                    let size = d[4 + n] as usize;
                    if 5 + n + size <= d.len() {
                        event.language = String::from_utf8_lossy(&d[..3]).into_owned();
                        event.name = text(&d[4..4 + n]);
                        event.description = text(&d[5 + n..5 + n + size]);
                    }
                }
            } else if tag == 0x4e && d.len() >= 6 {
                let items_end = 5 + d[4] as usize;
                if items_end >= d.len() {
                    continue;
                }
                let mut detail = String::new();
                let mut i = 5;
                while i < items_end {
                    let n = d[i] as usize;
                    i += 1;
                    if i + n >= items_end {
                        break;
                    }
                    let label = text(&d[i..i + n]);
                    i += n;
                    let n = d[i] as usize;
                    i += 1;
                    if i + n > items_end {
                        break;
                    }
                    let value = text(&d[i..i + n]);
                    i += n;
                    if !label.is_empty() {
                        detail.push_str(&label);
                        detail.push_str(": ");
                    }
                    detail.push_str(&value);
                    detail.push('\n');
                }
                let n = d[items_end] as usize;
                if items_end + 1 + n <= d.len() {
                    detail.push_str(&text(&d[items_end + 1..items_end + 1 + n]));
                    extended.insert(d[0] >> 4, detail);
                }
            }
        }
        for part in extended.values() {
            if !part.is_empty() && *part != event.description {
                if !event.description.is_empty() {
                    event.description.push('\n');
                }
                event.description.push_str(part);
            }
        }
        let key = (network_id, transport_id, service_id, event_id);
        if let Some(old) = events.get(&key).filter(|old| old.start == event.start) {
            if event.name.is_empty() {
                event.name = old.name.clone();
            }
            if event.description.is_empty() {
                event.description = old.description.clone();
            }
        }
        // Hard bound protects against malformed or endless broadcaster event IDs.
        if events.len() < 8192 || events.contains_key(&key) {
            if s[0]==0x4e && event.running && !event.following {
                for old in events.values_mut().filter(|e|e.service_id==service_id&&e.transport_id==transport_id&&e.network_id==network_id&&e.event_id!=event_id){old.running=false;}
            }
            events.insert(key, event);
        }
        at = last;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn eit_times_latin_text_and_malformed_lengths() {
        let mut s = vec![0x4e, 0xb0, 0, 0x42, 0x40, 0xc1, 0, 0, 0, 1, 0, 2, 0, 0x4e];
        let mut d = vec![
            0x4d, 0, b'p', b'o', b'r', 5, b'J', b'o', b'r', b'n', b'a', 3, b'O', b'l', 0xe1,
        ];
        d[1] = (d.len() - 2) as u8;
        s.extend([
            0,
            9,
            0x9e,
            0x8b,
            0x12,
            0x34,
            0x56,
            0x01,
            0x30,
            0,
            0x80,
            d.len() as u8,
        ]);
        s.extend(d);
        s.extend([0; 4]);
        let mut events = Events::new();
        parse(&s, &mut events);
        let e = events.values().next().unwrap();
        assert_eq!(e.start, 12 * 3600 + 34 * 60 + 56);
        assert_eq!(e.duration, 5400);
        assert_eq!(e.description, "Olá");
        assert!(e.running);
        for n in 0..s.len() - 4 {
            let mut rejected = Events::new();
            parse(&s[..n], &mut rejected);
            assert!(rejected.is_empty());
        }
        s[18] = 0x29;
        let mut rejected = Events::new();
        parse(&s, &mut rejected);
        assert!(rejected.is_empty());
    }
}
