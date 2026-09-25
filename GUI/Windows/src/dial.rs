//! Supplied artwork, sampled once per animation frame into a small native control.
use std::time::{Duration, Instant};
use windows::{
    core::Result,
    Win32::{Foundation::*, Graphics::Gdi::*},
};
const TRANSITION: Duration = Duration::from_millis(220);
pub struct Dial {
    image: image::RgbaImage,
    pointer: image::RgbaImage,
    from: f32,
    target: f32,
    started: Instant,
    bitmap: Option<Bitmap>,
    last: Option<(i32, u32)>,
}
struct Bitmap {
    dc: HDC,
    handle: HBITMAP,
    old: HGDIOBJ,
    bits: *mut u8,
    size: i32,
}
impl Drop for Bitmap {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.old);
            let _ = DeleteObject(self.handle);
            let _ = DeleteDC(self.dc);
        }
    }
}
impl Bitmap {
    unsafe fn new(dc: HDC, size: i32) -> Result<Self> {
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: size,
                biHeight: -size,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let handle = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, None, 0)?;
        let memory = CreateCompatibleDC(dc);
        let old = SelectObject(memory, handle);
        Ok(Self {
            dc: memory,
            handle,
            old,
            bits: bits.cast(),
            size,
        })
    }
}
fn interpolate(from: f32, to: f32, elapsed: Duration) -> f32 {
    let t = (elapsed.as_secs_f32() / TRANSITION.as_secs_f32()).min(1.);
    from + (to - from) * (1. - (1. - t).powi(3))
}
// Screen angles increase clockwise; unwrap the seam so dragging never jumps.
pub use crate::interaction::drag_volume;

