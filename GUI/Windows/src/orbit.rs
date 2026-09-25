//! Orbit's native owner-drawn skin. Controls retain Win32 input/accessibility.
use crate::*;
pub(super) struct ButtonPaint {pub ink:u32,pub icon_ink:u32,pub offset:i32,pub h:i32}
pub const CREAM:u32=233 | (223<<8) | (206<<16);
use std::{collections::HashMap, time::Instant};

// Match the measured typography in the concept; avoid GDI's proportional-font
// substitution between measurement and painting.
unsafe fn label(dc:HDC,r:RECT,s:&str,size:i32,color:u32,center:bool){label_raw(dc,r,&i18n::text(s),size,color,center);}
unsafe fn label_raw(dc: HDC, r: RECT, s: &str, size: i32, color: u32, center: bool) {
    if s.is_empty() || r.right <= r.left || r.bottom <= r.top {
        return;
    }
    let f = font(dc, size);
    SetBkMode(dc, TRANSPARENT);
    SetTextColor(dc, COLORREF(color));
    let mut text: Vec<u16> = s.encode_utf16().collect();
    let mut r = r;
    DrawTextW(
        dc,
        &mut text,
        &mut r,
        DT_SINGLELINE
            | DT_VCENTER
            | DT_END_ELLIPSIS
            | DT_NOPREFIX
            | if center { DT_CENTER } else { DT_LEFT },
    );
    f.restore(dc);
}

// A mouse-transparent child layer dims only controls below an open combo.
// The native popup is a separate window and remains at its original brightness.
pub unsafe fn dropdown_backdrop(parent:HWND,msg:u32,wp:WPARAM,lp:LPARAM) {
    if msg!=WM_COMMAND || lp.0==0 {return;}
    let notification=((wp.0>>16)&0xffff) as u32;
    if notification!=CBN_DROPDOWN && notification!=CBN_CLOSEUP {return;}
    let key=w!("OrbitDropdownBackdrop");
    let mut overlay=HWND(GetPropW(parent,key).0);
    if notification==CBN_CLOSEUP {
        crate::scrollbars::close_combo(HWND(lp.0 as _));
        if !overlay.0.is_null() {let _=ShowWindow(overlay,SW_HIDE);}
        return;
    }
    let combo=HWND(lp.0 as _);
    crate::scrollbars::attach_combo(combo);
    let mut bounds=RECT::default();let _=GetWindowRect(combo,&mut bounds);
    let mut bottom=POINT{x:bounds.left,y:bounds.bottom};let _=ScreenToClient(parent,&mut bottom);
    if overlay.0.is_null() {
        let Ok(window)=CreateWindowExW(WS_EX_LAYERED|WS_EX_TRANSPARENT|WS_EX_NOACTIVATE,
            w!("STATIC"),w!(""),WS_CHILD|WINDOW_STYLE(4) /* SS_BLACKRECT */,
            0,0,1,1,parent,None,None,None) else {return;};
        overlay=window;
        let _=SetPropW(parent,key,HANDLE(overlay.0));
        let _=SetLayeredWindowAttributes(overlay,COLORREF(0),48,LWA_ALPHA);
    }
    let region=CreateRectRgn(0,0,0,0);
    let padding=(3*GetDpiForWindow(parent) as i32/96).max(2);
    let mut child=GetWindow(parent,GW_CHILD).unwrap_or_default();
    while !child.0.is_null() {
        if child!=overlay && child!=combo && IsWindowVisible(child).as_bool() {
            let mut r=RECT::default();let _=GetWindowRect(child,&mut r);
            let mut origin=POINT{x:r.left,y:r.top};let _=ScreenToClient(parent,&mut origin);
            if origin.y>=bottom.y {
                let area=CreateRectRgn(origin.x-padding,origin.y-padding,
                    origin.x+r.right-r.left+padding,origin.y+r.bottom-r.top+padding);
                let _=CombineRgn(region,region,area,RGN_OR);let _=DeleteObject(area);
            }
        }
        child=GetWindow(child,GW_HWNDNEXT).unwrap_or_default();
    }
    if SetWindowRgn(overlay,region,false)==0 {let _=DeleteObject(region);}
    let r=client(parent);
    let _=SetWindowPos(overlay,HWND_TOP,0,0,r.right,r.bottom,SWP_NOACTIVATE|SWP_SHOWWINDOW);
}

pub const TAB: u16 = 320;
pub const THEME: u16 = 321;
pub const MATERIAL: u16 = 322;
pub const WIDTH: f32 = 1679.;
pub const HEIGHT: f32 = 547.;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Material {
    #[default]
    Metal,
    Glass,
    Plastic,
}
impl Material {
    pub fn parse(s: &str) -> Self {
        match s {
            "glass" => Self::Glass,
            "plastic" => Self::Plastic,
            _ => Self::Metal,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Metal => "metal",
            Self::Glass => "glass",
            Self::Plastic => "plastic",
        }
    }
    pub fn index(self) -> usize {
        match self {
            Self::Metal => 0,
            Self::Glass => 1,
            Self::Plastic => 2,
        }
    }
}

pub fn clock_time(s: f64) -> String {
    let n = if s.is_finite() { s.max(0.) as u64 } else { 0 };
    format!("{:02}:{:02}:{:02}", n / 3600, n / 60 % 60, n % 60)
}
pub fn timeline_time(
    recording: bool,
    file: bool,
    seekable: bool,
    paused: bool,
    position: f64,
    duration: f64,
) -> String {
    if !seekable && !recording {
        return "—".into();
    }
    clock_time(
        if recording && !file && !paused && duration - position <= 3. {
            duration
        } else {
            position
        },
    )
}
pub fn settings_body(r:RECT,dpi:u32)->RECT {
    let u=|n:i32|n*dpi as i32/96;
    RECT{left:r.left+u(14),top:r.top+u(98),right:r.right-u(14),bottom:r.bottom-u(14)}
}
pub fn rect(r: RECT, x: f32, y: f32, w: f32, h: f32) -> RECT {
    let sx = (r.right - r.left) as f32 / WIDTH;
    let sy = (r.bottom - r.top) as f32 / HEIGHT;
    RECT {
        left: r.left + (x * sx).round() as i32,
        top: r.top + (y * sy).round() as i32,
        right: r.left + ((x + w) * sx).round() as i32,
        bottom: r.top + ((y + h) * sy).round() as i32,
    }
}
// Plate spans the display's top and the lower button row's bottom.
pub fn module_rect(r:RECT)->RECT {rect(r,1229.2727,85.,392.7273,360.)}
pub fn dial_rect(r:RECT)->RECT {
    let scale=360./379.5;
    rect(r,1229.2727+(1321.16-1208.)*scale,85.+(169.66-73.)*scale,187.68*scale,187.68*scale)
}
pub unsafe fn translated_buttons(hwnd:HWND,r:RECT)->Vec<(u16,f32,f32,f32)>{
    let dc=GetDC(hwnd);let sy=r.bottom as f64/547.;
    let f=font(dc,(46.*sy*0.31).round().max(11.) as i32);
    let mut result=BUTTONS.to_vec();
    let measure=|label:&str|{let text=wide(&i18n::text(label).to_uppercase());let mut size=SIZE::default();
        let _=GetTextExtentPoint32W(dc,&text[..text.len()-1],&mut size);size.cx as f64/sy};
    for (start,labels) in [(0,&["CH -","CH +","","","PLAY","STOP","REC","FULLSCREEN"][..]),
        (8,&["GUIDE","SNAPSHOT","AUDIO","CC OFF","LIBRARY","OPEN","SETTINGS"][..])]{
        let row=&mut result[start..start+labels.len()];
        let required:Vec<f64>=labels.iter().map(|label|{
            let width=match *label{"CC OFF"=>measure(label).max(measure("CC ON")),"PLAY"=>measure(label).max(measure("PAUSE")),"REC"=>measure(label).max(measure("REC ON")),_=>measure(label)};
            (width+46.*0.36+46./7.+if *label=="SETTINGS"{36.}else{24.}).max(if label.starts_with("CH "){80.}else{52.})
        }).collect();
        let original:Vec<f64>=row.iter().map(|b|b.3 as f64).collect();
        if required.iter().zip(&original).any(|(a,b)|a>b){
            let widths=button_style::row_widths(&required,&original,1071.-(labels.len()-1) as f64*8.);let mut x=61.;
            for (button,width) in row.iter_mut().zip(widths){button.1=x as f32;button.3=width as f32;x+=width+8.;}
        }
    }
    f.restore(dc);ReleaseDC(hwnd,dc);result
}
pub const BUTTONS: &[(u16, f32, f32, f32)] = &[
    (PREV, 61., 332., 80.),
    (NEXT, 149., 332., 80.),
    (BACK, 253., 332., 68.),
    (FORWARD, 329., 332., 68.),
    (PLAY, 413., 332., 120.),
    (STOP, 541., 332., 126.),
    (RECORD, 691., 332., 180.),
    (FULL, 883., 332., 249.),
    (EPG, 61., 429., 142.),
    (SNAP, 211., 429., 142.),
    (AUDIO, 381., 429., 139.),
    (CC, 528., 429., 139.),
    (EXPORTS, 695., 429., 139.),
    (OPEN, 842., 429., 139.),
    (SETTINGS, 1009., 429., 123.),
];

