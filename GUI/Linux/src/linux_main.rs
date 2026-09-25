#[path = "../../shared/button_style.rs"] mod button_style;
#[path = "../../shared/interaction.rs"] mod interaction;
#[path = "../../Windows/src/timeline.rs"] mod timeline;
#[path = "controls.rs"] mod controls;
#[path = "guide.rs"] mod guide;
#[path = "captions.rs"] mod captions;
#[path = "video_settings.rs"] mod video_settings;
#[path = "skin.rs"] mod skin;
#[path = "parental.rs"] mod parental;
#[path = "i18n.rs"] mod i18n;
#[path = "../../Windows/src/picture.rs"] mod picture;
#[path = "../../Windows/src/countries.rs"] mod countries;

use a865r_media::playback::{Control,DeinterlaceMode,Options};
use a865r_media::television::{self,Action as TvAction};
use gtk::prelude::*;
use gtk::glib::translate::ToGlibPtr;
use gtk::{gdk,glib,Application,ApplicationWindow,Button,ComboBoxText,DrawingArea,
    EventBox,Fixed,Label,Orientation,ResponseType};
use serde_json::Value;
use skin::{Action,DrawState,Skin};
use picture::Picture;
use std::cell::{Cell,RefCell};
use std::ffi::{c_int,c_void,CString};
use std::io::{Write,BufRead,BufReader};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::net::UnixStream;
use std::path::{Path,PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::sync::mpsc::{self,Receiver};
use std::sync::atomic::{AtomicU64,Ordering as AtomicOrdering};
use std::thread;
use std::time::{Duration,Instant,SystemTime,UNIX_EPOCH};

#[link(name="gdk-3")]
unsafe extern "C" { fn gdk_x11_window_get_xid(window:*mut c_void)->u64; }
#[link(name="fontconfig")]
unsafe extern "C" {
    fn FcConfigGetCurrent()->*mut c_void;
    fn FcConfigAppFontAddFile(config:*mut c_void,file:*const u8)->c_int;
}
fn register_selawik(){
    let source=Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");
    let installed=Path::new("/usr/share/fonts/truetype/open-volar-s");
    for name in ["selawk.ttf","selawksb.ttf"] {
        let path=if installed.join(name).is_file(){installed.join(name)}else{source.join(name)};
        if let Ok(c)=CString::new(path.as_os_str().as_bytes()) {
            unsafe{let _=FcConfigAppFontAddFile(FcConfigGetCurrent(),c.as_ptr() as *const u8);}
        }
    }
}
static NEXT_SESSION:AtomicU64=AtomicU64::new(0);
#[derive(Clone,Copy,PartialEq,Eq)]
enum JobKind{Watch,Discover,Record}
struct Job{control:Control,result:Receiver<Value>,kind:JobKind}
#[derive(Clone,Copy)]
struct WindowedPlacement {x:i32,y:i32,width:i32,height:i32,maximized:bool,deck:Option<(i32,i32)>}
struct ExportJob{result:Receiver<Result<Value,String>>,worker:Option<thread::JoinHandle<()>>}
struct State{
    guide:a865r_media::guide_data::Cache,
    skin:Skin,config_path:PathBuf,frequency_khz:u32,deinterlacing:DeinterlaceMode,
    scan_first_khz:u32,scan_last_khz:u32,scan_step_khz:u32,
    status:String,export_notice:Option<String>,services:Vec<Value>,service_index:usize,
    job:Option<Job>,pending:Option<(TvAction,JobKind)>,volume:i32,
    ipc:PathBuf,window_id:u64,fullscreen:bool,shader_revision:Cell<u64>,old_shaders:RefCell<Vec<PathBuf>>,shader_source:RefCell<String>,
    applied_video:Option<Vec<(String,Value)>>,picture_update:Option<glib::SourceId>,
    fullscreen_layout:Rc<Cell<bool>>,windowed:Option<WindowedPlacement>,
    recording_folder:PathBuf,snapshot_folder:PathBuf,
    has_frequency:bool,frequency_saved:bool,
    parental:Value,parental_unlocked:bool,button_material:usize,
    video_options:Value,picture:Picture,picture_applied:bool,service_applied:bool,countries:Vec<Value>,country:usize,
    file_player:Option<std::process::Child>,pending_file:Option<PathBuf>,file_path:Option<PathBuf>,
    paused:bool,captions:bool,audio_mode:usize,recording_path:Option<PathBuf>,record_when_ready:bool,record_settings:Option<a865r_media::recording_export::Settings>,exports:Vec<ExportJob>,
    channel_digits:String,channel_deadline:Option<Instant>,
    channel_osd:String,channel_osd_until:Option<Instant>,volume_osd_until:Option<Instant>,
    timeline_tick:Instant,timeline_position:f64,timeline_duration:f64,
    timeline_seekable:bool,seek_drag:Option<f64>,recording_start:Option<f64>,following_live:bool,
}
impl State{
    fn new(initial:Option<u32>)->Self{
        #[cfg(not(test))]
        let home=std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
        #[cfg(not(test))]
        let path=home.join(".config/open-volar-s/live-tv.json");
        #[cfg(test)]
        let path=std::env::temp_dir().join(format!("ovs-test-{}-{}.json",std::process::id(),
            NEXT_SESSION.fetch_add(1,AtomicOrdering::Relaxed)));
        Self::new_at(initial,path)
    }
    fn new_at(initial:Option<u32>,config_path:PathBuf)->Self{
        let stamp=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
        let session=NEXT_SESSION.fetch_add(1,AtomicOrdering::Relaxed);
        let home=std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
        let config=std::fs::read(&config_path).ok()
            .and_then(|bytes|serde_json::from_slice::<Value>(&bytes).ok())
            .unwrap_or(Value::Null);
        i18n::load(config["language"].as_str().unwrap_or("en"));
        let saved=config["last_frequency_khz"].as_u64()
            .and_then(|n|u32::try_from(n).ok())
            .filter(|n|(470_000..=697_999).contains(n));
        let mut video_options=config["video"].as_object().map(|v|Value::Object(v.clone()))
            .unwrap_or_else(||serde_json::json!({}));
        if video_options["size_schema"]!=1 {
            let old=video_options["size"].as_u64().unwrap_or(0);
            video_options["size"]=serde_json::json!(match old {0=>0,1|2=>1,_=>2});
            video_options["size_schema"]=serde_json::json!(1);
        }
        if video_options["decoder_schema"]!=1 {
            let old=video_options["decoder"].as_u64().unwrap_or(0);
            video_options["decoder"]=serde_json::json!(match old {2=>1,3=>2,_=>0});
            video_options["decoder_schema"]=serde_json::json!(1);
        }
        let skin=Skin::new();
        let button_material=match config["button_material"].as_str(){
            Some("glass")=>1,Some("plastic")=>2,_=>0};
        skin.set_material(button_material);
        let countries=config["countries"]["profiles"].as_array()
            .filter(|profiles|!profiles.is_empty()).cloned()
            .unwrap_or_else(countries::defaults);
        let country=(config["countries"]["selected"].as_u64().unwrap_or(0) as usize)
            .min(countries.len()-1);
        let profile=&countries[country];
        let scan_first=profile["first"].as_u64().unwrap_or(473_143) as u32;
        let scan_last=profile["last"].as_u64().unwrap_or(695_143) as u32;
        let scan_step=profile["step"].as_u64().unwrap_or(6_000) as u32;
        let mut services=config["services"].as_array().cloned().unwrap_or_default();
        let selected=(config["service_index"].as_u64().unwrap_or(0) as usize)
            .min(services.len().saturating_sub(1));
        let service_index=interaction::sort_channels(&mut services,selected);
        Self{guide:a865r_media::guide_data::Cache::load(config_path.with_extension("epg.json")),skin,config_path,button_material,countries,country,frequency_khz:initial.or(saved).unwrap_or(473_143),
            deinterlacing:match config["deinterlacing"].as_u64(){Some(0)=>DeinterlaceMode::DoubleRate,Some(1)=>DeinterlaceMode::SingleRate,Some(2)=>DeinterlaceMode::Off,_=>if std::env::var_os("WSL_DISTRO_NAME").is_some(){DeinterlaceMode::SingleRate}else{DeinterlaceMode::DoubleRate}},
            scan_first_khz:scan_first,scan_last_khz:scan_last,scan_step_khz:scan_step,
            status:"Choose a channel or open Settings to scan.".into(),export_notice:None,
            services,service_index,
            job:None,pending:None,volume:config["volume"].as_i64().unwrap_or(65).clamp(0,100) as i32,
            ipc:std::env::temp_dir().join(format!("open-volar-s-mpv-{}-{stamp}-{session}.sock",std::process::id())),
            window_id:0,fullscreen:false,
            fullscreen_layout:Rc::new(Cell::new(false)),windowed:None,
            recording_folder:config["recording_folder"].as_str().map(PathBuf::from)
                .unwrap_or_else(||home.join("Videos").join("Open Volar S")),
            snapshot_folder:config["snapshot_folder"].as_str().map(PathBuf::from)
                .unwrap_or_else(||home.join("Pictures").join("Open Volar S")),
            has_frequency:initial.or(saved).is_some(),frequency_saved:false,
            parental:config["parental"].clone(),parental_unlocked:false,
            video_options,picture:Picture::load(&config["picture"]),picture_applied:false,service_applied:false,
            shader_revision:Cell::new(0),old_shaders:RefCell::new(Vec::new()),shader_source:RefCell::new(String::new()),applied_video:None,picture_update:None,file_player:None,pending_file:None,file_path:None,paused:false,recording_path:None,record_when_ready:false,record_settings:None,exports:Vec::new(),
            captions:config["captions_enabled"].as_bool().unwrap_or(false),
            audio_mode:config["audio_mode"].as_u64().unwrap_or(0).min(4) as usize,
            channel_digits:String::new(),channel_deadline:None,channel_osd:String::new(),
            channel_osd_until:None,volume_osd_until:None,
            timeline_tick:Instant::now(),timeline_position:0.,timeline_duration:0.,
            timeline_seekable:false,seek_drag:None,recording_start:None,following_live:false}
    }
    fn start(&mut self,action:TvAction,kind:JobKind){
        self.export_notice=None;
        self.stop_file();self.paused=false;
        self.timeline_tick=Instant::now();self.timeline_position=0.;
        self.timeline_duration=0.;self.timeline_seekable=false;self.seek_drag=None;self.recording_start=None;self.following_live=false;
        if self.job.is_some(){
            self.pending=Some((action,kind));
            if let Some(job)=&self.job{job.control.stop();}
            return;
        }
        if kind==JobKind::Watch{
            self.picture_applied=false;self.service_applied=false;
            match self.video_args(){Ok(args)=>std::env::set_var("A865R_MPV_VIDEO_ARGS",serde_json::to_string(&args).unwrap()),Err(e)=>{self.status=e;return;}}
            if self.window_id==0{self.status="Embedded GTK video surface is not ready.".into();return;}
            std::env::set_var("A865R_MPV_WID",self.window_id.to_string());
            std::env::set_var("A865R_MPV_IPC",&self.ipc);
            std::env::set_var("A865R_MPV_SCREENSHOT_DIR",&self.snapshot_folder);
            std::env::set_var("A865R_MPV_VOLUME",self.volume.to_string());
            std::env::set_var("A865R_PLAYER_AUDIO_MODE",self.audio_mode.to_string());
            if a865r_media::wsl_video::is_wsl(){std::env::set_var("A865R_MPV_AF",controls::audio_filter(self.audio_mode));}
            std::env::set_var("A865R_MPV_CAPTIONS",if self.captions{"yes"}else{"no"});
        }
        let control=Control::default();let worker=control.clone();
        let mut options=Options::default();options.deinterlacing=self.deinterlacing;
        let program=self.current_program_id();if program!=0{options.program_id=Some(program);}
        let (sender,result)=mpsc::channel();
        self.status=match kind{JobKind::Watch=>"Connecting to A865R…",JobKind::Discover=>"Scanning TV channels…",JobKind::Record=>"Starting recording…"}.into();
        thread::spawn(move||{
            let stamp=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
            let folder=std::env::temp_dir().join(format!("open-volar-s-live-tv-{stamp}"));
            let report=television::run(action,PathBuf::new(),folder,options,worker);
            let _=sender.send(report);
        });
        self.job=Some(Job{control,result,kind});
    }
    fn current_program_id(&self)->u32{
        self.services.get(self.service_index)
            .and_then(|service|service["program_id"].as_u64())
            .or_else(||self.job.as_ref().and_then(|j|j.control.snapshot()["service"]["program_id"].as_u64()))
            .and_then(|n|u32::try_from(n).ok()).unwrap_or(0)
    }
    fn parental_check(&self)->Result<(),String>{
        let program=self.current_program_id();
        let events=self.job.as_ref().map(|j|j.control.snapshot()["epg"].clone())
            .unwrap_or(Value::Null);
        let rating=parental::rating(&events,self.frequency_khz,program);
        if parental::blocked(&self.parental,self.frequency_khz,program,rating,self.parental_unlocked){
            Err("Parental controls: this program is blocked. Unlock in Settings > Parental.".into())
        }else{Ok(())}
    }
    fn watch(&mut self){
        if let Err(e)=self.parental_check(){self.status=e;return;}
        self.export_notice=None;
        self.start(TvAction::Watch{frequency:self.frequency_khz},JobKind::Watch);
    }
    fn button_material_name(&self)->&'static str{["metal","glass","plastic"][self.button_material]}
    fn save_config(&self){
        let path=&self.config_path;
        if let Some(parent)=path.parent(){let _=std::fs::create_dir_all(parent);}
        let data=serde_json::json!({"last_frequency_khz":self.frequency_khz,
            "services":self.services,"service_index":self.service_index,"parental":self.parental,"language":i18n::code(),
            "volume":self.volume,"captions_enabled":self.captions,"audio_mode":self.audio_mode,
            "button_material":self.button_material_name(),"picture":self.picture.json(),"video":self.video_options,
            "deinterlacing":match self.deinterlacing{DeinterlaceMode::DoubleRate=>0,DeinterlaceMode::SingleRate=>1,DeinterlaceMode::Off=>2},
            "countries":{"selected":self.country,"profiles":self.countries},
            "recording_folder":self.recording_folder,"snapshot_folder":self.snapshot_folder});
        let temporary=path.with_extension("json.tmp");
        if let Err(error)=std::fs::write(&temporary,data.to_string())
            .and_then(|()|std::fs::rename(&temporary,&path)) {
            eprintln!("Cannot save Live TV settings at {}: {error}",path.display());
        }
    }
    fn remember_frequency(&mut self){
        if self.frequency_saved{return;}
        self.save_config();self.has_frequency=true;self.frequency_saved=true;
    }
    fn configure_recording_cache(&self,enabled:bool)->Result<(),String>{
        if !a865r_media::wsl_video::is_wsl(){
            // The native helper continuously keeps a bounded disk cache; recording
            // and pausing do not change the tuner-reader lifetime or block USB.
            self.player_request(serde_json::json!({"command":["get_property","demuxer-cache-state"]}))?;
            return Ok(());
        }
        // Keep packet data on disk while recording. A large forward cache is essential:
        // rewinding must never fill the cache and block the USB capture pipe.
        let options=if enabled{
            [("cache-on-disk",serde_json::json!(true)),
             ("demuxer-max-back-bytes",serde_json::json!("1GiB")),
             ("demuxer-max-bytes",serde_json::json!("1GiB")),
             ("cache-secs",serde_json::json!(3600000))]
        }else{
            [("cache-secs",serde_json::json!(2)),
             ("demuxer-max-bytes",serde_json::json!("150MiB")),
             ("demuxer-max-back-bytes",serde_json::json!("50MiB")),
             ("cache-on-disk",serde_json::json!(false))]
        };
        for (name,value) in options{
            self.player_request(serde_json::json!({"command":["set_property",name,value]}))?;
        }
        Ok(())
    }
    fn recording_cache_range(&self)->Option<(f64,f64)>{
        let state=self.player_request(serde_json::json!({"command":["get_property","demuxer-cache-state"]})).ok()?;
        let ranges=state["seekable-ranges"].as_array()?;
        ranges.iter().filter_map(|range|{
            let start=range["start"].as_f64()?;
            let end=range["end"].as_f64()?;
            (start.is_finite()&&end.is_finite()&&end>start).then_some((start,end))
        }).max_by(|a,b|a.1.total_cmp(&b.1))
    }
    fn record(&mut self){
        if self.recording_path.is_some(){
            if self.timeline_seekable{self.seek_recording_seconds(self.timeline_duration);}
            self.finish_recording();return;
        }
        self.export_notice=None;
        if let Err(e)=self.parental_check(){self.status=e;return;}
        if !self.job.as_ref().is_some_and(|j|j.kind==JobKind::Watch){
            if self.file_player.is_some(){self.status="Select a live channel to record.".into();return;}
            self.record_when_ready=true;self.watch();return;
        }
        if let Err(e)=std::fs::create_dir_all(&self.recording_folder){self.status=format!("Cannot create recording folder: {e}");return;}
        if let Err(e)=self.configure_recording_cache(true){self.status=format!("Cannot enable live recording seek: {e}");return;}
        let start=self.recording_cache_range().map(|(_,end)|end)
            .or_else(||self.player_request(serde_json::json!({"command":["get_property","time-pos"]})).ok()?.as_f64())
            .unwrap_or(0.);
        let stamp=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let path=self.recording_folder.join(format!("Live TV {stamp}.broadcast.ts"));
        let settings=a865r_media::recording_export::Settings{program:self.current_program_id(),
            resolution:match self.video_options["size"].as_u64().unwrap_or(0){1=>a865r::api::Resolution::Qhd,2=>a865r::api::Resolution::Uhd,_=>a865r::api::Resolution::Native},
            deinterlacing:self.deinterlacing,picture:self.picture.json()};
        if let Err(e)=std::fs::write(path.with_extension("settings.json"),settings.json().to_string()){self.status=e.to_string();return;}
        self.record_settings=Some(settings);
        match self.job.as_ref().unwrap().control.start_selected_recording(&path,self.current_program_id()).map_err(|e|e.to_string()){
            Ok(_)=>{self.status=format!("Recording to {}",path.display());self.recording_path=Some(path);
                self.recording_start=Some(start);self.following_live=!self.paused;self.timeline_position=0.;self.timeline_duration=0.;self.timeline_seekable=false;},
            Err(e)=>{let _=self.configure_recording_cache(false);self.status=e;},
        }
    }
    fn finish_recording(&mut self){
        let Some(input)=self.recording_path.take()else{return;};
        let result=self.job.as_ref().map(|j|j.control.stop_recording()).unwrap_or(Ok(()));
        self.recording_start=None;self.following_live=false;self.timeline_seekable=false;
        let _=self.configure_recording_cache(false);
        let settings=self.record_settings.take();
        if let Err(e)=result{self.status=format!("{e}; capture preserved at {}",input.display());return;}
        let Some(settings)=settings else{self.status="Recording settings missing; source capture preserved.".into();return;};
        let name=input.file_name().unwrap().to_string_lossy().replace(".broadcast.ts",".ts");
        let output=input.with_file_name(name);
        let (sender,result)=mpsc::channel();
        let worker=thread::spawn(move||{
            let report=a865r_media::recording_export::finish(&input,&output,&settings,Path::new("ffmpeg"),&Control::default());
            if report.is_ok(){
                // Only remove the temporary capture created by this operation,
                // after the final rendered recording has been verified/published.
                let _=std::fs::remove_file(&input);
                let _=std::fs::remove_file(input.with_extension("settings.json"));
            }
            if let Err(e)=&report{eprintln!("[ERROR]: {e}");}
            let _=sender.send(report);
        });
        self.exports.push(ExportJob{result,worker:Some(worker)});
        self.status="Processing recording with the selected resolution and picture settings…".into();
    }
    fn scan(&mut self){
        let mut frequencies=Vec::new();let mut freq=self.scan_first_khz;
        while freq<=self.scan_last_khz&&frequencies.len()<256{
            frequencies.push(freq);
            let Some(next)=freq.checked_add(self.scan_step_khz.max(1))else{break};freq=next;
        }
        self.start(TvAction::Discover(frequencies),JobKind::Discover);
    }
    fn poll(&mut self){
        if let Some(job)=&self.job {self.guide.ingest(&job.control.snapshot()["epg"]);}
        let mut index=0;
        while index<self.exports.len(){
            if let Ok(result)=self.exports[index].result.try_recv(){
                let mut export=self.exports.remove(index);if let Some(worker)=export.worker.take(){let _=worker.join();}
                self.status=match result{Ok(v)=>format!("Recording ready: {}",v["recording"].as_str().unwrap_or("")),Err(e)=>e};
                self.export_notice=Some(self.status.clone());
            }else{index+=1;}
        }
        if let Some(exit)=self.file_player.as_mut().and_then(|p|p.try_wait().ok().flatten()){
            self.file_player=None;self.status=if exit.success(){"Stopped.".into()}else{"Native playback failed. This backend requires supported Vulkan Video H.264 decoding; see NATIVE-LINUX-PLAYER.md.".into()};self.paused=false;
        }
        if !a865r_media::wsl_video::is_wsl()&&self.ipc.exists(){
            if let Ok(Value::String(error))=self.player_request(serde_json::json!({"command":["get_property","native-error"]})){self.status=error;}
        }
        self.update_timeline();
        let (kind,message,result,stopping)={
            let Some(job)=&self.job else{return};
            let stopping=job.control.cancel.load(std::sync::atomic::Ordering::Relaxed);
            // Once a new channel is selected, old service metadata must not
            // reset service_index before the pending channel starts playing.
            (job.kind,if stopping{String::new()}else{job.control.message.lock().ok().map(|m|m.clone()).unwrap_or_default()},
                job.result.try_recv().ok(),stopping)
        };
        if !message.is_empty(){
            if message.starts_with("Live TV! ·"){
                if let Some(notice)=&self.export_notice{self.status=notice.clone();}
                else if !self.exports.is_empty(){self.status="Processing recording with the selected resolution and picture settings…".into();}
                else{self.status=message.clone();}
            }else{self.status=message.clone();}
        }
        if kind==JobKind::Watch && message.starts_with("Live TV! ·"){
            if let Some(job)=&self.job{
                let snapshot=job.control.snapshot();
                if let Some(discovered)=snapshot["scan_services"].as_array(){
                    let before=self.services.clone();
                    for service in discovered{
                        if let Some(existing)=self.services.iter_mut().find(|v|v["frequency_khz"]==service["frequency_khz"]&&v["program_id"]==service["program_id"]){let number=existing["channel_number"].clone();*existing=service.clone();if existing["channel_number"].is_null(){existing["channel_number"]=number;}}
                        else{self.services.push(service.clone());}
                    }
                    self.service_index=interaction::sort_channels(&mut self.services,self.service_index);
                    if before!=self.services{
                        if let Some(index)=self.services.iter().position(|v|v["frequency_khz"]==snapshot["service"]["frequency_khz"]&&v["program_id"]==snapshot["service"]["program_id"]){self.service_index=index;}
                        self.save_config();
                    }
                }
            }
            self.remember_frequency();
            if !self.picture_applied{self.apply_picture();}
            if !self.service_applied{
                let service=self.job.as_ref().unwrap().control.snapshot()["service"].clone();
                if let Ok(tracks)=self.player_request(serde_json::json!({"command":["get_property","track-list"]})){
                    if let Some(tracks)=tracks.as_array(){
                        let mut selected=0;
                        for (property,pid) in [("vid",service["video_pid"].as_u64()),("aid",service["audio_pid"].as_u64())]{
                            if let Some(pid)=pid{if let Some(track)=tracks.iter().find(|t|t["demux-id"].as_u64()==Some(pid)){
                                if self.player_request(serde_json::json!({"command":["set_property",property,track["id"]]})).is_ok(){selected+=1;}
                            }}
                        }
                        self.service_applied=selected>0;
                    }
                }
            }
            if self.record_when_ready&&self.ipc.exists(){self.record_when_ready=false;self.record();}
        }
        if !stopping && matches!(kind,JobKind::Watch|JobKind::Record)
            && self.job.as_ref().is_some_and(|j|j.control.snapshot()["epg"].is_array()) {
            if let Err(e)=self.parental_check(){
                self.status=e;self.pending=None;
                if let Some(job)=&self.job{job.control.stop();}
                return;
            }
        }
        if let Some(report)=result{
            if self.recording_path.is_some(){self.finish_recording();}
            self.recording_path=None;self.recording_start=None;self.following_live=false;self.record_when_ready=false;
            if let Some(stations)=report["stations"].as_array(){
                let discovered:Vec<Value>=stations.iter()
                    .flat_map(|s|s["services"].as_array().cloned().unwrap_or_default())
                    .filter(|service|service["frequency_khz"].as_u64().is_some()
                        && service["program_id"].as_u64().is_some()).collect();
                if discovered.is_empty(){
                    self.status=format!("No TV services found. Keeping {} saved channels.",self.services.len());
                }else{
                    let found=discovered.len();
                    for service in discovered{
                        if let Some(existing)=self.services.iter_mut().find(|v|
                            v["frequency_khz"]==service["frequency_khz"]
                                && v["program_id"]==service["program_id"]){
                            let number=existing["channel_number"].clone();
                            *existing=service;
                            if existing["channel_number"].is_null(){existing["channel_number"]=number;}
                        }else{self.services.push(service);}
                    }
                    self.service_index=interaction::sort_channels(&mut self.services,self.service_index);
                    self.status=format!("Found {found} TV services. {} channels saved.",self.services.len());
                    if !self.has_frequency{
                        if let Some(first)=self.services.first(){
                            self.frequency_khz=first["frequency_khz"].as_u64().unwrap_or(self.frequency_khz as u64) as u32;
                            self.has_frequency=true;self.service_index=0;
                        }
                    }
                    self.save_config();
                }
            }else if report["success"]!=true{
                self.status=report["error"].as_str().unwrap_or("Receiver operation failed").into();
            }else if self.pending.is_none(){self.status="Stopped.".into();}
            self.job=None;
            if let Some(path)=self.pending_file.take(){self.open_file(path);return;}
            if let Some((action,kind))=self.pending.take(){self.start(action,kind);}
        }
    }
    fn update_timeline(&mut self){
        let now=Instant::now();
        let delta=now.saturating_duration_since(self.timeline_tick).as_secs_f64().min(3.);
        self.timeline_tick=now;
        if self.file_player.is_some(){
            if let Ok(value)=self.player_request(serde_json::json!({"command":["get_property","duration"]})){
                if let Some(duration)=value.as_f64().filter(|v|v.is_finite()&&*v>0.){
                    self.timeline_duration=duration;self.timeline_seekable=true;
                }
            }
            if let Ok(value)=self.player_request(serde_json::json!({"command":["get_property","time-pos"]})){
                if let Some(position)=value.as_f64().filter(|v|v.is_finite()&&*v>=0.){
                    self.timeline_position=position;
                }
            }
        }else if self.job.as_ref().is_some_and(|job|job.kind==JobKind::Watch){
            if let Some(start)=self.recording_start.filter(|_|self.recording_path.is_some()){
                if let Some((cache_start,cache_end))=self.recording_cache_range(){
                    self.timeline_duration=(cache_end-start).max(0.);
                    self.timeline_seekable=cache_start<=start+0.5 && self.timeline_duration>=0.5;
                    if let Ok(value)=self.player_request(serde_json::json!({"command":["get_property","time-pos"]})){
                        if let Some(position)=value.as_f64().filter(|v|v.is_finite()){
                            self.timeline_position=(position-start).clamp(0.,self.timeline_duration);
                        }
                    }
                }
                if let Some(job)=&self.job{job.control.set_timeline(self.timeline_position,self.timeline_duration,self.timeline_seekable,self.paused);}
            }else{
                if !self.paused{self.timeline_position+=delta;}
                self.timeline_duration=0.;self.timeline_seekable=false;
                if let Some(job)=&self.job{job.control.set_timeline(self.timeline_position,0.,false,self.paused);}
            }
        }else{
            self.timeline_position=0.;self.timeline_duration=0.;self.timeline_seekable=false;
            self.seek_drag=None;
        }
    }
    fn seek_recording_seconds(&mut self,seconds:f64){
        if !self.timeline_seekable{return;}
        let Some(start)=self.recording_start else{return;};
        let seconds=seconds.clamp(0.,self.timeline_duration);
        // A complete access unit needs to arrive before decoding the live edge.
        self.following_live=seconds>=self.timeline_duration-0.5 && !self.paused;
        let target=if seconds>=self.timeline_duration-0.5{
            timeline::live_position(self.timeline_duration)
        }else{seconds};
        self.timeline_position=target;
        self.mpv(serde_json::json!({"command":["seek",start+target,"absolute"]}));
    }
    fn seek_fraction(&mut self,fraction:f64){
        if !self.timeline_seekable{return;}
        let seconds=timeline::relative_position(0.,fraction.clamp(0.,1.)*self.timeline_duration,self.timeline_duration);
        if self.recording_start.is_some(){self.seek_recording_seconds(seconds);}
        else{self.timeline_position=seconds;self.mpv(serde_json::json!({"command":["seek",seconds,"absolute"]}));}
    }
    fn shader_path(&self)->PathBuf{self.ipc.with_extension(format!("{}.glsl",self.shader_revision.get()))}
    fn video_filter(&self)->Result<String,String>{
        Ok(match self.deinterlacing{
            DeinterlaceMode::DoubleRate=>"lavfi=[bwdif=mode=send_field:deint=interlaced]",
            DeinterlaceMode::SingleRate=>"lavfi=[bwdif=mode=send_frame:deint=interlaced]",_=>""
        }.into())
    }
    fn video_properties(&self)->Result<Vec<(String,Value)>,String>{
        if !a865r_media::wsl_video::is_wsl(){return Ok(vec![
            ("deinterlace-mode".into(),serde_json::json!(match self.deinterlacing{DeinterlaceMode::Off=>0,DeinterlaceMode::SingleRate=>1,_=>2})),
            ("output-size".into(),serde_json::json!(self.video_options["size"].as_u64().unwrap_or(0))),
            ("picture".into(),self.picture.json()),
            ("icc-profile".into(),serde_json::json!(self.video_options["color"].as_str().unwrap_or("monitor"))),
            ("video-aspect-override".into(),serde_json::json!(self.video_options["aspect"].as_str().unwrap_or("-1"))),
        ]);}
        let source=video_settings::shader(self.picture,self.video_options["size"].as_u64().unwrap_or(0));
        if *self.shader_source.borrow()!=source || !self.shader_path().exists(){
            let previous=self.shader_path();
            self.shader_revision.set(self.shader_revision.get().wrapping_add(1));
            std::fs::write(self.shader_path(),&source).map_err(|e|e.to_string())?;
            *self.shader_source.borrow_mut()=source;
            self.old_shaders.borrow_mut().push(previous);
        }
        let profile=self.video_options["color"].as_str().unwrap_or("monitor");
        Ok(vec![
            ("vf".into(),serde_json::json!(self.video_filter()?)),
            ("glsl-shaders".into(),serde_json::json!([self.shader_path()])),
            ("icc-profile".into(),serde_json::json!(if profile=="monitor"||profile=="off"{""}else{profile})),
            ("icc-profile-auto".into(),serde_json::json!(profile=="monitor")),
            ("video-aspect-override".into(),serde_json::json!(self.video_options["aspect"].as_str().unwrap_or("-1")))])
    }
    fn video_args(&self)->Result<Vec<String>,String>{
        let mut args=Vec::new();
        if !a865r_media::wsl_video::is_wsl(){
            for(key,value) in self.video_properties()?{args.push(format!("--{key}={}",value.as_str().map(str::to_owned).unwrap_or_else(||value.to_string())));}
            args.push("--renderer=vulkan".into());return Ok(args);
        }
        for (key,value) in self.video_properties()?{
            let value=if key=="glsl-shaders"{self.shader_path().display().to_string()}
                else if let Some(b)=value.as_bool(){if b{"yes"}else{"no"}.into()}
                else{value.as_str().unwrap_or("").to_owned()};
            args.push(format!("--{key}={value}"));
        }
        args.extend(a865r_media::native_player::renderer_args(a865r_media::native_player::resolved_renderer(self.video_options["decoder"].as_u64().unwrap_or(0)))
            .into_iter().map(str::to_owned));
        args.push("--screenshot-format=png".into());
        Ok(args)
    }
    fn apply_picture(&mut self){
        if !self.ipc.exists(){self.picture_applied=false;return;}
        let properties=match self.video_properties(){Ok(v)=>v,Err(e)=>{self.status=e;return;}};
        if !self.picture_applied{self.applied_video=None;}
        for (name,value) in &properties{
            if self.applied_video.as_ref().is_some_and(|old|old.iter().any(|(n,v)|n==name&&v==value)){continue;}
            let command=if name=="vf"{serde_json::json!({"command":["vf","set",value]})}
                else{serde_json::json!({"command":["set_property",name,value]})};
            if let Err(e)=self.player_request(command){self.picture_applied=false;self.status=e;return;}
        }
        self.applied_video=Some(properties);
        // The WSL bridge reuses its original startup arguments when seeking.
        // Keep these small shader files until session teardown so it can reopen
        // the initial hook before replaying the current picture properties.
        self.picture_applied=true;
    }
    fn player_request(&self,mut command:Value)->Result<Value,String>{
        let mut stream=UnixStream::connect(&self.ipc).map_err(|e|format!("Player is not ready: {e}"))?;
        let _=stream.set_read_timeout(Some(Duration::from_millis(400)));
        let _=stream.set_write_timeout(Some(Duration::from_millis(400)));
        command["request_id"]=serde_json::json!(1);
        stream.write_all(format!("{command}\n").as_bytes()).map_err(|e|e.to_string())?;
        let mut reader=BufReader::new(stream);
        for _ in 0..20{
            let mut line=String::new();reader.read_line(&mut line).map_err(|e|e.to_string())?;
            let response:Value=serde_json::from_str(&line).map_err(|e|e.to_string())?;
            if response["request_id"]==1{
                return if response["error"]=="success"{Ok(response["data"].clone())}
                    else{Err(format!("Player control failed: {}",response["error"].as_str().unwrap_or("unknown error")))};
            }
        }
        Err("Player control failed: no reply".into())
    }
    fn mpv(&mut self,command:Value){
        if let Err(error)=self.player_request(command){self.status=error;}
    }
    fn stop_file(&mut self){
        if let Some(mut child)=self.file_player.take(){a865r_media::wsl_video::stop_player(&mut child);}
    }
    fn open_file(&mut self,path:PathBuf){
        if self.job.is_some(){
            self.pending=None;self.pending_file=Some(path);
            if let Some(job)=&self.job{job.control.stop();}
            return;
        }
        self.stop_file();let _=std::fs::create_dir_all(&self.snapshot_folder);
        let mut command=a865r_media::native_player::player_command();
        let video_args=match self.video_args(){Ok(args)=>args,Err(e)=>{self.status=e;return;}};
        if a865r_media::wsl_video::is_wsl(){
        command.args(["--no-config","--no-terminal","--keep-open=no","--osc=no","--osd-level=0",
            "--input-default-bindings=no","--input-vo-keyboard=no","--audio-channels=auto"])
            .arg(format!("--wid={}",self.window_id))
            .arg(format!("--input-ipc-server={}",self.ipc.display()))
            .arg(format!("--volume={}",self.volume))
            .arg(format!("--af={}",controls::audio_filter(self.audio_mode)))
            .arg(format!("--sub-visibility={}",if self.captions{"yes"}else{"no"}))
            .arg(format!("--screenshot-directory={}",self.snapshot_folder.display()))
            .args(video_args).arg(&path).stdout(std::process::Stdio::inherit()).stderr(std::process::Stdio::inherit());
        }else{
            command.arg(format!("--wid={}",self.window_id))
                .arg(format!("--input-ipc-server={}",self.ipc.display()))
                .arg(format!("--volume={}",self.volume)).arg(format!("--audio-mode={}",self.audio_mode))
                .arg(format!("--screenshot-directory={}",self.snapshot_folder.display())).args(video_args).arg("--").arg(&path);
        }
        match command.spawn(){
            Ok(child)=>{self.file_player=Some(child);self.file_path=Some(path.clone());self.paused=false;self.applied_video=None;self.picture_applied=false;
                self.timeline_tick=Instant::now();self.timeline_position=0.;self.timeline_duration=0.;
                self.timeline_seekable=false;self.seek_drag=None;
                self.status=format!("Playing {}",path.file_name().unwrap_or_default().to_string_lossy());},
            Err(error)=>self.status=format!("Cannot start the native player: {error}. Install open-volar-s-player or set A865R_NATIVE_PLAYER."),
        }
    }
    fn set_volume(&mut self,value:i32,persist:bool){
        self.volume=value.clamp(0,100);
        if self.ipc.exists(){self.mpv(serde_json::json!({"command":["set_property","volume",self.volume]}));}
        self.volume_osd_until=Some(Instant::now()+Duration::from_secs(3));
        if persist{self.save_config();}
    }
    fn show_channel(&mut self,message:String){
        self.channel_osd=message;self.channel_osd_until=Some(Instant::now()+Duration::from_secs(3));
    }
    fn select_channel(&mut self,index:usize){
        if self.recording_path.is_some()||self.job.as_ref().is_some_and(|j|matches!(j.kind,JobKind::Record|JobKind::Discover)){
            self.show_channel("Channel selection\nStop recording or scanning first".into());return;
        }
        let Some(service)=self.services.get(index) else{return;};
        let number=service["channel_number"].as_u64().unwrap_or(index as u64+1);
        let name=service["name"].as_str().unwrap_or("TV").to_owned();
        self.frequency_khz=service["frequency_khz"].as_u64().unwrap_or(self.frequency_khz as u64) as u32;
        self.service_index=index;self.has_frequency=true;self.frequency_saved=false;
        self.watch();self.save_config();self.show_channel(format!("{number} – {name}\nSignal quality: —"));
    }
    fn commit_channel(&mut self){
        self.channel_deadline=None;
        let digits=std::mem::take(&mut self.channel_digits);
        if digits.is_empty(){return;}
        if let Some(index)=interaction::channel_match(&self.services,self.services.len(),&digits){self.select_channel(index);}
        else{self.show_channel(format!("Channel {digits}\nChannel not found"));}
    }
    fn channel_key(&mut self,key:&str)->bool{
        let digit=key.strip_prefix("KP_").unwrap_or(key);
        if digit.len()==1 && digit.as_bytes()[0].is_ascii_digit(){
            if self.channel_digits.len()>=4{self.channel_digits.clear();}
            self.channel_digits.push_str(digit);
        }else if !self.channel_digits.is_empty(){
            match key{
                "Return"|"KP_Enter"=>{self.commit_channel();return true;},
                "Escape"=>{self.channel_digits.clear();self.channel_deadline=None;self.channel_osd_until=None;return true;},
                "BackSpace"=>{self.channel_digits.pop();},_=>return false,
            }
        }else{return false;}
        if self.channel_digits.is_empty(){self.channel_deadline=None;self.channel_osd_until=None;}
        else{self.channel_deadline=Some(Instant::now()+Duration::from_millis(1200));
            self.show_channel(format!("Channel {}",self.channel_digits));}
        true
    }
    fn step_channel(&mut self,delta:i32){
        if self.services.is_empty(){self.status="Scan for TV services in Settings first.".into();return;}
        let count=self.services.len() as i32;
        let index=(self.service_index as i32+delta).rem_euclid(count) as usize;
        self.select_channel(index);
    }
    fn handle(&mut self,action:Action,window:&ApplicationWindow){
        match action{
            Action::Play=>if self.file_player.is_some()||self.job.as_ref().is_some_and(|j|j.kind==JobKind::Watch){
                self.paused=!self.paused;if self.paused{self.following_live=false;}self.mpv(serde_json::json!({"command":["set_property","pause",self.paused]}));
            }else if let Some(path)=self.file_path.clone(){self.open_file(path);}
            else if !self.has_frequency{self.status="Choose a channel or scan in Settings first.".into();}else{self.watch()},
            Action::Live=>{if self.recording_path.is_some(){self.paused=false;self.mpv(serde_json::json!({"command":["set_property","pause",false]}));
                self.seek_recording_seconds(self.timeline_duration);}
                else{self.file_path=None;self.watch();}},
            Action::Stop=>{if self.recording_path.is_some(){self.record();}self.record_when_ready=false;self.stop_file();self.pending_file=None;self.paused=false;self.status="Stopped.".into();self.pending=None;self.timeline_position=0.;self.timeline_duration=0.;self.timeline_seekable=false;self.seek_drag=None;if let Some(job)=&self.job{job.control.stop();}},
            Action::Record=>self.record(),
            Action::Previous=>self.step_channel(-1),Action::Next=>self.step_channel(1),
            Action::VolumeDown|Action::VolumeUp=>{
                self.set_volume(self.volume+if action==Action::VolumeUp{5}else{-5},true);
            }
            Action::Snapshot=>self.mpv(serde_json::json!({"command":["screenshot","window"]})),
            Action::Audio=>self.mpv(serde_json::json!({"command":["cycle","audio"]})),
            Action::Fullscreen=>self.toggle_fullscreen(window),
            Action::Back|Action::Forward=>{
                if self.recording_path.is_some()&&self.timeline_seekable{
                    let delta=if action==Action::Forward{10.}else{-10.};
                    self.seek_recording_seconds(timeline::relative_position(self.timeline_position,delta,self.timeline_duration));
                }else if self.file_player.is_some(){self.mpv(serde_json::json!({"command":["seek",if action==Action::Forward{10}else{-10},"relative"]}));}
                else{self.status="Start recording to seek backward in live TV.".into();}
            },
            Action::Minimize=>window.iconify(),
            Action::Maximize=>if window.is_maximized(){window.unmaximize()}else{window.maximize()},
            Action::Close=>window.close(),
            Action::Captions=>{self.captions=!self.captions;
                if self.ipc.exists(){self.mpv(serde_json::json!({"command":["set_property","sub-visibility",self.captions]}));}
                self.save_config();},
            Action::Library=>{
                if let Err(e)=Command::new("xdg-open").arg(&self.recording_folder).spawn(){
                    self.status=format!("Cannot open recording folder: {e}");
                }
            }
            Action::Open=>self.status="Choose a recording in Settings or open it from your file manager.".into(),
            Action::Guide|Action::Deck|Action::Settings|Action::Channels=>{},
        }
    }
}
impl Drop for State{
    fn drop(&mut self){if self.recording_path.is_some(){self.finish_recording();}for export in &mut self.exports{if let Some(worker)=export.worker.take(){let _=worker.join();}}self.stop_file();if let Some(job)=&self.job{job.control.stop();}let _=std::fs::remove_file(&self.ipc);let _=std::fs::remove_file(self.shader_path());for path in self.old_shaders.borrow_mut().drain(..){let _=std::fs::remove_file(path);}}
}
thread_local! {
    static MATERIAL_CSS: gtk::CssProvider = gtk::CssProvider::new();
    static MATERIAL_SKIN: Skin = Skin::new();
}
fn apply_material_css(index:usize){
    MATERIAL_SKIN.with(|skin|skin.set_material(index));
    let css=match index{
        1=>"button.material-button, button.material-button label { background: transparent; color: #e9dfce; }",
        _=>"button.material-button, button.material-button label { background: transparent; color: #282621; }",
    };
    MATERIAL_CSS.with(|provider|{let _=provider.load_from_data(css.as_bytes());});
}
fn add_css(){
    controls::install_dropdown_css();
    let provider=gtk::CssProvider::new();
    let css=b"menu.orbit-menu, menu.orbit-menu menuitem { background: #322519; color: #e9dfce; font-family: Selawik; font-size: 16px; padding: 7px 12px; } menu.orbit-menu menuitem:hover { background: #443225; color: #f1d4a5; } stack.settings-pages { background: transparent; border: 0; box-shadow: none; } \
window.compact-settings, window.compact-settings label, window.compact-settings button, window.compact-settings entry, window.compact-settings combobox { font-size: 16px; } \
window.compact-settings notebook tab { padding: 3px 2px; min-height: 13px; border-radius: 5px; box-shadow: none; background-image: none; outline: none; } \
window.compact-settings notebook tab:hover, window.compact-settings notebook tab:focus, window.compact-settings notebook tab:active, window.compact-settings notebook tab:checked { box-shadow: none; background-image: none; outline: none; } \
window.compact-settings combobox button, window.compact-settings button { min-height: 0; padding: 1px 3px; border-radius: 4px; } \
window.orbit-window { background: #1e1610; color: #e9dfce; font-family: Selawik; } \
window.orbit-window { background: transparent; } \
button, spinbutton { background: #322519; color: #e9dfce; border: 1px solid #6b563f; background-image: none; box-shadow: none; text-shadow: none; -gtk-icon-shadow: none; } \
entry { min-width: 0; min-height: 0; padding: 1px 6px; background: #322519; color: #e9dfce; border: 1px solid #6b563f; border-radius: 5px; box-shadow: none; background-image: none; text-shadow: none; } \
entry:disabled { background: #2b2118; color: #b6a58d; border-color: #584734; } \
entry selection { background: #855e39; color: #fff3df; } \
scale { min-height: 0; padding: 0; } scale trough { min-height: 2px; background: #b9a081; border: 0; border-radius: 0; box-shadow: none; } \
scale slider, scale slider:disabled { min-width: 12px; min-height: 22px; background: #aa8a55; border: 0; border-radius: 0; box-shadow: none; background-image: linear-gradient(#eed99b,#806139); } \
scale highlight { background: #a7835a; border: 0; border-radius: 3px; } \
combobox { background: transparent; border: 0; border-radius: 9px; box-shadow: none; outline: none; } \
combobox box, combobox box.linked { background: transparent; border: 0; border-radius: 9px; box-shadow: none; } \
combobox button, combobox button.combo { background: #322519; color: #e9dfce; border: 1px solid #6b563f; border-radius: 9px; box-shadow: none; outline: none; background-image: none; text-shadow: none; -gtk-icon-shadow: none; } \
combobox button cellview { background: #322519; color: #e9dfce; text-shadow: none; -gtk-icon-shadow: none; } \
combobox button:focus, combobox button:hover, combobox button:active { border-radius: 9px; outline: none; box-shadow: none; background-image: none; } \
window.popup, combobox window.popup { background: #322519; color: #e9dfce; border: 0; border-radius: 0; box-shadow: none; } \
menu { background: #322519; color: #e9dfce; border: 0; border-radius: 8px; box-shadow: inset 0 0 0 1px #6b563f; } \
menuitem, menu menuitem, combobox menuitem { background: #322519; color: #e9dfce; padding: 7px 12px; } \
menuitem:hover, menuitem:checked, menuitem:selected, combobox menuitem:hover { background: #443225; color: #f1d4a5; } \
combobox window.popup treeview, combobox window.popup treeview.view { background: #322519; color: #e9dfce; } \
combobox window.popup treeview:selected, combobox window.popup treeview.view:selected { background: #443225; color: #f1d4a5; } \
notebook { background: transparent; } notebook header { background: transparent; border: none; } \
notebook tab { background: #261c15; color: #aa977e; border: 1px solid #5b442e; border-radius: 10px; padding: 9px 8px; min-height: 26px; } \
notebook tab:checked { background: #1c140e; color: #f1d4a5; border-color: #937047; box-shadow: none; } \
notebook > header.top > tabs > tab:checked, notebook > header.top > tabs > tab:focus { background-image: none; box-shadow: none; outline: none; border-bottom: 1px solid #937047; } \
notebook > header.top > tabs { background: transparent; border: 0; box-shadow: none; } \
notebook stack { background: #1c140e; border: 1px solid #937047; border-radius: 10px; } \
window.compact-settings notebook stack { border-radius: 5px; } \
button#orbit-close { background: transparent; border: none; color: #e9dfce; font-size: 18px; padding: 0; min-width: 28px; min-height: 28px; }";
    // Scope the skin to our own windows. GTK file choosers then retain the
    // desktop theme, including its text, selections and navigation controls.
    let source=String::from_utf8_lossy(css);
    let scoped=source.split('}').filter_map(|rule|{
        let (selectors,body)=rule.split_once('{')?;
        let names=selectors.split(',').map(|selector|{
            let selector=selector.trim();
            if selector.starts_with("window.orbit-window")||selector.starts_with("window.compact-settings")||selector.starts_with("menu.orbit-menu"){
                selector.to_owned()
            }else{format!("window.orbit-window {selector}")}
        }).collect::<Vec<_>>().join(", ");
        Some(format!("{names} {{{body}}}"))
    }).collect::<String>();
    let _=provider.load_from_data(scoped.as_bytes());
    if let Some(screen)=gdk::Screen::default(){
        gtk::StyleContext::add_provider_for_screen(&screen,&provider,gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        MATERIAL_CSS.with(|material|gtk::StyleContext::add_provider_for_screen(
            &screen,material,gtk::STYLE_PROVIDER_PRIORITY_USER));
    }
}


fn choose_folder(parent:&gtk::Window,title:&str)->Option<PathBuf>{
    let chooser=gtk::FileChooserDialog::new(Some(&i18n::text(title)),Some(parent),
        gtk::FileChooserAction::SelectFolder);
    chooser.set_local_only(true);
    chooser.set_modal(true);
    chooser.set_resizable(true);
    let (picker_width,picker_height)=if std::env::var_os("WSL_DISTRO_NAME").is_some(){
        if wsl_hidpi(){(450,320)}else{(900,640)}
    }else{
        let area=gdk::Display::default().and_then(|display|display.primary_monitor())
            .map(|monitor|monitor.workarea())
            .unwrap_or_else(||gdk::Rectangle::new(0,0,1024,768));
        ((area.width()-80).clamp(400,900),(area.height()-110).clamp(300,640))
    };
    chooser.set_default_size(picker_width,picker_height);
    let sized=chooser.clone();
    chooser.connect_map_event(move |_,_|{
        let dialog=sized.clone();
        glib::timeout_add_local_once(Duration::from_millis(50),move||{
            dialog.resize(picker_width,picker_height);
        });
        glib::Propagation::Proceed
    });
    chooser.set_position(gtk::WindowPosition::CenterOnParent);
    chooser.add_buttons(&[(&i18n::text("Cancel"),ResponseType::Cancel),(&i18n::text("Select"),ResponseType::Accept)]);
    let selected=if chooser.run()==ResponseType::Accept{chooser.filename()}else{None};
    chooser.close();selected
}
fn place_setting(page:&Fixed,child:&impl IsA<gtk::Widget>,
    x:i32,y:i32,width:i32,height:i32,scale:f64){
    if let Some(button)=child.as_ref().downcast_ref::<Button>(){
        button.style_context().add_class("material-button");
        button.connect_draw(|button,c|{
            MATERIAL_SKIN.with(|skin|skin.draw_material_button(c,button.allocated_width(),button.allocated_height(),button.has_visible_focus()));
            if let Some(child)=button.child(){button.propagate_draw(&child,c);}
            glib::Propagation::Stop
        });
    }
    if let Some(entry)=child.as_ref().downcast_ref::<gtk::Entry>(){
        entry.set_width_chars(1);
    }
    if let Some(combo)=child.as_ref().downcast_ref::<ComboBoxText>(){
        controls::style_combo(combo);
        for renderer in combo.cells(){
            if let Ok(text)=renderer.downcast::<gtk::CellRendererText>(){
                text.set_font(Some(if scale<1.0{"Selawik 6"}else{"Selawik 12"}));
                text.set_ypad(0);
            }
        }
    }
    child.set_size_request((width as f64*scale).round() as i32,
        (height as f64*scale).round() as i32);
    page.put(child,(x as f64*scale).round() as i32,(y as f64*scale).round() as i32);
}
fn settings_label(page:&Fixed,text:&str,x:i32,y:i32,width:i32,scale:f64){
    let label=i18n::label(text);label.set_xalign(0.);
    label.set_line_wrap(true);label.set_line_wrap_mode(gtk::pango::WrapMode::WordChar);
    label.set_width_chars(1);label.set_max_width_chars(1);
    place_setting(page,&label,x,y,width,28,scale);
}
fn picture_label(kind:usize,value:i32)->String{
    match kind{0=>format!("Saturation {value}%"),
        1=>format!("Brightness {value:+}"),_=>format!("Contrast {value}%")}
}
fn set_picture_slider(state:&Rc<RefCell<State>>,kind:usize,value:i32,
    label:&Label,chooser:&ComboBoxText,draw:&DrawingArea,cell:&Rc<Cell<i32>>,
    updating:&Rc<Cell<bool>>){
    if cell.get()==value{return;}
    cell.set(value);draw.queue_draw();i18n::set_label(label,&picture_label(kind,value));
    if updating.get(){return;}
    let mut s=state.borrow_mut();
    match kind{0=>s.picture.saturation=value,1=>s.picture.brightness=value,
        _=>s.picture.contrast=value}
    drop(s);
    if let Some(pending)=state.borrow_mut().picture_update.take(){pending.remove();}
    let weak=Rc::downgrade(state);
    let pending=glib::timeout_add_local_once(Duration::from_millis(150),move||{
        if let Some(state)=weak.upgrade(){let mut s=state.borrow_mut();
            s.picture_update=None;s.apply_picture();s.save_config();}
    });
    state.borrow_mut().picture_update=Some(pending);
    updating.set(true);chooser.set_active(Some(4));updating.set(false);
}
fn fixed_choice(page:&Fixed,label:&str,value:&str,y:i32,scale:f64){
    settings_label(page,label,26,y,190,scale);
    let combo=ComboBoxText::new();i18n::append(&combo,value);combo.set_active(Some(0));

    place_setting(page,&combo,246,y,450,33,scale);
}
fn settings_dialog(state:Rc<RefCell<State>>,window:&ApplicationWindow) {
    let panel=gtk::Window::new(gtk::WindowType::Toplevel);
    let manually_positioned=Rc::new(Cell::new(false));
    transparent_panel(&panel);
    round_window(&panel,if wsl_hidpi(){6}else{12});
    panel.style_context().add_class("compact-settings");
    i18n::title(&panel,"Live TV! · Settings");
    let owner:gtk::Window=window.clone().upcast();
    panel.set_transient_for(Some(&owner));panel.set_position(gtk::WindowPosition::None);
    panel.set_modal(false);
    panel.set_decorated(false);panel.set_resizable(false);
    let scale=if wsl_hidpi(){0.5}else{1.0};
    let settings_width=(750.0*scale) as i32;
    let settings_height=(580.0*scale) as i32;
    panel.set_default_size(settings_width,settings_height);
    let overlay=gtk::Overlay::new();
    let fascia=DrawingArea::new();
    fascia.set_size_request(settings_width,settings_height);
    fascia.add_events(gdk::EventMask::BUTTON_PRESS_MASK);
    let selected_tab=Rc::new(Cell::new(0usize));
    let skin_state=state.clone();let selection=selected_tab.clone();
    fascia.connect_draw(move |w,c|{
        let a=w.allocation();
        let skin=&skin_state.borrow().skin;
        c.set_operator(gtk::cairo::Operator::Source);
        c.set_source_rgba(0.,0.,0.,0.);let _=c.paint();
        c.set_operator(gtk::cairo::Operator::Over);
        skin.draw_settings_chrome(c,a.width(),a.height());
        skin.draw_settings_tabs(c,a.width(),a.height(),selection.get());
        glib::Propagation::Proceed
    });
    overlay.add(&fascia);
    let body=gtk::Box::new(Orientation::Vertical,0);
    body.set_margin_start((14.0*scale) as i32);
    body.set_margin_end((14.0*scale) as i32);
    body.set_margin_top((98.0*scale) as i32);
    body.set_margin_bottom((14.0*scale) as i32);
    overlay.add_overlay(&body);
    let tabs=gtk::Stack::new();
    tabs.style_context().add_class("settings-pages");
    tabs.set_hexpand(true);tabs.set_vexpand(true);
    body.pack_start(&tabs,true,true,0);
    let make_page=||{
        let page=Fixed::new();
        page.set_size_request((722.0*scale) as i32,(450.0*scale) as i32);
        page
    };

    let video=make_page();
    settings_label(&video,"Color profile",26,20,190,scale);
    let color=ComboBoxText::new();i18n::append(&color,"Monitor ICC");i18n::append(&color,"Off");
    let profile=state.borrow().video_options["color"].as_str().unwrap_or("monitor").to_owned();
    if profile!="monitor"&&profile!="off"{color.append_text(&profile);color.set_active(Some(2));}else{color.set_active(Some(u32::from(profile=="off")));}
    place_setting(&video,&color,246,20,230,33,scale);
    let choose=i18n::button("CHOOSE ICC…");place_setting(&video,&choose,486,20,210,33,scale);
    let st=state.clone();color.connect_changed(move|combo|{
        if i18n::refreshing(){return;}let Some(index)=combo.active()else{return;};
        let mut s=st.borrow_mut();s.video_options["color"]=serde_json::json!(if index<2{if index==0{"monitor".to_owned()}else{"off".to_owned()}}else{combo.active_text().unwrap_or_default().to_string()});s.apply_picture();s.save_config();
    });
    let st=state.clone();let parent=panel.clone();let selector=color.clone();
    choose.connect_clicked(move |_|{
        let dialog=gtk::FileChooserDialog::new(Some(&i18n::text("Color profile")),Some(&parent),gtk::FileChooserAction::Open);
        dialog.set_position(gtk::WindowPosition::CenterOnParent);
        dialog.add_buttons(&[(&i18n::text("Cancel"),ResponseType::Cancel),(&i18n::text("Open"),ResponseType::Accept)]);
        let filter=gtk::FileFilter::new();filter.add_pattern("*.icc");filter.add_pattern("*.icm");dialog.add_filter(filter);
        if dialog.run()==ResponseType::Accept{if let Some(path)=dialog.filename(){
            match a865r_media::color::validate_profile(&path){
                Ok(_)=>{let mut s=st.borrow_mut();s.video_options["color"]=serde_json::json!(path);s.apply_picture();s.save_config();drop(s);
                    if selector.model().is_some_and(|m|m.iter_n_children(None)>2){gtk::prelude::ComboBoxTextExt::remove(&selector,2);}selector.append_text(&path.display().to_string());selector.set_active(Some(2));},
                Err(e)=>st.borrow_mut().status=e.to_string(),
            }
        }}dialog.close();
    });
    for (key,label,y,names) in [
        ("size","Output size",82,vec!["Native • no enlargement","2K • up to 2560 × 1440","4K • up to 3840 × 2160"]),
        ("aspect","Aspect ratio",124,vec!["Auto","4:3","16:9","16:10","5:4"]),
        ("decoder","Decoder",166,if a865r_media::wsl_video::is_wsl(){vec!["Automatic","Vulkan","OpenGL"]}else{vec!["Automatic","Vulkan"]})]{
        settings_label(&video,label,26,y,190,scale);let combo=ComboBoxText::new();
        for name in names{i18n::append(&combo,name);}
        let selected=if key=="aspect"{match state.borrow().video_options[key].as_str(){Some("4:3")=>1,Some("16:9")=>2,Some("16:10")=>3,Some("5:4")=>4,_=>0}}else{state.borrow().video_options[key].as_u64().unwrap_or(0) as u32};
        let selected=if key=="decoder"&&!a865r_media::wsl_video::is_wsl(){selected.min(1)}else{selected};
        combo.set_active(Some(selected));place_setting(&video,&combo,246,y,450,33,scale);
        let st=state.clone();combo.connect_changed(move|combo|{
            if i18n::refreshing(){return;}let Some(index)=combo.active()else{return;};let mut s=st.borrow_mut();
            s.video_options[key]=if key=="aspect"{serde_json::json!(["-1","4:3","16:9","16:10","5:4"][index.min(4) as usize])}else{serde_json::json!(index)};
            if key=="decoder" {s.video_options["decoder_schema"]=serde_json::json!(1);}
            if key=="size" {s.video_options["size_schema"]=serde_json::json!(1);}
            s.save_config();
            if key=="decoder" {
                if s.job.as_ref().is_some_and(|job|job.kind==JobKind::Watch){s.watch();}
                else if let Some(path)=s.file_path.clone().filter(|_|s.file_player.is_some()){s.open_file(path);}
            } else {s.apply_picture();}
        });
    }
    let video_status=i18n::label("");video_status.set_line_wrap(true);video_status.set_xalign(0.);
    place_setting(&video,&video_status,26,308,670,76,scale);
    let st=state.clone();let weak=panel.downgrade();glib::timeout_add_local(Duration::from_millis(500),move||{
        if weak.upgrade().is_none(){return glib::ControlFlow::Break;}
        i18n::set_label(&video_status,&st.borrow().status);glib::ControlFlow::Continue
    });
    settings_label(&video,"Deinterlacing",26,208,190,scale);
    let modes=ComboBoxText::new();
    for name in ["Smooth • 50 / 59.94 fields per second","Standard • single rate","Off"]{
        i18n::append(&modes,name);
    }
    let selected=match state.borrow().deinterlacing{
        DeinterlaceMode::DoubleRate=>0,DeinterlaceMode::SingleRate=>1,DeinterlaceMode::Off=>2
    };
    modes.set_active(Some(selected));
    let st=state.clone();modes.connect_changed(move |combo|{if !i18n::refreshing(){state_mode(&mut st.borrow_mut(),combo.active());}});
    place_setting(&video,&modes,246,208,450,33,scale);
    tabs.add_named(&video,"Video");

    let channels=make_page();
    settings_label(&channels,"Country / scan profile",26,6,670,scale);
    let country=ComboBoxText::with_entry();
    {let s=state.borrow();
        for profile in &s.countries{
            country.append_text(profile["name"].as_str().unwrap_or("Country"));
        }
        country.set_active(Some(s.country as u32));
    }
    place_setting(&channels,&country,26,36,440,32,scale);
    let save_country=i18n::button("SAVE COUNTRY");
    place_setting(&channels,&save_country,486,32,210,34,scale);
    settings_label(&channels,"Channels",26,86,670,scale);
    let selected_service=ComboBoxText::new();
    {
        let s=state.borrow();
        if s.services.is_empty(){
            i18n::append(&selected_service,&format!("Live TV · {:.3} MHz",s.frequency_khz as f64/1000.));
            selected_service.set_active(Some(0));
        }else{
            for (index,service) in s.services.iter().enumerate(){
                selected_service.append_text(&format!("{} – {}",interaction::channel_number(Some(service),index),
                    service["name"].as_str().unwrap_or("TV service")));
            }
            selected_service.set_active(Some(s.service_index as u32));
        }
    }
    place_setting(&channels,&selected_service,26,114,670,32,scale);
    let updating_services=Rc::new(Cell::new(false));let guard=updating_services.clone();
    let st=state.clone();
    selected_service.connect_changed(move |combo|{
        if guard.get()||i18n::refreshing(){return;}
        if let Some(index)=combo.active(){st.borrow_mut().select_channel(index as usize);}
    });
    for (x,label) in [(26,"From (MHz)"),(261,"To (MHz)"),(496,"Step (MHz)")] {
        settings_label(&channels,label,x,166,200,scale);
    }
    let first=gtk::Entry::new();let last=gtk::Entry::new();let step=gtk::Entry::new();
    {let s=state.borrow();
        first.set_text(&format!("{:.3}",s.scan_first_khz as f64/1000.));
        last.set_text(&format!("{:.3}",s.scan_last_khz as f64/1000.));
        step.set_text(&format!("{:.3}",s.scan_step_khz as f64/1000.));
    }
    place_setting(&channels,&first,26,198,200,28,scale);
    place_setting(&channels,&last,261,198,200,28,scale);
    place_setting(&channels,&step,496,198,200,28,scale);
    let st=state.clone();let a=first.clone();let b=last.clone();let c=step.clone();
    country.connect_changed(move |combo|{
        let Some(index)=combo.active().map(|n|n as usize) else{return;};
        let mut s=st.borrow_mut();
        if index>=s.countries.len(){return;}
        s.country=index;
        let profile=s.countries[index].clone();
        let values=[
            profile["first"].as_u64().unwrap_or(473_143) as u32,
            profile["last"].as_u64().unwrap_or(695_143) as u32,
            profile["step"].as_u64().unwrap_or(6_000) as u32];
        a.set_text(&format!("{:.3}",values[0] as f64/1000.));
        b.set_text(&format!("{:.3}",values[1] as f64/1000.));
        c.set_text(&format!("{:.3}",values[2] as f64/1000.));
        s.scan_first_khz=values[0];s.scan_last_khz=values[1];s.scan_step_khz=values[2];
        s.status=countries::scan(&profile).err().unwrap_or_default();
        s.save_config();
    });
    let st=state.clone();let a=first.clone();let b=last.clone();let c=step.clone();
    let selector=country.clone();
    save_country.connect_clicked(move |_|{
        let name=selector.active_text().map(|s|s.trim().to_string()).unwrap_or_default();
        if name.is_empty()||name.len()>80{
            st.borrow_mut().status="Enter a country name (up to 80 characters).".into();return;
        }
        let parse=|entry:&gtk::Entry|entry.text().replace(',',".").parse::<f64>().ok()
            .filter(|v|v.is_finite()&&*v>=0.&&*v<=1000.)
            .map(|v|(v*1000.).round() as u32);
        let (Some(first),Some(last),Some(step))=(parse(&a),parse(&b),parse(&c))
            else{st.borrow_mut().status="Enter valid frequencies in MHz.".into();return;};
        let mut s=st.borrow_mut();
        let old=s.countries.iter().find(|p|p["name"].as_str()
            .is_some_and(|n|n.eq_ignore_ascii_case(&name)));
        let profile=serde_json::json!({"name":name,"first":first,"last":last,"step":step,
            "standard":old.map(|p|p["standard"].clone()).unwrap_or(serde_json::json!("ISDB-T")),
            "supported":old.map(|p|p["supported"].clone()).unwrap_or(serde_json::json!(true))});
        if let Err(error)=countries::scan(&profile){s.status=error;return;}
        let index=if let Some(index)=s.countries.iter().position(|p|p["name"].as_str()
            .is_some_and(|n|n.eq_ignore_ascii_case(&name))){
            s.countries[index]=profile;index
        }else{
            s.countries.push(profile);selector.append_text(&name);s.countries.len()-1
        };
        s.country=index;s.scan_first_khz=first;s.scan_last_khz=last;s.scan_step_khz=step;
        s.status="Country profile saved.".into();s.save_config();
        drop(s);
        selector.set_active(Some(index as u32));
        st.borrow_mut().status="Country profile saved.".into();
    });
    let scan=i18n::button("DISCOVER CHANNELS");
    let stop=i18n::button("■  STOP SCAN");
    place_setting(&channels,&scan,26,258,320,38,scale);
    place_setting(&channels,&stop,376,258,320,38,scale);
    let st=state.clone();let a=first.clone();let b=last.clone();let c=step.clone();
    scan.connect_clicked(move |_|{
        let mut s=st.borrow_mut();
        let values=[a.text(),b.text(),c.text()];
        let parsed=values.iter().map(|v|v.parse::<f64>().ok())
            .collect::<Option<Vec<_>>>();
        let Some(v)=parsed else{
            s.status="Enter valid scan frequencies in MHz.".into();return;
        };
        if let Err(error)=a865r::channel_plan::custom_scan(
            (v[0]*1000.).round() as u32,(v[1]*1000.).round() as u32,
            (v[2]*1000.).round() as u32){
            s.status=error.to_string();return;
        }
        s.scan_first_khz=(v[0]*1000.).round() as u32;
        s.scan_last_khz=(v[1]*1000.).round() as u32;
        s.scan_step_khz=(v[2]*1000.).round() as u32;
        s.scan();
    });
    let st=state.clone();stop.connect_clicked(move |_|{
        if let Some(job)=&st.borrow().job{job.control.stop();}
    });
    let scan_status=i18n::label(&state.borrow().status);
    scan_status.set_line_wrap(true);scan_status.set_xalign(0.);scan_status.set_yalign(0.);
    place_setting(&channels,&scan_status,26,320,670,76,scale);
    tabs.add_named(&channels,"Channels");

    let storage=make_page();
    settings_label(&storage,"Recording folder",26,14,670,scale);
    let record_path=Label::new(Some(&state.borrow().recording_folder.display().to_string()));
    record_path.set_xalign(0.);place_setting(&storage,&record_path,36,62,450,28,scale);
    let rec=i18n::button("Choose folder…");
    place_setting(&storage,&rec,516,54,180,36,scale);
    settings_label(&storage,"Snapshot folder",26,134,670,scale);
    let snap_path=Label::new(Some(&state.borrow().snapshot_folder.display().to_string()));
    snap_path.set_xalign(0.);place_setting(&storage,&snap_path,36,182,450,28,scale);
    let snap=i18n::button("Choose folder…");
    place_setting(&storage,&snap,516,174,180,36,scale);
    settings_label(&storage,"Recordings are saved as TS. Snapshots are saved as PNG.",26,236,670,scale);
    let st=state.clone();let p=panel.clone();let label=record_path.clone();
    rec.connect_clicked(move |_|{
        if let Some(path)=choose_folder(&p,"Choose recording folder"){
            label.set_text(&path.display().to_string());
            let mut s=st.borrow_mut();s.recording_folder=path;s.save_config();
        }
    });
    let st=state.clone();let p=panel.clone();let label=snap_path.clone();
    snap.connect_clicked(move |_|{
        if let Some(path)=choose_folder(&p,"Choose snapshot folder"){
            label.set_text(&path.display().to_string());
            let mut s=st.borrow_mut();s.snapshot_folder=path.clone();
            if let Ok(mut stream)=UnixStream::connect(&s.ipc){
                let command=serde_json::json!({"command":["set_property",
                    "screenshot-directory",path.display().to_string()]});
                let _=stream.write_all(format!("{command}\n").as_bytes());
            }
            s.save_config();
        }
    });
    tabs.add_named(&storage,"Storage");

    let themes=make_page();
    fixed_choice(&themes,"Main theme","Orbit",30,scale);
    settings_label(&themes,"Button variation",26,96,205,scale);
    let material=ComboBoxText::new();
    for name in ["Metal","Glass","Plastic"]{i18n::append(&material,name);}
    material.set_active(Some(state.borrow().button_material as u32));
    place_setting(&themes,&material,246,96,450,33,scale);
    let st=state.clone();
    material.connect_changed(move |combo|{
        if i18n::refreshing(){return;}
        if let Some(index)=combo.active(){
            let mut s=st.borrow_mut();
            s.button_material=(index as usize).min(2);
            s.skin.set_material(s.button_material);
            apply_material_css(s.button_material);
            s.save_config();
            RECEIVER_WINDOW.with(|slot|{if let Some(panel)=slot.borrow().as_ref(){panel.queue_draw();}});
        }
    });
    tabs.add_named(&themes,"Themes");

    let picture=make_page();
    settings_label(&picture,"Video preset",26,20,190,scale);
    let preset=ComboBoxText::new();
    for name in picture::PRESETS{i18n::append(&preset,name);}
    preset.set_active(Some(state.borrow().picture.index() as u32));
    place_setting(&picture,&preset,246,20,450,33,scale);
    let updating=Rc::new(Cell::new(false));
    let mut sliders:Vec<(DrawingArea,Rc<Cell<i32>>,Label)>=Vec::new();
    for i in 0..3{
        let y=76+i as i32*56;
        let value=match i{0=>state.borrow().picture.saturation,
            1=>state.borrow().picture.brightness,_=>state.borrow().picture.contrast};
        let label=i18n::label(&picture_label(i,value));label.set_xalign(0.);
        place_setting(&picture,&label,26,y,205,28,scale);
        let slider=DrawingArea::new();
        slider.add_events(gdk::EventMask::BUTTON_PRESS_MASK|
            gdk::EventMask::BUTTON1_MOTION_MASK|gdk::EventMask::POINTER_MOTION_MASK|gdk::EventMask::LEAVE_NOTIFY_MASK);
        let cell=Rc::new(Cell::new(value));
        let draw_value=cell.clone();
        slider.connect_draw(move |widget,c|{
            let a=widget.allocation();
            let _=c.save();
            c.scale(a.width() as f64/450.,a.height() as f64/32.);
            c.set_source_rgb(187./255.,162./255.,129./255.);
            c.set_line_width(2.);
            c.move_to(6.,16.);c.line_to(444.,16.);let _=c.stroke();
            let range=if i==1{(-100.,100.)}else{(0.,200.)};
            let pos=6.+(draw_value.get() as f64-range.0)/(range.1-range.0)*438.;
            let gradient=gtk::cairo::LinearGradient::new(0.,5.,0.,27.);
            gradient.add_color_stop_rgb(0.,236./255.,215./255.,157./255.);
            gradient.add_color_stop_rgb(1.,125./255.,93./255.,50./255.);
            let _=c.set_source(&gradient);
            c.rectangle(pos-6.,5.,12.,22.);let _=c.fill();
            let _=c.restore();
            glib::Propagation::Proceed
        });
        let st=state.clone();let lbl=label.clone();let chooser=preset.clone();
        let value_cell=cell.clone();let changing=updating.clone();
        slider.connect_button_press_event(move |widget,event|{
            let width=widget.allocation().width().max(1) as f64;
            let fraction=((event.position().0/width*450.-6.)/438.).clamp(0.,1.);
            let value=if i==1{(fraction*200.-100.).round() as i32}
                else{(fraction*200.).round() as i32};
            set_picture_slider(&st,i,value,&lbl,&chooser,widget,&value_cell,&changing);
            glib::Propagation::Stop
        });
        let st=state.clone();let lbl=label.clone();let chooser=preset.clone();
        let value_cell=cell.clone();let changing=updating.clone();
        slider.connect_motion_notify_event(move |widget,event|{
            if !event.state().contains(gdk::ModifierType::BUTTON1_MASK){
                return glib::Propagation::Proceed;
            }
            let width=widget.allocation().width().max(1) as f64;
            let fraction=((event.position().0/width*450.-6.)/438.).clamp(0.,1.);
            let value=if i==1{(fraction*200.-100.).round() as i32}
                else{(fraction*200.).round() as i32};
            set_picture_slider(&st,i,value,&lbl,&chooser,widget,&value_cell,&changing);
            glib::Propagation::Stop
        });
        place_setting(&picture,&slider,246,y,450,32,scale);
        sliders.push((slider,cell,label));
    }
    let st=state.clone();let changing=updating.clone();let controls=sliders.clone();
    preset.connect_changed(move |combo|{
        if changing.get()||i18n::refreshing(){return;}
        let Some(index)=combo.active() else{return;};
        if index>=4{return;}
        let mut s=st.borrow_mut();
        s.picture=Picture{hdr_effect:s.picture.hdr_effect,..Picture::preset(index as usize)};
        changing.set(true);
        for (i,((draw,cell,label),value)) in controls.iter().zip(
            [s.picture.saturation,s.picture.brightness,s.picture.contrast]).enumerate(){
            cell.set(value);draw.queue_draw();i18n::set_label(label,&picture_label(i,value));
        }
        changing.set(false);
        s.apply_picture();s.save_config();
    });
    settings_label(&picture,"HDR effect",26,244,190,scale);
    let effect=i18n::button(if state.borrow().picture.hdr_effect{"HDR EFFECT: ON"}else{"HDR EFFECT: OFF"});
    place_setting(&picture,&effect,246,244,450,34,scale);
    let st=state.clone();
    effect.connect_clicked(move |button|{
        let mut s=st.borrow_mut();
        s.picture.hdr_effect=!s.picture.hdr_effect;
        i18n::set_button(button,if s.picture.hdr_effect{"HDR EFFECT: ON"}else{"HDR EFFECT: OFF"});
        s.apply_picture();s.save_config();
    });
    let reset=i18n::button("RESET DEFAULTS");
    place_setting(&picture,&reset,326,386,180,33,scale);
    let st=state.clone();let changing=updating.clone();let controls=sliders.clone();
    let chooser=preset.clone();let effect_button=effect.clone();
    reset.connect_clicked(move |_|{
        let mut s=st.borrow_mut();s.picture=Picture::default();
        changing.set(true);
        for (i,((draw,cell,label),value)) in controls.iter().zip([100,0,100]).enumerate(){
            cell.set(value);draw.queue_draw();i18n::set_label(label,&picture_label(i,value));
        }
        chooser.set_active(Some(0));changing.set(false);
        i18n::set_button(&effect_button,"HDR EFFECT: OFF");
        s.apply_picture();s.save_config();
    });
    tabs.add_named(&picture,"Picture");

    let parental_page=make_page();
    let parental_draft=Rc::new(RefCell::new(state.borrow().parental.clone()));
    let has_password=!parental_draft.borrow()["password"].is_null();
    let auth=gtk::Entry::new();auth.set_visibility(false);i18n::placeholder(&auth,"Password");
    let unlock=i18n::button("Unlock");
    let new_password=gtk::Entry::new();new_password.set_visibility(false);
    let confirm=gtk::Entry::new();confirm.set_visibility(false);
    let enabled=ComboBoxText::new();
    let unrated=ComboBoxText::new();
    for combo in [&enabled,&unrated]{i18n::append(combo,"Off");i18n::append(combo,"On");}
    let age=ComboBoxText::new();
    for label in ["All ages","10","12","14","16","18"]{i18n::append(&age,label);}
    let channel=ComboBoxText::new();
    {
        let s=state.borrow();
        if s.services.is_empty(){i18n::append(&channel,"Scan for channels first");}
        else{for service in &s.services{
            let key=parental::channel_key(
                service["frequency_khz"].as_u64().unwrap_or(0) as u32,
                service["program_id"].as_u64().unwrap_or(0) as u32);
            let locked=parental_draft.borrow()["channels"].as_array()
                .is_some_and(|list|list.iter().any(|v|v.as_str()==Some(&key)));
            channel.append_text(&format!("{}{}",if locked{"[Locked] "}else{""},
                service["name"].as_str().unwrap_or("TV")));
        }}
    }
    channel.set_active(Some(0));
    let lock=i18n::button("Lock / unlock");
    let save=i18n::button("Save parental controls");
    let parental_status=i18n::label(if has_password{
        "Enter your password to edit or temporarily unlock."
    }else{"Set a password before enabling TV blocking."});
    parental_status.set_xalign(0.);parental_status.set_line_wrap(true);
    enabled.set_active(Some(u32::from(parental_draft.borrow()["enabled"]==true)));
    unrated.set_active(Some(u32::from(parental_draft.borrow()["block_unrated"]==true)));
    age.set_active(Some(parental::AGES.iter().position(|&n|
        parental_draft.borrow()["max_age"].as_u64()==Some(n as u64)).unwrap_or(5) as u32));
    for (label,y) in [("Password",12),("New password",60),("Confirm",108),
        ("TV blocking",156),("Maximum age",204),("Channel",252)]{
        settings_label(&parental_page,label,26,y,205,scale);
    }
    settings_label(&parental_page,"Block unrated",480,156,170,scale);
    place_setting(&parental_page,&auth,246,12,265,33,scale);
    place_setting(&parental_page,&unlock,520,12,176,33,scale);
    place_setting(&parental_page,&new_password,246,60,450,33,scale);
    place_setting(&parental_page,&confirm,246,108,450,33,scale);
    place_setting(&parental_page,&enabled,246,156,130,33,scale);
    place_setting(&parental_page,&unrated,600,156,96,33,scale);
    place_setting(&parental_page,&age,246,204,190,33,scale);
    place_setting(&parental_page,&channel,246,252,265,33,scale);
    place_setting(&parental_page,&lock,520,252,176,33,scale);
    place_setting(&parental_page,&save,246,308,450,37,scale);
    place_setting(&parental_page,&parental_status,26,366,670,65,scale);
    for widget in [&new_password,&confirm]{widget.set_sensitive(!has_password);}
    for widget in [&enabled,&unrated,&age,&channel]{widget.set_sensitive(!has_password);}
    lock.set_sensitive(!has_password&&!state.borrow().services.is_empty());
    save.set_sensitive(!has_password);
    auth.set_sensitive(has_password);unlock.set_sensitive(has_password);
    let st=state.clone();let status=parental_status.clone();
    let edit_widgets=[new_password.clone().upcast::<gtk::Widget>(),
        confirm.clone().upcast(),enabled.clone().upcast(),unrated.clone().upcast(),
        age.clone().upcast(),channel.clone().upcast(),save.clone().upcast()];
    let lock_button=lock.clone();let auth_entry=auth.clone();
    unlock.connect_clicked(move |_|{
        if parental::verify(&st.borrow().parental,&auth_entry.text()){
            st.borrow_mut().parental_unlocked=true;
            for widget in &edit_widgets{widget.set_sensitive(true);}
            lock_button.set_sensitive(!st.borrow().services.is_empty());
            i18n::set_label(&status,"Parental controls temporarily unlocked.");
        }else{i18n::set_label(&status,"Incorrect password.");}
        auth_entry.set_text("");
    });
    let st=state.clone();let draft=parental_draft.clone();let list=channel.clone();
    let status=parental_status.clone();
    lock.connect_clicked(move |_|{
        let Some(index)=list.active().map(|n|n as usize) else{return;};
        let s=st.borrow();
        let Some(service)=s.services.get(index) else{return;};
        let key=parental::channel_key(
            service["frequency_khz"].as_u64().unwrap_or(0) as u32,
            service["program_id"].as_u64().unwrap_or(0) as u32);
        let mut policy=draft.borrow_mut();
        let channels=policy["channels"].as_array_mut();
        if channels.is_none(){policy["channels"]=serde_json::json!([]);}
        let channels=policy["channels"].as_array_mut().unwrap();
        if let Some(pos)=channels.iter().position(|v|v.as_str()==Some(&key)){
            channels.remove(pos);i18n::set_label(&status,"Channel unlocked. Save to apply.");
        }else{
            channels.push(serde_json::json!(key));i18n::set_label(&status,"Channel locked. Save to apply.");
        }
    });
    let st=state.clone();let draft=parental_draft.clone();let status=parental_status.clone();
    let password=new_password.clone();let confirmation=confirm.clone();
    let auth_after_save=auth.clone();let unlock_after_save=unlock.clone();
    let controls=[new_password.clone().upcast::<gtk::Widget>(),confirm.clone().upcast(),
        enabled.clone().upcast(),unrated.clone().upcast(),age.clone().upcast(),
        channel.clone().upcast(),lock.clone().upcast()];
    save.connect_clicked(move |button|{
        let mut policy=draft.borrow().clone();
        let new=password.text().to_string();
        if !new.is_empty(){
            if new!=confirmation.text().as_str(){i18n::set_label(&status,"Passwords do not match.");return;}
            match parental::set_password(&new){
                Ok(value)=>policy["password"]=value,
                Err(error)=>{i18n::set_label(&status,&error);return;}
            }
        }
        let active=enabled.active()==Some(1);
        if active&&policy["password"].is_null(){
            i18n::set_label(&status,"Set and confirm a password first.");return;
        }
        policy["enabled"]=serde_json::json!(active);
        policy["block_unrated"]=serde_json::json!(unrated.active()==Some(1));
        policy["max_age"]=serde_json::json!(parental::AGES[age.active().unwrap_or(5).min(5) as usize]);
        {
            let mut s=st.borrow_mut();
            s.parental=policy.clone();s.parental_unlocked=false;s.save_config();
            if s.job.is_some(){
                if let Err(error)=s.parental_check(){
                    s.status=error;if let Some(job)=&s.job{job.control.stop();}
                }
            }
        }
        *draft.borrow_mut()=policy;
        password.set_text("");confirmation.set_text("");
        let protected=!draft.borrow()["password"].is_null();
        auth_after_save.set_sensitive(protected);unlock_after_save.set_sensitive(protected);
        for control in &controls{control.set_sensitive(!protected);}
        button.set_sensitive(!protected);
        i18n::set_label(&status,"Parental controls saved.");
    });
    tabs.add_named(&parental_page,"Parental");

    let general=make_page();
    settings_label(&general,"Language",26,32,210,scale);
    let language=ComboBoxText::new();
    for name in i18n::NAMES{language.append_text(name);}
    language.set_active(Some(i18n::index() as u32));
    place_setting(&general,&language,246,32,450,33,scale);
    let st=state.clone();
    language.connect_changed(move |combo|{
        let Some(index)=combo.active() else{return;};
        if index as usize==i18n::index(){return;}
        i18n::set(index as usize);
        st.borrow().save_config();
        i18n::refresh();
    });
    tabs.add_named(&general,"General");

    tabs.set_visible_child_name("Video");
    let stack=tabs.clone();let selection=selected_tab.clone();let art=fascia.clone();let p=panel.clone();
    fascia.connect_button_press_event(move |w,event|{
        let a=w.allocation();let (x,y)=event.position();
        let scale=a.width() as f64/750.;
        if x>=678.*scale && x<=728.*scale && y<=46.*scale {p.close();return glib::Propagation::Stop;}
        if let Some(index)=Skin::settings_tab_at(a.width(),x,y){
            selection.set(index);
            let name=["Video","Channels","Storage","Themes","Picture","Parental","General"][index];
            stack.set_visible_child_name(name);art.queue_draw();
        }
        glib::Propagation::Stop
    });
    let header=EventBox::new();header.set_visible_window(false);
    header.set_halign(gtk::Align::Fill);header.set_valign(gtk::Align::Start);
    header.set_size_request(1,(44.0*scale).round() as i32);
    header.add_events(gdk::EventMask::BUTTON_PRESS_MASK);
    let move_panel=panel.clone();let manual=manually_positioned.clone();
    header.connect_button_press_event(move |_,ev|{
        if ev.button()==1 && ev.event_type()==gdk::EventType::ButtonPress{
            manual.set(true);
            move_panel.begin_move_drag(1,ev.root().0 as i32,ev.root().1 as i32,ev.time());
        }
        glib::Propagation::Stop
    });
    overlay.add_overlay(&header);
    let close=Button::new();
    close.set_widget_name("orbit-close");
    let close_art=DrawingArea::new();
    close.add(&close_art);
    close.set_size_request((30.0*scale) as i32,(30.0*scale) as i32);
    close.set_halign(gtk::Align::End);close.set_valign(gtk::Align::Start);
    close.set_margin_end((34.0*scale) as i32);
    close.set_margin_top((12.0*scale) as i32);
    close.add_events(gdk::EventMask::BUTTON_PRESS_MASK);
    let st=state.clone();
    close_art.connect_draw(move |w,c|{
        let a=w.allocation();st.borrow().skin.draw_caption_close(c,a.width(),a.height());
        glib::Propagation::Proceed
    });
    let p=panel.clone();
    close.connect_clicked(move |_|p.close());
    overlay.add_overlay(&close);
    let drag=fascia.clone();let p=panel.clone();let manual=manually_positioned.clone();
    drag.add_events(gdk::EventMask::BUTTON_PRESS_MASK);
    drag.connect_button_press_event(move |_,ev|{
        if ev.button()==1 && ev.event_type()==gdk::EventType::ButtonPress && ev.position().1<41.{
            manual.set(true);
            p.begin_move_drag(ev.button() as i32,ev.root().0 as i32,ev.root().1 as i32,ev.time());
        }
        glib::Propagation::Proceed
    });
    panel.add(&overlay);
    panel.show_all();
    center_settings_on_open(&panel,&owner,manually_positioned);
    let weak=panel.downgrade();let st=state.clone();
    let mut listed_services=state.borrow().services.clone();
    glib::timeout_add_local(Duration::from_millis(500),move||{
        if weak.upgrade().is_none(){return glib::ControlFlow::Break;}
        let s=st.borrow();i18n::set_label(&scan_status,&s.status);
        if listed_services!=s.services{
            updating_services.set(true);selected_service.remove_all();
            for (index,service) in s.services.iter().enumerate(){selected_service.append_text(&format!("{} – {}",interaction::channel_number(Some(service),index),service["name"].as_str().unwrap_or("TV")));}
            selected_service.set_active(Some(s.service_index as u32));
            updating_services.set(false);listed_services=s.services.clone();
        }
        glib::ControlFlow::Continue
    });
}

fn state_mode(state:&mut State,index:Option<u32>){
    state.deinterlacing=match index{Some(0)=>DeinterlaceMode::DoubleRate,Some(1)=>DeinterlaceMode::SingleRate,_=>DeinterlaceMode::Off};
    state.apply_picture();state.save_config();
}
fn guide_dialog(state:Rc<RefCell<State>>,window:&ApplicationWindow){guide::show(state,window);}


thread_local! {
    static RECEIVER_WINDOW: RefCell<Option<gtk::Window>> = const { RefCell::new(None) };
}
impl State {
    fn toggle_fullscreen(&mut self,window:&ApplicationWindow) {
        self.fullscreen=!self.fullscreen;
        self.fullscreen_layout.set(self.fullscreen);
        if self.fullscreen {
            let (x,y)=client_position(window.upcast_ref());let (width,height)=window.size();
            let deck=RECEIVER_WINDOW.with(|slot|slot.borrow().as_ref().filter(|p|p.is_visible()).map(client_position));
            self.windowed=Some(WindowedPlacement{x,y,width,height,maximized:window.is_maximized(),deck});
            RECEIVER_WINDOW.with(|slot|{if let Some(panel)=slot.borrow().as_ref(){panel.hide();}});
            window.fullscreen();
            // Hiding the currently active DAC can leave WSLg's keyboard focus
            // on its withdrawn X window. Explicitly activate the viewer once.
            window.present();
        }else{
            window.unfullscreen();
            if let Some(WindowedPlacement{x,y,width,height,maximized,deck})=self.windowed.take() {
                // The WM processes unfullscreen asynchronously. Restore only
                // after it acknowledges the transition, with no new min size.
                let weak=window.downgrade();let fullscreen=self.fullscreen_layout.clone();
                let mut attempts=0;
                glib::timeout_add_local(Duration::from_millis(16),move||{
                    let Some(window)=weak.upgrade()else{return glib::ControlFlow::Break;};
                    if fullscreen.get(){return glib::ControlFlow::Break;}
                    attempts+=1;
                    if window.window().is_some_and(|w|w.state().contains(gdk::WindowState::FULLSCREEN)) && attempts<120 {
                        return glib::ControlFlow::Continue;
                    }
                    if maximized {window.maximize();}else{
                        window.unmaximize();window.resize(width,height);
                        place_mapped_window(window.upcast_ref(),x,y);
                    }
                    window.queue_resize();
                    if let Some((px,py))=deck {RECEIVER_WINDOW.with(|slot|{if let Some(panel)=slot.borrow().as_ref(){
                        panel.show_all();place_mapped_window(panel,px,py);
                    }});}
                    glib::ControlFlow::Break
                });
            }
        }
        window.queue_resize();window.queue_draw();
    }
}

// Overlay children must not contribute their current allocation as a minimum
// size. GTK3's out-rectangle signal lacks a generated gtk-rs wrapper.
fn video_overlay_layout(overlay:&gtk::Overlay,video:&EventBox,fullscreen:Rc<Cell<bool>>) {
    struct Layout { video:EventBox, fullscreen:Rc<Cell<bool>> }
    unsafe extern "C" fn position(overlay:*mut gtk::ffi::GtkOverlay,child:*mut gtk::ffi::GtkWidget,
        rect:*mut gdk::ffi::GdkRectangle,data:glib::ffi::gpointer)->glib::ffi::gboolean {
        let layout=&*(data as *const Layout);
        if child!=layout.video.as_ptr() as *mut gtk::ffi::GtkWidget{return 0;}
        let overlay:glib::translate::Borrowed<gtk::Overlay>=glib::translate::from_glib_borrow(overlay);
        let a=overlay.allocation();
        let (x,y,width,height)=if layout.fullscreen.get(){(0,0,a.width(),a.height())}
            else{Skin::video_rect(a.width(),a.height())};
        *rect=gdk::ffi::GdkRectangle{x,y,width,height};1
    }
    unsafe {
        glib::signal::connect_raw(overlay.as_ptr() as *mut _,c"get-child-position".as_ptr(),
            Some(std::mem::transmute::<*const (),unsafe extern "C" fn()>(position as *const ())),
            Box::into_raw(Box::new(Layout{video:video.clone(),fullscreen})));
    }
}
// Port of GUI/Windows/src/window_placement.rs in GTK logical pixels.
fn startup_pair()->[(i32,i32,i32,i32);2]{
    let mut work=gdk::Display::default().and_then(|d|d.primary_monitor())
        .map(|m|m.workarea()).unwrap_or_else(||gdk::Rectangle::new(0,0,960,540));
    // WSLg can briefly report a 320x240 monitor while XRandR still exposes
    // the real desktop mode. Read that mode before placing either window.
    if std::env::var_os("WSL_DISTRO_NAME").is_some() ||
        work.width()<500||work.height()<400 {
        let mode=Command::new("xrandr").arg("--current").output().ok()
            .and_then(|output|String::from_utf8(output.stdout).ok())
            .and_then(|output|{
                let words:Vec<_>=output.split_whitespace().collect();
                let i=words.iter().position(|word|*word=="current")?;
                Some((words.get(i+1)?.parse::<i32>().ok()?,
                    words.get(i+3)?.trim_end_matches(',').parse::<i32>().ok()?))
            });
        let scale=std::env::var("GDK_SCALE").ok()
            .and_then(|s|s.parse::<i32>().ok()).unwrap_or(1).max(1);
        let (width,height)=match mode {
            Some((w,h)) if std::env::var_os("WSL_DISTRO_NAME").is_some()
                && (w<960 || h<540) => (1920,1080),
            Some(size) => size,
            None if std::env::var_os("WSL_DISTRO_NAME").is_some() => (1920,1080),
            None => (1024,768),
        };
        work=gdk::Rectangle::new(0,0,width/scale,height/scale);
    }
    if wsl_hidpi() && (work.width()<960||work.height()<540) {
        // WSLg can publish its 640x480 bootstrap mode during GTK activation.
        work=gdk::Rectangle::new(0,0,960,540);
    }
    let margin=16;
    let width=(work.width()-2*margin).max(2);
    let height=(work.height()-2*margin).max(3);
    let scale=0.9*1_f64.min(width as f64/1120.).min(height as f64/1048.);
    let px=|n:f64|(n*scale).floor().max(1.) as i32;
    let (vw,vh,dw,dh,overlap)=(px(1120.),px(770.),px(1088.),px(354.),px(76.));
    let top=work.y()+(work.height()-vh+overlap-dh)/2;
    eprintln!("Live TV work area {}x{}, startup viewer {}x{}, DAC {}x{}",work.width(),work.height(),vw,vh,dw,dh);
    [(work.x()+(work.width()-vw)/2,top,vw,vh),
     (work.x()+(work.width()-dw)/2,top+vh-overlap,dw,dh)]
}
fn client_position(window:&gtk::Window)->(i32,i32) {
    window.window().map(|w|{let (_,x,y)=w.origin();(x,y)}).unwrap_or_else(||window.position())
}
fn center_settings_on_open(panel:&gtk::Window,owner:&gtk::Window,manually_positioned:Rc<Cell<bool>>){
    let panel=panel.downgrade();let owner=owner.downgrade();
    let mut attempts=0;
    // WSLg may finish mapping after show_all. Settle the initial placement only;
    // a permanent centering timer fights the window manager during user drags.
    glib::timeout_add_local(Duration::from_millis(100),move||{
        let (Some(panel),Some(owner))=(panel.upgrade(),owner.upgrade())else{return glib::ControlFlow::Break;};
        if !panel.is_visible()||manually_positioned.get(){return glib::ControlFlow::Break;}
        attempts+=1;
        if panel.is_mapped()&&owner.is_mapped(){
            let (x,y)=client_position(&owner);let (w,h)=owner.size();let (pw,ph)=panel.size();
            let (mut tx,mut ty)=(x+(w-pw)/2,y+(h-ph)/2);
            if let Some(surface)=owner.window(){if let Some(monitor)=owner.display().monitor_at_window(&surface){
                let area=monitor.workarea();
                tx=tx.clamp(area.x(),area.x()+(area.width()-pw).max(0));
                ty=ty.clamp(area.y(),area.y()+(area.height()-ph).max(0));
            }}
            let (px,py)=client_position(&panel);let (wx,wy)=panel.position();
            if (px-tx).abs()<=1&&(py-ty).abs()<=1{return glib::ControlFlow::Break;}
            panel.move_(wx+tx-px,wy+ty-py);
        }
        if attempts>=10{glib::ControlFlow::Break}else{glib::ControlFlow::Continue}
    });
}
fn place_mapped_window(window:&gtk::Window,x:i32,y:i32) {
    let weak=window.downgrade();let mut attempts=0;
    // WSLg's initial map/owner-centering arrives after show_all/move_. Apply
    // placement after mapping, in client coordinates (excluding WM shadows).
    glib::timeout_add_local(Duration::from_millis(100),move||{
        let Some(window)=weak.upgrade()else{return glib::ControlFlow::Break;};
        attempts+=1;
        if !window.is_visible(){return glib::ControlFlow::Break;}
        if let Some(surface)=window.window().filter(|_|window.is_mapped()) {
            let (_,cx,cy)=surface.origin();let (wx,wy)=window.position();
            if attempts>1&&(cx-x).abs()<=1&&(cy-y).abs()<=1{return glib::ControlFlow::Break;}
            window.move_(wx+x-cx,wy+y-cy);
        }
        if attempts>=10{glib::ControlFlow::Break}else{glib::ControlFlow::Continue}
    });
}
// Raise the visible pair without moving keyboard focus or restoring hidden windows.
// This app uses X11 (also under Ubuntu's Wayland session via XWayland).
fn raise_visible_pair(active:&gtk::Window,peer:&gtk::Window) {
    let visible=|w:&gtk::Window|w.is_visible() && w.is_mapped() && w.window()
        .is_some_and(|s|!s.state().intersects(gdk::WindowState::ICONIFIED|gdk::WindowState::WITHDRAWN));
    if visible(active) && visible(peer) {
        if let Some(surface)=peer.window(){surface.raise();}
        if let Some(surface)=active.window(){surface.raise();}
    }
}
fn link_window_activation(viewer:&ApplicationWindow,deck:&gtk::Window) {
    if std::env::var_os("WSL_DISTRO_NAME").is_some(){return;}
    let weak=deck.downgrade();
    viewer.connect_focus_in_event(move |viewer,_|{
        if let Some(deck)=weak.upgrade(){raise_visible_pair(viewer.upcast_ref(),&deck);}
        glib::Propagation::Proceed
    });
    let weak=viewer.downgrade();
    deck.connect_focus_in_event(move |deck,_|{
        if let Some(viewer)=weak.upgrade(){raise_visible_pair(deck,viewer.upcast_ref());}
        glib::Propagation::Proceed
    });
}
fn receiver_dialog(state:Rc<RefCell<State>>,window:&ApplicationWindow) {
    if let Some(existing)=RECEIVER_WINDOW.with(|slot|slot.borrow().clone()) {
        existing.show_all();
        existing.present();
        return;
    }
    let panel=gtk::Window::new(gtk::WindowType::Toplevel);
    // Match Windows: two independent top-level windows. Making the DAC a
    // transient popup causes WSLg/Windows to redirect viewer activation to it.
    panel.set_focus_on_map(false);
    link_window_activation(window,&panel);
    let weak=panel.downgrade();
    window.connect_destroy(move |_|{if let Some(panel)=weak.upgrade(){panel.close();}});
    RECEIVER_WINDOW.with(|slot|*slot.borrow_mut()=Some(panel.clone()));
    transparent_panel(&panel);
    round_window(&panel,12);
    i18n::title(&panel,"Live TV! · Receiver");
    panel.set_decorated(false);
    let (dx,dy,dw,dh)=startup_pair()[1];
    panel.set_default_size(dw,dh);
    panel.set_resizable(false);
    let fascia=DrawingArea::new();
    fascia.set_size_request(dw,dh);
    fascia.add_events(gdk::EventMask::BUTTON_PRESS_MASK|gdk::EventMask::BUTTON_RELEASE_MASK|gdk::EventMask::BUTTON1_MOTION_MASK|gdk::EventMask::POINTER_MOTION_MASK|gdk::EventMask::LEAVE_NOTIFY_MASK);
    let dial_drag=Rc::new(RefCell::new(None::<(f64,f64)>));
    let st=state.clone();
    fascia.connect_draw(move |w,c|{
        let a=w.allocation();let s=st.borrow();
        let channel=s.services.get(s.service_index)
            .and_then(|v|v["name"].as_str()).unwrap_or("Live TV");
        let playing=s.file_player.is_some()||s.job.as_ref().is_some_and(|j|j.kind==JobKind::Watch);
        let recording=s.recording_path.is_some()||s.job.as_ref().is_some_and(|j|j.kind==JobKind::Record);
        let quality=s.job.as_ref().and_then(|job|job.control.snapshot()["signal_quality_percent"].as_u64())
            .map(|value|value.min(100) as u8);
        s.skin.draw_deck(c,a.width(),a.height(),channel,s.services.get(s.service_index).and_then(|v|v["channel_number"].as_u64()).unwrap_or(s.service_index as u64+1) as usize,
            &s.status,playing,recording,s.volume,s.paused,s.captions,s.audio_mode,quality);
        glib::Propagation::Proceed
    });
    let st=state.clone();let win=window.clone();let p=panel.clone();let drag=dial_drag.clone();
    fascia.connect_button_press_event(move |w,ev|{
        let a=w.allocation();let (x,y)=ev.position(); if std::env::var_os("OPEN_VOLAR_S_UI_TRACE").is_some(){eprintln!("DAC input {x} {y} {:?}",Skin::dac_hit(a.width(),a.height(),x,y));}
        if ev.button()!=1 || ev.event_type()!=gdk::EventType::ButtonPress{return glib::Propagation::Stop;}
        if let Some(angle)=Skin::dac_dial_angle(a.width(),a.height(),x,y,true){
            *drag.borrow_mut()=Some((angle,st.borrow().volume as f64));
        }else if let Some(action)=Skin::dac_hit(a.width(),a.height(),x,y){
            match action{
                Action::Close=>p.hide(),
                Action::Minimize=>p.iconify(),
                Action::Audio=>controls::audio_menu(st.clone(),&win,true),
                _=>controls::dispatch(action,st.clone(),&win),
            }
        }else if y<80.0*a.height() as f64/547.0{
            p.begin_move_drag(ev.button() as i32,ev.root().0 as i32,ev.root().1 as i32,ev.time());
        }
        w.queue_draw();glib::Propagation::Stop
    });
    let st=state.clone();let drag=dial_drag.clone();
    fascia.connect_motion_notify_event(move |w,ev|{
        let a=w.allocation();let (x,y)=ev.position();
        st.borrow().skin.set_hover(Skin::dac_hit(a.width(),a.height(),x,y));w.queue_draw();
        if ev.state().contains(gdk::ModifierType::BUTTON1_MASK){
            let mut drag=drag.borrow_mut();
            if let Some((previous,value))=drag.as_mut(){
                let a=w.allocation();let (x,y)=ev.position(); if std::env::var_os("OPEN_VOLAR_S_UI_TRACE").is_some(){eprintln!("DAC input {x} {y} {:?}",Skin::dac_hit(a.width(),a.height(),x,y));}
                if let Some(angle)=Skin::dac_dial_angle(a.width(),a.height(),x,y,false){
                    *value=interaction::drag_volume(*value as f32,*previous as f32,angle as f32) as f64;*previous=angle;
                    st.borrow_mut().set_volume(value.round() as i32,false);w.queue_draw();
                }
            }
        }
        glib::Propagation::Stop
    });
    let st=state.clone();
    fascia.connect_leave_notify_event(move |w,_|{st.borrow().skin.set_hover(None);w.queue_draw();glib::Propagation::Proceed});
    let st=state.clone();let drag=dial_drag;
    fascia.connect_button_release_event(move |_,_|{
        if drag.borrow_mut().take().is_some(){st.borrow().save_config();}
        glib::Propagation::Stop
    });
    controls::connect_keys(&panel,state.clone(),window);
    panel.add(&fascia);panel.show_all();
    panel.move_(dx,dy);
    place_mapped_window(&panel,dx,dy);
    if let Some(surface)=panel.window(){surface.raise();}
    let weak=panel.downgrade();
    glib::timeout_add_local(Duration::from_millis(1000),move||{
        if weak.upgrade().is_none(){return glib::ControlFlow::Break;}
        fascia.queue_draw();glib::ControlFlow::Continue
    });
}

fn round_window(window:&gtk::Window,radius:i32){
    // RGBA visuals preserve the antialiasing in the original Windows PNG.
    // WSLg also needs an X11 bounding shape: alpha alone can leave opaque corner pixels.
    if std::env::var_os("WSL_DISTRO_NAME").is_none() && gtk::prelude::WidgetExt::screen(window)
        .is_some_and(|screen|screen.is_composited() && screen.rgba_visual().is_some()){return;}
    window.connect_size_allocate(move |widget,a|{
        let width=a.width();let height=a.height();
        if width<=0||height<=0{return;}
        let r=radius.min(width/2).min(height/2);
        let region=gtk::cairo::Region::create();
        let _=region.union_rectangle(&gtk::cairo::RectangleInt::new(
            0,r,width,height-2*r));
        for y in 0..r {
            let dy=r as f64-y as f64-0.5;
            let inset=(r as f64-(r as f64*r as f64-dy*dy).sqrt()).ceil() as i32;
            let span=(width-2*inset).max(0);
            let _=region.union_rectangle(&gtk::cairo::RectangleInt::new(
                inset,y,span,1));
            let _=region.union_rectangle(&gtk::cairo::RectangleInt::new(
                inset,height-y-1,span,1));
        }
        widget.shape_combine_region(Some(&region));
    });
}

fn wsl_hidpi()->bool{
    std::env::var_os("WSL_DISTRO_NAME").is_some()
        && std::env::var("GDK_SCALE").ok().as_deref()==Some("2")
}
fn transparent_panel(window:&gtk::Window){
    window.style_context().add_class("orbit-window");
    window.set_app_paintable(true);
    if let Some(visual)=gtk::prelude::WidgetExt::screen(window).and_then(|screen|screen.rgba_visual()){
        window.set_visual(Some(&visual));
    }
}

fn activate(app:&Application,initial:Option<u32>)->(ApplicationWindow,Rc<RefCell<State>>,EventBox){
    add_css();
    let window=ApplicationWindow::new(app);
    window.set_title("Live TV! — Open Volar S");
    transparent_panel(window.upcast_ref());
    round_window(window.upcast_ref(),4);
    let (vx,vy,vw,vh)=startup_pair()[0];
    window.set_default_size(vw,vh);
    window.set_position(gtk::WindowPosition::None);
    window.set_size_request((vw/2).max(320),(vh/2).max(220));
    window.set_decorated(false);
    let overlay=gtk::Overlay::new();
    let area=DrawingArea::new();area.set_size_request(1,1);
    area.add_events(gdk::EventMask::BUTTON_PRESS_MASK|gdk::EventMask::BUTTON_RELEASE_MASK|
        gdk::EventMask::BUTTON1_MOTION_MASK|gdk::EventMask::POINTER_MOTION_MASK);
    let video=EventBox::new();video.set_visible_window(true);
    video.set_app_paintable(true);
    video.connect_draw(|widget,c|{
        let a=widget.allocation();
        c.set_operator(gtk::cairo::Operator::Source);
        c.set_source_rgb(0.,0.,0.);
        c.rectangle(0.,0.,a.width() as f64,a.height() as f64);
        let _=c.fill();
        glib::Propagation::Proceed
    });
    video.set_size_request(1,1);
    overlay.add(&area);overlay.add_overlay(&video);
    window.add(&overlay);
    let state=Rc::new(RefCell::new(State::new(initial)));
    video_overlay_layout(&overlay,&video,state.borrow().fullscreen_layout.clone());
    apply_material_css(state.borrow().button_material);
    let st=state.clone();
    area.connect_draw(move |widget,c|{
        let a=widget.allocation();let state=st.borrow();
        let playing=state.file_player.is_some()||state.job.as_ref().is_some_and(|j|j.kind==JobKind::Watch);
        let recording=state.recording_path.is_some()||state.job.as_ref().is_some_and(|j|j.kind==JobKind::Record);
        let live=state.job.as_ref().is_some_and(|job|job.kind==JobKind::Watch);
        state.skin.draw(c,a.width(),a.height(),DrawState{playing,recording,live,status:&state.status,
            paused:state.paused,following_live:recording&&state.following_live,position:state.timeline_position,
            duration:state.timeline_duration,seekable:state.timeline_seekable,drag:state.seek_drag});
        glib::Propagation::Proceed
    });
    let st=state.clone();let win=window.clone();
    area.connect_button_press_event(move |widget,event|{
        if event.button()!=1 || event.event_type()!=gdk::EventType::ButtonPress{return glib::Propagation::Stop;}
        let a=widget.allocation();let (x,y)=event.position();
        if let Some(fraction)=Skin::seek_fraction(a.width(),a.height(),x,y){
            if st.borrow().timeline_seekable{st.borrow_mut().seek_drag=Some(fraction);widget.queue_draw();}
            return glib::Propagation::Stop;
        }
        if let Some(action)=Skin::hit(a.width(),a.height(),x,y){
            controls::dispatch(action,st.clone(),&win);
        }else if y<51.*a.width() as f64/1600.{
            win.begin_move_drag(event.button() as i32,event.root().0 as i32,event.root().1 as i32,event.time());
        }
        widget.queue_draw();glib::Propagation::Stop
    });
    let st=state.clone();
    area.connect_motion_notify_event(move |widget,event|{
        if st.borrow().seek_drag.is_some(){
            let a=widget.allocation();let (x,y)=event.position();
            if event.state().contains(gdk::ModifierType::BUTTON1_MASK){
                if let Some(fraction)=Skin::seek_fraction(a.width(),a.height(),x,y){
                    st.borrow_mut().seek_drag=Some(fraction);widget.queue_draw();
                }
            }
        }
        glib::Propagation::Stop
    });
    let st=state.clone();
    area.connect_button_release_event(move |widget,event|{
        if event.button()==1{
            let mut state=st.borrow_mut();
            if let Some(fraction)=state.seek_drag.take(){state.seek_fraction(fraction);widget.queue_draw();}
        }
        glib::Propagation::Stop
    });
    controls::connect_keys(&window,state.clone(),&window);
    let st=state.clone();window.connect_delete_event(move|_,_|{let mut s=st.borrow_mut();s.stop_file();if let Some(job)=&s.job{job.control.stop();}glib::Propagation::Proceed});
    window.show_all();
    window.move_(vx,vy);
    place_mapped_window(window.upcast_ref(),vx,vy);
    controls::install_osd(state.clone(),&window,&video);
    captions::install(state.clone(),&window);
    if std::env::var_os("OPEN_VOLAR_S_PREVIEW_SETTINGS").is_some(){
        settings_dialog(state.clone(),&window);
    }else{
        receiver_dialog(state.clone(),&window);
    }
    state.borrow_mut().window_id=video.window().map(|w|unsafe{
        gdk_x11_window_get_xid((w.to_glib_none() as gtk::glib::translate::Stash<'_,*mut gtk::gdk::ffi::GdkWindow,gtk::gdk::Window>).0 as *mut c_void)
    }).unwrap_or(0);
    if initial.is_some(){state.borrow_mut().watch();}
    let st=state.clone();let draw=area.clone();
    glib::timeout_add_local(Duration::from_millis(1000),move||{
        st.borrow_mut().poll();draw.queue_draw();glib::ControlFlow::Continue
    });
    (window,state,video)
}
pub fn run(){
    register_selawik();
    std::env::set_var("GDK_BACKEND","x11");
    if std::env::var_os("WSL_DISTRO_NAME").is_some() && std::env::var_os("GDK_SCALE").is_none(){
        std::env::set_var("GDK_SCALE","1");
    }
    let initial=std::env::args().nth(1).and_then(|s|s.parse::<u32>().ok());
    let app=Application::new(Some("org.openvolars.livetv"),Default::default());
    app.connect_activate(move |app|{activate(app,initial);});
    app.run_with_args(&["open-volar-s-live-tv"]);
}







#[cfg(test)]mod ui_tests {
    use super::*;
    #[test]fn channel_selection_queues_playback_and_ignores_stopping_service(){
        let path=std::env::temp_dir().join(format!("ovs-channel-switch-{}.json",std::process::id()));
        let mut s=State::new_at(None,path.clone());
        s.services=vec![
            serde_json::json!({"program_id":1,"frequency_khz":521143,"channel_number":4,"name":"Old channel"}),
            serde_json::json!({"program_id":2,"frequency_khz":557143,"channel_number":8,"name":"New channel"}),
        ];s.service_index=0;
        let control=Control::default();control.status("Live TV! · 521.143 MHz");
        let mut old=s.services[0].clone();old["name"]=serde_json::json!("Old channel updated");
        control.set_scan_services(serde_json::json!([old]));control.set_service(s.services[0].clone());
        let (_sender,result)=mpsc::channel();
        s.job=Some(Job{control:control.clone(),result,kind:JobKind::Watch});
        s.select_channel(1);
        assert!(control.cancel.load(AtomicOrdering::Relaxed));
        assert!(matches!(s.pending,Some((TvAction::Watch{frequency:557143},JobKind::Watch))));
        s.poll();
        assert_eq!(s.service_index,1);assert_eq!(s.current_program_id(),2);
        assert_eq!(s.frequency_khz,557143);
        // A second selection during shutdown replaces the queued channel.
        s.select_channel(0);s.poll();
        assert!(matches!(s.pending,Some((TvAction::Watch{frequency:521143},JobKind::Watch))));
        assert_eq!(s.current_program_id(),1);
        drop(s);let _=std::fs::remove_file(path);
    }
    #[test]fn independent_windows_restore_and_shrink_after_fullscreen() {
        std::env::set_var("GDK_BACKEND","x11");
        gtk::init().expect("GTK display required; run this test under xvfb-run");
        let app=Application::new(Some("org.openvolars.window-regression"),Default::default());
        app.register(None::<&gtk::gio::Cancellable>).unwrap();
        let (viewer,state,video)=activate(&app,None);
        let pump=|millis|{
            let until=Instant::now()+Duration::from_millis(millis);
            while Instant::now()<until {
                while glib::MainContext::default().iteration(false){}
                std::thread::sleep(Duration::from_millis(5));
            }
        };
        pump(1200);
        let deck=RECEIVER_WINDOW.with(|slot|slot.borrow().as_ref().unwrap().clone());
        assert!(deck.transient_for().is_none(),"DAC must not redirect activation from the viewer");
        assert!(!deck.gets_focus_on_map());
        let original=viewer.size();
        let (_,vx,vy)=viewer.window().unwrap().origin();
        let (_,dx,dy)=deck.window().unwrap().origin();
        assert!((2*vx+original.0-(2*dx+deck.size().0)).abs()<=2,"viewer {vx},{vy} {original:?}, DAC {dx},{dy} {:?}",deck.size());
        let overlap=vy+original.1-dy;
        assert!(overlap>0&&overlap<original.1/8,"Unexpected overlap {overlap}");
        for _ in 0..2 {
            state.borrow_mut().handle(Action::Fullscreen,&viewer);
            assert!(!deck.is_visible());
            // Xvfb has no WM. Supply its fullscreen allocation explicitly;
            // exercising the real GTK hierarchy catches the former min-size loop.
            viewer.resize(1600,1000);pump(150);
            assert_eq!((video.allocated_width(),video.allocated_height()),viewer.size());
            state.borrow_mut().handle(Action::Fullscreen,&viewer);pump(300);
            assert_eq!(viewer.size(),original,"Fullscreen must not become a minimum size");
            assert!(deck.is_visible());
            let (_,_,w,h)=Skin::video_rect(original.0,original.1);
            assert_eq!((video.allocated_width(),video.allocated_height()),(w,h));
        }
        viewer.resize(original.0-80,original.1-50);pump(150);
        assert_eq!(viewer.size(),(original.0-80,original.1-50));
        let pos=viewer.position();viewer.move_(pos.0+30,pos.1+20);pump(150);
        assert_eq!(viewer.position(),(pos.0+30,pos.1+20));
        viewer.close();pump(100);
    }
    fn descendants(widget:&gtk::Widget)->Vec<gtk::Widget>{
        let mut found=vec![widget.clone()];
        if let Some(container)=widget.downcast_ref::<gtk::Container>(){for child in container.children(){found.extend(descendants(&child));}}
        found
    }
    /// Uses an existing broadcast fixture and the real embedded native helper.
    #[test]
    #[ignore = "Requires desktop Vulkan Video/audio and A865R_TEST_TS; run with --ignored"]
    fn native_embedded_playback_controls(){
        std::env::set_var("GDK_BACKEND","x11");gtk::init().unwrap();register_selawik();
        let fixture=PathBuf::from(std::env::var_os("A865R_TEST_TS").expect("Set A865R_TEST_TS to a >=10-second broadcast TS"));
        let app=Application::new(Some("org.openvolars.native-regression"),Default::default());
        app.register(None::<&gtk::gio::Cancellable>).unwrap();
        let (viewer,state,_)=activate(&app,None);
        let pump=|ms|{let end=Instant::now()+Duration::from_millis(ms);while Instant::now()<end{while glib::MainContext::default().iteration(false){}thread::sleep(Duration::from_millis(5));}};
        pump(500);
        {let mut s=state.borrow_mut();assert!(s.window_id!=0);s.volume=0;s.video_options=serde_json::json!({"color":"off","size":1,"size_schema":1,"aspect":"4:3","decoder":0,"decoder_schema":1});s.open_file(fixture);}
        let get=|name:&str|state.borrow().player_request(serde_json::json!({"command":["get_property",name]}));
        for _ in 0..100{pump(50);if get("native-video-pos").ok().and_then(|v|v.as_f64()).is_some_and(|p|p>0.1){break;}}
        assert!(get("native-backend").unwrap().as_str().unwrap().contains("Vulkan Video"));
        assert!(get("native-video-pos").unwrap().as_f64().is_some_and(|p|p>0.1));
        {let mut s=state.borrow_mut();s.handle(Action::Play,&viewer);assert!(s.paused);s.picture=Picture::preset(2);s.apply_picture();assert!(s.picture_applied,"{}",s.status);s.update_timeline();assert!(s.timeline_seekable);assert!(s.timeline_duration>10.);}
        pump(150);let position=get("time-pos").unwrap().as_f64().unwrap();pump(250);
        assert!((get("time-pos").unwrap().as_f64().unwrap()-position).abs()<0.03);
        assert_eq!(get("picture").unwrap(),state.borrow().picture.json());assert_eq!(get("output-size").unwrap(),1);
        assert!((get("video-aspect-override").unwrap().as_f64().unwrap()-4./3.).abs()<0.001);
        let check_ratio=|expected:f64|{let d=get("osd-dimensions").unwrap();let w=d["w"].as_f64().unwrap()-d["ml"].as_f64().unwrap()-d["mr"].as_f64().unwrap();let h=d["h"].as_f64().unwrap()-d["mt"].as_f64().unwrap()-d["mb"].as_f64().unwrap();assert!((w/h-expected).abs()<0.004,"Wrong display proportions: {d}");};
        check_ratio(4./3.);
        {let mut s=state.borrow_mut();s.video_options["aspect"]=serde_json::json!("-1");s.video_options["size"]=serde_json::json!(2);s.apply_picture();}
        pump(300);check_ratio(16./9.);
        viewer.resize(800,800);pump(300);check_ratio(16./9.);
        let screenshot=state.borrow().ipc.with_extension("png");
        state.borrow().player_request(serde_json::json!({"command":["screenshot-to-file",screenshot,"window"]})).unwrap();
        for _ in 0..100{pump(20);if std::fs::File::open(&screenshot).ok().is_some_and(|mut f|gtk::cairo::ImageSurface::create_from_png(&mut f).is_ok()){break;}}
        assert!(std::fs::metadata(&screenshot).unwrap().len()>10000,"Embedded screenshot must contain video");
        {let mut s=state.borrow_mut();s.seek_fraction(0.5);}
        for _ in 0..100{pump(30);if get("native-video-pos").unwrap().as_f64().is_some_and(|p|p>5.){break;}}
        assert!(get("native-video-pos").unwrap().as_f64().is_some_and(|p|p>5.),"GUI seek did not render a new frame");
        {let mut s=state.borrow_mut();s.save_config();let loaded=State::new_at(None,s.config_path.clone());assert_eq!(loaded.picture,s.picture);assert_eq!(loaded.video_options,s.video_options);s.stop_file();let _=std::fs::remove_file(&s.config_path);}
        viewer.close();pump(100);println!("Embedded video snapshot: {}",screenshot.display());
    }
    #[test]fn recording_bar_tracks_duration_and_seeks(){
        let mut s=State::new(None);
        s.video_options=serde_json::json!({"color":"off","decoder":3});
        s.deinterlacing=DeinterlaceMode::Off;
        let sample=s.ipc.with_extension("mp4");
        let status=Command::new("ffmpeg").args(["-v","error","-f","lavfi","-i",
            "testsrc2=size=160x90:rate=10","-t","8","-c:v","mpeg4","-q:v","8","-y"])
            .arg(&sample).status().unwrap();assert!(status.success());
        // This test exercises timeline IPC, independent of codec/GPU support.
        s.file_player=Some(Command::new("mpv").args(["--no-config","--vo=null","--ao=null"])
            .arg(format!("--input-ipc-server={}",s.ipc.display())).arg(&sample)
            .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().unwrap());
        for _ in 0..100{
            std::thread::sleep(Duration::from_millis(50));
            s.update_timeline();if s.timeline_seekable{break;}
        }
        assert!(s.timeline_seekable,"{}",s.status);
        assert!(s.timeline_duration>=7.,"duration {}",s.timeline_duration);
        s.seek_fraction(0.5);
        std::thread::sleep(Duration::from_millis(250));s.update_timeline();
        assert!(s.timeline_position>=3.,"position {}",s.timeline_position);
        s.stop_file();let _=std::fs::remove_file(sample);
    }
    #[test]fn active_recording_can_seek_back_without_stopping_capture(){
        let mut state=State::new(None);
        state.services=vec![serde_json::json!({"program_id":1})];state.service_index=0;
        state.recording_folder=std::env::temp_dir().join(format!("ovs-live-seek-{}",std::process::id()));
        let sample=state.ipc.with_extension("ts");
        let status=Command::new("ffmpeg").args(["-v","error","-f","lavfi","-i",
            "testsrc2=size=160x90:rate=25","-f","lavfi","-i","sine=frequency=1000",
            "-t","20","-c:v","mpeg2video","-b:v","1M","-c:a","mp2","-f","mpegts","-y"])
            .arg(&sample).status().unwrap();assert!(status.success());
        let bytes=std::fs::read(&sample).unwrap();
        let mut child=Command::new("mpv").args(["--no-config","--vo=null","--ao=null",
            "--cache=yes","--cache-secs=2","--demuxer-seekable-cache=yes"])
            .arg(format!("--input-ipc-server={}",state.ipc.display()))
            .arg("-").stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().unwrap();
        let mut input=child.stdin.take().unwrap();
        let control=Control::default();
        let (sender,result)=mpsc::channel();
        state.job=Some(Job{control:control.clone(),result,kind:JobKind::Watch});
        let feeder=thread::spawn(move||{
            for chunk in bytes.chunks(188*50){
                control.record_chunk(chunk).unwrap();
                if input.write_all(chunk).is_err(){break;}
                thread::sleep(Duration::from_millis(25));
            }
        });
        for _ in 0..100{if state.ipc.exists(){break;}thread::sleep(Duration::from_millis(20));}
        state.record();assert!(state.recording_path.is_some(),"{}",state.status);assert!(state.following_live);
        for _ in 0..240{
            thread::sleep(Duration::from_millis(50));state.update_timeline();
            if state.timeline_seekable&&state.timeline_duration>7.{break;}
        }
        assert!(state.timeline_seekable,"{}",state.status);
        assert!(state.timeline_duration>7.,"duration {}",state.timeline_duration);
        assert!(state.following_live,"Growing capture must retain live follow");
        state.seek_fraction(1.);assert!(state.following_live);
        for _ in 0..80{
            thread::sleep(Duration::from_millis(50));state.update_timeline();
            if state.timeline_position>3.{break;}
        }
        assert!(state.timeline_position>3.,"position {}",state.timeline_position);
        let path=state.recording_path.clone().unwrap();
        let before=std::fs::metadata(&path).unwrap().len();
        state.seek_recording_seconds(1.);assert!(!state.following_live,"Rewind must release the live thumb");
        thread::sleep(Duration::from_millis(500));state.update_timeline();
        assert!(state.timeline_position<3.,"backward seek position {}",state.timeline_position);
        thread::sleep(Duration::from_millis(500));
        assert!(std::fs::metadata(&path).unwrap().len()>before,"capture stopped after seeking");
        state.seek_fraction(1.);assert!(state.following_live,"End seek must restore live follow");
        state.paused=true;state.seek_fraction(1.);assert!(!state.following_live,"Paused recordings must not follow the end");
        // This regression exercises live capture/rewind without requiring an
        // encoder. GPU export is exercised separately on real HD recordings.
        state.record_settings=None;
        state.record();assert!(state.recording_path.is_none());assert!(!state.following_live);
        let _=child.kill();let _=child.wait();let _=feeder.join();drop(sender);
        let _=std::fs::remove_file(sample);let _=std::fs::remove_file(path);
        let _=std::fs::remove_dir(state.recording_folder.clone());
    }
    #[test]fn empty_scan_keeps_saved_channels(){
        let path=std::env::temp_dir().join(format!("ovs-empty-scan-test-{}-{}.json",
            std::process::id(),NEXT_SESSION.fetch_add(1,AtomicOrdering::Relaxed)));
        let mut state=State::new_at(None,path.clone());
        state.services=vec![serde_json::json!({"name":"Saved channel","frequency_khz":485143,"program_id":32})];
        state.save_config();
        let (sender,result)=mpsc::channel();
        sender.send(serde_json::json!({"success":true,"stations":[]})).unwrap();
        state.job=Some(Job{control:Control::default(),result,kind:JobKind::Discover});
        state.poll();
        assert_eq!(state.services.len(),1);
        assert_eq!(State::new_at(None,path.clone()).services,state.services);
        let _=std::fs::remove_file(path);
    }
    #[test]fn old_output_size_migrates_to_windows_choices(){
        let path=std::env::temp_dir().join(format!("ovs-size-migration-{}-{}.json",
            std::process::id(),NEXT_SESSION.fetch_add(1,AtomicOrdering::Relaxed)));
        for (old,expected) in [(0,0),(1,1),(2,1),(3,2)]{
            std::fs::write(&path,serde_json::json!({"video":{"size":old}}).to_string()).unwrap();
            let state=State::new_at(None,path.clone());
            assert_eq!(state.video_options["size"],expected);
            assert_eq!(state.video_options["size_schema"],1);
        }
        let _=std::fs::remove_file(path);
    }
    #[test]fn failed_tune_does_not_scan_or_erase_channels(){
        let path=std::env::temp_dir().join(format!("ovs-failed-tune-test-{}-{}.json",
            std::process::id(),NEXT_SESSION.fetch_add(1,AtomicOrdering::Relaxed)));
        let mut state=State::new_at(None,path.clone());
        state.services=(0..50).map(|n|serde_json::json!({"name":format!("Saved {n}"),
            "frequency_khz":485143,"program_id":n+1})).collect();
        state.save_config();
        let saved=std::fs::read(&path).unwrap();
        let (sender,result)=mpsc::channel();
        sender.send(serde_json::json!({"success":false,"error":"No MPEG lock"})).unwrap();
        state.job=Some(Job{control:Control::default(),result,kind:JobKind::Watch});
        state.poll();
        assert!(state.job.is_none(),"Failed tune must not start a scan");
        assert_eq!(state.status,"No MPEG lock");
        assert_eq!(state.services.len(),50);
        assert_eq!(std::fs::read(&path).unwrap(),saved,"Failed tune must not rewrite settings");
        let _=std::fs::remove_file(path);
    }
    #[test]fn partial_scan_merges_with_saved_channels(){
        let path=std::env::temp_dir().join(format!("ovs-partial-scan-test-{}-{}.json",
            std::process::id(),NEXT_SESSION.fetch_add(1,AtomicOrdering::Relaxed)));
        let mut state=State::new_at(None,path.clone());
        state.services=(0..50).map(|n|serde_json::json!({"name":format!("Saved {n}"),
            "frequency_khz":485143,"program_id":n+1})).collect();
        state.service_index=32;state.frequency_khz=485143;state.has_frequency=true;
        state.save_config();
        let (sender,result)=mpsc::channel();
        sender.send(serde_json::json!({"success":true,"stations":[{"services":[
            {"name":"Updated 0","frequency_khz":485143,"program_id":1},
            {"name":"New channel","frequency_khz":491143,"program_id":99}
        ]}]})).unwrap();
        state.job=Some(Job{control:Control::default(),result,kind:JobKind::Discover});
        state.poll();
        assert_eq!(state.services.len(),51);
        assert_eq!(state.services[0]["name"],"Updated 0");
        assert_eq!(state.services[32]["name"],"Saved 32");
        assert_eq!(state.service_index,32);
        assert_eq!(state.frequency_khz,485143);
        let loaded=State::new_at(None,path.clone());
        assert_eq!(loaded.services,state.services);
        assert_eq!(loaded.service_index,32);
        let _=std::fs::remove_file(path);
    }
    #[test]
    #[ignore = "Legacy WSL mpv renderer; native GTK playback has its own hardware integration test"]
    fn settings_change_the_player_and_persist(){

        let config_path=std::env::temp_dir().join(format!("ovs-settings-test-{}-{}.json",
            std::process::id(),NEXT_SESSION.fetch_add(1,AtomicOrdering::Relaxed)));
        let mut s=State::new_at(None,config_path.clone());s.video_options=serde_json::json!({"color":"off","aspect":"4:3","size":1,"size_schema":1,"decoder":1,"decoder_schema":1});
        s.picture=Picture{hdr_effect:true,..Picture::preset(2)};s.deinterlacing=DeinterlaceMode::DoubleRate;
        let sample=s.ipc.with_extension("ppm");
        let mut pixels=b"P6\n640 360\n255\n".to_vec();pixels.extend([153u8,102,51].repeat(640*360));std::fs::write(&sample,pixels).unwrap();
        // Match production gpu-next; permit software GL under Xvfb and request
        // 8-bit screenshots because the Cairo pixel assertions read RGB24 bytes.
        let mut player=Command::new("mpv").args(["--no-config","--idle=yes","--vo=gpu-next","--gpu-sw=yes","--screenshot-high-bit-depth=no","--gpu-api=opengl","--gpu-context=x11egl","--ao=null","--image-display-duration=inf","--osd-level=0"])
            .arg(&sample).arg(format!("--input-ipc-server={}",s.ipc.display())).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().unwrap();
        for _ in 0..100{if s.ipc.exists(){break;}std::thread::sleep(Duration::from_millis(20));}
        s.apply_picture();assert!(s.picture_applied,"{}",s.status);
        let vf=s.player_request(serde_json::json!({"command":["get_property","vf"]})).unwrap().to_string();
        assert!(vf.contains("bwdif")&&!vf.contains("lut3d")&&!vf.contains("scale"),"CPU picture processing remains: {vf}");
        let aspect=s.player_request(serde_json::json!({"command":["get_property","video-aspect-override"]})).unwrap();
        assert!((aspect.as_f64().unwrap()-4./3.).abs()<0.001);
        let hwdec=s.player_request(serde_json::json!({"command":["get_property","hwdec"]})).unwrap();
        // mpv versions expose this option as either a string or a string list.
        assert!(hwdec==serde_json::json!("no")||hwdec==serde_json::json!(["no"]),
            "Picture controls must not change the decoder: {hwdec}");
        let shader=std::fs::read_to_string(s.shader_path()).unwrap();assert!(shader.contains("//!WIDTH 2560")&&shader.contains("//!HEIGHT 1440"));
        s.video_options["decoder"]=serde_json::json!(2);
        s.services=vec![serde_json::json!({"name":"First"}),serde_json::json!({"name":"Second"})];
        s.service_index=1;
        s.save_config();let loaded=State::new_at(None,config_path.clone());
        assert_eq!(loaded.picture,s.picture);assert_eq!(loaded.video_options,s.video_options);
        assert_eq!(loaded.deinterlacing,s.deinterlacing);
        assert_eq!(loaded.service_index,1);
        assert_eq!(loaded.services,s.services);
        assert!(loaded.video_args().unwrap().contains(&"--gpu-api=opengl".to_owned()));
        // Read rendered GPU pixels, including shader updates, not pre-GPU video screenshots.
        s.video_options=serde_json::json!({"color":"off"});s.deinterlacing=DeinterlaceMode::Off;
        let screenshot=s.ipc.with_extension("png");
        let capture=|s:&State,mode:&str|{
            std::thread::sleep(Duration::from_millis(350));
            s.player_request(serde_json::json!({"command":["screenshot-to-file",screenshot,mode]})).unwrap();
            gtk::cairo::ImageSurface::create_from_png(&mut std::fs::File::open(&screenshot).unwrap()).unwrap()
        };
        let pixel=|surface:&mut gtk::cairo::ImageSurface,x:usize,y:usize|{
            let stride=surface.stride() as usize;let data=surface.data().unwrap();let i=y*stride+x*4;[data[i+2],data[i+1],data[i]]
        };
        let mut colors=Vec::new();
        for preset in [2,1,2,0]{
            s.picture=Picture::preset(preset);s.apply_picture();assert!(s.picture_applied,"{}",s.status);
            let mut image=capture(&s,"window");let (x,y)=(image.width() as usize/2,image.height() as usize/2);colors.push(pixel(&mut image,x,y));
        }
        assert!(colors[0][0]>colors[1][0]+5&&colors[0][2]+5<colors[1][2],"Warm/cold did not change pixels: {colors:?}");
        assert_eq!(colors[0],colors[2],"Returning to Warm must reload its shader");
        for picture in [Picture{saturation:177,..Default::default()},
            Picture{brightness:30,contrast:140,..Default::default()},
            Picture{saturation:0,..Default::default()},
            Picture{hdr_effect:true,..Picture::preset(2)},Picture::default()]{
            s.picture=picture;s.apply_picture();assert!(s.picture_applied,"{}",s.status);
            let mut image=capture(&s,"window");let (x,y)=(image.width() as usize/2,image.height() as usize/2);
            let actual=pixel(&mut image,x,y);let expected=picture.apply_rgb([153./255.,102./255.,51./255.]);
            for i in 0..3{assert!((actual[i] as f32-expected[i]*255.).abs()<=3.,"GPU mismatch {picture:?}: {actual:?}, {expected:?}");}
            assert_eq!(s.player_request(serde_json::json!({"command":["get_property","vf"]})).unwrap(),serde_json::json!([]));
        }
        let revision=s.shader_revision.get();s.apply_picture();assert_eq!(revision,s.shader_revision.get(),"Unchanged picture must not regenerate the shader");
        // Alpha must blend over the video in a desktop without a compositor too.
        let mut baseline=capture(&s,"window");let background=pixel(&mut baseline,30,74);
        let mut overlay=controls::osd_bitmap("Volume 50%",Some(50),320,68,1);
        controls::send_osd(&s,60,&mut overlay,24,24).unwrap();
        let mut composed=capture(&s,"window");let foreground=pixel(&mut composed,30,74);
        for (index,color) in [0u16,255,0].into_iter().enumerate(){
            let expected=(color*191+background[index] as u16*64)/255;
            assert!((foreground[index] as i32-expected as i32).abs()<=3,"OSD alpha mismatch: {background:?} -> {foreground:?}, expected {expected}");
        }
        assert_eq!(pixel(&mut baseline,24,24),pixel(&mut composed,24,24),"OSD background must be transparent");
        s.player_request(serde_json::json!({"command":["overlay-remove",60]})).unwrap();
        let mut cleared=capture(&s,"window");assert_eq!(pixel(&mut cleared,30,74),background);
        player.kill().unwrap();player.wait().unwrap();
        let _=std::fs::remove_file(sample);let _=std::fs::remove_file(screenshot);
        let _=std::fs::remove_file(config_path);
    }
    #[test]fn language_switch_preserves_settings_and_close_works(){
        std::env::set_var("GDK_BACKEND","x11");
        gtk::init().expect("GTK display required for UI regression test");
        let app=Application::new(Some("org.openvolars.ui-regression"),Default::default());
        app.register(None::<&gtk::gio::Cancellable>).unwrap();
        let viewer=ApplicationWindow::new(&app);viewer.set_default_size(1000,700);viewer.show_all();viewer.move_(300,100);
        let config_path=std::env::temp_dir().join(format!("ovs-language-test-{}-{}.json",
            std::process::id(),NEXT_SESSION.fetch_add(1,AtomicOrdering::Relaxed)));
        let state=Rc::new(RefCell::new(State::new_at(None,config_path.clone())));
        i18n::set(0);add_css();settings_dialog(state,&viewer);
        let panel=gtk::Window::list_toplevels().into_iter().find_map(|w|w.downcast::<gtk::Window>().ok().filter(|w|w.title().as_deref()==Some("Live TV! · Settings"))).unwrap();
        let pump=||{let until=Instant::now()+Duration::from_millis(400);while Instant::now()<until{
            while glib::MainContext::default().iteration(false){}thread::sleep(Duration::from_millis(5));}};
        assert_eq!(panel.transient_for().as_ref(),Some(viewer.upcast_ref()),"Settings must center over the viewer, not the DAC");
        pump();
        let (vx,vy)=client_position(viewer.upcast_ref());let (pw,ph)=panel.size();let (px,py)=client_position(&panel);
        assert!((px-(vx+(viewer.size().0-pw)/2)).abs()<=1);
        assert!((py-(vy+(viewer.size().1-ph)/2)).abs()<=1);
        // Reproduce a WM drag after initial mapping, then wait through multiple
        // former recenter intervals. Moving the owner must not undo placement.
        panel.move_(110,130);pump();
        assert_eq!(client_position(&panel),(110,130),"Settings snapped back after being moved");
        viewer.move_(350,170);viewer.resize(1100,760);pump();
        assert_eq!(client_position(&panel),(110,130),"Owner geometry must not teleport Settings");
        // A drag begun before the first centering tick cancels startup placement.
        let manual=Rc::new(Cell::new(false));
        center_settings_on_open(&panel,viewer.upcast_ref(),manual.clone());
        manual.set(true);panel.move_(160,190);pump();
        assert_eq!(client_position(&panel),(160,190),"Early drag must win over pending centering");
        assert!(!panel.is_modal(),"Settings must not block the DAC");
        let widgets=descendants(panel.upcast_ref());
        assert!(!widgets.iter().filter_map(|w|w.downcast_ref::<Label>())
            .any(|w|w.text().as_str()=="Start receiver with Linux"));
        let stack=widgets.iter().find_map(|w|w.clone().downcast::<gtk::Stack>().ok()).unwrap();
        let language=widgets.iter().filter_map(|w|w.clone().downcast::<ComboBoxText>().ok()).find(|w|w.active_text().as_deref()==Some("English")).unwrap();
        let entry=widgets.iter().find_map(|w|w.clone().downcast::<gtk::Entry>().ok()).unwrap();
        entry.set_text("unsaved input");stack.set_visible_child_name("General");
        for index in [1,2,3,0]{
            language.set_active(Some(index));Skin::verify_translated_buttons();
            assert!(panel.is_visible());assert_eq!(stack.visible_child_name().as_deref(),Some("General"));
            assert_eq!(entry.text().as_str(),"unsaved input");
            while glib::MainContext::default().iteration(false){}
            let label=widgets.iter().filter_map(|w|w.downcast_ref::<Label>()).find(|w|w.text().as_str()==i18n::text("Language")).unwrap();
            let color=label.style_context().color(gtk::StateFlags::NORMAL);
            assert!(color.red()>0.85&&color.green()>0.8&&color.blue()>0.7,"Settings labels must retain warm text: {color:?}");
            assert!(label.allocation().width()<=210,"Translated label must stay inside its column");
            let expected=i18n::text("Save parental controls");
            assert!(widgets.iter().filter_map(|w|w.downcast_ref::<Button>()).any(|w|w.label().as_deref()==Some(expected.as_str())));
        }
        let close=widgets.iter().find_map(|w|w.downcast_ref::<Button>().filter(|w|w.widget_name()=="orbit-close")).unwrap();
        close.emit_clicked();
        while glib::MainContext::default().iteration(false){}
        assert!(!panel.is_visible());viewer.close();
        let _=std::fs::remove_file(config_path);
    }
}
