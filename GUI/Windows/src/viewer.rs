//! Native viewing console. Geometry and artwork follow the approved physical-TV face.
use crate::*;
use image::{
    imageops::{self, FilterType},
    RgbaImage,
};
use std::collections::HashMap;

pub const OPTIONS: u16 = 127;
pub const MAXIMIZE: u16 = 128;
pub const OPEN_OPTIONS: u32 = WM_APP + 24;
pub const CONTROLS: &[u16] = &[
    BACK, FORWARD, PLAY, STOP, RECORD, PREV, NEXT, VOL_DOWN, VOL_UP, SNAP, EPG, FULL, OPTIONS,
    LIVE, AUDIO, DECK, MIN, MAXIMIZE, CLOSE,
];

pub struct Layout {
    pub surface: RECT,
    pub buttons: Vec<(u16, RECT)>,
    pub seek: RECT,
    pub time: RECT,
    pub scale: f32,
    pub height: i32,
}
impl Layout {
    pub fn new(width: i32, height: i32) -> Self {
        let s = width.max(1) as f32 / 1600.;
        let p = |v: f32| (v * s).round() as i32;
        let bottom = |x: f32, y: f32, w: f32, h: f32| RECT {
            left: p(x),
            top: height - p(1100. - y),
            right: p(x + w),
            bottom: height - p(1100. - y - h),
        };
        let mut buttons = Vec::new();
        for (id, x, w) in [
            (BACK, 48., 62.),
            (FORWARD, 114., 62.),
            (PLAY, 180., 76.),
            (STOP, 260., 62.),
            (RECORD, 397., 48.),
            (PREV, 520., 80.),
            (NEXT, 600., 80.),
            (VOL_DOWN, 772., 80.),
            (VOL_UP, 852., 80.),
            (SNAP, 1030., 52.),
            (EPG, 1090., 76.),
            (FULL, 1174., 52.),
            (AUDIO, 1234., 84.),
            (DECK, 1326., 52.),
            (OPTIONS, 1386., 166.),
        ] {
            let mut r = bottom(x, 1011., w, 48.);
            // One rounded pixel size for the circular lamp at every scale.
            if id == RECORD {
                r.right = r.left + p(48.);
                r.bottom = r.top + p(48.);
            }
            buttons.push((id, r));
        }
        buttons.push((LIVE, bottom(1430., 948., 122., 27.)));
        for (id, x) in [(MIN, 1468.), (MAXIMIZE, 1506.), (CLOSE, 1544.)] {
            buttons.push((
                id,
                RECT {
                    left: p(x),
                    top: p(12.),
                    right: p(x + 34.),
                    bottom: p(44.),
                },
            ));
        }
        Self {
            surface: RECT {
                left: p(28.),
                top: p(62.),
                right: width - p(28.),
                bottom: height - p(169.5),
            },
            seek: bottom(40., 948., 1244., 27.),
            time: bottom(1292., 948., 116., 27.),
            buttons,
            scale: s,
            height,
        }
    }
    pub fn rect(&self, id: u16) -> RECT {
        self.buttons.iter().find(|(i, _)| *i == id).unwrap().1
    }
    fn bottom(&self, x: f32, y: f32, w: f32, h: f32) -> RECT {
        let p = |v: f32| (v * self.scale).round() as i32;
        RECT {
            left: p(x),
            top: self.height - p(1100. - y),
            right: p(x + w),
            bottom: self.height - p(1100. - y - h),
        }
    }
}

pub fn aspect_size(
    proposed: (i32, i32),
    vertical: bool,
    dpi: f32,
    aspect: (u32, u32),
) -> (i32, i32) {
    let ratio = aspect.0.max(1) as f64 / aspect.1.max(1) as f64;
    let factor = (231.5 + 1544. / ratio) / 1600.;
    let minw = (760. * dpi).ceil() as i32;
    let minh = (460. * dpi).ceil() as i32;
    let mut w = (if vertical {
        (proposed.1 as f64 / factor).round() as i32
    } else {
        proposed.0
    })
    .max(minw)
    .max((minh as f64 / factor).ceil() as i32);
    let height_for = |w: i32| {
        let l = Layout::new(w, 0);
        l.surface.top - l.surface.bottom
            + ((l.surface.right - l.surface.left) as f64 / ratio).round() as i32
    };
    while height_for(w) < minh {
        w += 1;
    }
    (w, height_for(w))
}

