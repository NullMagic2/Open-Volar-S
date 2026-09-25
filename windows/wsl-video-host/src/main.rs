#[cfg(not(windows))]
fn main() { eprintln!("This helper runs on the Windows host, launched by WSL."); std::process::exit(1); }

#[cfg(windows)]
mod host {
    use std::{ffi::{c_char,c_int,c_void,CString},sync::Arc};
    use gpu_video::{VulkanInstance,VulkanDevice,RawFrameData,broadcast::Decoder,parameters::*};
    struct State { decoder: Option<Decoder<RawFrameData>>, device: Option<Arc<VulkanDevice>> }
    extern "C" {
        fn ovs_audio_run(read:unsafe extern "C" fn(*mut c_void,*mut u8,c_int)->c_int,
            write:unsafe extern "C" fn(*mut c_void,*mut u8,c_int)->c_int,opaque:*mut c_void)->c_int;
        fn ovs_host_run(input:*const c_char, output:*const c_char, backend:c_int, program:c_int,
            max_frames:c_int, seek:f64, follow:c_int, opaque:*mut c_void,
            decode:unsafe extern "C" fn(*mut c_void,*mut c_void,*const u8,c_int,i64)->c_int,
            write:Option<unsafe extern "C" fn(*mut c_void,*const u8,c_int)->c_int>,write_opaque:*mut c_void)->c_int;
        fn ovs_host_frame(mux:*mut c_void,bytes:*const u8,len:c_int,width:c_int,height:c_int,
            pts:i64,duration:i64,interlaced:c_int,top_first:c_int)->c_int;
    }
    unsafe extern "C" fn audio_read(opaque:*mut c_void,bytes:*mut u8,len:c_int)->c_int {
        use std::io::Read;
        let stream=&mut *(opaque as *mut std::net::TcpStream);
        if stream.read_exact(std::slice::from_raw_parts_mut(bytes,len as usize)).is_ok(){len}else{-1}
    }
    unsafe extern "C" fn audio_write(opaque:*mut c_void,bytes:*mut u8,len:c_int)->c_int {
        write(opaque,bytes,len)
    }
    unsafe extern "C" fn write(opaque:*mut c_void,bytes:*const u8,len:c_int)->c_int {
        use std::io::Write;
        let stream=&mut *(opaque as *mut std::net::TcpStream);
        if stream.write_all(std::slice::from_raw_parts(bytes,len as usize)).is_ok(){len}else{-5}
    }
    unsafe extern "C" fn decode(opaque:*mut c_void,mux:*mut c_void,data:*const u8,len:c_int,pts:i64)->c_int {
        let state=&mut *(opaque as *mut State);
        let mut output_failed=false;
        let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||->Result<(),String>{
            if state.decoder.is_none() {
                let instance=VulkanInstance::new().map_err(|e|e.to_string())?;
                let adapter=instance.create_adapter(&VulkanAdapterDescriptor{
                    supports_decoding:true,supports_encoding:false,compatible_surface:None,
                }).map_err(|e|e.to_string())?;
                eprintln!("[DEBUG]: Vulkan Video adapter: {}",adapter.info().name);
                let device=adapter.create_device(&Default::default()).map_err(|e|e.to_string())?;
                state.decoder=Some(Decoder::new_bytes(&device).map_err(|e|e.to_string())?);
                state.device=Some(device);
            }
            let decoder=state.decoder.as_mut().unwrap();
            let frames=if data.is_null(){decoder.flush()}else{
                decoder.decode(std::slice::from_raw_parts(data,len as usize),
                    (pts!=i64::MIN).then_some(pts))
            }.map_err(|e|e.to_string())?;
            for frame in frames {
                let raw=frame.texture;
                if ovs_host_frame(mux,raw.frame.as_ptr(),raw.frame.len() as c_int,
                    raw.width as c_int,raw.height as c_int,frame.pts,frame.duration,
                    frame.interlaced as c_int,frame.top_first as c_int)<0 {
                    output_failed=true;
                    return Err("Cannot deliver decoded frame to WSL".into());
                }
            }
            Ok(())
        }));
        match result {
            Ok(Ok(()))=>0,
            failure=>{
                let why=match failure {Ok(Err(e))=>e,_=>"Vulkan decoder panic".into()};
                eprintln!("[DEBUG]: Vulkan Video failure: {why}");
                // Keep failed device resources alive until process exit. Some GPU
                // drivers block in destruction after losing a device.
                if let Some(decoder)=state.decoder.take(){std::mem::forget(decoder);}
                if let Some(device)=state.device.take(){std::mem::forget(device);}
                if output_failed{-2}else{-1}
            }
        }
    }
    pub fn run()->Result<(),String>{
        eprintln!("[DEBUG]: host process ID: {}",std::process::id());
        let mut input="pipe:0".to_string();let mut output="pipe:1".to_string();
        let mut backend=0;let mut program=0;let mut frames=0;let mut seek=0.0f64;let mut follow=0;let mut liveness=false;
        let mut connect=None;let mut token=None;let mut audio=false;
        let mut args=std::env::args().skip(1);
        while let Some(arg)=args.next(){
            let value=args.next().ok_or_else(||format!("Missing value for {arg}"))?;
            match arg.as_str(){
                "--input"=>input=value,"--output"=>output=value,
                "--backend"=>backend=match value.as_str(){"auto"|"vulkan"=>0,"d3d11va"=>1,"dxva2"=>2,_=>return Err("Unknown hardware decoder".into())},
                "--program"=>program=value.parse().map_err(|_|"Invalid service ID")?,
                "--frames"=>frames=value.parse().map_err(|_|"Invalid frame limit")?,
                "--seek"=>{seek=value.parse().map_err(|_|"Invalid seek position")?;if !seek.is_finite()||seek<0.0{return Err("Invalid seek position".into());}},
                "--follow"=>follow=if value=="yes"{1}else{0},
                "--liveness"=>liveness=value=="yes",
                "--audio"=>audio=value=="yes",
                "--connect"=>connect=Some(value),"--token"=>token=Some(value),
                _=>return Err(format!("Unknown option {arg}")),
            }
        }
        let input=CString::new(input).map_err(|e|e.to_string())?;
        let output=CString::new(output).map_err(|e|e.to_string())?;
        if liveness {std::thread::spawn(||{use std::io::Read;let mut byte=[0];let _=std::io::stdin().read(&mut byte);std::process::exit(0);});}
        let mut tcp=if let Some(address)=connect{
            use std::io::Write;
            let token=token.ok_or("Missing bridge authentication token")?;
            if token.len()!=64{return Err("Invalid bridge authentication token".into());}
            let secret=(0..32).map(|i|u8::from_str_radix(&token[i*2..i*2+2],16)).collect::<std::result::Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
            let mut stream=std::net::TcpStream::connect_timeout(&address.parse().map_err(|_|"Invalid bridge address")?,std::time::Duration::from_secs(5)).map_err(|e|e.to_string())?;
            stream.set_nodelay(true).map_err(|e|e.to_string())?;
            stream.write_all(&secret).map_err(|e|e.to_string())?;Some(stream)
        }else{None};
        let custom_output=tcp.is_some();
        let write_opaque=tcp.as_mut().map_or(std::ptr::null_mut(),|stream|(stream as *mut std::net::TcpStream).cast());
        if audio {
            let stream=tcp.as_mut().ok_or("Audio requires an authenticated TCP connection")?;
            stream.set_read_timeout(Some(std::time::Duration::from_secs(10))).map_err(|e|e.to_string())?;
            stream.set_write_timeout(Some(std::time::Duration::from_secs(3))).map_err(|e|e.to_string())?;
            return if unsafe{ovs_audio_run(audio_read,audio_write,write_opaque)}<0 {
                Err("Windows WASAPI output failed".into())
            }else{Ok(())};
        }
        let mut state=State{decoder:None,device:None};
        let result=unsafe{ovs_host_run(input.as_ptr(),output.as_ptr(),backend,program,frames,seek,follow,
            (&mut state as *mut State).cast(),decode,custom_output.then_some(write),write_opaque)};
        if result<0 {Err("Windows video bridge failed (see console diagnostics)".into())}else{Ok(())}
    }
}
#[cfg(windows)]
fn main(){if let Err(error)=host::run(){eprintln!("[ERROR]: {error}");std::process::exit(1);}}