struct Texture {
    w: i32,
    h: i32,
    bgra: Vec<u8>,
}
impl Texture {
    fn new(bytes: &[u8]) -> Self {
        let im = image::load_from_memory(bytes)
            .expect("embedded Orbit artwork")
            .to_rgba8();
        Self {
            w: im.width() as i32,
            h: im.height() as i32,
            bgra: im.pixels().flat_map(|p| [p[2], p[1], p[0], p[3]]).collect(),
        }
    }
    unsafe fn draw(&self,dc:HDC,r:RECT) {
        self.draw_part(dc,r,RECT{left:0,top:0,right:self.w,bottom:self.h});
    }
    unsafe fn draw_part(&self,dc:HDC,r:RECT,source:RECT) {
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: self.w,
                biHeight: -self.h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let old = SetStretchBltMode(dc, HALFTONE);
        let _ = SetBrushOrgEx(dc, 0, 0, None);
        StretchDIBits(
            dc,
            r.left,
            r.top,
            r.right - r.left,
            r.bottom - r.top,
            source.left,
            self.h-source.bottom,
            source.right-source.left,
            source.bottom-source.top,
            Some(self.bgra.as_ptr().cast()),
            &info,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
        SetStretchBltMode(dc, STRETCH_BLT_MODE(old));
    }
}
pub struct Skin {
    fascia: Texture,
    settings_fascia: Texture,
    module: Texture,
    buttons: [Texture; 3],
    button_cache: RefCell<HashMap<(usize, i32, i32, u32), Texture>>,
    frame_cache: RefCell<Option<Texture>>,
    title_cache: RefCell<Option<(i32,i32,i32,image::RgbaImage)>>,
    icons: HashMap<&'static str, image::RgbaImage>,
    pub field_brush: HBRUSH,
}
impl Drop for Skin {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.field_brush);
        }
    }
}
impl Skin {
    pub fn new() -> Self {
        let mut icons = HashMap::new();
        macro_rules! art {($($n:literal),*)=>{$(icons.insert($n,image::load_from_memory(include_bytes!(concat!("../assets/orbit/",$n,".png"))).unwrap().to_rgba8());)*};}
        art!(
            "fullscreen",
            "record",
            "play",
            "pause",
            "stop",
            "seek-back",
            "seek-forward",
            "minimize",
            "caption-screw",
            "channel-chevron",
            "channel-frame",
            "channel-face",
            "close",
            "live",
            "guide",
            "snapshot",
            "audio",
            "mono",
            "stereo",
            "surround",
            "signal",
            "captions",
            "library",
            "open",
            "settings"
        );
        Self {
            fascia: Texture::new(include_bytes!("../assets/orbit/fascia.png")),
            settings_fascia: Texture::new(include_bytes!("../assets/orbit/settings-fascia.png")),
            module: Texture::new(include_bytes!("../assets/orbit/module.png")),
            button_cache: RefCell::new(HashMap::new()),
            frame_cache: RefCell::new(None),
            title_cache: RefCell::new(None),
            buttons: [
                Texture::new(include_bytes!("../assets/orbit/metal.png")),
                Texture::new(include_bytes!("../assets/orbit/glass.png")),
                Texture::new(include_bytes!("../assets/orbit/plastic.png")),
            ],
            icons,
            field_brush: unsafe { CreateSolidBrush(COLORREF(rgb(43, 31, 22))) },
        }
    }
    pub unsafe fn background(&self, dc: HDC, r: RECT) {
        let (w,h)=((r.right-r.left).max(1),(r.bottom-r.top).max(1));
        let mut cache=self.frame_cache.borrow_mut();
        if cache.as_ref().is_none_or(|t|t.w!=w || t.h!=h) {
            // Reconstruct the bevel at the output resolution instead of relying
            // on GDI's coarse stretched-bitmap sampling at diagonal edges.
            let source=image::RgbaImage::from_raw(self.fascia.w as u32,self.fascia.h as u32,self.fascia.bgra.clone()).unwrap();
            let im=image::imageops::resize(&source,w as u32,h as u32,image::imageops::FilterType::Lanczos3);
            *cache=Some(Texture{w,h,bgra:im.into_raw()});
        }
        cache.as_ref().unwrap().draw(dc,r)
    }
    pub unsafe fn window_shape(&self, hwnd: HWND, r: RECT) {
        let (w,h)=(r.right-r.left,r.bottom-r.top);
        if w<=0 || h<=0 {return;}
        let key=((w as usize)<<32)|h as usize;
        if GetPropW(hwnd,w!("OrbitShapeSize")).0 as usize==key {return;}
        // Match the artwork silhouette so desktop shows outside the curved rim.
        let diameter=if GetPropW(hwnd,w!("OrbitBorderless")).0 as usize==2 {32*GetDpiForWindow(hwnd) as i32/96} else {(58. * w as f32/self.fascia.w as f32).round().max(2.) as i32};
        let region=CreateRoundRectRgn(0,0,w+1,h+1,diameter,diameter);
        if SetWindowRgn(hwnd,region,true)==0 {let _=DeleteObject(region);}
        else {let _=SetPropW(hwnd,w!("OrbitShapeSize"),HANDLE(key as _));}
    }
    pub unsafe fn settings_background(&self,dc:HDC,r:RECT,dpi:u32) {
        let t=&self.settings_fascia;let w=r.right-r.left;let h=r.bottom-r.top;
        let d=(32*dpi as i32/96).min(w/2).min(h/2).max(1);
        let sx=[0,64,t.w-64,t.w];let sy=[0,64,t.h-64,t.h];
        let dx=[r.left,r.left+d,r.right-d,r.right];let dy=[r.top,r.top+d,r.bottom-d,r.bottom];
        for y in 0..3 {for x in 0..3 {
            t.draw_part(dc,RECT{left:dx[x],top:dy[y],right:dx[x+1],bottom:dy[y+1]},
                RECT{left:sx[x],top:sy[y],right:sx[x+1],bottom:sy[y+1]});
        }}
    }
    pub unsafe fn settings_widget(&self,dc:HDC,r:RECT,tabs:HWND,current:usize,dpi:u32) {
        let u=|n:i32|n*dpi as i32/96;
        let body=settings_body(r,dpi);
        let face=rgb(28,20,14);
        let tab_rect=|i:usize,selected:bool| {
            let mut tab=RECT::default();
            SendMessageW(tabs,TCM_GETITEMRECT,WPARAM(i),LPARAM((&mut tab as *mut RECT) as isize));
            let left=body.left+tab.left+u(1);
            let right=(body.left+tab.right-u(1)).min(body.right-u(2));
            RECT{left:if selected && i==0 {body.left+3}else if selected {left.max(body.left+u(9))}else{left},
                right:if selected && i==6 {body.right-3}else if selected {right.min(body.right-u(9))}else{right},
                top:r.top+u(if matches!(i,2|5){if selected{44}else{51}}else if selected {48}else{55}),bottom:body.top+u(4)}
        };
        let draw_tab=|i:usize,selected:bool| {
            let tab=tab_rect(i,selected);
            let saved=SaveDC(dc);
            IntersectClipRect(dc,tab.left,tab.top,tab.right,tab.bottom);
            let color=if selected {face}else{rgb(38,28,21)};
            let brush=CreateSolidBrush(COLORREF(color));
            let pen=CreatePen(PS_SOLID,1,COLORREF(if selected {rgb(147,112,71)}else{rgb(91,68,46)}));
            let old_b=SelectObject(dc,brush);let old_p=SelectObject(dc,pen);
            let _=RoundRect(dc,tab.left,tab.top,tab.right,tab.bottom+u(8),u(12),u(12));
            SelectObject(dc,old_b);SelectObject(dc,old_p);let _=DeleteObject(brush);let _=DeleteObject(pen);
            label(dc,RECT{bottom:body.top,..tab},i18n::tab(i),
                u(if i18n::index()==0 {if selected {15}else{14}}else{12}),if selected {rgb(241,212,165)}else{rgb(170,151,126)},true);
            let _=RestoreDC(dc,saved);
        };
        // Inactive tabs sit behind the page rim. The selected tab opens through
        // that rim and shares the page's fill, forming one continuous widget.
        for i in 0..7 {if i!=current {draw_tab(i,false);}}
        well(dc,body,face);
        let active=tab_rect(current,true);
        let join=body.top+3; // Meet the inner page rim, not the outside shadow.
        let radius=u(7).max(4);
        let left=if current==0 {active.left}else{(active.left-radius).max(body.left+3)};
        let right=(active.right+radius).min(body.right-3);
        let lr=(active.left-left).max(1);let rr=(right-active.right).max(1);
        let k=|n:i32|(n as f32*0.5523).round() as i32;
        // Rounded shoulders flare into the page rim, leaving no dangling tab edges.
        let contour=|| {
            let _=MoveToEx(dc,left,join,None);
            if current!=0 {let _=PolyBezierTo(dc,&[POINT{x:left+k(lr),y:join},POINT{x:active.left,y:join-lr+k(lr)},POINT{x:active.left,y:join-lr}]);}
            let _=LineTo(dc,active.left,active.top+radius);
            let _=PolyBezierTo(dc,&[POINT{x:active.left,y:active.top+radius-k(radius)},POINT{x:active.left+radius-k(radius),y:active.top},POINT{x:active.left+radius,y:active.top}]);
            let _=LineTo(dc,active.right-radius,active.top);
            let _=PolyBezierTo(dc,&[POINT{x:active.right-radius+k(radius),y:active.top},POINT{x:active.right,y:active.top+radius-k(radius)},POINT{x:active.right,y:active.top+radius}]);
            if current==6 {let _=LineTo(dc,active.right,join);}else{
                let _=LineTo(dc,active.right,join-rr);
                let _=PolyBezierTo(dc,&[POINT{x:active.right,y:join-rr+k(rr)},POINT{x:right-k(rr),y:join},POINT{x:right,y:join}]);
            }
        };
        let brush=CreateSolidBrush(COLORREF(face));let old_brush=SelectObject(dc,brush);
        let pen=CreatePen(PS_SOLID,1,COLORREF(rgb(147,112,71)));let old_pen=SelectObject(dc,pen);
        let _=BeginPath(dc);contour();let _=LineTo(dc,right,join+u(3));let _=LineTo(dc,left,join+u(3));let _=CloseFigure(dc);let _=EndPath(dc);let _=FillPath(dc);
        let _=BeginPath(dc);contour();let _=EndPath(dc);let _=StrokePath(dc);
        SelectObject(dc,old_brush);SelectObject(dc,old_pen);let _=DeleteObject(brush);let _=DeleteObject(pen);
        label(dc,RECT{bottom:body.top,..active},i18n::tab(current),u(if i18n::index()==0{15}else{12}),rgb(241,212,165),true);
    }
    pub unsafe fn child_background(&self, dc: HDC, parent: HWND, child: HWND) {
        let mut p = POINT::default();
        let _ = ClientToScreen(child, &mut p);
        let _ = ScreenToClient(parent, &mut p);
        let r = client(parent);
        let shifted = RECT {
            left: -p.x,
            top: -p.y,
            right: r.right - p.x,
            bottom: r.bottom - p.y,
        };
        let id=GetDlgCtrlID(child) as u16;
        if GetPropW(parent,w!("OrbitBorderless")).0 as usize==2 {
            let dpi=GetDpiForWindow(parent);
            self.settings_background(dc,shifted,dpi);
            // Child status labels and edit controls must reveal their tab page,
            // not repaint the chassis texture over it (notably Channels status).
            let body=settings_body(shifted,dpi);
            let saved=SaveDC(dc);let child_rect=client(child);
            IntersectClipRect(dc,child_rect.left,child_rect.top,child_rect.right,child_rect.bottom);
            fill(dc,RECT{left:body.left+3,top:body.top+3,right:body.right-3,bottom:body.bottom-3},rgb(28,20,14));
            let _=RestoreDC(dc,saved);
            return;
        }
        if !matches!(id,DIAL|MIN|CLOSE) {
            fill(dc, client(child), rgb(28, 20, 14));
            return;
        }
        self.background(dc, shifted);
        if id!=DIAL {return;}
        // The dial's transparent corners must reveal its mounting plate, not fascia.
        let m = module_rect(r);
        self.module.draw(
            dc,
            RECT {
                left: m.left - p.x,
                top: m.top - p.y,
                right: m.right - p.x,
                bottom: m.bottom - p.y,
            },
        );
    }
    pub unsafe fn icon(&self, dc: HDC, name: &str, r: RECT, color: u32) {
        self.artwork(dc,name,r,Some(color));
    }
    unsafe fn artwork(&self, dc:HDC, name:&str, r:RECT, tint:Option<u32>) {
        let Some(im) = self.icons.get(name) else {
            return;
        };
        let w = (r.right - r.left).max(1);
        let h = (r.bottom - r.top).max(1);
        let icon = image::imageops::resize(
            im,
            w as u32,
            h as u32,
            image::imageops::FilterType::Lanczos3,
        );
        blend_image(dc,r,&icon,tint);
    }
    unsafe fn etched_title(&self,dc:HDC,r:RECT,size:i32) {
        let (w,h)=(r.right-r.left,r.bottom-r.top);
        let mut cache=self.title_cache.borrow_mut();
        if cache.as_ref().is_none_or(|(cw,ch,cs,_)|(*cw,*ch,*cs)!=(w,h,size)) {
            if let Some(im)=engraved_title_mask(dc,w,h,size) {*cache=Some((w,h,size,im));}
        }
        if let Some((_,_,_,im))=cache.as_ref() {blend_image(dc,r,im,None);}
    }
    pub unsafe fn panel(&self, a: &App, dc: HDC, r: RECT) {
        self.background(dc, r);
        let rr = |x, y, w, h| rect(r, x, y, w, h);
        let sy = r.bottom as f32 / HEIGHT;
        let text =
            |dc, r, s: &str, size: f32, c| label(dc, r, s, (size * sy).round() as i32, c, false);
        self.module.draw(dc, module_rect(r));
        let title = rr(66., 25., 200., 40.);
        crate::label(dc,title,"Live TV!",(29. * sy) as i32,CREAM,false);
        rounded_well(dc, rr(61., 85., 1072., 185.), rgb(20, 14, 10), (22. * sy).round().max(12.) as i32);
        let state = a.control.snapshot();
        let t = &state["timeline"];
        let now = GetLocalTime();
        let clock = rr(838., 105., 180., 48.);
        rounded_well(dc, clock, rgb(38, 28, 19), (12. * sy).round().max(8.) as i32);
        crate::label(
            dc, clock,
            &format!("{:02}:{:02}:{:02}", now.wHour, now.wMinute, now.wSecond),
            (28. * sy) as i32, CREAM, true,
        );
        let recording = matches!(a.current, Some(Job::Record));
        let file = matches!(a.current, Some(Job::File(_)));
        if recording {
            self.status_icon(dc, rr(782., 161., 36., 36.), "record", rgb(199, 101, 80));
            // A single measured baseline preserves exactly one space around '+'.
            let region = rr(830., 161., 300., 36.);
            let font = font(dc, (14. * sy) as i32);
            let first = i18n::text("RECORDING ");
            let wide_first = wide(&first);
            let mut size = SIZE::default();
            let _ = GetTextExtentPoint32W(dc, &wide_first[..wide_first.len() - 1], &mut size);
            text(dc, region, &first, 14., rgb(225, 113, 93));
            text(
                dc,
                RECT {
                    left: region.left + size.cx,
                    ..region
                },
                "+ TIME SHIFT",
                14.,
                rgb(235, 222, 119),
            );
            font.restore(dc);
        } else if file || matches!(a.current, Some(Job::Scan(_))) {
            text(
                dc,
                rr(782., 161., 325., 36.),
                if file {
                    "RECORDING PLAYBACK"
                } else {
                    "SCANNING CHANNELS"
                },
                14.,
                rgb(188, 169, 148),
            );
        }
        if !file {
            let delay = (t["duration"].as_f64().unwrap_or(0.)
                - t["position"].as_f64().unwrap_or(0.))
            .max(0.);
            let behind = if recording && (a.paused || delay > 3.) {
                format!("{} s BEHIND LIVE", delay.round() as u64)
            } else if a.rx.is_some() {
                "LIVE".into()
            } else {
                "READY".into()
            };
            // Align the live indicator below the left edge of the clock box.
            let live_y=if recording || matches!(a.current,Some(Job::Scan(_))) {210.}else{171.};
            let row = RECT{left:clock.left,..rr(838., live_y, 271., 36.)};
            let status_font = font(dc, (14. * sy).round() as i32);
            let status_text = wide(&i18n::text(&behind));
            let mut status_size = SIZE::default();
            let _ = GetTextExtentPoint32W(dc, &status_text[..status_text.len()-1], &mut status_size);
            status_font.restore(dc);
            let gap = (12. * sy).round() as i32;
            let icon_size = row.bottom - row.top;
            let group_left = row.left;
            let text_left = group_left + icon_size + gap;
            self.status_icon(dc, RECT{left:group_left,right:group_left+icon_size,..row}, "live", if behind == "LIVE" { rgb(103,194,120) } else { rgb(94,121,89) });
            let status_rect=RECT{left:text_left,right:(text_left+status_size.cx).min(row.right),..row};
            text(dc, status_rect, &behind, 14., if behind == "LIVE" { rgb(136,201,145) } else { rgb(234,158,70) });
        }
        let (mode, icon) = match a.audio_mode {
            audio::Mode::Mono => ("MONO", "mono"),
            audio::Mode::Left => ("LEFT CHANNEL", "mono"),
            audio::Mode::Right => ("RIGHT CHANNEL", "mono"),
            audio::Mode::Surround => ("SURROUND", "surround"),
            audio::Mode::Stereo => ("STEREO", "stereo"),
        };
        self.status_icon(dc, rr(84., 228., 32., 32.), icon, rgb(201, 181, 153));
        text(
            dc,
            rr(128., 228., 600., 32.),
            mode,
            14.,
            rgb(186, 165, 144),
        );
        let sep = rr(61., 374., 1072., 1.);
        fill(dc, sep, rgb(68, 48, 34));
    }
    unsafe fn status_icon(&self, dc: HDC, r: RECT, name: &str, c: u32) {
        well(dc, r, rgb(38, 28, 19));
        let inset = (r.right - r.left) / 5;
        self.icon(
            dc,
            name,
            RECT {
                left: r.left + inset,
                top: r.top + inset,
                right: r.right - inset,
                bottom: r.bottom - inset,
            },
            c,
        );
    }
    pub unsafe fn channel(&self, a: &App, dc: HDC, r: RECT) {
        let curve=((r.bottom-r.top) as f32*18./108.).round().max(12.) as i32;
        // The selector sits on the display glass; preserve that material outside its arc.
        fill(dc,r,rgb(20,14,10));
        rounded_well(dc, r, rgb(38, 27, 19), curve);
        // A broad, shallow glass reflection, not a bright opaque stripe.
        let saved = SaveDC(dc);
        let clip = CreateRoundRectRgn(r.left + 4, r.top + 4, r.right - 4, r.bottom - 4, (curve-8).max(4), (curve-8).max(4));
        ExtSelectClipRgn(dc, clip, RGN_AND);
        let top = RECT {
            left: r.left + 4,
            top: r.top + 4,
            right: r.right - 4,
            bottom: r.top + (r.bottom - r.top) / 3,
        };
        gradient(dc, top, [62, 47, 35], [38, 27, 19]);
        let _ = RestoreDC(dc, saved);
        let _ = DeleteObject(clip);
        let w = (r.right - r.left) as f32;
        let h = (r.bottom - r.top) as f32;
        let rr = |x: f32, y: f32, ww: f32, hh: f32| RECT {
            left: r.left + (x * w) as i32,
            top: r.top + (y * h) as i32,
            right: r.left + ((x + ww) * w) as i32,
            bottom: r.top + ((y + hh) * h) as i32,
        };
        label(
            dc,
            rr(0.027, 0.04, 0.13, 0.23),
            "CHANNEL",
            (h * 0.13) as i32,
            rgb(188, 169, 152),
            false,
        );
        crate::label(
            dc,
            rr(0.027, 0.29, 0.12, 0.60),
            &format!("{:02}", crate::channel_number(a.services.get(a.channel_index),a.channel_index)),
            (h * 0.45) as i32,
            rgb(229, 199, 110),
            false,
        );
        let sep = rr(0.15, 0.12, 0.002, 0.75);
        fill(dc, sep, rgb(97, 70, 50));
        let name = if let Some(Job::File(p)) = &a.current {
            p.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        } else {
            a.services
                .get(a.channel_index)
                .and_then(|s| s["name"].as_str())
                .unwrap_or("Select a channel")
                .to_owned()
        };
        crate::label_raw(
            dc,
            rr(0.185, 0.06, 0.69, 0.34),
            &name,
            (h * 0.28) as i32,
            CREAM,
            false,
        );
        label_raw(
            dc,
            rr(0.185, 0.43, 0.69, 0.20),
            &a.current_programme().unwrap_or_else(||if matches!(a.current,Some(Job::File(_))){i18n::text("RECORDING")}else{String::new()}),
            (h * 0.13) as i32,
            rgb(188, 169, 152),
            false,
        );
        self.icon(
            dc,
            "signal",
            rr(0.185, 0.75, 0.026, 0.15),
            rgb(183, 164, 116),
        );
        label(
            dc,
            rr(0.22, 0.70, 0.60, 0.26),
            &a.quality
                .map(|q| format!("SIGNAL QUALITY: {q}%"))
                .unwrap_or_else(|| "SIGNAL QUALITY: —".into()),
            (h * 0.125) as i32,
            rgb(189, 169, 128),
            false,
        );
        let arrow = rr(0.905, 0.26, 0.075, 0.49);
        self.icon(dc,"channel-face",arrow,rgb(58,42,29));
        self.icon(dc,"channel-frame",RECT{top:arrow.top+1,bottom:arrow.bottom+1,..arrow},rgb(116,88,60));
        self.icon(dc,"channel-frame",arrow,rgb(19,13,9));
        let glyph=RECT{left:arrow.left+(arrow.right-arrow.left)/4,
            right:arrow.right-(arrow.right-arrow.left)/4,
            top:arrow.top+(arrow.bottom-arrow.top)/4,
            bottom:arrow.bottom-(arrow.bottom-arrow.top)/4};
        self.icon(dc,"channel-chevron",RECT{top:glyph.top+1,bottom:glyph.bottom+1,..glyph},rgb(18,12,8));
        self.icon(dc,"channel-chevron",glyph,rgb(224,195,145));
    }
    /// Shared DAC material, shape, animated reflection, press depth and ink.
    /// The caller supplies its own chassis background and glyph arrangement.
    pub(super) unsafe fn button_face(&self,a:&App,dc:HDC,draw:&DRAWITEMSTRUCT,r:RECT)->ButtonPaint {
        let id=draw.CtlID as u16;
        let anim = animation(draw.hwndItem);
        let down = (draw.itemState.0 & ODS_SELECTED.0) != 0;
        let material = a.material;
        let offset = if down {
            1
        } else {
            (anim.press * 1.5).round() as i32
        };
        let face = RECT {
            left: r.left + 1,
            top: r.top + 1 + offset,
            right: r.right - 1,
            bottom: r.bottom - 2 + offset,
        };
        let saved = SaveDC(dc);
        let curve=((face.bottom-face.top) as f32*0.25).round().max(8.) as i32;
        let region = CreateRoundRectRgn(face.left, face.top, face.right + 1, face.bottom + 1, curve, curve);
        ExtSelectClipRgn(dc, region, RGN_AND);
        let mut cache = self.button_cache.borrow_mut();
        let key = (
            material.index(),
            face.right - face.left,
            face.bottom - face.top,
            GetDpiForWindow(draw.hwndItem),
        );
        if cache.len() > 96 {
            cache.clear();
        }
        let texture = cache.entry(key).or_insert_with(|| {
            let (w, h) = (key.1.max(1), key.2.max(1));
            let source = &self.buttons[key.0];
            let pixels = button_style::material_pixels(&source.bgra, source.w, source.h,
                source.w as usize * 4, key.0, w, h, key.3);
            Texture { w, h, bgra: pixels }
        });
        texture.draw(dc, face);
        // Lighting is applied separately to the stationary grain/material.
        let h = (face.bottom - face.top).max(1);
        if anim.hover > 0. || down {
            let light = if material == Material::Glass {
                [244, 218, 172]
            } else {
                [255, 236, 209]
            };
            let strength = if down {
                0.025
            } else if material == Material::Glass {
                0.19
            } else if material == Material::Metal {
                0.38
            } else {
                0.09
            };
            // Metal reflects a dark environment band beside the moving softbox.
            // Both are planar reflections over stationary grain, not a domed bevel.
            if material == Material::Metal {
                reflection(dc, face, [54, 35, 25], 0.16 * anim.hover,
                    anim.sweep * 1.5 - 0.02);
            }
            reflection(
                dc,
                face,
                light,
                strength * anim.hover,
                anim.sweep * 1.5 - 0.25,
            );
        }
        let _ = RestoreDC(dc, saved);
        let _ = DeleteObject(region);
        let tint = if id == CC {
            {let (r,g,b)=button_style::CAPTION_EDGE;rgb(r,g,b)}
        } else if id == LIVE {
            rgb(90, 166, 101)
        } else {
            {let (r,g,b)=button_style::RECORD_EDGE;rgb(r,g,b)}
        };
        let enabled = IsWindowEnabled(draw.hwndItem).as_bool();
        let accented = enabled && matches!(id,RECORD|CC|LIVE) && anim.hovered;
        let focused=draw.itemState.0 & ODS_FOCUS.0!=0 && draw.itemState.0 & ODS_NOFOCUSRECT.0==0;
        let edge = if accented {
            tint
        } else if focused {
            rgb(201,171,119)
        } else if material == Material::Glass {
            rgb(139, 113, 83)
        } else {
            rgb(150, 137, 119)
        };
        outline_radius(dc, face, edge, curve);
        if accented && matches!(id,RECORD|CC) {
            // Draw inward so a 3-DIP hover edge never changes the button bounds.
            let width=((3*GetDpiForWindow(draw.hwndItem)+48)/96).max(2) as i32;
            for inset in 1..width {
                outline_radius(dc,RECT{left:face.left+inset,top:face.top+inset,right:face.right-inset,bottom:face.bottom-inset},edge,(curve-2*inset).max(2));
            }
        }
        if !accented {
            line(
                dc,
                face.left + curve/2,
                face.top,
                face.right - curve/2,
                face.top,
                if material == Material::Glass {
                    rgb(175, 147, 112)
                } else {
                    rgb(225, 218, 203)
                },
                1,
            );
        }
        let accent_ink = if id == CC {
            if material == Material::Glass { rgb(197, 163, 55) } else { rgb(145, 109, 6) }
        } else if id == LIVE {
            if material == Material::Glass { rgb(103, 194, 120) } else { rgb(39, 107, 58) }
        } else if material == Material::Glass {
            rgb(217, 106, 85)
        } else {
            rgb(152, 47, 37)
        };
        let ink = if !enabled {
            if material == Material::Glass { rgb(126, 111, 94) } else { rgb(101, 95, 87) }
        } else if accented {
            accent_ink
        } else if material == Material::Glass {
            CREAM
        } else {
            rgb(40, 38, 33)
        };
        // Status lamps stay illuminated independently of hover and action availability.
        // Labels and borders acquire their accent only while hovered.
        let icon_ink = if matches!(id, RECORD | LIVE) || (id == CC && a.captions_enabled) {
            accent_ink
        } else { ink };
        ButtonPaint{ink,icon_ink,offset,h}
    }
    pub unsafe fn button(&self, a: &App, dc: HDC, draw: &DRAWITEMSTRUCT) {
        let r = draw.rcItem;
        let id = draw.CtlID as u16;
        if matches!(id,MIN|CLOSE) {
            self.child_background(dc,GetParent(draw.hwndItem).unwrap_or(a.deck),draw.hwndItem);
            let anim=animation(draw.hwndItem);
            let down=draw.itemState.0 & ODS_SELECTED.0!=0;
            let h=(r.bottom-r.top).max(1);
            let head_size=(h as f32*0.94).round() as i32;
            let head=RECT{left:(r.left+r.right-head_size)/2,top:(r.top+r.bottom-head_size)/2,
                right:(r.left+r.right+head_size)/2,bottom:(r.top+r.bottom+head_size)/2};
            self.artwork(dc,"caption-screw",head,None);
            if anim.hover>0. {
                let saved=SaveDC(dc);let clip=CreateEllipticRgn(head.left+2,head.top+2,head.right-2,head.bottom-2);
                ExtSelectClipRgn(dc,clip,RGN_AND);
                reflection(dc,head,[228,205,168],0.06*anim.hover,0.45);
                let _=RestoreDC(dc,saved);let _=DeleteObject(clip);
            }
            let size=(h as f32*0.40).round() as i32;
            let offset=if down {1}else{(anim.press*1.5).round() as i32};
            let x=(r.left+r.right-size)/2;let y=(r.top+r.bottom-size)/2+offset;
            let ir=RECT{left:x,top:y,right:x+size,bottom:y+size};
            let name=if id==MIN {"minimize"}else{"close"};
            let glint=(anim.hover*22.) as u8;
            self.icon(dc,name,RECT{top:ir.top+1,bottom:ir.bottom+1,..ir},rgb(137+glint,119+glint,94+glint));
            self.icon(dc,name,ir,if down {rgb(20,12,8)}else{rgb(36,26,19)});
            if draw.itemState.0 & ODS_FOCUS.0!=0 {let _=focus_outline(dc,&RECT{left:3,top:3,right:r.right-3,bottom:r.bottom-3});}
            return;
        }
        let mut text = i18n::raw(draw.hwndItem).unwrap_or_else(||text_of(draw.hwndItem))
            .replace(['▶', 'Ⅱ', '■', '●', '⚙'], "")
            .trim()
            .to_uppercase();
        if id == EPG {
            text = "GUIDE".into()
        }
        if matches!(id, BACK | FORWARD) {
            text.clear();
        }
        let recording = matches!(a.current, Some(Job::Record));
        if id == RECORD {
            text = if recording { "REC ON" } else { "REC" }.into()
        }
        // Rounded button corners reveal the same material behind the control.
        if GetParent(draw.hwndItem).unwrap_or_default()==a.deck {
            let mut origin=POINT::default();let _=ClientToScreen(draw.hwndItem,&mut origin);let _=ScreenToClient(a.deck,&mut origin);
            let parent=client(a.deck);
            self.background(dc,RECT{left:-origin.x,top:-origin.y,right:parent.right-origin.x,bottom:parent.bottom-origin.y});
        } else if GetParent(draw.hwndItem).unwrap_or_default()==a.video {
            a.viewer.child_background(dc,a.video,draw.hwndItem);
        } else {fill(dc,r,rgb(28,20,14));}
        if id==LIVE && GetParent(draw.hwndItem).unwrap_or_default()==a.video {
            let h=r.bottom-r.top;let size=h.max(1);let gap=(h/3).max(6);
            let icon=RECT{left:r.left,top:r.top,right:r.left+size,bottom:r.bottom};
            let hover=animation(draw.hwndItem).hovered;
            let green=rgb(90,166,101);
            self.status_icon(dc,icon,"live",green);
            if hover {let pen=CreatePen(PS_SOLID,(2*GetDpiForWindow(draw.hwndItem)/96).max(2) as i32,COLORREF(green));let old=SelectObject(dc,pen);let b=SelectObject(dc,GetStockObject(NULL_BRUSH));let _=RoundRect(dc,icon.left+1,icon.top+1,icon.right-1,icon.bottom-1,5,5);SelectObject(dc,b);SelectObject(dc,old);let _=DeleteObject(pen);}
            label(dc,RECT{left:icon.right+gap,..r},"LIVE",(h as f32*0.46).round().max(11.) as i32,if hover{green}else{CREAM},false);
            if draw.itemState.0 & ODS_FOCUS.0!=0 {let _=focus_outline(dc,&r);}
            return;
        }
        let ButtonPaint {ink,icon_ink,offset,h}=self.button_face(a,dc,draw,r);
        let material=a.material;
        let icon = match id {
            PLAY => {
                if a.rx.is_some() && !a.paused {
                    "pause"
                } else {
                    "play"
                }
            }
            STOP => "stop",
            BACK => "seek-back",
            FORWARD => "seek-forward",
            RECORD => "record",
            LIVE => "live",
            FULL => "fullscreen",
            EPG => "guide",
            SNAP => "snapshot",
            AUDIO => "audio",
            CC => "captions",
            EXPORTS => "library",
            OPEN => "open",
            SETTINGS => "settings",
            _ => "",
        };
        let reference_h=h;
        let mut font_size = (reference_h as f32 * 0.31).round().max(11.) as i32;
        let f = font(dc, font_size);
        let text=i18n::text(&text);
        let s = wide(&text);
        let mut size = SIZE::default();
        let _ = GetTextExtentPoint32W(dc, &s[..s.len() - 1], &mut size);
        let icon_size = (reference_h as f32 * 0.36).round() as i32;
        let gap = if icon.is_empty() || text.is_empty() { 0 } else { (reference_h / 7).max(4) };
        let available=(r.right-r.left-16-if icon.is_empty(){0}else{icon_size+gap}).max(1);
        f.restore(dc);
        if size.cx>available{font_size=(font_size as f64*available as f64/size.cx as f64).floor().max(1.) as i32;}
        let f=font(dc,font_size);let _=GetTextExtentPoint32W(dc,&s[..s.len()-1],&mut size);
        let total = size.cx + if icon.is_empty() { 0 } else { icon_size + gap };
        let x = (r.left + r.right - total) / 2;
        let y = (r.top + r.bottom - icon_size) / 2 + offset;
        if !icon.is_empty() {
            self.icon(
                dc,
                icon,
                RECT {
                    left: x,
                    top: y,
                    right: x + icon_size,
                    bottom: y + icon_size,
                },
                icon_ink,
            );
        }
        let tr = RECT {
            left: x + total - size.cx,
            top: r.top + offset,
            right: r.right - 3,
            bottom: r.bottom + offset,
        };
        if material != Material::Glass {
            label(
                dc,
                RECT {
                    top: tr.top + 1,
                    bottom: tr.bottom + 1,
                    ..tr
                },
                &text,
                font_size,
                rgb(228, 218, 200),
                false,
            );
        }
        label(dc, tr, &text, font_size, ink, false);
        f.restore(dc);

    }
}
// Rasterize at 4x and shade within the glyph mask, producing an inset cut rather
// than a drop shadow. Cached per output size; normal paints only alpha-blend it.
unsafe fn engraved_title_mask(dc:HDC,w:i32,h:i32,size:i32)->Option<image::RgbaImage> {
    if w<=0 || h<=0 {return None;}
    let scale=4;let (mw,mh)=(w*scale,h*scale);
    let info=BITMAPINFO{bmiHeader:BITMAPINFOHEADER{biSize:std::mem::size_of::<BITMAPINFOHEADER>() as u32,biWidth:mw,biHeight:-mh,biPlanes:1,biBitCount:32,biCompression:BI_RGB.0,..Default::default()},..Default::default()};
    let mut bits=std::ptr::null_mut();let bitmap=CreateDIBSection(dc,&info,DIB_RGB_COLORS,&mut bits,None,0).ok()?;
    let memory=CreateCompatibleDC(dc);let old=SelectObject(memory,bitmap);
    std::ptr::write_bytes(bits.cast::<u8>(),0,(mw*mh*4) as usize);
    crate::label(memory,RECT{left:0,top:0,right:mw,bottom:mh},"Live TV!",size*scale,rgb(255,255,255),false);
    let _=GdiFlush();
    let bytes=std::slice::from_raw_parts(bits.cast::<u8>(),(mw*mh*4) as usize);
    let mask:Vec<f32>=bytes.chunks_exact(4).map(|p|(p[0] as f32+p[1] as f32+p[2] as f32)/765.).collect();
    SelectObject(memory,old);let _=DeleteObject(bitmap);let _=DeleteDC(memory);
    let sample=|x:i32,y:i32|if x<0||x>=mw||y<0||y>=mh {0.}else{mask[(y*mw+x) as usize]};
    // Recessed dark lettering: a restrained lower lip catches the light.
    // A bright letter face and a surrounding outline read as raised metal.
    let depth=(size as f32/29.*0.5*scale as f32).round().max(1.) as i32;
    let mut im=image::RgbaImage::new(mw as u32,mh as u32);
    for y in 0..mh {for x in 0..mw {
        let a=sample(x,y);let mut out=[0f32;4];
        let mut over=|color:[f32;3],opacity:f32| {for c in 0..3 {out[c]=color[c]*opacity+out[c]*(1.-opacity);}out[3]=255.*opacity+out[3]*(1.-opacity);};
        let lip=(sample(x,y-depth)-a).max(0.);
        over([147.,118.,90.],lip*0.55);
        over([16.,12.,8.],a*0.94);
        im.put_pixel(x as u32,y as u32,image::Rgba(out.map(|v|v.round().clamp(0.,255.) as u8)));
    }}
    // Resize premultiplied color so transparent edges do not introduce halos.
    let mut im=image::imageops::resize(&im,w as u32,h as u32,image::imageops::FilterType::Lanczos3);
    for pixel in im.pixels_mut() {let alpha=pixel[3] as u32;if alpha>0 {for c in 0..3 {pixel[c]=(pixel[c] as u32*255/alpha).min(255) as u8;}}}
    Some(im)
}
pub(super) unsafe fn blend_image(dc:HDC,r:RECT,icon:&image::RgbaImage,tint:Option<u32>) {
    let (w,h)=(r.right-r.left,r.bottom-r.top);
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let Ok(bmp) = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, None, 0) else {
            return;
        };
        let bytes = std::slice::from_raw_parts_mut(bits.cast::<u8>(), (w * h * 4) as usize);
        for (p, d) in icon.pixels().zip(bytes.chunks_exact_mut(4)) {
            let a = p[3] as u32;
            let color=tint.unwrap_or_else(||rgb(p[0],p[1],p[2]));
            d.copy_from_slice(&[
                ((color >> 16 & 255) * a / 255) as u8,
                ((color >> 8 & 255) * a / 255) as u8,
                ((color & 255) * a / 255) as u8,
                a as u8,
            ]);
        }
        let mem = CreateCompatibleDC(dc);
        let old = SelectObject(mem, bmp);
        let _ = AlphaBlend(
            dc,
            r.left,
            r.top,
            w,
            h,
            mem,
            0,
            0,
            w,
            h,
            BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            },
        );
        SelectObject(mem, old);
        let _ = DeleteObject(bmp);
        let _ = DeleteDC(mem);
    }
