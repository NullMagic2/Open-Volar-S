//! Linux mpv host used only by authenticated Wine clients.
use crate::{Error,Result};
use serde_json::{json,Value};
use std::{io::{BufRead,BufReader,Read,Write},os::unix::net::UnixStream,path::PathBuf,process::{Child,Command,Stdio},time::Duration};
unsafe extern "C" {
    fn XInitThreads()->i32;
    fn XSetErrorHandler(handler:Option<unsafe extern "C" fn(*mut std::ffi::c_void,*mut XErrorEvent)->i32>)->Option<unsafe extern "C" fn(*mut std::ffi::c_void,*mut XErrorEvent)->i32>;
    fn XOpenDisplay(name:*const i8)->*mut std::ffi::c_void;
    fn XCreateSimpleWindow(d:*mut std::ffi::c_void,p:u64,x:i32,y:i32,w:u32,h:u32,b:u32,bc:u64,bg:u64)->u64;
    fn XRaiseWindow(d:*mut std::ffi::c_void,w:u64)->i32;
    fn XMapWindow(d:*mut std::ffi::c_void,w:u64)->i32;
    fn XMoveResizeWindow(d:*mut std::ffi::c_void,w:u64,x:i32,y:i32,width:u32,height:u32)->i32;
    fn XDestroyWindow(d:*mut std::ffi::c_void,w:u64)->i32;
    fn XCloseDisplay(d:*mut std::ffi::c_void)->i32;
    fn XFlush(d:*mut std::ffi::c_void)->i32;
}
#[link(name="X11")] unsafe extern "C" {}
#[repr(C)]
struct XErrorEvent { kind:i32,display:*mut std::ffi::c_void,resource:u64,serial:u64,error:u8,request:u8,minor:u8 }
unsafe extern "C" fn display_error(_: *mut std::ffi::c_void,event:*mut XErrorEvent)->i32 {
    // Wine can destroy the parent before the playback connection is dropped.
    // BadWindow must not let Xlib terminate every tuner session in the helper.
    if (*event).error!=3 {eprintln!("Wine playback X11 error {} in request {}",(*event).error,(*event).request);}
    0
}
pub fn initialize(){unsafe{XInitThreads();XSetErrorHandler(Some(display_error));}}
#[derive(Default)]
pub struct Player {child:Option<Child>,ipc:Option<BufReader<UnixStream>>,socket:PathBuf,display:*mut std::ffi::c_void,window:u64,serial:u64}
impl Drop for Player {
    fn drop(&mut self) {
        if let Some(child)=&mut self.child {let _=child.kill();let _=child.wait();}
        if self.window!=0 {unsafe{XDestroyWindow(self.display,self.window);XCloseDisplay(self.display);}}
        if !self.socket.as_os_str().is_empty(){let _=std::fs::remove_file(&self.socket);}
    }
}
impl Player {
    pub fn request(&mut self,op:u8,data:&[u8])->Result<Vec<u8>> {
        let request:Value=serde_json::from_slice(data).map_err(|e|Error::InvalidArgument(e.to_string()))?;
        let result=match op {6=>self.start(&request)?,7=>self.command(request)?,8=>{self.resize(&request)?;Value::Null},_=>unreachable!()};
        serde_json::to_vec(&result).map_err(|e|Error::Protocol(e.to_string()))
    }
    fn resize(&mut self,v:&Value)->Result<()> {
        if self.window==0 {return Err(Error::NotConnected);}
        let (x,y,w,h)=(v["x"].as_i64().unwrap_or(0),v["y"].as_i64().unwrap_or(0),v["width"].as_u64().unwrap_or(1).clamp(1,16384),v["height"].as_u64().unwrap_or(1).clamp(1,16384));
        unsafe{XMoveResizeWindow(self.display,self.window,x as i32,y as i32,w as u32,h as u32);XRaiseWindow(self.display,self.window);XFlush(self.display);}
        Ok(())
    }
    fn start(&mut self,v:&Value)->Result<Value> {
        if self.child.is_some(){return Err(Error::InvalidArgument("Player already started".into()));}
        let source=v["source"].as_str().ok_or_else(||Error::InvalidArgument("Missing playback source".into()))?;
        // Only local files or the app's loopback TS stream are accepted.
        if !source.starts_with('/') && !source.starts_with("tcp://127.0.0.1:"){return Err(Error::InvalidArgument("Playback source must be local".into()));}
        let parent=v["parent"].as_u64().filter(|p|*p!=0).ok_or_else(||Error::Unsupported("Use Wine's X11 driver for embedded playback".into()))?;
        self.display=unsafe{XOpenDisplay(std::ptr::null())};
        if self.display.is_null(){return Err(Error::Unsupported("Linux X11 display unavailable".into()));}
        self.window=unsafe{XCreateSimpleWindow(self.display,parent,0,0,1,1,0,0,0)};
        self.resize(v)?;
        unsafe{XMapWindow(self.display,self.window);XRaiseWindow(self.display,self.window);XFlush(self.display);}
        let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        self.socket=std::env::temp_dir().join(format!("ovs-wine-player-{}-{stamp}.sock",std::process::id()));
        let mut cmd=Command::new("mpv");
        cmd.args(["--no-config","--idle=yes","--keep-open=yes","--hwdec=no","--vo=gpu-next","--gpu-sw=yes","--gpu-api=opengl","--gpu-context=x11egl","--osc=no","--input-default-bindings=no","--input-terminal=no","--terminal=no","--audio-client-name=Open Volar S (Wine)"])
            .arg(format!("--log-file={}",self.socket.with_extension("log").display())).arg(format!("--wid={}",self.window)).arg(format!("--input-ipc-server={}",self.socket.display()))
            .arg(format!("--volume={}",v["volume"].as_u64().unwrap_or(80).min(100)))
            .arg(format!("--deinterlace={}",if v["deinterlace"].as_bool().unwrap_or(true){"yes"}else{"no"}));
        if v["follow"].as_bool()==Some(true){cmd.arg("--stream-lavf-o=follow=1");}
        cmd.arg("--").arg(source).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        self.child=Some(cmd.spawn().map_err(|e|Error::Transport(format!("Install native Linux mpv for Wine playback: {e}")))?);
        for _ in 0..100 {
            if let Ok(stream)=UnixStream::connect(&self.socket) {
                stream.set_read_timeout(Some(Duration::from_secs(3)))?;stream.set_write_timeout(Some(Duration::from_secs(3)))?;
                self.ipc=Some(BufReader::new(stream));return Ok(json!({"backend":"Wine / Linux mpv","window":self.window}));
            }
            if self.child.as_mut().unwrap().try_wait()?.is_some(){return Err(Error::Transport("Linux mpv exited during startup".into()));}
            std::thread::sleep(Duration::from_millis(30));
        }
        Err(Error::Transport("Linux mpv IPC startup timed out".into()))
    }
    fn command(&mut self,mut v:Value)->Result<Value> {
        // No shell or arbitrary process commands: expose only the player's UI controls.
        let command=v["command"][0].as_str().unwrap_or("");
        if !["get_property","set_property","cycle","seek","frame-step","frame-back-step","screenshot-to-file","quit"].contains(&command){return Err(Error::InvalidArgument("Unsupported player command".into()));}
        self.serial+=1;v["request_id"]=json!(self.serial);
        let ipc=self.ipc.as_mut().ok_or(Error::NotConnected)?;
        writeln!(ipc.get_mut(),"{v}")?;
        for _ in 0..256 {
            let mut line=String::new();if (&mut *ipc).take(65_537).read_line(&mut line)?==0{return Err(Error::NotConnected);}
            if line.len()>65_536{return Err(Error::Protocol("Player reply exceeds limit".into()));}
            if let Ok(reply)=serde_json::from_str::<Value>(&line) {if reply["request_id"].as_u64()==Some(self.serial){return Ok(reply);}}
        }
        Err(Error::Transport("No matching player response".into()))
    }
}
