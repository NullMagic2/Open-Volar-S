//! Native Windows ICC transform, applied to decoded SDR video before GPU presentation.
//! The original transport stream is never modified. No external color or codec executable.
use a865r_bda::VideoTransform;
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicI64, AtomicU64, Ordering},
        Mutex,
    },
    time::Instant,
};
use windows::{
    core::*,
    Win32::{Foundation::*, Graphics::Gdi::*, Media::MediaFoundation::*, UI::ColorSystem::*},
};
const GRID: usize = 33;
#[derive(Clone, Copy, Default, PartialEq)]
struct Layout {
    width: usize,
    height: usize,
    bt709: bool,
}
#[derive(Default)]
struct GpuState {
    layout: Layout,
    processor: Option<crate::icc_gpu::Processor>,
    attempted: bool,
    error: Option<String>,
    initializations:u64,
    initialize_micros:u64,
    apply_micros:u64,
    apply_count:u64,
    max_apply_micros:u64,
}
pub struct Transform {
    lut: Vec<[f32; 3]>,
    picture: Mutex<crate::picture::Picture>,
    shader:Mutex<crate::backend::Shader>,
    delivered_until:AtomicI64,
    layout: Mutex<Layout>,
    gpu: Mutex<GpuState>,
    pub path: PathBuf,
    pub sha256: String,
    pub frames: AtomicU64,
    pub micros: AtomicU64,
}
fn error(s: impl Into<String>) -> Error {
    Error::new(E_FAIL, s.into())
}
struct ProfileHandle(isize);
impl Drop for ProfileHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseColorProfile(self.0);
        }
    }
}
struct ColorTransform(isize);
impl Drop for ColorTransform {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteColorTransform(self.0);
        }
    }
}
unsafe fn open(path: &Path) -> Result<ProfileHandle> {
    use std::os::windows::ffi::OsStrExt;
    let name: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let profile = PROFILE {
        dwType: PROFILE_FILENAME,
        pProfileData: name.as_ptr() as _,
        cbDataSize: (name.len() * 2) as u32,
    };
    let handle = OpenColorProfileW(&profile, PROFILE_READ, 1, 3);
    if handle == 0 {
        Err(Error::from_win32())
    } else {
        Ok(ProfileHandle(handle))
    }
}
pub unsafe fn monitor_profile(hwnd: HWND) -> Option<PathBuf> {
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of_val(&info) as u32;
    if !GetMonitorInfoW(
        MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST),
        (&mut info as *mut MONITORINFOEXW).cast(),
    )
    .as_bool()
    {
        return None;
    }
    let dc = CreateDCW(w!("DISPLAY"), PCWSTR(info.szDevice.as_ptr()), None, None);
    if dc.is_invalid() {
        return None;
    }
    let mut name = [0u16; 32768];
    let mut size = name.len() as u32;
    let ok = GetICMProfileW(dc, &mut size, PWSTR(name.as_mut_ptr()));
    let _ = DeleteDC(dc);
    ok.as_bool().then(|| {
        PathBuf::from(String::from_utf16_lossy(
            &name[..name.iter().position(|c| *c == 0).unwrap_or(0)],
        ))
    })
}
impl Transform {
    pub fn identity() -> Self {
        let lut=(0..GRID*GRID*GRID).map(|i|[(i%GRID)as f32/(GRID-1)as f32,((i/GRID)%GRID)as f32/(GRID-1)as f32,(i/GRID/GRID)as f32/(GRID-1)as f32]).collect();
        Self{lut,picture:Mutex::new(Default::default()),shader:Mutex::new(Default::default()),delivered_until:AtomicI64::new(0),layout:Mutex::new(Layout::default()),gpu:Mutex::new(GpuState::default()),path:PathBuf::new(),sha256:String::new(),frames:AtomicU64::new(0),micros:AtomicU64::new(0)}
    }
    /// EVR schedules accepted samples against the shared DirectShow audio clock.
    /// Limit captions to delivered video so a slow decoder/shader cannot run them ahead.
    pub fn caption_position(&self,graph_ms:i64)->i64 {graph_ms.max(0).min(self.delivered_until.load(Ordering::Acquire))}
    pub fn set_shader(&self,shader:crate::backend::Shader) {let mut selected=self.shader.lock().unwrap();if *selected!=shader{*selected=shader;*self.gpu.lock().unwrap()=GpuState::default();}}
    pub fn set_picture(&self, picture:crate::picture::Picture) {*self.picture.lock().unwrap()=picture;}

