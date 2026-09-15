//! DirectX picture effects on Microsoft-decoded NV12, cached across display fields.
use super::{error, Frame};
use std::{collections::VecDeque, sync::Arc, time::Instant};
use windows::core::Result;

pub(super) struct PictureStage {
    transform: Arc<crate::icc::Transform>,
    cache: VecDeque<Frame>,
    picture: crate::picture::Picture,
    frames: u64,
    micros: u64,
}
impl PictureStage {
    pub fn new(profile: Option<Arc<crate::icc::Transform>>, shader: crate::backend::Shader) -> Self {
        let transform = profile.unwrap_or_else(|| Arc::new(crate::icc::Transform::identity()));
        transform.set_shader(shader);
        Self { transform, cache: VecDeque::new(), picture: Default::default(),
            frames: 0, micros: 0 }
    }
    pub fn picture(&mut self, picture: crate::picture::Picture) -> bool {
        let changed = self.picture != picture;
        if changed { self.cache.clear(); self.picture = picture; }
        self.transform.set_picture(picture);
        changed
    }
    pub fn report(&self) -> serde_json::Value {
        let mut report = self.transform.backend();
        report["frames_processed"] = self.frames.into();
        report["preparation_average_ms"] = serde_json::json!(self.micros as f64 / self.frames.max(1) as f64 / 1000.);
        report
    }
    pub fn frame(&mut self, frame: &Frame) -> Result<Frame> {
        if let Some(cached) = self.cache.iter().find(|v| v.epoch == frame.epoch && v.id == frame.id) {
            return Ok(cached.clone());
        }
        let started = Instant::now();
        let f = frame.format;
        if frame.texture.is_some() {
            return Err(error("DirectX effects require Microsoft-decoded frames; Vulkan frames stay on their shared device"));
        }
        let mut data = frame.bytes.to_vec();
        self.transform.process_nv12(&mut data, f.pitch, f.height, f.bt709)?;
        let mut result = frame.clone(); result.texture = None; result.bytes = data.into();
        self.cache.push_back(result.clone());
        while self.cache.len() > 3 { self.cache.pop_front(); }
        self.frames += 1; self.micros += started.elapsed().as_micros() as u64;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{backend::Shader, picture::Picture};
    fn frame() -> Frame {
        Frame { bytes: vec![110; 64 * 32 * 3 / 2].into(), texture: None,
            epoch: 0, id: 1, start: 1000, end: 2000, flags: 0,
            format: super::super::Format { pitch:64, width:64, height:32,
                visible_height:32, bt709:true, ..Default::default() } }
    }
    #[test]
    fn cache_respects_epoch_and_retains_original_decoded_pixels() {
        let mut stage = PictureStage::new(None, Shader::Off);
        let mut input = frame(); let original = input.bytes.clone();
        let first = stage.frame(&input).unwrap();
        let second = stage.frame(&input).unwrap();
        assert!(Arc::ptr_eq(&first.bytes, &second.bytes));
        assert_eq!(stage.frames, 1); assert_eq!(input.bytes, original);
        input.epoch += 1; input.bytes = vec![70; input.bytes.len()].into();
        let after_seek = stage.frame(&input).unwrap();
        assert_ne!(first.bytes, after_seek.bytes);
        assert_eq!(after_seek.start, input.start); assert_eq!(after_seek.end, input.end);
    }
    #[test]
    #[ignore = "Explicit DirectX hardware validation"]
    fn effects_apply_once_and_change_on_the_same_paused_frame() {
        for shader in [Shader::Dx12, Shader::Dx11] {
            let mut stage = PictureStage::new(None, shader);
            let input = frame(); let original = input.bytes.clone();
            stage.picture(Picture { brightness:20, ..Default::default() });
            let first = stage.frame(&input).unwrap();
            assert_ne!(first.bytes, original);
            assert_eq!(stage.frame(&input).unwrap().bytes, first.bytes);
            assert_eq!(stage.frames, 1);
            stage.picture(Default::default());
            assert_eq!(stage.frame(&input).unwrap().bytes, original);
            assert_eq!(stage.frames, 2); assert_eq!(input.bytes, original);
        }
    }
}
