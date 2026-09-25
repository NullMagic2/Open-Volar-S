//! Live, persistent GTK guide with the same event data as the Windows guide.
use super::*;
use a865r_media::guide_data;
pub(super) fn show(state:Rc<RefCell<State>>,owner:&ApplicationWindow){
    let panel=gtk::Window::new(gtk::WindowType::Toplevel);i18n::title(&panel,"Live TV! · Program guide");
    panel.set_transient_for(Some(owner));panel.set_default_size(920,600);panel.set_position(gtk::WindowPosition::CenterOnParent);
    transparent_panel(&panel);round_window(&panel,12);panel.set_decorated(false);
    panel.style_context().add_class("orbit-guide");
    let css=gtk::CssProvider::new();css.load_from_data(include_bytes!("guide.css")).expect("guide CSS");
    if let Some(screen)=gtk::prelude::WidgetExt::screen(&panel){gtk::StyleContext::add_provider_for_screen(&screen,&css,gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);}
    let overlay=gtk::Overlay::new();let fascia=DrawingArea::new();fascia.set_size_request(720,480);
    let st=state.clone();fascia.connect_draw(move|w,c|{let a=w.allocation();st.borrow().skin.draw_panel_chrome(c,a.width(),a.height(),"");glib::Propagation::Proceed});overlay.add(&fascia);
    let frame=gtk::Box::new(Orientation::Vertical,8);frame.set_margin_start(18);frame.set_margin_end(18);frame.set_margin_bottom(18);
    let header=EventBox::new();header.set_visible_window(false);header.set_size_request(-1,44);
    let title=i18n::label("Program guide");title.set_xalign(0.);title.set_margin_start(20);title.style_context().add_class("guide-title");header.add(&title);
    let p=panel.clone();header.connect_button_press_event(move|_,ev|{
        if ev.button()==1 {if ev.event_type()==gdk::EventType::DoubleButtonPress{if p.is_maximized(){p.unmaximize();}else{p.maximize();}}
        else if ev.event_type()==gdk::EventType::ButtonPress{p.begin_move_drag(1,ev.root().0 as i32,ev.root().1 as i32,ev.time());}}
        glib::Propagation::Stop
    });
    let bar=gtk::Box::new(Orientation::Horizontal,8);bar.pack_start(&header,true,true,0);
    let close=Button::new();close.set_valign(gtk::Align::Center);close.set_widget_name("orbit-close");close.set_tooltip_text(Some(&i18n::text("Close program guide")));
    let close_art=DrawingArea::new();close_art.set_size_request(30,30);close.add(&close_art);let st=state.clone();
    close_art.connect_draw(move|w,c|{let a=w.allocation();st.borrow().skin.draw_caption_close(c,a.width(),a.height());glib::Propagation::Proceed});
    let p=panel.clone();close.connect_clicked(move|_|p.close());bar.pack_end(&close,false,false,10);frame.pack_start(&bar,false,false,0);
    let body=gtk::Box::new(Orientation::Vertical,10);body.style_context().add_class("guide-well");frame.pack_start(&body,true,true,0);overlay.add_overlay(&frame);
    let filter=ComboBoxText::new();filter.append(Some("all"),&i18n::text("All channels"));filter.set_active_id(Some("all"));body.pack_start(&filter,false,false,0);
    let store=gtk::ListStore::new(&[String::static_type(),String::static_type(),String::static_type(),String::static_type(),String::static_type(),String::static_type(),u32::static_type()]);
    let tree=gtk::TreeView::with_model(&store);tree.set_headers_visible(true);tree.set_vexpand(true);
    for (i,title) in ["Channel","Start","End","Programme","Status","Age rating"].iter().enumerate(){let renderer=gtk::CellRendererText::new();let column=gtk::TreeViewColumn::new();column.set_title(&i18n::text(title));gtk::prelude::TreeViewColumnExt::pack_start(&column,&renderer,true);gtk::prelude::TreeViewColumnExt::add_attribute(&column,&renderer,"text",i as i32);column.set_resizable(true);if i==3{column.set_expand(true);}tree.append_column(&column);}
    let scroll=gtk::ScrolledWindow::new(None::<&gtk::Adjustment>,None::<&gtk::Adjustment>);scroll.add(&tree);body.pack_start(&scroll,true,true,0);
    let details=gtk::TextView::new();details.set_editable(false);details.set_cursor_visible(false);details.set_wrap_mode(gtk::WrapMode::WordChar);details.set_left_margin(12);details.set_right_margin(12);details.set_top_margin(10);details.set_bottom_margin(10);
    let detail_scroll=gtk::ScrolledWindow::new(None::<&gtk::Adjustment>,None::<&gtk::Adjustment>);detail_scroll.set_min_content_height(150);detail_scroll.add(&details);body.pack_start(&detail_scroll,false,true,0);
    let info=i18n::label("Programme information is received while watching TV.");info.set_xalign(0.);info.set_line_wrap(true);info.style_context().add_class("guide-footer");body.pack_start(&info,false,false,0);
    let grip=EventBox::new();grip.set_visible_window(false);grip.set_halign(gtk::Align::End);grip.set_valign(gtk::Align::End);
    let mark=Label::new(Some("◢"));mark.style_context().add_class("guide-grip");grip.add(&mark);grip.set_size_request(18,18);
    let p=panel.clone();grip.connect_button_press_event(move|_,ev|{if ev.button()==1{p.begin_resize_drag(gdk::WindowEdge::SouthEast,1,ev.root().0 as i32,ev.root().1 as i32,ev.time());}glib::Propagation::Stop});overlay.add_overlay(&grip);
    let rows=Rc::new(RefCell::new(Vec::<Value>::new()));let chosen=rows.clone();let buffer=details.buffer().unwrap();
    tree.selection().connect_changed(move|selection|{if let Some((model,it))=selection.selected(){if let Ok(n)=model.value(&it,6).get::<u32>(){if let Some(v)=chosen.borrow().get(n as usize){buffer.set_text(&guide_data::detail(v));}}}});
    let st=state.clone();let chosen=rows.clone();tree.connect_row_activated(move|view,path,_|{let Some(model)=view.model()else{return};let Some(it)=model.iter(path)else{return};let Ok(n)=model.value(&it,6).get::<u32>()else{return};if let Some(v)=chosen.borrow().get(n as usize){let mut s=st.borrow_mut();if let Some(i)=s.services.iter().position(|c|c["frequency_khz"]==v["frequency_khz"]&&c["program_id"]==v["program_id"]){s.select_channel(i);}}});
    panel.add(&overlay);panel.show_all();let weak=panel.downgrade();let mut last=None;let mut stations=Vec::<(String,String)>::new();
    glib::timeout_add_local(Duration::from_millis(250),move||{
        if weak.upgrade().is_none_or(|p|!p.is_visible()){return glib::ControlFlow::Break}
        let s=state.borrow();let selection=filter.active_id().map(|s|s.to_string()).unwrap_or_else(||"all".into());
        let signature=(s.guide.revision,selection.clone());if last.as_ref()==Some(&signature){return glib::ControlFlow::Continue}last=Some(signature);
        let mut channels=s.guide.events.clone();channels.sort_by_key(|v|(v["channel_number"].as_u64().unwrap_or(u64::MAX),v["channel_name"].as_str().unwrap_or("").to_owned()));
        let mut updated=Vec::new();for v in &channels{let id=format!("{}:{}",v["frequency_khz"],v["program_id"]);if !updated.iter().any(|(key,_)|key==&id){updated.push((id,guide_data::station(v)));}}
        if stations!=updated {filter.remove_all();filter.append(Some("all"),&i18n::text("All channels"));for (id,label) in &updated{filter.append(Some(id),label);}if !filter.set_active_id(Some(&selection)){filter.set_active_id(Some("all"));}stations=updated;}
        let selected=tree.selection().selected().and_then(|(m,it)|m.value(&it,6).get::<u32>().ok()).and_then(|i|rows.borrow().get(i as usize).map(guide_data::key));
        let events:Vec<_>=s.guide.events.iter().filter(|v|selection=="all"||format!("{}:{}",v["frequency_khz"],v["program_id"])==selection).cloned().collect();
        store.clear();*rows.borrow_mut()=events.clone();for (i,v) in events.iter().enumerate(){let start=v["start"].as_i64().unwrap_or(0);let end=start.saturating_add(v["duration"].as_i64().unwrap_or(0));let status=if v["following"]==true{"Next"}else if v["running"]==true{"Now"}else{""};let age=v["minimum_age"].as_u64().map(|v|format!("{v}+")).unwrap_or_default();
            let it=store.insert_with_values(None,&[(0,&guide_data::station(v)),(1,&guide_data::date(start)),(2,&guide_data::date(end)),(3,&v["name"].as_str().unwrap_or("")),(4,&i18n::text(status)),(5,&age),(6,&(i as u32))]);if selected==Some(guide_data::key(v)){tree.selection().select_iter(&it);}}
        if events.is_empty(){i18n::set_label(&info,"No programme information available.");}else{i18n::set_label(&info,"Programme information is received while watching TV.");}glib::ControlFlow::Continue
    });
}

