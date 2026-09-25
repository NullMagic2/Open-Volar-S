//! Authenticated loopback transport for Windows clients running under Wine.
//! The native server retains the Linux driver's permissions and exclusive ownership.
use super::{BulkPipe, Transport};
use crate::{Error, Result};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[cfg(target_os="linux")]
#[path="../../../../linux/wine/player.rs"]
mod player;

const MAGIC: &str = "OVS-WINE-1";
const MAX_FRAME: usize = 65_536;
fn invalid(s: &str) -> Error { Error::Transport(format!("Wine bridge: {s}")) }
fn frame_write(stream: &mut TcpStream, bytes: &[u8]) -> Result<()> {
    if bytes.len() > MAX_FRAME { return Err(invalid("frame too large")); }
    stream.write_all(&(bytes.len() as u32).to_le_bytes())?;
    stream.write_all(bytes)?;
    Ok(())
}
fn frame_read(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let mut size = [0; 4]; stream.read_exact(&mut size)?;
    let size = u32::from_le_bytes(size) as usize;
    if size == 0 || size > MAX_FRAME { return Err(invalid("invalid frame length")); }
    let mut bytes = vec![0; size]; stream.read_exact(&mut bytes)?; Ok(bytes)
}
fn configuration(path: &Path) -> Result<(SocketAddr, String)> {
    if std::fs::metadata(path)?.len() > 1024 { return Err(invalid("configuration too large")); }
    let text = std::fs::read_to_string(path)?;
    let lines: Vec<_> = text.lines().collect();
    if lines.len() != 3 || lines[0] != MAGIC { return Err(invalid("invalid configuration version")); }
    let addr: SocketAddr = lines[1].parse().map_err(|_| invalid("invalid server address"))?;
    if !addr.ip().is_loopback() || addr.port() == 0 { return Err(invalid("only loopback servers are allowed")); }
    if lines[2].len() != 64 || !lines[2].bytes().all(|c| c.is_ascii_hexdigit()) { return Err(invalid("invalid authentication token")); }
    Ok((addr, lines[2].to_owned()))
}

/// Detect Wine without changing native Windows behavior.
#[cfg(windows)]
pub fn is_wine() -> bool {
    unsafe extern "system" {
        fn GetModuleHandleA(name: *const u8) -> *mut std::ffi::c_void;
        fn GetProcAddress(module: *mut std::ffi::c_void, name: *const u8) -> *mut std::ffi::c_void;
    }
    unsafe {
        let module = GetModuleHandleA(c"ntdll.dll".as_ptr().cast());
        !module.is_null() && !GetProcAddress(module, c"wine_get_version".as_ptr().cast()).is_null()
    }
}

