//! WSL-only Windows GPU decode bridge. Native Linux never launches a PE helper.
use crate::playback::Control;
use std::{io::{self,Read,Write,BufRead,BufReader},path::PathBuf,process::{Child,ChildStdin,Command,Stdio},
    os::fd::AsRawFd,sync::{Arc,Mutex,atomic::{AtomicBool,AtomicU32,Ordering}},thread,time::{Duration,Instant}};
pub fn is_wsl()->bool {
    std::env::var_os("WSL_DISTRO_NAME").is_some() || std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .is_ok_and(|s|s.to_ascii_lowercase().contains("microsoft"))
}
pub fn player_command()->Command {
    Command::new(if is_wsl(){std::env::var_os("OVS_WSL_PLAYER").unwrap_or_else(||"/usr/bin/open-volar-s-wsl-player".into())}
        else{crate::color::player_path().into_os_string()})
}
pub fn stop_player(child:&mut Child) {
    if is_wsl() {
        unsafe{libc::kill(child.id() as i32,libc::SIGTERM);}
        let until=Instant::now()+Duration::from_secs(2);
        while Instant::now()<until {
            if child.try_wait().ok().flatten().is_some(){return;}
            thread::sleep(Duration::from_millis(10));
        }
    }
    let _=child.kill();let _=child.wait();
}
#[derive(Default,Clone)]
pub struct HostInfo {pub duration:f64,pub field_order:u32,pub backend:String}
fn helper_path()->io::Result<PathBuf>{
    let path=std::env::var_os("OVS_WSL_VIDEO_HOST").map(PathBuf::from)
        .unwrap_or_else(||PathBuf::from("/usr/lib/open-volar-s/wsl-video-host/open-volar-s-wsl-video-host.exe"));
    if path.is_file(){Ok(path)}else{Err(io::Error::new(io::ErrorKind::NotFound,
        "WSL hardware decoder helper is missing. Install the WSL video host bundle, or set OVS_WSL_VIDEO_HOST to its .exe path."))}
}
fn nonblocking(fd:i32)->io::Result<()> {
    let flags=unsafe{libc::fcntl(fd,libc::F_GETFL)};
    if flags<0 || unsafe{libc::fcntl(fd,libc::F_SETFL,flags|libc::O_NONBLOCK)}<0 {return Err(io::Error::last_os_error());}
    Ok(())
}
fn wait_io(fd:i32,events:i16)->io::Result<()> {
    let mut descriptor=libc::pollfd{fd,events,revents:0};
    let result=unsafe{libc::poll(&mut descriptor,1,10)};
    if result<0 {
        let error=io::Error::last_os_error();
        if error.kind()!=io::ErrorKind::Interrupted{return Err(error);}
    }
    Ok(())
}
pub struct HostBridge {
    child:Child,quit:Arc<AtomicBool>,pid:Arc<AtomicU32>,error:Arc<Mutex<Option<String>>>,
    relay:Option<thread::JoinHandle<()>>,log:Option<thread::JoinHandle<()>>,
    pub info:Arc<Mutex<HostInfo>>,
}
impl HostBridge {
    pub fn spawn(mut presentation:ChildStdin,control:Control,source:&std::path::Path,seek:f64,follow:bool)->io::Result<Self> {
        let path=Command::new("wslpath").arg("-w").arg(source.canonicalize()?).output()?;
        if !path.status.success(){return Err(io::Error::other("Cannot resolve the WSL media path for Windows"));}
        let path=String::from_utf8_lossy(&path.stdout).trim().to_owned();
        let listener=std::net::TcpListener::bind("0.0.0.0:0")?;
        listener.set_nonblocking(true)?;
        let addresses=Command::new("hostname").arg("-I").output()?;
        let address=String::from_utf8_lossy(&addresses.stdout).split_whitespace()
            .find_map(|s|s.parse::<std::net::Ipv4Addr>().ok()).ok_or_else(||io::Error::other("Cannot find WSL IPv4 address"))?;
        let address=format!("{address}:{}",listener.local_addr()?.port());
        let mut secret=[0u8;32];std::fs::File::open("/dev/urandom")?.read_exact(&mut secret)?;
        let token=secret.iter().map(|b|format!("{b:02x}")).collect::<String>();
        let mut child=Command::new(helper_path()?).args(["--backend","auto"])
            .args(["--input",&path,"--seek",&seek.to_string(),"--follow",if follow{"yes"}else{"no"},"--liveness","yes"])
            .args(["--connect",&address,"--token",&token])
            .stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped()).spawn()?;
        let began=Instant::now();
        let mut frames=loop {
            match listener.accept(){
                Ok((mut stream,_))=>{
                    stream.set_read_timeout(Some(Duration::from_secs(1)))?;
                    let mut received=[0u8;32];
                    if stream.read_exact(&mut received).is_ok()&&received==secret {
                        stream.set_read_timeout(None)?;stream.set_nodelay(true)?;break stream;
                    }
                },Err(e) if e.kind()==io::ErrorKind::WouldBlock=>{},Err(e)=>return Err(e),
            }
            if let Some(status)=child.try_wait()? {return Err(io::Error::other(format!("Windows decoder did not connect: {status}")));}
            if began.elapsed()>Duration::from_secs(10){child.stdin.take();let _=child.kill();let _=child.wait();return Err(io::Error::new(io::ErrorKind::TimedOut,"Windows decoder could not connect to WSL"));}
            thread::sleep(Duration::from_millis(10));
        };
        drop(listener);
        let diagnostics=child.stderr.take().unwrap();
        // Raw HD frames need fewer cross-process wakeups than the default
        // small Linux pipe permits. Failure is harmless on restricted kernels.
        unsafe{libc::fcntl(presentation.as_raw_fd(),libc::F_SETPIPE_SZ,1024*1024);}
        nonblocking(frames.as_raw_fd())?;nonblocking(presentation.as_raw_fd())?;
        let quit=Arc::new(AtomicBool::new(false));let pid=Arc::new(AtomicU32::new(0));
        let error=Arc::new(Mutex::new(None));let failed=error.clone();let stopped=quit.clone();
        let relay=thread::spawn(move||{
            let result=(||->io::Result<()>{
                let mut buffer=vec![0u8;256*1024];
                let mut progress=Instant::now();
                while !stopped.load(Ordering::Relaxed) && !control.cancel.load(Ordering::Relaxed) {
                    let count=match frames.read(&mut buffer){
                        Ok(0)=>return Ok(()),Ok(n)=>n,
                        Err(e) if e.kind()==io::ErrorKind::WouldBlock=>{
                            if progress.elapsed()>Duration::from_secs(30){return Err(io::Error::new(io::ErrorKind::TimedOut,"Windows hardware decoder stopped producing frames"));}
                            wait_io(frames.as_raw_fd(),libc::POLLIN)?;continue;
                        },Err(e)=>return Err(e),
                    };
                    progress=Instant::now();let mut offset=0;
                    while offset<count {
                        if stopped.load(Ordering::Relaxed)||control.cancel.load(Ordering::Relaxed){return Ok(());}
                        match presentation.write(&buffer[offset..count]) {
                            Ok(0)=>return Err(io::Error::new(io::ErrorKind::BrokenPipe,"Linux presentation stopped")),
                            Ok(n)=>{offset+=n;},
                            Err(e) if e.kind()==io::ErrorKind::WouldBlock=>{wait_io(presentation.as_raw_fd(),libc::POLLOUT)?;},
                            Err(e)=>return Err(e),
                        }
                    }
                }
                Ok(())
            })();
            if let Err(e)=result { *failed.lock().unwrap()=Some(e.to_string()); }
        });
        let host_pid=pid.clone();let failed=error.clone();let info=Arc::new(Mutex::new(HostInfo::default()));let details=info.clone();
        let log=thread::spawn(move||{
            for line in BufReader::new(diagnostics).lines().map_while(Result::ok) {
                if let Some(value)=line.strip_prefix("[DEBUG]: host process ID: ") {
                    if let Ok(value)=value.parse(){host_pid.store(value,Ordering::Release);}
                }
                if line.starts_with("[ERROR]:") {*failed.lock().unwrap()=Some(line.clone());}
                if let Some(value)=line.strip_prefix("[DEBUG]: source duration: "){details.lock().unwrap().duration=value.parse().unwrap_or(0.);}
                if let Some(value)=line.strip_prefix("[DEBUG]: source field order: "){details.lock().unwrap().field_order=value.parse().unwrap_or(0);}
                if let Some(value)=line.strip_prefix("[DEBUG]: hardware decoder active: "){details.lock().unwrap().backend=value.to_owned();}
                eprintln!("{line}");
            }
        });
        Ok(Self{child,quit,pid,error,relay:Some(relay),log:Some(log),info})
    }
    pub fn check(&mut self)->io::Result<()> {
        if let Some(error)=self.error.lock().unwrap().as_ref(){return Err(io::Error::other(error.clone()));}
        if let Some(status)=self.child.try_wait()? {if !status.success(){return Err(io::Error::other(format!("Windows hardware decoder exited ({status}); see console diagnostics")));}}
        Ok(())
    }
    fn shutdown(&mut self) {
        // Close liveness before the raw-video pipe, so an ordinary seek/stop
        // is not reported by the host as a failed decoder output.
        self.child.stdin.take();
        // WSL's Linux interop PID differs from the Windows process PID. Kill the
        // exact helper which announced its identity, never other player processes.
        if self.child.try_wait().ok().flatten().is_none() {
            let pid=self.pid.load(Ordering::Acquire);
            if pid!=0 {let _=Command::new("/mnt/c/Windows/System32/taskkill.exe")
                .args(["/PID",&pid.to_string(),"/F"]).stdout(Stdio::null()).stderr(Stdio::null()).status();}
            let _=self.child.kill();
        }
        self.quit.store(true,Ordering::Relaxed);
        let _=self.child.wait();
        if let Some(thread)=self.relay.take(){let _=thread.join();}
        if let Some(thread)=self.log.take(){let _=thread.join();}
    }
}
impl Drop for HostBridge {fn drop(&mut self){self.shutdown();}}