pub fn rgb(r: u8, g: u8, b: u8) -> u32 {
    r as u32 | (g as u32) << 8 | (b as u32) << 16
}
unsafe fn outline(dc:HDC,r:RECT,color:u32){outline_radius(dc,r,color,5);}
unsafe fn outline_radius(dc: HDC, r: RECT, color: u32, radius:i32) {
    let pen = CreatePen(PS_SOLID, 1, COLORREF(color));
    let old = SelectObject(dc, pen);
    let brush = SelectObject(dc, GetStockObject(NULL_BRUSH));
    let _ = RoundRect(dc, r.left, r.top, r.right, r.bottom, radius, radius);
    SelectObject(dc, brush);
    SelectObject(dc, old);
    let _ = DeleteObject(pen);
}
// Rounded display wells preserve the already painted material outside the rim.
pub(super) unsafe fn rounded_well(dc:HDC,r:RECT,color:u32,curve:i32) {
    let pen=CreatePen(PS_SOLID,1,COLORREF(rgb(13,9,6)));
    let brush=CreateSolidBrush(COLORREF(rgb(12,8,5)));
    let old_pen=SelectObject(dc,pen);let old_brush=SelectObject(dc,brush);
    let _=RoundRect(dc,r.left,r.top,r.right,r.bottom,curve,curve);
    let inner=RECT{left:r.left+3,top:r.top+3,right:r.right-3,bottom:r.bottom-3};
    let inside=CreateSolidBrush(COLORREF(color));SelectObject(dc,inside);
    let inner_curve=(curve-6).max(4);
    let _=RoundRect(dc,inner.left,inner.top,inner.right,inner.bottom,inner_curve,inner_curve);
    SelectObject(dc,old_brush);SelectObject(dc,old_pen);
    let _=DeleteObject(pen);let _=DeleteObject(brush);let _=DeleteObject(inside);
    outline_radius(dc,inner,rgb(113,87,59),inner_curve);
}
pub unsafe fn well(dc: HDC, r: RECT, color: u32) {
    fill(dc, r, rgb(28, 20, 14));
    let pen = CreatePen(PS_SOLID, 1, COLORREF(rgb(13, 9, 6)));
    let brush = CreateSolidBrush(COLORREF(rgb(12, 8, 5)));
    let oldpen = SelectObject(dc, pen);
    let oldbrush = SelectObject(dc, brush);
    let _ = RoundRect(dc, r.left, r.top, r.right, r.bottom, 10, 10);
    let inner = RECT {
        left: r.left + 3,
        top: r.top + 3,
        right: r.right - 3,
        bottom: r.bottom - 3,
    };
    let inside = CreateSolidBrush(COLORREF(color));
    SelectObject(dc, inside);
    let _ = RoundRect(dc, inner.left, inner.top, inner.right, inner.bottom, 8, 8);
    SelectObject(dc, oldbrush);
    SelectObject(dc, oldpen);
    let _ = DeleteObject(pen);
    let _ = DeleteObject(brush);
    let _ = DeleteObject(inside);
    outline(dc, inner, rgb(113, 87, 59));
    line(
        dc,
        r.left + 7,
        r.top + 1,
        r.right - 7,
        r.top + 1,
        rgb(6, 4, 3),
        1,
    );
}
struct Font(HFONT, HGDIOBJ);
unsafe fn font(dc: HDC, size: i32) -> Font {
    let f = CreateFontW(
        -crate::wine_ui::pixels(size),
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
        w!("Consolas"),
    );
    Font(f, SelectObject(dc, f))
}
impl Font {
    unsafe fn restore(self, dc: HDC) {
        SelectObject(dc, self.1);
        let _ = DeleteObject(self.0);
    }
}
unsafe fn reflection(dc: HDC, r: RECT, c: [u8; 3], strength: f32, center: f32) {
    let w = (r.right - r.left).max(1);
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -1,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits = std::ptr::null_mut();
    let Ok(bitmap) = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, None, 0) else {
        return;
    };
    for (x, p) in std::slice::from_raw_parts_mut(bits.cast::<u8>(), w as usize * 4)
        .chunks_exact_mut(4)
        .enumerate()
    {
        let a = (-((x as f32 / w as f32 - center) / 0.16).powi(2)).exp() * strength;
        p.copy_from_slice(&[
            (c[2] as f32 * a) as u8,
            (c[1] as f32 * a) as u8,
            (c[0] as f32 * a) as u8,
            (255. * a) as u8,
        ]);
    }
    let mem = CreateCompatibleDC(dc);
    let old = SelectObject(mem, bitmap);
    let _ = AlphaBlend(
        dc,
        r.left,
        r.top,
        w,
        r.bottom - r.top,
        mem,
        0,
        0,
        w,
        1,
        BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        },
    );
    SelectObject(mem, old);
    let _ = DeleteObject(bitmap);
    let _ = DeleteDC(mem);
}

