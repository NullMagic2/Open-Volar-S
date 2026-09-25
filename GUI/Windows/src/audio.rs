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
#[path = "../../shared/audio.rs"]
mod pcm;
pub use pcm::Mode;
use pcm::{Format, Samples};
fn mix(bytes: &mut [u8], format: Format, mode: Mode) -> Result<()> {
    pcm::mix(bytes, format, mode).map_err(|_| invalid())
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
#[cfg(test)]
mod tests {
    use super::*;
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
}
