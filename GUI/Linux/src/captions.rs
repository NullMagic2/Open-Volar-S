//! ISDB captions use the same decoder and layout as Windows, timed by mpv's video clock.
use super::*;
use a865r_media::caption_stream::{Packet,Stream};
use std::{collections::VecDeque,sync::{Arc,Mutex,atomic::{AtomicBool,AtomicI64,Ordering}}};
const OVERLAY:u32=62;
const WRAP:i64=(1i64<<33)/90;
#[repr(C)]
#[derive(Default)]
struct Image {data:*const u8,length:usize,width:i32,height:i32,stride:i32,x:i32,y:i32}
unsafe extern "C" {
    fn a865r_cc_new(profile:i32)->*mut c_void;
    fn a865r_cc_free(p:*mut c_void);
    fn a865r_cc_decode(p:*mut c_void,data:*const u8,len:usize,pts:i64)->i32;
    fn a865r_cc_render(p:*mut c_void,pts:i64,w:i32,h:i32,image:*mut Image)->i32;
    fn a865r_cc_flush(p:*mut c_void);
    fn a865r_cc_invalidate(p:*mut c_void);
}
fn nearest(pts:i64,clock:i64)->i64 {pts+((clock-pts+WRAP/2).div_euclid(WRAP))*WRAP}
struct Engine {ptr:*mut c_void,history:VecDeque<Packet>,decoded:usize,last:Option<i64>,bytes:usize}
impl Drop for Engine {fn drop(&mut self){unsafe{a865r_cc_free(self.ptr)}}}
impl Engine {
    fn new(profile:u16)->Self{Self{ptr:unsafe{a865r_cc_new(profile as i32)},history:VecDeque::new(),decoded:0,last:None,bytes:0}}
    fn push(&mut self,packets:Vec<Packet>){for p in packets{self.bytes+=p.bytes.len();self.history.push_back(p);}
        while self.history.len()>4096||self.bytes>8*1024*1024 {if let Some(p)=self.history.pop_front(){self.bytes-=p.bytes.len();self.decoded=self.decoded.saturating_sub(1);}}
    }
    fn render(&mut self,clock:i64,w:i32,h:i32)->(i32,Image){
        if self.last.is_some_and(|old|clock<old-100){unsafe{a865r_cc_flush(self.ptr)}self.decoded=0;}
        while let Some(p)=self.history.get(self.decoded){let pts=nearest(p.pts_ms,clock);if pts>clock{break}
            unsafe{a865r_cc_decode(self.ptr,p.bytes.as_ptr(),p.bytes.len(),pts);}self.decoded+=1;
        }
        self.last=Some(clock);let mut im=Image::default();let status=unsafe{a865r_cc_render(self.ptr,clock,w,h,&mut im)};(status,im)
    }
}
fn surface(im:&Image)->Option<gtk::cairo::ImageSurface>{
    if im.data.is_null()||im.width<=0||im.height<=0||im.width>7680||im.height>4320||im.stride<im.width*4||im.length<(im.stride as usize)*(im.height as usize){return None}
    let input=unsafe{std::slice::from_raw_parts(im.data,im.length)};
    let mut out=gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32,im.width,im.height).ok()?;
    let stride=out.stride() as usize;{let mut dst=out.data().ok()?;
        for y in 0..im.height as usize{for x in 0..im.width as usize{let p=&input[y*im.stride as usize+x*4..][..4];let q=&mut dst[y*stride+x*4..][..4];
            let a=p[3] as u16;q.copy_from_slice(&[((p[2] as u16*a+127)/255) as u8,((p[1] as u16*a+127)/255) as u8,((p[0] as u16*a+127)/255) as u8,p[3]]);
        }}
    }Some(out)
}
// A recording reader stays a few seconds ahead of the displayed picture. Seeking
// backward restarts parsing off the GTK thread, preserving management data and DRCS.
struct Recording {stream:Arc<Mutex<Stream>>,clock:Arc<AtomicI64>,video_pid:Arc<AtomicI64>,stop:Arc<AtomicBool>}
impl Drop for Recording {fn drop(&mut self){self.stop.store(true,Ordering::Relaxed);}}
impl Recording {
    fn new(path:PathBuf)->Self{
        let stream=Arc::new(Mutex::new(Stream::default()));let clock=Arc::new(AtomicI64::new(i64::MIN));let stop=Arc::new(AtomicBool::new(false));
        let video_pid=Arc::new(AtomicI64::new(-1));let (output,target,video,quit)=(stream.clone(),clock.clone(),video_pid.clone(),stop.clone());
        thread::spawn(move||{
            use std::io::{Read,Seek};let Ok(mut file)=std::fs::File::open(path) else{return};
            let mut probe=[0u8;8192];let Ok(n)=file.read(&mut probe) else{return};
            if n<188*3||!(0..n-188*2).any(|i|probe[i]==0x47&&probe[i+188]==0x47&&probe[i+376]==0x47){return}
            if file.rewind().is_err(){return}
            let mut analyzer=a865r::TsAnalyzer::new();let mut buffer=[0;188*128];let mut last_clock=i64::MIN;let mut ahead=None;let mut eof=false;let mut last_video=-1;
            while !quit.load(Ordering::Relaxed){let now=target.load(Ordering::Relaxed);
                if now==i64::MIN{thread::sleep(Duration::from_millis(40));continue}
                let requested_video=video.load(Ordering::Relaxed);
                if (last_clock!=i64::MIN&&now<last_clock-500)||(last_clock!=i64::MIN&&requested_video!=last_video){
                    if file.rewind().is_err(){break}analyzer=a865r::TsAnalyzer::new();let mut s=output.lock().unwrap();let revision=s.revision.wrapping_add(1);*s=Stream::default();s.revision=revision;ahead=None;eof=false;
                }last_clock=now;last_video=requested_video;
                if eof||ahead.is_some_and(|pts|nearest(pts,now)>now+3000){thread::sleep(Duration::from_millis(40));continue}
                let n=match file.read(&mut buffer){Ok(0)=>{eof=true;continue},Ok(n)=>n,Err(_)=>break};analyzer.push(&buffer[..n]);
                let stats=analyzer.stats();let program=stats.streams.iter().find(|s|matches!(s.stream_type,1|2|0x1b|0x24)&&(requested_video<0||s.pid as i64==requested_video)).map(|s|s.program_number);
                if let Some(program)=program{let mut s=output.lock().unwrap();s.select(stats,program);s.push(&buffer[..n]);ahead=s.timeline.or_else(||s.packets.back().map(|p|p.pts_ms));}
            }
        });Self{stream,clock,video_pid,stop}
    }
}
pub(super) fn install(state:Rc<RefCell<State>>,viewer:&ApplicationWindow){
    use std::os::unix::fs::MetadataExt;
    let weak=viewer.downgrade();let mut source=String::new();let mut recording:Option<Recording>=None;let mut engine:Option<Engine>=None;let mut selection=None;let mut visible=false;let mut enabled=false;let mut rect=None;
    glib::timeout_add_local(Duration::from_millis(80),move||{
        if weak.upgrade().is_none(){return glib::ControlFlow::Break}
        let s=state.borrow();let active=s.file_player.is_some()||s.job.as_ref().is_some_and(|j|j.kind==JobKind::Watch);
        let inode=std::fs::metadata(&s.ipc).ok().map(|m|m.ino()).unwrap_or(0);
        let key=if active{format!("{inode}:{}",if s.file_player.is_some(){s.file_path.as_ref().map(|p|p.to_string_lossy().into_owned()).unwrap_or_default()}else{"live".into()})}else{String::new()};
        if key!=source{source=key;recording=if s.file_player.is_some(){s.file_path.clone().map(Recording::new)}else{None};engine=None;selection=None;rect=None;visible=false;enabled=false;}
        if !active||inode==0{return glib::ControlFlow::Continue}
        let stream=recording.as_ref().map(|r|r.stream.clone()).or_else(||s.job.as_ref().map(|j|j.control.caption_stream.clone()));
        let Some(stream)=stream else{return glib::ControlFlow::Continue};
        let get=|name|s.player_request(serde_json::json!({"command":["get_property",name]})).ok();
        let Some(time)=get("time-pos").and_then(|v|v.as_f64()) else{return glib::ControlFlow::Continue};
        let start=get("demuxer-start-time").and_then(|v|v.as_f64()).unwrap_or(0.);let clock=((time+start)*1000.).round() as i64;
        if let Some(r)=&recording{
            if let Some(tracks)=get("track-list"){if let Some(pid)=tracks.as_array().and_then(|tracks|tracks.iter().find(|t|t["type"]=="video"&&t["selected"]==true)).and_then(|t|t["demux-id"].as_i64()){r.video_pid.store(pid,Ordering::Relaxed);}}
            r.clock.store(clock,Ordering::Relaxed);
        }
        {let mut stream=stream.lock().unwrap();let next=(stream.pid,stream.profile,stream.revision);
            if selection!=Some(next){selection=Some(next);engine=stream.pid.map(|_|Engine::new(stream.profile));if engine.as_ref().is_some_and(|e|!e.ptr.is_null()){let _=s.player_request(serde_json::json!({"command":["set_property","sid","no"]}));}if visible{let _=s.player_request(serde_json::json!({"command":["overlay-remove",OVERLAY]}));visible=false;}}
            if let Some(engine)=engine.as_mut(){engine.push(stream.drain());}
        }
        if !s.captions||engine.is_none(){if visible{let _=s.player_request(serde_json::json!({"command":["overlay-remove",OVERLAY]}));visible=false;}enabled=false;return glib::ControlFlow::Continue}
        let engine=engine.as_mut().unwrap();
        let Some(dim)=get("osd-dimensions") else{return glib::ControlFlow::Continue};let n=|key|dim[key].as_i64().unwrap_or(0) as i32;
        let (x,y)=(n("ml"),n("mt"));let (w,h)=(n("w")-x-n("mr"),n("h")-y-n("mb"));let bounds=(x,y,w,h);
        if !enabled||rect!=Some(bounds){unsafe{a865r_cc_invalidate(engine.ptr)}rect=Some(bounds);enabled=true;}
        let (status,im)=engine.render(clock,w,h);
        if status==2{if let Some(mut frame)=surface(&im){visible=controls::send_osd(&s,OVERLAY,&mut frame,x+im.x,y+im.y).is_ok();if !visible{unsafe{a865r_cc_invalidate(engine.ptr)}}}}
        else if status!=3&&visible{let _=s.player_request(serde_json::json!({"command":["overlay-remove",OVERLAY]}));visible=false;}
        glib::ControlFlow::Continue
    });
}