struct Background {
    dc: HDC,
    bitmap: HBITMAP,
    old: HGDIOBJ,
    size: (i32, i32),
}
impl Drop for Background {
    fn drop(&mut self) { unsafe {
        SelectObject(self.dc, self.old);
        let _ = DeleteObject(self.bitmap);
        let _ = DeleteDC(self.dc);
    }}
}

pub struct Skin {
    art: HashMap<&'static str, RgbaImage>,
    background: Vec<u8>,
    scaled: RefCell<Option<Background>>,
}
impl Skin {
    pub fn new() -> Self {
        let mut art = HashMap::new();
        macro_rules! asset {
            ($n:literal) => {
                art.insert(
                    $n,
                    image::load_from_memory(include_bytes!(concat!(
                        "../assets/viewer/",
                        $n,
                        ".png"
                    )))
                    .expect("embedded viewer artwork")
                    .to_rgba8(),
                );
            };
        }
        asset!("chassis");
        asset!("rewind");
        asset!("fast-forward");
        asset!("pause");
        asset!("play");
        asset!("stop");
        asset!("snapshot");
        asset!("guide");
        asset!("fullscreen");
        asset!("settings");
        asset!("mounted-indicator-off");
        asset!("mounted-indicator-on");
        // Convert the fixed chassis artwork once; resize its three bands in GDI.
        let mut background = Vec::with_capacity(1600 * 1100 * 4);
        for p in art["chassis"].pixels() {
            let alpha = p[3] as u32;
            for channel in [2, 1, 0] {
                background.push(((p[channel] as u32 * alpha + 12 * (255-alpha)) / 255) as u8);
            }
            background.push(255);
        }
        Self { art, background, scaled: RefCell::new(None) }
    }
    unsafe fn background(&self, dc: HDC, width: i32, height: i32) {
        if width <= 0 || height <= 0 { return; }
        let mut cache = self.scaled.borrow_mut();
        if cache.as_ref().is_none_or(|b| b.size != (width, height)) {
            let memory = CreateCompatibleDC(dc);
            let mut info = BITMAPINFO::default();
            info.bmiHeader = BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width, biHeight: -height, biPlanes: 1, biBitCount: 32,
                biCompression: BI_RGB.0, ..Default::default()
            };
            let mut bits = std::ptr::null_mut();
            if let Ok(bitmap) = CreateDIBSection(memory, &info, DIB_RGB_COLORS, &mut bits, None, 0) {
                let old = SelectObject(memory, bitmap);
                let saved = SaveDC(memory);
                let v = Layout::new(width, height).surface;
                ExcludeClipRect(memory, v.left, v.top, v.right, v.bottom);
                self.draw_background(memory, width, height);
                let _ = RestoreDC(memory, saved);
                *cache = Some(Background { dc: memory, bitmap, old, size: (width,height) });
            } else {
                let _ = DeleteDC(memory);
                self.draw_background(dc, width, height);
                return;
            }
        }
        // All buttons share this scaled frame instead of resampling the bands again.
        let _ = BitBlt(dc, 0, 0, width, height, cache.as_ref().unwrap().dc, 0, 0, SRCCOPY);
    }
    unsafe fn draw_background(&self, dc: HDC, width: i32, height: i32) {
        if width <= 0 || height <= 0 { return; }
        let mut info = BITMAPINFO::default();
        info.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: 1600, biHeight: -1100, biPlanes: 1, biBitCount: 32,
            biCompression: BI_RGB.0, ..Default::default()
        };
        let scale = width as f64 / 1600.;
        let top = (100. * scale).round() as i32;
        let bottom = (200. * scale).round() as i32;
        let old = SetStretchBltMode(dc, HALFTONE);
        let mut origin = POINT::default();
        let _ = SetBrushOrgEx(dc, 0, 0, Some(&mut origin));
        for (source_y, source_height, y, h) in [
            (0, 100, 0, top),
            (100, 800, top, (height-top-bottom).max(0)),
            (900, 200, (height-bottom).max(0), bottom),
        ] {
            if h > 0 {
                StretchDIBits(dc, 0, y, width, h, 0, 1100-source_y-source_height, 1600, source_height,
                    Some(self.background.as_ptr().cast()), &info, DIB_RGB_COLORS, SRCCOPY);
            }
        }
        let _ = SetBrushOrgEx(dc, origin.x, origin.y, None);
        SetStretchBltMode(dc, STRETCH_BLT_MODE(old));
    }
    pub unsafe fn child_background(&self, dc: HDC, parent: HWND, child: HWND) {
        let pr = client(parent);
        let cr = client(child);
        let mut origin = POINT::default();
        let _ = ClientToScreen(child, &mut origin);
        let _ = ScreenToClient(parent, &mut origin);
        let saved = SaveDC(dc);
        IntersectClipRect(dc, 0, 0, cr.right, cr.bottom);
        let _ = SetViewportOrgEx(dc, -origin.x, -origin.y, None);
        self.background(dc, pr.right, pr.bottom);
        let _ = RestoreDC(dc, saved);
    }
    unsafe fn artwork(&self, dc: HDC, name: &str, r: RECT, tint: Option<u32>) {
        if r.right <= r.left || r.bottom <= r.top {
            return;
        }
        let art = imageops::resize(
            &self.art[name],
            (r.right - r.left) as u32,
            (r.bottom - r.top) as u32,
            FilterType::Triangle,
        );
        orbit::blend_image(dc, r, &art, tint);
    }
    pub unsafe fn paint(&self, a: &App, dc: HDC, r: RECT) {
        self.background(dc, r.right, r.bottom);
        let l = Layout::new(r.right, r.bottom);
        let p = |v: f32| (v * l.scale).round() as i32;
        legend(
            dc,
            RECT {
                left: p(35.),
                top: p(8.),
                right: p(500.),
                bottom: p(49.),
            },
            "Live TV!",
            p(26.),
            orbit::CREAM,
            false,
        );
        let rec = l.rect(RECORD);
        legend(
            dc,
            RECT {
                left: rec.left - p(12.),
                top: rec.top - p(24.),
                right: rec.right + p(12.),
                bottom: rec.top - p(3.),
            },
            "REC",
            p(11.),
            orbit::CREAM,
            true,
        );
        for (id, text) in [(PREV, "CHANNEL"), (VOL_DOWN, "VOLUME")] {
            let rr = l.rect(id);
            legend(
                dc,
                RECT {
                    top: rr.top - p(24.),
                    bottom: rr.top - p(3.),
                    right: rr.right + p(60.),
                    ..rr
                },
                text,
                p(11.),
                orbit::CREAM,
                false,
            );
        }
        // Retain actionable backend status without competing with playback controls.
        if !a.status.is_empty() && !a.status.starts_with("Ready") {
            legend(
                dc,
                l.bottom(48., 1070., 1404., 19.),
                &a.status,
                p(11.),
                orbit::CREAM,
                false,
            );
        }
    }
    pub unsafe fn time(&self, a: &App, dc: HDC, hwnd: HWND, r: RECT) {
        self.child_background(dc, a.video, hwnd);
        // A shallow rounded recess over the same continuous chassis texture.
        let face=RECT{left:1,top:1,right:r.right-1,bottom:r.bottom-2};
        let curve=(r.bottom/4).max(6);
        outline(dc,RECT{top:face.top+1,bottom:face.bottom+1,..face},orbit::rgb(82,64,47),1,curve);
        outline(dc,face,orbit::rgb(13,10,8),1,curve);
        legend(
            dc,
            r,
            &text_of(hwnd),
            (r.bottom * 45 / 100).max(10),
            orbit::CREAM,
            true,
        );
    }
    pub unsafe fn button(&self, a: &App, dc: HDC, draw: &DRAWITEMSTRUCT) {
        if matches!(draw.CtlID as u16,LIVE|AUDIO) {a.skin.button(a,dc,draw);return;}
        let r = draw.rcItem;
        let id = draw.CtlID as u16;
        if a.fullscreen {
            fill(dc, r, orbit::rgb(12, 10, 8));
        } else {
            self.child_background(dc, a.video, draw.hwndItem);
        }
        let hover = orbit::hovered(draw.hwndItem) && IsWindowEnabled(draw.hwndItem).as_bool();
        let down = draw.itemState.0 & ODS_SELECTED.0 != 0;
        let h = r.bottom - r.top;
        let w = r.right - r.left;
        let s = h as f32 / 48.;
        let p = |v: f32| (v * s).round().max(1.) as i32;
        let centered = |size: i32| RECT {
            left: (w - size) / 2,
            top: (h - size) / 2 + if down { 1 } else { 0 },
            right: (w - size) / 2 + size,
            bottom: (h - size) / 2 + size + if down { 1 } else { 0 },
        };
        if id == RECORD {
            let active = matches!(a.current, Some(Job::Record));
            self.artwork(
                dc,
                if active {
                    "mounted-indicator-on"
                } else {
                    "mounted-indicator-off"
                },
                centered(h.min(w)),
                None,
            );
        } else if matches!(id, MIN | MAXIMIZE | CLOSE) {
            let color = if hover {
                orbit::rgb(255, 243, 222)
            } else {
                orbit::CREAM
            };
            let size = (h * 35 / 100).max(6);
            let ir = centered(size);
            let t = (h / 22).max(1);
            if id == MIN {
                line(
                    dc,
                    ir.left,
                    (ir.top + ir.bottom) / 2,
                    ir.right,
                    (ir.top + ir.bottom) / 2,
                    color,
                    t,
                );
            } else if id == CLOSE {
                line(dc, ir.left, ir.top, ir.right, ir.bottom, color, t);
                line(dc, ir.right, ir.top, ir.left, ir.bottom, color, t);
            } else if IsZoomed(a.video).as_bool() {
                let offset = (size / 4).max(2);
                // The overlapping-window symbol distinguishes Restore from Maximize.
                line(dc, ir.left+offset, ir.top, ir.right, ir.top, color, t);
                line(dc, ir.right, ir.top, ir.right, ir.bottom-offset, color, t);
                outline(dc, RECT { left: ir.left, top: ir.top+offset,
                    right: ir.right-offset, bottom: ir.bottom }, color, t, 0);
            } else {
                outline(dc, ir, color, t, 0);
            }
        } else {
            let utility = matches!(id, SNAP | EPG | FULL | OPTIONS);
            let right_half = matches!(id, NEXT | VOL_UP);
            let rocker = matches!(id, PREV | NEXT | VOL_DOWN | VOL_UP);
            let face = if rocker {
                RECT {
                    left: if right_half { -w } else { 0 },
                    right: if right_half { w } else { w * 2 },
                    ..r
                }
            } else {
                r
            };
            let paint = a.skin.button_face(a, dc, draw, face);
            if right_half {
                line(
                    dc,
                    0,
                    2 + paint.offset,
                    0,
                    h - 3 + paint.offset,
                    orbit::rgb(100, 86, 65),
                    1,
                );
            }
            let ink = paint.ink;
            let centered = |size: i32| RECT {
                left: (w - size) / 2,
                top: (h - size) / 2 + paint.offset,
                right: (w - size) / 2 + size,
                bottom: (h - size) / 2 + size + paint.offset,
            };
            if id==DECK {
                let q=centered(p(26.));let t=p(1.4).max(1);
                let body=RECT{top:q.top+p(4.),bottom:q.bottom-p(4.),..q};
                outline(dc,body,ink,t,p(3.));
                outline(dc,RECT{left:q.left+p(4.),right:q.right-p(11.),top:body.top+p(4.),bottom:body.bottom-p(4.)},ink,t,0);
                outline(dc,RECT{left:q.right-p(8.),right:q.right-p(3.),top:body.top+p(6.),bottom:body.top+p(11.)},ink,t,p(5.));
            }
            let name = match id {
                BACK => "rewind",
                FORWARD => "fast-forward",
                PLAY => {
                    if text_of(draw.hwndItem).contains('Ⅱ') {
                        "pause"
                    } else {
                        "play"
                    }
                }
                STOP => "stop",
                SNAP => "snapshot",
                EPG => "guide",
                FULL => "fullscreen",
                OPTIONS => "settings",
                _ => "",
            };
            if !name.is_empty() {
                let has_text = matches!(id, EPG | OPTIONS);
                let size = p(if has_text { 17. } else { 22. });
                let ir = if has_text {
                    RECT {
                        left: p(14.),
                        top: (h - size) / 2,
                        right: p(14.) + size,
                        bottom: (h - size) / 2 + size,
                    }
                } else {
                    centered(size)
                };
                self.artwork(dc, name, ir, Some(ink));
            }
            let text = match id {
                PREV | VOL_DOWN => "−",
                NEXT | VOL_UP => "+",
                EPG => "EPG",
                OPTIONS => "SETTINGS",
                _ => "",
            };
            if !text.is_empty() {
                legend(
                    dc,
                    RECT {
                        left: if matches!(id, EPG | OPTIONS) {
                            p(32.)
                        } else {
                            0
                        },
                        ..r
                    },
                    text,
                    p(if utility { 13. } else { 20. }),
                    ink,
                    true,
                );
            }
        }
        if draw.itemState.0 & ODS_FOCUS.0 != 0 && draw.itemState.0 & ODS_NOFOCUSRECT.0 == 0 {
            let mut focus = RECT {
                left: 3,
                top: 3,
                right: w - 3,
                bottom: h - 3,
            };
            let _ = orbit::focus_outline(dc, &mut focus);
        }
    }
}

