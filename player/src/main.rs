#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Native windows-rs frontend. The original driver/BDA release remains 0.7.0.
mod startup;
mod app_icon;
mod audio_health;
mod audio;
mod captions;
mod recordings;
mod recording_finalize;
mod snapshots;
mod parental;
mod picture;
mod display_hdr;
mod aspect;
mod osd;
mod dial;
mod orbit;
mod i18n;
mod scrollbars;
mod viewer;
mod window_placement;
use aspect::AspectRatio;
mod canvas;
mod frame_pool;
mod gpu_work;
mod countries;
mod epg_source;
mod guide;
mod vkdecode;
mod icc;
mod icc_gpu;
mod icc_dx12;
use backend::SHADER_CONTROL;
mod native;
mod backend;
mod capture_only;
mod pacing;
mod timeline;
mod vulkan;
use a865r::api::{ColorProfile, DeinterlaceMode, Resolution};
use a865r_media::{
    playback::{Control, Options},
    television::{self, Action},
};
use serde_json::{json, Value};
use std::{
    cell::RefCell,
    fs,
    path::PathBuf,
    sync::mpsc,
    time::{SystemTime, UNIX_EPOCH},
};
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::{LibraryLoader::GetModuleHandleW, SystemInformation::GetLocalTime},
        UI::{
            Controls::Dialogs::*, Controls::*, Shell::{SetWindowSubclass, DefSubclassProc}, HiDpi::*, Input::KeyboardAndMouse::*,
            WindowsAndMessaging::*,
        },
    },
};

const PLAY: u16 = 101;
const STOP: u16 = 102;
const RECORD: u16 = 103;
const OPEN: u16 = 104;
const PREV: u16 = 105;
const NEXT: u16 = 106;
const VOL_DOWN: u16 = 107;
const VOL_UP: u16 = 108;
const SETTINGS: u16 = 109;
const SCAN: u16 = 110;
const SNAP: u16 = 111;
const DECK: u16 = 112;
const CLOSE: u16 = 113;
const MIN: u16 = 114;
const FULL: u16 = 115;
const CHANNEL: u16 = 116;
const TBM_GETPOS: u32 = WM_USER;
const SEEK: u16 = 117;
const SEEK_TIME: u16 = 118;
const EPG: u16 = 119;
const BACK: u16 = 120;
const FORWARD: u16 = 121;
const LIVE: u16 = 122;
const AUDIO: u16 = 123;
const CC:u16=124;
const RECORD_FOLDER:u16=125;
const OPEN_RECORD_FOLDER:u32=WM_APP+22;
const RECORD_PATH:u16=314;
const SNAPSHOT_FOLDER:u16=126;
const SNAPSHOT_PATH:u16=315;
const STORAGE_STATUS:u16=316;
const SIGNAL_STATUS:u16=364;
const COMMIT_CHANNEL_SELECTION:u32=WM_APP+23;
const AUDIO_MODE_BASE:u16=600;
const AUDIO_TRACK_BASE:u16=700;
const OPEN_AUDIO:u32=WM_APP+21;
const OPEN_EPG: u32 = WM_APP + 19;
const WORK_COMPLETE: u32 = WM_APP + 20;
const RESOLUTIONS: [Resolution; 3] = [Resolution::Native, Resolution::Qhd, Resolution::Uhd];
const ASPECT: u16 = 313;
const DIAL: u16 = 503;
const DIAL_TIMER: usize = 2;
const SIGNAL_STATUS_TIMER:usize=3;
const CHANNEL_OSD: u16 = 501;
const VOLUME_OSD: u16 = 502;
const PROFILE: u16 = 202;
const FFMPEG: u16 = 203;
const EXPORTS: u16 = 204;
const GOLD: u32 = 0x00b9dcef;
const CHROME_TOP: [u8; 3] = [10, 5, 2];
const CHROME_BOTTOM: [u8; 3] = [107, 78, 36];
thread_local! { static APP:RefCell<Option<App>>=const{RefCell::new(None)}; }
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
// Tell the Shell explicitly; a borderless monitor-sized window alone can leave
// the taskbar above the player. This does not make the player always-on-top.
unsafe fn mark_fullscreen(hwnd: HWND, fullscreen: bool) {
    use windows::Win32::{System::Com::*, UI::Shell::{ITaskbarList2, TaskbarList}};
    let initialized = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
    if let Ok(taskbar) = CoCreateInstance::<_, ITaskbarList2>(&TaskbarList, None, CLSCTX_INPROC_SERVER) {
        if taskbar.HrInit().is_ok() {
            let _ = taskbar.MarkFullscreenWindow(hwnd, fullscreen);
        }
    }
    if initialized { CoUninitialize(); }
}
fn millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
fn data_dir() -> PathBuf {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|s| s == "--profile-dir" || s == "--verify-output") {
        if let Some(p) = args.get(i + 1) {
            return PathBuf::from(p);
        }
    }
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("A865R/TV")
}

#[derive(Clone)]
enum Job {
    File(PathBuf),
    Watch,
    Record,
    Scan(Vec<u32>),
}
struct App {
    video: HWND,
    deck: HWND,
    surface: HWND,
    surface_backend: Option<(backend::Backend, backend::Shader)>,
    settings: HWND,
    font: HFONT,
    skin: orbit::Skin,
    viewer: viewer::Skin,
    material: orbit::Material,
    settings_tab: usize,
    receiver_startup:bool,
    avertv_signal:usize,
    picture: picture::Picture,
    video_hdr:bool,
    parental_edit:bool,
    parental_draft:Value,
    options: Options,
    backend: backend::Backend,
    shader: backend::Shader,
    frequency: u32,
    channels: Vec<u32>,
    services: Vec<Value>,
    channel_index: usize,
    volume: u32,
    audio_mode: audio::Mode,
    captions_enabled:bool,
    caption_window:HWND,
    caption_revision:u64,
    recording_folder:PathBuf,
    snapshot_folder:PathBuf,
    pending_snapshot:Option<(PathBuf,std::time::Instant)>,
    pending_channel_selection:Option<(HWND,usize)>,
    recording_path:Option<PathBuf>,
    dial: RefCell<dial::Dial>,
    control: Control,
    rx: Option<mpsc::Receiver<Value>>,
    current: Option<Job>,
    pending: Option<Job>,
    status: String,
    folder: PathBuf,
    pipe: String,
    native_commands: Option<mpsc::Sender<native::Command>>,
    paused: bool,
    dragging: bool,
    paint_stamp: String,
    wheel_delta: i32,
    video_aspect: (u32, u32),
    aspect_ratio: AspectRatio,
    channel_osd_until: Option<std::time::Instant>,
    channel_digits: String,
    channel_entry_until: Option<std::time::Instant>,
    volume_osd_until: Option<std::time::Instant>,
    osd_on_first_frame: bool,
    countries: Vec<Value>,
    country: usize,
    verification: Option<(std::time::Instant, u8, PathBuf)>,
    closing: bool,
    quality: Option<u8>,
    fullscreen: bool,
    fullscreen_button_until: Option<std::time::Instant>,
    fullscreen_pointer: Option<(i32,i32)>,
    fullscreen_cursor_until: Option<std::time::Instant>,
    restore: RECT,
    restore_style: isize,
    restore_maximized: bool,
    restore_deck: bool,
}

unsafe fn child(parent: HWND, class: PCWSTR, label: &str, id: u16, style: u32) -> HWND {
    let name = wide(&i18n::text(label));
    let hwnd=CreateWindowExW(
        WINDOW_EX_STYLE(0),
        class,
        PCWSTR(name.as_ptr()),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(style),
        0,
        0,
        10,
        10,
        parent,
        HMENU(id as usize as _),
        GetModuleHandleW(None).unwrap(),
        None,
    )
    .unwrap();
    if class.to_string().unwrap_or_default()=="COMBOBOX" && ![CHANNEL,306,parental::CHANNEL,i18n::LANGUAGE].contains(&id){let _=SetWindowSubclass(hwnd,Some(i18n::combo_proc),29,0);}
    if !matches!(class.to_string().unwrap_or_default().as_str(),"EDIT"|"COMBOBOX"){i18n::remember(hwnd,label);}hwnd
}
// Opening Channels should display the saved profile without selecting its edit text.
unsafe fn clear_country_selection(settings: HWND) {
    let mut info=COMBOBOXINFO { cbSize: std::mem::size_of::<COMBOBOXINFO>() as u32, ..Default::default() };
    if GetComboBoxInfo(item(settings,311),&mut info).is_ok() && !info.hwndItem.0.is_null() {
        SendMessageW(info.hwndItem,EM_SETSEL,WPARAM(usize::MAX),LPARAM(0));
    }
}
unsafe fn button(parent: HWND, label: &str, id: u16) -> HWND {
    child(parent, w!("BUTTON"), label, id, BS_OWNERDRAW as u32)
}
unsafe fn item(parent: HWND, id: u16) -> HWND {
    GetDlgItem(parent, id as i32).unwrap_or_default()
}
unsafe fn place(parent: HWND, id: u16, x: i32, y: i32, width: i32, height: i32) {
    let _ = MoveWindow(item(parent, id), x, y, width.max(1), height.max(1), true);
}
unsafe fn set_text(hwnd: HWND, text: &str) {
    // Editable data, passwords and broadcaster descriptions are not interface strings.
    let mut class=[0u16;32];let n=GetClassNameW(hwnd,&mut class);let editable=matches!(String::from_utf16_lossy(&class[..n as usize]).to_ascii_lowercase().as_str(),"edit"|"combobox") || GetDlgCtrlID(hwnd)==CHANNEL_OSD as i32;
    if editable{set_data_text(hwnd,text);return;}
    i18n::remember(hwnd,text);set_data_text(hwnd,&i18n::text(text));
}
unsafe fn set_data_text(hwnd:HWND,text:&str){if text_of(hwnd)==text{return;}let s=wide(text);let _=SetWindowTextW(hwnd,PCWSTR(s.as_ptr()));}
unsafe fn text_of(hwnd: HWND) -> String {
    let mut b = [0u16; 1024];
    let n = GetWindowTextW(hwnd, &mut b);
    String::from_utf16_lossy(&b[..n as usize])
}
unsafe fn combo_add(hwnd: HWND, label: &str) {
    let s = wide(label);
    SendMessageW(hwnd, CB_ADDSTRING, WPARAM(0), LPARAM(s.as_ptr() as isize));
}
unsafe fn select(hwnd: HWND, n: usize) {
    SendMessageW(hwnd, CB_SETCURSEL, WPARAM(n), LPARAM(0));
}
unsafe fn selected(hwnd: HWND) -> usize {
    SendMessageW(hwnd, CB_GETCURSEL, WPARAM(0), LPARAM(0))
        .0
        .max(0) as usize
}
unsafe fn fill(dc: HDC, r: RECT, color: u32) {
    let b = CreateSolidBrush(COLORREF(color));
    FillRect(dc, &r, b);
    let _ = DeleteObject(b);
}
unsafe fn gradient(dc: HDC, r: RECT, top: [u8; 3], bottom: [u8; 3]) {
    if r.right<=r.left || r.bottom<=r.top {return;}
    let vertex=|x,y,c:[u8;3]|TRIVERTEX{x,y,Red:(c[0] as u16)<<8,
        Green:(c[1] as u16)<<8,Blue:(c[2] as u16)<<8,Alpha:0};
    let vertices=[vertex(r.left,r.top,top),vertex(r.right,r.bottom,bottom)];
    let mesh=GRADIENT_RECT{UpperLeft:0,LowerRight:1};
    // One native operation replaces a heap allocation, brush and draw per row.
    if GradientFill(dc,&vertices,(&mesh as *const GRADIENT_RECT).cast(),1,GRADIENT_FILL_RECT_V).as_bool(){return;}
    let h = (r.bottom - r.top).max(1);
    for y in 0..h {
        let c: Vec<u32> = (0..3)
            .map(|i| (top[i] as i32 + (bottom[i] as i32 - top[i] as i32) * y / h) as u32)
            .collect();
        fill(
            dc,
            RECT {
                top: r.top + y,
                bottom: r.top + y + 1,
                ..r
            },
            c[0] | c[1] << 8 | c[2] << 16,
        );
    }
}
/// Paint the portion of the parent gradient behind a child, without borrowing
/// App during synchronous child paint notifications or reading stale pixels.
unsafe fn chrome_background(dc: HDC, parent: HWND, child: HWND, r: RECT) {
    let themed=APP.with(|a|a.try_borrow().ok().is_some_and(|a|a.as_ref().is_some_and(|a|{
        if parent==a.settings && (340..=360).contains(&(GetDlgCtrlID(child) as u16)) {fill(dc,r,orbit::rgb(28,20,14));true}else if parent==a.video {a.viewer.child_background(dc,parent,child);true}else if parent==a.deck || parent==a.settings {a.skin.child_background(dc,parent,child);true}else{false}
    })));
    if themed{return;}
    let mut origin = POINT::default();
    let _ = ClientToScreen(child, &mut origin);
    let _ = ScreenToClient(parent, &mut origin);
    let height = client(parent).bottom.max(1);
    let saved=SaveDC(dc);
    if saved!=0 {
        let _=IntersectClipRect(dc,r.left,r.top,r.right,r.bottom);
        // Use the same parent-coordinate gradient so transparent controls match.
        gradient(dc,RECT{left:r.left,right:r.right,top:-origin.y,bottom:height-origin.y},CHROME_TOP,CHROME_BOTTOM);
        let _=RestoreDC(dc,saved);
        return;
    }
    for y in r.top..r.bottom {
        let row = (origin.y + y).clamp(0, height - 1);
        let c: [u32; 3] = std::array::from_fn(|i| {
            (CHROME_TOP[i] as i32 + (CHROME_BOTTOM[i] as i32 - CHROME_TOP[i] as i32) * row / height)
                as u32
        });
        fill(
            dc,
            RECT {
                top: y,
                bottom: y + 1,
                ..r
            },
            c[0] | c[1] << 8 | c[2] << 16,
        );
    }
}
unsafe fn buffered_control(dc: HDC, r: RECT, paint: impl FnOnce(HDC)) {
    if r.right <= 0 || r.bottom <= 0 {
        return;
    }
    let buffer = CreateCompatibleDC(dc);
    let bitmap = CreateCompatibleBitmap(dc, r.right, r.bottom);
    if buffer.is_invalid() || bitmap.is_invalid() {
        if !buffer.is_invalid() {
            let _ = DeleteDC(buffer);
        }
        if !bitmap.is_invalid() {
            let _ = DeleteObject(bitmap);
        }
        paint(dc);
        return;
    }
    let old = SelectObject(buffer, bitmap);
    paint(buffer);
    let _ = BitBlt(
        dc,
        r.left,
        r.top,
        r.right - r.left,
        r.bottom - r.top,
        buffer,
        r.left,
        r.top,
        SRCCOPY,
    );
    SelectObject(buffer, old);
    let _ = DeleteObject(bitmap);
    let _ = DeleteDC(buffer);
}
unsafe fn line(dc: HDC, x: i32, y: i32, x2: i32, y2: i32, color: u32, width: i32) {
    let p = CreatePen(PS_SOLID, width, COLORREF(color));
    let old = SelectObject(dc, p);
    let _ = MoveToEx(dc, x, y, None);
    let _ = LineTo(dc, x2, y2);
    SelectObject(dc, old);
    let _ = DeleteObject(p);
}
fn aspect_window_size(proposed:(i32,i32),vertical:bool,dpi:f32,
    _frame:(i32,i32),aspect:(u32,u32))->(i32,i32) {
    viewer::aspect_size(proposed,vertical,dpi,aspect)
}

unsafe fn label(dc:HDC,r:RECT,s:&str,size:i32,color:u32,center:bool){label_raw(dc,r,&i18n::text(s),size,color,center);}
unsafe fn label_raw(dc:HDC,r:RECT,s:&str,size:i32,color:u32,center:bool){label_aligned(dc,r,s,size,color,if center{DT_CENTER}else{DT_LEFT});}
unsafe fn label_aligned(dc: HDC, r: RECT, s: &str, size: i32, color: u32, alignment:DRAW_TEXT_FORMAT) {
    if s.is_empty() || r.right <= r.left || r.bottom <= r.top {
        return;
    }
    let f = CreateFontW(
        -size,
        0,
        0,
        0,
        400,
        0,
        0,
        0,
        DEFAULT_CHARSET.0 as u32,
        OUT_DEFAULT_PRECIS.0 as u32,
        CLIP_DEFAULT_PRECIS.0 as u32,
        CLEARTYPE_QUALITY.0 as u32,
        DEFAULT_PITCH.0 as u32,
        w!("Segoe UI"),
    );
    let old = SelectObject(dc, f);
    SetBkMode(dc, TRANSPARENT);
    SetTextColor(dc, COLORREF(color));
    let mut v: Vec<u16> = s.encode_utf16().collect();
    let mut r = r;
    DrawTextW(
        dc,
        &mut v,
        &mut r,
        DT_SINGLELINE
            | DT_VCENTER
            | DT_END_ELLIPSIS
            | DT_NOPREFIX
            | alignment,
    );
    SelectObject(dc, old);
    let _ = DeleteObject(f);
}
unsafe fn client(hwnd: HWND) -> RECT {
    let mut r = RECT::default();
    let _ = GetClientRect(hwnd, &mut r);
    r
}
unsafe fn pick(hwnd: HWND, filter: &str) -> Option<PathBuf> {
    let f: Vec<u16> = filter.encode_utf16().chain(Some(0)).collect();
    let mut path = [0u16; 32768];
    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd,
        lpstrFilter: PCWSTR(f.as_ptr()),
        lpstrFile: windows::core::PWSTR(path.as_mut_ptr()),
        nMaxFile: path.len() as u32,
        Flags: OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
        ..Default::default()
    };
    if GetOpenFileNameW(&mut ofn).as_bool() {
        Some(PathBuf::from(String::from_utf16_lossy(
            &path[..path.iter().position(|x| *x == 0).unwrap_or(0)],
        )))
    } else {
        None
    }
}

fn channel_number(service: Option<&Value>, index: usize) -> u64 {
    service.and_then(|s| s["channel_number"].as_u64()).unwrap_or(index as u64 + 1)
}
fn channel_match(services: &[Value], count: usize, digits: &str) -> Option<usize> {
    let number = digits.parse::<u64>().ok()?;
    (0..count).find(|&i| channel_number(services.get(i), i) == number)
}
fn channel_label(service: Option<&Value>, index: usize) -> String {
    let name = service.and_then(|s| s["name"].as_str()).unwrap_or("TV");
    let number = channel_number(service, index);
    format!("{number} – {name}")
}
impl App {
    fn current_programme(&self)->Option<String>{
        if matches!(self.current,Some(Job::File(_)|Job::Scan(_))){return None;}
        let program=self.services.get(self.channel_index)?["program_id"].as_u64()? as u32;
        epg_source::current_title(&self.control.snapshot()["epg"],self.frequency,program)
    }
    fn channel_osd_text(&self) -> String {
        let name = channel_label(self.services.get(self.channel_index), self.channel_index);
        let quality = self.quality.map(|q| format!("{}%", q.min(100))).unwrap_or_else(|| "—".into());
        match self.current_programme(){Some(title)=>format!("{name}\n{title}\n{}",i18n::text(&format!("Signal quality: {quality}"))),None=>format!("{name}\n{}",i18n::text(&format!("Signal quality: {quality}")))}
    }
    unsafe fn entry_osd(&mut self, text: &str) {
        set_text(item(self.video,CHANNEL_OSD),text);
        self.channel_osd_until=Some(std::time::Instant::now()+std::time::Duration::from_secs(3));
        self.layout_osd();
        let _=ShowWindow(item(self.video,CHANNEL_OSD),SW_SHOWNA);
        let _=InvalidateRect(item(self.video,CHANNEL_OSD),None,false);
    }
    unsafe fn commit_channel_entry(&mut self) {
        self.channel_entry_until=None;
        let digits=std::mem::take(&mut self.channel_digits);
        if digits.is_empty() {return;}
        let Some(index)=channel_match(&self.services,self.channels.len(),&digits) else {
            self.entry_osd(&format!("{}\n{}",i18n::text(&format!("Channel {digits}")),i18n::text("Channel not found")));
            return;
        };
        if matches!(self.current,Some(Job::Record|Job::Scan(_))) {
            self.entry_osd(&format!("{}\n{}",i18n::text("Channel selection"),i18n::text("Stop recording or scanning first")));
            return;
        }
        self.channel_index=index;
        self.frequency=self.channels[index];
        self.quality=None;
        self.update_channels();
        self.save();
        self.show_osd(false);
        if !std::env::args().any(|s|s=="--ui-preview") {self.start(Job::Watch);}
    }
    unsafe fn channel_key(&mut self, key: usize, repeated: bool) -> bool {
        let digit=match key {0x30..=0x39=>Some((key-0x30) as u8),0x60..=0x69=>Some((key-0x60) as u8),_=>None};
        if let Some(digit)=digit {
            if repeated {return true;}
            if self.channel_digits.len()>=4 {self.channel_digits.clear();}
            self.channel_digits.push(char::from(b'0'+digit));
        } else if !self.channel_digits.is_empty() {
            match key {
                0x0d=>{self.commit_channel_entry();return true;}
                0x1b=>{self.channel_digits.clear();self.channel_entry_until=None;self.show_osd(false);return true;}
                0x08=>{self.channel_digits.pop();}
                _=>return false,
            }
        } else {return false;}
        if self.channel_digits.is_empty() {
            self.channel_entry_until=None;self.show_osd(false);
        } else {
            self.channel_entry_until=Some(std::time::Instant::now()+std::time::Duration::from_millis(1200));
            self.entry_osd(&format!("Channel {}",self.channel_digits));
        }
        true
    }
    unsafe fn fullscreen_pointer_activity(&mut self, point: POINT) {
        let position=(point.x,point.y);
        if self.fullscreen && self.fullscreen_pointer!=Some(position) {
            self.fullscreen_pointer=Some(position);
            self.fullscreen_cursor_until=Some(std::time::Instant::now()+std::time::Duration::from_secs(3));
            SetCursor(LoadCursorW(None,IDC_ARROW).unwrap_or_default());
            self.fullscreen_button_until=Some(std::time::Instant::now()+std::time::Duration::from_secs(2));
            if !IsWindowVisible(item(self.video,FULL)).as_bool() { let _=ShowWindow(item(self.video,FULL),SW_SHOWNA); }
        }
    }
    unsafe fn show_osd(&mut self, volume: bool) {
        let id = if volume { VOLUME_OSD } else { CHANNEL_OSD };
        let text = if volume { self.volume.to_string() } else { self.channel_osd_text() };
        set_text(item(self.video,id), &text);
        let deadline=Some(std::time::Instant::now()+std::time::Duration::from_secs(3));
        if volume { self.volume_osd_until=deadline; } else { self.channel_osd_until=deadline; }
        self.layout_osd();
        let _=ShowWindow(item(self.video,id),SW_SHOWNA);
        let _=InvalidateRect(item(self.video,id),None,false);
    }
    unsafe fn layout_osd(&self) {
        let d=GetDpiForWindow(self.video) as i32;
        let px=|n:i32|n*d/96;
        let mut r=RECT::default();let _=GetWindowRect(self.surface,&mut r);
        let mut points=[POINT{x:r.left,y:r.top},POINT{x:r.right,y:r.bottom}];
        MapWindowPoints(HWND::default(),self.video,&mut points);
        let margin=px(24);let width=(points[1].x-points[0].x-2*margin).max(1);
        for (id,w,h,y) in [(CHANNEL_OSD,px(640),px(if text_of(item(self.video,CHANNEL_OSD)).lines().count()>2{104}else{76}),points[0].y+margin),
            (VOLUME_OSD,px(320),px(68),points[1].y-margin-px(68))] {
            let _=SetWindowPos(item(self.video,id),HWND_TOP,points[0].x+margin,y,w.min(width),h,SWP_NOACTIVATE);
        }
    }

