//! PCM output modes; compressed broadcast audio and recordings remain unchanged.
use a865r_bda::VideoTransform;
use std::sync::{
    atomic::{AtomicU64, AtomicU8, Ordering},
    Mutex,
};
use windows::{
    core::*,
    Win32::{Foundation::E_INVALIDARG, Media::MediaFoundation::*},
};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum Mode {
    #[default]
    Stereo,
    Mono,
    Left,
    Right,
    Surround,
}
impl Mode {
    pub const ALL: [Self; 5] = [
        Self::Stereo,
        Self::Mono,
        Self::Left,
        Self::Right,
        Self::Surround,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Stereo => "Stereo",
            Self::Mono => "Mono",
            Self::Left => "Left channel",
            Self::Right => "Right channel",
            Self::Surround => "5.1 surround",
        }
    }
    pub fn from_index(i: usize) -> Self {
        Self::ALL.get(i).copied().unwrap_or_default()
    }
}
#[derive(Clone, Copy)]
enum Samples {
    I16,
    I32,
    F32,
}
#[derive(Clone, Copy)]
struct Format {
    channels: usize,
    samples: Samples,
}
fn invalid() -> Error {
    Error::new(E_INVALIDARG, "Unsupported PCM audio format")
}
fn format(mt: &AM_MEDIA_TYPE) -> Option<Format> {
    if mt.majortype != MEDIATYPE_Audio
        || mt.formattype != FORMAT_WaveFormatEx
        || mt.pbFormat.is_null()
        || mt.cbFormat < 18
        || mt.cbFormat > 65536
    {
        return None;
    }
    // DirectShow owns the bounded media-format allocation during this call.
    let b = unsafe { std::slice::from_raw_parts(mt.pbFormat, mt.cbFormat as usize) };
    let word = |i| u16::from_le_bytes([b[i], b[i + 1]]);
    let mut tag = word(0);
    let channels = word(2) as usize;
    let bits = word(14);
    if tag == 0xfffe {
        if b.len() < 40 || word(16) < 22 || word(18) != bits {
            return None;
        }
        if b[28..40] != [0, 0, 16, 0, 128, 0, 0, 170, 0, 56, 155, 113] || b[26..28] != [0, 0] {
            return None;
        }
        if channels == 6 {
            let mask = u32::from_le_bytes(b[20..24].try_into().ok()?);
            if !matches!(mask, 0x3f | 0x60f) {
                return None;
            }
        }
        tag = word(24);
    }
    if !matches!(channels, 1 | 2 | 6) || word(12) as usize != channels * (bits as usize / 8) {
        return None;
    }
    let samples = match (tag, bits) {
        (1, 16) => Samples::I16,
        (1, 32) => Samples::I32,
        (3, 32) => Samples::F32,
        _ => return None,
    };
    Some(Format { channels, samples })
}
/// Request native multichannel PCM, without synthesized speaker fill.
pub unsafe fn configure_decoder(
    decoder: &windows::Win32::Media::DirectShow::IBaseFilter,
) -> Result<()> {
    let api: ICodecAPI = decoder.cast()?;
    let value = VARIANT::from(BSTR::from(
        format!("{{{:?}}}", CODECAPI_GUID_AVDecAudioOutputFormat_PCM).as_str(),
    ));
    api.SetValue(&CODECAPI_AVDecCommonOutputFormat, &value)?;
    let _ = api.SetValue(&CODECAPI_AVDSPSpeakerFill, &VARIANT::from(false));
    Ok(())
}
pub struct Mixer {
    mode: AtomicU8,
    format: Mutex<Option<Format>>,
    frames: AtomicU64,
}
impl Mixer {
    pub fn new(mode: Mode) -> Self {
        Self {
            mode: AtomicU8::new(mode as u8),
            format: Mutex::new(None),
            frames: AtomicU64::new(0),
        }
    }
    pub fn report(&self) -> serde_json::Value {
        let f = *self.format.lock().unwrap();
        serde_json::json!({"mode":Mode::from_index(self.mode.load(Ordering::Relaxed) as usize).name(),"decoded_pcm_frames":self.frames.load(Ordering::Relaxed),"channels":f.map(|f|f.channels),"sample_format":f.map(|f|match f.samples{Samples::I16=>"PCM16",Samples::I32=>"PCM32",Samples::F32=>"Float32"})})
    }
    pub fn set_mode(&self, mode: Mode) {
        self.mode.store(mode as u8, Ordering::Relaxed);
    }
}
impl VideoTransform for Mixer {
    fn accepts(&self, mt: &AM_MEDIA_TYPE) -> bool {
        format(mt).is_some()
    }
    fn configure(&self, mt: &AM_MEDIA_TYPE) -> Result<()> {
        *self.format.lock().unwrap() = Some(format(mt).ok_or_else(invalid)?);
        Ok(())
    }
    fn bytes_required(&self) -> usize {
        65536
    }
    fn allocator_bytes(&self) -> usize {
        65536
    }
    fn requires_nv12_repack(&self) -> bool {
        false
    }
    fn process(&self, bytes: &mut [u8]) -> Result<()> {
        let format = self.format.lock().unwrap().ok_or_else(invalid)?;
        mix(
            bytes,
            format,
            Mode::from_index(self.mode.load(Ordering::Relaxed) as usize),
        )?;
        let size = match format.samples {
            Samples::I16 => 2,
            _ => 4,
        };
        self.frames.fetch_add(
            (bytes.len() / (size * format.channels)) as u64,
            Ordering::Relaxed,
        );
        Ok(())
    }
}
fn mix(bytes: &mut [u8], format: Format, mode: Mode) -> Result<()> {
    let size = match format.samples {
        Samples::I16 => 2,
        _ => 4,
    };
    if bytes.len() % (size * format.channels) != 0 {
        return Err(invalid());
    }
    if format.channels == 1
        || mode == Mode::Surround
        || (format.channels == 2 && mode == Mode::Stereo)
    {
        return Ok(());
    }
    if format.channels == 6 {
        return downmix_surround(bytes, format, mode);
    }
    for frame in bytes.chunks_exact_mut(size * 2) {
        match format.samples {
            Samples::I16 => {
                let l = i16::from_le_bytes(frame[..2].try_into().unwrap()) as i32;
                let r = i16::from_le_bytes(frame[2..4].try_into().unwrap()) as i32;
                let v = match mode {
                    Mode::Mono => (l + r) / 2,
                    Mode::Left => l,
                    _ => r,
                } as i16;
                frame[..2].copy_from_slice(&v.to_le_bytes());
                frame[2..].copy_from_slice(&v.to_le_bytes());
            }
            Samples::I32 => {
                let l = i32::from_le_bytes(frame[..4].try_into().unwrap()) as i64;
                let r = i32::from_le_bytes(frame[4..8].try_into().unwrap()) as i64;
                let v = match mode {
                    Mode::Mono => (l + r) / 2,
                    Mode::Left => l,
                    _ => r,
                } as i32;
                frame[..4].copy_from_slice(&v.to_le_bytes());
                frame[4..].copy_from_slice(&v.to_le_bytes());
            }
            Samples::F32 => {
                let l = f32::from_le_bytes(frame[..4].try_into().unwrap());
                let r = f32::from_le_bytes(frame[4..8].try_into().unwrap());
                let v = match mode {
                    Mode::Mono => l * 0.5 + r * 0.5,
                    Mode::Left => l,
                    _ => r,
                };
                let v = if v.is_finite() { v.clamp(-1., 1.) } else { 0. };
                frame[..4].copy_from_slice(&v.to_le_bytes());
                frame[4..].copy_from_slice(&v.to_le_bytes());
            }
        }
    }
    Ok(())
}
// Windows 5.1 interleave: FL, FR, center, LFE, surround L, surround R.
// Keep center dialogue and surrounds in stereo; reserve headroom and omit LFE.
fn downmix_surround(bytes: &mut [u8], format: Format, mode: Mode) -> Result<()> {
    let size = match format.samples {
        Samples::I16 => 2,
        _ => 4,
    };
    for frame in bytes.chunks_exact_mut(size * 6) {
        let mut channels = [0f64; 6];
        for (i, sample) in frame.chunks_exact(size).enumerate() {
            let v = match format.samples {
                Samples::I16 => i16::from_le_bytes(sample.try_into().unwrap()) as f64 / 32768.,
                Samples::I32 => i32::from_le_bytes(sample.try_into().unwrap()) as f64 / 2147483648.,
                Samples::F32 => f32::from_le_bytes(sample.try_into().unwrap()) as f64,
            };
            channels[i] = if v.is_finite() { v } else { 0. };
        }
        let k = std::f64::consts::FRAC_1_SQRT_2;
        let l = (channels[0] + k * (channels[2] + channels[4])) / (1. + 2. * k);
        let r = (channels[1] + k * (channels[2] + channels[5])) / (1. + 2. * k);
        let (l, r) = match mode {
            Mode::Mono => ((l + r) * 0.5, (l + r) * 0.5),
            Mode::Left => (l, l),
            Mode::Right => (r, r),
            _ => (l, r),
        };
        frame.fill(0);
        for (i, v) in [l, r].into_iter().enumerate() {
            let out = &mut frame[i * size..(i + 1) * size];
            match format.samples {
                Samples::I16 => out.copy_from_slice(
                    &((v * 32768.).round().clamp(i16::MIN as f64, i16::MAX as f64) as i16)
                        .to_le_bytes(),
                ),
                Samples::I32 => out.copy_from_slice(
                    &((v * 2147483648.)
                        .round()
                        .clamp(i32::MIN as f64, i32::MAX as f64) as i32)
                        .to_le_bytes(),
                ),
                Samples::F32 => out.copy_from_slice(&(v.clamp(-1., 1.) as f32).to_le_bytes()),
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn surround_preserves_all_speakers_and_stereo_retains_center_dialogue() {
        let original = [0.1f32, 0.2, 0.3, 0.4, 0.5, 0.6]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>();
        let fmt = Format {
            channels: 6,
            samples: Samples::F32,
        };
        let mut bytes = original.clone();
        mix(&mut bytes, fmt, Mode::Surround).unwrap();
        assert_eq!(bytes, original);
        let mut center = [0f32, 0., 1., 0., 0., 0.]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>();
        mix(&mut center, fmt, Mode::Stereo).unwrap();
        let v: Vec<_> = center
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
            .collect();
        assert!(v[0] > 0.29 && v[0] < 0.30);
        assert_eq!(v[0], v[1]);
        assert!(v[2..].iter().all(|v| *v == 0.));
        let mut hot = [1f32; 6]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>();
        mix(&mut hot, fmt, Mode::Mono).unwrap();
        assert!(hot
            .chunks_exact(4)
            .all(|b| f32::from_le_bytes(b.try_into().unwrap()).abs() <= 1.));
    }
    #[test]
    fn negotiated_pcm_formats_are_bounded_and_validated() {
        let mut b = vec![0u8; 40];
        b[0..2].copy_from_slice(&1u16.to_le_bytes());
        b[2..4].copy_from_slice(&2u16.to_le_bytes());
        b[12..14].copy_from_slice(&4u16.to_le_bytes());
        b[14..16].copy_from_slice(&16u16.to_le_bytes());
        let mt = |b: &mut Vec<u8>, n| AM_MEDIA_TYPE {
            majortype: MEDIATYPE_Audio,
            formattype: FORMAT_WaveFormatEx,
            pbFormat: b.as_mut_ptr(),
            cbFormat: n,
            ..Default::default()
        };
        assert!(format(&mt(&mut b, 18)).is_some());
        assert!(format(&mt(&mut b, 17)).is_none());
        b[0..2].copy_from_slice(&0xfffeu16.to_le_bytes());
        b[16..18].copy_from_slice(&22u16.to_le_bytes());
        b[18..20].copy_from_slice(&16u16.to_le_bytes());
        b[24..26].copy_from_slice(&1u16.to_le_bytes());
        b[28..40].copy_from_slice(&[0, 0, 16, 0, 128, 0, 0, 170, 0, 56, 155, 113]);
        assert!(format(&mt(&mut b, 40)).is_some());
        assert!(format(&mt(&mut b, 39)).is_none());
        b[2..4].copy_from_slice(&6u16.to_le_bytes());
        b[12..14].copy_from_slice(&12u16.to_le_bytes());
        for mask in [0x3fu32, 0x60f] {
            b[20..24].copy_from_slice(&mask.to_le_bytes());
            assert!(format(&mt(&mut b, 40)).is_some());
        }
        b[20..24].copy_from_slice(&0x33u32.to_le_bytes());
        assert!(format(&mt(&mut b, 40)).is_none());
        b[2..4].copy_from_slice(&2u16.to_le_bytes());
        b[12..14].copy_from_slice(&4u16.to_le_bytes());
        b[28] = 1;
        assert!(format(&mt(&mut b, 40)).is_none());
        b[28] = 0;
        b[12] = 8;
        assert!(format(&mt(&mut b, 40)).is_none());
    }
    #[test]
    fn stereo_mono_and_channel_routing_preserve_timing_and_do_not_overflow() {
        let original = [32767i16, -32768, 30000, 30000]
            .into_iter()
            .flat_map(i16::to_le_bytes)
            .collect::<Vec<_>>();
        for (mode, expected) in [
            (Mode::Stereo, vec![32767, -32768, 30000, 30000]),
            (Mode::Mono, vec![0, 0, 30000, 30000]),
            (Mode::Left, vec![32767, 32767, 30000, 30000]),
            (Mode::Right, vec![-32768, -32768, 30000, 30000]),
        ] {
            let mut b = original.clone();
            mix(
                &mut b,
                Format {
                    channels: 2,
                    samples: Samples::I16,
                },
                mode,
            )
            .unwrap();
            assert_eq!(
                b.chunks_exact(2)
                    .map(|x| i16::from_le_bytes(x.try_into().unwrap()))
                    .collect::<Vec<_>>(),
                expected
            );
        }
        assert!(mix(
            &mut [0; 3],
            Format {
                channels: 2,
                samples: Samples::I16
            },
            Mode::Mono
        )
        .is_err());
        let mut wide = [i32::MAX, i32::MAX]
            .into_iter()
            .flat_map(i32::to_le_bytes)
            .collect::<Vec<_>>();
        mix(
            &mut wide,
            Format {
                channels: 2,
                samples: Samples::I32,
            },
            Mode::Mono,
        )
        .unwrap();
        assert_eq!(i32::from_le_bytes(wide[..4].try_into().unwrap()), i32::MAX);
        let mut mono = vec![5, 0];
        mix(
            &mut mono,
            Format {
                channels: 1,
                samples: Samples::I16,
            },
            Mode::Right,
        )
        .unwrap();
        assert_eq!(mono, vec![5, 0]);
        let mut floats = [0.8f32, -0.2]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>();
        mix(
            &mut floats,
            Format {
                channels: 2,
                samples: Samples::F32,
            },
            Mode::Mono,
        )
        .unwrap();
        assert!((f32::from_le_bytes(floats[..4].try_into().unwrap()) - 0.3).abs() < 0.00001);
    }
}