#[cfg(test)]mod tests {
 use super::*;
 fn group(id:u8,body:&[u8])->Vec<u8>{let mut g=vec![id<<2,0,0,(body.len()>>8) as u8,body.len() as u8];g.extend(body);let mut crc=0u16;for &b in &g{crc^=(b as u16)<<8;for _ in 0..8{crc=if crc&0x8000!=0{(crc<<1)^0x1021}else{crc<<1};}}g.extend(crc.to_be_bytes());let mut p=vec![0x80,0xff,0xf0];p.extend(g);p}
 fn statement(text:&[u8])->Vec<u8>{let mut unit=vec![0x1f,0x20,0,0,text.len() as u8];unit.extend(text);let mut body=vec![0,0,0,unit.len() as u8];body.extend(unit);group(1,&body)}
 #[test]fn portuguese_captions_follow_video_clock_seek_resize_and_alpha(){
  let mut e=Engine::new(8);assert!(!e.ptr.is_null());e.push(vec![Packet{pts_ms:0,bytes:group(0,&[0,1,0,b'p',b'o',b'r',0x80,0,0,0])},Packet{pts_ms:1000,bytes:statement(b"\x0c\x1c\x48\x42Legenda de teste")},Packet{pts_ms:2000,bytes:statement(b"\x0c")}]);
  assert_eq!(e.render(500,960,540).0,1);let (status,im)=e.render(1000,960,540);assert_eq!(status,2);let mut f=surface(&im).unwrap();let bytes=f.data().unwrap();assert!(bytes.chunks_exact(4).any(|p|p[3]>0));assert!(bytes.chunks_exact(4).all(|p|p[0]<=p[3]&&p[1]<=p[3]&&p[2]<=p[3]));drop(bytes);
  assert_eq!(e.render(1000,960,540).0,3);assert_eq!(e.render(1000,1280,720).0,2);assert_eq!(e.render(2500,1280,720).0,1);assert_eq!(e.render(1000,1280,720).0,2);assert_eq!(nearest(100,WRAP+120),WRAP+100);
 }
 #[test]#[ignore="Requires an original broadcast TS in A865R_CAPTION_TEST_TS"]
 fn recorded_broadcast_captions_decode_and_recording_reader_rewinds(){
  let path=PathBuf::from(std::env::var_os("A865R_CAPTION_TEST_TS").expect("A865R_CAPTION_TEST_TS"));let bytes=std::fs::read(&path).unwrap();let mut analyzer=a865r::TsAnalyzer::new();let mut stream=Stream::default();
  for chunk in bytes.chunks(188*128){analyzer.push(chunk);let stats=analyzer.stats();if let Some(program)=stats.streams.iter().find(|s|matches!(s.stream_type,1|2|0x1b|0x24)).map(|s|s.program_number){stream.select(stats,program);stream.push(chunk);}}
  assert!(stream.pid.is_some());let packets=stream.drain();assert!(!packets.is_empty());let first=packets[0].pts_ms;let times:Vec<_>=packets.iter().map(|p|p.pts_ms).collect();let mut engine=Engine::new(stream.profile);engine.push(packets);let mut images=0;
  for time in times {if engine.render(time,960,540).0==2{images+=1;}}assert!(images>0,"Broadcast captions must render");
  let r=Recording::new(path);r.clock.store(first,Ordering::Relaxed);for _ in 0..100{if !r.stream.lock().unwrap().packets.is_empty(){break}thread::sleep(Duration::from_millis(20));}assert!(!r.stream.lock().unwrap().packets.is_empty());let revision=r.stream.lock().unwrap().revision;r.clock.store(first-1000,Ordering::Relaxed);for _ in 0..100{if r.stream.lock().unwrap().revision>revision{break}thread::sleep(Duration::from_millis(20));}assert!(r.stream.lock().unwrap().revision>revision);
  let stats=analyzer.stats();let first_program=stats.streams.iter().find(|s|matches!(s.stream_type,1|2|0x1b|0x24)).unwrap().program_number;
  if let Some(video)=stats.streams.iter().find(|s|s.program_number!=first_program&&matches!(s.stream_type,1|2|0x1b|0x24)){
   if let Some(caption)=stats.streams.iter().find(|s|s.program_number==video.program_number&&stats.caption_profiles.contains_key(&s.pid)){
    r.video_pid.store(video.pid as i64,Ordering::Relaxed);r.clock.store(first,Ordering::Relaxed);
    for _ in 0..100{if r.stream.lock().unwrap().pid==Some(caption.pid){break}thread::sleep(Duration::from_millis(20));}assert_eq!(r.stream.lock().unwrap().pid,Some(caption.pid),"File captions must follow mpv's selected video service");
   }
  }println!("Rendered {images} broadcast caption images; recording rewind reset confirmed");
 }
 #[test]fn caption_overlay_follows_paused_mpv_and_cc_toggle(){
  gtk::init().expect("Run with xvfb-run");let app=Application::new(Some("org.openvolars.caption-test"),Default::default());app.register(None::<&gtk::gio::Cancellable>).unwrap();let owner=ApplicationWindow::new(&app);
  let state=Rc::new(RefCell::new(State::new(None)));let sample=state.borrow().ipc.with_extension("ts");
  assert!(Command::new("ffmpeg").args(["-v","error","-f","lavfi","-i","color=c=blue:size=640x360:rate=25","-t","4","-c:v","mpeg2video","-f","mpegts","-y"]).arg(&sample).status().unwrap().success());
  let mut player=Command::new("mpv").args(["--no-config","--vo=gpu-next","--gpu-sw=yes","--gpu-api=opengl","--gpu-context=x11egl","--ao=null","--pause","--osd-level=0","--screenshot-high-bit-depth=no"])
    .arg(format!("--input-ipc-server={}",state.borrow().ipc.display())).arg(&sample).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().unwrap();
  let pump=|ms|{let until=Instant::now()+Duration::from_millis(ms);while Instant::now()<until{while glib::MainContext::default().iteration(false){}thread::sleep(Duration::from_millis(5));}};
  for _ in 0..80{pump(25);if state.borrow().player_request(serde_json::json!({"command":["get_property","time-pos"]})).is_ok(){break}}
  let start=state.borrow().player_request(serde_json::json!({"command":["get_property","demuxer-start-time"]})).unwrap().as_f64().unwrap();
  let control=Control::default();{let mut stream=control.caption_stream.lock().unwrap();stream.pid=Some(42);stream.profile=8;
    stream.packets.push_back(Packet{pts_ms:(start*1000.) as i64,bytes:group(0,&[0,1,0,b'p',b'o',b'r',0x80,0,0,0])});stream.packets.push_back(Packet{pts_ms:(start*1000.) as i64+1000,bytes:statement(b"\x0c\x1c\x48\x42Legenda de teste")});}
  let (_tx,result)=mpsc::channel();{let mut s=state.borrow_mut();s.job=Some(Job{control,result,kind:JobKind::Watch});s.captions=true;}install(state.clone(),&owner);
  let image=sample.with_extension("png");let capture=||{state.borrow().player_request(serde_json::json!({"command":["screenshot-to-file",image,"window"]})).unwrap();let mut surface=gtk::cairo::ImageSurface::create_from_png(&mut std::fs::File::open(&image).unwrap()).unwrap();let bytes=surface.data().unwrap().to_vec();bytes};
  pump(250);let baseline=capture();state.borrow().player_request(serde_json::json!({"command":["seek",1.4,"absolute+exact"]})).unwrap();pump(600);let caption=capture();assert_ne!(caption,baseline,"Caption must be composed over the video");pump(250);assert_eq!(capture(),caption,"Paused picture must retain its caption");
  state.borrow_mut().captions=false;pump(250);assert_eq!(capture(),baseline,"CC off must remove the overlay");state.borrow_mut().captions=true;pump(250);assert_eq!(capture(),caption,"CC on must restore the paused caption");
  state.borrow().player_request(serde_json::json!({"command":["seek",0,"absolute+exact"]})).unwrap();pump(350);assert_eq!(capture(),baseline,"Backward seeking must not display a future caption");
  player.kill().unwrap();player.wait().unwrap();owner.close();let _=std::fs::remove_file(sample);let _=std::fs::remove_file(image);
 }
}