    unsafe fn country_fields(&self) {
        let profile = &self.countries[self.country];
        for (id, key) in [(308, "first"), (309, "last"), (310, "step")] {
            set_text(
                item(self.settings, id),
                &format!("{:.3}", profile[key].as_u64().unwrap_or(0) as f64 / 1000.),
            );
        }
        let supported = profile["supported"] == true;
        let _ = EnableWindow(item(self.settings, SCAN), supported);
        set_text(
            item(self.settings, 307),
            &countries::scan(profile).err().unwrap_or_default(),
        );
    }
    fn save_countries(&self) {
        let _ = fs::write(
            data_dir().join("countries.json"),
            json!({"selected":self.country,"profiles":self.countries}).to_string(),
        );
    }
    unsafe fn merge_channels(&mut self, found: &[Value]) {
        if found.is_empty() {
            return;
        }
        let selected = self.services.get(self.channel_index).cloned();
        let mut updated = self.services.clone();
        for service in found {
            if service["program_id"].as_u64().is_none()
                || service["frequency_khz"].as_u64().is_none()
            {
                continue;
            }
            if let Some(old) = updated.iter_mut().find(|s| {
                s["program_id"] == service["program_id"]
                    && s["frequency_khz"] == service["frequency_khz"]
            }) {
                let number = old["channel_number"].clone();
                *old = service.clone();
                if old["channel_number"].is_null() {
                    old["channel_number"] = number;
                }
            } else {
                updated.push(service.clone());
            }
        }
        updated.retain(|s| s["program_id"].is_number() && s["frequency_khz"].is_number());
        updated.sort_by_key(|s| {
            (
                s["channel_number"].as_u64().unwrap_or(999),
                s["frequency_khz"].as_u64(),
                s["program_id"].as_u64(),
            )
        });
        if updated != self.services {
            self.services = updated;
            self.channels = self
                .services
                .iter()
                .filter_map(|s| s["frequency_khz"].as_u64().map(|v| v as u32))
                .collect();
            self.channel_index = selected
                .and_then(|v| {
                    self.services.iter().position(|s| {
                        s["program_id"] == v["program_id"]
                            && s["frequency_khz"] == v["frequency_khz"]
                    })
                })
                .or_else(|| self.channels.iter().position(|f| *f == self.frequency))
                .unwrap_or(0);
            if let Some(f) = self.channels.get(self.channel_index) {
                self.frequency = *f;
            }
            self.update_channels();
            self.save();
        }
    }

