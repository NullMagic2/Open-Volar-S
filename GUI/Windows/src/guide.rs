//! Native program guide; populated passively from the same stream as playback.
use serde_json::Value;
use std::{cell::RefCell, fs};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*, Controls::*, Shell::{SetWindowSubclass}},
    },
};
thread_local! {static GUIDE:RefCell<State>=RefCell::new(State::default());}
#[derive(Default)]
struct State {
    window: HWND,
    font: HFONT,
    events: Vec<Value>,
    rows: Vec<Value>,
    filter: usize,
    channels: Vec<(u64, u64, String)>,
    last_ingest: Vec<Value>,
}
const LAYOUT: u32 = WM_APP + 20;
const FILTER: u16 = 601;
const LIST: u16 = 602;
const DETAIL: u16 = 603;
const INFO: u16 = 604;
fn key(v: &Value) -> (u64, u64, u64, u64, u64) {
    (
        v["frequency_khz"].as_u64().unwrap_or(0),
        v["network_id"].as_u64().unwrap_or(0),
        v["transport_id"].as_u64().unwrap_or(0),
        v["program_id"].as_u64().unwrap_or(0),
        v["event_id"].as_u64().unwrap_or(0),
    )
}
fn station(v: &Value) -> String {
    let number = v["channel_number"]
        .as_u64()
        .map(|n| n.to_string())
        .unwrap_or_else(|| "—".into());
    format!("{number} – {}", v["channel_name"].as_str().unwrap_or("TV"))
}
fn date(seconds: i64) -> String {
    // Proleptic Gregorian civil date; values are station clock time, not PC time.
    let days = seconds.div_euclid(86400);
    let time = seconds.rem_euclid(86400);
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    format!(
        "{day:02}/{month:02} {:02}:{:02}",
        time / 3600,
        time / 60 % 60
    )
}
fn range(v: &Value) -> String {
    let start = v["start"].as_i64().unwrap_or(0);
    let end = start + v["duration"].as_i64().unwrap_or(0);
    format!("{} – {}", date(start), date(end))
}
pub fn ingest(events: &Value) {
    let Some(events) = events.as_array().filter(|v| !v.is_empty()) else {
        return;
    };
    GUIDE.with(|cell| {
        let Ok(mut s) = cell.try_borrow_mut() else {
            return;
        };
        let mut changed = false;
        if s.last_ingest == *events {
            return;
        }
        s.last_ingest = events.clone();
        if s.events.is_empty() {
            s.events = fs::read(super::data_dir().join("epg.json"))
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
                .unwrap_or_default();
        }
        for event in events {
            if !event.is_object() {
                continue;
            }
            if let Some(old) = s.events.iter_mut().find(|old| key(old) == key(event)) {
                if old != event {
                    *old = event.clone();
                    changed = true;
                }
            } else {
                s.events.push(event.clone());
                changed = true;
            }
        }
        if changed {
            s.events.sort_by_key(|v| v["start"].as_i64().unwrap_or(0));
            if s.events.len() > 8192 {
                let n = s.events.len() - 8192;
                s.events.drain(..n);
            }
            let _ = fs::write(
                super::data_dir().join("epg.json"),
                serde_json::to_vec(&s.events).unwrap_or_default(),
            );
            unsafe {
                if !s.window.is_invalid() && IsWindowVisible(s.window).as_bool() {
                    s.refresh();
                }
            }
        }
    });
}
pub unsafe fn show(owner: HWND) -> Result<()> {
    GUIDE.with(|cell| -> Result<()> {
        let mut s = cell
            .try_borrow_mut()
            .map_err(|_| Error::new(E_UNEXPECTED, "Program guide is updating; try again"))?;
        if s.window.is_invalid() || !IsWindow(s.window).as_bool() {
            let instance = GetModuleHandleW(None)?;
            let class = w!("A865RProgramGuide");
            RegisterClassW(&WNDCLASSW {
                lpfnWndProc: Some(proc),
                hInstance: instance.into(),
                lpszClassName: class,
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                ..Default::default()
            });
            let d = (GetDpiForWindow(owner) as i32).max(96);
            let px = |v: i32| v * d / 96;
            s.window = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class,
                w!("Live TV! — Program guide"),
                WS_POPUP | WS_THICKFRAME | WS_SYSMENU | WS_CLIPCHILDREN,
                220,
                140,
                px(920),
                px(600),
                None,
                None,
                instance,
                None,
            )?;
            control(
                s.window,
                w!("COMBOBOX"),
                "Channel",
                FILTER,
                (CBS_DROPDOWNLIST | CBS_HASSTRINGS | CBS_OWNERDRAWFIXED) as u32 | WS_VSCROLL.0,
            )?;
            control(
                s.window,
                w!("LISTBOX"),
                "Programs",
                LIST,
                (LBS_NOTIFY | LBS_NOINTEGRALHEIGHT | LBS_OWNERDRAWFIXED | LBS_HASSTRINGS) as u32 | WS_VSCROLL.0,
            )?;
            control(
                s.window,
                w!("EDIT"),
                "",
                DETAIL,
                (ES_MULTILINE | ES_READONLY | ES_AUTOVSCROLL) as u32 | WS_VSCROLL.0,
            )?;
            control(s.window, w!("STATIC"), "", INFO, 0)?;
            super::set_text(s.window,"Live TV! — Program guide");
            let _=SetPropW(s.window,w!("OrbitBorderless"),HANDLE(2 as _));
            {
                use windows::Win32::Graphics::Dwm::*;
                let rounding=DWMWCP_ROUND;let border=0xfffffffeu32;
                let _=DwmSetWindowAttribute(s.window,DWMWA_WINDOW_CORNER_PREFERENCE,(&rounding as *const DWM_WINDOW_CORNER_PREFERENCE).cast(),4);
                let _=DwmSetWindowAttribute(s.window,DWMWA_BORDER_COLOR,(&border as *const u32).cast(),4);
            }
            super::button(s.window,"Close program guide",super::CLOSE);
            let _=SetWindowSubclass(super::item(s.window,super::CLOSE),Some(super::orbit::button_proc),3,0);
            let _=SetWindowSubclass(super::item(s.window,FILTER),Some(super::orbit::field_proc),4,0);
            for id in [LIST,DETAIL] {
                crate::scrollbars::attach(super::item(s.window,id));
            }

            s.font = CreateFontW(
                -px(15),
                0,
                0,
                0,
                400,
                0,
                0,
                0,
                DEFAULT_CHARSET.0 as u32,
                0,
                0,
                CLEARTYPE_QUALITY.0 as u32,
                0,
                w!("Segoe UI"),
            );
            for id in [FILTER, LIST, DETAIL, INFO] {
                SendMessageW(
                    super::item(s.window, id),
                    WM_SETFONT,
                    WPARAM(s.font.0 as usize),
                    LPARAM(0),
                );
            }
            if s.events.is_empty() {
                s.events = fs::read(super::data_dir().join("epg.json"))
                    .ok()
                    .and_then(|b| serde_json::from_slice(&b).ok())
                    .unwrap_or_default();
            }
        }
        s.layout();
        s.refresh();
        let window = s.window;
        let mut panel=RECT::default();let _=GetWindowRect(owner,&mut panel);
        if !owner.is_invalid() {
            let r=super::client(window);let mut mi=MONITORINFO{cbSize:std::mem::size_of::<MONITORINFO>() as u32,..Default::default()};
            let _=GetMonitorInfoW(MonitorFromWindow(owner,MONITOR_DEFAULTTONEAREST),&mut mi);
            let x=((panel.left+panel.right-r.right)/2).clamp(mi.rcWork.left,(mi.rcWork.right-r.right).max(mi.rcWork.left));
            let y=((panel.top+panel.bottom-r.bottom)/2).clamp(mi.rcWork.top,(mi.rcWork.bottom-r.bottom).max(mi.rcWork.top));
            let _=SetWindowPos(window,None,x,y,0,0,SWP_NOSIZE|SWP_NOZORDER|SWP_NOACTIVATE|SWP_FRAMECHANGED);
        }
        drop(s);
        let _ = ShowWindow(window, SW_SHOW);
        shape(window);
        let _ = SetForegroundWindow(window);
        Ok(())
    })
}
unsafe fn control(parent: HWND, class: PCWSTR, label: &str, id: u16, style: u32) -> Result<HWND> {
    let text = super::wide(&crate::i18n::text(label));
    CreateWindowExW(
        WINDOW_EX_STYLE(0),
        class,
        PCWSTR(text.as_ptr()),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(style),
        0,
        0,
        10,
        10,
        parent,
        HMENU(id as usize as _),
        GetModuleHandleW(None)?,
        None,
    )
}
pub unsafe fn hide() {
    let window = GUIDE.with(|s| s.try_borrow().ok().map(|s| s.window));
    if let Some(window) = window {
        let _ = ShowWindow(window, SW_HIDE);
    }
}
pub unsafe fn verify_select() {
    let window = GUIDE.with(|s| s.try_borrow().ok().map(|s| s.window));
    if let Some(window) = window {
        if IsWindow(window).as_bool() {
            super::select(super::item(window, FILTER), 1);
            let _ = PostMessageW(
                window,
                WM_COMMAND,
                WPARAM(FILTER as usize | ((CBN_SELCHANGE as usize) << 16)),
                LPARAM(0),
            );
        }
    }
}
pub unsafe fn diagnostics() -> Value {
    GUIDE.with(|s|s.try_borrow().map(|s|serde_json::json!({"exists":IsWindow(s.window).as_bool(),"visible":IsWindowVisible(s.window).as_bool(),"events":s.events.len(),"rows":s.rows.len(),"filter":s.filter})).unwrap_or(Value::Null))
}
impl State {
    unsafe fn layout(&self) {
        let r = super::client(self.window);
        let d = GetDpiForWindow(self.window) as i32;
        let p = |v: i32| v * d / 96;
        shape(self.window);
        super::place(self.window,super::CLOSE,r.right-p(64),p(9),p(30),p(30));
        let filter=super::item(self.window,FILTER);
        SendMessageW(filter,CB_SETITEMHEIGHT,WPARAM(usize::MAX),LPARAM(p(28) as isize));
        SendMessageW(filter,CB_SETITEMHEIGHT,WPARAM(0),LPARAM(p(28) as isize));
        SendMessageW(super::item(self.window,LIST),LB_SETITEMHEIGHT,WPARAM(0),LPARAM(p(29) as isize));
        super::place(self.window,FILTER,p(28),p(64),r.right-p(56),p(220));
        super::place(self.window,INFO,p(30),p(107),r.right-p(60),p(24));
        let top=p(146);
        let h=((r.bottom-p(160)-top)*2/3).max(p(100));
        super::place(self.window,LIST,p(30),top,r.right-p(60),h);
        super::place(self.window,DETAIL,p(30),top+h+p(20),r.right-p(60),(r.bottom-top-h-p(52)).max(p(60)));
        let edit=super::item(self.window,DETAIL);let area=super::client(edit);
        let inset=RECT{left:p(10),top:p(8),right:area.right-p(10),bottom:area.bottom-p(8)};
        SendMessageW(edit,EM_SETRECT,WPARAM(0),LPARAM((&inset as *const RECT) as isize));
        let _=InvalidateRect(self.window,None,false);
    }
    unsafe fn refresh(&mut self) {
        let filter = super::item(self.window, FILTER);
        let mut channels: Vec<_> = self
            .events
            .iter()
            .map(|v| {
                (
                    v["frequency_khz"].as_u64().unwrap_or(0),
                    v["program_id"].as_u64().unwrap_or(0),
                    station(v),
                )
            })
            .collect();
        channels.sort_by(|a, b| a.2.cmp(&b.2));
        channels.dedup();
        if channels != self.channels
            || SendMessageW(filter, CB_GETCOUNT, WPARAM(0), LPARAM(0)).0 == 0
        {
            let selected = self
                .filter
                .checked_sub(1)
                .and_then(|i| self.channels.get(i))
                .map(|c| (c.0, c.1));
            self.channels = channels;
            self.filter = selected
                .and_then(|c| self.channels.iter().position(|v| (v.0, v.1) == c))
                .map(|i| i + 1)
                .unwrap_or(0);
            SendMessageW(filter, CB_RESETCONTENT, WPARAM(0), LPARAM(0));
            super::combo_add(filter, &crate::i18n::text("All received channels"));
            for c in &self.channels {
                super::combo_add(filter, &c.2);
            }
            super::select(filter, self.filter);
        }
        let list = super::item(self.window, LIST);
        let selected = SendMessageW(list, LB_GETCURSEL, WPARAM(0), LPARAM(0)).0;
        let selected = usize::try_from(selected)
            .ok()
            .and_then(|i| self.rows.get(i))
            .map(key);
        self.rows = self
            .events
            .iter()
            .filter(|v| {
                self.filter == 0
                    || self.channels.get(self.filter - 1).is_some_and(|c| {
                        v["frequency_khz"].as_u64() == Some(c.0)
                            && v["program_id"].as_u64() == Some(c.1)
                    })
            })
            .cloned()
            .collect();
        SendMessageW(list, WM_SETREDRAW, WPARAM(0), LPARAM(0));
        SendMessageW(list, LB_RESETCONTENT, WPARAM(0), LPARAM(0));
        for event in &self.rows {
            let row = super::wide(&format!(
                "{}   |   {}   |   {}",
                range(event),
                station(event),
                event["name"]
                    .as_str()
                    .filter(|v| !v.is_empty())
                    .map(str::to_owned).unwrap_or_else(||crate::i18n::text("Program title not provided"))
            ));
            SendMessageW(list, LB_ADDSTRING, WPARAM(0), LPARAM(row.as_ptr() as isize));
        }
        let index = selected
            .and_then(|key_| self.rows.iter().position(|v| key(v) == key_))
            .or_else(|| self.rows.iter().position(|v| v["running"] == true))
            .unwrap_or(0);
        SendMessageW(list, LB_SETCURSEL, WPARAM(index), LPARAM(0));
        SendMessageW(list, WM_SETREDRAW, WPARAM(1), LPARAM(0));
        let _ = InvalidateRect(list, None, false);
        super::set_text(
            super::item(self.window, INFO),
            if self.rows.is_empty() {
                "No guide received yet. Keep watching a channel to receive its programs."
            } else {
                "Times as broadcast • Guide updates while watching"
            },
        );
        self.details();
    }
    unsafe fn details(&self) {
        let index = SendMessageW(
            super::item(self.window, LIST),
            LB_GETCURSEL,
            WPARAM(0),
            LPARAM(0),
        )
        .0;
        let text = usize::try_from(index)
            .ok()
            .and_then(|i| self.rows.get(i))
            .map(|v| {
                format!(
                    "{}\r\n{}\r\n{}\r\n\r\n{}",
                    v["name"].as_str().unwrap_or("Program"),
                    station(v),
                    range(v),
                    v["description"]
                        .as_str()
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned).unwrap_or_else(||crate::i18n::text("No description supplied by the broadcaster."))
                        .replace('\n', "\r\n")
                )
            })
            .unwrap_or_default();
        super::set_data_text(super::item(self.window, DETAIL), &text);
    }
}
unsafe fn shape(hwnd:HWND) {
    let r=super::client(hwnd);let d=GetDpiForWindow(hwnd) as i32;
    let region=CreateRoundRectRgn(0,0,r.right+1,r.bottom+1,32*d/96,32*d/96);
    if SetWindowRgn(hwnd,region,true)==0 {let _=DeleteObject(region);}
}
unsafe fn paint(hwnd:HWND,dc:HDC) {
    let r=super::client(hwnd);let dpi=GetDpiForWindow(hwnd);let p=|v:i32|v*dpi as i32/96;
    super::fill(dc,r,super::orbit::rgb(38,28,19));
    super::APP.with(|cell|{if let Ok(a)=cell.try_borrow(){if let Some(a)=a.as_ref(){a.skin.settings_background(dc,r,dpi);}}});
    super::label(dc,RECT{left:p(38),top:p(8),right:r.right-p(80),bottom:p(38)},"Program guide",p(16),super::orbit::rgb(217,194,159),false);
    super::orbit::rounded_well(dc,RECT{left:p(18),top:p(54),right:r.right-p(18),bottom:r.bottom-p(20)},super::orbit::rgb(28,20,14),p(12));
    for id in [LIST,DETAIL] {
        let child=super::item(hwnd,id);let mut rect=RECT::default();let _=GetWindowRect(child,&mut rect);
        let mut origin=POINT{x:rect.left,y:rect.top};let _=ScreenToClient(hwnd,&mut origin);
        super::orbit::rounded_well(dc,RECT{left:origin.x-p(3),top:origin.y-p(3),right:origin.x+rect.right-rect.left+p(3),bottom:origin.y+rect.bottom-rect.top+p(3)},super::orbit::rgb(35,25,18),p(8));
    }
}
unsafe fn draw_row(draw:&DRAWITEMSTRUCT) {
    let is_list=draw.CtlID==LIST as u32;
    let get_len=if is_list {LB_GETTEXTLEN}else{CB_GETLBTEXTLEN};
    let get_text=if is_list {LB_GETTEXT}else{CB_GETLBTEXT};
    let mut text=Vec::new();
    if draw.itemID!=u32::MAX {
        let len=SendMessageW(draw.hwndItem,get_len,WPARAM(draw.itemID as usize),LPARAM(0)).0;
        if (0..100_000).contains(&len) {text.resize(len as usize+1,0u16);SendMessageW(draw.hwndItem,get_text,WPARAM(draw.itemID as usize),LPARAM(text.as_mut_ptr() as isize));text.pop();}
    }
    let r=draw.rcItem;let p=|v:i32|v*GetDpiForWindow(draw.hwndItem) as i32/96;
    let selected=draw.itemState.0 & ODS_SELECTED.0!=0;
    super::fill(draw.hDC,r,super::orbit::rgb(if selected {76}else{35},if selected {52}else{25},if selected {33}else{18}));
    if selected {super::fill(draw.hDC,RECT{right:r.left+p(3),..r},super::orbit::rgb(190,153,91));}
    super::label_raw(draw.hDC,RECT{left:r.left+p(10),right:r.right-p(8),..r},&String::from_utf16_lossy(&text),p(14),super::orbit::rgb(233,215,184),false);
}
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    super::orbit::dropdown_backdrop(hwnd,msg,wp,lp);
    if msg == WM_ERASEBKGND {
        return LRESULT(1);
    }
    if msg==WM_NCCALCSIZE {return LRESULT(0);}
    if msg==WM_NCHITTEST {
        let mut point=POINT{x:lp.0 as u16 as i16 as i32,y:(lp.0>>16) as u16 as i16 as i32};let _=ScreenToClient(hwnd,&mut point);
        let r=super::client(hwnd);let p=|v:i32|v*GetDpiForWindow(hwnd) as i32/96;
        let close=point.x>=r.right-p(64) && point.x<r.right-p(34) && point.y>=p(9) && point.y<p(39);
        let hit=match (point.x<p(6),point.x>=r.right-p(6),point.y<p(6),point.y>=r.bottom-p(6)) {
            (true,_,true,_)=>HTTOPLEFT,(_,true,true,_)=>HTTOPRIGHT,(true,_,_,true)=>HTBOTTOMLEFT,(_,true,_,true)=>HTBOTTOMRIGHT,
            (true,_,_,_)=>HTLEFT,(_,true,_,_)=>HTRIGHT,(_,_,true,_)=>HTTOP,(_,_,_,true)=>HTBOTTOM,
            _ if point.y<p(48) && !close=>HTCAPTION,_=>HTCLIENT,
        };return LRESULT(hit as isize);
    }
    if matches!(msg,WM_PAINT|WM_PRINT|WM_PRINTCLIENT) {
        let mut ps=PAINTSTRUCT::default();let dc=if msg==WM_PAINT {BeginPaint(hwnd,&mut ps)}else{HDC(wp.0 as _)};
        paint(hwnd,dc);
        if msg==WM_PAINT {let _=EndPaint(hwnd,&ps);}return LRESULT(0);
    }
    if msg==WM_CTLCOLORLISTBOX{crate::scrollbars::attach(HWND(lp.0 as _));}
    if matches!(msg,WM_CTLCOLORSTATIC|WM_CTLCOLOREDIT|WM_CTLCOLORLISTBOX) {
        let dc=HDC(wp.0 as _);let color=if GetDlgCtrlID(HWND(lp.0 as _))==INFO as i32 {super::orbit::rgb(28,20,14)}else{super::orbit::rgb(35,25,18)};
        SetTextColor(dc,COLORREF(super::orbit::rgb(221,205,179)));SetBkColor(dc,COLORREF(color));SetDCBrushColor(dc,COLORREF(color));return LRESULT(GetStockObject(DC_BRUSH).0 as isize);
    }
    if msg==WM_MEASUREITEM {let m=&mut *(lp.0 as *mut MEASUREITEMSTRUCT);m.itemHeight=29*GetDpiForWindow(hwnd)/96;return LRESULT(1);}
    if msg==WM_DRAWITEM {
        let draw=&*(lp.0 as *const DRAWITEMSTRUCT);
        if draw.CtlID==super::CLOSE as u32 {super::APP.with(|cell|{if let Ok(a)=cell.try_borrow(){if let Some(a)=a.as_ref(){a.skin.button(a,draw.hDC,draw);}}});}
        else {draw_row(draw);}return LRESULT(1);
    }
    if msg==WM_COMMAND && (wp.0&65535)==super::CLOSE as usize {let _=ShowWindow(hwnd,SW_HIDE);return LRESULT(0);}
    // Closing during an update must hide the guide, never destroy a cached HWND.
    if msg == WM_CLOSE {
        let _ = ShowWindow(hwnd, SW_HIDE);
        return LRESULT(0);
    }
    let handled = GUIDE.with(|cell| {
        let Ok(mut s) = cell.try_borrow_mut() else {
            if msg == WM_SIZE {
                let _ = PostMessageW(hwnd, LAYOUT, WPARAM(0), LPARAM(0));
                return true;
            }
            return msg == WM_COMMAND;
        };
        match msg {
            WM_SIZE | LAYOUT => {
                if s.window == hwnd {
                    s.layout();
                }
                true
            }
            WM_CLOSE => {
                let _ = ShowWindow(hwnd, SW_HIDE);
                true
            }
            WM_COMMAND => {
                let id = (wp.0 & 65535) as u16;
                let code = (wp.0 >> 16) as u32;
                if id == FILTER && code == CBN_SELCHANGE {
                    s.filter = super::selected(super::item(hwnd, FILTER));
                    s.refresh();
                } else if id == LIST && code == LBN_SELCHANGE {
                    s.details();
                }
                true
            }
            WM_GETMINMAXINFO => {
                let m = &mut *(lp.0 as *mut MINMAXINFO);
                let d = GetDpiForWindow(hwnd) as i32;
                m.ptMinTrackSize = POINT {
                    x: 680 * d / 96,
                    y: 420 * d / 96,
                };
                true
            }
            WM_DPICHANGED => {
                let r = &*(lp.0 as *const RECT);
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    r.left,
                    r.top,
                    r.right - r.left,
                    r.bottom - r.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
                s.layout();
                true
            }
            _ => false,
        }
    });
    if handled {
        LRESULT(0)
    } else {
        DefWindowProcW(hwnd, msg, wp, lp)
    }
}
pub unsafe fn close() {
    GUIDE.with(|cell| {
        let Ok(mut s) = cell.try_borrow_mut() else {
            return;
        };
        if !s.window.is_invalid() {
            let _ = DestroyWindow(s.window);
            s.window = HWND::default();
            let _ = DeleteObject(s.font);
            s.font = HFONT::default();
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guide_survives_reentrant_close_filter_resize_and_recreation() {
        unsafe {
            GUIDE.with(|cell| {
                cell.borrow_mut().events = vec![
                    serde_json::json!({"frequency_khz":521143,"program_id":1,"event_id":1,"channel_number":4,"channel_name":"First","name":"First program","start":0,"duration":3600}),
                    serde_json::json!({"frequency_khz":557143,"program_id":2,"event_id":2,"channel_number":7,"channel_name":"Second","name":"Second program","start":0,"duration":3600}),
                ];
            });
            for _ in 0..3 {
                show(HWND::default()).unwrap();
                let hwnd = GUIDE.with(|cell| cell.borrow().window);
                assert!(IsWindow(hwnd).as_bool());
                SendMessageW(hwnd, WM_SIZE, WPARAM(0), LPARAM(0));
                super::super::select(super::super::item(hwnd, FILTER), 2);
                SendMessageW(
                    hwnd,
                    WM_COMMAND,
                    WPARAM(FILTER as usize | ((CBN_SELCHANGE as usize) << 16)),
                    LPARAM(0),
                );
                GUIDE.with(|cell| {
                    let s = cell.borrow_mut();
                    assert_eq!(s.rows.len(), 1);
                    assert_eq!(s.rows[0]["channel_name"], "Second");
                    // Windows may synchronously close a window during another UI update.
                    SendMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
                });
                assert!(IsWindow(hwnd).as_bool());
                assert!(!IsWindowVisible(hwnd).as_bool());
                show(HWND::default()).unwrap();
                assert!(IsWindowVisible(hwnd).as_bool());
                close();
                assert!(!IsWindow(hwnd).as_bool());
            }
        }
    }
    #[test]
    fn guide_uses_broadcast_clock_without_local_timezone_shift() {
        assert_eq!(super::date(0), "01/01 00:00");
        assert_eq!(super::date(86400 + 12 * 3600 + 34 * 60), "02/01 12:34");
    }
}

pub unsafe fn language_changed(){GUIDE.with(|cell|{if let Ok(mut s)=cell.try_borrow_mut(){if IsWindow(s.window).as_bool(){let filter=super::item(s.window,FILTER);SendMessageW(filter,CB_RESETCONTENT,WPARAM(0),LPARAM(0));s.refresh();let _=InvalidateRect(s.window,None,false);}}});}