#[cfg(test)]mod tests {
 use super::*;
 #[test]fn guide_updates_filters_and_retains_programme_details(){
  gtk::init().expect("Run with xvfb-run");register_selawik();add_css();let app=Application::new(Some("org.openvolars.guide-test"),Default::default());app.register(None::<&gtk::gio::Cancellable>).unwrap();
  let owner=ApplicationWindow::new(&app);let state=Rc::new(RefCell::new(State::new(None)));
  let event=serde_json::json!({"frequency_khz":641143,"program_id":17056,"event_id":5,"start":1790272800i64,"duration":3600,"channel_name":"TV TRIBUNA HD","channel_number":18,"name":"BRASIL URGENTE","description":"Descrição com acentuação: notícias.","minimum_age":12,"running":true});
  state.borrow_mut().guide.ingest(&serde_json::json!([event]));show(state.clone(),&owner);
  let pump=||{let until=Instant::now()+Duration::from_millis(350);while Instant::now()<until{while glib::MainContext::default().iteration(false){}thread::sleep(Duration::from_millis(5));}};pump();
  let panel=gtk::Window::list_toplevels().into_iter().filter_map(|w|w.downcast::<gtk::Window>().ok()).find(|w|w.title().as_deref()==Some("Live TV! · Program guide")).unwrap();
  fn all(w:&gtk::Widget)->Vec<gtk::Widget>{let mut v=vec![w.clone()];if let Some(c)=w.downcast_ref::<gtk::Container>(){for child in c.children(){v.extend(all(&child));}}v}
  let widgets=all(panel.upcast_ref());let tree=widgets.iter().find_map(|w|w.downcast_ref::<gtk::TreeView>()).unwrap();let model=tree.model().unwrap();assert_eq!(model.iter_n_children(None),1);
  tree.selection().select_iter(&model.iter_first().unwrap());let details=widgets.iter().find_map(|w|w.downcast_ref::<gtk::TextView>()).unwrap().buffer().unwrap();assert!(details.text(&details.start_iter(),&details.end_iter(),false).unwrap().contains("notícias"));
  let mut second=event.clone();second["program_id"]=serde_json::json!(20);second["channel_number"]=serde_json::json!(2);second["name"]=serde_json::json!("Second");state.borrow_mut().guide.ingest(&serde_json::json!([second]));pump();assert_eq!(model.iter_n_children(None),2);
  let filter=widgets.iter().find_map(|w|w.downcast_ref::<ComboBoxText>()).unwrap();filter.set_active_id(Some("641143:17056"));pump();assert_eq!(model.iter_n_children(None),1);
  let mut updated=event.clone();updated["description"]=serde_json::json!("Updated description");state.borrow_mut().guide.ingest(&serde_json::json!([updated]));pump();assert!(details.text(&details.start_iter(),&details.end_iter(),false).unwrap().contains("Updated description"));
  assert!(!panel.is_decorated());assert!(panel.style_context().has_class("orbit-guide"));
  if let Some(path)=std::env::var_os("OVS_GUIDE_PREVIEW"){
   pump();let a=panel.allocation();panel.window().unwrap().pixbuf(0,0,a.width(),a.height()).unwrap().savev(path,"png",&[]).unwrap();
  }
  panel.close();owner.close();let _=std::fs::remove_file(state.borrow().config_path.with_extension("epg.json"));
 }
}