    unsafe fn layout(&self) {
        self.layout_video();
        self.layout_deck();
        self.layout_settings();
    }
    unsafe fn layout_window(&self, hwnd: HWND) {
        if IsIconic(hwnd).as_bool() { return; }
        if hwnd == self.video { self.layout_video(); }
        else if hwnd == self.deck { self.layout_deck(); }
        else if hwnd == self.settings { self.layout_settings(); }
    }
    unsafe fn layout_video(&self) {
        if IsIconic(self.video).as_bool() { return; }
        let r = client(self.video);
        let d = GetDpiForWindow(self.video) as f32 / 96.;
        let px = |v: f32| (v * d) as i32;
        let width = r.right;
        for &id in viewer::CONTROLS.iter().chain([SEEK,SEEK_TIME].iter()) {
            if id!=FULL {let _=ShowWindow(item(self.video,id),if self.fullscreen {SW_HIDE}else{SW_SHOWNA});}
        }
        for id in [SETTINGS,OPEN,CC] {let _=ShowWindow(item(self.video,id),SW_HIDE);}
        if self.fullscreen || IsZoomed(self.video).as_bool() {
            let _=SetWindowRgn(self.video,None,false);
        } else {
            let diameter=(40.*width as f32/1600.).round() as i32;
            let region=CreateRoundRectRgn(0,0,r.right+1,r.bottom+1,diameter,diameter);
            if SetWindowRgn(self.video,region,false)==0 {let _=DeleteObject(region);}
        }
        if self.fullscreen {
            // One physical pixel keeps the Vulkan surface below monitor size.
            // This retains display-vblank pacing and avoids the measured
            // monitor-sized acquisition stall; the outer window stays fullscreen.
            let _ = MoveWindow(self.surface, 1, 1, (r.right-2).max(1), (r.bottom-2).max(1), true);
            place(
                self.video,
                FULL,
                r.right - px(44.),
                px(8.),
                px(32.),
                px(28.),
            );
            let _ = SetWindowPos(
                item(self.video, FULL),
                HWND_TOP,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
            if let Some(tx) = &self.native_commands {
                let _ = tx.send(native::Command::Resize);
            }
            self.layout_osd();
            return;
        }
        let layout=viewer::Layout::new(width,r.bottom);
        let v=layout.surface;
        let mut positions = vec![(self.surface, v)];
        positions.extend(layout.buttons.into_iter().map(|(id, rect)| (item(self.video, id), rect)));
        positions.push((item(self.video, SEEK), layout.seek));
        positions.push((item(self.video, SEEK_TIME), layout.time));
        viewer::position_children(&positions);
        if let Some(tx) = &self.native_commands {
            let _ = tx.send(native::Command::Resize);
        }
        self.layout_osd();
        {let mut c=self.control.captions.lock().unwrap();c.revision=c.revision.wrapping_add(1);}
        let _ = InvalidateRect(self.video, None, false);
    }
    unsafe fn layout_deck(&self) {
        let r=client(self.deck);
        for &(id,x,y,w) in orbit::BUTTONS {
            let y=y-30.;
            let rr=orbit::rect(r,x,y,w,46.);
            place(self.deck,id,rr.left,rr.top,rr.right-rr.left,rr.bottom-rr.top);
        }
        self.skin.window_shape(self.deck,r);
        let rr=orbit::dial_rect(r);
        place(self.deck,DIAL,rr.left,rr.top,rr.right-rr.left,rr.bottom-rr.top);
        let rr=orbit::rect(r,84.,104.,664.,108.);
        let combo=item(self.deck,CHANNEL);
        SendMessageW(combo,CB_SETITEMHEIGHT,WPARAM(usize::MAX),LPARAM((rr.bottom-rr.top-6).max(24) as isize));
        SendMessageW(combo,CB_SETITEMHEIGHT,WPARAM(0),LPARAM((30*r.bottom/547).max(22) as isize));
        place(self.deck,CHANNEL,rr.left,rr.top,rr.right-rr.left,(r.bottom*2/3).max(200));
        for (id,x) in [(MIN,1545.),(CLOSE,1583.)] {
            let rr=orbit::rect(r,x,24.,42.,36.);
            place(self.deck,id,rr.left,rr.top,rr.right-rr.left,rr.bottom-rr.top);
        }
        let _=ShowWindow(item(self.deck,SCAN),SW_HIDE);
        // Keep the shared command handle, but omit Live from the receiver row.
        let _=ShowWindow(item(self.deck,LIVE),SW_HIDE);
        let _ = InvalidateRect(self.deck, None, false);
    }
    unsafe fn update_channels(&self) {
        let labels:Vec<String>=(0..self.channels.len()).map(|i|channel_label(self.services.get(i),i)).collect();
        for combo in [item(self.deck,CHANNEL),item(self.settings,306)] {
            // Do not reset native tracking while the user is choosing an item.
            // CBN_CLOSEUP schedules a refresh for any metadata received meanwhile.
            if SendMessageW(combo,CB_GETDROPPEDSTATE,WPARAM(0),LPARAM(0)).0!=0 {continue;}
            let count=SendMessageW(combo,CB_GETCOUNT,WPARAM(0),LPARAM(0)).0;
            let unchanged=count==labels.len() as isize && labels.iter().enumerate().all(|(i,label)| {
                let n=SendMessageW(combo,CB_GETLBTEXTLEN,WPARAM(i),LPARAM(0)).0;
                if !(0..100_000).contains(&n) {return false;}
                let mut value=vec![0u16;n as usize+1];SendMessageW(combo,CB_GETLBTEXT,WPARAM(i),LPARAM(value.as_mut_ptr() as isize));
                String::from_utf16_lossy(&value[..n as usize])==*label
            });
            if !unchanged {
                SendMessageW(combo,CB_RESETCONTENT,WPARAM(0),LPARAM(0));
                for label in &labels {combo_add(combo,label);}
            }
            if !unchanged || selected(combo)!=self.channel_index {select(combo,self.channel_index);}
        }
    }
    unsafe fn choose_channel(&mut self,index:usize) {
        if index>=self.channels.len() {return;}
        if matches!(self.current,Some(Job::Record|Job::Scan(_))) {
            self.entry_osd(&format!("{}\n{}",i18n::text("Channel selection"),i18n::text("Stop recording or scanning first")));self.update_channels();return;
        }
        if index==self.channel_index && matches!(self.current,Some(Job::Watch)) {self.update_channels();return;}
        self.channel_index=index;self.frequency=self.channels[index];self.quality=None;
        self.channel_digits.clear();self.channel_entry_until=None;
        self.update_channels();self.save();self.show_osd(false);self.start(Job::Watch);
    }
    fn save(&self) {
        let _ = fs::create_dir_all(data_dir());
        let v = json!({"shader_acceleration":self.shader.id(),"processing_backend":self.backend.id(),"receiver_startup":self.receiver_startup,"language":i18n::code(),"parental":parental::config(),"picture":self.picture.json(),"video_hdr":self.video_hdr,"theme":"orbit","button_material":self.material.name(),"aspect_ratio":self.aspect_ratio.name(),"frequency":self.frequency,"channels":self.channels,"services":self.services,"channel_index":self.channel_index,"volume":self.volume,"audio_mode":self.audio_mode as u8,"captions_enabled":self.captions_enabled,"recording_folder":self.recording_folder,"snapshot_folder":self.snapshot_folder,"ffmpeg":self.options.ffmpeg,"gpu":self.options.gpu,"resolution":Resolution::ALL.iter().position(|r|*r==self.options.resolution).unwrap_or(0),"smooth":match self.options.deinterlacing{DeinterlaceMode::DoubleRate=>0,DeinterlaceMode::SingleRate=>1,DeinterlaceMode::Off=>2},"profile":match &self.options.color_profile{ColorProfile::Monitor=>json!("monitor"),ColorProfile::Disabled=>json!("off"),ColorProfile::File(p)=>json!(p)}});
        let _ = fs::write(data_dir().join("settings.json"), v.to_string());
    }
    fn ipc(&self, command: Value) {
        if let Some(tx) = &self.native_commands {
            let action = match command[0].as_str() {
                Some("cycle") => Some(native::Command::Pause),
                Some("set_property") => Some(native::Command::Volume(self.volume)),
                Some("screenshot-to-file") => command[1]
                    .as_str()
                    .map(|p| native::Command::Snapshot(p.into())),
                _ => None,
            };
            if let Some(action) = action {
                let _ = tx.send(action);
            }
        }
    }
    fn stop(&mut self) {
        self.pending_snapshot=None;
        self.pending = None;
        if self.rx.is_some() {
            self.status = "Stopping playback…".into();
            // Mark cancellation immediately; child cleanup stays off the UI thread.
            if !self.control.cancel.swap(true,std::sync::atomic::Ordering::Relaxed) {
                let c = self.control.clone();
                std::thread::spawn(move || c.stop());
            }
        }
    }
    fn start(&mut self, job: Job) {
        // UI preview must never open hardware, including actions exercised by UI checks.
        if std::env::args().any(|s|s=="--ui-preview") {return;}
        if matches!(job,Job::File(_)) && parental::active(){self.status="Parental controls: unlock in Settings > Parental to open a recording.".into();return;}
        unsafe{let _=ShowWindow(self.surface,SW_SHOWNA);}
        if self.rx.is_some() {
            self.stop();
            self.pending = Some(job);
            return;
        }
        // Some Windows graphics drivers retain presentation state on an HWND
        // even after its Vulkan swapchain is destroyed. Change APIs on a fresh
        // child window, only after the previous playback worker has finished.
        let output = (self.backend, self.shader);
        if self.surface_backend.is_some_and(|previous| previous != output) {
            let replacement = unsafe {
                (|| -> windows::core::Result<(HWND, HWND)> {
                    let surface = CreateWindowExW(WINDOW_EX_STYLE(0), w!("STATIC"), w!(""),
                        WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN | WS_CLIPSIBLINGS | WINDOW_STYLE(13),
                        0, 0, 1, 1, self.video, HMENU(500 as _), GetModuleHandleW(None)?, None)?;
                    match captions::create(surface) {
                        Ok(caption) => Ok((surface, caption)),
                        Err(error) => { let _ = DestroyWindow(surface); Err(error) }
                    }
                })()
            };
            match replacement {
                Ok((surface, caption)) => unsafe {
                    let old = self.surface;
                    self.surface = surface;
                    self.caption_window = caption;
                    let _ = SetWindowPos(surface, HWND_BOTTOM, 0, 0, 0, 0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                    let _ = DestroyWindow(old); // Also destroys its caption child.
                    self.layout();
                },
                Err(error) => { self.status = format!("Video window: {error}"); return; }
            }
        }
        self.surface_backend = Some(output);
        self.folder = data_dir().join("sessions").join(millis().to_string());
        if let Err(e) = fs::create_dir_all(&self.folder) {
            self.status = e.to_string();
            return;
        }
        let _ = fs::write(self.folder.join("video-window.json"), json!({
            "surface": self.surface.0 as usize, "decoder": self.backend.id(), "shader": self.shader.id()
        }).to_string());
        self.control = Control::default();
        self.control.captions_enabled.store(self.captions_enabled,std::sync::atomic::Ordering::Relaxed);
        self.caption_revision=u64::MAX;
        self.recording_path=if matches!(job,Job::Record){Some(recordings::new_path(&self.recording_folder,self.services.get(self.channel_index).and_then(|s|s["name"].as_str()).unwrap_or("TV")))}else{None};
        self.pipe = format!(r"\\.\pipe\a865r-tv-{}-{}", std::process::id(), millis());
        self.control
            .attach_presenter(self.surface.0 as usize, self.pipe.clone(), self.volume);
        self.status = if matches!(job, Job::Scan(_)) {
            "Preparing channel scan…"
        } else {
            "Preparing playback…"
        }.into();
        *self.control.message.lock().unwrap() = self.status.clone();
        self.paused = false;
        self.quality = None;
        self.current = Some(job.clone());
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        let mut options = self.options.clone();
        options.program_id = if matches!(job, Job::Watch | Job::Record) {
            self.services
                .get(self.channel_index)
                .and_then(|s| s["program_id"].as_u64())
                .map(|n| n as u32)
        } else {
            None
        };
        let diagnostic_args:Vec<_>=std::env::args().collect();
        if diagnostic_args.iter().any(|a|a=="--diagnostic") {if let Some(program)=diagnostic_args.iter().position(|a|a=="--diagnostic-program").and_then(|i|diagnostic_args.get(i+1)).and_then(|v|v.parse().ok()){options.program_id=Some(program);}}
        let service_hint=self.services.get(self.channel_index).cloned();
        let ui_window=self.video.0 as usize;
        let control = self.control.clone();
        let folder = self.folder.clone();
        let frequency = self.frequency;
        let (native_tx, native_rx) = mpsc::channel();
        let _=native_tx.send(native::Command::AspectRatio(self.aspect_ratio));
        let _=native_tx.send(native::Command::Picture(self.picture));
        let _=native_tx.send(native::Command::VideoHdr(self.video_hdr));
        self.native_commands = Some(native_tx.clone());
        let aspect_ratio=self.aspect_ratio;
        let picture=self.picture;let video_hdr=self.video_hdr;
        self.osd_on_first_frame=matches!(job,Job::Watch|Job::Record);
        if self.osd_on_first_frame { unsafe { self.show_osd(false); } }
        let surface = self.surface.0 as usize;
        let volume = self.volume;
        let audio_mode=self.audio_mode;
        let backend=self.backend;let shader=self.shader;
        let recording_path=self.recording_path.clone();
        std::thread::spawn(move || {
            let result = match job {
                Job::File(path) => native::run(
                    Some(path),
                    frequency,
                    options.program_id,
                    surface,
                    folder.clone(),
                    control,
                    &native_rx,
                    volume,
                    options.resolution,
                    options.deinterlacing,
                    audio_mode,
                    options.color_profile,
                    None,
                    None,
                    backend,shader,
                ),
                Job::Watch => {
                    let play=|hint|native::run(None,frequency,options.program_id,surface,folder.clone(),
                        control.clone(),&native_rx,volume,options.resolution,options.deinterlacing,audio_mode,options.color_profile.clone(),None,hint,backend,shader);
                    let result=play(service_hint);
                    if result["retry_without_cache"]==true && !control.cancel.load(std::sync::atomic::Ordering::Relaxed) {
                        let _=native_tx.send(native::Command::AspectRatio(aspect_ratio));
                        let _=native_tx.send(native::Command::Picture(picture));
                        let _=native_tx.send(native::Command::VideoHdr(video_hdr));
                        play(None)
                    }else{result}
                },
                Job::Record => native::record(
                    recording_path.unwrap(),
                    frequency,
                    options.program_id,
                    surface,
                    folder.clone(),
                    control,
                    native_rx,
                    volume,
                    options.resolution,
                    options.deinterlacing,
                    audio_mode,
                    options.color_profile,
                    options.ffmpeg,
                    backend,shader,
                ),
                Job::Scan(frequencies) => television::run(
                    Action::Discover(frequencies),
                    PathBuf::new(),
                    folder.clone(),
                    options,
                    control,
                ),
            };
            let _ = fs::write(folder.join("result.json"), result.to_string());
            if tx.send(result).is_ok() {
                // Wake the UI as soon as the old tuner is released; do not wait for
                // the 300-ms status timer to start the latest pending channel.
                unsafe {let _=PostMessageW(HWND(ui_window as _),WORK_COMPLETE,WPARAM(0),LPARAM(0));}
            }
        });
    }
    unsafe fn tick(&mut self) {
        self.update_effect_controls();
        let args:Vec<_>=std::env::args().collect();
        if args.iter().any(|a|a=="--diagnostic") && !self.closing {
            if args.iter().position(|a|a=="--diagnostic-control").and_then(|i|args.get(i+1)).is_some_and(|p|std::path::Path::new(p).exists()) {self.closing=true;self.control.finalization_cancel.store(true,std::sync::atomic::Ordering::Relaxed);self.stop();}
        }

        let now=std::time::Instant::now();
        if self.channel_entry_until.is_some_and(|t|now>=t) {self.commit_channel_entry();}
        if self.fullscreen && self.fullscreen_cursor_until.is_some_and(|deadline|now>=deadline)
            && GetForegroundWindow()==self.video && GetCapture()==HWND::default() {
            let mut point=POINT::default();let _=GetCursorPos(&mut point);
            let under=WindowFromPoint(point);
            if under==self.video || IsChild(self.video,under).as_bool() {SetCursor(HCURSOR::default());}
        }
        if self.fullscreen && self.fullscreen_button_until.is_some_and(|deadline|now>=deadline) {
            self.fullscreen_button_until=None;
            let _=ShowWindow(item(self.video,FULL),SW_HIDE);
        }
        for (id,deadline) in [(CHANNEL_OSD,&mut self.channel_osd_until),(VOLUME_OSD,&mut self.volume_osd_until)] {
            if deadline.is_some_and(|t| now>=t) { *deadline=None; let _=ShowWindow(item(self.video,id),SW_HIDE); }
        }
        if self.osd_on_first_frame && self.control.snapshot()["video_size"].as_array().is_some() {
            self.osd_on_first_frame=false; self.show_osd(false);
        }

        // Exercise the same restart path used by Settings without desktop automation.
        let switch = if args.iter().any(|a| a == "--verify-shader-switch") {
            self.verification.as_ref().and_then(|(start, stage, _)| {
                let seconds = start.elapsed().as_secs();
                if *stage == 10 && seconds >= 20 { Some((backend::Shader::Dx12, 11)) }
                else if *stage == 12 && seconds >= 40 { Some((backend::Shader::Dx11, 13)) }
                else if *stage == 14 && seconds >= 60 { Some((backend::Shader::Vulkan, 15)) }
                else { None }
            })
        } else { None };
        if let Some((shader, stage)) = switch {
            self.backend = backend::Backend::Microsoft;
            self.shader = shader;
            self.restart_picture_output();
            if let Some((_, value, _)) = &mut self.verification { *value = stage; }
        }
        if let Some((start, stage, report)) = &mut self.verification {
            let warmup = if std::env::args().any(|s| s == "--verify-record") {
                8
            } else {
                0
            };
            let elapsed = start.elapsed().as_secs().saturating_sub(warmup);
            let args: Vec<String> = std::env::args().collect();
            let soak = args
                .iter()
                .position(|s| s == "--soak-seconds")
                .and_then(|i| args.get(i + 1))
                .and_then(|s| s.parse::<u64>().ok())
                .map(|n| n.clamp(15, 300));
            if let Some(tx) = &self.native_commands {
                if args.iter().any(|a| a == "--verify-shader-switch") {
                    let shot = match *stage {
                        0 if elapsed >= 12 => Some(("vulkan", 10)),
                        11 if elapsed >= 32 => Some(("dx12", 12)),
                        13 if elapsed >= 52 => Some(("dx11", 14)),
                        15 if elapsed >= 72 => Some(("vulkan-return", 16)),
                        _ => None,
                    };
                    if let Some((name, next)) = shot {
                        let _ = tx.send(native::Command::Snapshot(report.join(format!("{name}.bmp"))));
                        *stage = next;
                    }
                } else if soak.is_some() && elapsed >= 12 && *stage == 0 {
                    let _ = tx.send(native::Command::Snapshot(report.join("frame.bmp")));
                    *stage = 4;
                }
                if soak.is_none() && elapsed >= 4 && *stage == 0 {
                    let _ = tx.send(native::Command::Pause);
                    *stage = 1;
                }
                if elapsed >= 6 && *stage == 1 {
                    let _ = tx.send(native::Command::Seek(1.));
                    *stage = 2;
                }
                if elapsed >= 8 && *stage == 2 {
                    let _ = tx.send(native::Command::Pause);
                    *stage = 3;
                }
                if elapsed >= 11 && *stage == 3 {
                    let _ = tx.send(native::Command::Snapshot(report.join("frame.bmp")));
                    *stage = 4;
                }
            }
            let _ = fs::create_dir_all(&report);
            let state=json!({"runtime":self.control.snapshot(),"status":self.status,"stage":stage,"recording_bytes":self.recording_path.as_ref().and_then(|p|fs::metadata(p).or_else(|_|fs::metadata(p.with_extension("broadcast.ts"))).ok()).map(|m|m.len())}).to_string();
            let _ = fs::write(report.join("state.json"), &state);
            let _ = fs::write(report.join(format!("stage-{stage}.json")), &state);
            if args.iter().any(|s| s == "--verify-epg") && elapsed >= 2 {
                if elapsed % 8 == 2 || elapsed % 8 == 3 {
                    let _ = PostMessageW(self.video, OPEN_EPG, WPARAM(0), LPARAM(0));
                }
                if elapsed % 8 == 5 {
                    guide::verify_select();
                }
                if elapsed % 8 == 6 {
                    guide::hide();
                }
                let _ = fs::write(
                    report.join("guide-state.json"),
                    guide::diagnostics().to_string(),
                );
            }
            if elapsed >= soak.unwrap_or(14) {
                self.verification = None;
                self.closing = true;
                    self.control.finalization_cancel.store(true,std::sync::atomic::Ordering::Relaxed);
                self.stop();
            }
        }
        if let Some(rx) = &self.rx {
            match rx.try_recv() {
                Ok(v) => {
                    if std::env::args().any(|a|a=="--diagnostic"){let _=fs::write(data_dir().join("diagnostic-result.json"),v.to_string());self.closing=true;}
                    self.rx = None;
                    self.current = None;
                    self.quality = None;
                    self.status = if let Some(e) = v["error"].as_str() {
                        e.to_owned()
                    } else if let Some(p) = v["recording"].as_str() {
                        format!("Saved recording: {p}")
                    } else {
                        "Ready • choose a channel or open a recording".into()
                    };
                    if let Some(stations) = v["stations"].as_array() {
                        let found: Vec<Value> = stations
                            .iter()
                            .flat_map(|s| s["services"].as_array().cloned().unwrap_or_default())
                            .collect();
                        self.merge_channels(&found);
                        self.status = format!(
                            "{} channels saved{}",
                            self.services.len(),
                            if v["stopped"] == true {
                                " • scan stopped"
                            } else {
                                ""
                            }
                        );
                    }
                    if let Some(next) = self.pending.take() {
                        self.start(next);
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.rx = None;
                    self.current = None;
                    self.status = "Playback worker ended unexpectedly. See session logs.".into();
                }
                Err(mpsc::TryRecvError::Empty) => {
                    self.status = self.control.message.lock().unwrap().clone();
                    if self.status=="Playing • recovered using Windows rendering" {
                        self.status="Playback recovered • Windows rendering".into();
                    } else if self.status.starts_with("Playing •") {
                        self.status.clear();
                    }
                    
                    self.quality = self.control.snapshot()["signal_quality_percent"]
                        .as_u64()
                        .map(|n| n.min(100) as u8);
                    if self.channel_digits.is_empty() && self.channel_osd_until.is_some() && text_of(item(self.video,CHANNEL_OSD)).starts_with(&channel_label(self.services.get(self.channel_index),self.channel_index)) && text_of(item(self.video,CHANNEL_OSD))!=self.channel_osd_text() {
                        set_text(item(self.video,CHANNEL_OSD),&self.channel_osd_text());self.layout_osd();
                        let _=InvalidateRect(item(self.video,CHANNEL_OSD),None,false);
                    }
                }
            }
        }
        if self.closing && self.rx.is_none() {
            let _ = DestroyWindow(self.settings);
            let _ = DestroyWindow(self.deck);
            let _ = DestroyWindow(self.video);
            PostQuitMessage(0);
            return;
        }
        let state = self.control.snapshot();
        let (revision,frame)={let c=self.control.captions.lock().unwrap();(c.revision,c.frame.clone())};
        if revision!=self.caption_revision {
            if let Err(e)=captions::display(self.caption_window,self.surface,frame.as_ref()){self.status=format!("Caption display: {e}");}
            self.caption_revision=revision;
        }
        for h in [self.video,self.deck]{set_text(item(h,CC),if self.captions_enabled{"CC On"}else{"CC Off"});}
        if let (Some(w), Some(h)) = (state["video_size"][0].as_u64(), state["video_size"][1].as_u64()) {
            if w > 0 && h > 0 && w <= 16384 && h <= 16384 { self.video_aspect = (w as u32, h as u32); }
        }
        if self.rx.is_none() {self.pending_snapshot=None;}
        if let Some((path,started))=&self.pending_snapshot {
            let outcome=if path.is_file() {Some(format!("Snapshot saved: {}",path.display()))}
                else if let Ok(error)=fs::read_to_string(path.with_extension("error.txt")) {Some(format!("Snapshot failed: {error}"))}
                else if started.elapsed()>std::time::Duration::from_secs(15) {Some("Snapshot unavailable: no image was returned.".into())}else{None};
            if let Some(message)=outcome {self.status=message;self.pending_snapshot=None;}
        }
        guide::ingest(&state["epg"]);
        if state["status"].as_str().unwrap_or("").starts_with("Parental controls:") || self.status.starts_with("Parental controls:"){let _=ShowWindow(self.surface,SW_HIDE);let _=ShowWindow(self.caption_window,SW_HIDE);}
        let t = &state["timeline"];
        if let Some(found) = state["scan_services"].as_array() {
            self.merge_channels(found);
        }
        if self.pending.is_none()
            && matches!(self.current, Some(Job::Watch | Job::Record))
            && state["service"].is_object()
            && state["service"]["frequency_khz"].as_u64() == Some(self.frequency as u64)
            && self.services.get(self.channel_index).is_none_or(|s| {
                s["program_id"].is_null() || s["program_id"] == state["service"]["program_id"]
            })
        {
            if self.services.get(self.channel_index) != Some(&state["service"]) {
                self.services.resize(self.channels.len(), Value::Null);
                if let Some(s) = self.services.get_mut(self.channel_index) {
                    *s = state["service"].clone();
                }
                self.update_channels();
                self.save();
            }
        }
        let enabled = self.rx.is_some() && t["seekable"] == true;
        let position = t["position"].as_f64().unwrap_or(0.);
        let duration = t["duration"].as_f64().unwrap_or(0.);
        self.paused = t["paused"].as_bool().unwrap_or(self.paused);
        let running = self.rx.is_some() && matches!(self.current, Some(Job::Watch | Job::Record | Job::File(_)));
        for h in [self.video, self.deck] {
            set_text(item(h, PLAY), if running && !self.paused { "Ⅱ Pause" } else { "▶ Play" });
            let _=EnableWindow(item(h,BACK),enabled);
            let _=EnableWindow(item(h,FORWARD),enabled);
            let recording=matches!(self.current,Some(Job::Record));
            let _=EnableWindow(item(h,LIVE),enabled&&recording);
            set_text(item(h,LIVE),if recording&&enabled&&!self.paused&&duration-position<=3. {"● Live"}else{"Live"});
            let slider = item(h, SEEK);
            let _ = EnableWindow(slider, enabled);
            if !self.dragging {
                let pos = if enabled && duration > 0. {
                    (position / duration * 10000.) as isize
                } else {
                    0
                };
                if SendMessageW(slider, TBM_GETPOS, WPARAM(0), LPARAM(0)).0 != pos {
                    SendMessageW(slider, TBM_SETPOS, WPARAM(1), LPARAM(pos));
                }
            }
            let display_position=if self.dragging && enabled {
                SendMessageW(item(self.video,SEEK),TBM_GETPOS,WPARAM(0),LPARAM(0)).0 as f64/10000.*duration
            }else{position};
            let time=if !running {"—".into()}else if !enabled && !recording {orbit::clock_time(display_position)}
                else {orbit::timeline_time(recording,matches!(self.current,Some(Job::File(_))),enabled,self.paused||self.dragging,display_position,duration)};
            set_text(item(h, SEEK_TIME),&time);

        }
        // Always clear obsolete text when playback ends or its progress message disappears.
        set_text(item(self.settings, 307), settings_status(self.current.as_ref(), &self.status));
        let stamp = format!(
            "{}:{:?}:{}:{}",
            self.status,
            self.quality,
            self.volume,
            millis() / 1000
        );
        if stamp != self.paint_stamp {
            self.paint_stamp = stamp;
            if !self.fullscreen {
            let r = client(self.video);
            let _ = InvalidateRect(
                self.video,
                Some(&RECT {
                    top: viewer::Layout::new(r.right,r.bottom).surface.bottom,
                    ..r
                }),
                false,
            );
            }
            let _=InvalidateRect(self.deck,None,false);
            let _=InvalidateRect(item(self.deck,CHANNEL),None,false);
            for id in [PLAY,RECORD,LIVE,CC] {let _=InvalidateRect(item(self.deck,id),None,false);}
            if !self.fullscreen {for id in [PLAY,RECORD,LIVE] {let _=InvalidateRect(item(self.video,id),None,false);}}

        }
    }
    unsafe fn update_volume(&mut self, volume:u32, persist:bool) {
        self.volume=volume.min(100);
        set_text(item(self.deck,DIAL),&format!("Volume: {} percent",self.volume));
        self.dial.borrow_mut().set_volume(self.volume);
        if IsWindowVisible(self.deck).as_bool(){SetTimer(self.deck,DIAL_TIMER,16,None);}
        let _=InvalidateRect(item(self.deck,DIAL),None,false);
        self.ipc(json!(["set_property","volume",self.volume]));
        self.show_osd(true);
        if persist {self.save();}
    }

    unsafe fn command(&mut self, hwnd: HWND, id: u16, notification: u16) {
        match id {
            viewer::OPTIONS=>{let _=PostMessageW(self.video,viewer::OPEN_OPTIONS,WPARAM(0),LPARAM(0));}
            viewer::MAXIMIZE=>{
                // Dispatch after releasing APP's borrow so Windows can deliver
                // WM_GETMINMAXINFO/WM_SIZE and retain the normal restore placement.
                viewer::toggle_maximize(self.video);
            }
            orbit::MATERIAL if notification==CBN_SELCHANGE as u16 => {
                self.material=[orbit::Material::Metal,orbit::Material::Glass,orbit::Material::Plastic][selected(item(self.settings,orbit::MATERIAL)).min(2)];
                self.save();
                let _=RedrawWindow(self.deck,None,None,RDW_INVALIDATE|RDW_ALLCHILDREN);
                let _=RedrawWindow(self.video,None,None,RDW_INVALIDATE|RDW_ALLCHILDREN);
                let _=RedrawWindow(self.settings,None,None,RDW_INVALIDATE|RDW_ALLCHILDREN);
            }
            CLOSE => {
                if hwnd == self.deck || hwnd == self.settings {
                    let _ = ShowWindow(hwnd, SW_HIDE);
                } else {
                    self.closing = true;
                    self.control.finalization_cancel.store(true,std::sync::atomic::Ordering::Relaxed);
                    self.stop();
                }
            }
            MIN => {
                let _ = ShowWindow(hwnd, SW_MINIMIZE);
            }
            FULL => {
                self.fullscreen = !self.fullscreen;
                self.fullscreen_cursor_until=if self.fullscreen {Some(std::time::Instant::now()+std::time::Duration::from_secs(3))}else{None};
                SetCursor(LoadCursorW(None,IDC_ARROW).unwrap_or_default());
                self.fullscreen_button_until=if self.fullscreen {Some(std::time::Instant::now()+std::time::Duration::from_secs(2))}else{None};
                let mut pointer=POINT::default();let _=GetCursorPos(&mut pointer);
                self.fullscreen_pointer=Some((pointer.x,pointer.y));
                let _=ShowWindow(item(self.video,FULL),SW_SHOWNA);
                if self.fullscreen {
                    let _ = GetWindowRect(self.video, &mut self.restore);
                    self.restore_style = GetWindowLongPtrW(self.video, GWL_STYLE);
                    self.restore_maximized = IsZoomed(self.video).as_bool();
                    self.restore_deck = IsWindowVisible(self.deck).as_bool();
                    let _ = ShowWindow(self.deck, SW_HIDE);
                    let _ = ShowWindow(self.settings, SW_HIDE);
                    guide::hide();
                    SetWindowLongPtrW(
                        self.video,
                        GWL_STYLE,
                        ((self.restore_style as u32
                            & !(WS_OVERLAPPEDWINDOW.0 | WS_MAXIMIZE.0 | WS_MINIMIZE.0))
                            | WS_POPUP.0) as isize,
                    );
                    let mut info = MONITORINFO {
                        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                        ..Default::default()
                    };
                    let _ = GetMonitorInfoW(
                        MonitorFromWindow(self.video, MONITOR_DEFAULTTONEAREST),
                        &mut info,
                    );
                    let r = info.rcMonitor;
                    let _ = SetWindowPos(
                        self.video,
                        None,
                        r.left,
                        r.top,
                        r.right - r.left,
                        r.bottom - r.top,
                        SWP_NOZORDER | SWP_FRAMECHANGED,
                    );
                } else {
                    SetWindowLongPtrW(self.video, GWL_STYLE, self.restore_style);
                    let r = self.restore;
                    let _ = SetWindowPos(
                        self.video,
                        None,
                        r.left,
                        r.top,
                        r.right - r.left,
                        r.bottom - r.top,
                        SWP_NOZORDER | SWP_FRAMECHANGED,
                    );
                    if self.restore_maximized {
                        let _ = ShowWindow(self.video, SW_MAXIMIZE);
                    }
                    if self.restore_deck {
                        let _ = ShowWindow(self.deck, SW_SHOWNOACTIVATE);
                    }
                }
                self.layout();
                mark_fullscreen(self.video, self.fullscreen);
                if self.fullscreen { let _=SetForegroundWindow(self.video); }
            }
            DECK => {
                let _ = ShowWindow(self.deck, SW_SHOW);
                let _ = SetForegroundWindow(self.deck);
            }
            CC=>{self.captions_enabled=!self.captions_enabled;self.control.captions_enabled.store(self.captions_enabled,std::sync::atomic::Ordering::Relaxed);if !self.captions_enabled{self.control.caption_frame(None);}self.save();}
            RECORD_FOLDER|SNAPSHOT_FOLDER=>{let _=PostMessageW(self.settings,OPEN_RECORD_FOLDER,WPARAM(id as usize),LPARAM(0));}
            AUDIO => {let _=PostMessageW(self.video,OPEN_AUDIO,WPARAM(0),LPARAM(0));}
            n if (AUDIO_MODE_BASE..AUDIO_MODE_BASE+audio::Mode::ALL.len() as u16).contains(&n)=>{
                self.audio_mode=audio::Mode::from_index((n-AUDIO_MODE_BASE) as usize);
                if let Some(tx)=&self.native_commands {let _=tx.send(native::Command::AudioMode(self.audio_mode));}
                self.save();
            }
            n if (AUDIO_TRACK_BASE..AUDIO_TRACK_BASE+64).contains(&n)=>{
                let tracks=audio_tracks(&self.control.snapshot());
                if let Some(track)=tracks.get((n-AUDIO_TRACK_BASE) as usize) {
                    if matches!(track["stream_type"].as_u64(),Some(0x0f|0x11)) {
                        if let Some(pid)=track["pid"].as_u64() {
                            if let Some(tx)=&self.native_commands {let _=tx.send(native::Command::AudioTrack(pid as u16));}
                        }
                    }
                }
            }
            BACK | FORWARD=>{
                let state=self.control.snapshot();let t=&state["timeline"];
                if t["seekable"]==true {
                    if let Some(tx)=&self.native_commands {let _=tx.send(native::Command::Step(if id==BACK{-1}else{1}));}
                }
            }
            LIVE=>{if let Some(tx)=&self.native_commands {let _=tx.send(native::Command::GoLive);}}
            EPG => {
                // Create/activate secondary windows after releasing the App borrow.
                let _ = PostMessageW(self.video, OPEN_EPG, WPARAM(0), LPARAM(0));
            }
            PLAY => {
                if self.rx.is_some()
                    && matches!(self.current, Some(Job::Watch | Job::Record | Job::File(_)))
                {
                    self.ipc(json!(["cycle", "pause"]));
                    self.paused = !self.paused;
                } else {
                    self.start(Job::Watch);
                }
            }
            STOP => self.stop(),
            RECORD => {
                if matches!(self.current, Some(Job::Record)) {
                    self.start(Job::Watch);
                } else {
                    self.start(Job::Record);
                }
            }
            311 if notification == CBN_SELCHANGE as u16 => {
                self.country = selected(item(self.settings, 311)).min(self.countries.len() - 1);
                self.country_fields();
                self.save_countries();
            }
            312 => {
                let name = text_of(item(self.settings, 311)).trim().to_owned();
                if name.is_empty() || name.len() > 80 {
                    set_text(
                        item(self.settings, 307),
                        "Enter a country name (up to 80 characters).",
                    );
                    return;
                }
                let parse = |id| {
                    text_of(item(self.settings, id))
                        .replace(',', ".")
                        .parse::<f64>()
                        .ok()
                        .filter(|v| v.is_finite() && *v >= 0. && *v <= 1000.)
                        .map(|v| (v * 1000.).round() as u32)
                };
                let (Some(first), Some(last), Some(step)) = (parse(308), parse(309), parse(310))
                else {
                    set_text(item(self.settings, 307), "Enter valid frequencies in MHz.");
                    return;
                };
                let old = self.countries.iter().find(|p| {
                    p["name"]
                        .as_str()
                        .is_some_and(|n| n.eq_ignore_ascii_case(&name))
                });
                let profile = json!({"name":name,"first":first,"last":last,"step":step,"standard":old.map(|p|p["standard"].clone()).unwrap_or(json!("ISDB-T")),"supported":old.map(|p|p["supported"].clone()).unwrap_or(json!(true))});
                if let Err(e) = countries::scan(&profile) {
                    set_text(item(self.settings, 307), &e);
                    return;
                }
                if let Some(i) = self.countries.iter().position(|p| {
                    p["name"]
                        .as_str()
                        .is_some_and(|n| n.eq_ignore_ascii_case(&name))
                }) {
                    self.countries[i] = profile;
                    self.country = i;
                } else {
                    self.countries.push(profile);
                    self.country = self.countries.len() - 1;
                    combo_add(item(self.settings, 311), &name);
                }
                select(item(self.settings, 311), self.country);
                self.country_fields();
                self.save_countries();
                set_text(item(self.settings, 307), "Country profile saved.");
            }
            SCAN => {
                if hwnd != self.settings {
                    self.settings_tab=1;SendMessageW(item(self.settings,orbit::TAB),TCM_SETCURSEL,WPARAM(1),LPARAM(0));self.layout_settings();
                    self.fill_settings();
                    let _ = ShowWindow(self.settings, SW_SHOW);
                    let _ = SetForegroundWindow(self.settings);
                } else {
                    if let Err(e) = countries::scan(&self.countries[self.country]) {
                        self.status = e;
                        return;
                    }
                    let parse = |id| -> Result<u32, String> {
                        let text = text_of(item(self.settings, id));
                        let n =
                            text.trim().replace(',', ".").parse::<f64>().map_err(|_| {
                                "Enter frequencies in MHz, such as 473.143".to_owned()
                            })?;
                        if !n.is_finite() || n < 0. || n > 1000. {
                            return Err("Enter a frequency between 470 and 697.999 MHz".into());
                        }
                        Ok((n * 1000.).round() as u32)
                    };
                    let list = (|| {
                        a865r::channel_plan::custom_scan(parse(308)?, parse(309)?, parse(310)?)
                            .map_err(|e| e.to_string())
                    })();
                    match list {
                        Ok(list) => self.start(Job::Scan(list)),
                        Err(e) => {
                            self.status = e;
                            set_text(item(self.settings, 307), &self.status);
                        }
                    }
                }
            }
            OPEN => {
                if let Some(p) = pick(hwnd, "TV recordings\0*.ts\0") {
                    self.start(Job::File(p));
                }
            }
            parental::UNLOCK=>{
                if parental::verify(&text_of(item(self.settings,parental::AUTH))){self.parental_edit=true;parental::UNLOCKED.store(true,std::sync::atomic::Ordering::Release);self.fill_parental();}
                else{set_text(item(self.settings,parental::STATUS),"Incorrect password.");}
                set_text(item(self.settings,parental::AUTH),"");
            }
            parental::LOCK if self.parental_edit=>{
                let index=selected(item(self.settings,parental::CHANNEL));
                if let Some(service)=self.services.get(index){let key=parental::channel_key(self.channels.get(index).copied().unwrap_or(0),service["program_id"].as_u64().unwrap_or(0) as u32);
                    let mut keys=self.parental_draft["channels"].as_array().cloned().unwrap_or_default();
                    if keys.iter().any(|v|v.as_str()==Some(&key)){keys.retain(|v|v.as_str()!=Some(&key));}else{keys.push(json!(key));}
                    self.parental_draft["channels"]=json!(keys);
                    // Preserve currently edited switches while refreshing channel labels.
                    self.parental_draft["enabled"]=json!(selected(item(self.settings,parental::ENABLE))==1);self.parental_draft["block_unrated"]=json!(selected(item(self.settings,parental::UNRATED))==1);self.parental_draft["max_age"]=json!([0,10,12,14,16,18][selected(item(self.settings,parental::RATING)).min(5)]);self.fill_parental();
                    if self.apply_parental(){self.save();}
                }
            }
            i18n::LANGUAGE if notification==CBN_SELCHANGE as u16=>{
                i18n::set(selected(item(self.settings,i18n::LANGUAGE)));i18n::refresh();self.save();
                for(i,name)in ["Video","Channels","Storage","Themes","Picture","Parental","General"].iter().enumerate(){let mut text=wide(&i18n::text(name));let tab=TCITEMW{mask:TCIF_TEXT,pszText:windows::core::PWSTR(text.as_mut_ptr()),..Default::default()};SendMessageW(item(self.settings,orbit::TAB),TCM_SETITEMW,WPARAM(i),LPARAM((&tab as *const TCITEMW) as isize));}
                for h in [self.settings,self.video,self.deck]{let _=RedrawWindow(h,None,None,RDW_INVALIDATE|RDW_ALLCHILDREN);}
                guide::language_changed();
            }
            picture::RESET=>{self.picture=Default::default();self.video_hdr=false;self.apply_picture();self.fill_picture();}
            picture::EFFECT=>{self.picture.hdr_effect=!self.picture.hdr_effect;self.apply_picture();self.fill_picture();}
            picture::PRESET if notification==CBN_SELCHANGE as u16=>{
                let index=selected(item(self.settings,picture::PRESET));
                if index<picture::PRESETS.len()-1 {self.picture=picture::Picture{hdr_effect:self.picture.hdr_effect,..picture::Picture::preset(index)};self.apply_picture();self.fill_picture();}
            }
            ASPECT if notification == CBN_SELCHANGE as u16 => {
                self.aspect_ratio=AspectRatio::ALL[selected(item(self.settings,ASPECT)).min(AspectRatio::ALL.len()-1)];
                if let Some(tx)=&self.native_commands { let _=tx.send(native::Command::AspectRatio(self.aspect_ratio)); }
                self.save();
            }
            PREV | NEXT => {
                if !self.channels.is_empty() {
                    let count=self.channels.len();
                    self.choose_channel((self.channel_index+if id==NEXT {1}else{count-1})%count);
                }
            }
            CHANNEL | 306 => {
                let combo=item(hwnd,id);
                match notification as u32 {
                    CBN_SELENDOK=>{
                        self.pending_channel_selection=Some((combo,selected(combo)));
                        let _=PostMessageW(hwnd,COMMIT_CHANNEL_SELECTION,WPARAM(0),LPARAM(0));
                    }
                    CBN_SELCHANGE if SendMessageW(combo,CB_GETDROPPEDSTATE,WPARAM(0),LPARAM(0)).0==0=>{
                        let index=selected(combo);
                        if index!=self.channel_index {
                            self.pending_channel_selection=Some((combo,index));
                            let _=PostMessageW(hwnd,COMMIT_CHANNEL_SELECTION,WPARAM(0),LPARAM(0));
                        }
                    }
                    CBN_SELENDCANCEL=>{self.pending_channel_selection=None;}
                    CBN_CLOSEUP=>{let _=PostMessageW(hwnd,COMMIT_CHANNEL_SELECTION,WPARAM(0),LPARAM(0));}
                    _=>{}
                }
            }
            VOL_DOWN | VOL_UP => {
                self.volume = if id == VOL_UP {
                    (self.volume + 5).min(100)
                } else {
                    self.volume.saturating_sub(5)
                };
                self.update_volume(self.volume, true);
            }
            SNAP => {
                if self.pending_snapshot.is_some() {return;}
                if let Some(tx)=self.native_commands.as_ref().filter(|_| self.rx.is_some() && !self.control.cancel.load(std::sync::atomic::Ordering::Relaxed)) {
                    if let Err(e)=fs::create_dir_all(&self.snapshot_folder) {self.status=format!("Cannot create snapshot folder: {e}");return;}
                    let path=self.snapshot_folder.join(format!("snapshot-{}.png",millis()));
                    if tx.send(native::Command::Snapshot(path.clone())).is_ok() {
                        self.pending_snapshot=Some((path,std::time::Instant::now()));self.status="Saving snapshot…".into();
                    } else {self.status="Snapshot unavailable: playback has stopped.".into();}
                } else {self.status="Play a channel or recording before taking a snapshot.".into();}
            }
            SETTINGS => {
                self.center_settings();
                self.fill_settings();
                let _ = ShowWindow(self.settings, SW_SHOW);
                let _=RemovePropW(self.settings,w!("OrbitShapeSize"));
                self.layout_settings();
                let _ = SetForegroundWindow(self.settings);
            }
            PROFILE => {
                if let Some(p) = pick(self.settings, "Color profiles\0*.icc;*.icm\0") {
                    match a865r_media::color::validate_profile(&p) {
                        Ok(_) => {
                            self.options.color_profile = ColorProfile::File(p);
                            self.fill_settings();self.save();self.restart_picture_output();
                        }
                        Err(e) => {
                            let m = wide(&e.to_string());
                            MessageBoxW(
                                self.settings,
                                PCWSTR(m.as_ptr()),
                                w!("Color profile"),
                                MB_OK | MB_ICONERROR,
                            );
                        }
                    }
                }
            }
            FFMPEG => {
                if let Some(p) = pick(self.settings, "FFmpeg executable\0ffmpeg.exe\0") {
                    self.options.ffmpeg = p;
                    self.fill_settings();self.save();
                }
            }
            301|302|303|304|SHADER_CONTROL if notification==CBN_SELCHANGE as u16 => {
                let before=format!("{:?}",(&self.options.resolution,&self.backend,&self.shader,&self.options.deinterlacing,&self.options.color_profile));
                self.options.resolution=RESOLUTIONS[selected(item(self.settings,301)).min(2)];
                self.backend=backend::Backend::choices(backend::microsoft_available()).get(selected(item(self.settings,302))).copied().unwrap_or_default();
                if id==SHADER_CONTROL {self.shader=backend::Shader::choices(self.backend,backend::capabilities()).get(selected(item(self.settings,SHADER_CONTROL))).copied().unwrap_or_default();}
                self.shader=self.shader.compatible(self.backend,backend::capabilities());
                self.fill_shader();self.fill_picture();
                self.options.deinterlacing=[DeinterlaceMode::DoubleRate,DeinterlaceMode::SingleRate,DeinterlaceMode::Off][selected(item(self.settings,303)).min(2)];
                match selected(item(self.settings,304)){0=>self.options.color_profile=ColorProfile::Monitor,1=>self.options.color_profile=ColorProfile::Disabled,_=>{}}
                self.save();
                if before!=format!("{:?}",(&self.options.resolution,&self.backend,&self.shader,&self.options.deinterlacing,&self.options.color_profile)) {self.restart_picture_output();}
            }
            a865r_bda::signal::CONTROL if notification==CBN_SELCHANGE as u16 => {
                let mode=selected(item(self.settings,a865r_bda::signal::CONTROL)).min(3);
                let config=a865r_bda::signal::Config{mode,ffmpeg:self.options.ffmpeg.clone()};
                let path=if std::env::args().any(|s|s=="--ui-preview"){data_dir().join("avertv-signal-preview.json")}else{a865r_bda::signal::config_path()};
                match a865r_bda::signal::save_at(&path,&config){
                    Ok(())=>{self.avertv_signal=mode;set_text(item(self.settings,SIGNAL_STATUS),"Saved. Applies when AverTV next tunes a channel.");SetTimer(self.settings,SIGNAL_STATUS_TIMER,5000,None);},
                    Err(e)=>{let _=KillTimer(self.settings,SIGNAL_STATUS_TIMER);select(item(self.settings,a865r_bda::signal::CONTROL),self.avertv_signal);set_text(item(self.settings,SIGNAL_STATUS),&e);}
                }
            }
            startup::ENABLE if notification==CBN_SELCHANGE as u16 => {
                let enabled=selected(item(self.settings,startup::ENABLE))==1;
                if enabled!=self.receiver_startup {
                    match startup::configure(enabled,std::env::args().any(|s|s=="--ui-preview"),&data_dir()) {
                        Ok(())=>{self.receiver_startup=enabled;self.save();set_text(item(self.settings,startup::STATUS),"");}
                        Err(e)=>{select(item(self.settings,startup::ENABLE),usize::from(self.receiver_startup));set_text(item(self.settings,startup::STATUS),&format!("{} {}",i18n::text("Could not update Windows startup."),e));}
                    }
                }
            }
            RECORD_PATH|SNAPSHOT_PATH if notification==EN_KILLFOCUS as u16 => {self.save_storage_field(id);}
            parental::ENABLE|parental::UNRATED|parental::RATING if notification==CBN_SELCHANGE as u16 => {
                if self.apply_parental(){self.save();}
            }
            parental::CONFIRM if notification==EN_KILLFOCUS as u16 => {
                if !text_of(item(self.settings,parental::CONFIRM)).is_empty() && self.apply_parental(){self.save();}
            }
            EXPORTS => {
                let _ = fs::create_dir_all(&self.recording_folder);
                let _ = std::process::Command::new("explorer.exe")
                    .arg(&self.recording_folder)
                    .spawn();
            }
            _ => {}
        }
    }
    unsafe fn restart_picture_output(&mut self) {
        if let Some(job)=self.current.clone().filter(|job|matches!(job,Job::Watch|Job::File(_))) {self.start(job);}
    }
    unsafe fn save_storage_field(&mut self,id:u16) {
        match recordings::validate_folder(&text_of(item(self.settings,id))) {
            Ok(path)=>{
                if id==RECORD_PATH {self.recording_folder=path;}else{self.snapshot_folder=path;}
                self.save();set_text(item(self.settings,STORAGE_STATUS),"");
            }
            Err(e)=>set_text(item(self.settings,STORAGE_STATUS),&e),
        }
    }
    unsafe fn apply_parental(&mut self)->bool {
        if !self.parental_edit {return true;}
        let mut draft=self.parental_draft.clone();
        let password=text_of(item(self.settings,parental::NEW));
        let confirm=text_of(item(self.settings,parental::CONFIRM));
        if !password.is_empty() || !confirm.is_empty() {
            if password!=confirm {set_text(item(self.settings,parental::STATUS),"Passwords do not match.");return false;}
            match parental::set_password(&password){Ok(value)=>draft["password"]=value,Err(e)=>{set_text(item(self.settings,parental::STATUS),&e);return false;}}
        }
        let enabled=selected(item(self.settings,parental::ENABLE))==1;
        if enabled&&draft["password"].is_null(){set_text(item(self.settings,parental::STATUS),"Set and confirm a password first.");return false;}
        draft["enabled"]=json!(enabled);draft["block_unrated"]=json!(selected(item(self.settings,parental::UNRATED))==1);draft["max_age"]=json!([0,10,12,14,16,18][selected(item(self.settings,parental::RATING)).min(5)]);
        if draft==parental::config() && self.settings_tab!=5 {return true;}
        self.parental_draft=draft;parental::load(self.parental_draft.clone());parental::UNLOCKED.store(false,std::sync::atomic::Ordering::Release);self.parental_edit=self.parental_draft["password"].is_null();
        for id in [parental::NEW,parental::CONFIRM]{set_text(item(self.settings,id),"");}self.fill_parental();
        if enabled && self.rx.is_some(){self.stop();self.status="Parental controls saved. Select a channel to continue.".into();}
        true
    }
    fn apply_picture(&self) {
        if let Some(tx)=&self.native_commands {let _=tx.send(native::Command::Picture(self.picture));let _=tx.send(native::Command::VideoHdr(self.video_hdr));}
        self.save();
    }
    unsafe fn fill_parental(&self) {
        let v=&self.parental_draft;
        select(item(self.settings,parental::ENABLE),usize::from(v["enabled"]==true));
        select(item(self.settings,parental::UNRATED),usize::from(v["block_unrated"]==true));
        select(item(self.settings,parental::RATING),[0,10,12,14,16,18].iter().position(|&age|Some(age)==v["max_age"].as_u64()).unwrap_or(5));
        let combo=item(self.settings,parental::CHANNEL);let previous=selected(combo);
        SendMessageW(combo,CB_RESETCONTENT,WPARAM(0),LPARAM(0));
        for (i,service) in self.services.iter().enumerate(){let key=parental::channel_key(self.channels.get(i).copied().unwrap_or(0),service["program_id"].as_u64().unwrap_or(0) as u32);let locked=v["channels"].as_array().is_some_and(|a|a.iter().any(|n|n.as_str()==Some(&key)));combo_add(combo,&format!("{}{}",if locked{"[Locked] "}else{""},service["name"].as_str().unwrap_or("TV")));}
        select(combo,previous.min(self.services.len().saturating_sub(1)));
        for id in [parental::NEW,parental::CONFIRM,parental::ENABLE,parental::UNRATED,parental::RATING,parental::CHANNEL,parental::LOCK ]{let _=EnableWindow(item(self.settings,id),self.parental_edit);}
        set_text(item(self.settings,parental::STATUS),if self.parental_edit{""}else{"Enter your password to edit or temporarily unlock."});
    }
    unsafe fn fill_shader(&self) {
        let combo=item(self.settings,SHADER_CONTROL);SendMessageW(combo,CB_RESETCONTENT,WPARAM(0),LPARAM(0));
        let choices=backend::Shader::choices(self.backend,backend::capabilities());
        for shader in &choices {combo_add(combo,shader.label());}
        select(combo,choices.iter().position(|s|*s==self.shader).unwrap_or(0));
    }
    unsafe fn update_effect_controls(&self) {
        let enabled=self.shader.effects(self.backend,backend::capabilities()) && self.control.snapshot()["shader_acceleration"]["enabled"].as_bool().unwrap_or(true);
        for id in [picture::PRESET,picture::SATURATION,picture::BRIGHTNESS,picture::CONTRAST,picture::EFFECT] {let h=item(self.settings,id);if IsWindowEnabled(h).as_bool()!=enabled {let _=EnableWindow(h,enabled);}}
    }
    unsafe fn fill_picture(&self) {
        self.update_effect_controls();

        select(item(self.settings,picture::PRESET),self.picture.index());
        for (id,value) in [(picture::SATURATION,self.picture.saturation),(picture::BRIGHTNESS,self.picture.brightness+100),(picture::CONTRAST,self.picture.contrast)] {
            SendMessageW(item(self.settings,id),TBM_SETPOS,WPARAM(1),LPARAM(value as isize));
        }
        set_text(item(self.settings,picture::EFFECT),if self.picture.hdr_effect{"HDR effect: On"}else{"HDR effect: Off"});
        let _=InvalidateRect(self.settings,None,false);
    }
    unsafe fn fill_settings(&self) {
        let _=KillTimer(self.settings,SIGNAL_STATUS_TIMER);set_text(item(self.settings,SIGNAL_STATUS),"");
        self.fill_shader();self.fill_picture();self.fill_parental();select(item(self.settings,i18n::LANGUAGE),i18n::index());
        select(item(self.settings,startup::ENABLE),usize::from(self.receiver_startup));
        select(item(self.settings,a865r_bda::signal::CONTROL),self.avertv_signal);
        set_text(item(self.settings,startup::STATUS),"");
        select(item(self.settings,orbit::THEME),0);
        select(item(self.settings,orbit::MATERIAL),self.material.index());
        set_text(item(self.settings,RECORD_PATH),&self.recording_folder.to_string_lossy());
        set_text(item(self.settings,SNAPSHOT_PATH),&self.snapshot_folder.to_string_lossy());
        set_text(item(self.settings,STORAGE_STATUS),"");
        select(item(self.settings,ASPECT),AspectRatio::ALL.iter().position(|a|*a==self.aspect_ratio).unwrap_or(0));
        select(
            item(self.settings, 301),
            RESOLUTIONS
                .iter()
                .position(|r| *r == self.options.resolution)
                .unwrap_or(0),
        );
        select(item(self.settings,302),backend::Backend::choices(backend::microsoft_available()).iter().position(|b|*b==self.backend).unwrap_or(0));
        select(item(self.settings,303),match self.options.deinterlacing{DeinterlaceMode::DoubleRate=>0,DeinterlaceMode::SingleRate=>1,DeinterlaceMode::Off=>2});
        let c = item(self.settings, 304);
        SendMessageW(c, CB_RESETCONTENT, WPARAM(0), LPARAM(0));
        combo_add(c, "Monitor ICC");
        combo_add(c, "Off");
        match &self.options.color_profile {
            ColorProfile::Monitor => select(c, 0),
            ColorProfile::Disabled => select(c, 1),
            ColorProfile::File(p) => {
                combo_add(c, &p.file_name().unwrap_or_default().to_string_lossy());
                select(c, 2);
            }
        }
    }
    unsafe fn center_settings(&self) {
        let mut panel=RECT::default();
        let _=GetWindowRect(self.deck,&mut panel);
        // Size and center for the panel's monitor, including after it changes monitors.
        let dpi=GetDpiForWindow(self.deck) as i32;
        let (w,h)=(750*dpi/96,580*dpi/96);
        let mut monitor=MONITORINFO{cbSize:std::mem::size_of::<MONITORINFO>() as u32,..Default::default()};
        let _=GetMonitorInfoW(MonitorFromWindow(self.deck,MONITOR_DEFAULTTONEAREST),&mut monitor);
        let work=monitor.rcWork;
        let x=((panel.left+panel.right-w)/2).clamp(work.left,(work.right-w).max(work.left));
        let y=((panel.top+panel.bottom-h)/2).clamp(work.top,(work.bottom-h).max(work.top));
        let _=SetWindowPos(self.settings,None,x,y,w,h,SWP_NOZORDER|SWP_NOACTIVATE);
    }
    unsafe fn layout_settings(&self) {
        let dpi=GetDpiForWindow(self.settings) as i32;let scale=|n:i32|n*dpi/96;
        let r=client(self.settings);
        self.skin.window_shape(self.settings,r);
        place(self.settings,CLOSE,r.right-scale(64),scale(9),scale(30),scale(30));
        let tabs=item(self.settings,orbit::TAB);
        place(self.settings,orbit::TAB,scale(14),scale(48),r.right-scale(28),scale(54));
        let tab_width=(r.right-scale(36))/7;
        SendMessageW(tabs,TCM_SETITEMSIZE,WPARAM(0),LPARAM(((scale(42) as u32)<<16|tab_width as u32) as isize));
        let pages:[&[(u16,i32,i32,i32,i32)];7]=[
            &[(304,260,88,230,180),(PROFILE,500,88,210,32),(301,260,150,450,180),(ASPECT,260,192,450,180),(302,260,234,450,180),(SHADER_CONTROL,260,276,450,200),(303,260,318,450,180),(a865r_bda::signal::CONTROL,260,374,450,220),(SIGNAL_STATUS,40,420,670,26)],
            &[(311,40,104,440,180),(312,500,100,210,34),(306,40,182,670,180),(308,40,266,200,28),(309,275,266,200,28),(310,510,266,200,28),(SCAN,40,326,320,38),(STOP,390,326,320,38),(307,40,388,670,75)],
            &[(RECORD_PATH,50,130,450,22),(RECORD_FOLDER,530,122,180,36),(SNAPSHOT_PATH,50,250,450,22),(SNAPSHOT_FOLDER,530,242,180,36),(STORAGE_STATUS,40,370,670,54)],
            &[(orbit::THEME,260,98,450,180),(orbit::MATERIAL,260,164,450,180)],
            &[(picture::PRESET,260,88,450,180),(picture::SATURATION,260,144,450,32),(picture::BRIGHTNESS,260,200,450,32),(picture::CONTRAST,260,256,450,32),(picture::EFFECT,260,312,450,34)],
            &[(parental::AUTH,260,80,270,32),(parental::UNLOCK,540,80,170,32),(parental::NEW,260,128,450,32),(parental::CONFIRM,260,176,450,32),(parental::ENABLE,260,224,125,120),(parental::UNRATED,600,224,110,120),(parental::RATING,260,272,160,180),(parental::CHANNEL,260,320,260,180),(parental::LOCK,530,320,180,32),(parental::STATUS,40,414,670,42)],
            &[(i18n::LANGUAGE,260,100,450,200),(startup::ENABLE,530,172,180,180),(startup::STATUS,40,280,670,65)],
        ];
        for (page,controls) in pages.iter().enumerate(){for &(id,x,y,w,h) in *controls {
            if [301,302,365,303,304,306,311,313,321,322,340,347,startup::ENABLE,a865r_bda::signal::CONTROL,354,355,356,357].contains(&id){
                SendMessageW(item(self.settings,id),CB_SETITEMHEIGHT,WPARAM(usize::MAX),LPARAM(scale(28) as isize));
                SendMessageW(item(self.settings,id),CB_SETITEMHEIGHT,WPARAM(0),LPARAM(scale(28) as isize));
            }
            place(self.settings,id,scale(x),scale(y+30),scale(w),scale(h));
            let _=ShowWindow(item(self.settings,id),if page==self.settings_tab{SW_SHOWNA}else{SW_HIDE});
        }}
        for id in [FFMPEG,305]{let _=ShowWindow(item(self.settings,id),SW_HIDE);}
        place(self.settings,picture::RESET,scale(340),r.bottom-scale(66),scale(180),scale(34));
        let _=ShowWindow(item(self.settings,picture::RESET),if self.settings_tab==4{SW_SHOWNA}else{SW_HIDE});
        let _=InvalidateRect(tabs,None,false);
        let _=InvalidateRect(self.settings,None,false);
    }
    unsafe fn paint_settings(&self,dc:HDC,r:RECT){
        self.skin.settings_background(dc,r,GetDpiForWindow(self.settings));
        let dpi=GetDpiForWindow(self.settings) as i32;let unit=|v:i32|v*dpi/96;
        let rr=|x,y,w,h|RECT{left:unit(x),top:unit(y+30),right:unit(x+w),bottom:unit(y+h+30)};
        label(dc,RECT{left:unit(38),top:unit(8),right:r.right-unit(80),bottom:unit(38)},
            "Settings",unit(16),orbit::rgb(217,194,159),false);
        self.skin.settings_widget(dc,r,item(self.settings,orbit::TAB),self.settings_tab,GetDpiForWindow(self.settings));
        let text=|x,y,w,h,t:&str|label(dc,rr(x,y,w,h),t,unit(15),orbit::rgb(221,205,179),false);
        match self.settings_tab {
            0=>{text(40,88,210,32,"Color profile");for(i,t)in ["Output size","Aspect ratio","Decoder","Shader acceleration","Deinterlacing"].iter().enumerate(){text(40,150+i as i32*42,210,32,t);}text(40,374,210,32,"AverTV signal");}
            1=>{text(40,74,670,26,"Country / scan profile");text(40,154,670,25,"Channels");for(x,t)in [(40,"From (MHz)"),(275,"To (MHz)"),(510,"Step (MHz)")]{text(x,234,152,26,t);}}
            2=>{
                text(40,82,670,30,"Recording folder");text(40,202,670,30,"Snapshot folder");
                for y in [122,242] {orbit::rounded_well(dc,rr(40,y,470,36),orbit::rgb(43,31,22),unit(12));}
                text(40,304,670,28,"Recordings are saved as TS. Snapshots are saved as PNG.");
            }
            6=>{text(40,100,210,32,"Language");text(40,172,480,32,"Start receiver with Windows");}
            5=>{text(40,80,210,32,"Password");text(40,128,210,32,"New password");text(40,176,210,32,"Confirm");text(40,224,170,32,"TV blocking");label_aligned(dc,rr(390,224,204,32),&i18n::text("Block unrated"),unit(15),orbit::rgb(221,205,179),DT_RIGHT);text(40,272,210,32,"Maximum age");text(40,320,210,32,"Channel");}
            4=>{text(40,88,210,32,"Video preset");
                text(40,144,210,32,&format!("Saturation {}%",self.picture.saturation));
                text(40,200,210,32,&format!("Brightness {:+}",self.picture.brightness));
                text(40,256,210,32,&format!("Contrast {}%",self.picture.contrast));text(40,312,210,32,"HDR effect");}
            _=>{text(30,98,150,32,"Main theme");text(30,164,150,32,"Button variation");}
        }
    }
    unsafe fn paint(&self, hwnd: HWND, dc: HDC) {
        let r = client(hwnd);
        if hwnd == self.video {
            if self.fullscreen {
                let _=FillRect(dc,&r,HBRUSH(GetStockObject(BLACK_BRUSH).0));
                return;
            }
            self.viewer.paint(self,dc,r);
        } else if hwnd == self.deck {
            self.skin.panel(self,dc,r);
        } else {
            self.paint_settings(dc,r);
        }
    }
}

thread_local! { static DIAL_DRAG: RefCell<Option<(f32,f32)>> = const { RefCell::new(None) }; }
unsafe extern "system" fn dial_proc(hwnd:HWND,msg:u32,wp:WPARAM,lp:LPARAM,_id:usize,_data:usize)->LRESULT {
    let mut r=RECT::default(); let _=GetClientRect(hwnd,&mut r);
    let x=(lp.0 as u16 as i16) as f32-r.right as f32/2.;
    let y=((lp.0 >>16) as u16 as i16) as f32-r.bottom as f32/2.;
    let angle=y.atan2(x);
    match msg {
        WM_ERASEBKGND => return LRESULT(1),
        WM_KEYDOWN if matches!(wp.0,0x25|0x26|0x27|0x28|0x24|0x23) => {
            APP.with(|cell|{if let Ok(mut state)=cell.try_borrow_mut(){if let Some(a)=state.as_mut(){
                let volume=match wp.0{0x24=>0,0x23=>100,0x26|0x27=>(a.volume+5).min(100),_=>a.volume.saturating_sub(5)};
                a.update_volume(volume,true);
            }}});return LRESULT(0);
        }
        WM_LBUTTONDOWN if x*x+y*y>16. => {
            APP.with(|cell| {if let Ok(state)=cell.try_borrow(){if let Some(a)=state.as_ref(){
                DIAL_DRAG.with(|d| *d.borrow_mut()=Some((angle,a.volume as f32)));
            }}});
            SetCapture(hwnd); let _=SetFocus(hwnd); return LRESULT(0);
        }
        WM_MOUSEMOVE if GetCapture()==hwnd && wp.0 & 1 != 0 => {
            if x*x+y*y>16. {
                let volume=DIAL_DRAG.with(|d| {let mut d=d.borrow_mut();d.as_mut().map(|(previous,volume)| {
                    *volume=dial::drag_volume(*volume,*previous,angle);*previous=angle;volume.round() as u32
                })});
                if let Some(volume)=volume {APP.with(|cell| {if let Ok(mut state)=cell.try_borrow_mut(){if let Some(a)=state.as_mut(){
                    if a.volume!=volume {a.update_volume(volume,false);}
                }}});}
            }
            return LRESULT(0);
        }
        WM_LBUTTONUP | WM_CANCELMODE | WM_CAPTURECHANGED => {
            let active=DIAL_DRAG.with(|d|d.borrow_mut().take().is_some());
            if active {APP.with(|cell| {if let Ok(state)=cell.try_borrow(){if let Some(a)=state.as_ref(){a.save();}}});}
            if msg!=WM_CAPTURECHANGED && GetCapture()==hwnd {let _=ReleaseCapture();}
        }
        _=>{}
    }
    DefSubclassProc(hwnd,msg,wp,lp)
}

fn audio_tracks(state:&Value)->Vec<Value> {
    let program=&state["service"]["program_id"];
    state["audio_tracks"].as_array().map(|a|a.iter().filter(|t|&t["program_id"]==program).take(64).cloned().collect()).unwrap_or_default()
}
unsafe fn show_audio_menu(hwnd:HWND) {
    let Some((mode,state))=APP.with(|cell|cell.try_borrow().ok().and_then(|a|a.as_ref().map(|a|(a.audio_mode,a.control.snapshot())))) else{return;};
    let Ok(menu)=CreatePopupMenu() else{return;};
    for (i,m) in audio::Mode::ALL.iter().enumerate(){
        let label=wide(&i18n::text(m.name()));let _=AppendMenuW(menu,MF_STRING|if *m==mode{MF_CHECKED}else{MF_UNCHECKED},AUDIO_MODE_BASE as usize+i,PCWSTR(label.as_ptr()));
    }
    let _=AppendMenuW(menu,MF_SEPARATOR,0,PCWSTR::null());
    if let Some(channels)=state["audio_details"]["broadcast_channels"].as_u64(){
        let label=wide(&match channels{6=>"Broadcast: 5.1 surround".to_owned(),2=>"Broadcast: stereo".to_owned(),1=>"Broadcast: mono".to_owned(),n=>format!("Broadcast: {n} channels")});
        let _=AppendMenuW(menu,MF_STRING|MF_GRAYED,0,PCWSTR(label.as_ptr()));
    }
    let tracks=audio_tracks(&state);
    if tracks.is_empty(){let _=AppendMenuW(menu,MF_STRING|MF_GRAYED,0,PCWSTR(wide(&i18n::text("Waiting for broadcast audio tracks…")).as_ptr()));}
    for (i,t) in tracks.iter().enumerate(){
        let language=match t["language"].as_str(){Some("por")=>"Português",Some("eng")=>"English",Some("spa")=>"Español",Some(s)=>s,None=>"Audio"};
        let supported=matches!(t["stream_type"].as_u64(),Some(0x0f|0x11));
        let label=wide(&format!("{} — track {}{}",language,i+1,if supported{String::new()}else{i18n::text(" (unsupported codec)")}));
        let flags=MF_STRING|if t["pid"]==state["audio_output"]["pid"]{MF_CHECKED}else{MF_UNCHECKED}|if supported{MF_ENABLED}else{MF_GRAYED};
        let _=AppendMenuW(menu,flags,AUDIO_TRACK_BASE as usize+i,PCWSTR(label.as_ptr()));
    }
    let background=CreateSolidBrush(COLORREF(orbit::rgb(30,22,16)));
    let info=MENUINFO{cbSize:std::mem::size_of::<MENUINFO>() as u32,fMask:MIM_BACKGROUND,hbrBack:background,..Default::default()};let _=SetMenuInfo(menu,&info);
    for i in 0..GetMenuItemCount(menu).max(0) as u32 {
        let mut entry=MENUITEMINFOW{cbSize:std::mem::size_of::<MENUITEMINFOW>() as u32,fMask:MIIM_FTYPE,..Default::default()};
        let _=GetMenuItemInfoW(menu,i,true,&mut entry);
        if entry.fType.0&MFT_SEPARATOR.0==0 {entry.fType=MFT_OWNERDRAW;let _=SetMenuItemInfoW(menu,i,true,&entry);}
    }
    let mut point=POINT::default();let _=GetCursorPos(&mut point);
    let selected=TrackPopupMenu(menu,TPM_RETURNCMD|TPM_NONOTIFY,point.x,point.y,0,hwnd,None).0 as u16;
    let _=DestroyMenu(menu);
    let _=DeleteObject(background);
    if selected!=0 {APP.with(|cell|{if let Ok(mut a)=cell.try_borrow_mut(){if let Some(a)=a.as_mut(){a.command(hwnd,selected,0);}}});}
}
unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    orbit::dropdown_backdrop(hwnd,msg,wp,lp);
    if GetPropW(hwnd,w!("OrbitBorderless")).0 as usize==3 {
        if msg==WM_NCCALCSIZE{return LRESULT(0);}
        // WS_THICKFRAME retains native resizing, but DefWindowProc must never
        // paint its system border over our full-client custom frame.
        if msg==WM_NCPAINT{return LRESULT(0);}
        if msg==WM_NCACTIVATE {
            let result=DefWindowProcW(hwnd,msg,wp,LPARAM(-1));
            viewer::invalidate_chrome(hwnd);
            return result;
        }
        if msg==WM_EXITSIZEMOVE {viewer::invalidate_chrome(hwnd);}
        if matches!(msg,WM_THEMECHANGED|WM_DWMCOMPOSITIONCHANGED) {viewer::configure_frame(hwnd);}
        if msg==WM_NCHITTEST{return viewer::hit_test(hwnd,lp);}
    }
    if msg==viewer::OPEN_OPTIONS {viewer::show_options(hwnd);return LRESULT(0);}
    // Keep native resize/system-menu behavior, but let Orbit own the whole face.
    if GetPropW(hwnd,w!("OrbitBorderless")).0 as usize==2 {
        if msg==WM_NCCALCSIZE {return LRESULT(0);}
        if msg==WM_NCHITTEST {
            let mut p=POINT{x:lp.0 as u16 as i16 as i32,y:(lp.0>>16) as u16 as i16 as i32};
            let _=ScreenToClient(hwnd,&mut p);
            let dpi=GetDpiForWindow(hwnd) as i32;let r=client(hwnd);
            let close=p.x>=r.right-64*dpi/96 && p.x<r.right-34*dpi/96 && p.y>=9*dpi/96 && p.y<39*dpi/96;
            let apply=p.x>=390*dpi/96 && p.x<540*dpi/96
                && p.y>=r.bottom-66*dpi/96 && p.y<r.bottom-32*dpi/96;
            let fascia=p.y<48*dpi/96 || p.y>=r.bottom-14*dpi/96
                || p.x<14*dpi/96 || p.x>=r.right-14*dpi/96;
            return LRESULT(if fascia && !close && !apply {HTCAPTION}else{HTCLIENT} as isize);
        }
    }
    if GetPropW(hwnd,w!("OrbitBorderless")).0 as usize==1 {
        if msg==WM_NCCALCSIZE {return LRESULT(0);}
        if msg==WM_NCHITTEST {
            let mut p=POINT{x:lp.0 as u16 as i16 as i32,y:(lp.0>>16) as u16 as i16 as i32};
            let _=ScreenToClient(hwnd,&mut p);
            let r=client(hwnd);let edge=(6*GetDpiForWindow(hwnd)/96).max(4) as i32;
            let left=p.x<edge;let right=p.x>=r.right-edge;
            let top=p.y<edge;let bottom=p.y>=r.bottom-edge;
            let controls=orbit::rect(r,1541.,24.,84.,36.);
            let over_controls=p.x>=controls.left && p.x<controls.right && p.y>=controls.top && p.y<controls.bottom;
            let hit=match (left,right,top,bottom) {
                (true,_,true,_)=>HTTOPLEFT,(_,true,true,_)=>HTTOPRIGHT,
                (true,_,_,true)=>HTBOTTOMLEFT,(_,true,_,true)=>HTBOTTOMRIGHT,
                (true,_,_,_)=>HTLEFT,(_,true,_,_)=>HTRIGHT,
                (_,_,true,_)=>HTTOP,(_,_,_,true)=>HTBOTTOM,
                _ if p.y<r.bottom*70/547 && !over_controls=>HTCAPTION,
                _=>HTCLIENT,
            };
            return LRESULT(hit as isize);
        }
    }
    if msg==WM_SETCURSOR {
        let hidden=APP.with(|cell|cell.try_borrow().ok().is_some_and(|a|a.as_ref().is_some_and(|a|
            hwnd==a.video && a.fullscreen && GetForegroundWindow()==a.video
            && GetCapture()==HWND::default()
            && a.fullscreen_cursor_until.is_some_and(|deadline|std::time::Instant::now()>=deadline))));
        if hidden {SetCursor(HCURSOR::default());return LRESULT(1);}
    }
    if msg==OPEN_RECORD_FOLDER {
        let snapshot=wp.0==SNAPSHOT_FOLDER as usize;let field=if snapshot {SNAPSHOT_PATH}else{RECORD_PATH};
        let current=APP.with(|c|c.try_borrow().ok().and_then(|a|a.as_ref().map(|a|if snapshot {a.snapshot_folder.clone()}else{a.recording_folder.clone()})));
        if let Some(current)=current {if let Some(path)=recordings::choose(hwnd,&current,if snapshot {"Snapshot folder"}else{"Recording folder"}) {
            set_text(item(hwnd,field),&path.to_string_lossy());
            APP.with(|cell|{if let Ok(mut app)=cell.try_borrow_mut(){if let Some(app)=app.as_mut(){app.save_storage_field(field);}}});
        }}
        return LRESULT(0);
    }
    if msg == OPEN_AUDIO {show_audio_menu(hwnd);return LRESULT(0);}
    if msg == OPEN_EPG {
        if let Err(e) = guide::show(hwnd) {
            APP.with(|cell| {
                if let Ok(mut app) = cell.try_borrow_mut() {
                    if let Some(app) = app.as_mut() {
                        app.status = e.to_string();
                    }
                }
            });
        }
        return LRESULT(0);
    }

    if msg==WM_MEASUREITEM && lp.0!=0 {let m=&mut *(lp.0 as *mut MEASUREITEMSTRUCT);if m.CtlID==CHANNEL as u32 || [startup::ENABLE,a865r_bda::signal::CONTROL].contains(&(m.CtlID as u16)) || [301,302,365,303,304,306,313,321,322,340,347,354,355,356,357].contains(&m.CtlID){m.itemHeight=30;return LRESULT(1);}}
    if msg==WM_NOTIFY && lp.0!=0 {
        let hdr=&*(lp.0 as *const NMHDR);
        if hdr.idFrom==orbit::TAB as usize && hdr.code==TCN_SELCHANGE {
            APP.with(|c|{if let Ok(mut a)=c.try_borrow_mut(){if let Some(a)=a.as_mut(){a.settings_tab=SendMessageW(hdr.hwndFrom,TCM_GETCURSEL,WPARAM(0),LPARAM(0)).0.max(0) as usize;a.layout_settings();if a.settings_tab==1 {clear_country_selection(a.settings);}}}});return LRESULT(0);
        }
    }
    if msg == WM_NOTIFY && lp.0 != 0 {
        let header = &*(lp.0 as *const NMHDR);
        if (header.idFrom == SEEK as usize || picture::SLIDERS.contains(&(header.idFrom as u16))) && header.code == NM_CUSTOMDRAW {
            let draw = &*(lp.0 as *const NMCUSTOMDRAW);
            if draw.dwDrawStage == CDDS_PREPAINT {
                let child = header.hwndFrom;
                if GetPropW(hwnd,w!("OrbitBorderless")).0 as usize==3 && !APP.with(|a|a.try_borrow().is_ok()) {
                    let _=InvalidateRect(child,None,false);return LRESULT(CDRF_SKIPDEFAULT as isize);
                }
                let r = client(child);
                let mut track = RECT::default();
                let mut thumb = RECT::default();
                SendMessageW(
                    child,
                    TBM_GETCHANNELRECT,
                    WPARAM(0),
                    LPARAM((&mut track as *mut RECT) as isize),
                );
                SendMessageW(
                    child,
                    TBM_GETTHUMBRECT,
                    WPARAM(0),
                    LPARAM((&mut thumb as *mut RECT) as isize),
                );
                buffered_control(draw.hdc, r, |dc| {
                    chrome_background(dc, hwnd, child, r);
                    let center = (track.top + track.bottom) / 2;
                    let thickness = (GetDpiForWindow(child) as i32 / 96).max(1);
                    fill(
                        dc,
                        RECT {
                            left: track.left,
                            right: track.right,
                            top: center - thickness,
                            bottom: center + thickness,
                        },
                        0x006c8fab,
                    );
                    if IsWindowEnabled(child).as_bool() {
                        gradient(dc, thumb, [219, 192, 128], [111, 83, 42]);
                    } else {
                        gradient(dc, thumb, [158, 136, 94], [111, 83, 42]);
                    }
                });
                return LRESULT(CDRF_SKIPDEFAULT as isize);
            }
        }
    }
    if msg==WM_CTLCOLORLISTBOX {scrollbars::attach(HWND(lp.0 as _));}
    if msg==WM_CTLCOLOREDIT || msg==WM_CTLCOLORLISTBOX {
        let brush=APP.with(|c|c.try_borrow().ok().and_then(|a|a.as_ref().map(|a|a.skin.field_brush)));
        if let Some(brush)=brush{let dc=HDC(wp.0 as _);SetTextColor(dc,COLORREF(orbit::rgb(228,213,190)));SetBkColor(dc,COLORREF(orbit::rgb(43,31,22)));return LRESULT(brush.0 as isize);}
    }
    if msg == WM_CTLCOLORSTATIC {
        let dc = HDC(wp.0 as _);
        let child = HWND(lp.0 as _);
        if [STORAGE_STATUS,startup::STATUS,SIGNAL_STATUS,parental::STATUS].contains(&(GetDlgCtrlID(child) as u16)) {
            SetTextColor(dc,COLORREF(GOLD));SetBkColor(dc,COLORREF(orbit::rgb(28,20,14)));
            SetDCBrushColor(dc,COLORREF(orbit::rgb(28,20,14)));return LRESULT(GetStockObject(DC_BRUSH).0 as isize);
        }
        // Only the video surface needs a black background before frames arrive.
        if GetDlgCtrlID(child) == 500 {
            return LRESULT(GetStockObject(BLACK_BRUSH).0 as isize);
        }
        if GetDlgCtrlID(child) != DIAL as i32 {
            chrome_background(dc, hwnd, child, client(child));
        }
        SetTextColor(dc, COLORREF(GOLD));
        SetBkMode(dc, TRANSPARENT);
        return LRESULT(GetStockObject(NULL_BRUSH).0 as isize);
    }
    if msg == WM_ERASEBKGND {
        return LRESULT(1);
    }
    if msg == WM_DESTROY {
        let is_video = APP.with(|a| {
            a.try_borrow()
                .ok()
                .and_then(|a| a.as_ref().map(|a| a.video == hwnd))
                .unwrap_or(false)
        });
        if is_video {
            PostQuitMessage(0);
        }
        return LRESULT(0);
    }
    if msg == WM_PAINT {
        let mut ps = PAINTSTRUCT::default();
        let dc = BeginPaint(hwnd, &mut ps);
        let fullscreen=APP.with(|cell|cell.try_borrow().ok().is_some_and(|a|
            a.as_ref().is_some_and(|a|a.video==hwnd && a.fullscreen)));
        if fullscreen {
            // Only the opaque inset is exposed; BeginPaint already clips out
            // child video/overlays. No 4K bitmap or hidden skin needs repainting.
            let _=FillRect(dc,&ps.rcPaint,HBRUSH(GetStockObject(BLACK_BRUSH).0));
            let _=EndPaint(hwnd,&ps);
            return LRESULT(0);
        }
        let r = client(hwnd);
        let memory = CreateCompatibleDC(dc);
        let bitmap = CreateCompatibleBitmap(dc, r.right.max(1), r.bottom.max(1));
        let old = SelectObject(memory, bitmap);
        // Match the real paint clip and avoid drawing the chassis behind the video.
        IntersectClipRect(memory, ps.rcPaint.left, ps.rcPaint.top, ps.rcPaint.right, ps.rcPaint.bottom);
        APP.with(|cell| {
            if let Ok(app) = cell.try_borrow() {
                if app.as_ref().is_some_and(|a| a.video == hwnd) {
                    let v = viewer::Layout::new(r.right, r.bottom).surface;
                    ExcludeClipRect(memory, v.left, v.top, v.right, v.bottom);
                }
            }
        });
        // Always initialize the frame, including reentrant paints before App
        // is available. An untouched compatible bitmap starts out black.
        gradient(memory, r, CHROME_TOP, CHROME_BOTTOM);
        let painted=APP.with(|a| {
            if let Ok(a) = a.try_borrow() {
                if let Some(a) = a.as_ref() {a.paint(hwnd,memory);return true;}
            }
            false
        });
        let p = ps.rcPaint;
        let _ = BitBlt(
            dc,
            p.left,
            p.top,
            p.right - p.left,
            p.bottom - p.top,
            memory,
            p.left,
            p.top,
            SRCCOPY,
        );
        SelectObject(memory, old);
        let _ = DeleteObject(bitmap);
        let _ = DeleteDC(memory);
        let _ = EndPaint(hwnd, &ps);
        if !painted && GetPropW(hwnd,w!("OrbitBorderless")).0 as usize==3 {let _=InvalidateRect(hwnd,None,false);}
        return LRESULT(0);
    }
    if msg==WM_MEASUREITEM && lp.0!=0 {
        let m=&mut *(lp.0 as *mut MEASUREITEMSTRUCT);
        if m.CtlType==ODT_MENU {let dpi=GetDpiForWindow(hwnd);m.itemWidth=340*dpi/96;m.itemHeight=36*dpi/96;return LRESULT(1);}
    }
    if msg == WM_DRAWITEM {
        let item=&*(lp.0 as *const DRAWITEMSTRUCT);
        if item.CtlType==ODT_MENU {viewer::draw_menu(hwnd,item.hDC,item);return LRESULT(1);}
        if item.CtlID == 500 {
            APP.with(|state| {
                if let Ok(app) = state.try_borrow() {
                    if let Some(app) = app.as_ref() {
                        if app.current.is_some() {
                            if let Some(tx) = &app.native_commands { let _ = tx.send(native::Command::Repaint); }
                            return;
                        }
                    }
                    fill(item.hDC, item.rcItem, 0);
                }
            });
            return LRESULT(1);
        }
        let draw = &*(lp.0 as *const DRAWITEMSTRUCT);
        let r = draw.rcItem;
        if [startup::ENABLE,a865r_bda::signal::CONTROL].contains(&(draw.CtlID as u16)) || [301,302,365,303,304,306,311,313,321,322,340,347,354,355,356,357].contains(&draw.CtlID) {
            // The subclass paints the closed face. Native selection/focus redraws
            // must not stamp a square list-row background over its rounded rim.
            if draw.itemState.0 & ODS_COMBOBOXEDIT.0 != 0 {
                let _=InvalidateRect(draw.hwndItem,None,false);return LRESULT(1);
            }
            let mut text=vec![0u16;1024];
            if draw.itemID!=u32::MAX {SendMessageW(draw.hwndItem,CB_GETLBTEXT,WPARAM(draw.itemID as usize),LPARAM(text.as_mut_ptr() as isize));}
            let n=text.iter().position(|c|*c==0).unwrap_or(text.len());
            fill(draw.hDC,r,if draw.itemState.0 & ODS_SELECTED.0 != 0 {orbit::rgb(76,52,33)}else{orbit::rgb(35,25,18)});
            let language=draw.CtlID==i18n::LANGUAGE as u32;
            let d=GetDpiForWindow(hwnd) as i32;
            if language {i18n::flag(draw.hDC,RECT{left:r.left+10*d/96,right:r.left+34*d/96,top:r.top+(r.bottom-r.top-16*d/96)/2,bottom:r.top+(r.bottom-r.top+16*d/96)/2},draw.itemID as usize);}
            label(draw.hDC,RECT{left:r.left+if language{44*d/96}else{10},..r},&String::from_utf16_lossy(&text[..n]),14*d/96,GOLD,false);
            return LRESULT(1);
        }
        if draw.CtlID==CHANNEL as u32 {
            APP.with(|c|{if let Ok(a)=c.try_borrow(){if let Some(a)=a.as_ref(){
                if (draw.itemState.0 & ODS_COMBOBOXEDIT.0)!=0{a.skin.channel(a,draw.hDC,r);}else{
                    fill(draw.hDC,r,if (draw.itemState.0 & ODS_SELECTED.0)!=0{orbit::rgb(76,52,33)}else{orbit::rgb(35,25,18)});
                    let name=if draw.itemID==u32::MAX{"Select a channel".into()}else{channel_label(a.services.get(draw.itemID as usize),draw.itemID as usize)};
                    label_raw(draw.hDC,RECT{left:r.left+12,..r},&name,15*GetDpiForWindow(hwnd) as i32/96,GOLD,false);
                }
            }}});return LRESULT(1);
        }
        if draw.CtlID==orbit::TAB as u32 {
            APP.with(|c|{if let Ok(a)=c.try_borrow(){if let Some(a)=a.as_ref(){
                fill(draw.hDC,r,if draw.itemID as usize==a.settings_tab{orbit::rgb(67,45,30)}else{orbit::rgb(32,23,17)});
                label(draw.hDC,r,["Video","Channels","Storage","Themes","Picture","Parental","General"].get(draw.itemID as usize).unwrap_or(&""),14*GetDpiForWindow(hwnd) as i32/96,GOLD,true);
            }}});return LRESULT(1);
        }
        if draw.CtlID==DIAL as u32 {
            // Never publish a background-only frame during a reentrant update.
            APP.with(|cell| {if let Ok(state)=cell.try_borrow(){if let Some(app)=state.as_ref(){
                if let Ok(mut dial)=app.dial.try_borrow_mut(){
                    buffered_control(draw.hDC,r,|dc| {
                        chrome_background(dc,hwnd,draw.hwndItem,r);
                        dial.paint(dc,r);
                    });
                }
            }}});
            return LRESULT(1);
        }
        if draw.CtlID == CHANNEL_OSD as u32 || draw.CtlID == VOLUME_OSD as u32 {
            buffered_control(draw.hDC,r,|dc| {
                osd::paint(dc,r,&text_of(draw.hwndItem),draw.CtlID==VOLUME_OSD as u32,GetDpiForWindow(draw.hwndItem));
            });return LRESULT(1);
        }
        if draw.CtlID == SEEK_TIME as u32 {
            let themed=APP.with(|c|c.try_borrow().ok().is_some_and(|a|a.as_ref().is_some_and(|a|{
                if hwnd==a.video {buffered_control(draw.hDC,r,|dc|a.viewer.time(a,dc,draw.hwndItem,r));true}else{false}
            })));
            if themed{return LRESULT(1);}
            // set_text can request owner drawing synchronously inside tick,
            // while APP is mutably borrowed. Defer it instead of publishing
            // the retired bronze timer background until the next value change.
            if GetPropW(hwnd,w!("OrbitBorderless")).0 as usize==3 {
                let _=InvalidateRect(draw.hwndItem,None,false);return LRESULT(1);
            }
            buffered_control(draw.hDC, r, |dc| {
                chrome_background(dc, hwnd, draw.hwndItem, r);
                label(
                    dc,
                    r,
                    &text_of(draw.hwndItem),
                    14 * GetDpiForWindow(draw.hwndItem) as i32 / 96,
                    GOLD,
                    true,
                );
            });
            return LRESULT(1);
        }
        let themed=APP.with(|c|c.try_borrow().ok().is_some_and(|a|a.as_ref().is_some_and(|a|{
            if hwnd==a.video {buffered_control(draw.hDC,r,|dc|a.viewer.button(a,dc,draw));true}
            else if hwnd==a.deck || hwnd==a.settings {buffered_control(draw.hDC,r,|dc|a.skin.button(a,dc,draw));true}else{false}
        })));
        if themed{return LRESULT(1);}
        // EnableWindow can synchronously request painting while tick holds APP.
        // Keep the existing Orbit face until a deferred paint can borrow state;
        // never publish the legacy renderer during that reentrant callback.
        if !GetPropW(hwnd,w!("OrbitBorderless")).0.is_null() {
            let _=InvalidateRect(draw.hwndItem,None,false);
            return LRESULT(1);
        }
        let buffer = CreateCompatibleDC(draw.hDC);
        let bitmap = CreateCompatibleBitmap(draw.hDC, r.right.max(1), r.bottom.max(1));
        let old = SelectObject(buffer, bitmap);
        let pressed = (draw.itemState.0 & ODS_SELECTED.0) != 0;
        gradient(buffer, r, [27, 18, 9], [101, 73, 36]);
        let edge = RECT {
            left: r.left + 2,
            top: r.top + 2,
            right: r.right - 2,
            bottom: r.bottom - 2,
        };
        gradient(
            buffer,
            edge,
            if pressed {
                [231, 163, 22]
            } else {
                [204, 184, 141]
            },
            if pressed { [135, 77, 5] } else { [69, 51, 27] },
        );
        let inner = RECT {
            left: r.left + 4,
            top: r.top + 4,
            right: r.right - 4,
            bottom: r.bottom - 4,
        };
        gradient(
            buffer,
            inner,
            if draw.CtlID == PLAY as u32 || draw.CtlID == RECORD as u32 {
                [254, 192, 48]
            } else {
                [117, 90, 48]
            },
            [60, 38, 15],
        );
        if draw.CtlID == SNAP as u32 {
            let dpi = GetDpiForWindow(draw.hwndItem) as i32;
            let unit = |v: i32| v * dpi / 96;
            let cx = (r.left + r.right) / 2;
            let cy = (r.top + r.bottom) / 2;
            let pen = CreatePen(PS_SOLID, unit(1).max(1), COLORREF(GOLD));
            let old_pen = SelectObject(buffer, pen);
            let old_brush = SelectObject(buffer, GetStockObject(NULL_BRUSH));
            let _ = RoundRect(
                buffer,
                cx - unit(12),
                cy - unit(7),
                cx + unit(12),
                cy + unit(9),
                unit(3),
                unit(3),
            );
            let _ = Rectangle(
                buffer,
                cx - unit(6),
                cy - unit(10),
                cx + unit(3),
                cy - unit(7),
            );
            let _ = Ellipse(
                buffer,
                cx - unit(5),
                cy - unit(4),
                cx + unit(5),
                cy + unit(6),
            );
            let _ = Rectangle(
                buffer,
                cx + unit(7),
                cy - unit(4),
                cx + unit(9),
                cy - unit(2),
            );
            SelectObject(buffer, old_brush);
            SelectObject(buffer, old_pen);
            let _ = DeleteObject(pen);
        } else {
            label(
                buffer,
                r,
                &text_of(draw.hwndItem),
                ((r.bottom - r.top) * 42 / 100).clamp(
                    12 * GetDpiForWindow(draw.hwndItem) as i32 / 96,
                    16 * GetDpiForWindow(draw.hwndItem) as i32 / 96,
                ),
                GOLD,
                true,
            );
        }
        if (draw.itemState.0 & ODS_FOCUS.0) != 0 {
            let mut rr = inner;
            let _ = orbit::focus_outline(buffer, &mut rr);
        }
        let _ = BitBlt(
            draw.hDC,
            r.left,
            r.top,
            r.right - r.left,
            r.bottom - r.top,
            buffer,
            r.left,
            r.top,
            SRCCOPY,
        );
        SelectObject(buffer, old);
        let _ = DeleteObject(bitmap);
        let _ = DeleteDC(buffer);
        return LRESULT(1);
    }
    let handled = APP.with(|cell| {
        let Ok(mut b) = cell.try_borrow_mut() else {
            return false;
        };
        let Some(a) = b.as_mut() else {
            return false;
        };
        match msg {
            WM_ACTIVATE if hwnd==a.video => {
                a.fullscreen_cursor_until=if a.fullscreen && (wp.0 as u16)!=WA_INACTIVE as u16 {
                    Some(std::time::Instant::now()+std::time::Duration::from_secs(3))
                }else{None};
                SetCursor(LoadCursorW(None,IDC_ARROW).unwrap_or_default());
                true
            }
            WM_MOUSEWHEEL if hwnd == a.video || hwnd == a.deck => {
                if scrollbars::wheel_combo(item(a.deck,CHANNEL),wp) || scrollbars::wheel_combo(item(a.settings,306),wp){return true;}
                a.wheel_delta += ((wp.0 >> 16) as u16 as i16) as i32;
                while a.wheel_delta.abs() >= 120 {
                    let forward = a.wheel_delta > 0;
                    a.wheel_delta += if forward { -120 } else { 120 };
                    a.command(hwnd, if forward { NEXT } else { PREV }, 0);
                }
                true
            }
            WM_SIZING if hwnd == a.video && !a.fullscreen && lp.0 != 0 => {
                let proposed = &mut *(lp.0 as *mut RECT);
                let dpi = GetDpiForWindow(hwnd);
                let mut border = RECT::default();
                let style = WINDOW_STYLE(GetWindowLongPtrW(hwnd, GWL_STYLE) as u32);
                let exstyle = WINDOW_EX_STYLE(GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32);
                if AdjustWindowRectExForDpi(&mut border, style, false, exstyle, dpi).is_ok() {
                    let edge = wp.0 as u32;
                    let (width, height) = aspect_window_size(
                        (proposed.right - proposed.left, proposed.bottom - proposed.top),
                        matches!(edge, WMSZ_TOP | WMSZ_BOTTOM), dpi as f32 / 96.,
                        (0,0), a.aspect_ratio.dimensions(a.video_aspect));
                    if matches!(edge, WMSZ_LEFT | WMSZ_TOPLEFT | WMSZ_BOTTOMLEFT) { proposed.left = proposed.right - width; }
                    else { proposed.right = proposed.left + width; }
                    if matches!(edge, WMSZ_TOP | WMSZ_TOPLEFT | WMSZ_TOPRIGHT) { proposed.top = proposed.bottom - height; }
                    else { proposed.bottom = proposed.top + height; }
                }
                true
            }
            WM_GETMINMAXINFO if hwnd==a.deck && lp.0!=0 => {
                let m=&mut *(lp.0 as *mut MINMAXINFO);let d=GetDpiForWindow(hwnd) as i32;
                m.ptMinTrackSize=POINT{x:780*d/96,y:254*d/96};true
            }
            WM_SIZING if hwnd==a.deck && lp.0!=0 => {
                let r=&mut *(lp.0 as *mut RECT);let edge=wp.0 as u32;
                if matches!(edge,WMSZ_TOP|WMSZ_BOTTOM) {
                    r.right=r.left+((r.bottom-r.top) as f32*orbit::WIDTH/orbit::HEIGHT).round() as i32;
                } else {
                    let height=((r.right-r.left) as f32*orbit::HEIGHT/orbit::WIDTH).round() as i32;
                    if matches!(edge,WMSZ_TOPLEFT|WMSZ_TOPRIGHT){r.top=r.bottom-height;}else{r.bottom=r.top+height;}
                }
                true
            }
            COMMIT_CHANNEL_SELECTION=>{
                if let Some((combo,index))=a.pending_channel_selection {
                    if SendMessageW(combo,CB_GETDROPPEDSTATE,WPARAM(0),LPARAM(0)).0==0 {
                        a.pending_channel_selection=None;a.choose_channel(index);
                    }
                } else {a.update_channels();}
                true
            }
            WM_DISPLAYCHANGE if hwnd == a.video => {
                if let Some(tx) = &a.native_commands { let _ = tx.send(native::Command::Resize); }
                true
            }
            WM_SIZE => {
                if wp.0 != SIZE_MINIMIZED as usize { a.layout_window(hwnd); }
                true
            }
            WM_TIMER if hwnd==a.settings && wp.0==SIGNAL_STATUS_TIMER => {
                let _=KillTimer(a.settings,SIGNAL_STATUS_TIMER);set_text(item(a.settings,SIGNAL_STATUS),"");true
            }
            WM_TIMER if wp.0==DIAL_TIMER => {
                let _=InvalidateRect(item(a.deck,DIAL),None,false);
                if !a.dial.borrow().animating() || !IsWindowVisible(a.deck).as_bool(){let _=KillTimer(a.deck,DIAL_TIMER);}
                true
            }
            WM_TIMER | WORK_COMPLETE => {
                a.tick();
                true
            }
            WM_HSCROLL if picture::SLIDERS.contains(&(GetDlgCtrlID(HWND(lp.0 as _)) as u16))=>{
                let value=SendMessageW(HWND(lp.0 as _),TBM_GETPOS,WPARAM(0),LPARAM(0)).0 as i32;
                match GetDlgCtrlID(HWND(lp.0 as _)) as u16 {picture::SATURATION=>a.picture.saturation=value,picture::BRIGHTNESS=>a.picture.brightness=value-100,_=>a.picture.contrast=value}
                a.apply_picture();select(item(a.settings,picture::PRESET),a.picture.index());let _=InvalidateRect(a.settings,None,false);true
            }
            WM_HSCROLL if GetDlgCtrlID(HWND(lp.0 as _)) == SEEK as i32 => {
                let kind = wp.0 as u32 & 0xffff;
                if kind == TB_THUMBTRACK {
                    a.dragging = true;
                } else if kind != TB_ENDTRACK {
                    a.dragging = false;
                    let t = a.control.snapshot();
                    if t["timeline"]["seekable"] == true {
                        let p = SendMessageW(HWND(lp.0 as _), TBM_GETPOS, WPARAM(0), LPARAM(0)).0
                            as f64
                            / 10000.;
                        if let Some(tx) = &a.native_commands {
                            let _ = tx.send(native::Command::Seek(
                                p * t["timeline"]["duration"].as_f64().unwrap_or(0.),
                            ));
                        }
                    }
                } else {
                    a.dragging = false;
                }
                true
            }
            WM_COMMAND => {
                a.command(hwnd, (wp.0 & 0xffff) as u16, ((wp.0 >> 16) & 0xffff) as u16);
                true
            }
            WM_CLOSE => {
                a.command(hwnd, CLOSE, 0);
                true
            }
            WM_KEYDOWN => {
                match wp.0 as u32 {
                    0x20 => a.command(hwnd, PLAY, 0),
                    0x1b => a.command(hwnd, if a.fullscreen { FULL } else { STOP }, 0),
                    0x7a => a.command(hwnd, FULL, 0),
                    0x26 => a.command(hwnd, if hwnd==a.video {NEXT}else{VOL_UP}, 0),
                    0x28 => a.command(hwnd, if hwnd==a.video {PREV}else{VOL_DOWN}, 0),
                    0x25 if hwnd==a.video=>a.command(hwnd,VOL_DOWN,0),
                    0x27 if hwnd==a.video=>a.command(hwnd,VOL_UP,0),
                    _ => {}
                }
                true
            }
            WM_DPICHANGED => {
                let r = *(lp.0 as *const RECT);
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    r.left,
                    r.top,
                    r.right - r.left,
                    r.bottom - r.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
                a.layout_window(hwnd);
                true
            }
            WM_GETMINMAXINFO => {
                let p = &mut *(lp.0 as *mut MINMAXINFO);
                let d = GetDpiForWindow(hwnd) as i32;
                p.ptMinTrackSize = POINT {
                    x: (if hwnd == a.video {
                        760
                    } else if hwnd == a.settings {
                        750
                    } else {
                        680
                    }) * d
                        / 96,
                    y: (if hwnd == a.video {
                        460
                    } else if hwnd == a.settings {
                        580
                    } else {
                        320
                    }) * d
                        / 96,
                };
                true
            }
            _ => false,
        }
    });
    if handled {
        LRESULT(if msg == WM_SIZING { 1 } else { 0 })
    } else {
        DefWindowProcW(hwnd, msg, wp, lp)
    }
}

unsafe fn run() -> windows::core::Result<()> {
    windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID(w!("OpenVolarS.LiveTV"))?;
    let language:Value=fs::read(data_dir().join("settings.json")).ok().and_then(|b|serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);i18n::load(language["language"].as_str().unwrap_or("en"));
    InitCommonControlsEx(&INITCOMMONCONTROLSEX {
        dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
        dwICC: ICC_BAR_CLASSES | ICC_TAB_CLASSES,
    })
    .ok()?;
    let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    let scale = GetDpiForSystem() as i32;
    let sp = |v: i32| v * scale / 96;
    let instance = GetModuleHandleW(None)?;
    let class = w!("A865RNativeClassic");
    let (large_icon, small_icon) = app_icon::load(GetDpiForSystem())?;
    RegisterClassExW(&WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(window_proc),
        hInstance: instance.into(),
        lpszClassName: class,
        hCursor: LoadCursorW(None, IDC_ARROW)?,
        hIcon: large_icon,
        hIconSm: small_icon,
        style: CS_HREDRAW | CS_VREDRAW,
        ..Default::default()
    });
    let video = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        class,
        w!("Live TV! — 0.8.0-alpha.43 • Vulkan preview"),
        WS_POPUP | WS_THICKFRAME | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_CLIPCHILDREN,
        80,
        50,
        sp(1120),
        sp(770),
        None,
        None,
        instance,
        None,
    )?;
    app_icon::apply(video)?;
    let _=SetPropW(video,w!("OrbitBorderless"),HANDLE(3 as _));
    viewer::configure_frame(video);
    let _=SetWindowPos(video,None,0,0,0,0,SWP_NOMOVE|SWP_NOSIZE|SWP_NOZORDER|SWP_NOACTIVATE|SWP_FRAMECHANGED);
    let surface = child(video, w!("STATIC"), "", 500, 13u32 | WS_CLIPCHILDREN.0 | WS_CLIPSIBLINGS.0);
    for id in [CHANNEL_OSD,VOLUME_OSD] {
        let h=child(video,w!("STATIC"),"",id,13u32);
        osd::make_transparent(h)?;
        let _=ShowWindow(h,SW_HIDE);
    }
    let deck = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        class,
        w!("Live TV! — Control Panel"),
        WS_POPUP | WS_THICKFRAME | WS_SYSMENU | WS_MINIMIZEBOX | WS_CLIPCHILDREN,
        900,
        240,
        sp(1088),
        sp(354),
        None,
        None,
        instance,
        None,
    )?;
    app_icon::apply(deck)?;
    set_text(deck,"Live TV! — Control Panel");
    let _=SetPropW(deck,w!("OrbitBorderless"),HANDLE(1 as _));
    {
        use windows::Win32::Graphics::Dwm::*;
        let rounding=DWMWCP_ROUND;
        let border=0xfffffffeu32; // DWMWA_COLOR_NONE: no default system outline.
        let _=DwmSetWindowAttribute(deck,DWMWA_WINDOW_CORNER_PREFERENCE,(&rounding as *const DWM_WINDOW_CORNER_PREFERENCE).cast(),4);
        let _=DwmSetWindowAttribute(deck,DWMWA_BORDER_COLOR,(&border as *const u32).cast(),4);
        let _=SetWindowPos(deck,None,0,0,0,0,SWP_NOMOVE|SWP_NOSIZE|SWP_NOZORDER|SWP_NOACTIVATE|SWP_FRAMECHANGED);
    }
    button(deck,"Minimize panel",MIN);
    button(deck,"Close panel",CLOSE);
    // Settings is independent: activating an owned popup also raises its video owner.
    let settings = CreateWindowExW(
        WS_EX_TOOLWINDOW,
        class,
        w!("Live TV! — Settings"),
        WS_POPUP | WS_SYSMENU | WS_CLIPCHILDREN,
        330,
        250,
        sp(750),
        sp(580),
        None,
        None,
        instance,
        None,
    )?;
    app_icon::apply(settings)?;
    set_text(settings,"Live TV! — Settings");
    let _=SetPropW(settings,w!("OrbitBorderless"),HANDLE(2 as _));
    {
        use windows::Win32::Graphics::Dwm::*;
        let rounding=DWMWCP_ROUND;let border=0xfffffffeu32;
        let _=DwmSetWindowAttribute(settings,DWMWA_WINDOW_CORNER_PREFERENCE,(&rounding as *const DWM_WINDOW_CORNER_PREFERENCE).cast(),4);
        let _=DwmSetWindowAttribute(settings,DWMWA_BORDER_COLOR,(&border as *const u32).cast(),4);
        let _=SetWindowPos(settings,None,0,0,0,0,SWP_NOMOVE|SWP_NOSIZE|SWP_NOZORDER|SWP_NOACTIVATE|SWP_FRAMECHANGED);
    }
    button(settings,"Close settings",CLOSE);
    for (id, s) in [
        (PREV, "CH −"),
        (PLAY, "▶ Play"),
        (STOP, "■ Stop"),
        (NEXT, "CH +"),
        (RECORD, "● REC"),
        (SNAP, "Snapshot"),
        (EPG, "EPG"),
        (VOL_DOWN, "VOL −"),
        (VOL_UP, "VOL +"),
        (SETTINGS, "⚙ Settings"),
        (DECK, "Receiver panel"),
        (OPEN, "Open"),
    ] {
        button(video, s, id);
    }
    for (id, s) in [
        (SNAP, "Snapshot"),
        (EPG, "EPG"),
        (OPEN, "Open"),
        (SCAN, "Scan TV"),
        (SETTINGS, "⚙ Settings"),
        (EXPORTS, "Library"),
        (PREV, "CH −"),
        (PLAY, "▶ Play"),
        (STOP, "■ Stop"),
        (NEXT, "CH +"),
        (RECORD, "● REC"),
    ] {
        button(deck, s, id);
    }

    let knob=child(deck,w!("STATIC"),"Volume dial",DIAL,13u32|0x100);
    let _=SetWindowSubclass(knob,Some(dial_proc),1,0);
    button(video, "⛶", FULL);
    for (id,text) in [(MIN,"Minimize"),(viewer::MAXIMIZE,"Maximize / restore"),(CLOSE,"Close"),(viewer::OPTIONS,"Settings")] {button(video,text,id);}
    button(deck,"Fullscreen",FULL);
    for h in [video, deck] {
        for (id,label) in [(BACK,"Previous frame"),(FORWARD,"Next frame"),(LIVE,"Live"),(AUDIO,"Audio"),(CC,"CC On")]{
            let label=if h==deck {match id {BACK=>"Previous frame",FORWARD=>"Next frame",_=>label}}else{label};
            button(h,label,id);
        }
        if h==video {
        let bar = child(
            h,
            TRACKBAR_CLASSW,
            "Stream position",
            SEEK,
            TBS_HORZ | TBS_NOTICKS,
        );
        SendMessageW(bar, TBM_SETRANGEMAX, WPARAM(0), LPARAM(10000));
        SendMessageW(bar, TBM_SETPAGESIZE, WPARAM(0), LPARAM(500));
        let _ = EnableWindow(bar, false);
        child(h, w!("STATIC"), "—", SEEK_TIME, 13u32); // SS_OWNERDRAW
        }
    }
    child(
        deck,
        w!("COMBOBOX"),
        "",
        CHANNEL,
        (CBS_DROPDOWNLIST | CBS_HASSTRINGS | CBS_OWNERDRAWFIXED) as u32 | WS_VSCROLL.0,
    );
    let _=SetWindowSubclass(item(deck,CHANNEL),Some(orbit::channel_proc),2,0);
    for (id, list) in [
        (
            301,
            vec![
                "Native • no enlargement",
                "2K • up to 2560 × 1440",
                "4K • up to 3840 × 2160",
            ],
        ),
        (ASPECT, AspectRatio::ALL.iter().map(|ratio| if *ratio==AspectRatio::Auto {"Auto"}else{ratio.name()}).collect()),
        (SHADER_CONTROL, vec!["Automatic"]),
        (302, backend::Backend::choices(backend::microsoft_available()).iter().map(|b|b.label()).collect()),
        (
            303,
            vec![
                "Smooth • 50 / 59.94 fields per second",
                "Standard • single rate",
                "Off",
            ],
        ),
        (304, vec!["Monitor ICC", "Off"]),
        (orbit::THEME,vec!["Orbit"]),
        (orbit::MATERIAL,vec!["Metal","Glass","Plastic"]),
    ] {
        let h = child(
            settings,
            w!("COMBOBOX"),
            "",
            id,
            (CBS_DROPDOWNLIST | CBS_HASSTRINGS | CBS_OWNERDRAWFIXED) as u32 | WS_VSCROLL.0,
        );
        let _=SetWindowSubclass(h,Some(orbit::field_proc),4,0);
        for s in list {
            combo_add(h, s);
        }
        place(settings, id, 170, 65 + (id as i32 - 301) * 48, 380, 200);
    }
    child(settings, w!("STATIC"), "", 305, 0x8000u32);
    place(settings, 305, 24, 296, 520, 22);
    for (id, s, x, y, width) in [
        (PROFILE, "Choose ICC…", 24, 322, 160),
        (FFMPEG, "Choose FFmpeg…", 200, 322, 175),
    ] {
        button(settings, s, id);
        place(settings, id, x, y, width, 34);
    }
    child(
        settings,
        w!("COMBOBOX"),
        "",
        306,
        (CBS_DROPDOWNLIST | CBS_HASSTRINGS | CBS_OWNERDRAWFIXED) as u32 | WS_VSCROLL.0,
    );
    let _=SetWindowSubclass(item(settings,306),Some(orbit::field_proc),4,0);
    place(settings, 306, 24, 430, 526, 160);
    child(
        settings,
        w!("COMBOBOX"),
        "",
        311,
        (CBS_DROPDOWN | CBS_HASSTRINGS | CBS_OWNERDRAWFIXED) as u32 | WS_VSCROLL.0,
    );
    button(settings, "Save country", 312);
    for (id, text) in [(308, "473.143"), (309, "695.143"), (310, "6.000")] {
        child(
            settings,
            w!("EDIT"),
            text,
            id,
            WS_BORDER.0 | ES_AUTOHSCROLL as u32,
        );
    }
    button(settings, "Discover channels", SCAN);
    place(settings, SCAN, 24, 475, 260, 36);
    button(settings, "Stop scan", STOP);
    place(settings, STOP, 304, 475, 246, 36);
    child(settings, w!("STATIC"), "", 307, 0);
    place(settings, 307, 24, 525, 526, 36);
    let font = CreateFontW(
        -sp(16),
        0,
        0,
        0,
        400,
        0,
        0,
        0,
        DEFAULT_CHARSET.0 as u32,
        OUT_DEFAULT_PRECIS.0 as u32,
        CLIP_DEFAULT_PRECIS.0 as u32,
        CLEARTYPE_QUALITY.0 as u32,
        DEFAULT_PITCH.0 as u32,
        w!("Segoe UI"),
    );
    for (h, id) in [
        (deck, CHANNEL),
        (settings, ASPECT),
        (settings, 301),
        (settings, 302),
        (settings, SHADER_CONTROL),
        (settings, 303),
        (settings, 304),
        (settings, 305),
        (settings, 306),
        (settings, 307),
        (settings, 308),
        (settings, 309),
        (settings, 310),
        (settings, 311),
        (video, SEEK_TIME),
        (deck, SEEK_TIME),
    ] {
        SendMessageW(item(h, id), WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));
    }
    let mut aspect_ratio = AspectRatio::Auto;
    let mut options = Options::default();
    let mut frequency = 521143;
    let mut channels = vec![521143];
    let mut volume = 65;
    let mut audio_mode=audio::Mode::Stereo;
    let mut services = vec![];
    let mut channel_index = 0;
    if let Ok(bytes) = fs::read(data_dir().join("settings.json")) {
        if let Ok(v) = serde_json::from_slice::<Value>(&bytes) {
            aspect_ratio=AspectRatio::parse(v["aspect_ratio"].as_str().unwrap_or("auto"));
            if let Some(f) = v["frequency"]
                .as_u64()
                .and_then(|f| u32::try_from(f).ok())
                .filter(|f| a865r::channel_plan::validate_frequency(*f).is_ok())
            {
                frequency = f;
            }
            if let Some(c) = v["channels"].as_array() {
                channels = c
                    .iter()
                    .filter_map(|f| f.as_u64().and_then(|f| u32::try_from(f).ok()))
                    .filter(|f| a865r::channel_plan::validate_frequency(*f).is_ok())
                    .collect();
            }
            if channels.is_empty() {
                channels.push(frequency);
            }
            services = v["services"].as_array().cloned().unwrap_or_default();
            channel_index =
                (v["channel_index"].as_u64().unwrap_or(0) as usize).min(channels.len() - 1);
            audio_mode=audio::Mode::from_index(v["audio_mode"].as_u64().unwrap_or(0) as usize);
            volume = v["volume"].as_u64().unwrap_or(65).min(100) as u32;
            options.gpu = v["gpu"].as_bool().unwrap_or(true);
            options.resolution =
                Resolution::ALL[v["resolution"].as_u64().unwrap_or(3).min(3) as usize];
            options.deinterlacing = [
                DeinterlaceMode::DoubleRate,
                DeinterlaceMode::SingleRate,
                DeinterlaceMode::Off,
            ][v["smooth"].as_u64().unwrap_or(0).min(2) as usize];
            if let Some(p) = v["ffmpeg"].as_str() {
                options.ffmpeg = p.into();
            }
            if let Some(p) = v["profile"].as_str() {
                options.color_profile = match p {
                    "monitor" => ColorProfile::Monitor,
                    "off" => ColorProfile::Disabled,
                    _ => ColorProfile::File(p.into()),
                };
            }
        }
    }
    let verify_args: Vec<String> = std::env::args().collect();
    if let Some(i) = verify_args.iter().position(|s| s == "--resolution") {
        options.resolution = match verify_args.get(i + 1).map(String::as_str) {
            Some("native") => Resolution::Native,
            Some("2k") => Resolution::Qhd,
            _ => Resolution::Uhd,
        };
    }
    if let Some(i) = verify_args.iter().position(|s| s == "--icc") {
        if let Some(path) = verify_args.get(i + 1) {
            options.color_profile = if path == "off" {
                ColorProfile::Disabled
            } else {
                ColorProfile::File(path.into())
            };
        }
    }
    let mut country = 0;
    let mut profiles = countries::defaults();
    if let Ok(bytes) = fs::read(data_dir().join("countries.json")) {
        if let Ok(v) = serde_json::from_slice::<Value>(&bytes) {
            if let Some(p) = v["profiles"].as_array().filter(|p| !p.is_empty()) {
                profiles = p.clone();
                country = (v["selected"].as_u64().unwrap_or(0) as usize).min(profiles.len() - 1);
            }
        }
    }
    for p in &profiles {
        combo_add(item(settings, 311), p["name"].as_str().unwrap_or("Country"));
    }
    select(item(settings, 311), country);
    clear_country_selection(settings);
    let saved:Value=fs::read(data_dir().join("settings.json")).ok().and_then(|b|serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
    let recording_folder=recordings::load(&saved);
    let snapshot_folder=snapshots::load(&saved);
    let captions_enabled=saved["captions_enabled"].as_bool().unwrap_or(true);
    let caption_window=captions::create(surface)?;
    button(settings,"Choose folder…",RECORD_FOLDER);
    button(settings,"Choose folder…",SNAPSHOT_FOLDER);
    for id in [RECORD_PATH,SNAPSHOT_PATH] {child(settings,w!("EDIT"),"",id,ES_AUTOHSCROLL as u32);}
    child(settings,w!("STATIC"),"",STORAGE_STATUS,0);
    let tabs=child(settings,WC_TABCONTROLW,"Settings categories",orbit::TAB,TCS_OWNERDRAWFIXED|TCS_FIXEDWIDTH);
    let _=SetWindowSubclass(tabs,Some(orbit::tabs_proc),5,0);
    let _=SetWindowSubclass(item(settings,311),Some(orbit::field_proc),4,0);
    for(i,name)in ["Video","Channels","Storage","Themes","Picture","Parental","General"].iter().enumerate(){let mut text=wide(&i18n::text(name));let tab=TCITEMW{mask:TCIF_TEXT,pszText:windows::core::PWSTR(text.as_mut_ptr()),..Default::default()};SendMessageW(tabs,TCM_INSERTITEMW,WPARAM(i),LPARAM((&tab as *const TCITEMW) as isize));}
    for id in [orbit::TAB,orbit::THEME,orbit::MATERIAL,RECORD_PATH,SNAPSHOT_PATH,STORAGE_STATUS]{SendMessageW(item(settings,id),WM_SETFONT,WPARAM(font.0 as usize),LPARAM(1));}
    for &(id,_,_,_) in orbit::BUTTONS {let _=SetWindowSubclass(item(deck,id),Some(orbit::button_proc),3,0);}
    for id in [PROFILE,RECORD_FOLDER,SNAPSHOT_FOLDER,SCAN,STOP,312,CLOSE]{let _=SetWindowSubclass(item(settings,id),Some(orbit::button_proc),3,0);}
    for &id in viewer::CONTROLS {let _=SetWindowSubclass(item(video,id),Some(orbit::button_proc),3,0);}
    child(settings,w!("COMBOBOX"),"Language",i18n::LANGUAGE,CBS_DROPDOWNLIST as u32|CBS_OWNERDRAWFIXED as u32|CBS_HASSTRINGS as u32);
    for name in i18n::NAMES{combo_add(item(settings,i18n::LANGUAGE),name);}
    child(settings,w!("COMBOBOX"),"Start receiver with Windows",startup::ENABLE,CBS_DROPDOWNLIST as u32|CBS_OWNERDRAWFIXED as u32|CBS_HASSTRINGS as u32);
    for name in ["Off","On"]{combo_add(item(settings,startup::ENABLE),name);}
    let _=SetWindowSubclass(item(settings,startup::ENABLE),Some(orbit::field_proc),4,0);
    child(settings,w!("STATIC"),"",startup::STATUS,0);
    child(settings,w!("STATIC"),"",SIGNAL_STATUS,0);
    let signal=child(settings,w!("COMBOBOX"),"AverTV signal",a865r_bda::signal::CONTROL,CBS_DROPDOWNLIST as u32|CBS_OWNERDRAWFIXED as u32|CBS_HASSTRINGS as u32|WS_VSCROLL.0);
    for name in a865r_bda::signal::LABELS{combo_add(signal,name);}
    let _=SetWindowSubclass(signal,Some(orbit::field_proc),4,0);
    SendMessageW(signal,WM_SETFONT,WPARAM(font.0 as usize),LPARAM(1));
    for id in [startup::ENABLE,startup::STATUS,SIGNAL_STATUS]{SendMessageW(item(settings,id),WM_SETFONT,WPARAM(font.0 as usize),LPARAM(1));}
    let _=SetWindowSubclass(item(settings,i18n::LANGUAGE),Some(orbit::field_proc),4,0);
    child(settings,w!("COMBOBOX"),"Video preset",picture::PRESET,CBS_DROPDOWNLIST as u32|CBS_OWNERDRAWFIXED as u32|CBS_HASSTRINGS as u32);
    for name in picture::PRESETS{combo_add(item(settings,picture::PRESET),name);}
    let _=SetWindowSubclass(item(settings,picture::PRESET),Some(orbit::field_proc),4,0);
    for id in picture::SLIDERS {
        let slider=child(settings,TRACKBAR_CLASSW,match id {picture::SATURATION=>"Saturation",picture::BRIGHTNESS=>"Brightness",_=>"Contrast"},id,TBS_NOTICKS);
        SendMessageW(slider,TBM_SETRANGE,WPARAM(1),LPARAM((200<<16) as isize));SendMessageW(slider,TBM_SETPAGESIZE,WPARAM(0),LPARAM(5));
    }
    button(settings,"Reset defaults",picture::RESET);
    let _=SetWindowSubclass(item(settings,picture::RESET),Some(orbit::button_proc),3,0);
    button(settings,"HDR effect: Off",picture::EFFECT);
    let _=SetWindowSubclass(item(settings,picture::EFFECT),Some(orbit::button_proc),3,0);
    for id in [picture::EFFECT,picture::PRESET]{SendMessageW(item(settings,id),WM_SETFONT,WPARAM(font.0 as usize),LPARAM(1));}
    parental::load(saved["parental"].as_object().map(|_|saved["parental"].clone()).unwrap_or(json!({})));
    for id in [parental::AUTH,parental::NEW,parental::CONFIRM]{child(settings,w!("EDIT"),"",id,ES_PASSWORD as u32|ES_AUTOHSCROLL as u32);SendMessageW(item(settings,id),EM_SETLIMITTEXT,WPARAM(128),LPARAM(0));}
    for id in [parental::ENABLE,parental::UNRATED,parental::RATING,parental::CHANNEL]{child(settings,w!("COMBOBOX"),"",id,CBS_DROPDOWNLIST as u32|CBS_OWNERDRAWFIXED as u32|CBS_HASSTRINGS as u32);let _=SetWindowSubclass(item(settings,id),Some(orbit::field_proc),4,0);}
    for id in [parental::ENABLE,parental::UNRATED]{for name in ["Off","On"]{combo_add(item(settings,id),name);}}
    for name in ["All ages","10","12","14","16","18"]{combo_add(item(settings,parental::RATING),name);}
    for (id,name) in [(parental::UNLOCK,"Unlock"),(parental::LOCK,"Lock / unlock")]{button(settings,name,id);let _=SetWindowSubclass(item(settings,id),Some(orbit::button_proc),3,0);}
    child(settings,w!("STATIC"),"",parental::STATUS,0);
    for id in 350..=360 {SendMessageW(item(settings,id),WM_SETFONT,WPARAM(font.0 as usize),LPARAM(1));}
    for id in [308,309,310,parental::AUTH,parental::NEW,parental::CONFIRM]{
        let h=item(settings,id);let _=SetWindowSubclass(h,Some(orbit::text_field_proc),6,0);
        SetWindowLongPtrW(h,GWL_STYLE,GetWindowLongPtrW(h,GWL_STYLE)|WS_BORDER.0 as isize);
        let _=SetWindowPos(h,None,0,0,0,0,SWP_NOMOVE|SWP_NOSIZE|SWP_NOZORDER|SWP_NOACTIVATE|SWP_FRAMECHANGED);
    }
    SendMessageW(item(settings,parental::CHANNEL),CB_SETDROPPEDWIDTH,WPARAM(350*GetDpiForWindow(settings) as usize/96),LPARAM(0));
    let mut app = App {
        video,
        deck,
        surface,
        surface_backend: None,
        settings,
        font,
        skin:orbit::Skin::new(),
        viewer:viewer::Skin::new(),
        material:orbit::Material::parse(saved["button_material"].as_str().unwrap_or("metal")),
        settings_tab:0,
        avertv_signal:a865r_bda::signal::load_at(&a865r_bda::signal::config_path()).map(|c|c.mode).unwrap_or(0),
        receiver_startup:startup::enabled(std::env::args().any(|s|s=="--ui-preview"),saved["receiver_startup"].as_bool().unwrap_or(false)),
        picture:picture::Picture::load(&saved["picture"]),
        // The removed output switch must not leave an invisible HDR mode enabled.
        video_hdr:false,
        parental_edit:parental::config()["password"].is_null(),parental_draft:parental::config(),
        options,
        backend:backend::Backend::load(saved["processing_backend"].as_str(),backend::microsoft_available()),
        shader:backend::Shader::load(saved["shader_acceleration"].as_str()).compatible(backend::Backend::load(saved["processing_backend"].as_str(),backend::microsoft_available()),backend::capabilities()),
        frequency,
        channels,
        services,
        channel_index,
        volume,
        audio_mode,
        captions_enabled,caption_window,caption_revision:u64::MAX,recording_folder,snapshot_folder,pending_snapshot:None,pending_channel_selection:None,recording_path:None,
        dial: RefCell::new(dial::Dial::new(volume).map_err(|e|windows::core::Error::new(E_FAIL,e.to_string()))?),
        control: Control::default(),
        rx: None,
        current: None,
        pending: None,
        status: "Ready • Scan TV or open a recording.".into(),
        folder: PathBuf::new(),
        pipe: String::new(),
        native_commands: None,
        paused: false,
        dragging: false,
        paint_stamp: String::new(),
        wheel_delta: 0,
        video_aspect: (16, 9),
        aspect_ratio,
        channel_osd_until: None,
        channel_digits: String::new(),
        channel_entry_until: None,
        volume_osd_until: None,
        osd_on_first_frame: false,
        verification: None,
        countries: profiles,
        country,
        closing: false,
        quality: None,
        fullscreen: false,
        fullscreen_button_until: None,
        fullscreen_pointer: None,
        fullscreen_cursor_until: None,
        restore: RECT::default(),
        restore_style: 0,
        restore_maximized: false,
        restore_deck: false,
    };
    // Recover services already discovered by earlier builds, including interrupted scans.
    if let Ok(entries) = fs::read_dir(data_dir().join("sessions")) {
        let mut dirs: Vec<_> = entries.filter_map(Result::ok).collect();
        dirs.sort_by_key(|d| d.file_name());
        for dir in dirs {
            if let Ok(bytes) = fs::read(dir.path().join("scan.json")) {
                if let Ok(stations) = serde_json::from_slice::<Vec<Value>>(&bytes) {
                    let found: Vec<Value> = stations
                        .iter()
                        .flat_map(|s| s["services"].as_array().cloned().unwrap_or_default())
                        .collect();
                    app.merge_channels(&found);
                }
            }
        }
    }
    // Center the streaming window and control deck as one vertical group.
    // Use the work area to leave the taskbar visible; place before showing either window.
    let mut screen = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    if GetMonitorInfoW(MonitorFromWindow(deck, MONITOR_DEFAULTTONEAREST), &mut screen).as_bool() {
        let r = screen.rcWork;
        let [stream, panel] = window_placement::centered_pair(
            window_placement::Rect { x: r.left, y: r.top, width: r.right-r.left, height: r.bottom-r.top },
            GetDpiForWindow(deck),
        );
        for (hwnd, r) in [(video, stream), (deck, panel)] {
            let _ = SetWindowPos(hwnd, None, r.x, r.y, r.width, r.height, SWP_NOZORDER | SWP_NOACTIVATE);
        }
    }
    app.layout();
    app.update_channels();
    app.fill_settings();
    app.country_fields();
    let _=EnableWindow(item(settings,302),backend::microsoft_available());

    APP.with(|a| *a.borrow_mut() = Some(app));
    SetTimer(video, 1, 300, None);
    if !std::env::args().any(|s| s == "--verify-hidden") {
    let _ = ShowWindow(video, SW_SHOW);
    let _ = ShowWindow(deck, SW_SHOW);
    // Keep the overlapping control panel in front at startup without making it topmost.
    let _ = SetWindowPos(deck, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    let _ = UpdateWindow(video);
    let _ = UpdateWindow(deck);
    }
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|s| s == "--fullscreen") {
        APP.with(|a| a.borrow_mut().as_mut().unwrap().command(video, FULL, 0));
    }
    if let Some(i) = args.iter().position(|s| s == "--play") {
        if let Some(p) = args.get(i + 1) {
            APP.with(|a| a.borrow_mut().as_mut().unwrap().start(Job::File(p.into())));
        }
    }
    if let Some(i) = args.iter().position(|s| s == "--verify-output") {
        if let Some(path) = args.get(i + 1) {
            APP.with(|a| {
                a.borrow_mut().as_mut().unwrap().verification =
                    Some((std::time::Instant::now(), 0, path.into()))
            });
        }
    }
    if !args.iter().any(|s| s == "--play" || s=="--ui-preview") {
        APP.with(|a| {
            a.borrow_mut()
                .as_mut()
                .unwrap()
                .start(if args.iter().any(|s| s == "--verify-record") {
                    Job::Record
                } else {
                    Job::Watch
                })
        });
    }
    // Optional app-owned resize regression; no desktop automation or tuner access.
    let resize_test = args.iter().any(|a| a == "--verify-resize");
    let resize_started = std::time::Instant::now();
    let mut resize_samples = Vec::new();
    let mut resize_original = RECT::default();
    if resize_test {
        let _ = GetWindowRect(video, &mut resize_original);
        SetTimer(video, 0x4242, 16, None);
    }
    let mut msg = MSG::default();
    loop {
        let result = GetMessageW(&mut msg, None, 0, 0).0;
        if result <= 0 {
            break;
        }
        if resize_test && msg.message == WM_TIMER && msg.wParam.0 == 0x4242 {
            if resize_started.elapsed().as_secs() < 5 { continue; }
            if resize_samples.len() < 180 {
                let n = resize_samples.len() as i32;
                let width = 900 + if n < 90 { n*5 } else { (179-n)*5 };
                let (width,height) = viewer::aspect_size((width,0), false, 1., (16,9));
                let started = std::time::Instant::now();
                let _ = SetWindowPos(video,None,resize_original.left,resize_original.top,width,height,
                    SWP_NOZORDER|SWP_NOACTIVATE);
                // Explicitly exercise chrome painting even for a hidden diagnostic window.
                APP.with(|cell| {
                    if let Some(app) = cell.borrow().as_ref() {
                        let dc = CreateCompatibleDC(None);
                        let mut info = BITMAPINFO::default();
                        info.bmiHeader = BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                            biWidth: width, biHeight: -height, biPlanes: 1, biBitCount: 32,
                            biCompression: BI_RGB.0, ..Default::default() };
                        let mut bits = std::ptr::null_mut();
                        if let Ok(bitmap) = CreateDIBSection(dc,&info,DIB_RGB_COLORS,&mut bits,None,0) {
                            let old = SelectObject(dc,bitmap);
                            let v = viewer::Layout::new(width,height).surface;
                            ExcludeClipRect(dc,v.left,v.top,v.right,v.bottom);
                            app.viewer.paint(app,dc,client(video));
                            let _=GdiFlush();
                            SelectObject(dc,old);let _=DeleteObject(bitmap);
                        }
                        let _=DeleteDC(dc);
                    }
                });
                resize_samples.push(started.elapsed().as_secs_f64()*1000.);
            } else {
                let _=KillTimer(video,0x4242);
                let r=resize_original;
                let _=SetWindowPos(video,None,r.left,r.top,r.right-r.left,r.bottom-r.top,SWP_NOZORDER|SWP_NOACTIVATE);
                resize_samples.sort_by(f64::total_cmp);
                let report=json!({"resizes":resize_samples.len(),"mean_layout_and_chrome_ms":resize_samples.iter().sum::<f64>()/resize_samples.len() as f64,
                    "p95_layout_and_chrome_ms":resize_samples[resize_samples.len()*95/100],"maximum_ms":resize_samples.last()});
                let _=fs::write(data_dir().join("resize-verification.json"),report.to_string());
            }
            continue;
        }
        if msg.message==WM_MOUSEMOVE {
            APP.with(|cell| {
                if let Some(app)=cell.borrow_mut().as_mut() {
                    if msg.hwnd==app.video || IsChild(app.video,msg.hwnd).as_bool() {
                        let mut point=POINT{x:msg.lParam.0 as u16 as i16 as i32,y:(msg.lParam.0>>16) as u16 as i16 as i32};
                        let _=ClientToScreen(msg.hwnd,&mut point);
                        app.fullscreen_pointer_activity(point);
                    }
                }
            });
        }
        // Stream shortcuts also apply while its buttons or seek slider have focus.
        if msg.message==WM_KEYDOWN && matches!(msg.wParam.0,0x25..=0x28)
            && GetKeyState(VK_CONTROL.0 as i32)>=0 && GetKeyState(VK_MENU.0 as i32)>=0 && GetKeyState(VK_SHIFT.0 as i32)>=0 {
            let handled=APP.with(|cell|{let mut state=cell.borrow_mut();let Some(app)=state.as_mut() else{return false;};
                if msg.hwnd!=app.video && !IsChild(app.video,msg.hwnd).as_bool() {return false;}
                let action=match msg.wParam.0 {0x26=>NEXT,0x28=>PREV,0x25=>VOL_DOWN,_=>VOL_UP};
                app.command(app.video,action,0);true
            });
            if handled {continue;}
        }
        // Number entry works from both player windows and their child controls,
        // but never consumes text typed in Settings or an open channel list.
        if msg.message==WM_KEYDOWN && GetKeyState(VK_CONTROL.0 as i32)>=0 && GetKeyState(VK_MENU.0 as i32)>=0 && GetKeyState(VK_SHIFT.0 as i32)>=0 {
            let handled=APP.with(|cell| {
                let mut state=cell.borrow_mut();
                let Some(app)=state.as_mut() else {return false;};
                if !(msg.hwnd==app.video || msg.hwnd==app.deck || IsChild(app.video,msg.hwnd).as_bool() || IsChild(app.deck,msg.hwnd).as_bool()) {return false;}
                if SendMessageW(item(app.deck,CHANNEL),CB_GETDROPPEDSTATE,WPARAM(0),LPARAM(0)).0!=0 {return false;}
                app.channel_key(msg.wParam.0,(msg.lParam.0 & (1<<30))!=0)
            });
            if handled {continue;}
        }
        // Native child buttons keep keyboard focus after clicking fullscreen.
        // Route these shortcuts at the message loop so they work from the video
        // surface and all player controls, not only the top-level window.
        if msg.message == WM_KEYDOWN && (msg.wParam.0 == 0x7a || msg.wParam.0 == 0x1b) {
            let handled = APP.with(|cell| {
                let mut state = cell.borrow_mut();
                let Some(app) = state.as_mut() else {
                    return false;
                };
                if msg.hwnd != app.video && !IsChild(app.video, msg.hwnd).as_bool() {
                    return false;
                }
                if msg.wParam.0 == 0x7a || app.fullscreen {
                    app.command(app.video, FULL, 0);
                    return true;
                }
                false
            });
            if handled {
                continue;
            }
        }
        if (msg.hwnd==settings || IsChild(settings,msg.hwnd).as_bool()) && IsDialogMessageW(settings,&msg).as_bool(){continue;}
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
    APP.with(|a| {
        if let Some(app) = a.borrow_mut().take() {
            app.save();
            let _ = DeleteObject(app.font);
        }
    });
    guide::close();
    Ok(())
}
fn settings_status<'a>(job: Option<&Job>, status: &'a str) -> &'a str {
    let playback=matches!(job,Some(Job::Watch|Job::Record|Job::File(_)));
    if status.starts_with("Preparing playback") && !playback {
        return "";
    }
    if matches!(job,Some(Job::Scan(_))) || (!status.is_empty()
        && !status.starts_with("Recording")
        && !status.starts_with("Saved recording")
        && !status.starts_with("Ready")) { status } else { "" }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str)==Some("--capture-only") {
        std::process::exit(capture_only::run(&args[2..],&data_dir()));
    }
    if args.get(1).map(String::as_str)==Some("--repair-recording") {
        let result=if args.len()>=5 {
            let control=Control::default();
            recording_finalize::finish(PathBuf::from(&args[2]).as_path(),PathBuf::from(&args[3]).as_path(),args[4].parse().ok(),&Options::default().ffmpeg,&data_dir().join("recording-repair"),&control)
        }else{Err("Usage: --repair-recording INPUT.ts OUTPUT.ts PROGRAM_ID --profile-dir DIRECTORY".into())};
        let ok=result.is_ok();let report=result.unwrap_or_else(|e|json!({"success":false,"error":e}));
        let _=fs::create_dir_all(data_dir());let _=fs::write(data_dir().join("recording-repair-result.json"),report.to_string());
        std::process::exit(if ok{0}else{1});
    }
    if args.iter().any(|s|s=="--receiver-startup") {
        std::process::exit(startup::background(&data_dir(),args.iter().any(|s|s=="--ui-preview")));
    }
    if args.get(1).map(String::as_str) == Some("--inspect-stream") {
        if let (Some(input), Some(output)) = (args.get(2), args.get(3)) {
            let data = fs::read(input).unwrap();
            let result = television::discover_ts(&data, 521143);
            fs::write(output, json!(result).to_string()).unwrap();
        }
        return;
    }
    unsafe {
        if let Err(e) = run() {
            let s = wide(&format!("Could not start Live TV!: {e}"));
            MessageBoxW(
                None,
                PCWSTR(s.as_ptr()),
                w!("Live TV!"),
                MB_OK | MB_ICONERROR,
            );
        }
    }
}