/// Windows client; usable on Linux too for transport contract tests.
pub struct WineTransport {
    config: PathBuf,
    stream: Option<TcpStream>,
    pipes: Vec<BulkPipe>,
}
impl WineTransport {
    /// Create a client with a private configuration file written by the Linux launcher.
    pub fn new(config: PathBuf) -> Self { Self { config, stream: None, pipes: Vec::new() } }
    pub fn call(&mut self, op: u8, payload: &[u8]) -> Result<Vec<u8>> {
        let stream = self.stream.as_mut().ok_or(Error::NotConnected)?;
        let mut request = vec![op]; request.extend_from_slice(payload);
        frame_write(stream, &request)?;
        let reply = frame_read(stream)?;
        if reply[0] != 0 { return Err(invalid(&String::from_utf8_lossy(&reply[1..]))); }
        Ok(reply[1..].to_vec())
    }
}
impl WineTransport {
    pub fn connect(&mut self) -> Result<()> {
        self.stream = None; self.pipes.clear();
        #[cfg(windows)]
        ensure_helper(&self.config)?;
        let (address, token) = configuration(&self.config).map_err(|e| invalid(&format!("{e}; launch with open-volar-s-wine on Linux")))?;
        let stream = TcpStream::connect_timeout(&address, Duration::from_secs(3))?;
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(Duration::from_secs(25)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        self.stream = Some(stream);
        self.call(0, token.as_bytes())?;
        Ok(())
    }
}
impl Transport for WineTransport {
    fn open(&mut self, vendor: u16, product: u16) -> Result<()> {
        self.connect()?;
        let mut args = vendor.to_le_bytes().to_vec(); args.extend(product.to_le_bytes());
        let data = self.call(1, &args)?;
        if data.len() % 4 != 0 || data.len() > 64 { return Err(invalid("invalid USB endpoints")); }
        self.pipes = data.chunks_exact(4).map(|p| BulkPipe { id:p[0], input:p[1]!=0, maximum_packet_size:u16::from_le_bytes([p[2],p[3]]) }).collect();
        Ok(())
    }
    fn cycle_port(&mut self) -> Result<()> { self.call(2, &[]).map(|_| ()) }
    fn bulk_pipes(&self) -> &[BulkPipe] { &self.pipes }
    fn set_command_pipes(&mut self, out: u8, input: u8) -> Result<()> { self.call(3, &[out,input]).map(|_| ()) }
    fn exchange(&mut self, request: &[u8], expected: usize, timeout: u32) -> Result<Vec<u8>> {
        if request.len()>16_384 || expected>16_384 {return Err(invalid("command exceeds USB transfer limit"));}
        let mut args = (expected as u32).to_le_bytes().to_vec(); args.extend(timeout.to_le_bytes()); args.extend(request);
        self.call(4, &args)
    }
    fn read_bulk(&mut self, pipe: u8, size: usize, timeout: u32) -> Result<Vec<u8>> {
        let mut args=vec![pipe]; args.extend((size.min(16_384) as u32).to_le_bytes()); args.extend(timeout.to_le_bytes());
        self.call(5, &args)
    }
    fn description(&self) -> String { "Wine loopback bridge → Linux USB driver".into() }
}

fn number(bytes: &[u8], offset: usize) -> u32 { u32::from_le_bytes(bytes[offset..offset+4].try_into().unwrap()) }
fn dispatch(transport: &mut dyn Transport, request: &[u8]) -> Result<Vec<u8>> {
    match request {
        [1, a,b,c,d] => {
            transport.open(u16::from_le_bytes([*a,*b]),u16::from_le_bytes([*c,*d]))?;
            Ok(transport.bulk_pipes().iter().flat_map(|p| [p.id,p.input as u8,p.maximum_packet_size as u8,(p.maximum_packet_size>>8) as u8]).collect())
        }
        [2] => {transport.cycle_port()?; Ok(vec![])}
        [3, out, input] => {transport.set_command_pipes(*out,*input)?; Ok(vec![])}
        [4, args @ ..] if args.len()>=8 && args.len()<=16_392 && number(args,0)<=16_384 && number(args,4)<=10_000 =>
            transport.exchange(&args[8..],number(args,0) as usize,number(args,4)),
        [5, pipe, args @ ..] if args.len()==8 && number(args,0)<=16_384 && number(args,4)<=10_000 =>
            transport.read_bulk(*pipe,number(args,0) as usize,number(args,4)),
        _ => Err(invalid("invalid request or transfer bounds")),
    }
}
fn session(mut stream: TcpStream, token: &str, mut transport: Box<dyn Transport>) -> Result<()> {
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let auth = frame_read(&mut stream)?;
    if auth.len()!=65 || auth[0]!=0 || auth[1..].iter().zip(token.bytes()).fold(0u8,|v,(a,b)|v|(a^b))!=0 {
        frame_write(&mut stream,b"\x01Authentication failed")?;
        return Err(invalid("authentication failed"));
    }
    frame_write(&mut stream,&[0])?;
    stream.set_read_timeout(Some(Duration::from_secs(60)))?;
    #[cfg(target_os="linux")]
    let mut player=player::Player::default();
    loop {
        let request = frame_read(&mut stream)?;
        #[cfg(target_os="linux")]
        let result=if (6..=8).contains(&request[0]) {player.request(request[0],&request[1..])} else {dispatch(transport.as_mut(),&request)};
        #[cfg(not(target_os="linux"))]
        let result=dispatch(transport.as_mut(),&request);
        let reply = match result {
            Ok(data) => {let mut reply=vec![0]; reply.extend(data); reply},
            Err(error) => {let mut reply=vec![1]; reply.extend(error.to_string().bytes().take(2048)); reply},
        };
        frame_write(&mut stream,&reply)?;
    }
}

/// Serve Wine clients as the desktop user, with a random token in a mode-0600 file.
#[cfg(target_os="linux")]
pub fn serve(path: &Path) -> Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    use std::sync::{Arc,atomic::{AtomicUsize,Ordering}};
    player::initialize();
    let listener=std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST,0))?;
    let mut random=[0u8;32]; std::fs::File::open("/dev/urandom")?.read_exact(&mut random)?;
    let token:String=random.iter().map(|b|format!("{b:02x}")).collect();
    let mut file=std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(path)?;
    writeln!(file,"{MAGIC}\n{}\n{token}",listener.local_addr()?)?;
    file.sync_all()?;
    println!("Wine USB bridge ready on {} (desktop-user permissions)",listener.local_addr()?);
    let clients=Arc::new(AtomicUsize::new(0));
    struct ConfigGuard(PathBuf,String);
    impl Drop for ConfigGuard {fn drop(&mut self){
        if std::fs::read_to_string(&self.0).ok().is_some_and(|s|s.lines().last()==Some(self.1.as_str())) {let _=std::fs::remove_file(&self.0);}
    }}
    let _guard=ConfigGuard(path.to_owned(),token.clone());
    listener.set_nonblocking(true)?;
    let mut idle=std::time::Instant::now();
    loop {
        if clients.load(Ordering::SeqCst)>0 {idle=std::time::Instant::now();}
        let stream=match listener.accept() {
            Ok((stream,_))=>{idle=std::time::Instant::now();stream},
            Err(e) if e.kind()==std::io::ErrorKind::WouldBlock=>{
                if idle.elapsed()>Duration::from_secs(120){break;}
                std::thread::sleep(Duration::from_millis(50));continue;
            },
            Err(e)=>return Err(e.into()),
        };
        if clients.fetch_add(1,Ordering::SeqCst)>=8 { clients.fetch_sub(1,Ordering::SeqCst);continue; }
        let clients=clients.clone();let token=token.clone();
        std::thread::spawn(move || {
            // Never use default_transport here: the server must own the native device.
            let _=session(stream,&token,Box::new(super::linux::LinuxUsbTransport::new()));
            clients.fetch_sub(1,Ordering::SeqCst);
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Mock;
    impl Transport for Mock {
        fn open(&mut self,v:u16,p:u16)->Result<()> {assert_eq!((v,p),(0x07ca,0xb865));Ok(())}
        fn cycle_port(&mut self)->Result<()> {Err(Error::Unsupported("cycle test".into()))}
        fn bulk_pipes(&self)->&[BulkPipe] {&[BulkPipe{id:0x84,input:true,maximum_packet_size:512}]}
        fn set_command_pipes(&mut self,o:u8,i:u8)->Result<()> {assert_eq!((o,i),(2,0x81));Ok(())}
        fn exchange(&mut self,r:&[u8],e:usize,t:u32)->Result<Vec<u8>> {assert_eq!((r,e,t),(&[1,2][..],2,800));Ok(vec![9,8])}
        fn read_bulk(&mut self,p:u8,s:usize,t:u32)->Result<Vec<u8>> {assert_eq!((p,s,t),(0x84,16384,500));Ok(vec![0x47;188])}
        fn description(&self)->String {"mock".into()}
    }
    #[test] fn invalid_requests_cannot_reach_usb() {
        for request in [&[][..],&[4,0],&[5,0],&[0],&[255]] {assert!(dispatch(&mut Mock,request).is_err());}
        let mut request=vec![5,0x84];request.extend(16385u32.to_le_bytes());request.extend(500u32.to_le_bytes());
        assert!(dispatch(&mut Mock,&request).is_err());
    }
    #[test] fn authenticated_roundtrip_and_disconnect() {
        let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address=listener.local_addr().unwrap();let token="a".repeat(64);
        let server_token=token.clone();
        let server=std::thread::spawn(move || session(listener.accept().unwrap().0,&server_token,Box::new(Mock)));
        let path=std::env::temp_dir().join(format!("ovs-bridge-test-{}-{}.txt",std::process::id(),address.port()));
        std::fs::write(&path,format!("{MAGIC}\n{address}\n{token}\n")).unwrap();
        let mut client=WineTransport::new(path.clone());
        client.open(0x07ca,0xb865).unwrap();assert_eq!(client.bulk_pipes()[0].maximum_packet_size,512);
        client.set_command_pipes(2,0x81).unwrap();assert_eq!(client.exchange(&[1,2],2,800).unwrap(),[9,8]);
        assert_eq!(client.read_bulk(0x84,65536,500).unwrap().len(),188);
        assert!(client.cycle_port().unwrap_err().to_string().contains("cycle test"));
        drop(client); assert!(server.join().unwrap().is_err());std::fs::remove_file(path).unwrap();
    }
    #[test] fn authentication_rejected_before_device_open() {
        let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();let address=listener.local_addr().unwrap();
        let server=std::thread::spawn(move || session(listener.accept().unwrap().0,&"a".repeat(64),Box::new(Mock)));
        let mut client=TcpStream::connect(address).unwrap();frame_write(&mut client,b"\x00wrong").unwrap();
        assert_eq!(frame_read(&mut client).unwrap()[0],1);assert!(server.join().unwrap().is_err());
    }
}