unsafe fn layout_field_edit(hwnd:HWND) {
    if GetWindowLongPtrW(hwnd,GWL_STYLE) as u32 & 3 != CBS_DROPDOWN as u32 {return;}
    let mut info=COMBOBOXINFO{cbSize:std::mem::size_of::<COMBOBOXINFO>() as u32,..Default::default()};
    if GetComboBoxInfo(hwnd,&mut info).is_err() || info.hwndItem.0.is_null() {return;}
    let r=client(hwnd);let dpi=GetDpiForWindow(hwnd) as i32;
    let inset=(5*dpi/96).max(4);let left=10*dpi/96;let arrow=28*dpi/96;
    let height=(info.rcItem.bottom-info.rcItem.top).min(r.bottom-inset*2).max(1);
    let top=(r.bottom-height)/2;let width=(r.right-left-arrow-inset).max(1);
    let mut current=RECT::default();let _=GetWindowRect(info.hwndItem,&mut current);
    let mut origin=POINT{x:current.left,y:current.top};let _=ScreenToClient(hwnd,&mut origin);
    if origin.x!=left || origin.y!=top || current.right-current.left!=width || current.bottom-current.top!=height {
        let _=SetWindowPos(info.hwndItem,None,left,top,width,height,SWP_NOZORDER|SWP_NOACTIVATE);
    }
}
/// Consistent light contrast rim for editable settings fields. The native
/// edit still owns text, masking, selection, scrolling and accessibility.
pub unsafe extern "system" fn text_field_proc(hwnd:HWND,msg:u32,wp:WPARAM,lp:LPARAM,_id:usize,_data:usize)->LRESULT {
    if msg==WM_NCCALCSIZE {
        let r=if wp.0!=0{&mut (*(lp.0 as *mut NCCALCSIZE_PARAMS)).rgrc[0]}else{&mut *(lp.0 as *mut RECT)};
        let edge=(GetDpiForWindow(hwnd) as i32/96).max(1);r.left+=edge;r.top+=edge;r.right-=edge;r.bottom-=edge;return LRESULT(0);
    }
    if msg==WM_NCPAINT || msg==WM_PRINT {
        if msg==WM_PRINT {DefSubclassProc(hwnd,msg,wp,lp);}
        let dc=if msg==WM_PRINT{HDC(wp.0 as _)}else{GetWindowDC(hwnd)};
        let mut r=RECT::default();let _=GetWindowRect(hwnd,&mut r);r.right-=r.left;r.bottom-=r.top;r.left=0;r.top=0;
        let brush=CreateSolidBrush(COLORREF(rgb(157,133,101)));
        for _ in 0..(GetDpiForWindow(hwnd)/96).max(1){FrameRect(dc,&r,brush);r.left+=1;r.top+=1;r.right-=1;r.bottom-=1;}
        let _=DeleteObject(brush);if msg!=WM_PRINT{ReleaseDC(hwnd,dc);}return LRESULT(0);
    }
    DefSubclassProc(hwnd,msg,wp,lp)
}
pub unsafe extern "system" fn field_proc(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if matches!(msg,WM_SIZE|WM_SETFONT|CB_SETITEMHEIGHT|CB_SHOWDROPDOWN|CB_SETCURSEL|WM_SETTEXT) {
        let result=DefSubclassProc(hwnd,msg,wp,lp);layout_field_edit(hwnd);let _=InvalidateRect(hwnd,None,false);return result;
    }
    if msg==WM_CTLCOLORLISTBOX {crate::scrollbars::attach(HWND(lp.0 as _));}
    if msg==WM_CTLCOLOREDIT {
        let dc=HDC(wp.0 as _);SetTextColor(dc,COLORREF(rgb(228,213,190)));
        SetBkColor(dc,COLORREF(rgb(43,31,22)));SetDCBrushColor(dc,COLORREF(rgb(43,31,22)));
        return LRESULT(GetStockObject(DC_BRUSH).0 as isize);
    }
    if msg == WM_PAINT || msg==WM_PRINTCLIENT || msg==WM_PRINT {
        layout_field_edit(hwnd);
        let mut ps = PAINTSTRUCT::default();
        let dc = if msg!=WM_PAINT {HDC(wp.0 as _)} else {BeginPaint(hwnd, &mut ps)};
        let r = client(hwnd);
        buffered_control(dc, r, |dc| {
            let dpi = GetDpiForWindow(hwnd) as i32;
            fill(dc,r,rgb(28,20,14));
            rounded_well(dc,r,rgb(43,31,22),(12*dpi/96).max(12));
            let editable=GetWindowLongPtrW(hwnd,GWL_STYLE) as u32 & 3 == CBS_DROPDOWN as u32;
            let space = 28 * dpi / 96;
            let language=GetDlgCtrlID(hwnd)==i18n::LANGUAGE as i32;
            if language {i18n::flag(dc,RECT{left:10*dpi/96,right:34*dpi/96,top:(r.bottom-16*dpi/96)/2,bottom:(r.bottom+16*dpi/96)/2},selected(hwnd));}
            if !editable {label(
                dc,
                RECT {
                    left: (if language{44}else{10}) * dpi / 96,
                    right: r.right - space,
                    ..r
                },
                &text_of(hwnd),
                14 * dpi / 96,
                rgb(228, 213, 190),
                false,
            );}
            let x = r.right - space / 2;
            let y = r.bottom / 2;
            let n = (4 * dpi / 96).max(3);
            line(dc, x - n, y - 2, x, y + n - 2, rgb(221, 193, 143), 2);
            line(dc, x, y + n - 2, x + n, y - 2, rgb(221, 193, 143), 2);

        });
        if msg==WM_PRINT && GetWindowLongPtrW(hwnd,GWL_STYLE) as u32 & 3 == CBS_DROPDOWN as u32 {
            let mut info=COMBOBOXINFO{cbSize:std::mem::size_of::<COMBOBOXINFO>() as u32,..Default::default()};
            if GetComboBoxInfo(hwnd,&mut info).is_ok() && !info.hwndItem.0.is_null() {
                let mut child=RECT::default();let _=GetWindowRect(info.hwndItem,&mut child);
                let mut origin=POINT{x:child.left,y:child.top};let _=ScreenToClient(hwnd,&mut origin);
                let saved=SaveDC(dc);let mut viewport=POINT::default();let _=GetViewportOrgEx(dc,&mut viewport);
                let _=SetViewportOrgEx(dc,viewport.x+origin.x,viewport.y+origin.y,None);
                IntersectClipRect(dc,0,0,child.right-child.left,child.bottom-child.top);
                SendMessageW(info.hwndItem,WM_PRINT,WPARAM(dc.0 as usize),LPARAM((PRF_CLIENT|PRF_ERASEBKGND) as isize));
                let _=RestoreDC(dc,saved);
            }
        }
        if msg==WM_PAINT {let _ = EndPaint(hwnd, &ps);}
        return LRESULT(0);
    }
    if msg == WM_NCPAINT {return LRESULT(0);}
    if msg == WM_ERASEBKGND {
        return LRESULT(1);
    }
    DefSubclassProc(hwnd, msg, wp, lp)
}