#[cfg(test)]
mod resize_tests {
    #[test]
    fn typed_channel_uses_displayed_number_and_accepts_leading_zero() {
        let services=vec![serde_json::json!({"channel_number":7}),serde_json::json!({"channel_number":4}),serde_json::json!({})];
        assert_eq!(super::channel_match(&services,3,"04"),Some(1));
        assert_eq!(super::channel_match(&services,3,"7"),Some(0));
        assert_eq!(super::channel_match(&services,3,"03"),Some(2));
        for invalid in ["", "00", "99", "-1", "x"] {
            assert_eq!(super::channel_match(&services,3,invalid),None);
        }
    }
    #[test]
    fn dragging_preserves_video_ratio_across_dpi_and_console_sizes() {
        for dpi in [1., 1.25, 1.5, 2.] {
            let px = |v: i32| (v as f32 * dpi) as i32;
            let frame = (0,0);
            for aspect in [(16, 9), (4, 3), (16, 10), (5, 4)] {
                for vertical in [false, true] {
                    for size in [(800, 600), (1120, 740), (1500, 900)] {
                        let (w, h) = super::aspect_window_size((px(size.0), px(size.1)), vertical, dpi, frame, aspect);
                        let surface=super::viewer::Layout::new(w,h).surface;
                        let video_w=surface.right-surface.left;let video_h=surface.bottom-surface.top;
                        assert!((video_h as f64-video_w as f64*aspect.1 as f64/aspect.0 as f64).abs()<=0.51);
                        assert!(w >= px(760) && h >= px(460));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod playback_status_tests {
    #[test]
    fn settings_controls_keep_hdr_startup_and_country_independent() {
        use super::*;
        let ids = [picture::PRESET,picture::SATURATION,picture::BRIGHTNESS,picture::CONTRAST,
            picture::RESET,picture::EFFECT,i18n::LANGUAGE,startup::ENABLE,startup::STATUS,SIGNAL_STATUS,SHADER_CONTROL,a865r_bda::signal::CONTROL,
            parental::AUTH,parental::NEW,parental::CONFIRM,parental::UNLOCK,parental::ENABLE,
            parental::UNRATED,parental::RATING,parental::CHANNEL,parental::LOCK,parental::STATUS];
        assert_eq!(ids.iter().copied().collect::<std::collections::HashSet<_>>().len(),ids.len());
        unsafe {
            let host=CreateWindowExW(WINDOW_EX_STYLE(0),w!("STATIC"),w!("Settings regression"),
                WS_POPUP,0,0,750,580,None,None,GetModuleHandleW(None).unwrap(),None).unwrap();
            let hdr=button(host,"HDR effect: Off",picture::EFFECT);
            let startup=child(host,w!("COMBOBOX"),"",startup::ENABLE,CBS_DROPDOWNLIST as u32);
            assert_eq!(item(host,picture::EFFECT),hdr);
            assert_eq!(item(host,startup::ENABLE),startup);
            let _=ShowWindow(startup,SW_HIDE);
            assert_ne!(GetWindowLongPtrW(hdr,GWL_STYLE) as u32 & WS_VISIBLE.0,0);
            assert_eq!(text_of(hdr),i18n::text("HDR effect: Off"));
            let country=child(host,w!("COMBOBOX"),"",311,CBS_DROPDOWN as u32);
            combo_add(country,"Brazil");select(country,0);
            let mut info=COMBOBOXINFO {cbSize: std::mem::size_of::<COMBOBOXINFO>() as u32,..Default::default()};
            GetComboBoxInfo(country,&mut info).unwrap();
            SendMessageW(info.hwndItem,EM_SETSEL,WPARAM(0),LPARAM(-1));
            clear_country_selection(host);
            let (mut start,mut end)=(0u32,0u32);
            SendMessageW(info.hwndItem,EM_GETSEL,WPARAM((&mut start as *mut u32) as usize),LPARAM((&mut end as *mut u32) as isize));
            assert_eq!(start,end);
            assert_eq!(text_of(country),"Brazil");
            let _=DestroyWindow(host);
        }
    }

    use super::{settings_status,Job};
    #[test]
    fn inactive_and_scan_jobs_never_show_preparing_playback() {
        assert_eq!(settings_status(None,"Preparing playback…"),"");
        assert_eq!(settings_status(Some(&Job::Scan(vec![])),"Preparing playback…"),"");
        assert_eq!(settings_status(Some(&Job::Watch),"Preparing playback…"),"Preparing playback…");
    }
    #[test]
    fn cleared_or_finished_progress_clears_the_settings_label() {
        assert_eq!(settings_status(Some(&Job::Watch),""),"");
        assert_eq!(settings_status(None,"Ready - choose a channel"),"");
        assert_eq!(settings_status(None,"Saved recording: test.ts"),"");
        assert_eq!(settings_status(None,"Tuner unavailable"),"Tuner unavailable");
        assert_eq!(settings_status(Some(&Job::Scan(vec![])),"Scanning channels"),"Scanning channels");
    }
}