unsafe fn legend(dc:HDC,r:RECT,text:&str,size:i32,color:u32,center:bool){legend_raw(dc,r,&i18n::text(text),size,color,center);}
unsafe fn legend_raw(dc: HDC, r: RECT, text: &str, size: i32, color: u32, center: bool) {
    let font = CreateFontW(
        -crate::wine_ui::pixels(size.max(8)),
        0,
        0,
        0,
        600,
        0,
        0,
        0,
        DEFAULT_CHARSET.0 as u32,
        OUT_DEFAULT_PRECIS.0 as u32,
        CLIP_DEFAULT_PRECIS.0 as u32,
        CLEARTYPE_QUALITY.0 as u32,
        0,
        w!("Segoe UI"),
    );
    let old = SelectObject(dc, font);
    SetTextColor(dc, COLORREF(color));
    SetBkMode(dc, TRANSPARENT);
    let mut text = wide(text);
    let len = text.len().saturating_sub(1);
    let mut r = r;
    let _ = DrawTextW(
        dc,
        &mut text[..len],
        &mut r,
        DT_SINGLELINE
            | DT_VCENTER
            | DT_END_ELLIPSIS
            | DT_NOPREFIX
            | if center { DT_CENTER } else { DT_LEFT },
    );
    SelectObject(dc, old);
    let _ = DeleteObject(font);
}
unsafe fn outline(dc: HDC, r: RECT, color: u32, width: i32, radius: i32) {
    let pen = CreatePen(PS_SOLID, width, COLORREF(color));
    let old = SelectObject(dc, pen);
    let brush = SelectObject(dc, GetStockObject(NULL_BRUSH));
    let _ = RoundRect(dc, r.left, r.top, r.right, r.bottom, radius, radius);
    SelectObject(dc, brush);
    SelectObject(dc, old);
    let _ = DeleteObject(pen);
}