/// Translate a Wine path without guessing the prefix or drive mappings.
#[cfg(windows)]
pub fn unix_path(path: &Path) -> Result<String> {
    use std::os::windows::ffi::OsStrExt;
    unsafe extern "system" {
        fn GetModuleHandleA(name:*const u8)->*mut std::ffi::c_void;
        fn GetProcAddress(module:*mut std::ffi::c_void,name:*const u8)->*mut std::ffi::c_void;
        fn GetProcessHeap()->*mut std::ffi::c_void;
        fn HeapFree(heap:*mut std::ffi::c_void,flags:u32,ptr:*mut std::ffi::c_void)->i32;
    }
    unsafe {
        let address=GetProcAddress(GetModuleHandleA(c"kernel32.dll".as_ptr().cast()),c"wine_get_unix_file_name".as_ptr().cast());
        if address.is_null(){return Err(invalid("Wine path conversion unavailable"));}
        let convert:unsafe extern "system" fn(*const u16)->*mut std::ffi::c_char=std::mem::transmute(address);
        let wide:Vec<u16>=path.as_os_str().encode_wide().chain(Some(0)).collect();
        let value=convert(wide.as_ptr());
        if value.is_null(){return Err(invalid("Cannot translate Wine path"));}
        let text=std::ffi::CStr::from_ptr(value).to_string_lossy().into_owned();
        HeapFree(GetProcessHeap(),0,value.cast());Ok(text)
    }
}
/// Install the helper location for automatic startup from normal Wine shortcuts.
#[cfg(windows)]
pub fn install_helper(helper:&Path,remove:bool)->Result<()> {
    if !is_wine(){return Ok(());}
    let metadata=Path::new(r"C:\open-volar-s-wine-helper.txt");
    if remove {
        if std::fs::read_to_string(metadata).ok().is_some_and(|s|s.lines().next()==unix_path(helper).ok().as_deref()) {
            std::fs::remove_file(metadata)?;
        }
        return Ok(());
    }
    if !helper.is_file(){return Err(invalid("Linux helper is missing from installation"));}
    let text=format!("{}\n{}\n",unix_path(helper)?,unix_path(Path::new(r"C:\open-volar-s-bridge.txt"))?);
    std::fs::write(metadata,text)?;
    ensure_helper(Path::new(r"C:\open-volar-s-bridge.txt"))
}
#[cfg(windows)]
fn ensure_helper(config:&Path)->Result<()> {
    if !is_wine(){return Ok(());}
    if let Ok((address,_))=configuration(config) {
        match TcpStream::connect_timeout(&address,Duration::from_millis(250)) {
            Ok(_)=>return Ok(()),
            Err(e) if e.kind()==std::io::ErrorKind::ConnectionRefused=>{let _=std::fs::remove_file(config);},
            Err(e)=>return Err(e.into()),
        }
    }
    let metadata=Path::new(r"C:\open-volar-s-wine-helper.txt");
    if !metadata.exists(){return Ok(());} // Explicit Linux launchers already supply their own server.
    let text=std::fs::read_to_string(metadata)?;let lines:Vec<_>=text.lines().collect();
    if lines.len()!=2 || !lines.iter().all(|s|s.starts_with('/')) {return Err(invalid("Invalid installed helper paths"));}
    let status=std::process::Command::new(r"C:\windows\system32\start.exe").args(["/b","/unix","/usr/bin/env","/lib64/ld-linux-x86-64.so.2",lines[0],"wine-bridge",lines[1]])
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status()?;
    if !status.success(){return Err(invalid("Could not start Linux helper; install the native Linux package"));}
    for _ in 0..100 {
        if let Ok((address,_))=configuration(config) {if TcpStream::connect_timeout(&address,Duration::from_millis(100)).is_ok(){return Ok(());}}
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(invalid("Linux helper startup failed; check Linux runtime dependencies and device permissions"))
}