    pub fn lut_rgba(&self) -> Vec<f32> {
        self.lut
            .iter()
            .flat_map(|v| [v[0], v[1], v[2], 1.])
            .collect()
    }
    pub fn load(path: &Path) -> Result<Self> {
        let sha256 =
            a865r_media::color::validate_profile(path).map_err(|e| error(e.to_string()))?;
        let source =
            PathBuf::from(std::env::var_os("WINDIR").unwrap_or_else(|| "C:/Windows".into()))
                .join("System32/spool/drivers/color/sRGB Color Space Profile.icm");
        let mut lut = vec![[0.; 3]; GRID * GRID * GRID];
        unsafe {
            let source = open(&source)?;
            let destination = open(path)?;
            let transform = ColorTransform(CreateMultiProfileTransform(
                &[source.0, destination.0],
                &[INTENT_RELATIVE_COLORIMETRIC],
                BEST_MODE,
                0,
            ));
            if transform.0 == 0 {
                return Err(error(format!(
                    "Windows could not construct this ICC transform: {}",
                    Error::from_win32()
                )));
            }
            let mut input = vec![COLOR::default(); lut.len()];
            for b in 0..GRID {
                for g in 0..GRID {
                    for r in 0..GRID {
                        input[(b * GRID + g) * GRID + r] = COLOR {
                            rgb: RGBCOLOR {
                                red: (r * 65535 / (GRID - 1)) as u16,
                                green: (g * 65535 / (GRID - 1)) as u16,
                                blue: (b * 65535 / (GRID - 1)) as u16,
                            },
                        };
                    }
                }
            }
            let mut output = vec![COLOR::default(); lut.len()];
            if !TranslateColors(
                transform.0,
                input.as_ptr(),
                input.len() as u32,
                COLOR_RGB,
                output.as_mut_ptr(),
                COLOR_RGB,
            )
            .as_bool()
            {
                return Err(Error::from_win32());
            }
            for (entry, color) in lut.iter_mut().zip(output) {
                let c = color.rgb;
                *entry = [
                    c.red as f32 / 65535.,
                    c.green as f32 / 65535.,
                    c.blue as f32 / 65535.,
                ];
            }
        }
        Ok(Self {
            lut,
            picture: Mutex::new(Default::default()),shader:Mutex::new(Default::default()),delivered_until:AtomicI64::new(0),            layout: Mutex::new(Layout::default()),
            gpu: Mutex::new(GpuState::default()),
            path: path.into(),
            sha256,
            frames: AtomicU64::new(0),
            micros: AtomicU64::new(0),
        })
    }
    pub fn process_nv12(&self, bytes: &mut [u8], width: usize, height: usize, bt709: bool) -> Result<()> {
        *self.layout.lock().unwrap() = Layout { width, height, bt709 };
        self.process(bytes)
    }
    pub fn backend(&self) -> serde_json::Value {
        let shader=*self.shader.lock().unwrap();let gpu = self.gpu.lock().unwrap();
        serde_json::json!({"delivered_until_ms":self.delivered_until.load(Ordering::Acquire),"enabled":gpu.processor.is_some() || (!gpu.attempted && shader.effects(crate::backend::Backend::Microsoft,crate::backend::capabilities())),"name":if let Some(p)=&gpu.processor{p.name()}else if gpu.attempted{"CPU ICC / shaders disabled"}else{"initializing"},"layout":[gpu.layout.width,gpu.layout.height],"initializations":gpu.initializations,"initialize_ms":gpu.initialize_micros as f64/1000.,"gpu_average_ms":gpu.apply_micros as f64/gpu.apply_count.max(1)as f64/1000.,"gpu_max_ms":gpu.max_apply_micros as f64/1000.,"fallback_reason":gpu.error,"picture":self.picture.lock().unwrap().json()})
    }
    #[inline]
    fn map(&self, rgb: [f32; 3]) -> [f32; 3] {
        let p = rgb.map(|v| v.clamp(0., 1.) * (GRID - 1) as f32);
        let lo = p.map(|v| (v as usize).min(GRID - 2));
        let f = [
            p[0] - lo[0] as f32,
            p[1] - lo[1] as f32,
            p[2] - lo[2] as f32,
        ];
        let base = (lo[2] * GRID + lo[1]) * GRID + lo[0];
        // Tetrahedral interpolation needs only four table samples per pixel.
        let (a, b, x, y, z) = if f[0] >= f[1] {
            if f[1] >= f[2] {
                (1, 1 + GRID, f[0], f[1], f[2])
            } else if f[0] >= f[2] {
                (1, 1 + GRID * GRID, f[0], f[2], f[1])
            } else {
                (GRID * GRID, GRID * GRID + 1, f[2], f[0], f[1])
            }
        } else if f[0] >= f[2] {
            (GRID, GRID + 1, f[1], f[0], f[2])
        } else if f[1] >= f[2] {
            (GRID, GRID + GRID * GRID, f[1], f[2], f[0])
        } else {
            (GRID * GRID, GRID * GRID + GRID, f[2], f[1], f[0])
        };
        let p0 = self.lut[base];
        let p1 = self.lut[base + a];
        let p2 = self.lut[base + b];
        let p3 = self.lut[base + 1 + GRID + GRID * GRID];
        std::array::from_fn(|i| {
            p0[i] + x * (p1[i] - p0[i]) + y * (p2[i] - p1[i]) + z * (p3[i] - p2[i])
        })
    }
    fn rows(&self, y: &mut [u8], uv: &mut [u8], width: usize, bt709: bool, picture:crate::picture::Picture) {
        let (kr, kb) = if bt709 {
            (0.2126, 0.0722)
        } else {
            (0.299, 0.114)
        };
        let kg = 1. - kr - kb;
        for (ys, cs) in y
            .chunks_exact_mut(width * 2)
            .zip(uv.chunks_exact_mut(width))
        {
            for x in (0..width).step_by(2) {
                let u = (cs[x] as f32 - 128.) / 224.;
                let v = (cs[x + 1] as f32 - 128.) / 224.;
                let mut su = 0.;
                let mut sv = 0.;
                for offset in [x, x + 1, width + x, width + x + 1] {
                    let l = (ys[offset] as f32 - 16.) / 219.;
                    let rgb = picture.apply_rgb([
                        l + 2. * (1. - kr) * v,
                        l - 2. * kb * (1. - kb) / kg * u - 2. * kr * (1. - kr) / kg * v,
                        l + 2. * (1. - kb) * u,
                    ]);
                    let rgb=if self.path.as_os_str().is_empty(){rgb}else{self.map(rgb)};
                    let yy = kr * rgb[0] + kg * rgb[1] + kb * rgb[2];
                    su += (rgb[2] - yy) / (2. * (1. - kb));
                    sv += (rgb[0] - yy) / (2. * (1. - kr));
                    ys[offset] = (16. + 219. * yy).round().clamp(16., 235.) as u8;
                }
                cs[x] = (128. + 56. * su).round().clamp(16., 240.) as u8;
                cs[x + 1] = (128. + 56. * sv).round().clamp(16., 240.) as u8;
            }
        }
    }
}
fn layout(media: &AM_MEDIA_TYPE) -> Option<Layout> {
    unsafe {
        if media.majortype != MEDIATYPE_Video
            || media.subtype != MEDIASUBTYPE_NV12
            || media.pbFormat.is_null()
        {
            return None;
        }
        let (header, flags) = if media.formattype == FORMAT_VideoInfo2
            && media.cbFormat as usize >= std::mem::size_of::<VIDEOINFOHEADER2>()
        {
            let v = std::ptr::read_unaligned(media.pbFormat.cast::<VIDEOINFOHEADER2>());
            (v.bmiHeader, v.Anonymous.dwControlFlags)
        } else if media.formattype == FORMAT_VideoInfo
            && media.cbFormat as usize >= std::mem::size_of::<VIDEOINFOHEADER>()
        {
            (
                std::ptr::read_unaligned(media.pbFormat.cast::<VIDEOINFOHEADER>()).bmiHeader,
                0,
            )
        } else {
            return None;
        };
        let width = usize::try_from(header.biWidth).ok()?;
        let height = header.biHeight.unsigned_abs() as usize;
        if width == 0
            || height == 0
            || width > 8192
            || height > 4320
            || width % 2 != 0
            || height % 2 != 0
        {
            return None;
        }
        let matrix = (flags >> 15) & 7;
        Some(Layout {
            width,
            height,
            bt709: if flags & 0x80 != 0 && matrix > 0 {
                matrix == 1
            } else {
                width >= 1280 || height > 576
            },
        })
    }
}
impl VideoTransform for Transform {
    fn accepts(&self, media: &AM_MEDIA_TYPE) -> bool {
        layout(media).is_some()
    }
    fn delivered(&self,sample:&windows::Win32::Media::DirectShow::IMediaSample) {
        let (mut start,mut end)=(0,0);
        let until=if unsafe{sample.GetTime(&mut start,&mut end)}.is_ok(){end.max(start).max(0)/10000}else{i64::MAX};
        self.delivered_until.store(until,Ordering::Release);
    }
    fn stop(&self) {self.delivered_until.store(0,Ordering::Release);}
    fn end_flush(&self) {self.stop();}
    fn configure(&self, media: &AM_MEDIA_TYPE) -> Result<()> {
        *self.layout.lock().unwrap() = layout(media)
            .ok_or_else(|| error("ICC rendering requires decoded 8-bit NV12 video"))?;
        Ok(())
    }
    fn bytes_required(&self) -> usize {
        let f = *self.layout.lock().unwrap();
        f.width * f.height * 3 / 2
    }
    fn process(&self, bytes: &mut [u8]) -> Result<()> {
        let f = *self.layout.lock().unwrap();
        let len = f.width * f.height;
        if len == 0 || bytes.len() < len * 3 / 2 {
            return Err(error("Decoded frame is shorter than its color format"));
        }
        let shader=*self.shader.lock().unwrap();
        let picture=if shader==crate::backend::Shader::Off {Default::default()}else{*self.picture.lock().unwrap()};
        // Neutral without an ICC profile is a byte-exact pass-through.
        if self.path.as_os_str().is_empty() && picture==Default::default() {
            self.frames.fetch_add(1,Ordering::Relaxed);return Ok(());
        }
        let start = Instant::now();
        let applied = {
            let mut gpu = self.gpu.lock().unwrap();
            if shader!=crate::backend::Shader::Off && (!gpu.attempted || gpu.layout != f) {
                let initializations=gpu.initializations+1;
                let initialize=Instant::now();
                *gpu = GpuState {
                    initializations,
                    layout: f,
                    attempted: true,
                    ..Default::default()
                };
                match unsafe {
                    crate::icc_gpu::Processor::selected(&self.lut, f.width, f.height, f.bt709,shader)
                } {
                    Ok(processor) => gpu.processor = Some(processor),
                    Err(e) if crate::native::graphics_lost(&serde_json::json!({"error":e.to_string()})) => return Err(e),
                    Err(e) if shader!=crate::backend::Shader::Auto => return Err(e),
                    Err(e) => gpu.error = Some(e.to_string()),
                }
                gpu.initialize_micros=initialize.elapsed().as_micros() as u64;
            }
            if let Some(processor) = &gpu.processor {
                let apply_started=Instant::now();
                let applied=unsafe { processor.apply(bytes,picture,!self.path.as_os_str().is_empty()) };
                let elapsed=apply_started.elapsed().as_micros() as u64;
                gpu.apply_count+=1;gpu.apply_micros+=elapsed;gpu.max_apply_micros=gpu.max_apply_micros.max(elapsed);
                match applied {
                    Ok(()) => true,
                    Err(e) if crate::native::graphics_lost(&serde_json::json!({"error":e.to_string()})) => return Err(e),
                    Err(e) => {
                        gpu.processor = None;
                        gpu.error = Some(e.to_string());
                        false
                    }
                }
            } else {
                false
            }
        };
        if !applied {
            // CPU-only playback retains ICC correction but never emulates shader effects.
            if self.path.as_os_str().is_empty() {self.frames.fetch_add(1,Ordering::Relaxed);return Ok(());}
            let picture=Default::default();
            let (y, uv) = bytes[..len * 3 / 2].split_at_mut(len);
            let workers = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
                .min(8);
            let rows = (f.height / 2).div_ceil(workers);
            std::thread::scope(|scope| {
                for (y, uv) in y
                    .chunks_mut(rows * f.width * 2)
                    .zip(uv.chunks_mut(rows * f.width))
                {
                    scope.spawn(move || self.rows(y, uv, f.width, f.bt709,picture));
                }
            });
        }
        self.frames.fetch_add(1, Ordering::Relaxed);
        self.micros
            .fetch_add(start.elapsed().as_micros() as u64, Ordering::Relaxed);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Requires real GPU execution; excluded after reported driver instability"]
    fn gpu_color_transform_matches_reference_and_changes_pixels() {
        // A linear nonidentity transform makes GPU interpolation independently
        // comparable with the scalar implementation at every sample position.
        let lut = (0..GRID * GRID * GRID)
            .map(|i| {
                [
                    0.75 * (i % GRID) as f32 / (GRID - 1) as f32,
                    ((i / GRID) % GRID) as f32 / (GRID - 1) as f32,
                    0.9 * (i / GRID / GRID) as f32 / (GRID - 1) as f32,
                ]
            })
            .collect();
        let t = Transform {
            lut,
            picture: Mutex::new(Default::default()),shader:Mutex::new(Default::default()),delivered_until:AtomicI64::new(0),            layout: Mutex::new(Layout::default()),
            gpu: Mutex::new(GpuState::default()),
            path: PathBuf::from("fixture.icc"),
            sha256: String::new(),
            frames: AtomicU64::new(0),
            micros: AtomicU64::new(0),
        };
        for bt709 in [true, false] {
            let (width, height) = (64, 32);
            let mut original = vec![0u8; width * height * 3 / 2];
            for (i, v) in original[..width * height].iter_mut().enumerate() {
                *v = 40 + (i % 160) as u8;
            }
            for (i, v) in original[width * height..].iter_mut().enumerate() {
                *v = 110 + (i % 36) as u8;
            }
            let mut expected = original.clone();
            let (y, uv) = expected.split_at_mut(width * height);
            t.rows(y, uv, width, bt709,Default::default());
            let mut actual = original.clone();
            unsafe {
                let gpu = crate::icc_gpu::Processor::new(&t.lut, width, height, bt709)
                    .expect("D3D11 test adapter");
                gpu.apply(&mut actual,Default::default(),true).unwrap();
            }
            assert!(
                actual.iter().zip(&original).filter(|(a, b)| a != b).count() > width * height / 2
            );
            let error = actual
                .iter()
                .zip(&expected)
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap();
            assert!(
                error <= 1,
                "GPU/reference maximum code value difference: {error}"
            );
        }
    }
    #[test]
    fn tetrahedral_identity_is_continuous_at_cube_edges() {
        let lut = (0..GRID * GRID * GRID)
            .map(|i| {
                [
                    (i % GRID) as f32 / (GRID - 1) as f32,
                    ((i / GRID) % GRID) as f32 / (GRID - 1) as f32,
                    (i / GRID / GRID) as f32 / (GRID - 1) as f32,
                ]
            })
            .collect();
        let t = Transform {
            lut,
            picture: Mutex::new(Default::default()),shader:Mutex::new(Default::default()),delivered_until:AtomicI64::new(0),            layout: Mutex::new(Layout::default()),
            gpu: Mutex::new(GpuState::default()),
            path: PathBuf::from("fixture.icc"),
            sha256: String::new(),
            frames: AtomicU64::new(0),
            micros: AtomicU64::new(0),
        };
        for p in [
            [0., 0., 0.],
            [1., 1., 1.],
            [0.2, 0.7, 0.4],
            [0.7, 0.2, 0.4],
            [0.4, 0.2, 0.7],
            [0.2, 0.4, 0.7],
            [0.4, 0.7, 0.2],
            [0.7, 0.4, 0.2],
        ] {
            let q = t.map(p);
            for i in 0..3 {
                assert!((p[i] - q[i]).abs() < 0.00001);
            }
        }
    }
}

#[cfg(test)] mod picture_tests {
    use super::*;
    use crate::picture::Picture;
    #[test] fn neutral_without_profile_preserves_every_input_byte() {
        let t=Transform::identity();*t.layout.lock().unwrap()=Layout{width:64,height:32,bt709:true};
        let mut bytes=(0..64*32*3/2).map(|n|(n%256)as u8).collect::<Vec<_>>();let original=bytes.clone();
        t.process(&mut bytes).unwrap();assert_eq!(bytes,original);assert!(!t.gpu.lock().unwrap().attempted);
    }
    #[test]
    #[ignore = "Explicit Direct3D hardware validation"]
    fn d3d_picture_controls_match_reference_and_toggle_without_device_recreation() {
        let t=Transform::identity();let (width,height)=(64,32);
        let mut input=vec![128u8;width*height*3/2];
        for (i,v) in input[..width*height].iter_mut().enumerate(){*v=40+(i%160)as u8;}
        for (i,v) in input[width*height..].iter_mut().enumerate(){*v=110+(i%36)as u8;}
        for bt709 in [true,false] {
            let gpu=unsafe{crate::icc_gpu::Processor::new(&t.lut,width,height,bt709).unwrap()};
            let mut neutral=Vec::new();
            for picture in [Picture::default(),Picture::preset(1),Picture::preset(2),Picture::preset(3),
                Picture{saturation:0,..Default::default()},Picture{brightness:20,..Default::default()},
                Picture{contrast:130,..Default::default()},Picture{hdr_effect:true,..Default::default()},Picture::default()] {
                let mut expected=input.clone();let (y,uv)=expected.split_at_mut(width*height);
                t.rows(y,uv,width,bt709,picture);
                let mut actual=input.clone();unsafe{gpu.apply(&mut actual,picture,false).unwrap();}
                let error=actual.iter().zip(&expected).map(|(a,b)|a.abs_diff(*b)).max().unwrap();
                assert!(error<=2,"{picture:?}: GPU/CPU difference {error}");
                if picture==Picture::default() {if neutral.is_empty(){neutral=actual;}else{assert_eq!(actual,neutral);}}
                else {assert!(actual.iter().zip(&neutral).filter(|(a,b)|a!=b).count()>width*height/16,"No visible change: {picture:?}");}
            }
        }
    }    #[test]
    #[ignore = "Explicit Direct3D hardware validation"]
    fn dx12_picture_controls_match_reference_and_toggle_without_device_recreation() {
        let t=Transform::identity();let (width,height)=(64,32);
        let mut input=vec![128u8;width*height*3/2];
        for (i,v) in input[..width*height].iter_mut().enumerate(){*v=40+(i%160)as u8;}
        for (i,v) in input[width*height..].iter_mut().enumerate(){*v=110+(i%36)as u8;}
        for bt709 in [true,false] {
            let gpu=unsafe{crate::icc_gpu::Processor::selected(&t.lut,width,height,bt709,crate::backend::Shader::Dx12).unwrap()};
            let mut neutral=Vec::new();
            for picture in [Picture::default(),Picture::preset(1),Picture::preset(2),Picture::preset(3),
                Picture{saturation:0,..Default::default()},Picture{brightness:20,..Default::default()},
                Picture{contrast:130,..Default::default()},Picture{hdr_effect:true,..Default::default()},Picture::default()] {
                let mut expected=input.clone();let (y,uv)=expected.split_at_mut(width*height);
                t.rows(y,uv,width,bt709,picture);
                let mut actual=input.clone();unsafe{gpu.apply(&mut actual,picture,false).unwrap();}
                let error=actual.iter().zip(&expected).map(|(a,b)|a.abs_diff(*b)).max().unwrap();
                assert!(error<=2,"{picture:?}: GPU/CPU difference {error}");
                if picture==Picture::default() {if neutral.is_empty(){neutral=actual;}else{assert_eq!(actual,neutral);}}
                else {assert!(actual.iter().zip(&neutral).filter(|(a,b)|a!=b).count()>width*height/16,"No visible change: {picture:?}");}
            }
        }
    }
}

