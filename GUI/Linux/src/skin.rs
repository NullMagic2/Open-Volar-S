use gtk::cairo::{Context, FontSlant, FontWeight, ImageSurface};
use super::i18n;
use std::collections::HashMap;
use std::cell::{Cell,RefCell};
use std::io::Cursor;

// Slightly larger control captions without scaling the surrounding artwork.
const BUTTON_FONT_RATIO:f64=0.36;
fn ui_font()->&'static str { if i18n::index()==3 {"Sans"} else {"Selawik"} }

const ART: &[(&str, &[u8])] = &[
    ("chassis", include_bytes!("../../Windows/assets/viewer/chassis.png")),
    ("settings-fascia", include_bytes!("../../Windows/assets/orbit/settings-fascia.png")),
    ("orbit-fascia", include_bytes!("../../Windows/assets/orbit/fascia.png")),
    ("module", include_bytes!("../../Windows/assets/orbit/module.png")),
    ("pointer", include_bytes!("../../Windows/assets/orbit/pointer.png")),
    ("dac-play", include_bytes!("../../Windows/assets/orbit/play.png")),
    ("dac-stop", include_bytes!("../../Windows/assets/orbit/stop.png")),
    ("dac-record", include_bytes!("../../Windows/assets/orbit/record.png")),
    ("dac-guide", include_bytes!("../../Windows/assets/orbit/guide.png")),
    ("dac-snapshot", include_bytes!("../../Windows/assets/orbit/snapshot.png")),
    ("dac-audio", include_bytes!("../../Windows/assets/orbit/audio.png")),
    ("dac-live", include_bytes!("../../Windows/assets/orbit/live.png")),
    ("dac-settings", include_bytes!("../../Windows/assets/orbit/settings.png")),
    ("dac-fullscreen", include_bytes!("../../Windows/assets/orbit/fullscreen.png")),
    ("dac-library", include_bytes!("../../Windows/assets/orbit/library.png")),
    ("dac-open", include_bytes!("../../Windows/assets/orbit/open.png")),
    ("dac-signal", include_bytes!("../../Windows/assets/orbit/signal.png")),
    ("dac-surround", include_bytes!("../../Windows/assets/orbit/surround.png")),
    ("dac-mono", include_bytes!("../../Windows/assets/orbit/mono.png")),
    ("dac-stereo", include_bytes!("../../Windows/assets/orbit/stereo.png")),
    ("dac-channel-face", include_bytes!("../../Windows/assets/orbit/channel-face.png")),
    ("dac-channel-frame", include_bytes!("../../Windows/assets/orbit/channel-frame.png")),
    ("dac-captions", include_bytes!("../../Windows/assets/orbit/captions.png")),
    ("dac-channel-chevron", include_bytes!("../../Windows/assets/orbit/channel-chevron.png")),
    ("dac-glass", include_bytes!("../../Windows/assets/orbit/glass.png")),
    ("dac-metal", include_bytes!("../../Windows/assets/orbit/metal.png")),
    ("dac-plastic", include_bytes!("../../Windows/assets/orbit/plastic.png")),
    ("dac-caption-screw", include_bytes!("../../Windows/assets/orbit/caption-screw.png")),
    ("dac-minimize", include_bytes!("../../Windows/assets/orbit/minimize.png")),
    ("dac-close", include_bytes!("../../Windows/assets/orbit/close.png")),
    ("dac-seek-back", include_bytes!("../../Windows/assets/orbit/seek-back.png")),
    ("dac-seek-forward", include_bytes!("../../Windows/assets/orbit/seek-forward.png")),
    ("rewind", include_bytes!("../../Windows/assets/viewer/rewind.png")),
    ("fast-forward", include_bytes!("../../Windows/assets/viewer/fast-forward.png")),
    ("pause", include_bytes!("../../Windows/assets/viewer/pause.png")),
    ("play", include_bytes!("../../Windows/assets/viewer/play.png")),
    ("stop", include_bytes!("../../Windows/assets/viewer/stop.png")),
    ("snapshot", include_bytes!("../../Windows/assets/viewer/snapshot.png")),
    ("guide", include_bytes!("../../Windows/assets/viewer/guide.png")),
    ("fullscreen", include_bytes!("../../Windows/assets/viewer/fullscreen.png")),
    ("settings", include_bytes!("../../Windows/assets/viewer/settings.png")),
    ("mounted-indicator-off", include_bytes!("../../Windows/assets/viewer/mounted-indicator-off.png")),
    ("mounted-indicator-on", include_bytes!("../../Windows/assets/viewer/mounted-indicator-on.png")),
    ("live-green-off", include_bytes!("../../Windows/assets/viewer/live-green-off.png")),
    ("live-green-on", include_bytes!("../../Windows/assets/viewer/live-green-on.png")),
];

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Action {
    Back,Forward,Play,Stop,Record,Previous,Next,VolumeDown,VolumeUp,
    Snapshot,Guide,Fullscreen,Audio,Deck,Settings,Live,Minimize,Maximize,Close,
    Captions,Library,Open,Channels,
}
const BUTTONS: &[(Action,f64,f64)] = &[
    (Action::Back,48.,62.),(Action::Forward,114.,62.),
    (Action::Play,180.,76.),(Action::Stop,260.,62.),
    (Action::Record,397.,48.),(Action::Previous,520.,80.),
    (Action::Next,600.,80.),(Action::VolumeDown,772.,80.),
    (Action::VolumeUp,852.,80.),(Action::Snapshot,1030.,52.),
    (Action::Guide,1090.,76.),(Action::Fullscreen,1174.,52.),
    (Action::Audio,1234.,84.),(Action::Deck,1326.,52.),
    (Action::Settings,1386.,166.),
];
pub struct DrawState<'a> {
    pub playing:bool,pub recording:bool,pub status:&'a str,
    pub paused:bool,pub live:bool,pub following_live:bool,
    pub position:f64,pub duration:f64,pub seekable:bool,pub drag:Option<f64>,
}
impl DrawState<'_>{
    fn timeline_fraction(&self)->f64{
        self.drag.unwrap_or_else(||if self.following_live&&!self.paused{1.}
            else if self.seekable&&self.duration>0.{(self.position/self.duration).clamp(0.,1.)}
            else{0.})
    }
}
pub struct Skin { art:HashMap<&'static str,ImageSurface>, material:Cell<usize>, hover:Cell<Option<Action>>,
    button_cache:RefCell<HashMap<(usize,i32,i32,u32),ImageSurface>> }
