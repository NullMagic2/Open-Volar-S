
use super::*;
pub fn dispatch(action:Action,state:Rc<RefCell<State>>,window:&ApplicationWindow){
    match action{
        Action::Settings=>settings_dialog(state,window),
        Action::Deck=>receiver_dialog(state,window),
        Action::Guide=>guide_dialog(state,window),
        Action::Channels=>channel_menu(state),
        Action::Open|Action::Library=>{
            let owner=RECEIVER_WINDOW.with(|slot|slot.borrow().clone())
                .unwrap_or_else(||window.clone().upcast());
            let dialog=gtk::FileChooserDialog::new(Some(&i18n::text("Open recording")),Some(&owner),gtk::FileChooserAction::Open);
            dialog.set_position(gtk::WindowPosition::CenterOnParent);
            dialog.add_buttons(&[(&i18n::text("Cancel"),ResponseType::Cancel),(&i18n::text("Open"),ResponseType::Accept)]);
            dialog.set_default_size(900,640);
            let _=dialog.set_current_folder(&state.borrow().recording_folder);
            let filter=gtk::FileFilter::new();filter.set_name(Some(&i18n::text("Recordings")));
            for pattern in ["*.ts","*.mts","*.m2ts","*.mp4","*.mkv","*.avi"]{filter.add_pattern(pattern);}
            dialog.add_filter(filter);
            let all=gtk::FileFilter::new();all.set_name(Some(&i18n::text("All files")));all.add_pattern("*");dialog.add_filter(all);
            if dialog.run()==ResponseType::Accept{if let Some(path)=dialog.filename(){state.borrow_mut().open_file(path);}}
            dialog.close();
        },
        Action::Audio=>audio_menu(state,window,false),
        _=>state.borrow_mut().handle(action,window),
    }
    window.queue_draw();
    RECEIVER_WINDOW.with(|slot|{if let Some(panel)=slot.borrow().as_ref(){panel.queue_draw();}});
}
fn channel_menu(state:Rc<RefCell<State>>){
    let Some(owner)=RECEIVER_WINDOW.with(|slot|slot.borrow().clone()) else{return;};
    let Some(surface)=owner.window() else{return;};
    let size=owner.allocation();
    // Windows orbit::rect(84,104,664,108): anchor the list to the whole field.
    let sx=size.width() as f64/1679.;let sy=size.height() as f64/547.;
    let anchor=gdk::Rectangle::new((84.*sx).round() as i32,(104.*sy).round() as i32,
        (664.*sx).round() as i32,(108.*sy).round() as i32);
    let row_height=(30.*sy).round().max(22.) as i32;
    let menu=gtk::Menu::new();style_dropdown(&menu,anchor.width(),row_height);
    let s=state.borrow();let mut selected=None;
    for (index,service) in s.services.iter().enumerate(){
        let number=interaction::channel_number(Some(service),index);
        let name=service["name"].as_str().unwrap_or("TV");
        let item=gtk::MenuItem::with_label(&format!("{number} – {name}"));
        if let Some(label)=item.child().and_then(|w|w.downcast::<gtk::Label>().ok()){
            label.set_xalign(0.);label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            label.set_width_chars(1);label.set_max_width_chars(1);
        }
        if index==s.service_index{selected=Some(item.clone());}
        let st=state.clone();item.connect_activate(move |_|st.borrow_mut().select_channel(index));menu.append(&item);
    }
    if s.services.is_empty(){
        let item=gtk::MenuItem::with_label(&i18n::text("Scan in Settings to find TV channels."));
        item.set_sensitive(false);menu.append(&item);
    }
    drop(s);style_dropdown(&menu,anchor.width(),row_height);menu.show_all();
    menu.popup_at_rect(&surface,&anchor,gdk::Gravity::SouthWest,gdk::Gravity::NorthWest,None::<&gdk::Event>);
    if let Some(item)=selected{menu.select_item(&item);}
}
pub fn install_dropdown_css(){
    let css=gtk::CssProvider::new();
    css.load_from_data(b"menu.orbit-dropdown {background:#231912;color:#efdcb9;border:1px solid #6c5339;border-radius:0;padding:0;margin:0;box-shadow:none;} \
        menu.orbit-dropdown menuitem {background:#231912;color:#efdcb9;border:0;border-radius:0;padding:0 12px;margin:0;min-height:22px;box-shadow:none;} \
        menu.orbit-dropdown menuitem:hover,menu.orbit-dropdown menuitem:selected {background:#4c3421;color:#efdcb9;} \
        menu.orbit-dropdown label,menu.orbit-dropdown cellview {background:transparent;font-family:Selawik;font-size:15px;font-weight:normal;color:inherit;padding:0;margin:0;text-shadow:none;} \
        menu.orbit-dropdown arrow {background:#1b140f;color:#efdcb9;border:0;box-shadow:none;}").expect("dropdown skin CSS");
    if let Some(screen)=gdk::Screen::default(){gtk::StyleContext::add_provider_for_screen(&screen,&css,gtk::STYLE_PROVIDER_PRIORITY_APPLICATION+5);}
}
fn style_dropdown(menu:&gtk::Menu,width:i32,row_height:i32){
    menu.style_context().add_class("orbit-dropdown");menu.set_reserve_toggle_size(false);
    menu.set_size_request(width,-1);menu.set_anchor_hints(gdk::AnchorHints::FLIP_Y|gdk::AnchorHints::SLIDE_X|gdk::AnchorHints::RESIZE_Y);
    for item in menu.children(){item.set_size_request(-1,row_height);}
}
pub fn style_combo(combo:&ComboBoxText){
    combo.set_popup_fixed_width(true);
    combo.connect_popup_shown_notify(|combo|{
        if !combo.is_popup_shown(){return;}
        // GTK owns the popup's pointer grab and button release. Replacing the menu
        // here lets that same release close the replacement under WSLg.
        for widget in gtk::Menu::for_attach_widget(combo){
            if let Ok(menu)=widget.downcast::<gtk::Menu>(){
                style_dropdown(&menu,combo.allocation().width(),30);
            }
        }
    });
}
pub fn audio_menu(state:Rc<RefCell<State>>,viewer:&ApplicationWindow,from_deck:bool){
    let receiver=RECEIVER_WINDOW.with(|slot|slot.borrow().clone());
    let owner=if from_deck{receiver.unwrap()}else{viewer.clone().upcast()};
    let Some(surface)=owner.window() else{return;};let size=owner.allocation();
    let (x,y,w,h)=Skin::audio_rect(size.width(),size.height(),from_deck);
    let anchor=gdk::Rectangle::new(x,y,w,h);
    let menu=gtk::Menu::new();let mut selected=None;
    for (index,label) in [(0,"Stereo"),(1,"Mono"),(2,"Left channel"),(3,"Right channel"),(4,"5.1 surround")]{
        let item=gtk::MenuItem::with_label(&i18n::text(label));
        if index==state.borrow().audio_mode{selected=Some(item.clone());}
        let st=state.clone();item.connect_activate(move |_|{
            let mut s=st.borrow_mut();
            if s.ipc.exists(){
                if let Err(error)=s.player_request(if a865r_media::wsl_video::is_wsl(){serde_json::json!({"command":["af","set",audio_filter(index)]})}else{serde_json::json!({"command":["set_property","audio-mode",index]})}){s.status=error;return;}
            }
            s.audio_mode=index;s.save_config();
        });menu.append(&item);
    }
    style_dropdown(&menu,w.max(180),30);menu.show_all();
    menu.popup_at_rect(&surface,&anchor,gdk::Gravity::SouthWest,gdk::Gravity::NorthWest,None::<&gdk::Event>);
    if let Some(item)=selected{menu.select_item(&item);}
}
pub fn connect_keys(widget:&impl IsA<gtk::Window>,state:Rc<RefCell<State>>,viewer:&ApplicationWindow){
    let window=viewer.clone();
    widget.as_ref().connect_key_press_event(move |_,event|{
        if event.state().intersects(gdk::ModifierType::CONTROL_MASK|gdk::ModifierType::MOD1_MASK|gdk::ModifierType::SHIFT_MASK){return glib::Propagation::Proceed;}
        let key=event.keyval().name().map(|s|s.to_string()).unwrap_or_default();
        let mut s=state.borrow_mut();
        if s.channel_key(&key){return glib::Propagation::Stop;}
        let action=match key.as_str(){
            "Left"=>Some(Action::VolumeDown),"Right"=>Some(Action::VolumeUp),
            "Up"=>Some(Action::Next),"Down"=>Some(Action::Previous),
            "space"=>Some(Action::Play),"F11"=>Some(Action::Fullscreen),
            "Escape" if s.fullscreen=>Some(Action::Fullscreen),_=>None,
        };
        if let Some(action)=action{s.handle(action,&window);window.queue_draw();glib::Propagation::Stop}
        else{glib::Propagation::Proceed}
    });
}
// Compose in the player so alpha works even when WSLg does not blend child windows.
const OSD_OPACITY:f64=191.0/255.0; // Same foreground alpha as Windows osd::OPACITY.
fn osd_text(c:&gtk::cairo::Context,text:&str,x:f64,y:f64,size:f64){
    c.select_font_face("sans-serif",gtk::cairo::FontSlant::Normal,gtk::cairo::FontWeight::Bold);
    c.set_font_size(size);c.move_to(x,y);c.text_path(text);
    c.set_source_rgb(100./255.,100./255.,100./255.);c.set_line_width(2.);let _=c.stroke_preserve();
    c.set_source_rgb(0.,1.,0.);let _=c.fill();
}
pub(super) fn osd_bitmap(label:&str,volume:Option<i32>,width:i32,height:i32,scale:i32)->gtk::cairo::ImageSurface{
    let surface=gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32,width*scale,height*scale).expect("OSD surface");
    let c=gtk::cairo::Context::new(&surface).expect("OSD context");
    c.scale(scale as f64,scale as f64);c.set_antialias(gtk::cairo::Antialias::None);
    c.push_group();
    for (i,line) in label.lines().take(3).enumerate(){osd_text(&c,line,3.,if i==0{29.}else{62.+(i-1) as f64*28.},if i==0{24.}else{18.});}
    if let Some(value)=volume{
        let bar_width=((width-8)/20-3).max(1);
        for i in 0..20{
            let x=(4+i*(bar_width+3)) as f64;
            c.set_source_rgb(100./255.,100./255.,100./255.);c.rectangle(x,44.,bar_width as f64,16.);let _=c.fill();
            c.set_operator(gtk::cairo::Operator::Source);
            if i*5<value{c.set_source_rgb(0.,1.,0.);}else{c.set_source_rgba(0.,0.,0.,0.);}
            c.rectangle(x+1.,45.,(bar_width-2).max(0) as f64,14.);let _=c.fill();c.set_operator(gtk::cairo::Operator::Over);
        }
    }
    let _=c.pop_group_to_source();let _=c.paint_with_alpha(OSD_OPACITY);
    drop(c);surface.flush();surface
}
pub(super) fn send_osd(s:&State,id:u32,frame:&mut gtk::cairo::ImageSurface,x:i32,y:i32)->Result<(),String>{
    let path=s.ipc.with_extension(format!("osd-{id}.bgra"));
    let (width,height,stride)=(frame.width(),frame.height(),frame.stride());
    // Cairo ARgb32 is premultiplied BGRA on the supported little-endian Linux builds.
    let bytes=frame.data().map_err(|e|e.to_string())?;
    std::fs::write(&path,&*bytes).map_err(|e|e.to_string())?;drop(bytes);
    let result=s.player_request(serde_json::json!({"command":{"name":"overlay-add","id":id,"x":x,"y":y,"file":path,"offset":0,"fmt":"bgra","w":width,"h":height,"stride":stride}}));
    let _=std::fs::remove_file(path);result.map(|_|())
}
pub fn install_osd(state:Rc<RefCell<State>>,viewer:&ApplicationWindow,video:&EventBox){
    type Frame=(gtk::cairo::ImageSurface,i32,i32);
    let idle_frames:Rc<RefCell<Vec<Frame>>>=Rc::new(RefCell::new(Vec::new()));
    let idle=idle_frames.clone();
    video.connect_draw(move|_,c|{
        for (surface,x,y) in idle.borrow().iter(){
            let _=c.save();c.translate(*x as f64,*y as f64);
            let _=c.set_source_surface(surface,0.,0.);let _=c.paint();let _=c.restore();
        }
        glib::Propagation::Proceed
    });
    let weak=viewer.downgrade();let video=video.clone();
    let mut shown:[Option<String>;2]=[None,None];let mut player_inode=0;
    glib::timeout_add_local(Duration::from_millis(50),move||{
        let Some(viewer)=weak.upgrade() else{return glib::ControlFlow::Break;};
        let now=Instant::now();let mut s=state.borrow_mut();
        if s.channel_deadline.is_some_and(|t|now>=t){s.commit_channel();}
        use std::os::unix::fs::MetadataExt;
        let playing=s.file_player.is_some()||s.job.as_ref().is_some_and(|j|j.kind==JobKind::Watch);
        let inode=if playing{std::fs::metadata(&s.ipc).map(|m|m.ino()).unwrap_or(0)}else{0};
        if inode!=player_inode{shown=[None,None];player_inode=inode;}
        let allocation=video.allocation();let (w,h)=(allocation.width(),allocation.height());
        let scale=video.scale_factor().max(1);
        let visible=viewer.is_visible()&&!viewer.window().is_some_and(|w|w.state().contains(gdk::WindowState::ICONIFIED));
        let requests=[(s.volume_osd_until.is_some_and(|t|now<t),i18n::text(&format!("Volume {}%",s.volume)),Some(s.volume),320,68,24,(h-92).max(0)),
            (s.channel_osd_until.is_some_and(|t|now<t),i18n::text(&s.channel_osd),None,640,if s.channel_osd.lines().count()>2{104}else{76},24,24)];
        let mut idle=idle_frames.borrow_mut();let had_idle=!idle.is_empty();idle.clear();
        for (index,(active,label,value,width,height,x,y)) in requests.into_iter().enumerate(){
            let id=60+index as u32;
            if active&&visible&&w>48&&h>height{
                let width=width.min(w-48);let key=format!("{label}:{w}:{h}:{scale}");
                if inode!=0{
                    if shown[index].as_ref()!=Some(&key){
                        let mut frame=osd_bitmap(&label,value,width,height,scale);
                        if send_osd(&s,id,&mut frame,x*scale,y*scale).is_ok(){shown[index]=Some(key);}
                    }
                }else{idle.push((osd_bitmap(&label,value,width,height,1),x,y));}
            }else if shown[index].take().is_some()&&inode!=0{
                let _=s.player_request(serde_json::json!({"command":["overlay-remove",id]}));
            }
        }
        if had_idle||!idle.is_empty(){video.queue_draw();}
        glib::ControlFlow::Continue
    });
}
#[cfg(test)]mod osd_alpha_tests{
    use super::*;
    #[test]fn windows_opacity_and_clear_background(){
        let mut frame=osd_bitmap("Volume 50%",Some(50),320,68,1);
        let stride=frame.stride() as usize;let data=frame.data().unwrap();
        assert_eq!(&data[0..4],&[0,0,0,0]);
        assert_eq!(&data[50*stride+6*4..50*stride+6*4+4],&[0,191,0,191]);
        assert_eq!(&data[50*stride+230*4..50*stride+230*4+4],&[0,0,0,0]);
        assert!(data.chunks_exact(4).all(|p|p[3]<=191));
    }
}