#[cfg(test)] mod performance_tests {
    use super::*;
    #[test]
    #[ignore = "Explicit HD performance measurement"]
    fn hd_picture_processing_cost() {
        let t=Transform::identity();let (width,height)=(1920,1080);
        let picture=crate::picture::Picture{hdr_effect:true,..crate::picture::Picture::preset(3)};
        let mut original=vec![128u8;width*height*3/2];for (i,v) in original[..width*height].iter_mut().enumerate(){*v=32+(i%190)as u8;}
        let mut data=original.clone();let started=Instant::now();
        for _ in 0..12 {
            data.copy_from_slice(&original);let (y,uv)=data.split_at_mut(width*height);
            let rows=(height/2).div_ceil(8);
            std::thread::scope(|scope|{for(y,uv)in y.chunks_mut(rows*width*2).zip(uv.chunks_mut(rows*width)){let t=&t;scope.spawn(move||t.rows(y,uv,width,true,picture));}});
        }
        println!("CPU picture-only average_ms={:.3}",started.elapsed().as_secs_f64()*1000./12.);
        let gpu=unsafe{crate::icc_gpu::Processor::new(&t.lut,width,height,true).unwrap()};
        unsafe{gpu.apply(&mut data,picture,false).unwrap();}
        let started=Instant::now();
        for _ in 0..12 {data.copy_from_slice(&original);unsafe{gpu.apply(&mut data,picture,false).unwrap();}}
        println!("Direct3D picture-only average_ms={:.3}",started.elapsed().as_secs_f64()*1000./12.);
    }
}