impl Dial {
    pub fn new(volume: u32) -> std::result::Result<Self, image::ImageError> {
        let image = image::load_from_memory(include_bytes!("../assets/orbit/dial.png"))?.to_rgba8();
        let pointer =
            image::load_from_memory(include_bytes!("../assets/orbit/pointer.png"))?.to_rgba8();
        Ok(Self {
            image,
            pointer,
            from: volume.min(100) as f32,
            target: volume.min(100) as f32,
            started: Instant::now(),
            bitmap: None,
            last: None,
        })
    }
    fn position(&self, now: Instant) -> f32 {
        interpolate(
            self.from,
            self.target,
            now.saturating_duration_since(self.started),
        )
    }
    pub fn set_volume(&mut self, volume: u32) {
        let now = Instant::now();
        self.from = self.position(now);
        self.target = volume.min(100) as f32;
        self.started = now;
    }
    pub fn animating(&self) -> bool {
        self.from != self.target && self.started.elapsed() < TRANSITION
    }
    fn sample_image(image: &image::RgbaImage, x: f32, y: f32) -> [f32; 4] {
        let x = x.clamp(0., image.width() as f32 - 2.);
        let y = y.clamp(0., image.height() as f32 - 2.);
        let (ix, iy) = (x as u32, y as u32);
        let (tx, ty) = (x - ix as f32, y - iy as f32);
        std::array::from_fn(|c| {
            let a = image.get_pixel(ix, iy)[c] as f32 * (1. - tx)
                + image.get_pixel(ix + 1, iy)[c] as f32 * tx;
            let b = image.get_pixel(ix, iy + 1)[c] as f32 * (1. - tx)
                + image.get_pixel(ix + 1, iy + 1)[c] as f32 * tx;
            a * (1. - ty) + b * ty
        })
    }
    unsafe fn render(&mut self, dc: HDC, size: i32, volume: f32) -> Result<()> {
        if self.bitmap.as_ref().is_none_or(|b| b.size != size) {
            self.bitmap = Some(Bitmap::new(dc, size)?);
            self.last = None;
        }
        let key = (size, (volume * 100.).round() as u32);
        if self.last == Some(key) {
            return Ok(());
        }
        let bits = self.bitmap.as_ref().unwrap().bits;
        let dest = std::slice::from_raw_parts_mut(bits, (size * size * 4) as usize);
        // The ceramic pointer rotates from the 65% reference around the exact
        // center; the metal's world-space illumination and scale remain fixed.
        let angle = ((volume - 65.) * 1.8).to_radians();
        let (sin, cos) = angle.sin_cos();
        for y in 0..size {
            for x in 0..size {
                // Integrate nine subpixel samples to soften downscaled edges without
                // blurring the artwork. Transparent samples stay premultiplied.
                let mut total = [0f32; 4];
                for oy in [1. / 6., 0.5, 5. / 6.] {
                    for ox in [1. / 6., 0.5, 5. / 6.] {
                        let sx = (x as f32 + ox) / size as f32 * 640.;
                        let sy = (y as f32 + oy) / size as f32 * 640.;
                        let (dx, dy) = (sx - 320., sy - 320.);
                        let base = Self::sample_image(&self.image, sx, sy);
                        let mark = Self::sample_image(
                            &self.pointer,
                            320. + cos * dx + sin * dy,
                            320. - sin * dx + cos * dy,
                        );
                        let alpha = mark[3] / 255.;
                        for c in 0..3 {
                            total[c] +=
                                (base[c] * (1. - alpha) + mark[c] * alpha) * base[3] / 255. / 9.;
                        }
                        total[3] += base[3] / 9.;
                    }
                }
                let pixel = &mut dest[((y * size + x) * 4) as usize..][..4];
                for c in 0..3 {
                    pixel[2 - c] = total[c].round() as u8;
                }
                pixel[3] = total[3].round() as u8;
            }
        }
        self.last = Some(key);
        Ok(())
    }
    pub unsafe fn paint(&mut self, dc: HDC, r: RECT) {
        let size = (r.right - r.left).min(r.bottom - r.top).clamp(1, 768);
        if self
            .render(dc, size, self.position(Instant::now()))
            .is_err()
        {
            return;
        }
        let bitmap = self.bitmap.as_ref().unwrap();
        let _ = AlphaBlend(
            dc,
            r.left + (r.right - r.left - size) / 2,
            r.top + (r.bottom - r.top - size) / 2,
            size,
            size,
            bitmap.dc,
            0,
            0,
            size,
            size,
            BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            },
        );
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owner_draw_dial_does_not_erase_before_buffered_paint() {
        use windows::{core::w, Win32::UI::WindowsAndMessaging::*};
        unsafe {
            let parent = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                w!(""),
                WS_POPUP,
                0,
                0,
                100,
                100,
                None,
                None,
                None,
                None,
            )
            .unwrap();
            let child = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                w!(""),
                WS_CHILD,
                0,
                0,
                96,
                96,
                parent,
                HMENU(crate::DIAL as usize as _),
                None,
                None,
            )
            .unwrap();
            let screen = GetDC(None);
            let bitmap = Bitmap::new(screen, 96).unwrap();
            let bytes = std::slice::from_raw_parts_mut(bitmap.bits, 96 * 96 * 4);
            bytes.fill(0x35);
            crate::window_proc(
                parent,
                WM_CTLCOLORSTATIC,
                WPARAM(bitmap.dc.0 as usize),
                LPARAM(child.0 as isize),
            );
            assert!(
                bytes.iter().all(|b| *b == 0x35),
                "color callback must not expose a background-only frame"
            );
            assert_eq!(
                crate::dial_proc(
                    child,
                    WM_ERASEBKGND,
                    WPARAM(bitmap.dc.0 as usize),
                    LPARAM(0),
                    1,
                    0
                )
                .0,
                1
            );
            assert!(bytes.iter().all(|b| *b == 0x35));
            let _ = DestroyWindow(parent);
            let _ = ReleaseDC(None, screen);
        }
    }
    #[test]
    fn circular_drag_wraps_and_clamps() {
        let r = f32::to_radians;
        assert!((drag_volume(50., r(179.), r(-179.)) - 51.11111).abs() < 0.001);
        assert!((drag_volume(50., r(-90.), r(0.)) - 100.).abs() < 0.001);
        assert_eq!(drag_volume(0., r(0.), r(-30.)), 0.);
        assert_eq!(drag_volume(100., r(0.), r(30.)), 100.);
    }

    #[test]
    fn animation_retargets_continuously_and_finishes_without_overshoot() {
        let middle = interpolate(20., 80., Duration::from_millis(80));
        assert!(middle > 20. && middle < 80.);
        assert_eq!(interpolate(middle, 10., Duration::ZERO), middle);
        assert_eq!(interpolate(middle, 10., TRANSITION), 10.);
        assert_eq!(interpolate(0., 100., Duration::from_secs(1)), 100.);
    }
    #[test]
    fn artwork_rotation_preserves_scale_and_pointer_endpoints() {
        unsafe {
            let dc = GetDC(None);
            let mut dial = Dial::new(50).unwrap();
            let mut ring = None;
            let size = 384;
            for volume in [0., 50., 100.] {
                dial.render(dc, size, volume).unwrap();
                let b = dial.bitmap.as_ref().unwrap();
                let pixels = std::slice::from_raw_parts(b.bits, (size * size * 4) as usize);
                assert_eq!(pixels[3], 0); // No white square around the circular artwork.
                let mut red = (0f64, 0f64, 0usize);
                for y in 0..size {
                    for x in 0..size {
                        let at = ((y * size + x) * 4) as usize;
                        if pixels[at + 2] > 140
                            && pixels[at + 1] < 65
                            && pixels[at] < 65
                            && pixels[at + 3] > 240
                        {
                            red.0 += x as f64;
                            red.1 += y as f64;
                            red.2 += 1;
                        }
                    }
                }
                assert!(red.2 > 100);
                let (cx, cy) = (red.0 / red.2 as f64, red.1 / red.2 as f64);
                if volume == 0. {
                    assert!(cx < 160. && (cy - 190.).abs() < 8.);
                }
                if volume == 50. {
                    assert!((cx - 192.).abs() < 8. && cy < 160.);
                }
                if volume == 100. {
                    assert!(cx > 220. && (cy - 190.).abs() < 8.);
                }
                let strip = pixels[(320 * size * 4) as usize..(340 * size * 4) as usize].to_vec();
                if let Some(r) = &ring {
                    assert_eq!(r, &strip);
                } else {
                    ring = Some(strip);
                }
                if let Ok(folder) = std::env::var("DIAL_TEST_PREVIEW") {
                    let rgba: Vec<u8> = pixels
                        .chunks_exact(4)
                        .flat_map(|p| [p[2], p[1], p[0], p[3]])
                        .collect();
                    image::save_buffer(
                        std::path::Path::new(&folder).join(format!("dial-{}.png", volume as u32)),
                        &rgba,
                        size as u32,
                        size as u32,
                        image::ColorType::Rgba8,
                    )
                    .unwrap();
                }
            }
            let _ = ReleaseDC(None, dc);
        }
    }
}
