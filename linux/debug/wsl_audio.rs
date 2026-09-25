//! Lifetime and authenticated transport for the WSL-only Windows audio output.
use std::{fs,io::{self,Read},net::{TcpListener,TcpStream,Shutdown},os::fd::AsRawFd,
    path::{Path,PathBuf},process::{Child,Command,Stdio},thread,time::{Duration,Instant}};
pub struct AudioHost { child:Child, stream:Option<TcpStream> }
impl AudioHost {
    pub fn seal(&self) {
        // Only the presentation child inherits this descriptor. In particular,
        // the separate video decoder must not keep the audio socket alive.
        if let Some(stream)=&self.stream {unsafe{libc::fcntl(stream.as_raw_fd(),libc::F_SETFD,libc::FD_CLOEXEC);}}
    }
    pub fn start(command:&mut Command,directory:&Path)->io::Result<Self> {
        let helper=std::env::var_os("OVS_WSL_VIDEO_HOST").map(PathBuf::from)
            .unwrap_or_else(||"/usr/lib/open-volar-s/wsl-video-host/open-volar-s-wsl-video-host.exe".into());
        let listener=TcpListener::bind("0.0.0.0:0")?;
        listener.set_nonblocking(true)?;
        let addresses=Command::new("hostname").arg("-I").output()?;
        let address=String::from_utf8_lossy(&addresses.stdout).split_whitespace()
            .find_map(|s|s.parse::<std::net::Ipv4Addr>().ok()).ok_or_else(||io::Error::other("Missing WSL IPv4 address"))?;
        let address=format!("{address}:{}",listener.local_addr()?.port());
        let mut secret=[0u8;32];fs::File::open("/dev/urandom")?.read_exact(&mut secret)?;
        let token=secret.iter().map(|b|format!("{b:02x}")).collect::<String>();
        let child=Command::new(helper).args(["--audio","yes","--connect",&address,"--token",&token,"--liveness","yes"])
            .stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::inherit()).spawn()?;
        let mut host=Self{child,stream:None};
        let began=Instant::now();
        let mut stream=loop {
            match listener.accept() {
                Ok((mut stream,_))=>{
                    stream.set_read_timeout(Some(Duration::from_secs(1)))?;
                    let mut received=[0;32];
                    if stream.read_exact(&mut received).is_ok() && received==secret{break stream;}
                },Err(e) if e.kind()==io::ErrorKind::WouldBlock=>{},Err(e)=>return Err(e),
            }
            if let Some(status)=host.child.try_wait()?{return Err(io::Error::other(format!("Windows audio helper exited: {status}")));}
            if began.elapsed()>Duration::from_secs(10){return Err(io::Error::new(io::ErrorKind::TimedOut,"Windows audio helper connection timed out"));}
            thread::sleep(Duration::from_millis(10));
        };
        stream.set_read_timeout(Some(Duration::from_secs(3)))?;
        stream.set_write_timeout(Some(Duration::from_secs(3)))?;
        stream.set_nodelay(true)?;
        let mut header=[0;8];stream.read_exact(&mut header)?;
        if header!=[0x4f,0x56,0x53,0x41,1,0,0,0]{return Err(io::Error::other("Windows audio protocol mismatch"));}
        let fd=stream.as_raw_fd();
        if unsafe{libc::fcntl(fd,libc::F_SETFD,0)}<0{return Err(io::Error::last_os_error());}
        let path=directory.join("wsl-audio.so");
        fs::write(&path,include_bytes!(concat!(env!("OUT_DIR"),"/wsl-audio.so")))?;
        let mut preload=path.into_os_string();
        if let Some(value)=command.get_envs().find_map(|(key,value)|(key=="LD_PRELOAD").then_some(value).flatten()) {
            preload.push(":");preload.push(value);
        }
        command.env("LD_PRELOAD",preload).env("OVS_WSL_AUDIO_FD",fd.to_string())
            .env("SDL_AUDIODRIVER","dummy");
        host.stream=Some(stream);
        Ok(host)
    }
}
impl Drop for AudioHost {
    fn drop(&mut self) {
        if let Some(stream)=&self.stream {let _=stream.shutdown(Shutdown::Both);}
        self.child.stdin.take();
        let until=Instant::now()+Duration::from_secs(2);
        while Instant::now()<until {
            if self.child.try_wait().ok().flatten().is_some(){return;}
            thread::sleep(Duration::from_millis(10));
        }
        let _=self.child.kill();let _=self.child.wait();
    }
}
