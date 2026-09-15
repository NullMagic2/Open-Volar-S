//! Runtime UI localization. Broadcast metadata, filenames and stored control values stay unchanged.
use crate::*;
use std::{collections::HashMap,sync::{OnceLock,atomic::{AtomicUsize,Ordering}}};
pub const LANGUAGE:u16=347;
pub const NAMES:[&str;4]=["English","Português (Brasil)","Español","Ελληνικά"];
pub const CODES:[&str;4]=["en","pt-BR","es","el"];
static LANGUAGE_INDEX:AtomicUsize=AtomicUsize::new(0);
static CATALOG:OnceLock<HashMap<String,[String;4]>>=OnceLock::new();
thread_local!{static COMBOS:RefCell<HashMap<isize,Vec<String>>>=RefCell::new(HashMap::new());}
thread_local!{static CAPTIONS:RefCell<HashMap<isize,String>>=RefCell::new(HashMap::new());}
fn catalog()->&'static HashMap<String,[String;4]>{CATALOG.get_or_init(||serde_json::from_str(include_str!("translations.json")).expect("validated translation catalog"))}
pub fn index()->usize{LANGUAGE_INDEX.load(Ordering::Relaxed)}
pub fn code()->&'static str{CODES[index()]}
pub fn set(index:usize){LANGUAGE_INDEX.store(index.min(3),Ordering::Relaxed);}
pub fn load(code:&str){set(CODES.iter().position(|c|*c==code).unwrap_or(0));}
pub fn remember(hwnd:HWND,source:&str){CAPTIONS.with(|c|{c.borrow_mut().insert(hwnd.0 as isize,source.into());});}
pub fn raw(hwnd:HWND)->Option<String>{CAPTIONS.with(|c|c.borrow().get(&(hwnd.0 as isize)).cloned())}
pub fn text(source:&str)->String{translate(source,index())}
fn captures<'a>(pattern:&str,value:&'a str)->Option<Vec<&'a str>> {
    let pieces:Vec<_>=pattern.split("{}").collect();if pieces.len()<2{return None;}
    let mut rest=value.strip_prefix(pieces[0])?;let mut captures=Vec::new();
    for (i,next) in pieces[1..].iter().enumerate(){
        if i==pieces.len()-2 {captures.push(rest.strip_suffix(next)?);return Some(captures);}
        let end=rest.find(next)?;captures.push(&rest[..end]);rest=&rest[end+next.len()..];
    }None
}
pub fn translate(source:&str,lang:usize)->String {
    let lang=lang.min(3);if lang==0{return source.into();}
    if let Some(value)=catalog().get(source){return value[lang].clone();}
    let trim=source.trim();if trim!=source {let leading=source.len()-source.trim_start().len();let trailing=source.len()-source.trim_end().len();return format!("{}{}{}",&source[..leading],translate(trim,lang),&source[source.len()-trailing..]);}
    let upper=source.chars().any(char::is_alphabetic) && source==source.to_uppercase();
    if upper {if let Some((_,value))=catalog().iter().find(|(key,_)|key.to_uppercase()==source){return value[lang].to_uppercase();}}
    for (key,value) in catalog(){
        if !key.contains("{}"){continue;}
        if let Some(args)=captures(key,source){let mut out=String::new();let mut args=args.iter();for (i,part) in value[lang].split("{}").enumerate(){if i>0{out.push_str(args.next().copied().unwrap_or(""));}out.push_str(part);}return out;}
    }
    source.into()
}
pub unsafe fn refresh(){
    let combos:Vec<_>=COMBOS.with(|c|c.borrow().iter().map(|(h,v)|(*h,v.clone())).collect());
    for (h,values) in combos {let hwnd=HWND(h as _);if !IsWindow(hwnd).as_bool(){continue;}let selected=SendMessageW(hwnd,CB_GETCURSEL,WPARAM(0),LPARAM(0)).0;let entered=text_of(hwnd);
        SendMessageW(hwnd,WM_SETREDRAW,WPARAM(0),LPARAM(0));SendMessageW(hwnd,CB_RESETCONTENT,WPARAM(0),LPARAM(0));for value in values{combo_add(hwnd,&value);}if selected>=0{SendMessageW(hwnd,CB_SETCURSEL,WPARAM(selected as usize),LPARAM(0));}else{set_data_text(hwnd,&entered);}SendMessageW(hwnd,WM_SETREDRAW,WPARAM(1),LPARAM(0));let _=InvalidateRect(hwnd,None,false);
    }

    let captions:Vec<_>=CAPTIONS.with(|c|c.borrow().iter().map(|(h,s)|(*h,s.clone())).collect());
    for (h,s) in captions{let hwnd=HWND(h as _);if !IsWindow(hwnd).as_bool(){CAPTIONS.with(|c|c.borrow_mut().remove(&h));continue;}let text=wide(&text(&s));let _=SetWindowTextW(hwnd,PCWSTR(text.as_ptr()));let _=InvalidateRect(hwnd,None,false);}
}
/// Flag artwork is drawn directly, so it works even when the system has no emoji flags.
pub unsafe fn flag(dc:HDC,r:RECT,language:usize){
    let saved=SaveDC(dc);IntersectClipRect(dc,r.left,r.top,r.right,r.bottom);
    let w=r.right-r.left;let h=r.bottom-r.top;let color=orbit::rgb;
    let rr=|x:i32,y:i32,ww:i32,hh:i32|RECT{left:r.left+x*w/30,top:r.top+y*h/20,right:r.left+(x+ww)*w/30,bottom:r.top+(y+hh)*h/20};
    let blue=color(30,66,132);let red=color(184,39,46);let white=color(242,238,225);
    match language {
        1=>{fill(dc,r,color(30,137,69));let brush=CreateSolidBrush(COLORREF(color(246,202,47)));let old=SelectObject(dc,brush);let pen=SelectObject(dc,GetStockObject(NULL_PEN));let points=[POINT{x:r.left+w/2,y:r.top+h/8},POINT{x:r.right-w/12,y:r.top+h/2},POINT{x:r.left+w/2,y:r.bottom-h/8},POINT{x:r.left+w/12,y:r.top+h/2}];let _=Polygon(dc,&points);SelectObject(dc,old);let _=DeleteObject(brush);let brush=CreateSolidBrush(COLORREF(blue));let old=SelectObject(dc,brush);let _=Ellipse(dc,r.left+w/2-h/4,r.top+h/4,r.left+w/2+h/4,r.bottom-h/4);SelectObject(dc,old);let _=DeleteObject(brush);SelectObject(dc,pen);fill(dc,rr(10,9,10,1),white);},
        2=>{fill(dc,r,red);fill(dc,rr(0,5,30,10),color(248,195,44));fill(dc,rr(8,8,3,5),color(165,48,39));fill(dc,rr(7,7,5,1),white);},
        3=>{fill(dc,r,white);for i in 0..5{fill(dc,RECT{top:r.top+i*h/9*2,bottom:r.top+((i*2+1)*h/9).min(h),..r},blue);}fill(dc,rr(0,0,13,11),blue);fill(dc,rr(5,0,3,11),white);fill(dc,rr(0,4,13,3),white);},
        _=>{fill(dc,r,blue);for (width,c) in [(h/4,white),(h/10,red)]{let p=CreatePen(PS_SOLID,width.max(1),COLORREF(c));let old=SelectObject(dc,p);let _=MoveToEx(dc,r.left,r.top,None);let _=LineTo(dc,r.right,r.bottom);let _=MoveToEx(dc,r.right,r.top,None);let _=LineTo(dc,r.left,r.bottom);SelectObject(dc,old);let _=DeleteObject(p);}fill(dc,rr(0,7,30,6),white);fill(dc,rr(12,0,6,20),white);fill(dc,rr(0,8,30,4),red);fill(dc,rr(13,0,4,20),red);}
    }
    let b=CreateSolidBrush(COLORREF(color(132,108,77)));FrameRect(dc,&r,b);let _=DeleteObject(b);let _=RestoreDC(dc,saved);
}
#[cfg(test)]mod tests{use super::*;
    #[test]fn all_catalog_entries_have_four_translations_and_matching_slots(){for(key,values)in catalog(){for value in values{assert!(!value.trim().is_empty(),"{key}");assert_eq!(key.matches("{}").count(),value.matches("{}").count(),"{key}");}}}
    #[test]fn dynamic_labels_and_unicode(){assert_eq!(translate("Saturation 137%",1),"Saturação 137%");assert_eq!(translate("Brightness +17",2),"Brillo +17");assert_eq!(translate("Settings",3),"Ρυθμίσεις");assert_eq!(translate("LIVE",1),"AO VIVO");}
    #[test]fn unknown_metadata_and_paths_are_unchanged(){for s in ["TV TRIBUNA HD","Jornal Nacional",r"C:\Recordings\Cold.ts"]{for lang in 0..4{assert_eq!(translate(s,lang),s);}}}
}