pub unsafe extern "system" fn tabs_proc(
    hwnd:HWND,msg:u32,wp:WPARAM,lp:LPARAM,_id:usize,_data:usize,
)->LRESULT {
    if matches!(msg,WM_PAINT|WM_PRINT|WM_PRINTCLIENT) {
        let mut ps=PAINTSTRUCT::default();
        let dc=if msg==WM_PAINT {BeginPaint(hwnd,&mut ps)}else{HDC(wp.0 as _)};
        let r=client(hwnd);
        buffered_control(dc,r,|dc| {
            APP.with(|cell| {
                if let Ok(app)=cell.try_borrow() {if let Some(app)=app.as_ref() {
                    let mut origin=POINT::default();let _=ClientToScreen(hwnd,&mut origin);
                    let _=ScreenToClient(app.settings,&mut origin);
                    let parent=client(app.settings);
                    let shifted=RECT{left:-origin.x,top:-origin.y,right:parent.right-origin.x,bottom:parent.bottom-origin.y};
                    let dpi=GetDpiForWindow(hwnd);
                    app.skin.settings_background(dc,shifted,dpi);
                    let selected=SendMessageW(hwnd,TCM_GETCURSEL,WPARAM(0),LPARAM(0)).0.clamp(0,6) as usize;
                    app.skin.settings_widget(dc,shifted,hwnd,selected,dpi);
                }}
            });
        });
        if msg==WM_PAINT {let _=EndPaint(hwnd,&ps);}
        return LRESULT(0);
    }
    if msg==WM_ERASEBKGND {return LRESULT(1);}
    DefSubclassProc(hwnd,msg,wp,lp)
}

