//! WSL presentation adapter. Cache compressed TS, not multi-megabyte raw frames.
//! Seeking restarts the host decoder at a keyframe in that compressed source.
use a865r_media::{playback::Control,wsl_video::HostBridge};
use serde_json::{json,Value};
use std::{collections::BTreeMap,ffi::OsString,fs::{self,File},io::{self,Read,Write,BufRead,BufReader},
    os::{fd::AsRawFd,unix::{net::{UnixListener,UnixStream},fs::PermissionsExt}},
    path::{Path,PathBuf},process::{Child,Command,Stdio},sync::{Arc,Mutex,mpsc,atomic::{AtomicBool,AtomicU64,Ordering}},
    thread,time::{Duration,Instant,SystemTime,UNIX_EPOCH}};
static STOP:AtomicBool=AtomicBool::new(false);
mod wsl_audio;
extern "C" fn stop_signal(_:i32){STOP.store(true,Ordering::Relaxed);}
type Result<T>=std::result::Result<T,String>;
fn request(path:&Path,command:Value)->Result<Value>{
    let mut stream=UnixStream::connect(path).map_err(|e|e.to_string())?;
    stream.set_read_timeout(Some(Duration::from_millis(500))).map_err(|e|e.to_string())?;
    stream.set_write_timeout(Some(Duration::from_millis(500))).map_err(|e|e.to_string())?;
    let message=json!({"command":command,"request_id":1});
    writeln!(stream,"{message}").map_err(|e|e.to_string())?;
    for line in BufReader::new(stream).lines(){
        let value:Value=serde_json::from_str(&line.map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
        if value["request_id"]==1{return if value["error"]=="success"{Ok(value["data"].clone())}else{Err(value["error"].to_string())};}
    }
    Err("Presentation IPC closed".into())
}
fn corrected_filter(value:&Value,order:u32)->Value{
    let Some(filter)=value.as_str()else{return value.clone()};
    let field=match order{2|5=>"tff",3|4=>"bff",_=>return value.clone()};
    if let Some(rest)=filter.strip_prefix("lavfi=[") {json!(format!("lavfi=[setfield={field},{rest}"))}
    else if let Some(rest)=filter.strip_prefix("lavfi-bwdif="){json!(format!("lavfi=[setfield={field},bwdif={rest}]"))}
    else{value.clone()}
}
struct Session {
    child:Child,bridge:Option<HostBridge>,audio:Option<wsl_audio::AudioHost>,ipc:PathBuf,offset:f64,ready:bool,field_applied:u32,
}
impl Session {
    fn start(args:&[OsString],source:&Path,ipc:PathBuf,offset:f64,live:bool,control:Control)->Result<Self>{
        let _=fs::remove_file(&ipc);
        let mut command=Command::new(a865r_media::color::player_path());
        configure_clock(&mut command,ipc.parent().ok_or("Missing presentation directory")?)?;
        // An explicit AO or OVS_WSL_AUDIO=sdl keeps the diagnostic/legacy path.
        let native_audio=std::env::var("OVS_WSL_AUDIO").as_deref()!=Ok("sdl") &&
            !args.iter().any(|arg|arg=="--ao" || arg.to_string_lossy().starts_with("--ao="));
        let audio=if native_audio {
            match wsl_audio::AudioHost::start(&mut command,ipc.parent().unwrap()) {
                Ok(host)=>Some(host),
                Err(error)=>{eprintln!("[DEBUG]: Windows WASAPI not available ({error}); trying WSLg SDL audio");None}
            }
        }else{None};
        // WSLg's PulseAudio latency reports can jump and drag the video clock
        // backwards. SDL clocks the locally queued samples instead. Preserve
        // caller overrides (diagnostics, custom devices), and retain a real
        // audio fallback on mpv builds without SDL support.
        command.arg("--ao=sdl,pulse").args(args).args(["--demuxer-lavf-format=nut","--hwdec=no","--cache=yes",
            "--cache-secs=1","--demuxer-max-bytes=150MiB","--demuxer-max-back-bytes=1MiB"])
            .arg(format!("--input-ipc-server={}",ipc.display())).arg("-")
            .stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::inherit());
        let mut child=command.spawn().map_err(|e|e.to_string())?;
        if let Some(host)=&audio {host.seal();}
        let bridge=match HostBridge::spawn(child.stdin.take().unwrap(),control,source,offset,live){
            Ok(bridge)=>bridge,Err(e)=>{let _=child.kill();let _=child.wait();return Err(e.to_string());}
        };
        Ok(Self{child,bridge:Some(bridge),audio,ipc,offset,ready:false,field_applied:u32::MAX})
    }
    fn restart(&mut self,args:&[OsString],source:&Path,offset:f64,live:bool,control:Control)->Result<()> {
        self.bridge.take();let _=self.child.kill();let _=self.child.wait();
        self.audio.take();
        *self=Self::start(args,source,self.ipc.clone(),offset,live,control)?;
        Ok(())
    }
    fn position(&self)->Option<f64>{request(&self.ipc,json!(["get_property","time-pos"])).ok()?.as_f64().map(|p|p+self.offset)}
}
fn configure_clock(command:&mut Command,directory:&Path)->Result<()> {
    #[cfg(target_env="gnu")]
    {
        // This executable is WSL-only. Keep the compatibility library private
        // to its presentation child, including across decoder/seek restarts.
        let path=directory.join("wsl-clock.so");
        if !path.exists(){
            fs::write(&path,include_bytes!(concat!(env!("OUT_DIR"),"/wsl-clock.so"))).map_err(|e|e.to_string())?;
            fs::set_permissions(&path,fs::Permissions::from_mode(0o700)).map_err(|e|e.to_string())?;
        }
        let mut preload=path.into_os_string();
        if let Some(previous)=std::env::var_os("LD_PRELOAD").filter(|s|!s.is_empty()){
            preload.push(":");preload.push(previous);
        }
        command.env("LD_PRELOAD",preload);
    }
    Ok(())
}
impl Drop for Session {
    fn drop(&mut self){self.bridge.take();let _=self.child.kill();let _=self.child.wait();}
}
struct Cache {
    directory:PathBuf,bytes:Arc<AtomicU64>,done:Arc<AtomicBool>,error:Arc<Mutex<Option<String>>>,
    control:Control,worker:Option<thread::JoinHandle<()>>,
}
impl Cache {
    fn live(directory:PathBuf,source:&Path,control:Control)->Result<Self>{
        let mut output=File::create(source).map_err(|e|e.to_string())?;
        let bytes=Arc::new(AtomicU64::new(0));let done=Arc::new(AtomicBool::new(false));let error=Arc::new(Mutex::new(None));
        let count=bytes.clone();let finished=done.clone();let failure=error.clone();let cancel=control.cancel.clone();
        let worker=thread::spawn(move||{
            let result=(||->io::Result<()>{
                let stdin=io::stdin();let mut input=stdin.lock();
                let fd=input.as_raw_fd();let flags=unsafe{libc::fcntl(fd,libc::F_GETFL)};
                if flags<0||unsafe{libc::fcntl(fd,libc::F_SETFL,flags|libc::O_NONBLOCK)}<0{return Err(io::Error::last_os_error());}
                let mut buffer=vec![0u8;256*1024];
                while !cancel.load(Ordering::Relaxed){
                    match input.read(&mut buffer){
                        Ok(0)=>break,Ok(n)=>{
                            // A per-session bound protects the disk. Stop clearly
                            // instead of truncating a source that is being sought.
                            if count.load(Ordering::Relaxed)+n as u64>16*1024*1024*1024{return Err(io::Error::other("WSL compressed playback cache reached its 16 GiB limit"));}
                            output.write_all(&buffer[..n])?;count.fetch_add(n as u64,Ordering::Release);
                        },Err(e) if e.kind()==io::ErrorKind::WouldBlock=>thread::sleep(Duration::from_millis(5)),Err(e)=>return Err(e),
                    }
                }
                Ok(())
            })();
            if let Err(e)=result{*failure.lock().unwrap()=Some(e.to_string());}
            finished.store(true,Ordering::Release);
        });
        Ok(Self{directory,bytes,done,error,control,worker:Some(worker)})
    }
}
impl Drop for Cache {
    fn drop(&mut self){self.control.cancel.store(true,Ordering::Relaxed);if let Some(worker)=self.worker.take(){let _=worker.join();}let _=fs::remove_file(self.directory.join("capture.ts"));let _=fs::remove_dir(&self.directory);}
}
struct Request {value:Value,reply:mpsc::Sender<Value>}
fn response(request:&Value,result:Result<Value>)->Value{
    match result{Ok(data)=>json!({"request_id":request["request_id"],"error":"success","data":data}),
        Err(error)=>json!({"request_id":request["request_id"],"error":error})}
}
pub fn run()->Result<()> {
    if !a865r_media::wsl_video::is_wsl(){return Err("The Windows video bridge is only for WSL".into());}
    unsafe{libc::signal(libc::SIGTERM,stop_signal as *const () as usize);libc::signal(libc::SIGINT,stop_signal as *const () as usize);
        libc::prctl(libc::PR_SET_PDEATHSIG,libc::SIGTERM);}
    let mut args:Vec<OsString>=std::env::args_os().skip(1).collect();
    let source=args.pop().ok_or("Missing playback source")?;
    let live=source=="-";
    let stamp=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let directory=std::env::temp_dir().join(format!("open-volar-s-wsl-{}-{stamp}",std::process::id()));
    fs::create_dir(&directory).map_err(|e|e.to_string())?;
    fs::set_permissions(&directory,fs::Permissions::from_mode(0o700)).map_err(|e|e.to_string())?;
    let mut public_ipc=None;let private_ipc=directory.join("presentation.sock");
    let mut properties:BTreeMap<String,Value>=BTreeMap::new();let mut filter=json!("");
    args.retain(|arg|{
        let text=arg.to_string_lossy();
        if let Some(value)=text.strip_prefix("--input-ipc-server="){public_ipc=Some(PathBuf::from(value));return false;}
        if let Some(value)=text.strip_prefix("--vf="){filter=json!(value);}
        true
    });
    let control=Control::default();
    let source=if live{directory.join("capture.ts")}else{PathBuf::from(source).canonicalize().map_err(|e|e.to_string())?};
    let cache=if live{Some(Cache::live(directory.clone(),&source,control.clone())?)}else{None};
    let began=Instant::now();
    if let Some(cache)=&cache {
        while cache.bytes.load(Ordering::Acquire)<256*1024 && !cache.done.load(Ordering::Acquire) {
            if STOP.load(Ordering::Relaxed){return Ok(());}
            thread::sleep(Duration::from_millis(10));
        }
    }
    let (sender,requests)=mpsc::channel::<Request>();
    let listener=if let Some(path)=&public_ipc{
        let _=fs::remove_file(path);let socket=UnixListener::bind(path).map_err(|e|e.to_string())?;
        fs::set_permissions(path,fs::Permissions::from_mode(0o600)).map_err(|e|e.to_string())?;
        socket.set_nonblocking(true).map_err(|e|e.to_string())?;Some(socket)
    }else{None};
    let cancelled=control.cancel.clone();
    let ipc_thread=thread::spawn(move||{
        let Some(listener)=listener else{return};
        while !cancelled.load(Ordering::Relaxed){
            match listener.accept(){
                Ok((mut stream,_))=>{
                    let _=stream.set_read_timeout(Some(Duration::from_millis(500)));
                    let _=stream.set_write_timeout(Some(Duration::from_millis(500)));
                    let mut line=String::new();
                    if BufReader::new(&mut stream).read_line(&mut line).is_ok(){
                        if let Ok(value)=serde_json::from_str(&line){
                            let (reply,receive)=mpsc::channel();
                            if sender.send(Request{value,reply}).is_ok(){
                                if let Ok(value)=receive.recv_timeout(Duration::from_secs(5)){let _=writeln!(stream,"{value}");}
                            }
                        }
                    }
                },Err(e) if e.kind()==io::ErrorKind::WouldBlock=>thread::sleep(Duration::from_millis(5)),Err(_)=>break,
            }
        }
    });
    let result=(||->Result<()>{
        let mut session=Session::start(&args,&source,private_ipc.clone(),0.,live,control.clone())?;
        let mut duration=0.0;let mut last_position=0.0;
        while !STOP.load(Ordering::Relaxed){
            if let Some(cache)=&cache{if let Some(error)=cache.error.lock().unwrap().as_ref(){return Err(error.clone());}}
            if let Some(status)=session.child.try_wait().map_err(|e|e.to_string())?{
                if !live && status.success(){return Ok(());}
                return Err(format!("Linux presentation exited: {status}"));
            }
            if let Err(error)=session.bridge.as_mut().unwrap().check(){
                // A finite playback/--frames limit closes its pipe just before
                // mpv reports successful exit. This is normal cancellation.
                thread::sleep(Duration::from_millis(50));
                if !live && session.child.try_wait().ok().flatten().is_some_and(|s|s.success()){return Ok(());}
                return Err(error.to_string());
            }
            let info=session.bridge.as_ref().unwrap().info.lock().unwrap().clone();
            if info.duration>0.0{duration=info.duration;}
            if live{duration=began.elapsed().as_secs_f64();}
            if !session.ready && private_ipc.exists(){
                session.ready=true;
                for (name,value) in &properties {let _=request(&private_ipc,json!(["set_property",name,value]));}
            }
            if session.ready && session.field_applied!=info.field_order {
                let corrected=corrected_filter(&filter,info.field_order);
                if request(&private_ipc,json!(["vf","set",corrected])).is_ok(){session.field_applied=info.field_order;}
            }
            if let Some(status)=session.child.try_wait().map_err(|e|e.to_string())?{
                if !live && status.success(){return Ok(());}
                return Err(format!("Linux presentation exited: {status}"));
            }
            while let Ok(Request{value,reply})=requests.try_recv(){
                let command=&value["command"];let verb=command[0].as_str().unwrap_or("");
                let answer=(||->Result<Value>{
                    match verb {
                        "quit"=>{STOP.store(true,Ordering::Relaxed);return Ok(Value::Null);},
                        "get_property"=>match command[1].as_str().unwrap_or(""){
                            "duration"=>return Ok(json!(duration)),
                            "time-pos"=>{if let Some(position)=session.position(){last_position=position;}return Ok(json!(last_position));},
                            "demuxer-cache-state" if live=>return Ok(json!({"seekable-ranges":[{"start":0.,"end":duration}]})),
                            "hwdec-current"=>return Ok(json!(format!("Windows {}",info.backend))),
                            _=>{},
                        },
                        "seek"=>{
                            let amount=command[1].as_f64().ok_or("Invalid seek time")?;
                            if !amount.is_finite(){return Err("Invalid seek time".into());}
                            let mode=command[2].as_str().unwrap_or("relative");
                            let target=if mode.contains("absolute"){amount}else{session.position().unwrap_or(last_position)+amount};
                            let target=target.clamp(0.,(duration-if live{1.5}else{0.05}).max(0.));
                            // Drop the old raw stream before starting the new NUT
                            // header. Source capture runs independently throughout.
                            session.restart(&args,&source,target,live,control.clone())?;
                            last_position=target;return Ok(Value::Null);
                        },
                        "set_property"=>{
                            let name=command[1].as_str().unwrap_or("");
                            // Compressed source lives on disk; raw-frame cache must
                            // stay small even while recording or rewound.
                            if ["cache-on-disk","demuxer-max-back-bytes","demuxer-max-bytes","cache-secs"].contains(&name){return Ok(Value::Null);}
                            if name=="hwdec"{return Ok(Value::Null);}
                            properties.insert(name.into(),command[2].clone());
                        },
                        "vf" if command[1]=="set"=>{
                            filter=command[2].clone();return request(&private_ipc,json!(["vf","set",corrected_filter(&filter,info.field_order)]));
                        },_=>{},
                    }
                    request(&private_ipc,command.clone())
                })();
                let failed=answer.is_err();let _=reply.send(response(&value,answer));
                if failed && verb=="seek"{return Err("Could not restart hardware decoder for seek".into());}
            }
            thread::sleep(Duration::from_millis(10));
        }
        Ok(())
    })();
    control.cancel.store(true,Ordering::Relaxed);let _=ipc_thread.join();
    if let Some(path)=public_ipc{let _=fs::remove_file(path);}
    let _=fs::remove_file(private_ipc);
    let _=fs::remove_file(directory.join("wsl-clock.so"));
    let _=fs::remove_file(directory.join("wsl-audio.so"));
    drop(cache);let _=fs::remove_dir(&directory);
    result
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test] fn raw_video_preserves_field_order_without_deinterlacing_progressive_sources(){
        let filter=json!("lavfi=[bwdif=mode=send_field:deint=interlaced]");
        assert_eq!(corrected_filter(&filter,1),filter);
        assert_eq!(corrected_filter(&filter,2),json!("lavfi=[setfield=tff,bwdif=mode=send_field:deint=interlaced]"));
        assert_eq!(corrected_filter(&filter,4),json!("lavfi=[setfield=bff,bwdif=mode=send_field:deint=interlaced]"));
    }
}
