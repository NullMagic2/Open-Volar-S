//! DVB broker: forwards standard kernel frontend requests to the IT9175 controller.
#[cfg(target_os = "linux")]
mod linux {
    use a865r::{Device, Error};
    use std::{fs::{File, OpenOptions}, io::{self, Write}, os::fd::AsRawFd,
        sync::{Arc, atomic::{AtomicBool, Ordering}}, thread, time::Duration};

    #[repr(C)]
    #[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
    struct Request { generation:u32, frequency_hz:u32, active:u32, device_index:u32 }
    #[repr(C)]
    struct Status { generation:u32, status:u32 }
    const GET_REQUEST:u64 = (2 << 30) | (16 << 16) | ((b'V' as u64) << 8) | 16;
    const SET_STATUS:u64 = (1 << 30) | (8 << 16) | ((b'V' as u64) << 8) | 17;
    unsafe extern "C" { fn ioctl(fd:i32, request:std::os::raw::c_ulong, data:*mut std::ffi::c_void)->i32; }
    fn call<T>(file:&File, command:u64, data:&mut T)->io::Result<()> {
        // Both structs exactly match the fixed-width kernel ABI in open_volar_s_dvb.h.
        if unsafe{ioctl(file.as_raw_fd(),command as _,(data as *mut T).cast())}<0 {
            Err(io::Error::last_os_error())
        }else{Ok(())}
    }
    fn report(file:&File,generation:u32,locked:bool,present:bool) {
        let mut status=Status{generation,status:if locked{0x1f}else if present{1}else{0}};
        let _=call(file,SET_STATUS,&mut status);
    }
    fn receive(file:File,request:Request,cancel:Arc<AtomicBool>)->a865r::Result<()> {
        let mut device=Device::with_transport(a865r::transport::linux_device_transport(request.device_index));
        let info=device.connect_and_probe()?;
        if !info.firmware_running {
            device.load_firmware(&a865r::execution_probe_image()?)?;
        }
        let mut receiver=device.receiver()?;
        let result=(||{
            receiver.initialize()?;
            while !cancel.load(Ordering::Relaxed) {
                let tune=receiver.tune(request.frequency_hz/1000,6000)?;
                if cancel.load(Ordering::Relaxed){break;}
                report(&file,request.generation,tune.mpeg_locked,tune.channel_found);
                if !tune.mpeg_locked {
                    for _ in 0..10 {
                        if cancel.load(Ordering::Relaxed){break;}
                        thread::sleep(Duration::from_millis(100));
                    }
                    continue;
                }
                eprintln!("DVB tuner {} locked at {} Hz",request.device_index,request.frequency_hz);
                receiver.stream_chunks_monitored(86400,&cancel,|bytes|{
                    // A generation prefix prevents data from a previous tune reaching a new client.
                    for chunk in bytes.chunks(65532) {
                        let mut packet=Vec::with_capacity(4+chunk.len());
                        packet.extend_from_slice(&request.generation.to_ne_bytes());packet.extend_from_slice(chunk);
                        let written=(&file).write(&packet).map_err(Error::Io)?;
                        if written!=packet.len(){return Err(Error::Protocol("Short DVB broker write".into()));}
                    }
                    Ok(())
                },|measurement|match measurement {
                    Ok(signal)=>report(&file,request.generation,signal.mpeg_locked,signal.mpeg_locked),
                    Err(_)=>report(&file,request.generation,false,false),
                })?;
            }
            Ok(())
        })();
        report(&file,request.generation,false,false);
        let stopped=receiver.stop();
        result.and(stopped)
    }
    pub fn run()->Result<(),Box<dyn std::error::Error>> {
        let path=std::env::args().nth(1).ok_or("Usage: open-volar-s-dvb-bridge /dev/open-volar-dvbN")?;
        let file=OpenOptions::new().read(true).write(true).open(path)?;
        let mut current=None;
        let mut worker:Option<thread::JoinHandle<a865r::Result<()>>>=None;
        let mut cancel=Arc::new(AtomicBool::new(false));
        let result=loop {
            let mut request=Request::default();
            if let Err(error)=call(&file,GET_REQUEST,&mut request){break Err(error);}
            if current!=Some(request) || worker.as_ref().is_some_and(|w|w.is_finished()) {
                let completed=worker.as_ref().is_some_and(|w|w.is_finished());
                cancel.store(true,Ordering::Relaxed);
                if let Some(worker)=worker.take(){
                    match worker.join(){
                        Ok(Err(error))=>{
                            if completed{eprintln!("DVB receiver: {error}");thread::sleep(Duration::from_secs(1));}
                        },
                        Err(_)=>eprintln!("DVB receiver thread panicked"),
                        _=>{},
                    }
                }
                // Tuning/USB operations may have blocked; use the latest request after joining.
                if let Err(error)=call(&file,GET_REQUEST,&mut request){break Err(error);}
                current=Some(request);
                cancel=Arc::new(AtomicBool::new(false));
                if request.active!=0 {
                    let output=file.try_clone()?;let stop=cancel.clone();
                    worker=Some(thread::spawn(move||receive(output,request,stop)));
                }
            }
            thread::sleep(Duration::from_millis(100));
        };
        cancel.store(true,Ordering::Relaxed);
        if let Some(worker)=worker{let _=worker.join();}
        match result {
            Err(error) if matches!(error.raw_os_error(),Some(19|6))=>Ok(()), // unplugged
            other=>other.map_err(Into::into),
        }
    }
    #[cfg(test)] mod tests {
        use super::*;
        #[test] fn kernel_abi_layout() {
            assert_eq!(std::mem::size_of::<Request>(),16);
            assert_eq!(std::mem::size_of::<Status>(),8);
            assert_eq!(GET_REQUEST,0x80105610);
            assert_eq!(SET_STATUS,0x40085611);
        }
    }
}
#[cfg(target_os = "linux")]
fn main(){if let Err(error)=linux::run(){eprintln!("DVB bridge: {error}");std::process::exit(1);}}
#[cfg(not(target_os = "linux"))]
fn main(){eprintln!("The DVB broker runs on Linux only.");}