/// Let the native window manager preserve the last normal size and position.
pub unsafe fn toggle_maximize(hwnd: HWND) {
    let command = if IsZoomed(hwnd).as_bool() { SC_RESTORE } else { SC_MAXIMIZE };
    let _ = PostMessageW(hwnd, WM_SYSCOMMAND, WPARAM(command as usize), LPARAM(0));
}

/// Commit one layout without synchronous MoveWindow repaints for every child.
pub unsafe fn position_children(positions: &[(HWND, RECT)]) {
    let flags = SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOCOPYBITS;
    let mut batch = BeginDeferWindowPos(positions.len() as i32);
    for &(hwnd, r) in positions {
        batch = batch.and_then(|h| DeferWindowPos(h, hwnd, None, r.left, r.top,
            (r.right-r.left).max(1), (r.bottom-r.top).max(1), flags));
        if batch.is_err() { break; }
    }
    if batch.and_then(|h| EndDeferWindowPos(h)).is_err() {
        // Low-resource failure must not leave controls at stale coordinates.
        for &(hwnd, r) in positions {
            let _ = SetWindowPos(hwnd, None, r.left, r.top,
                (r.right-r.left).max(1), (r.bottom-r.top).max(1), flags);
        }
    }
}

pub unsafe fn hit_test(hwnd: HWND, lp: LPARAM) -> LRESULT {
    let fs = APP.with(|c| {
        c.try_borrow()
            .ok()
            .is_some_and(|a| a.as_ref().is_some_and(|a| a.fullscreen))
    });
    if fs {
        return LRESULT(HTCLIENT as isize);
    }
    let mut p = POINT {
        x: lp.0 as u16 as i16 as i32,
        y: (lp.0 >> 16) as u16 as i16 as i32,
    };
    let _ = ScreenToClient(hwnd, &mut p);
    let r = client(hwnd);
    let edge = if IsZoomed(hwnd).as_bool() {
        0
    } else {
        (6 * GetDpiForWindow(hwnd) / 96).max(4) as i32
    };
    let hit = match (
        p.x < edge,
        p.x >= r.right - edge,
        p.y < edge,
        p.y >= r.bottom - edge,
    ) {
        (true, _, true, _) => HTTOPLEFT,
        (_, true, true, _) => HTTOPRIGHT,
        (true, _, _, true) => HTBOTTOMLEFT,
        (_, true, _, true) => HTBOTTOMRIGHT,
        (true, _, _, _) => HTLEFT,
        (_, true, _, _) => HTRIGHT,
        (_, _, true, _) => HTTOP,
        (_, _, _, true) => HTBOTTOM,
        _ if p.y < (r.right as f32 * 51. / 1600.) as i32
            && p.x < (r.right as f32 * 1468. / 1600.) as i32 =>
        {
            HTCAPTION
        }
        _ => HTCLIENT,
    };
    LRESULT(hit as isize)
}

