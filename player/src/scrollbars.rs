//! Skin native combo-list scrollbars while retaining the list's selection and keyboard behavior.
use crate::*;
use std::cell::RefCell;
#[derive(Clone,Copy)] struct Drag {hwnd:HWND,offset:i32,action:i32,restore_capture:HWND}
thread_local!{static DRAG:RefCell<Option<Drag>>=const{RefCell::new(None)};}
pub unsafe fn attach(hwnd:HWND){
    if hwnd.0.is_null() || !GetPropW(hwnd,w!("OrbitScrollbar")).0.is_null(){return;}
    let _=SetPropW(hwnd,w!("OrbitScrollbar"),HANDLE(1 as _));
    let _=SetWindowTheme(hwnd,w!(""),w!(""));
    let _=SetWindowSubclass(hwnd,Some(proc),28,0);
}
// Intercept captured client clicks and wheels at either native capture owner.
pub unsafe fn attach_combo(combo: HWND) {
    let mut info=COMBOBOXINFO{cbSize:std::mem::size_of::<COMBOBOXINFO>() as u32,..Default::default()};
    if GetComboBoxInfo(combo,&mut info).is_ok() {
        attach(info.hwndList);
        let _=SetWindowSubclass(combo,Some(combo_proc),31,info.hwndList.0 as usize);
    }
}
pub unsafe fn close_combo(combo: HWND) {
    let mut info=COMBOBOXINFO{cbSize:std::mem::size_of::<COMBOBOXINFO>() as u32,..Default::default()};
    if GetComboBoxInfo(combo,&mut info).is_ok(){finish(info.hwndList);}
}
// A parent wheel shortcut must not tune a channel while a dropdown is open.
pub unsafe fn wheel_combo(combo: HWND, wp: WPARAM) -> bool {
    if combo.0.is_null() || SendMessageW(combo,CB_GETDROPPEDSTATE,WPARAM(0),LPARAM(0)).0==0{return false;}
    let mut info=COMBOBOXINFO{cbSize:std::mem::size_of::<COMBOBOXINFO>() as u32,..Default::default()};
    if GetComboBoxInfo(combo,&mut info).is_ok(){wheel(info.hwndList,wp);}
    true
}
unsafe fn wheel(hwnd: HWND, wp: WPARAM) {
    let key=w!("OrbitWheelRemainder");
    let delta=((wp.0>>16) as u16 as i16) as i32;
    let total=GetPropW(hwnd,key).0 as isize as i32+delta;
    let ticks=total/120;
    let _=SetPropW(hwnd,key,HANDLE((total%120) as isize as _));
    let mut lines=3u32;
    let _=SystemParametersInfoW(SPI_GETWHEELSCROLLLINES,0,Some((&mut lines as *mut u32).cast()),SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0));
    if let Some((_,info))=geometry(hwnd) {
        let step=if lines==u32::MAX{info.nPage.max(1)}else{lines};
        let max=(info.nMax-info.nPage as i32+1).max(info.nMin);
        let top=(info.nPos as i64-ticks as i64*step as i64).clamp(info.nMin as i64,max as i64) as i32;
        set_top(hwnd,top);draw(hwnd);
    }
}
unsafe fn mouse_point(hwnd:HWND,msg:u32,lp:LPARAM)->POINT {
    let mut pt=POINT{x:lp.0 as u16 as i16 as i32,y:(lp.0>>16) as u16 as i16 as i32};
    if matches!(msg,WM_LBUTTONDOWN|WM_LBUTTONDBLCLK|WM_MOUSEMOVE){let _=ClientToScreen(hwnd,&mut pt);}
    pt
}
unsafe fn begin(hwnd:HWND,pt:POINT)->bool {
    let Some((bar,_))=geometry(hwnd) else{return false;};
    let r=bar.rcScrollBar;
    if pt.x<r.left || pt.x>=r.right || pt.y<r.top || pt.y>=r.bottom{return false;}
    if let Some(previous)=DRAG.with(|d|*d.borrow()){finish(previous.hwnd);}
    let y=pt.y-r.top;let height=r.bottom-r.top;
    let action=if y<bar.dxyLineButton{-1}else if y>=height-bar.dxyLineButton{1}else if y<bar.xyThumbTop{-2}else if y>=bar.xyThumbBottom{2}else{0};
    let capture=GetCapture();
    DRAG.with(|d|*d.borrow_mut()=Some(Drag{hwnd,offset:y-bar.xyThumbTop,action,restore_capture:capture}));
    if capture.0.is_null(){SetCapture(hwnd);}else if capture!=hwnd{let _=SetWindowSubclass(capture,Some(capture_proc),30,0);}
    if action!=0{scroll(hwnd,action);SetTimer(hwnd,28,350,None);}
    draw(hwnd);true
}
unsafe extern "system" fn combo_proc(hwnd:HWND,msg:u32,wp:WPARAM,lp:LPARAM,_id:usize,data:usize)->LRESULT {
    let list=HWND(data as _);
    if matches!(msg,WM_MOUSEWHEEL|WM_LBUTTONDOWN|WM_LBUTTONDBLCLK|WM_NCLBUTTONDOWN) && SendMessageW(hwnd,CB_GETDROPPEDSTATE,WPARAM(0),LPARAM(0)).0!=0 {
        if msg==WM_MOUSEWHEEL{wheel(list,wp);return LRESULT(0);}
        if matches!(msg,WM_LBUTTONDOWN|WM_LBUTTONDBLCLK|WM_NCLBUTTONDOWN) && begin(list,mouse_point(hwnd,msg,lp)){return LRESULT(0);}
    }
    if msg==WM_NCDESTROY || (msg==CB_SHOWDROPDOWN && wp.0==0){finish(list);}
    DefSubclassProc(hwnd,msg,wp,lp)
}
unsafe fn geometry(hwnd:HWND)->Option<(SCROLLBARINFO,SCROLLINFO)> {
    let mut bar=SCROLLBARINFO{cbSize:std::mem::size_of::<SCROLLBARINFO>() as u32,..Default::default()};
    let mut info=SCROLLINFO{cbSize:std::mem::size_of::<SCROLLINFO>() as u32,fMask:SIF_ALL,..Default::default()};
    if GetScrollBarInfo(hwnd,OBJID_VSCROLL,&mut bar).is_err() || GetScrollInfo(hwnd,SB_VERT,&mut info).is_err() || info.nMax-info.nMin+1<=info.nPage as i32{return None;}
    Some((bar,info))
}
unsafe fn draw(hwnd:HWND){draw_into(hwnd,None);}
unsafe fn draw_into(hwnd:HWND,target:Option<HDC>){
    let Some((bar,_))=geometry(hwnd) else{return;};
    let mut wr=RECT::default();let _=GetWindowRect(hwnd,&mut wr);
    let r=RECT{left:bar.rcScrollBar.left-wr.left,top:bar.rcScrollBar.top-wr.top,right:bar.rcScrollBar.right-wr.left,bottom:bar.rcScrollBar.bottom-wr.top};
    let dc=target.unwrap_or_else(||GetWindowDC(hwnd));if dc.0.is_null(){return;}
    fill(dc,r,orbit::rgb(27,20,15));
    let border=CreateSolidBrush(COLORREF(orbit::rgb(108,83,57)));FrameRect(dc,&r,border);let _=DeleteObject(border);
    let pad=(2*GetDpiForWindow(hwnd)/96).max(2) as i32;
    let thumb=RECT{left:r.left+pad,right:r.right-pad,top:r.top+bar.xyThumbTop,bottom:r.top+bar.xyThumbBottom};
    let pressed=DRAG.with(|d|d.borrow().is_some_and(|d|d.hwnd==hwnd && d.action==0));
    orbit::rounded_well(dc,thumb,orbit::rgb(if pressed{119}else{148},if pressed{100}else{127},if pressed{75}else{98}),pad*2);
    for (y,sign) in [(r.top+bar.dxyLineButton/2,-1),(r.bottom-bar.dxyLineButton/2,1)] {
        let x=(r.left+r.right)/2;let d=((r.right-r.left)/5).max(2);
        let p=CreatePen(PS_SOLID,pad.max(1),COLORREF(orbit::CREAM));let old=SelectObject(dc,p);
        let _=MoveToEx(dc,x-d,y-sign*d/2,None);let _=LineTo(dc,x,y+sign*d/2);let _=LineTo(dc,x+d,y-sign*d/2);SelectObject(dc,old);let _=DeleteObject(p);
    }
    if target.is_none(){ReleaseDC(hwnd,dc);}
}
unsafe fn set_top(hwnd:HWND,top:i32){let mut class=[0u16;32];let n=GetClassNameW(hwnd,&mut class);if String::from_utf16_lossy(&class[..n as usize]).eq_ignore_ascii_case("Edit") {let current=SendMessageW(hwnd,EM_GETFIRSTVISIBLELINE,WPARAM(0),LPARAM(0)).0 as i32;SendMessageW(hwnd,EM_LINESCROLL,WPARAM(0),LPARAM((top-current) as isize));}else{SendMessageW(hwnd,LB_SETTOPINDEX,WPARAM(top as usize),LPARAM(0));}}
unsafe fn scroll(hwnd:HWND,action:i32){
    if let Some((_,info))=geometry(hwnd){
        let step=if action.abs()==2{info.nPage.max(1) as i32}else{1};
        let top=(info.nPos+action.signum()*step).clamp(info.nMin,(info.nMax-info.nPage as i32+1).max(info.nMin));
        set_top(hwnd,top);draw(hwnd);
    }
}
unsafe fn finish(hwnd:HWND){
    let value=DRAG.with(|d|{let mut state=d.borrow_mut();if state.is_some_and(|v|v.hwnd==hwnd){state.take()}else{None}});let _=KillTimer(hwnd,28);
    if let Some(d)=value {if !d.restore_capture.0.is_null() && d.restore_capture!=hwnd{let _=windows::Win32::UI::Shell::RemoveWindowSubclass(d.restore_capture,Some(capture_proc),30);}
        if GetCapture()==hwnd && d.restore_capture!=hwnd {if d.restore_capture.0.is_null(){let _=ReleaseCapture();}else{SetCapture(d.restore_capture);}}}
    draw(hwnd);
}
unsafe extern "system" fn proc(hwnd:HWND,msg:u32,wp:WPARAM,lp:LPARAM,_id:usize,_data:usize)->LRESULT {
    if msg==WM_NCDESTROY {finish(hwnd);let _=RemovePropW(hwnd,w!("OrbitScrollbar"));let _=RemovePropW(hwnd,w!("OrbitWheelRemainder"));return DefSubclassProc(hwnd,msg,wp,lp);}
    if matches!(msg,WM_NCLBUTTONDOWN|WM_LBUTTONDOWN|WM_LBUTTONDBLCLK) && begin(hwnd,mouse_point(hwnd,msg,lp)){return LRESULT(0);}
    if msg==WM_MOUSEWHEEL{wheel(hwnd,wp);return LRESULT(0);}
    if msg==WM_TIMER && wp.0==28 {if let Some(d)=DRAG.with(|d|*d.borrow()){if d.hwnd==hwnd && d.action!=0{scroll(hwnd,d.action);SetTimer(hwnd,28,65,None);}}return LRESULT(0);}
    if matches!(msg,WM_MOUSEMOVE|WM_NCMOUSEMOVE) {
        if let Some(d)=DRAG.with(|d|*d.borrow()).filter(|d|d.hwnd==hwnd){
            if d.action==0 {if let Some((bar,info))=geometry(hwnd){let pt=mouse_point(hwnd,msg,lp);let track=(bar.rcScrollBar.bottom-bar.rcScrollBar.top-2*bar.dxyLineButton-(bar.xyThumbBottom-bar.xyThumbTop)).max(1);let y=(pt.y-bar.rcScrollBar.top-d.offset-bar.dxyLineButton).clamp(0,track);let range=(info.nMax-info.nMin-info.nPage as i32+1).max(0);let top=info.nMin+((y as i64*range as i64+track as i64/2)/track as i64) as i32;set_top(hwnd,top);draw(hwnd);}}return LRESULT(0);
        }
    }
    if matches!(msg,WM_LBUTTONUP|WM_NCLBUTTONUP) && DRAG.with(|d|d.borrow().is_some_and(|d|d.hwnd==hwnd)){finish(hwnd);return LRESULT(0);}
    if msg==WM_CANCELMODE || msg==WM_CAPTURECHANGED {if DRAG.with(|d|d.borrow().is_some_and(|d|d.hwnd==hwnd)){finish(hwnd);}}
    let result=DefSubclassProc(hwnd,msg,wp,lp);
    if msg==WM_PRINT {draw_into(hwnd,Some(HDC(wp.0 as _)));}
    else if matches!(msg,WM_NCPAINT|WM_PAINT|WM_SIZE|WM_SHOWWINDOW|WM_MOUSEWHEEL|WM_KEYDOWN|WM_VSCROLL|LB_SETTOPINDEX|EM_LINESCROLL|WM_NCMOUSEMOVE){draw(hwnd);}
    result
}

