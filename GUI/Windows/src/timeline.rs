//! Incremental transport-stream clock index. Offsets refer to original, unmodified TS bytes.
use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

/// Start just behind the writer so a complete broadcast access unit is available.
pub fn live_position(duration:f64)->f64 {if duration.is_finite(){(duration-2.).max(0.)}else{0.}}
pub fn relative_position(position:f64,delta:f64,duration:f64)->f64 {
    if !position.is_finite() || !delta.is_finite() || !duration.is_finite(){return 0.;}
    (position+delta).clamp(0.,duration.max(0.))
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Point {
    pub seconds: f64,
    pub offset: u64,
}
pub struct Index {
    reader: File,
    pending: Vec<u8>,
    offset: u64,
    pid: u16,
    last_clock: Option<u64>,
    elapsed: u64,
    pub points: Vec<Point>,
    pub duration: f64,
}
impl Index {
    pub fn open(path: &Path, pid: u16) -> io::Result<Self> {
        Ok(Self {
            reader: File::open(path)?,
            pending: Vec::new(),
            offset: 0,
            pid,
            last_clock: None,
            elapsed: 0,
            points: vec![Point::default()],
            duration: 0.,
        })
    }
    pub fn update(&mut self) -> io::Result<()> {
        let mut buf = [0u8; 188 * 1024];
        // Snapshot the readable length; a live writer cannot keep this call busy forever.
        use std::io::Seek;
        let mut remaining = self
            .reader
            .metadata()?
            .len()
            .saturating_sub(self.reader.stream_position()?);
        while remaining > 0 {
            let want = remaining.min(buf.len() as u64) as usize;
            let n = self.reader.read(&mut buf[..want])?;
            if n == 0 {
                break;
            }
            remaining -= n as u64;
            self.push(&buf[..n]);
        }
        Ok(())
    }
    fn push(&mut self, bytes: &[u8]) {
        self.pending.extend_from_slice(bytes);
        let mut pos = 0;
        while pos + 188 <= self.pending.len() {
            if self.pending[pos] != 0x47
                || (pos + 376 < self.pending.len()
                    && (self.pending[pos + 188] != 0x47 || self.pending[pos + 376] != 0x47))
            {
                pos += 1;
                continue;
            }
            let p = &self.pending[pos..pos + 188];
            let pid = ((u16::from(p[1]) & 31) << 8) | u16::from(p[2]);
            if pid == self.pid
                && p[1] & 0x80 == 0
                && p[3] & 0x20 != 0
                && (7..=183).contains(&p[4])
                && p[5] & 0x10 != 0
            {
                let clock = (u64::from(p[6]) << 25)
                    | (u64::from(p[7]) << 17)
                    | (u64::from(p[8]) << 9)
                    | (u64::from(p[9]) << 1)
                    | u64::from(p[10] >> 7);
                if let Some(last) = self.last_clock {
                    let delta = clock.wrapping_sub(last) & ((1u64 << 33) - 1);
                    // Clock resets and reception discontinuities must not invent hours of video.
                    if delta <= 90000 * 10 && p[5] & 0x80 == 0 {
                        self.elapsed += delta;
                    }
                }
                self.last_clock = Some(clock);
                self.duration = self.elapsed as f64 / 90000.;
                if self.duration - self.points.last().unwrap().seconds >= 0.5 {
                    self.points.push(Point {
                        seconds: self.duration,
                        offset: self.offset + pos as u64,
                    });
                }
            }
            pos += 188;
        }
        self.pending.drain(..pos);
        self.offset += pos as u64;
    }
    pub fn seek(&self, seconds: f64) -> Point {
        let seconds = if seconds.is_finite() {
            seconds.clamp(0., self.duration)
        } else {
            0.
        };
        let i = self.points.partition_point(|p| p.seconds <= seconds);
        self.points[i.saturating_sub(1)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transport_seeks_stay_in_recorded_range_and_live_keeps_a_decoding_margin() {
        assert_eq!(relative_position(3.,-10.,20.),0.);
        assert_eq!(relative_position(15.,10.,20.),20.);
        assert_eq!(live_position(20.),18.);
        assert_eq!(live_position(1.),0.);
        assert_eq!(live_position(f64::NAN),0.);
    }
    fn packet(clock: u64, pid: u16) -> [u8; 188] {
        let mut p = [0xff; 188];
        p[0] = 0x47;
        p[1] = (pid >> 8) as u8;
        p[2] = pid as u8;
        p[3] = 0x20;
        p[4] = 183;
        p[5] = 0x10;
        p[6] = (clock >> 25) as u8;
        p[7] = (clock >> 17) as u8;
        p[8] = (clock >> 9) as u8;
        p[9] = (clock >> 1) as u8;
        p[10] = ((clock & 1) << 7) as u8;
        p
    }
    #[test]
    fn growing_file_noise_wrap_and_seek_boundaries() {
        use std::io::Write;
        let path = std::env::temp_dir().join(format!("a865r-index-{}.ts", std::process::id()));
        let mut writer = File::create(&path).unwrap();
        writer.write_all(&[0xaa; 4096]).unwrap();
        let mut index = Index::open(&path, 42).unwrap();
        index.update().unwrap();
        assert_eq!(index.duration, 0.);
        let wrap = 1u64 << 33;
        for (i, c) in [wrap - 90000, wrap - 45000, 0, 45000, 90000]
            .iter()
            .enumerate()
        {
            writer.write_all(&packet(*c, 42)).unwrap();
            writer.flush().unwrap();
            index.update().unwrap();
            assert_eq!(index.duration, i as f64 * 0.5);
        }
        assert_eq!(index.seek(1.3).seconds, 1.);
        assert_eq!(index.seek(1.3).offset, 4096 + 188 * 2);
        assert_eq!(index.seek(-3.).offset, 0);
        assert_eq!(index.seek(999.).seconds, 2.);
        writer.write_all(&packet(500_000_000, 42)).unwrap();
        index.update().unwrap();
        assert_eq!(index.duration, 2.);
        drop(index);
        drop(writer);
        std::fs::remove_file(path).unwrap();
    }
}