pub unsafe fn show_options(hwnd: HWND) {
    let Ok(menu) = CreatePopupMenu() else {
        return;
    };
    let background = CreateSolidBrush(COLORREF(orbit::rgb(30, 22, 16)));
    let info = MENUINFO {
        cbSize: std::mem::size_of::<MENUINFO>() as u32,
        fMask: MIM_BACKGROUND,
        hbrBack: background,
        ..Default::default()
    };
    let _ = SetMenuInfo(menu, &info);
    let checked = APP.with(|c| {
        c.try_borrow()
            .ok()
            .is_some_and(|a| a.as_ref().is_some_and(|a| a.captions_enabled))
    });
    for (id, text) in [
        (AUDIO, "Audio tracks and mode"),
        (CC, "Closed captions"),
        (OPEN, "Open media…"),
        (DECK, "Receiver panel"),
        (SETTINGS, "Preferences…"),
    ] {
        let text = wide(&i18n::text(text));
        let _ = AppendMenuW(
            menu,
            MF_STRING
                | if id == CC && checked {
                    MF_CHECKED
                } else {
                    MENU_ITEM_FLAGS(0)
                },
            id as usize,
            PCWSTR(text.as_ptr()),
        );
        // Preserve the accessible native item string while supplying DAC colors.
        let item = MENUITEMINFOW {
            cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
            fMask: MIIM_FTYPE,
            fType: MFT_OWNERDRAW,
            ..Default::default()
        };
        let _ = SetMenuItemInfoW(menu, id as u32, false, &item);
    }
    let mut r = RECT::default();
    let _ = GetWindowRect(item(hwnd, OPTIONS), &mut r);
    let selected = TrackPopupMenu(
        menu,
        TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTALIGN | TPM_BOTTOMALIGN,
        r.right,
        r.top - 4,
        0,
        hwnd,
        None,
    )
    .0 as u16;
    let _ = DestroyMenu(menu);
    let _ = DeleteObject(background);
    if selected != 0 {
        let _ = PostMessageW(hwnd, WM_COMMAND, WPARAM(selected as usize), LPARAM(0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn layout_keeps_transport_order_rockers_equal_and_lamp_concentric() {
        for w in [760, 950, 1120, 1600, 2000, 3200] {
            let l = Layout::new(w, (w as f64 * 1100. / 1600.) as i32);
            let ids: Vec<_> = l.buttons.iter().take(4).map(|(id, _)| *id).collect();
            assert_eq!(ids, vec![BACK, FORWARD, PLAY, STOP]);
            for pair in l.buttons[..4].windows(2) {
                assert!(pair[0].1.right < pair[1].1.left);
            }
            let a = l.rect(PREV);
            let b = l.rect(NEXT);
            let c = l.rect(VOL_DOWN);
            let d = l.rect(VOL_UP);
            assert!(((b.right - a.left) - (d.right - c.left)).abs() <= 1);
            assert_eq!(a.top, c.top);
            assert_eq!(a.bottom, c.bottom);
            let rec = l.rect(RECORD);
            assert_eq!(rec.right - rec.left, rec.bottom - rec.top);
            let middle = (l.rect(STOP).right + a.left) / 2;
            assert!(((rec.left + rec.right) / 2 - middle).abs() <= 1);
            assert!(l.surface.bottom < l.seek.top);
            assert!(l.seek.bottom < rec.top);
        }
    }
}

pub unsafe fn draw_menu(owner:HWND,dc: HDC, draw: &DRAWITEMSTRUCT) {
    let r = draw.rcItem;
    let dpi = GetDpiForWindow(owner) as i32;
    let d = if dpi == 0 { 1. } else { dpi as f32 / 96. };
    let px = |v: f32| (v * d).round() as i32;
    let selected = draw.itemState.0 & ODS_SELECTED.0 != 0;
    fill(
        dc,
        r,
        if selected {
            orbit::rgb(68, 50, 34)
        } else {
            orbit::rgb(30, 22, 16)
        },
    );
    let mut buffer=[0u16;512];
    let menu=HMENU(draw.hwndItem.0);
    let length=GetMenuStringW(menu,draw.itemID,Some(&mut buffer),MF_BYCOMMAND).max(0) as usize;
    let text=String::from_utf16_lossy(&buffer[..length]);
    legend(
        dc,
        RECT {
            left: r.left + px(34.),
            right: r.right - px(18.),
            ..r
        },
        &text,
        px(16.),
        orbit::CREAM,
        false,
    );
    if draw.itemState.0 & ODS_CHECKED.0 != 0 {
        legend(
            dc,
            RECT {
                right: r.left + px(32.),
                ..r
            },
            "✓",
            px(16.),
            orbit::CREAM,
            true,
        );
    }
}

/// The app owns every pixel of its border, including after native activation.
pub unsafe fn configure_frame(hwnd:HWND) {
    use windows::Win32::Graphics::Dwm::*;
    let policy=DWMNCRP_DISABLED;let border=0xfffffffeu32;
    let _=DwmSetWindowAttribute(hwnd,DWMWA_NCRENDERING_POLICY,(&policy as *const DWMNCRENDERINGPOLICY).cast(),4);
    let _=DwmSetWindowAttribute(hwnd,DWMWA_BORDER_COLOR,(&border as *const u32).cast(),4);
}
pub unsafe fn invalidate_chrome(hwnd:HWND) {
    let _=InvalidateRect(hwnd,None,false);
    // Preserve the live video surface; only the custom frame/controls need paint.
    for &id in CONTROLS.iter().chain([SEEK,SEEK_TIME].iter()) {
        let child=item(hwnd,id);
        if !child.0.is_null() && IsWindowVisible(child).as_bool() {let _=InvalidateRect(child,None,false);}
    }
}

#[cfg(test)]
mod window_tests {
    use super::*;

    #[test]
    fn chassis_drawing_preserves_band_orientation_and_pixels() {
        unsafe {
            let skin = Skin::new();
            assert_eq!(skin.art["chassis"].dimensions(), (1600, 1100));
            let dc = CreateCompatibleDC(None);
            let mut info = BITMAPINFO::default();
            info.bmiHeader = BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: 1600, biHeight: -1100, biPlanes: 1, biBitCount: 32,
                biCompression: BI_RGB.0, ..Default::default() };
            let mut bits = std::ptr::null_mut();
            let bitmap = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();
            let old = SelectObject(dc, bitmap);
            skin.draw_background(dc, 1600, 1100);
            let _ = GdiFlush();
            let pixels = std::slice::from_raw_parts(bits as *const u8, 1600*1100*4);
            for y in [0, 25, 99, 100, 350, 899, 900, 1000, 1099] {
                for x in [0, 10, 200, 800, 1400, 1599] {
                    let at = (y*1600+x)*4;
                    assert_eq!(&pixels[at..at+3], &skin.background[at..at+3], "pixel {x},{y}");
                }
            }
            // Real GDI work over varying dimensions, with the video area clipped out.
            let start = std::time::Instant::now();
            for i in 0..120 {
                let (w,h) = (800+i*6, 560+i*4);
                let saved = SaveDC(dc);
                let v = Layout::new(w,h).surface;
                ExcludeClipRect(dc,v.left,v.top,v.right,v.bottom);
                skin.background(dc,w,h);
                let _ = RestoreDC(dc,saved);
            }
            let _ = GdiFlush();
            eprintln!("120 chassis resizes: {:.2} ms", start.elapsed().as_secs_f64()*1000.);
            SelectObject(dc,old);let _=DeleteObject(bitmap);let _=DeleteDC(dc);
        }
    }

    #[test]
    #[ignore = "Creates a temporary native window to exercise maximize/restore"]
    fn maximize_toggle_restores_previous_size_and_position() {
        unsafe {
            let instance = GetModuleHandleW(None).unwrap();
            let class = w!("VolarResizeRegression");
            let wc = WNDCLASSW { lpfnWndProc: Some(crate::window_proc), hInstance: instance.into(),
                lpszClassName: class, ..Default::default() };
            RegisterClassW(&wc);
            let hwnd = CreateWindowExW(WS_EX_NOACTIVATE|WS_EX_TOOLWINDOW, class, w!("Window regression"),
                WS_POPUP|WS_THICKFRAME|WS_SYSMENU|WS_MINIMIZEBOX|WS_MAXIMIZEBOX,
                120,100,960,660,None,None,instance,None).unwrap();
            let _=SetPropW(hwnd,w!("OrbitBorderless"),HANDLE(3usize as _));
            let pump = || {
                let mut message = MSG::default();
                while PeekMessageW(&mut message,hwnd,WM_SYSCOMMAND,WM_SYSCOMMAND,PM_REMOVE).as_bool() {
                    DispatchMessageW(&message);
                }
            };
            let rect = || {let mut r=RECT::default();GetWindowRect(hwnd,&mut r).unwrap();[r.left,r.top,r.right,r.bottom]};
            for (x,y,w,h) in [(120,100,960,660),(165,130,1050,730)] {
                SetWindowPos(hwnd,None,x,y,w,h,SWP_NOZORDER|SWP_NOACTIVATE).unwrap();
                let original=rect();
                toggle_maximize(hwnd);pump();assert!(IsZoomed(hwnd).as_bool());
                toggle_maximize(hwnd);pump();assert!(!IsZoomed(hwnd).as_bool());assert_eq!(rect(),original);
                toggle_maximize(hwnd);pump();
                let _=ShowWindow(hwnd,SW_MINIMIZE);assert!(IsIconic(hwnd).as_bool());
                let _=ShowWindow(hwnd,SW_RESTORE);assert!(IsZoomed(hwnd).as_bool());
                toggle_maximize(hwnd);pump();assert_eq!(rect(),original);
            }
            DestroyWindow(hwnd).unwrap();let _=UnregisterClassW(class,instance);
        }
    }
}