pub fn tab(i:usize)->&'static str {if i==5 && index()!=0 {match index(){3=>"Γονικός",_=>"PARENTAL"}}else{["Video","Channels","Storage","Themes","Picture","Parental","General"][i]}}

pub unsafe extern "system" fn combo_proc(hwnd:HWND,msg:u32,wp:WPARAM,lp:LPARAM,_id:usize,_data:usize)->LRESULT {
    if msg==WM_NCDESTROY{COMBOS.with(|c|c.borrow_mut().remove(&(hwnd.0 as isize)));}
    if msg==CB_RESETCONTENT{COMBOS.with(|c|{c.borrow_mut().entry(hwnd.0 as isize).or_default().clear();});}
    if msg==CB_ADDSTRING && lp.0!=0 {
        let original=PCWSTR(lp.0 as *const u16).to_string().unwrap_or_default();let translated=wide(&text(&original));
        let result=DefSubclassProc(hwnd,msg,wp,LPARAM(translated.as_ptr() as isize));
        if result.0>=0 {COMBOS.with(|c|{let mut c=c.borrow_mut();let values=c.entry(hwnd.0 as isize).or_default();values.insert((result.0 as usize).min(values.len()),original);});}return result;
    }
    DefSubclassProc(hwnd,msg,wp,lp)
}
