//! Transparent Classic Tube OSD. Colors and font defaults recovered from AVerTV.exe.
use windows::{core::w, Win32::{Foundation::*,Graphics::Gdi::*,UI::WindowsAndMessaging::*}};
pub const KEY: COLORREF=COLORREF(0x00ff00ff);
pub const OPACITY: u8=191; // 75% foreground opacity; keyed background stays fully transparent.
pub const GREEN: COLORREF=COLORREF(0x0000ff00);
const EDGE: COLORREF=COLORREF(0x00646464);
/// Color-keyed child windows are composed above the video without reading GPU frames.
pub unsafe fn make_transparent(hwnd:HWND)->windows::core::Result<()> {
    SetWindowLongPtrW(hwnd,GWL_EXSTYLE,GetWindowLongPtrW(hwnd,GWL_EXSTYLE)|WS_EX_LAYERED.0 as isize|WS_EX_TRANSPARENT.0 as isize);
    SetLayeredWindowAttributes(hwnd,KEY,OPACITY,LWA_COLORKEY|LWA_ALPHA)
}
unsafe fn fill(dc:HDC,r:RECT,color:COLORREF){let b=CreateSolidBrush(color);FillRect(dc,&r,b);let _=DeleteObject(b);}
unsafe fn text(dc:HDC,r:RECT,s:&str,size:i32,edge:i32){
    // Crisp glyphs avoid a magenta antialias fringe against the transparent key.
    let font=CreateFontW(-size,0,0,0,700,0,0,0,DEFAULT_CHARSET.0 as u32,
        OUT_DEFAULT_PRECIS.0 as u32,CLIP_DEFAULT_PRECIS.0 as u32,NONANTIALIASED_QUALITY.0 as u32,
        DEFAULT_PITCH.0 as u32,w!("Microsoft Sans Serif"));
    let old=SelectObject(dc,font);SetBkMode(dc,TRANSPARENT);
    let mut chars:Vec<u16>=s.encode_utf16().collect();let flags=DT_SINGLELINE|DT_VCENTER|DT_NOPREFIX|DT_END_ELLIPSIS;
    SetTextColor(dc,EDGE);
    for (x,y) in [(-edge,-edge),(0,-edge),(edge,-edge),(-edge,0),(edge,0),(-edge,edge),(0,edge),(edge,edge)]{
        let mut shifted=RECT{left:r.left+x,top:r.top+y,right:r.right+x,bottom:r.bottom+y};DrawTextW(dc,&mut chars,&mut shifted,flags);
    }
    SetTextColor(dc,GREEN);let mut r=r;DrawTextW(dc,&mut chars,&mut r,flags);
    SelectObject(dc,old);let _=DeleteObject(font);
}
pub unsafe fn paint(dc:HDC,r:RECT,label:&str,volume:bool,dpi:u32){
    let px=|n:i32|(n*dpi as i32/96).max(1);
    fill(dc,r,KEY);
    let mut lines=label.lines();let headline=lines.next().unwrap_or("");
    text(dc,RECT{left:px(3),top:px(2),right:r.right-px(3),bottom:px(38)},
        &if volume{crate::i18n::text(&format!("Volume {}%",label))}else{headline.to_owned()},px(24),px(1));
    if !volume {for (index,detail) in lines.take(2).enumerate(){let top=42+index as i32*28;
        text(dc,RECT{left:px(3),top:px(top),right:r.right-px(3),bottom:px(top+26).min(r.bottom-px(2))},detail,px(18),px(1));
    }}
    if volume {
        let value=label.parse::<u32>().unwrap_or(0).min(100);
        let left=px(4);let gap=px(3);let width=((r.right-left-px(4))/20-gap).max(1);
        for i in 0..20 {
            let x=left+i*(width+gap);
            let rect=RECT{left:x,top:px(44),right:x+width,bottom:px(60)};
            // Unfilled segments remain transparent, with a thin gray outline.
            let border=CreateSolidBrush(EDGE);FrameRect(dc,&rect,border);let _=DeleteObject(border);
            if (i as u32)*5<value {fill(dc,RECT{left:rect.left+px(1),top:rect.top+px(1),right:rect.right-px(1),bottom:rect.bottom-px(1)},GREEN);}
        }
    }
}
