//! Private local control socket for the native player. The JSON command shape
//! remains compatible with the existing GTK controls, without invoking mpv.
use crate::{audio_modes::Mode,picture::Picture};
use serde_json::{Value,json};
use std::{sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}},path::{Path,PathBuf},collections::{BTreeMap,VecDeque},io::{Read,Write,Seek,SeekFrom},os::unix::{net::UnixListener,fs::PermissionsExt},time::Duration};
#[derive(Clone)]
pub struct Settings{pub paused:bool,pub volume:f32,pub audio:Mode,pub deinterlace:u8,pub size:u8,pub picture:Picture,pub aspect:f64,pub profile:String,pub video_pid:Option<u16>,pub audio_pid:Option<u16>,pub program:Option<u16>}
impl Default for Settings{fn default()->Self{Self{paused:false,volume:100.,audio:Mode::Stereo,deinterlace:2,size:0,picture:Picture::default(),aspect:-1.,profile:"monitor".into(),video_pid:None,audio_pid:None,program:None}}}
#[derive(Clone)]pub struct Overlay{pub x:u32,pub y:u32,pub width:u32,pub height:u32,pub data:Arc<Vec<u8>>,pub revision:u64}
pub struct Runtime{
    pub settings:Settings,pub position:f64,pub video_position:Option<f64>,pub start:f64,pub duration:f64,pub tracks:Value,pub osd:Value,pub error:Option<String>,pub revision:u64,pub overlays:BTreeMap<u32,Overlay>,pub snapshots:VecDeque<PathBuf>,pub snapshot_folder:PathBuf,pub seek_request:Option<f64>,pub cache_range:Option<(f64,f64)>,
}
impl Default for Runtime{fn default()->Self{Self{settings:Settings::default(),position:0.,video_position:None,start:0.,duration:0.,tracks:json!([]),osd:json!({"w":1,"h":1,"ml":0,"mr":0,"mt":0,"mb":0}),error:None,revision:0,overlays:BTreeMap::new(),snapshots:VecDeque::new(),snapshot_folder:std::env::temp_dir(),seek_request:None,cache_range:None}}}
pub type Shared=Arc<Mutex<Runtime>>;
impl Runtime{
    pub fn command(&mut self,command:&Value)->Result<Value,String>{
        if command["name"]=="overlay-add"{return self.overlay(command);}
        let args=command.as_array().ok_or("Expected a command array")?;
        let name=args.first().and_then(Value::as_str).ok_or("Missing command")?;
        let property=args.get(1).and_then(Value::as_str).unwrap_or("");
        match name{
            "get_property"=>Ok(match property{
                "pause"=>json!(self.settings.paused),"volume"=>json!(self.settings.volume),
                "audio-mode"=>json!(self.settings.audio as u8),"deinterlace-mode"=>json!(self.settings.deinterlace),
                "output-size"=>json!(self.settings.size),"picture"=>self.settings.picture.json(),
                "icc-profile"=>json!(self.settings.profile),"video-aspect-override"=>json!(self.settings.aspect),"time-pos"=>json!(self.position),
                "native-video-pos"=>json!(self.video_position),"demuxer-start-time"=>json!(self.start),"duration"=>json!(self.duration),"track-list"=>self.tracks.clone(),
                "osd-dimensions"=>self.osd.clone(),"native-error"=>json!(self.error),
                "seekable"=>json!(self.cache_range.is_some()),
                "demuxer-cache-state"=>json!({"seekable-ranges":self.cache_range.map(|(start,end)|vec![json!({"start":start,"end":end})]).unwrap_or_default()}),
                "native-backend"=>json!("Vulkan Video / shared Windows shader / native AAC and PulseAudio"),
                _=>return Err(format!("Property unavailable: {property}")),
            }),
            "set_property"=>{
                let value=args.get(2).ok_or("Missing property value")?;
                match property{
                    "vid"|"aid"=>{
                        let id=value.as_u64().and_then(|n|u16::try_from(n).ok()).ok_or("Invalid track id")?;
                        let kind=if property=="vid"{"video"}else{"audio"};
                        let track=self.tracks.as_array().and_then(|t|t.iter().find(|t|t["id"]==id&&t["type"]==kind)).ok_or("Unknown track")?;
                        if track["selected"]!=true{if property=="vid"{self.settings.video_pid=Some(id);}else{self.settings.audio_pid=Some(id);}self.seek_request=Some(self.position);}
                    },
                    "sid"|"sub-visibility"=>{}, // Caption bitmaps are owned by the GTK caption engine.
                    "screenshot-directory"=>self.snapshot_folder=PathBuf::from(value.as_str().ok_or("Expected a snapshot directory")?),
                    "pause"=>self.settings.paused=value.as_bool().ok_or("pause requires a boolean")?,
                    "volume"=>self.settings.volume=number(value)?.clamp(0.,100.) as f32,
                    "audio-mode"=>{let n=index(value,4)?;self.settings.audio=Mode::from_index(n as usize);},
                    "deinterlace-mode"=>self.settings.deinterlace=index(value,2)?,
                    "output-size"=>self.settings.size=index(value,2)?,
                    "icc-profile"=>self.settings.profile=value.as_str().ok_or("ICC profile requires a path or monitor/off")?.to_owned(),
                    "picture"=>{if !value.is_object(){return Err("picture requires an object".into());}self.settings.picture=Picture::load(value);},
                    "video-aspect-override"=>{let ratio=if let Some((a,b))=value.as_str().and_then(|s|s.split_once(':')){let a=a.parse::<f64>().map_err(|_|"Invalid video aspect")?;let b=b.parse::<f64>().map_err(|_|"Invalid video aspect")?;a/b}else{number(value)?};if !ratio.is_finite()||ratio!=-1.&&!(0.2..=5.).contains(&ratio){return Err("Invalid video aspect".into());}self.settings.aspect=ratio;},
                    _=>return Err(format!("Property unavailable: {property}")),
                }
                self.revision=self.revision.wrapping_add(1);Ok(Value::Null)
            },
            "seek"=>{
                let value=number(args.get(1).ok_or("Missing seek position")?)?;let target=if args.get(2).and_then(Value::as_str)==Some("absolute"){value}else{self.position+value};
                let(start,end)=self.cache_range.ok_or("Playback is not yet seekable")?;self.seek_request=Some(target.clamp(start,end));Ok(Value::Null)
            },
            "cycle" if property=="audio"=>{
                let tracks:Vec<_>=self.tracks.as_array().ok_or("No tracks")?.iter().filter(|t|t["type"]=="audio").collect();
                if tracks.len()>1{let current=tracks.iter().position(|t|t["selected"]==true).unwrap_or(0);self.settings.audio_pid=tracks[(current+1)%tracks.len()]["id"].as_u64().and_then(|n|u16::try_from(n).ok());self.seek_request=Some(self.position);}Ok(Value::Null)
            },
            "overlay-remove"=>{let id=args.get(1).and_then(Value::as_u64).and_then(|n|u32::try_from(n).ok()).ok_or("Invalid overlay id")?;self.overlays.remove(&id);self.revision=self.revision.wrapping_add(1);Ok(Value::Null)},
            "screenshot"|"screenshot-to-file"=>{
                if self.snapshots.len()>=4{return Err("Snapshot queue is full".into());}
                let path=if name=="screenshot-to-file"{PathBuf::from(args.get(1).and_then(Value::as_str).ok_or("Missing snapshot filename")?)}else{
                    let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();self.snapshot_folder.join(format!("Live TV {stamp}.png"))
                };
                if path.exists(){return Err("Snapshot already exists".into());}self.snapshots.push_back(path);Ok(Value::Null)
            },
            _=>Err(format!("Command unavailable: {name}")),
        }
    }
    fn overlay(&mut self,v:&Value)->Result<Value,String>{
        let uint=|key:&str|v[key].as_u64().and_then(|n|u32::try_from(n).ok()).ok_or_else(||format!("Invalid overlay {key}"));
        let(id,x,y,width,height,stride)=(uint("id")?,uint("x")?,uint("y")?,uint("w")?,uint("h")?,uint("stride")?);
        if v["fmt"]!="bgra"||width==0||height==0||width>8192||height>8192||stride<width*4||stride as u64*height as u64>64*1024*1024{return Err("Invalid overlay dimensions/format".into());}
        if self.overlays.len()>=8&&!self.overlays.contains_key(&id){return Err("Too many overlays".into());}
        let path=v["file"].as_str().ok_or("Missing overlay file")?;let offset=v["offset"].as_u64().unwrap_or(0);
        let mut file=std::fs::File::open(path).map_err(|e|e.to_string())?;file.seek(SeekFrom::Start(offset)).map_err(|e|e.to_string())?;
        let mut packed=Vec::with_capacity((width*height*4) as usize);let mut row=vec![0;stride as usize];
        for _ in 0..height{file.read_exact(&mut row).map_err(|e|e.to_string())?;packed.extend_from_slice(&row[..width as usize*4]);}
        self.revision=self.revision.wrapping_add(1);
        self.overlays.insert(id,Overlay{x,y,width,height,data:Arc::new(packed),revision:self.revision});Ok(Value::Null)
    }

}
fn number(value:&Value)->Result<f64,String>{value.as_f64().or_else(||value.as_str()?.parse().ok()).filter(|n|n.is_finite()).ok_or_else(||"Expected a finite number".into())}
fn index(value:&Value,max:u8)->Result<u8,String>{let n=value.as_u64().filter(|&n|n<=max as u64).ok_or("Invalid mode index")?;Ok(n as u8)}
// Channel changes kill the previous helper, so its socket can outlive the
// listener. Recover only a disconnected socket belonging to this desktop user.
fn bind_control(path:&Path)->std::io::Result<UnixListener>{
    use std::os::unix::{fs::{FileTypeExt,MetadataExt},net::UnixStream};
    match UnixListener::bind(path){
        Ok(listener)=>Ok(listener),
        Err(error) if error.kind()==std::io::ErrorKind::AddrInUse=>{
            let before=std::fs::symlink_metadata(path)?;
            if !before.file_type().is_socket()||before.uid()!=unsafe{libc::geteuid()}{return Err(error);}
            match UnixStream::connect(path){
                Err(disconnected) if disconnected.kind()==std::io::ErrorKind::ConnectionRefused=>{},
                _=>return Err(error), // Never unlink a live listener or an inaccessible path.
            }
            let after=std::fs::symlink_metadata(path)?;
            if (before.dev(),before.ino())!=(after.dev(),after.ino()){return Err(error);}
            std::fs::remove_file(path)?;
            UnixListener::bind(path)
        },
        Err(error)=>Err(error),
    }
}
pub struct Server{path:PathBuf,stop:Arc<AtomicBool>,thread:Option<std::thread::JoinHandle<()>>}
impl Server{
    pub fn start(path:&Path,state:Shared)->std::io::Result<Self>{
        let listener=bind_control(path)?;
        std::fs::set_permissions(path,std::fs::Permissions::from_mode(0o600))?;listener.set_nonblocking(true)?;
        let stop=Arc::new(AtomicBool::new(false));let stopped=stop.clone();
        let thread=std::thread::spawn(move||{
            while !stopped.load(Ordering::Relaxed){
                match listener.accept(){
                    Ok((mut socket,_))=>{
                        let _=socket.set_read_timeout(Some(Duration::from_millis(200)));let _=socket.set_write_timeout(Some(Duration::from_millis(200)));
                        let mut bytes=Vec::new();let mut b=[0u8;1];
                        while bytes.len()<65536&&socket.read_exact(&mut b).is_ok(){if b[0]==b'\n'{break;}bytes.push(b[0]);}
                        let reply=match serde_json::from_slice::<Value>(&bytes){
                            Ok(request)=>{let id=request.get("request_id").cloned().unwrap_or(Value::Null);
                                match state.lock().unwrap().command(&request["command"]){
                                    Ok(data)=>json!({"request_id":id,"error":"success","data":data}),
                                    Err(e)=>json!({"request_id":id,"error":e}),
                                }
                            },Err(e)=>json!({"error":format!("Invalid player command: {e}")}),
                        };
                        let _=writeln!(socket,"{reply}");
                    },Err(e) if e.kind()==std::io::ErrorKind::WouldBlock=>std::thread::sleep(Duration::from_millis(2)),Err(_)=>break,
                }
            }
        });
        Ok(Self{path:path.to_owned(),stop,thread:Some(thread)})
    }
}
impl Drop for Server{fn drop(&mut self){self.stop.store(true,Ordering::Relaxed);if let Some(thread)=self.thread.take(){let _=thread.join();}let _=std::fs::remove_file(&self.path);}}
#[cfg(test)]mod tests{
    use super::*;
    struct SocketFixture(PathBuf);
    impl SocketFixture{
        fn new()->Self{
            static NEXT:std::sync::atomic::AtomicUsize=std::sync::atomic::AtomicUsize::new(0);
            let path=std::env::temp_dir().join(format!("ovs-control-test-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
            std::fs::create_dir(&path).unwrap();Self(path)
        }
        fn path(&self)->PathBuf{self.0.join("control.sock")}
    }
    impl Drop for SocketFixture{fn drop(&mut self){let _=std::fs::remove_dir_all(&self.0);}}
    #[test]fn restart_recovers_socket_left_by_terminated_player(){
        let fixture=SocketFixture::new();let path=fixture.path();
        // SIGTERM/SIGKILL close the listener without running Server::drop.
        for _ in 0..3{
            drop(UnixListener::bind(&path).unwrap());
            let server=Server::start(&path,Arc::new(Mutex::new(Runtime::default()))).unwrap();
            use std::io::BufRead;
            let mut client=std::os::unix::net::UnixStream::connect(&path).unwrap();
            client.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
            writeln!(client,"{}",json!({"command":["get_property","volume"],"request_id":7})).unwrap();
            let mut line=String::new();std::io::BufReader::new(client).read_line(&mut line).unwrap();
            let reply:Value=serde_json::from_str(&line).unwrap();
            assert_eq!(reply["request_id"],7);assert_eq!(reply["data"],100.);
            drop(server);assert!(!path.exists());
        }
    }
    #[test]fn restart_does_not_replace_active_listener(){
        let fixture=SocketFixture::new();let path=fixture.path();
        let listener=UnixListener::bind(&path).unwrap();
        assert!(Server::start(&path,Arc::new(Mutex::new(Runtime::default()))).is_err());
        assert!(std::os::unix::net::UnixStream::connect(&path).is_ok());drop(listener);
    }
    #[test]fn restart_does_not_remove_regular_file(){
        let fixture=SocketFixture::new();let path=fixture.path();std::fs::write(&path,b"keep").unwrap();
        assert!(Server::start(&path,Arc::new(Mutex::new(Runtime::default()))).is_err());
        assert_eq!(std::fs::read(&path).unwrap(),b"keep");
    }
    #[test]fn reject_invalid_commands_without_mutating_settings(){
        let mut s=Runtime::default();
        for c in [json!(["set_property","pause",1]),json!(["set_property","audio-mode",9]),json!(["set_property","video-aspect-override",0]),json!(["set_property","picture",0]),json!(["set_property","volume","NaN"])]{assert!(s.command(&c).is_err());}
        assert_eq!(s.revision,0);assert!(!s.settings.paused);assert_eq!(s.settings.audio,Mode::Stereo);
    }
    #[test]fn sound_and_video_modes_are_independent(){
        let mut s=Runtime::default();s.command(&json!(["set_property","audio-mode",4])).unwrap();s.command(&json!(["set_property","pause",true])).unwrap();
        assert_eq!(s.settings.audio,Mode::Surround);assert_eq!(s.settings.deinterlace,2);assert!(s.settings.paused);
        assert!(s.command(&json!(["af","set","lavfi"])).is_err());
    }
}