pub fn audio_filter(index:usize)->&'static str {
    match index{
                1=>"lavfi=[aformat=channel_layouts=stereo,pan=stereo|c0=0.5*c0+0.5*c1|c1=0.5*c0+0.5*c1]",
                2=>"lavfi=[aformat=channel_layouts=stereo,pan=stereo|c0=c0|c1=c0]",
                3=>"lavfi=[aformat=channel_layouts=stereo,pan=stereo|c0=c1|c1=c1]",
                4=>"",_=>"lavfi=[aformat=channel_layouts=stereo]",
    }
}
#[cfg(test)]mod audio_routing_tests{
    use super::*;
    #[test]fn pcm_modes_route_samples_and_preserve_surround(){
        for (channels,input) in [(1,vec![0.25f32]),(2,vec![0.25,-0.125]),(6,vec![0.25,-0.125,0.2,0.05,0.1,-0.1])]{
            for mode in 0..5{
                let mut cmd=Command::new("ffmpeg");cmd.args(["-v","error","-f","f32le","-ar","48000","-ac",&channels.to_string(),"-channel_layout",match channels{1=>"mono",2=>"stereo",_=>"5.1"},"-i","pipe:0"]);
                let filter=audio_filter(mode);
                if !filter.is_empty(){cmd.args(["-af",filter.strip_prefix("lavfi=[").unwrap().strip_suffix(']').unwrap()]);}
                let mut child=cmd.args(["-f","f32le","pipe:1"]).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().unwrap();
                let bytes:Vec<u8>=(0..128).flat_map(|_|input.iter().flat_map(|v|v.to_le_bytes())).collect();
                child.stdin.take().unwrap().write_all(&bytes).unwrap();let result=child.wait_with_output().unwrap();
                assert!(result.status.success(),"{}",String::from_utf8_lossy(&result.stderr));
                let samples:Vec<f32>=result.stdout.chunks_exact(4).map(|b|f32::from_le_bytes(b.try_into().unwrap())).collect();
                if mode==4{assert_eq!(samples.len(),input.len()*128);assert_eq!(&samples[..input.len()],input.as_slice());}
                else{
                    assert_eq!(samples.len(),256);
                    if channels==2{
                        let expected=match mode{0=>[0.25,-0.125],1=>[0.0625,0.0625],2=>[0.25,0.25],_=>[-0.125,-0.125]};
                        assert!((samples[0]-expected[0]).abs()<0.0001&&(samples[1]-expected[1]).abs()<0.0001,"mode {mode}: {samples:?}");
                    }
                    if mode!=0||channels==1{assert!((samples[0]-samples[1]).abs()<0.0001);}
                }
            }
        }
    }
}