#[derive(Clone, Copy, Default)]
struct Animation {
    hovered: bool,
    hover: f32,
    press: f32,
    sweep: f32,
}
struct Motion {
    start: Instant,
    last: Instant,
    inside: bool,
    down: bool,
    value: Animation,
}
thread_local! {static MOTION:RefCell<HashMap<isize,Motion>>=RefCell::new(HashMap::new());}
pub fn hovered(hwnd:HWND)->bool {animation(hwnd).hovered}
fn animation(hwnd: HWND) -> Animation {
    MOTION.with(|m| {
        m.borrow()
            .get(&(hwnd.0 as isize))
            .map(|m| m.value)
            .unwrap_or_default()
    })
}
pub unsafe extern "system" fn button_proc(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if msg==WM_LBUTTONDOWN {SendMessageW(hwnd,WM_CHANGEUISTATE,WPARAM((UIS_SET | (UISF_HIDEFOCUS<<16)) as usize),LPARAM(0));}
    if msg==WM_KEYDOWN {SendMessageW(hwnd,WM_CHANGEUISTATE,WPARAM((UIS_CLEAR | (UISF_HIDEFOCUS<<16)) as usize),LPARAM(0));}
    if msg == WM_NCDESTROY {
        MOTION.with(|m| m.borrow_mut().remove(&(hwnd.0 as isize)));
        let _ = KillTimer(hwnd, 91);
    }
    if matches!(
        msg,
        WM_MOUSEMOVE | WM_MOUSELEAVE | WM_LBUTTONDOWN | WM_LBUTTONUP | WM_CAPTURECHANGED | WM_TIMER
    ) {
        MOTION.with(|map| {
            let mut map = map.borrow_mut();
            let m = map.entry(hwnd.0 as isize).or_insert_with(|| Motion {
                start: Instant::now(),
                last: Instant::now(),
                inside: false,
                down: false,
                value: Animation::default(),
            });
            match msg {
                WM_MOUSEMOVE => {
                    if !m.inside {
                        m.start = Instant::now();
                        m.inside = true;
                        let mut track = TRACKMOUSEEVENT {
                            cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                            dwFlags: TME_LEAVE,
                            hwndTrack: hwnd,
                            ..Default::default()
                        };
                        let _ = TrackMouseEvent(&mut track);
                    }
                }
                WM_MOUSELEAVE => m.inside = false,
                WM_LBUTTONDOWN => m.down = true,
                WM_LBUTTONUP | WM_CAPTURECHANGED => m.down = false,
                _ => {}
            }
            m.value.hovered = m.inside;
            let dt = m.last.elapsed().as_secs_f32();
            m.last = Instant::now();
            let k = (dt * 18.).min(1.);
            m.value.hover += (if m.inside { 1. } else { 0. } - m.value.hover) * k;
            m.value.press += (if m.down { 1. } else { 0. } - m.value.press) * (dt * 30.).min(1.);
            m.value.sweep = (m.start.elapsed().as_secs_f32() / 0.75).min(1.);
            let _ = InvalidateRect(hwnd, None, false);
            if (m.value.hover - if m.inside { 1. } else { 0. }).abs() > 0.01
                || (m.value.press - if m.down { 1. } else { 0. }).abs() > 0.01
                || m.value.sweep < 1.
            {
                SetTimer(hwnd, 91, 16, None);
            } else {
                let _ = KillTimer(hwnd, 91);
            }
        });
        if msg == WM_TIMER && wp.0 == 91 {
            return LRESULT(0);
        }
    }
    DefSubclassProc(hwnd, msg, wp, lp)
}

