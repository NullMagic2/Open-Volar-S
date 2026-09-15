//! Timestamped ISDB captions. Caption work never copies or decodes video frames.
use a865r_bda::VideoTransform;
use a865r_media::playback::{CaptionFrame, Control};
use std::{
    collections::VecDeque,
    ffi::c_void,
    sync::{Arc, Mutex},
};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        Media::{DirectShow::*, IReferenceClock, MediaFoundation::*},
        UI::WindowsAndMessaging::*,
    },
};
#[repr(C)]
#[derive(Default)]
struct Image {
    data: *const u8,
    length: usize,
    width: i32,
    height: i32,
    stride: i32,
    x: i32,
    y: i32,
}
extern "C" {
    fn a865r_cc_new(profile: i32) -> *mut c_void;
    fn a865r_cc_free(p: *mut c_void);
    fn a865r_cc_decode(p: *mut c_void, data: *const u8, len: usize, pts: i64) -> i32;
    fn a865r_cc_render(p: *mut c_void, pts: i64, w: i32, h: i32, image: *mut Image) -> i32;
    fn a865r_cc_flush(p: *mut c_void);
    fn a865r_cc_invalidate(p: *mut c_void);
}
struct Packet {
    pts: i64,
    bytes: Vec<u8>,
}
#[derive(Default)]
struct Clock {
    reference: Option<IReferenceClock>,
    start: i64,
    paused: Option<i64>,
}
// DirectShow reference clocks support calls from streaming threads. Access is serialized.
unsafe impl Send for Clock {}
impl Clock {
    fn position(&self) -> i64 {
        self.paused
            .unwrap_or_else(|| unsafe {
                self.reference
                    .as_ref()
                    .and_then(|c| c.GetTime().ok())
                    .unwrap_or(self.start)
                    - self.start
            })
            .max(0)
            / 10000
    }
}
#[derive(Default)]
pub struct Sink {
    queue: Mutex<VecDeque<Packet>>,
    clock: Mutex<Clock>,
    reset: std::sync::atomic::AtomicBool,
}
impl VideoTransform for Sink {
    fn accepts(&self, m: &AM_MEDIA_TYPE) -> bool {
        m.majortype == MEDIATYPE_Stream
    }
    fn configure(&self, _: &AM_MEDIA_TYPE) -> Result<()> {
        Ok(())
    }
    fn bytes_required(&self) -> usize {
        65536
    }
    fn allocator_bytes(&self) -> usize {
        65536
    }
    fn process(&self, _: &mut [u8]) -> Result<()> {
        Ok(())
    }
    fn terminal(&self) -> bool {
        true
    }
    fn receive(&self, s: &IMediaSample) -> Result<()> {
        unsafe {
            let len = s.GetActualDataLength();
            if len <= 0 || len > 65536 || len > s.GetSize() {
                return Ok(());
            }
            let p = s.GetPointer()?;
            if p.is_null() {
                return Ok(());
            }
            let (mut start, mut end) = (0, 0);
            let pts = if s.GetTime(&mut start, &mut end).is_ok() {
                start / 10000
            } else {
                self.clock.lock().unwrap().position()
            };
            let bytes = std::slice::from_raw_parts(p, len as usize).to_vec();
            let mut q = self.queue.lock().unwrap();
            if q.len() >= 128 {
                q.clear();
                self.reset.store(true, std::sync::atomic::Ordering::Release);
            }
            q.push_back(Packet { pts, bytes });
        }
        Ok(())
    }
    fn run(&self, start: i64, clock: Option<IReferenceClock>) {
        *self.clock.lock().unwrap() = Clock {
            reference: clock,
            start,
            paused: None,
        };
    }
    fn pause(&self) {
        let mut c = self.clock.lock().unwrap();
        c.paused = Some(c.position() * 10000);
    }
    fn stop(&self) {
        self.queue.lock().unwrap().clear();
        self.reset.store(true, std::sync::atomic::Ordering::Release);
    }
    fn end_flush(&self) {
        self.stop();
    }
}
pub struct Engine {
    ptr: *mut c_void,
    pub sink: Arc<Sink>,
    control: Control,
    pid: Option<u16>,
    decoded: u64,
    errors: u64,
    held: Option<i64>,
    enabled: bool,
    area: Option<[i32;4]>,
}
impl Engine {
    pub fn new(control: Control) -> Self {
        Self {
            ptr: std::ptr::null_mut(),
            sink: Arc::new(Sink::default()),
            control,
            pid: None,
            decoded: 0,
            errors: 0,
            held: None,
            enabled: false,
            area: None,
        }
    }
    pub fn select(&mut self, pid: Option<u16>, profile: u16) {
        if self.pid == pid {
            return;
        }
        unsafe {
            if !self.ptr.is_null() {
                a865r_cc_free(self.ptr);
            }
            self.ptr = if pid.is_some() {
                a865r_cc_new(profile as i32)
            } else {
                std::ptr::null_mut()
            };
        }
        self.pid = pid;
        self.sink.queue.lock().unwrap().clear();
        self.control.caption_frame(None);
        self.decoded = 0;
        self.errors = 0;
    }
    pub fn graph_position(&self)->i64 {self.sink.clock.lock().unwrap().position()}
    pub fn tick(&mut self, area: [i32; 4], paused: bool, video_position:Option<i64>) {
        let enabled = self
            .control
            .captions_enabled
            .load(std::sync::atomic::Ordering::Relaxed);
        let graph_position=self.sink.clock.lock().unwrap().position();
        let position=video_position.unwrap_or(graph_position).max(0);
        // A displayed frame is authoritative, including a step while paused.
        // EVR owns scheduling against the graph clock; retain its pause position.
        if video_position.is_some() || !paused {self.held=None;}
        else if self.held.is_none() {self.held=Some(position);}
        let render_position=self.held.unwrap_or(position);
        // Do not append read-ahead cues to the renderer until their PTS is due.
        // This also prevents future cue history from evicting the held caption.
        let packets={let mut q=self.sink.queue.lock().unwrap();let mut due=Vec::new();
            while q.front().is_some_and(|p|p.pts<=render_position) {due.push(q.pop_front().unwrap());}due};
        unsafe {
            if !self.ptr.is_null() {
                if self
                    .sink
                    .reset
                    .swap(false, std::sync::atomic::Ordering::AcqRel)
                {
                    self.held=None;
                    a865r_cc_flush(self.ptr);
                    self.control.caption_frame(None);
                }
                for p in packets {
                    match a865r_cc_decode(self.ptr, p.bytes.as_ptr(), p.bytes.len(), p.pts) {
                        2 => self.decoded += 1,
                        0 => self.errors += 1,
                        _ => {}
                    }
                }
                if enabled {
                    if !self.enabled || self.area != Some(area) {
                        a865r_cc_invalidate(self.ptr);
                    }
                    let mut im = Image::default();
                    match a865r_cc_render(
                        self.ptr,
                        render_position,
                        area[2],
                        area[3],
                        &mut im,
                    ) {
                        2 => self.control.caption_frame(copy_image(&im, area)),
                        0 | 1 => self.control.caption_frame(None),
                        _ => {}
                    }
                } else {
                    self.control.caption_frame(None);
                }
            } else {
                self.control.caption_frame(None);
            }
        }
        self.enabled = enabled;
        self.area = Some(area);
        self.control.set_caption_state(serde_json::json!({"enabled":enabled,"available":self.pid.is_some(),"pid":self.pid,"decoded":self.decoded,"errors":self.errors,"clock_ms":render_position,"graph_clock_ms":graph_position,"video_clock_ms":video_position,"queued":self.sink.queue.lock().unwrap().len(),"decoder_ready":!self.ptr.is_null()}));
    }
}
impl Drop for Engine {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                a865r_cc_free(self.ptr);
            }
        }
        self.control.caption_frame(None);
    }
}
unsafe fn copy_image(im: &Image, area: [i32; 4]) -> Option<CaptionFrame> {
    if im.data.is_null()
        || im.width <= 0
        || im.height <= 0
        || im.width > 7680
        || im.height > 4320
        || im.stride < im.width * 4
    {
        return None;
    }
    let needed = (im.stride as usize).checked_mul(im.height as usize)?;
    if needed > im.length || needed > 128 * 1024 * 1024 {
        return None;
    }
    // The adapter owns this allocation until the next call; copy only its bounded image rows.
    let src = std::slice::from_raw_parts(im.data, needed);
    let left = im.x.max(0);
    let top = im.y.max(0);
    let right = im.x.checked_add(im.width)?.min(area[2]);
    let bottom = im.y.checked_add(im.height)?.min(area[3]);
    if right <= left || bottom <= top {
        return None;
    }
    let width = right - left;
    let height = bottom - top;
    let mut bgra = Vec::with_capacity(width as usize * height as usize * 4);
    for y in top..bottom {
        let start = (y - im.y) as usize * im.stride as usize + (left - im.x) as usize * 4;
        for p in src[start..start + width as usize * 4].chunks_exact(4) {
            bgra.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
        }
    }
    Some(CaptionFrame {
        width,
        height,
        x: area[0].checked_add(left)?,
        y: area[1].checked_add(top)?,
        plane_width: area[2],
        plane_height: area[3],
        bgra: Arc::new(bgra),
    })
}
/// A per-pixel alpha child window. Input passes through to the video controls.
pub unsafe fn create(parent: HWND) -> Result<HWND> {
    CreateWindowExW(
        WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE,
        w!("STATIC"),
        w!("Closed captions"),
        WS_CHILD,
        0,
        0,
        1,
        1,
        parent,
        None,
        None,
        None,
    )
}
pub unsafe fn display(hwnd: HWND, surface: HWND, frame: Option<&CaptionFrame>) -> Result<()> {
    let Some(f) = frame else {
        let _ = ShowWindow(hwnd, SW_HIDE);
        return Ok(());
    };
    let mut origin = POINT::default();
    ClientToScreen(surface, &mut origin).ok()?;
    origin.x += f.x;
    origin.y += f.y;
    // UpdateLayeredWindow positions WS_CHILD windows in parent-client coordinates.
    // Supplying screen coordinates applies the parent's desktop offset twice.
    ScreenToClient(GetParent(hwnd)?, &mut origin).ok()?;
    let dc = CreateCompatibleDC(None);
    if dc.is_invalid() {
        return Err(Error::from_win32());
    }
    let mut info = BITMAPINFO::default();
    info.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: f.width,
        biHeight: -f.height,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        ..Default::default()
    };
    let mut bits = std::ptr::null_mut();
    let bitmap = match CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, None, 0) {
        Ok(b) => b,
        Err(e) => {
            let _ = DeleteDC(dc);
            return Err(e);
        }
    };
    std::ptr::copy_nonoverlapping(f.bgra.as_ptr(), bits.cast(), f.bgra.len());
    let old = SelectObject(dc, bitmap);
    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };
    let result = UpdateLayeredWindow(
        hwnd,
        None,
        Some(&origin),
        Some(&SIZE {
            cx: f.width,
            cy: f.height,
        }),
        dc,
        Some(&POINT::default()),
        COLORREF(0),
        Some(&blend),
        ULW_ALPHA,
    );
    SelectObject(dc, old);
    let _ = DeleteObject(bitmap);
    let _ = DeleteDC(dc);
    if result.is_ok() {
        SetWindowPos(
            hwnd,
            HWND_TOP,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )?;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn group(id: u8, body: &[u8]) -> Vec<u8> {
        let mut g = vec![id << 2, 0, 0, (body.len() >> 8) as u8, body.len() as u8];
        g.extend_from_slice(body);
        let mut crc = 0u16;
        for &b in &g {
            crc ^= (b as u16) << 8;
            for _ in 0..8 {
                crc = if crc & 0x8000 != 0 {
                    (crc << 1) ^ 0x1021
                } else {
                    crc << 1
                };
            }
        }
        g.extend_from_slice(&crc.to_be_bytes());
        let mut p = vec![0x80, 0xff, 0xf0];
        p.extend(g);
        p
    }
    fn statement(text: &[u8]) -> Vec<u8> {
        let mut unit = vec![0x1f, 0x20, 0, 0, text.len() as u8];
        unit.extend_from_slice(text);
        let mut body = vec![0, 0, 0, unit.len() as u8];
        body.extend(unit);
        group(1, &body)
    }
    #[test]
    fn long_latin_captions_fit_and_center_with_normal_spacing() {
        unsafe {
            let mut engine=Engine::new(Control::default());engine.select(Some(278),8);
            let management=group(0,&[0,1,0,b'p',b'o',b'r',0x80,0,0,0]);
            assert_ne!(a865r_cc_decode(engine.ptr,management.as_ptr(),management.len(),0),0);
            let text=statement(b"\x0c\x1c\x48\x50SABENDO O QUE OS OUTROS PENSAM DE VOCE E SEM CORTAR O FINAL DA LEGENDA");
            assert_eq!(a865r_cc_decode(engine.ptr,text.as_ptr(),text.len(),1000),2);
            for (w,h) in [(960,540),(1280,720),(640,480)] {
                a865r_cc_invalidate(engine.ptr);
                let mut im=Image::default();assert_eq!(a865r_cc_render(engine.ptr,1000,w,h,&mut im),2);
                let f=copy_image(&im,[37,19,w,h]).unwrap();
                assert!((2*(f.x-37)+f.width-w).abs()<=3,"center {} width {} in {}",f.x,f.width,w);
                assert!(f.width<=w*92/100 && f.x>=37 && f.x+f.width<=37+w);
                assert!(f.y>=19 && f.y+f.height<=19+h);
                if w==1280 {
                    if let Some(path)=std::env::var_os("A865R_CAPTION_PREVIEW") {
                        let rgba=f.bgra.chunks_exact(4).flat_map(|p|[p[2],p[1],p[0],p[3]]).collect();
                        image::RgbaImage::from_raw(f.width as u32,f.height as u32,rgba).unwrap().save(path).unwrap();
                    }
                }
            }
        }
    }
    #[test]
    fn portuguese_caption_has_timed_alpha_pixels_and_clears_on_flush() {
        unsafe {
            let mut engine = Engine::new(Control::default());
            engine.select(Some(278), 8);
            assert!(!engine.ptr.is_null());
            let management = group(0, &[0, 1, 0, b'p', b'o', b'r', 0x80, 0, 0, 0]);
            assert_ne!(
                a865r_cc_decode(engine.ptr, management.as_ptr(), management.len(), 0),
                0
            );
            let statement = statement(b"\x0c\x1c\x48\x42Legenda de teste");
            assert_eq!(
                a865r_cc_decode(engine.ptr, statement.as_ptr(), statement.len(), 1000),
                2
            );
            let mut im = Image::default();
            assert_eq!(a865r_cc_render(engine.ptr, 500, 960, 540, &mut im), 1);
            assert_eq!(a865r_cc_render(engine.ptr, 1000, 960, 540, &mut im), 2);
            let frame = copy_image(&im, [0, 0, 960, 540]).unwrap();
            assert!(frame.bgra.chunks_exact(4).any(|p| p[3] > 0));
            assert!(frame.bgra.chunks_exact(4).any(|p| p[3] < 255));
            assert!(
                frame.x >= 0
                    && frame.y >= 0
                    && frame.x + frame.width <= 960
                    && frame.y + frame.height <= 540,
                "rect {},{} {},{}",
                frame.x,
                frame.y,
                frame.width,
                frame.height
            );
            assert_eq!(a865r_cc_render(engine.ptr, 1000, 960, 540, &mut im), 3);
            a865r_cc_invalidate(engine.ptr);
            assert_eq!(a865r_cc_render(engine.ptr, 1000, 960, 540, &mut im), 2);
            assert_eq!(a865r_cc_render(engine.ptr, 1000, 1280, 720, &mut im), 2);
            a865r_cc_flush(engine.ptr);
            assert_eq!(a865r_cc_render(engine.ptr, 1000, 960, 540, &mut im), 1);
            for n in 0..statement.len() {
                a865r_cc_decode(engine.ptr, statement.as_ptr(), n, 1000);
            }
        }
    }
    #[test]
    fn cues_follow_displayed_video_through_pause_steps_and_read_ahead() {
        let control=Control::default();control.captions_enabled.store(true,std::sync::atomic::Ordering::Relaxed);
        let mut e=Engine::new(control.clone());e.select(Some(278),8);
        // The graph has read far ahead, while the video still shows the first cue.
        e.sink.clock.lock().unwrap().paused=Some(9000*10000);
        {let mut q=e.sink.queue.lock().unwrap();
         q.push_back(Packet{pts:0,bytes:group(0,&[0,1,0,b'p',b'o',b'r',0x80,0,0,0])});
         q.push_back(Packet{pts:1000,bytes:statement(b"\x0c\x1c\x48\x42Primeira legenda")});
         q.push_back(Packet{pts:3000,bytes:statement(b"\x0c\x1c\x48\x42Segunda legenda")});
         q.push_back(Packet{pts:5000,bytes:statement(b"\x0c")});}
        e.tick([0,0,960,540],false,Some(999));assert!(control.captions.lock().unwrap().frame.is_none());
        e.tick([0,0,960,540],false,Some(1000));
        let first=control.captions.lock().unwrap().frame.as_ref().unwrap().bgra.clone();
        assert_eq!(e.sink.queue.lock().unwrap().len(),2);
        e.tick([0,0,960,540],true,Some(1000));assert_eq!(control.captions.lock().unwrap().frame.as_ref().unwrap().bgra,first);
        e.tick([0,0,960,540],true,Some(3000));let second=control.captions.lock().unwrap().frame.as_ref().unwrap().bgra.clone();assert_ne!(first,second);
        // Buffered reverse stepping restores the earlier cue, including original duration.
        e.tick([0,0,960,540],true,Some(2000));assert_eq!(control.captions.lock().unwrap().frame.as_ref().unwrap().bgra,first);
        e.tick([0,0,960,540],false,Some(4999));assert_eq!(control.captions.lock().unwrap().frame.as_ref().unwrap().bgra,second);
        e.tick([0,0,960,540],false,Some(5000));assert!(control.captions.lock().unwrap().frame.is_none());
    }
    #[test]
    fn overlay_uses_video_coordinates_after_window_moves() {
        unsafe {
            let parent=CreateWindowExW(WINDOW_EX_STYLE::default(),w!("STATIC"),w!("Caption geometry test"),WS_POPUP,317,211,1280,800,None,None,None,None).unwrap();
            let surface=CreateWindowExW(WINDOW_EX_STYLE::default(),w!("STATIC"),w!("Video fixture"),WS_CHILD,23,51,1200,675,parent,None,None,None).unwrap();
            let overlay=create(surface).unwrap();
            let f=CaptionFrame{width:400,height:48,x:400,y:580,plane_width:1200,plane_height:675,bgra:Arc::new(vec![255;400*48*4])};
            for (x,y) in [(317,211),(-420,97),(15,42)] {
                SetWindowPos(parent,None,x,y,0,0,SWP_NOSIZE|SWP_NOACTIVATE).unwrap();
                display(overlay,surface,Some(&f)).unwrap();
                let mut expected=POINT{x:f.x,y:f.y};ClientToScreen(surface,&mut expected).ok().unwrap();
                let mut actual=RECT::default();GetWindowRect(overlay,&mut actual).unwrap();
                let matches=actual.left==expected.x && actual.top==expected.y;
                if !matches {let _=DestroyWindow(parent);}
                assert!(matches,"caption origin {},{}; expected {},{}",actual.left,actual.top,expected.x,expected.y);
            }
            DestroyWindow(parent).unwrap();
        }
    }

}