// A combo popup already owns capture. Keep that owner so scrolling cannot close
// the dropdown; forward only an active scrollbar gesture to its list window.
unsafe extern "system" fn capture_proc(hwnd:HWND,msg:u32,wp:WPARAM,lp:LPARAM,_id:usize,_data:usize)->LRESULT {
    if let Some(d)=DRAG.with(|d|*d.borrow()).filter(|d|d.restore_capture==hwnd){
        if matches!(msg,WM_MOUSEMOVE|WM_LBUTTONUP){
            let mut pt=POINT{x:lp.0 as u16 as i16 as i32,y:(lp.0>>16) as u16 as i16 as i32};
            let _=ClientToScreen(hwnd,&mut pt);let _=ScreenToClient(d.hwnd,&mut pt);
            let mapped=LPARAM(((pt.x as u16 as u32)|((pt.y as u16 as u32)<<16)) as isize);
            return SendMessageW(d.hwnd,msg,wp,mapped);
        }
        if matches!(msg,WM_CAPTURECHANGED|WM_CANCELMODE|WM_NCDESTROY){finish(d.hwnd);}
    }
    DefSubclassProc(hwnd,msg,wp,lp)
}

#[cfg(test)]
mod tests {
    use super::*;
    unsafe fn dropdown() -> (HWND, HWND, HWND) {
        let parent=CreateWindowExW(WINDOW_EX_STYLE(0),w!("STATIC"),w!("Scrollbar regression"),WS_POPUP,
            100,100,320,300,None,None,GetModuleHandleW(None).unwrap(),None).unwrap();
        let combo=CreateWindowExW(WINDOW_EX_STYLE(0),w!("COMBOBOX"),w!(""),
            WS_CHILD|WS_VISIBLE|WS_VSCROLL|WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
            10,10,280,180,parent,None,GetModuleHandleW(None).unwrap(),None).unwrap();
        for i in 0..60 {let name=wide(&format!("Channel {i}"));SendMessageW(combo,CB_ADDSTRING,WPARAM(0),LPARAM(name.as_ptr() as isize));}
        SendMessageW(combo,CB_SETCURSEL,WPARAM(30),LPARAM(0));
        let mut info=COMBOBOXINFO{cbSize:std::mem::size_of::<COMBOBOXINFO>() as u32,..Default::default()};GetComboBoxInfo(combo,&mut info).unwrap();
        attach_combo(combo);
        SendMessageW(combo,CB_SHOWDROPDOWN,WPARAM(1),LPARAM(0));
        SendMessageW(info.hwndList,LB_SETTOPINDEX,WPARAM(20),LPARAM(0));
        assert_ne!(SendMessageW(combo,CB_GETDROPPEDSTATE,WPARAM(0),LPARAM(0)).0,0);
        (parent,combo,info.hwndList)
    }
    fn packed(x:i32,y:i32)->LPARAM {LPARAM(((x as u16 as u32)|((y as u16 as u32)<<16)) as isize)}
    #[test]
    fn captured_scrollbar_up_click_keeps_dropdown_open() {unsafe {
        let (parent,combo,list)=dropdown();
        let (bar,_)=geometry(list).expect("scrollbar present");
        let mut point=POINT{x:(bar.rcScrollBar.left+bar.rcScrollBar.right)/2,y:bar.rcScrollBar.top+bar.dxyLineButton/2};
        let _=ScreenToClient(list,&mut point);
        let before=SendMessageW(list,LB_GETTOPINDEX,WPARAM(0),LPARAM(0)).0;
        SendMessageW(list,WM_LBUTTONDOWN,WPARAM(1),packed(point.x,point.y));
        SendMessageW(list,WM_LBUTTONUP,WPARAM(0),packed(point.x,point.y));
        let open=SendMessageW(combo,CB_GETDROPPEDSTATE,WPARAM(0),LPARAM(0)).0;
        let after=SendMessageW(list,LB_GETTOPINDEX,WPARAM(0),LPARAM(0)).0;
        let selected=SendMessageW(combo,CB_GETCURSEL,WPARAM(0),LPARAM(0)).0;
        let _=DestroyWindow(parent);
        assert_ne!(open,0,"scrolling must not close the dropdown");
        assert!(after<before,"up arrow must move the list upward");
        assert_eq!(selected,30,"scrolling must not select a channel");
    }}
    #[test]
    fn shared_scrollbar_handles_arrows_thumb_and_wheel_without_selection() {unsafe {
        let (parent,combo,list)=dropdown();
        let selected=SendMessageW(combo,CB_GETCURSEL,WPARAM(0),LPARAM(0)).0;
        let capture=GetCapture();
        for down in [false,true] {
            let (bar,_)=geometry(list).unwrap();
            let mut point=POINT{x:(bar.rcScrollBar.left+bar.rcScrollBar.right)/2,
                y:if down{bar.rcScrollBar.bottom-bar.dxyLineButton/2}else{bar.rcScrollBar.top+bar.dxyLineButton/2}};
            let _=ScreenToClient(combo,&mut point);
            let before=SendMessageW(list,LB_GETTOPINDEX,WPARAM(0),LPARAM(0)).0;
            SendMessageW(combo,WM_LBUTTONDOWN,WPARAM(1),packed(point.x,point.y));
            SendMessageW(list,WM_TIMER,WPARAM(28),LPARAM(0));
            SendMessageW(list,WM_LBUTTONUP,WPARAM(0),LPARAM(0));
            let after=SendMessageW(list,LB_GETTOPINDEX,WPARAM(0),LPARAM(0)).0;
            assert!(if down{after>before}else{after<before});
            assert_eq!(GetCapture(),capture);
            assert!(DRAG.with(|d|d.borrow().is_none()));
            assert_ne!(SendMessageW(combo,CB_GETDROPPEDSTATE,WPARAM(0),LPARAM(0)).0,0);
        }
        let (bar,_)=geometry(list).unwrap();
        let mut pt=POINT{x:(bar.rcScrollBar.left+bar.rcScrollBar.right)/2,y:bar.rcScrollBar.top+(bar.xyThumbTop+bar.xyThumbBottom)/2};
        let _=ScreenToClient(list,&mut pt);
        SendMessageW(list,WM_LBUTTONDOWN,WPARAM(1),packed(pt.x,pt.y));
        let before=SendMessageW(list,LB_GETTOPINDEX,WPARAM(0),LPARAM(0)).0;
        SendMessageW(list,WM_MOUSEMOVE,WPARAM(1),packed(pt.x,pt.y+35));
        SendMessageW(list,WM_LBUTTONUP,WPARAM(0),packed(pt.x,pt.y+35));
        assert!(SendMessageW(list,LB_GETTOPINDEX,WPARAM(0),LPARAM(0)).0>before);
        SendMessageW(list,LB_SETTOPINDEX,WPARAM(20),LPARAM(0));
        assert!(wheel_combo(combo,WPARAM(60usize<<16)));
        assert_eq!(SendMessageW(list,LB_GETTOPINDEX,WPARAM(0),LPARAM(0)).0,20);
        assert!(wheel_combo(combo,WPARAM(60usize<<16)));
        assert!(SendMessageW(list,LB_GETTOPINDEX,WPARAM(0),LPARAM(0)).0<20);
        SendMessageW(list,LB_SETTOPINDEX,WPARAM(0),LPARAM(0));
        SendMessageW(combo,WM_MOUSEWHEEL,WPARAM(120usize<<16),LPARAM(0));
        assert_eq!(SendMessageW(list,LB_GETTOPINDEX,WPARAM(0),LPARAM(0)).0,0);
        assert_ne!(SendMessageW(combo,CB_GETDROPPEDSTATE,WPARAM(0),LPARAM(0)).0,0);
        assert_eq!(SendMessageW(combo,CB_GETCURSEL,WPARAM(0),LPARAM(0)).0,selected);
        SendMessageW(combo,CB_SHOWDROPDOWN,WPARAM(0),LPARAM(0));
        assert_eq!(SendMessageW(combo,CB_GETDROPPEDSTATE,WPARAM(0),LPARAM(0)).0,0);
        let _=DestroyWindow(parent);
    }}

}