pub unsafe extern "system" fn channel_proc(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if msg == WM_PAINT {
        let mut ps = PAINTSTRUCT::default();
        let dc = BeginPaint(hwnd, &mut ps);
        APP.with(|c| {
            if let Ok(a) = c.try_borrow() {
                if let Some(a) = a.as_ref() {
                    buffered_control(dc, client(hwnd), |dc| a.skin.channel(a, dc, client(hwnd)));
                }
            }
        });
        let _ = EndPaint(hwnd, &ps);
        return LRESULT(0);
    }
    if msg == WM_ERASEBKGND {
        return LRESULT(1);
    }
    DefSubclassProc(hwnd, msg, wp, lp)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn translated_dac_buttons_fit_full_labels() {unsafe{
        let previous=i18n::index();
        for lang in 0..4 {i18n::set(lang);
            for height in [354,547,820] {
                let r=RECT{left:0,top:0,right:height*1679/547,bottom:height};
                let buttons=translated_buttons(HWND::default(),r);
                let dc=GetDC(HWND::default());let scale=height as f64/547.;
                let f=font(dc,(46.*scale*0.31).round().max(11.) as i32);
                let mut right=0.;
                for ((_,x,_,width),label) in buttons[8..].iter().zip(["GUIDE","SNAPSHOT","AUDIO","CC OFF","LIBRARY","OPEN","SETTINGS"]){
                    let text=wide(&i18n::text(label).to_uppercase());let mut size=SIZE::default();
                    let _=GetTextExtentPoint32W(dc,&text[..text.len()-1],&mut size);
                    assert!(*x>=right&&x+width<=1133.,"Overlapping translated button: {lang} {label}");
                    assert!(size.cx as f64+(46.*0.36+46./7.+16.)*scale<=*width as f64*scale,"Clipped translation: {lang} {label}");
                    right=x+width;
                }
                f.restore(dc);ReleaseDC(HWND::default(),dc);
            }
        }
        i18n::set(previous);
    }}
    #[test]
    fn contextual_time_has_one_counter() {
        assert_eq!(
            timeline_time(true, false, true, false, 1110., 1112.),
            "00:18:32"
        );
        assert_eq!(
            timeline_time(true, false, true, false, 600., 1112.),
            "00:10:00"
        );
        assert_eq!(
            timeline_time(false, true, true, false, 600., 3600.),
            "00:10:00"
        );
        assert_eq!(timeline_time(false, false, false, false, 0., 0.), "—");
        assert_eq!(clock_time(3661.), "01:01:01");
        assert_eq!(timeline_time(true,false,true,true,1110.,1112.),"00:18:30");
        assert_eq!(timeline_time(true,false,true,false,1798.,1800.),"00:30:00");
        assert_eq!(timeline_time(true,false,true,false,900.,1800.),"00:15:00");
        assert_eq!(timeline_time(true,false,true,false,1798.,1800.),"00:30:00");
    }
    #[test]
    fn material_preferences_are_independent_and_compatible() {
        for name in ["metal", "glass", "plastic"] {
            assert_eq!(Material::parse(name).name(), name);
        }
        assert_eq!(Material::parse("future"), Material::Metal);
    }
    #[test]
    fn transport_and_utility_have_clear_spacing() {
        for pair in BUTTONS.windows(2) {
            if pair[0].2 == pair[1].2 {
                assert!(pair[0].1 + pair[0].3 < pair[1].1)
            }
        }
    }
}

/// A quiet solid keyboard-focus cue in the same metal palette, never dotted.
pub unsafe fn focus_outline(dc:HDC,r:&RECT)->BOOL {
    let focused=GetFocus();
    if !focused.0.is_null() && SendMessageW(focused,WM_QUERYUISTATE,WPARAM(0),LPARAM(0)).0 as u32 & UISF_HIDEFOCUS !=0 {return BOOL(1);}
    let pen=CreatePen(PS_SOLID,1,COLORREF(rgb(157,133,101)));let old=SelectObject(dc,pen);let b=SelectObject(dc,GetStockObject(NULL_BRUSH));
    let _=RoundRect(dc,r.left+2,r.top+2,r.right-2,r.bottom-2,5,5);SelectObject(dc,b);SelectObject(dc,old);let _=DeleteObject(pen);BOOL(1)
}