#[cfg(test)] mod shader_policy_tests {
    use super::*;
    #[test] fn cpu_selection_ignores_saved_picture_effects_without_touching_pixels() {
        let t=Transform::identity();t.set_shader(crate::backend::Shader::Off);t.set_picture(crate::picture::Picture{hdr_effect:true,..crate::picture::Picture::preset(3)});
        *t.layout.lock().unwrap()=Layout{width:64,height:32,bt709:true};
        let mut data=(0..64*32*3/2).map(|i|(i%256) as u8).collect::<Vec<_>>();let original=data.clone();t.process(&mut data).unwrap();assert_eq!(data,original);assert!(!t.gpu.lock().unwrap().attempted);
    }
    #[test] fn evr_caption_clock_tracks_delivery_and_stops_during_video_stalls() {
        let t=Transform::identity();assert_eq!(t.caption_position(9000),0);
        for shader in [crate::backend::Shader::Dx12,crate::backend::Shader::Dx11,crate::backend::Shader::Off] {
            t.set_shader(shader);t.delivered_until.store(3000,Ordering::Release);
            assert_eq!(t.caption_position(1000),1000); // future renderer samples never advance captions early
            assert_eq!(t.caption_position(9000),3000); // stalled video bounds an advancing audio clock
            t.delivered_until.store(9500,Ordering::Release);assert_eq!(t.caption_position(9000),9000);
            t.end_flush();assert_eq!(t.caption_position(9000),0);
        }
    }
    #[test] fn dx12_shader_is_valid() {let module=naga::front::wgsl::parse_str(include_str!("picture_compute.wgsl")).unwrap();naga::valid::Validator::new(naga::valid::ValidationFlags::all(),naga::valid::Capabilities::all()).validate(&module).unwrap();}
}