impl Skin {
    pub fn new()->Self {
        let art=ART.iter().map(|&(name,bytes)|{
            let image=ImageSurface::create_from_png(&mut Cursor::new(bytes))
                .expect("embedded Windows Live TV artwork");
            (name,image)
        }).collect();
        Self{art,material:Cell::new(0),hover:Cell::new(None),button_cache:RefCell::new(HashMap::new())}
    }
    pub fn set_hover(&self,action:Option<Action>){self.hover.set(action);}
    pub fn set_material(&self,index:usize){self.material.set(index.min(2));}
    fn art(&self,c:&Context,name:&str,x:f64,y:f64,w:f64,h:f64) {
        let im=&self.art[name];
        let _=c.save();
        c.translate(x,y);
        c.scale(w/im.width() as f64,h/im.height() as f64);
        let _=c.set_source_surface(im,0.,0.);
        let _=c.paint();
        let _=c.restore();
    }
    fn art_part(&self,c:&Context,name:&str,source:(f64,f64,f64,f64),dest:(f64,f64,f64,f64)) {
        let (sx,sy,sw,sh)=source;let (dx,dy,dw,dh)=dest;
        let _=c.save();c.rectangle(dx,dy,dw,dh);c.clip();
        c.translate(dx,dy);c.scale(dw/sw,dh/sh);
        let _=c.set_source_surface(&self.art[name],-sx,-sy);
        let _=c.paint();let _=c.restore();
    }
    fn tinted_art(&self,c:&Context,name:&str,x:f64,y:f64,w:f64,h:f64,r:f64,g:f64,b:f64){
        let im=&self.art[name];
        let _=c.save();c.translate(x,y);
        c.scale(w/im.width() as f64,h/im.height() as f64);
        Self::color(c,r,g,b);let _=c.mask_surface(im,0.,0.);
        let _=c.restore();
    }
    fn status_icon(&self,c:&Context,name:&str,x:f64,y:f64,size:f64,r:f64,g:f64,b:f64){
        Self::color(c,12.,8.,5.);
        Self::round_rect(c,x,y,size,size,5.);let _=c.fill();
        Self::color(c,38.,28.,19.);
        Self::round_rect(c,x+3.,y+3.,size-6.,size-6.,4.);let _=c.fill();
        Self::color(c,113.,87.,59.);c.set_line_width(1.);
        Self::round_rect(c,x+3.5,y+3.5,size-7.,size-7.,4.);let _=c.stroke();
        let inset=size/5.;
        self.tinted_art(c,name,x+inset,y+inset,size-2.*inset,size-2.*inset,r,g,b);
    }
    pub fn draw_caption_close(&self,c:&Context,width:i32,height:i32){
        let size=(width.min(height)) as f64;
        self.art(c,"dac-caption-screw",0.,0.,size,size);
        self.tinted_art(c,"dac-close",size*0.30,size*0.30,size*0.40,size*0.40,36.,26.,19.);
    }
    fn button_face(&self,c:&Context,x:f64,y:f64,w:f64,h:f64){
        let _=c.save();
        Self::round_rect(c,x+1.,y+1.,w-2.,h-3.,8.);
        c.clip();
        let material=self.material.get();
        let source=&self.art[match material{1=>"dac-glass",2=>"dac-plastic",_=>"dac-metal"}];
        let matrix=c.matrix();let (dx,dy)=c.target().device_scale();
        let pw=((w-2.)*matrix.xx().hypot(matrix.yx())*dx).round().max(1.) as i32;
        let ph=((h-3.)*matrix.xy().hypot(matrix.yy())*dy).round().max(1.) as i32;
        let dpi=(96.*dx).round().max(1.) as u32;
        let mut cache=self.button_cache.borrow_mut();
        if cache.len()>96{cache.clear();}
        let surface=cache.entry((material,pw,ph,dpi)).or_insert_with(||{
            let mut pixels=Vec::new();
            source.with_data(|bytes|{
                pixels=super::button_style::material_pixels(bytes,source.width(),source.height(),
                    source.stride() as usize,material,pw,ph,dpi);
            }).expect("embedded Windows button pixels");
            ImageSurface::create_for_data(pixels,gtk::cairo::Format::ARgb32,pw,ph,pw*4)
                .expect("button material surface")
        });
        c.translate(x+1.,y+1.);c.scale((w-2.)/pw as f64,(h-3.)/ph as f64);
        let _=c.set_source_surface(&*surface,0.,0.);let _=c.paint();
        let _=c.restore();
        Self::color(c,139.,113.,83.);c.set_line_width(1.);
        Self::round_rect(c,x+1.5,y+1.5,w-3.,h-4.,8.);let _=c.stroke();
    }
    pub fn draw_material_button(&self,c:&Context,width:i32,height:i32,focused:bool){
        self.button_face(c,0.,0.,width as f64,height as f64);
        if focused{
            Self::color(c,201.,171.,119.);c.set_line_width(2.);
            Self::round_rect(c,2.,2.,(width-4) as f64,(height-5) as f64,7.);let _=c.stroke();
        }
    }
    pub fn draw_settings_chrome(&self,c:&Context,width:i32,height:i32){
        self.draw_panel_chrome(c,width,height,"Settings");
    }
    pub fn draw_panel_chrome(&self,c:&Context,width:i32,height:i32,title:&str){
        let w=width as f64;let h=height as f64;
        c.set_operator(gtk::cairo::Operator::Source);
        c.set_source_rgba(0.,0.,0.,0.);let _=c.paint();
        c.set_operator(gtk::cairo::Operator::Over);
        let _=c.save();
        Self::round_rect(c,0.,0.,w,h,12.*w/750.);c.clip();
        let im=&self.art["settings-fascia"];let sw=im.width() as f64;let sh=im.height() as f64;
        let sx=[0.,64.,sw-64.,sw];let sy=[0.,64.,sh-64.,sh];
        let scale=w/750.;
        let d=(32.*scale).min(w/2.).min(h/2.);
        let dx=[0.,d,w-d,w];let dy=[0.,d,h-d,h];
        for row in 0..3 {for col in 0..3 {
            self.art_part(c,"settings-fascia",
                (sx[col],sy[row],sx[col+1]-sx[col],sy[row+1]-sy[row]),
                (dx[col],dy[row],dx[col+1]-dx[col],dy[row+1]-dy[row]));
        }}
        Self::text(c,title,38.*scale,32.*scale,16.*scale,true);
        let _=c.restore();
    }
    pub fn settings_tab_at(width:i32,x:f64,y:f64)->Option<usize>{
        let s=width as f64/750.;
        if y<44.*s||y>102.*s||x<14.*s||x>736.*s{return None;}
        Some((((x/s-14.)/(722./7.)).floor() as usize).min(6))
    }
    pub fn draw_settings_tabs(&self,c:&Context,width:i32,height:i32,current:usize){
        let s=width as f64/750.;
        let _=c.save();c.scale(s,s);
        let body=(14.,98.,722.,(height as f64/s-112.).max(1.));
        let cell=722./7.;
        let titles=["Video","Channels","Storage","Themes","Picture","Parental","General"];
        for (i,title) in titles.iter().enumerate(){
            if i==current{continue;}
            let x=14.+i as f64*cell+1.;
            let w=cell-2.;
            let tab_top=if matches!(i,2|5){51.}else{55.};
            let _=c.save();c.rectangle(x,tab_top,w,103.-tab_top);c.clip();
            Self::color(c,38.,28.,21.);
            Self::round_rect(c,x,tab_top,w,111.-tab_top,6.);let _=c.fill_preserve();
            Self::color(c,91.,68.,46.);c.set_line_width(1.);let _=c.stroke();
            Self::color(c,170.,151.,126.);
            c.select_font_face(ui_font(),FontSlant::Normal,FontWeight::Normal);
            c.set_font_size(if i18n::index()==0{14.}else{12.});
            let name=i18n::text(title);let ext=c.text_extents(&name).ok();
            let tw=ext.as_ref().map_or(0.,|e|e.width());
            c.move_to(x+(w-tw)/2.,82.);let _=c.show_text(&name);
            let _=c.restore();
        }
        let left=body.0+3.;let top_edge=body.1+3.;
        let right_edge=body.0+body.2-3.;let bottom=body.1+body.3-3.;
        let x=if current==0{left}else{body.0+current as f64*cell+1.};
        let right=if current==6{right_edge}else{body.0+(current+1) as f64*cell-1.};
        let sel_left=if current==0{left}else{x-7.};
        let sel_right=if current==6{right_edge}else{right+7.};
        let top=if matches!(current,2|5){44.}else{48.};let join=top_edge;let r=7.;
        Self::color(c,12.,8.,5.);
        Self::round_rect(c,body.0,body.1,body.2,body.3,7.);let _=c.fill();
        Self::color(c,28.,20.,14.);
        Self::round_rect(c,left,top_edge,right_edge-left,bottom-top_edge,5.);
        let _=c.fill();
        // Join the page outline to the selected tab. There is no seam beneath it.
        Self::color(c,113.,87.,59.);c.set_line_width(1.);
        c.new_path();
        c.move_to(left,bottom-5.);
        c.line_to(left,top_edge+5.);
        c.curve_to(left,top_edge+2.,left+2.,top_edge,left+5.,top_edge);
        if current>0{c.line_to(sel_left,top_edge);}
        let _=c.stroke();
        c.new_path();
        c.move_to(right_edge,bottom-5.);
        c.line_to(right_edge,top_edge+5.);
        c.curve_to(right_edge,top_edge+2.,right_edge-2.,top_edge,right_edge-5.,top_edge);
        if current<6{c.line_to(sel_right,top_edge);}
        let _=c.stroke();
        c.new_path();
        c.move_to(left,bottom-5.);
        c.curve_to(left,bottom-2.,left+2.,bottom,left+5.,bottom);
        c.line_to(right_edge-5.,bottom);
        c.curve_to(right_edge-2.,bottom,right_edge,bottom-2.,right_edge,bottom-5.);
        let _=c.stroke();
        Self::color(c,28.,20.,14.);
        c.new_path();
        if current==0{c.move_to(x,join+5.);c.line_to(x,top+r);}
        else{
            c.move_to(sel_left,join);
            c.curve_to(x-3.,join,x,join-4.,x,join-7.);
            c.line_to(x,top+r);
        }
        c.curve_to(x,top+3.,x+3.,top,x+r,top);
        c.line_to(right-r,top);
        c.curve_to(right-3.,top,right,top+3.,right,top+r);
        if current==6{c.line_to(right,join+5.);}
        else{
            c.line_to(right,join-7.);
            c.curve_to(right,join-3.,right+4.,join,sel_right,join);
        }
        c.line_to(sel_right,join+3.);
        c.line_to(sel_left,join+3.);c.close_path();
        let _=c.fill();
        Self::color(c,147.,112.,71.);c.set_line_width(1.);
        c.new_path();
        if current==0{c.move_to(x,join+5.);c.line_to(x,top+r);}
        else{
            c.move_to(sel_left,join);
            c.curve_to(x-3.,join,x,join-4.,x,join-7.);
            c.line_to(x,top+r);
        }
        c.curve_to(x,top+3.,x+3.,top,x+r,top);
        c.line_to(right-r,top);
        c.curve_to(right-3.,top,right,top+3.,right,top+r);
        if current==6{c.line_to(right,join+5.);}
        else{
            c.line_to(right,join-7.);
            c.curve_to(right,join-3.,right+4.,join,sel_right,join);
        }
        let _=c.stroke();
        Self::color(c,241.,212.,165.);
        c.select_font_face(ui_font(),FontSlant::Normal,FontWeight::Normal);
        c.set_font_size(if i18n::index()==0{15.}else{12.});
        let name=i18n::text(titles[current.min(6)]);
        let ext=c.text_extents(&name).ok();let tw=ext.as_ref().map_or(0.,|e|e.width());
        c.move_to(x+(right-x-tw)/2.,79.);let _=c.show_text(&name);
        let _=c.restore();
    }
    fn round_rect(c:&Context,x:f64,y:f64,w:f64,h:f64,r:f64){
        let r=r.min(w/2.).min(h/2.).max(0.);
        c.new_sub_path();c.arc(x+w-r,y+r,r,-std::f64::consts::FRAC_PI_2,0.);
        c.arc(x+w-r,y+h-r,r,0.,std::f64::consts::FRAC_PI_2);
        c.arc(x+r,y+h-r,r,std::f64::consts::FRAC_PI_2,std::f64::consts::PI);
        c.arc(x+r,y+r,r,std::f64::consts::PI,3.*std::f64::consts::FRAC_PI_2);
        c.close_path();
    }
    fn viewer_buttons()->Vec<(Action,f64,f64)> {
        let mut buttons=BUTTONS.to_vec();
        let surface=ImageSurface::create(gtk::cairo::Format::ARgb32,1,1).unwrap();
        let c=Context::new(&surface).unwrap();
        c.select_font_face(ui_font(),FontSlant::Normal,FontWeight::Normal);
        c.set_font_size(48.*BUTTON_FONT_RATIO);
        let required:Vec<_>=[("",false),("EPG",true),("",false),("AUDIO",false),("",false),("SETTINGS",true)]
            .into_iter().map(|(label,icon)|{
                let text=c.text_extents(&i18n::text(label).to_uppercase()).map(|e|e.width()).unwrap_or(0.);
                (text+16.+if icon{48.*0.36+48./7.}else{0.}).max(52.)
            }).collect();
        let original:Vec<_>=buttons[9..].iter().map(|b|b.2).collect();
        if required.iter().zip(&original).any(|(need,width)|need>width) {
            let widths=super::button_style::row_widths(&required,&original,487.);
            let mut x=1030.;
            for (button,width) in buttons[9..].iter_mut().zip(widths) {
                button.1=x;button.2=width;x+=width+7.;
            }
        }
        buttons
    }
    fn dac_top()->Vec<(Action,f64,f64)>{
        let specs=[(Action::Previous,"CH -",80.),(Action::Next,"CH +",80.),
            (Action::Back,"",68.),(Action::Forward,"",68.),(Action::Play,"PLAY",120.),
            (Action::Stop,"STOP",126.),(Action::Record,"REC",180.),(Action::Fullscreen,"FULLSCREEN",249.)];
        let surface=ImageSurface::create(gtk::cairo::Format::ARgb32,1,1).unwrap();let c=Context::new(&surface).unwrap();
        c.select_font_face(ui_font(),FontSlant::Normal,FontWeight::Normal);c.set_font_size(46.*BUTTON_FONT_RATIO);
        let measure=|label:&str|c.text_extents(&i18n::text(label).to_uppercase()).map(|e|e.width()).unwrap_or(0.);
        let required:Vec<f64>=specs.iter().map(|(a,label,_)|{
            let text=if *a==Action::Play{measure(label).max(measure("PAUSE"))}else if *a==Action::Record{measure(label).max(measure("REC ON"))}else{measure(label)};
            (text+46.*0.36+46./7.+24.).max(if matches!(a,Action::Previous|Action::Next){80.}else{52.})
        }).collect();
        if required.iter().zip(specs).all(|(need,(_,_,w))|*need<=w){
            return specs.into_iter().zip([61.,149.,253.,329.,413.,541.,691.,883.]).map(|((a,_,w),x)|(a,x,w)).collect();
        }
        let original:Vec<f64>=specs.iter().map(|s|s.2).collect();let widths=super::button_style::row_widths(&required,&original,1071.-7.*8.);
        let mut x=61.;specs.into_iter().zip(widths).map(|((a,_,_),w)|{let b=(a,x,w);x+=w+8.;b}).collect()
    }
    fn dac_bottom()->Vec<(Action,f64,f64)>{
        let specs=[(Action::Guide,"GUIDE",142.),(Action::Snapshot,"SNAPSHOT",142.),
            (Action::Audio,"AUDIO",139.),(Action::Captions,"CC OFF",139.),
            (Action::Library,"LIBRARY",139.),(Action::Open,"OPEN",139.),(Action::Settings,"SETTINGS",123.)];
        let surface=ImageSurface::create(gtk::cairo::Format::ARgb32,1,1).unwrap();
        let c=Context::new(&surface).unwrap();c.select_font_face(ui_font(),FontSlant::Normal,FontWeight::Normal);c.set_font_size(46.*BUTTON_FONT_RATIO);
        let measure=|label:&str|c.text_extents(&i18n::text(label).to_uppercase()).map(|e|e.width()).unwrap_or(0.);
        let required:Vec<f64>=specs.iter().map(|(a,label,_)|{
            let text=if *a==Action::Captions{measure(label).max(measure("CC ON"))}else{measure(label)};
            text+46.*0.36+46./7.+if *a==Action::Settings{36.}else{24.}
        }).collect();
        if required.iter().zip(specs).all(|(minimum,(_,_,width))|*minimum<=width){
            return specs.into_iter().zip([61.,211.,381.,528.,695.,842.,1009.]).map(|((a,_,w),x)|(a,x,w)).collect();
        }
        let available=1071.-6.*8.;let minimum:f64=required.iter().sum();
        let _=minimum;let original:Vec<f64>=specs.iter().map(|s|s.2).collect();
        let widths=super::button_style::row_widths(&required,&original,available);
        let mut x=61.;specs.into_iter().zip(widths).map(|((action,_,_),width)|{let result=(action,x,width);x+=width+8.;result
        }).collect()
    }
    #[cfg(test)]pub fn verify_translated_buttons(){
        let surface=ImageSurface::create(gtk::cairo::Format::ARgb32,1,1).unwrap();let c=Context::new(&surface).unwrap();
        c.select_font_face(ui_font(),FontSlant::Normal,FontWeight::Normal);c.set_font_size(46.*BUTTON_FONT_RATIO);
        let mut right=0.;
        for ((action,x,width),labels) in Self::dac_top().into_iter().zip([
            &["CH -"][..],&["CH +"][..],&[""][..],&[""][..],&["PLAY","PAUSE"][..],
            &["STOP"][..],&["REC","REC ON"][..],&["FULLSCREEN"][..]]) {
            assert!(x>=right&&x+width<=1133.,"DAC top row overflow");
            for label in labels {
                let text=c.text_extents(&i18n::text(label).to_uppercase()).unwrap().width();
                assert!(text+46.*0.36+46./7.+16.<=width,"Top caption does not fit: {label}");
            }
            assert_eq!(Self::dac_hit(1679,547,x+width/2.,355.),Some(action));right=x+width;
        }
        c.set_font_size(48.*BUTTON_FONT_RATIO);
        right=0.;
        for ((action,x,width),(label,icon)) in Self::viewer_buttons().into_iter().skip(9).zip([
            ("",false),("EPG",true),("",false),("AUDIO",false),("",false),("SETTINGS",true)]) {
            let text=c.text_extents(&i18n::text(label).to_uppercase()).unwrap().width();
            assert!(x>=right&&x+width<=1553.,"Viewer buttons overlap or overflow");
            assert!(text+16.+(if icon{48.*0.36+48./7.}else{0.})<=width,"Viewer caption does not fit: {label}");
            assert_eq!(Self::hit(1600,1100,x+width/2.,1035.),Some(action));right=x+width;
        }
        c.set_font_size(46.*BUTTON_FONT_RATIO);
        let buttons=Self::dac_bottom();let mut right=0.;
        for ((action,x,width),label) in buttons.into_iter().zip(["GUIDE","SNAPSHOT","AUDIO","CC OFF","LIBRARY","OPEN","SETTINGS"]){
            let text=c.text_extents(&i18n::text(label).to_uppercase()).unwrap().width();
            assert!(x>=right&&x+width<=1133.,"Button bounds overlap or overflow: {label} at {x} width {width}");
            assert!(text+46.*0.36+46./7.+16.<=width,"Full translation does not fit: {label}");
            assert_eq!(Self::dac_hit(1679,547,x+width/2.,452.),Some(action));right=x+width;
        }
    }
    pub fn dac_hit(width:i32,height:i32,x:f64,y:f64)->Option<Action>{
        let sx=width as f64/1679.;let sy=height as f64/547.;
        let x=x/sx;let y=y/sy;
        if (84.0..748.0).contains(&x)&&(104.0..212.0).contains(&y){return Some(Action::Channels);}
        if (1545.0..1587.0).contains(&x)&&(24.0..60.0).contains(&y){return Some(Action::Minimize);}
        if (1583.0..1625.0).contains(&x)&&(24.0..60.0).contains(&y){return Some(Action::Close);}
        let mut buttons:Vec<_>=Self::dac_top().into_iter().map(|(a,x,w)|(a,x,332.,w)).collect();
        buttons.extend(Self::dac_bottom().into_iter().map(|(a,x,w)|(a,x,429.,w)));
        buttons.into_iter().find(|(_,bx,by,bw)|x>=*bx&&x<*bx+*bw&&y>=*by&&y<*by+46.)
            .map(|(action,_,_,_)|action)
    }
    pub fn dac_dial_angle(width:i32,height:i32,x:f64,y:f64,bounded:bool)->Option<f64>{
        let dx=x*1679./width as f64-1425.;let dy=y*547./height as f64-265.;
        if dx*dx+dy*dy<16. || (bounded && dx*dx+dy*dy>145.*145.){return None;}
        Some(dy.atan2(dx))
    }
    pub fn draw_deck(&self,c:&Context,width:i32,height:i32,channel:&str,channel_number:usize,
        status:&str,playing:bool,recording:bool,volume:i32,paused:bool,captions:bool,audio_mode:usize,quality:Option<u8>){
        let sx=width as f64/1679.;let sy=height as f64/547.;
        c.set_operator(gtk::cairo::Operator::Source);
        c.set_source_rgba(0.,0.,0.,0.);let _=c.paint();
        c.set_operator(gtk::cairo::Operator::Over);
        let _=c.save();
        Self::round_rect(c,0.,0.,width as f64,height as f64,
            18.*width as f64/1679.);c.clip();
        self.art(c,"orbit-fascia",0.,0.,width as f64,height as f64);
        let _=c.save();c.scale(sx,sy);
        self.art(c,"module",1229.,85.,393.,360.);
        let cx=1425.;let cy=265.;
        let angle=((volume.clamp(0,100) as f64-65.)*1.8).to_radians();
        let _=c.save();c.translate(cx,cy);c.rotate(angle);
        self.art(c,"pointer",-93.,-93.,186.,186.);let _=c.restore();
        Self::text(c,"Live TV!",66.,60.,29.,true);
        Self::color(c,20.,14.,10.);Self::round_rect(c,61.,85.,1072.,185.,22.);
        let _=c.fill_preserve();Self::color(c,100.,77.,57.);c.set_line_width(1.);let _=c.stroke();
        Self::color(c,38.,27.,19.);Self::round_rect(c,84.,104.,664.,108.,16.);
        let _=c.fill_preserve();Self::color(c,98.,72.,49.);let _=c.stroke();
        Self::text(c,"CHANNEL",100.,128.,14.,false);
        Self::text(c,&format!("{:02}",channel_number),100.,188.,44.,true);
        Self::color(c,97.,70.,50.);c.rectangle(181.,121.,1.,75.);let _=c.fill();
        let title:String=channel.chars().take(30).collect();
        Self::text(c,&title,207.,151.,26.,true);
        self.tinted_art(c,"dac-signal",207.,181.,19.,19.,183.,164.,116.);
        Self::text(c,&quality.map(|value|format!("SIGNAL QUALITY: {value}%")).unwrap_or_else(||"SIGNAL QUALITY: —".into()),233.,196.,15.,false);
        self.tinted_art(c,"dac-channel-face",685.,132.,50.,53.,58.,42.,29.);
        self.tinted_art(c,"dac-channel-frame",685.,132.,50.,53.,19.,13.,9.);
        self.tinted_art(c,"dac-channel-chevron",697.,145.,26.,27.,224.,195.,145.);
        Self::color(c,38.,28.,19.);Self::round_rect(c,838.,105.,180.,48.,9.);
        let _=c.fill_preserve();Self::color(c,94.,69.,45.);let _=c.stroke();
        let clock=gtk::glib::DateTime::now_local().ok()
            .and_then(|t|t.format("%H:%M:%S").ok()).map(|s|s.to_string())
            .unwrap_or_else(||"--:--:--".into());
        Self::centered(c,&clock,928.,129.,28.,false);
        self.status_icon(c,"dac-live",838.,171.,36.,
            if playing{103.}else{94.},if playing{194.}else{121.},if playing{120.}else{89.});
        Self::text(c,if recording{"RECORDING"}else if playing{"LIVE"}else{"READY"},885.,194.,16.,true);
        self.status_icon(c,match audio_mode{1..=3=>"dac-mono",4=>"dac-surround",_=>"dac-stereo"},84.,228.,32.,201.,181.,153.);
        Self::text(c,["STEREO","MONO","LEFT CHANNEL","RIGHT CHANNEL","SURROUND"][audio_mode.min(4)],128.,250.,16.,false);
        let line=Self::visible_status(status).chars().take(70).collect::<String>();
        Self::text(c,&line,61.,291.,13.,false);
        Self::color(c,68.,48.,34.);c.rectangle(61.,374.,1072.,1.);let _=c.fill();
        let mut buttons:Vec<_>=Self::dac_top().into_iter().zip([
            ("CH -",""),("CH +",""),("","dac-seek-back"),("","dac-seek-forward"),
            (if playing&&!paused{"PAUSE"}else{"PLAY"},if playing&&!paused{"pause"}else{"dac-play"}),
            ("STOP","dac-stop"),(if recording{"REC ON"}else{"REC"},"dac-record"),("FULLSCREEN","dac-fullscreen")
        ]).map(|((_,x,w),(label,icon))|(x,332.,w,label,icon)).collect();
        for ((_,x,w),(label,icon)) in Self::dac_bottom().into_iter().zip([
            ("GUIDE","dac-guide"),("SNAPSHOT","dac-snapshot"),("AUDIO","dac-audio"),
            (if captions{"CC ON"}else{"CC OFF"},"dac-captions"),("LIBRARY","dac-library"),
            ("OPEN","dac-open"),("SETTINGS","dac-settings")]){buttons.push((x,429.,w,label,icon));}
        let ink=if self.material.get()==1{(233.,223.,206.)}else{(40.,38.,33.)};
        for (x,y,w,label,icon) in buttons {
            let h=46.;
            self.button_face(c,x,y,w,h);
            let action=match icon{"dac-record"=>Some(Action::Record),"dac-captions"=>Some(Action::Captions),_=>None};
            let accented=action.is_some()&&self.hover.get()==action
                || action==Some(Action::Record)&&recording
                || action==Some(Action::Captions)&&captions;
            let ink=if accented{
                let edge=if action==Some(Action::Record){super::button_style::RECORD_EDGE}else{super::button_style::CAPTION_EDGE};
                Self::color(c,edge.0 as f64,edge.1 as f64,edge.2 as f64);c.set_line_width(3.);
                Self::round_rect(c,x+2.,y+2.,w-4.,h-4.,5.);let _=c.stroke();
                if action==Some(Action::Record){if self.material.get()==1{(217.,106.,85.)}else{(152.,47.,37.)}}
                else if self.material.get()==1{(197.,163.,55.)}else{(145.,109.,6.)}
            }else{ink};
            match label {
                "CH -"|"CH +"=>{
                    Self::text_colored(c,"CH",x+22.,y+28.,15.,true,ink);
                    Self::color(c,ink.0,ink.1,ink.2);
                    c.set_line_width(2.5);
                    c.move_to(x+56.,y+23.);c.line_to(x+67.,y+23.);let _=c.stroke();
                    if label=="CH +" {
                        c.move_to(x+61.5,y+17.5);c.line_to(x+61.5,y+28.5);let _=c.stroke();
                    }
                }
                "<<"|">>"=>{
                    Self::color(c,ink.0,ink.1,ink.2);
                    for offset in [0.,10.] {
                        let cx=x+w/2.+offset-5.;
                        if label=="<<" {
                            c.move_to(cx+5.,y+15.);c.line_to(cx-3.,y+23.);c.line_to(cx+5.,y+31.);
                        }else{
                            c.move_to(cx-5.,y+15.);c.line_to(cx+3.,y+23.);c.line_to(cx-5.,y+31.);
                        }
                        c.close_path();let _=c.fill();
                    }
                }
                _=>self.button_contents(c,x,y,w,h,label,icon,ink),
            }
        }
        for (x,name) in [(1548.,"dac-minimize"),(1586.,"dac-close")] {
            self.art(c,"dac-caption-screw",x,24.,36.,36.);
            self.tinted_art(c,name,x+10.,34.,16.,16.,36.,26.,19.);
        }
        let _=c.restore();
        let _=c.restore();
    }
    fn chassis_band(&self,c:&Context,width:f64,y:f64,h:f64,source_y:f64,source_h:f64) {
        let _=c.save();
        c.rectangle(0.,y,width,h);
        c.clip();
        c.translate(0.,y);
        c.scale(width/1600.,h/source_h);
        let _=c.set_source_surface(&self.art["chassis"],0.,-source_y);
        let _=c.paint();
        let _=c.restore();
    }
    fn color(c:&Context,r:f64,g:f64,b:f64) {c.set_source_rgb(r/255.,g/255.,b/255.);}
    fn button_contents(&self,c:&Context,x:f64,y:f64,w:f64,h:f64,label:&str,icon:&str,ink:(f64,f64,f64)){
        let text_h=h.min(48.);
        let translated=i18n::text(label).to_uppercase();
        let icon_size=if icon.is_empty(){0.}else{text_h*0.36};
        let gap=if icon.is_empty()||translated.is_empty(){0.}else{text_h/7.};
        let available=(w-16.-icon_size-gap).max(1.);
        c.select_font_face(ui_font(),FontSlant::Normal,FontWeight::Normal);
        let base=text_h*BUTTON_FONT_RATIO;c.set_font_size(base);
        let measure=|text:&str|c.text_extents(text).map(|e|e.width()).unwrap_or(0.);
        let mut lines=vec![translated.clone()];
        if measure(&translated)>available/0.8{
            let words:Vec<_>=translated.split_whitespace().collect();
            if words.len()>1{
                let split=(1..words.len()).min_by(|&a,&b|{
                    let score=|i|measure(&words[..i].join(" ")).max(measure(&words[i..].join(" ")));
                    score(a).total_cmp(&score(b))
                }).unwrap();
                lines=vec![words[..split].join(" "),words[split..].join(" ")];
            }
        }
        let widest=lines.iter().map(|line|measure(line)).fold(0f64,f64::max);
        let size=base*(available/widest.max(available));c.set_font_size(size);
        let text_width=lines.iter().map(|line|measure(line)).fold(0f64,f64::max);
        let left=x+(w-icon_size-gap-text_width)/2.;
        let _=c.save();c.rectangle(x+4.,y+2.,w-8.,h-4.);c.clip();
        if !icon.is_empty(){
            let color=if icon=="dac-record"{if self.material.get()==1{(217.,106.,85.)}else{(152.,47.,37.)}}else{ink};
            self.tinted_art(c,icon,left,y+(h-icon_size)/2.,icon_size,icon_size,color.0,color.1,color.2);
        }
        Self::color(c,ink.0,ink.1,ink.2);
        for (index,line) in lines.iter().enumerate(){
            if let Ok(ext)=c.text_extents(line){
                let center=y+h/2.+(index as f64-(lines.len()-1) as f64/2.)*(size+2.);
                c.move_to(left+icon_size+gap+(text_width-ext.width())/2.-ext.x_bearing(),center-ext.y_bearing()-ext.height()/2.);
                let _=c.show_text(line);
            }
        }
        let _=c.restore();
    }
    fn text(c:&Context,text:&str,x:f64,y:f64,size:f64,bold:bool) {
        Self::text_colored(c,text,x,y,size,bold,(233.,223.,206.));
    }
    fn text_colored(c:&Context,text:&str,x:f64,y:f64,size:f64,bold:bool,
        ink:(f64,f64,f64)) {
        c.select_font_face(ui_font(),FontSlant::Normal,if bold{FontWeight::Bold}else{FontWeight::Normal});
        c.set_font_size(size);
        Self::color(c,ink.0,ink.1,ink.2);
        c.move_to(x,y);
        let _=c.show_text(&i18n::text(text));
    }
    fn centered(c:&Context,text:&str,x:f64,y:f64,size:f64,bold:bool) {
        Self::centered_colored(c,text,x,y,size,bold,(233.,223.,206.));
    }
    fn centered_colored(c:&Context,text:&str,x:f64,y:f64,size:f64,bold:bool,
        ink:(f64,f64,f64)) {
        c.select_font_face(ui_font(),FontSlant::Normal,if bold{FontWeight::Bold}else{FontWeight::Normal});
        c.set_font_size(size);
        let translated=i18n::text(text);
        let ext=c.text_extents(&translated).ok();
        let tw=ext.as_ref().map_or(0.,|e|e.width());
        let th=ext.as_ref().map_or(0.,|e|e.height());
        Self::text_colored(c,text,x-tw/2.,y+th/2.,size,bold,ink);
    }
    pub fn video_rect(width:i32,height:i32)->(i32,i32,i32,i32) {
        let s=width as f64/1600.;
        let left=(28.*s).round() as i32;
        let top=(62.*s).round() as i32;
        let right=width-left;
        let bottom=height-(169.5*s).round() as i32;
        (left,top,(right-left).max(1),(bottom-top).max(1))
    }
    pub fn audio_rect(width:i32,height:i32,deck:bool)->(i32,i32,i32,i32){
        if deck{
            let (_,x,w)=Self::dac_bottom().into_iter().find(|(a,_,_)|*a==Action::Audio).unwrap();
            let sx=width as f64/1679.;let sy=height as f64/547.;
            ((x*sx).round() as i32,(429.*sy).round() as i32,(w*sx).round() as i32,(46.*sy).round() as i32)
        }else{
            let (_,x,w)=Self::viewer_buttons().into_iter().find(|(a,_,_)|*a==Action::Audio).unwrap();let scale=width as f64/1600.;
            ((x*scale).round() as i32,(height as f64-89.*scale).round() as i32,(w*scale).round() as i32,(48.*scale).round() as i32)
        }
    }
    pub fn seek_fraction(width:i32,height:i32,x:f64,y:f64)->Option<f64>{
        let s=width as f64/1600.;
        let top=height as f64-(1100.-948.)*s;
        if x<40.*s||x>1284.*s||y<top||y>top+27.*s{return None;}
        Some(((x/s-47.)/1230.).clamp(0.,1.))
    }
    pub fn hit(width:i32,height:i32,x:f64,y:f64)->Option<Action> {
        let s=width as f64/1600.;
        let bottom=|bx:f64,by:f64,bw:f64,bh:f64|{
            let left=bx*s;let top=height as f64-(1100.-by)*s;
            x>=left&&x<left+bw*s&&y>=top&&y<top+bh*s
        };
        for (action,bx,bw) in Self::viewer_buttons() {
            if bottom(bx,1011.,bw,48.) {return Some(action);}
        }
        if bottom(1430.,948.,122.,27.){return Some(Action::Live);}
        for &(action,bx) in &[(Action::Minimize,1468.),(Action::Maximize,1506.),(Action::Close,1544.)] {
            if x>=bx*s&&x<(bx+34.)*s&&y>=12.*s&&y<44.*s{return Some(action);}
        }
        None
    }
    pub fn draw(&self,c:&Context,width:i32,height:i32,state:DrawState<'_>) {
        if width<=0||height<=0{return;}
        c.set_operator(gtk::cairo::Operator::Source);
        c.set_source_rgba(0.,0.,0.,0.);let _=c.paint();
        c.set_operator(gtk::cairo::Operator::Over);
        let w=width as f64;let h=height as f64;let s=w/1600.;
        let top=100.*s;let bottom=200.*s;
        self.chassis_band(c,w,0.,top,0.,100.);
        self.chassis_band(c,w,top,(h-top-bottom).max(0.),100.,800.);
        self.chassis_band(c,w,(h-bottom).max(0.),bottom,900.,200.);
        let _=c.save();
        c.scale(s,s);
        Self::text(c,"Live TV!",35.,36.,26.,true);
        let by=|y:f64| h/s-(1100.-y);
        Self::centered_colored(c,"REC",421.,by(1000.),13.,true,
            if state.recording{(217.,106.,85.)}else{(233.,223.,206.)});
        Self::text(c,"CHANNEL",520.,by(1000.),13.,true);
        Self::text(c,"VOLUME",772.,by(1000.),13.,true);
        for (x,bw) in [(40.,1244.),(1292.,116.)] {
            Self::color(c,16.,12.,10.);
            c.rectangle(x,by(948.),bw,27.);let _=c.fill_preserve();
            Self::color(c,82.,64.,47.);c.set_line_width(1.);let _=c.stroke();
        }
        Self::color(c,190.,167.,132.);c.rectangle(47.,by(960.),1230.,3.);let _=c.fill();
        let fraction=state.timeline_fraction();
        let thumb=47.+1230.*fraction;
        Self::color(c,205.,172.,117.);c.rectangle(thumb-7.,by(948.),14.,27.);let _=c.fill();
        let position=if let Some(f)=state.drag{f*state.duration}else{state.position};
        let seconds=if position.is_finite(){position.max(0.) as u64}else{0};
        let time=if state.playing{format!("{:02}:{:02}:{:02}",seconds/3600,seconds/60%60,seconds%60)}else{"—".to_owned()};
        Self::centered(c,&time,1350.,by(961.5),15.,true);
        let ink=if self.material.get()==1{(233.,223.,206.)}else{(40.,38.,33.)};
        for (action,x,bw) in Self::viewer_buttons() {
            let y=by(1011.);
            if action!=Action::Record {
                self.button_face(c,x,y,bw,48.);
            }
            let (icon,label)=match action {
                Action::Back=>("rewind",""),Action::Forward=>("fast-forward",""),
                Action::Play=>(if state.playing&&!state.paused{"pause"}else{"play"},""),
                Action::Stop=>("stop",""),
                Action::Record=>(if state.recording{"mounted-indicator-on"}else{"mounted-indicator-off"},""),
                Action::Previous|Action::VolumeDown=>("","−"),
                Action::Next|Action::VolumeUp=>("",""),
                Action::Snapshot=>("snapshot",""),
                Action::Guide=>("guide","EPG"),
                Action::Fullscreen=>("fullscreen",""),
                Action::Audio=>("","AUDIO"),
                Action::Deck=>("",""),
                Action::Settings=>("settings","SETTINGS"),
                _=>("",""),
            };
            if action==Action::Record {self.art(c,icon,x,y,48.,48.);}
            else if !icon.is_empty()||!label.is_empty(){
                if !matches!(action,Action::Previous|Action::VolumeDown){self.button_contents(c,x,y,bw,48.,label,icon,ink);}
            }
            if matches!(action,Action::Previous|Action::Next|Action::VolumeDown|Action::VolumeUp) {
                let cx=x+bw/2.;let cy=y+24.;
                Self::color(c,ink.0,ink.1,ink.2);c.set_line_width(2.5);
                c.move_to(cx-9.,cy);c.line_to(cx+9.,cy);let _=c.stroke();
                if matches!(action,Action::Next|Action::VolumeUp) {
                    c.move_to(cx,cy-9.);c.line_to(cx,cy+9.);let _=c.stroke();
                }
            }
            if action==Action::Deck {
                let qx=x+bw/2.-13.;let qy=y+11.;
                Self::color(c,ink.0,ink.1,ink.2);c.set_line_width(1.5);
                Self::round_rect(c,qx,qy+4.,26.,18.,3.);let _=c.stroke();
                c.rectangle(qx+4.,qy+8.,11.,10.);let _=c.stroke();
                c.arc(qx+20.5,qy+10.5,2.5,0.,std::f64::consts::TAU);let _=c.stroke();
            }

        }
        self.art(c,if state.live{"live-green-on"}else{"live-green-off"},1433.,by(953.),16.,16.);
        Self::text(c,&i18n::text("Live").to_uppercase(),1461.,by(967.),14.,true);
        // Match the native Windows 35%-of-control caption glyphs.
        Self::color(c,240.,226.,203.);c.set_line_width(1.5);
        let size=11.;let cy=28.;
        c.move_to(1485.-size/2.,cy);c.line_to(1485.+size/2.,cy);let _=c.stroke();
        c.rectangle(1523.-size/2.,cy-size/2.,size,size);let _=c.stroke();
        for (x1,y1,x2,y2) in [(1561.-size/2.,cy-size/2.,1561.+size/2.,cy+size/2.),
            (1561.+size/2.,cy-size/2.,1561.-size/2.,cy+size/2.)]{
            c.move_to(x1,y1);c.line_to(x2,y2);let _=c.stroke();
        }
        let status:String=Self::visible_status(state.status).chars().take(160).collect();
        Self::text(c,&status,48.,by(1084.),13.,false);
        let _=c.restore();
    }
    fn visible_status(status:&str)->&str {
        if status.starts_with("Live TV! ·"){""}else{status}
    }
}

#[cfg(test)]mod recording_timeline_tests{
    use super::*;
    #[test]fn live_thumb_ignores_decoder_latency_but_respects_rewind_and_drag(){
        let mut s=DrawState{playing:true,recording:true,status:"",paused:false,live:true,
            following_live:true,position:7.,duration:10.,seekable:true,drag:None};
        assert_eq!(s.timeline_fraction(),1.);
        s.duration=30.;s.position=26.;assert_eq!(s.timeline_fraction(),1.);
        s.drag=Some(0.3);assert_eq!(s.timeline_fraction(),0.3);s.drag=None;
        s.following_live=false;s.position=9.;assert_eq!(s.timeline_fraction(),0.3);
        s.following_live=true;s.paused=true;assert_eq!(s.timeline_fraction(),0.3);
        s.following_live=false;s.recording=false;assert_eq!(s.timeline_fraction(),0.3);
    }
}
