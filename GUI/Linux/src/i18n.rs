use std::collections::HashMap;
use std::sync::{atomic::{AtomicUsize,Ordering},OnceLock};

pub const NAMES:[&str;4]=["English","Português (Brasil)","Español","Ελληνικά"];
pub const CODES:[&str;4]=["en","pt-BR","es","el"];
static INDEX:AtomicUsize=AtomicUsize::new(0);
static CATALOG:OnceLock<HashMap<String,[String;4]>>=OnceLock::new();
fn catalog()->&'static HashMap<String,[String;4]>{
    CATALOG.get_or_init(||serde_json::from_str(
        include_str!("../../Windows/src/translations.json"))
        .expect("shared Windows translation catalog"))
}
pub fn index()->usize{INDEX.load(Ordering::Relaxed)}
pub fn code()->&'static str{CODES[index()]}
pub fn set(index:usize){INDEX.store(index.min(3),Ordering::Relaxed);}
pub fn load(code:&str){set(CODES.iter().position(|c|*c==code).unwrap_or(0));}
fn captures<'a>(pattern:&str,value:&'a str)->Option<Vec<&'a str>>{
    let pieces:Vec<_>=pattern.split("{}").collect();
    if pieces.len()<2{return None;}
    let mut rest=value.strip_prefix(pieces[0])?;
    let mut captures=Vec::new();
    for (i,next) in pieces[1..].iter().enumerate(){
        if i==pieces.len()-2{
            captures.push(rest.strip_suffix(next)?);return Some(captures);
        }
        let end=rest.find(next)?;
        captures.push(&rest[..end]);rest=&rest[end+next.len()..];
    }
    None
}
pub fn text(source:&str)->String{translate(source,index())}
pub fn translate(source:&str,lang:usize)->String{
    let lang=lang.min(3);
    if lang==0{return source.into();}
    if let Some(value)=catalog().get(source){return value[lang].clone();}
    let trim=source.trim();
    if trim!=source{
        let leading=source.len()-source.trim_start().len();
        let trailing=source.len()-source.trim_end().len();
        return format!("{}{}{}",&source[..leading],translate(trim,lang),
            &source[source.len()-trailing..]);
    }
    let upper=source.chars().any(char::is_alphabetic)&&source==source.to_uppercase();
    if upper {
        if let Some((_,value))=catalog().iter()
            .find(|(key,_)|key.to_uppercase()==source){
            return value[lang].to_uppercase();
        }
    }
    for (key,value) in catalog(){
        if !key.contains("{}"){continue;}
        if let Some(args)=captures(key,source){
            let mut out=String::new();let mut args=args.iter();
            for (i,part) in value[lang].split("{}").enumerate(){
                if i>0{out.push_str(args.next().copied().unwrap_or(""));}
                out.push_str(part);
            }
            return out;
        }
    }
    source.into()
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn shared_catalog_translates_controls(){
        assert_eq!(translate("Settings",3),"Ρυθμίσεις");
        assert_eq!(translate("LIVE",1),"AO VIVO");
        assert_eq!(translate("TV blocking",1),"Bloqueio de TV");
    }
}


// Keep original captions as Windows does; language changes never reconstruct widgets.
use gtk::prelude::*;
use std::cell::{Cell,RefCell};
enum Binding {
    Label(gtk::glib::WeakRef<gtk::Label>,String),
    Button(gtk::glib::WeakRef<gtk::Button>,String),
    Combo(gtk::glib::WeakRef<gtk::ComboBoxText>,Vec<String>),
    Placeholder(gtk::glib::WeakRef<gtk::Entry>,String),
    Title(gtk::glib::WeakRef<gtk::Window>,String),
}
thread_local! {
    static BINDINGS:RefCell<Vec<Binding>>=const {RefCell::new(Vec::new())};
    static REFRESHING:Cell<bool>=const {Cell::new(false)};
}
pub fn refreshing()->bool{REFRESHING.with(Cell::get)}
pub fn label(source:&str)->gtk::Label{
    let widget=gtk::Label::new(None);set_label(&widget,source);widget
}
pub fn set_label(widget:&gtk::Label,source:&str){
    widget.set_text(&text(source));
    BINDINGS.with(|items|{
        let mut items=items.borrow_mut();
        for item in items.iter_mut(){
            if let Binding::Label(weak,stored)=item{
                if weak.upgrade().as_ref()==Some(widget){*stored=source.into();return;}
            }
        }
        items.push(Binding::Label(widget.downgrade(),source.into()));
    });
}
pub fn button(source:&str)->gtk::Button{
    let widget=gtk::Button::new();set_button(&widget,source);widget
}
pub fn set_button(widget:&gtk::Button,source:&str){
    widget.set_label(&text(source));
    BINDINGS.with(|items|{
        let mut items=items.borrow_mut();
        for item in items.iter_mut(){
            if let Binding::Button(weak,stored)=item{
                if weak.upgrade().as_ref()==Some(widget){*stored=source.into();return;}
            }
        }
        items.push(Binding::Button(widget.downgrade(),source.into()));
    });
}
pub fn append(combo:&gtk::ComboBoxText,source:&str){
    combo.append_text(&text(source));
    BINDINGS.with(|items|{
        let mut items=items.borrow_mut();
        for item in items.iter_mut(){
            if let Binding::Combo(weak,values)=item{
                if weak.upgrade().as_ref()==Some(combo){values.push(source.into());return;}
            }
        }
        items.push(Binding::Combo(combo.downgrade(),vec![source.into()]));
    });
}
pub fn placeholder(entry:&gtk::Entry,source:&str){
    entry.set_placeholder_text(Some(&text(source)));
    BINDINGS.with(|items|items.borrow_mut().push(Binding::Placeholder(entry.downgrade(),source.into())));
}
pub fn title(window:&gtk::Window,source:&str){
    window.set_title(&text(source));
    BINDINGS.with(|items|items.borrow_mut().push(Binding::Title(window.downgrade(),source.into())));
}
pub fn refresh(){
    REFRESHING.with(|flag|flag.set(true));
    BINDINGS.with(|items|items.borrow_mut().retain(|binding|match binding{
        Binding::Label(weak,source)=>weak.upgrade().is_some_and(|w|{w.set_text(&text(source));true}),
        Binding::Button(weak,source)=>weak.upgrade().is_some_and(|w|{w.set_label(&text(source));true}),
        Binding::Placeholder(weak,source)=>weak.upgrade().is_some_and(|w|{w.set_placeholder_text(Some(&text(source)));true}),
        Binding::Title(weak,source)=>weak.upgrade().is_some_and(|w|{w.set_title(&text(source));true}),
        Binding::Combo(weak,values)=>weak.upgrade().is_some_and(|w|{
            if let Some(model)=w.model().and_then(|m|m.downcast::<gtk::ListStore>().ok()){
                for (row,source) in values.iter().enumerate(){
                    if let Some(iter)=model.iter_nth_child(None,row as i32){model.set(&iter,&[(0,&text(source))]);}
                }
            }
            true
        }),
    }));
    REFRESHING.with(|flag|flag.set(false));
    for window in gtk::Window::list_toplevels(){window.queue_draw();}
}
