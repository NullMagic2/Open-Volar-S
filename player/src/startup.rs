//! Optional per-user sign-in preparation. No resident process, tuning, or UI.
use std::{ffi::c_void, fs, path::Path};
use serde_json::json;
use windows::{core::w, Win32::UI::WindowsAndMessaging::FindWindowW};

// Separate from Picture controls (340..348) and Parental controls (350..360).
pub const ENABLE:u16=361;
pub const STATUS:u16=362;
const RUN_KEY:&str=r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE:&str="OpenVolarSReceiver";
type HKey=*mut c_void;
const HKCU:HKey=0x80000001u32 as i32 as isize as HKey;
#[link(name="advapi32")]
extern "system" {
    fn RegGetValueW(key:HKey,subkey:*const u16,value:*const u16,flags:u32,kind:*mut u32,data:*mut c_void,size:*mut u32)->i32;
    fn RegCreateKeyExW(key:HKey,subkey:*const u16,reserved:u32,class:*mut u16,options:u32,access:u32,security:*const c_void,result:*mut HKey,disposition:*mut u32)->i32;
    fn RegOpenKeyExW(key:HKey,subkey:*const u16,options:u32,access:u32,result:*mut HKey)->i32;
    fn RegSetValueExW(key:HKey,name:*const u16,reserved:u32,kind:u32,data:*const u8,size:u32)->i32;
    fn RegDeleteValueW(key:HKey,name:*const u16)->i32;
    fn RegCloseKey(key:HKey)->i32;
}
fn wide(s:&str)->Vec<u16>{s.encode_utf16().chain(Some(0)).collect()}
fn registry_error(code:i32)->String {std::io::Error::from_raw_os_error(code).to_string()}
fn read_at(key:&str)->Result<Option<String>,String>{
    let key=wide(key);let name=wide(VALUE);let mut size=0;
    unsafe {
        let result=RegGetValueW(HKCU,key.as_ptr(),name.as_ptr(),2,std::ptr::null_mut(),std::ptr::null_mut(),&mut size);
        if result==2{return Ok(None);}
        if result!=0{return Err(registry_error(result));}
        if size>32768{return Err("Startup entry is too long.".into());}
        let mut data=vec![0u16;(size as usize+1)/2];
        let result=RegGetValueW(HKCU,key.as_ptr(),name.as_ptr(),2,std::ptr::null_mut(),data.as_mut_ptr().cast(),&mut size);
        if result!=0{return Err(registry_error(result));}
        let end=data.iter().position(|&c|c==0).unwrap_or(data.len());
        Ok(Some(String::from_utf16_lossy(&data[..end])))
    }
}
fn write_at(key:&str,command:Option<&str>)->Result<(),String>{
    let key=wide(key);let name=wide(VALUE);let mut handle=std::ptr::null_mut();
    unsafe {
        let result=if command.is_some(){RegCreateKeyExW(HKCU,key.as_ptr(),0,std::ptr::null_mut(),0,2,std::ptr::null(),&mut handle,std::ptr::null_mut())}
            else {RegOpenKeyExW(HKCU,key.as_ptr(),0,2,&mut handle)};
        if result==2 && command.is_none(){return Ok(());}
        if result!=0{return Err(registry_error(result));}
        let result=if let Some(command)=command {
            let data=wide(command);RegSetValueExW(handle,name.as_ptr(),0,1,data.as_ptr().cast(),(data.len()*2) as u32)
        }else{RegDeleteValueW(handle,name.as_ptr())};
        RegCloseKey(handle);
        if result==0 || (result==2 && command.is_none()){Ok(())}else{Err(registry_error(result))}
    }
}
fn command(exe:&Path)->Result<String,String>{
    let exe=exe.to_str().ok_or("Startup path is not valid Unicode.")?;
    if exe.contains(['"','\0']){return Err("Invalid startup path.".into());}
    let command=format!("\"{exe}\" --receiver-startup");
    // Windows documents a 260-character limit for Run commands.
    if command.encode_utf16().count()>260{return Err("Startup path is too long.".into());}
    Ok(command)
}
pub fn enabled(preview:bool,saved:bool)->bool {
    if preview{return saved;}
    read_at(RUN_KEY).ok().flatten().is_some()
}
pub fn configure(on:bool,preview:bool,profile:&Path)->Result<(),String>{
    let cmd=if on {Some(command(&std::env::current_exe().map_err(|e|e.to_string())?)?)}else{None};
    if preview {
        // Preview tests must never register a real Windows startup command.
        fs::create_dir_all(profile).map_err(|e|e.to_string())?;
        return fs::write(profile.join("startup-preview.json"),json!({"enabled":on,"command":cmd}).to_string()).map_err(|e|e.to_string());
    }
    write_at(RUN_KEY,cmd.as_deref())
}
/// Returns before the GUI, COM, renderer, audio graph or message loop is created.
pub fn background(profile:&Path,preview:bool)->i32 {
    let start=std::time::Instant::now();
    let mut report=json!({"version":env!("CARGO_PKG_VERSION"),"mode":"receiver-startup","ui_created":false,"tuned":false});
    let result:Result<(),String>=(||{
        if preview {report["status"]=json!("preview-skipped-hardware");return Ok(());}
        // An open player may be between channel operations. Avoid even probing it.
        if unsafe {FindWindowW(w!("A865RNativeClassic"),None)}.is_ok(){
            report["status"]=json!("player-already-open");return Ok(());
        }
        let mut device=a865r::Device::new();
        // WinUSB uses exclusive ownership; another client always wins over startup.
        let info=match device.connect_and_probe(){
            Ok(info)=>info,
            Err(e)=>{report["status"]=json!("receiver-unavailable");report["detail"]=json!(e.to_string());return Ok(());}
        };
        report["firmware_before"]=json!(info.firmware_version);
        if !info.firmware_running {
            let firmware=a865r::execution_probe_image().map_err(|e|e.to_string())?;
            device.load_firmware(&firmware).map_err(|e|e.to_string())?;
            report["firmware_loaded_into_ram"]=json!(true);
        }else{report["firmware_loaded_into_ram"]=json!(false);}
        let mut receiver=device.receiver().map_err(|e|e.to_string())?;
        let initialized=receiver.initialize();
        let stopped=receiver.stop();
        initialized.and(stopped).map_err(|e|e.to_string())?;
        report["status"]=json!("ready");
        // Receiver and Device drop here, releasing exclusive USB ownership.
        Ok(())
    })();
    if let Err(e)=&result{report["status"]=json!("failed");report["detail"]=json!(e);}
    report["elapsed_ms"]=json!(start.elapsed().as_millis());
    let _=fs::create_dir_all(profile);
    let _=fs::write(profile.join("receiver-startup.json"),serde_json::to_vec_pretty(&report).unwrap());
    if result.is_ok(){0}else{1}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn startup_command_quotes_unicode_paths_and_rejects_oversize(){
        assert_eq!(command(Path::new(r"C:\Apps\TV Ελληνικά\live-tv.exe")).unwrap(),"\"C:\\Apps\\TV Ελληνικά\\live-tv.exe\" --receiver-startup");
        assert!(command(Path::new(&format!("C:\\{}\\live-tv.exe","x".repeat(260)))).is_err());
        assert!(command(Path::new("C:\\bad\"path\\live-tv.exe")).is_err());
    }
    #[test] fn registration_roundtrip_in_isolated_key(){
        let key=format!(r"Software\A865R\Tests\ReceiverStartup-{}",std::process::id());
        assert_eq!(read_at(&key).unwrap(),None);
        let c=command(Path::new(r"C:\Test folder\live-tv.exe")).unwrap();
        write_at(&key,Some(&c)).unwrap();assert_eq!(read_at(&key).unwrap().as_deref(),Some(c.as_str()));
        write_at(&key,None).unwrap();write_at(&key,None).unwrap();assert_eq!(read_at(&key).unwrap(),None);
        // Only this disposable test key is removed; the Windows Run key is untouched.
        #[link(name="advapi32")] extern "system" {fn RegDeleteKeyW(key:HKey,subkey:*const u16)->i32;}
        unsafe {RegDeleteKeyW(HKCU,wide(&key).as_ptr());}
    }
    #[test] fn general_labels_fit_all_languages_at_common_dpi(){
        use windows::Win32::Graphics::Gdi::*;
        use windows::Win32::Foundation::RECT;
        unsafe {
            let dc=CreateCompatibleDC(None);assert!(!dc.0.is_null());
            for dpi in [96,120,144,192] {
                let size=15*dpi/96;
                let font=CreateFontW(-size,0,0,0,400,0,0,0,DEFAULT_CHARSET.0 as u32,OUT_DEFAULT_PRECIS.0 as u32,CLIP_DEFAULT_PRECIS.0 as u32,CLEARTYPE_QUALITY.0 as u32,DEFAULT_PITCH.0 as u32,w!("Segoe UI"));
                let old=SelectObject(dc,font);
                for (label,width) in [("Start receiver with Windows",480),("Prepare the receiver without opening Live TV.",670)] {
                    for language in 0..4 {
                        let mut text:Vec<u16>=crate::i18n::translate(label,language).encode_utf16().collect();
                        let mut rect=RECT::default();
                        assert!(DrawTextW(dc,&mut text,&mut rect,DT_CALCRECT|DT_SINGLELINE|DT_NOPREFIX)>0);
                        assert!(rect.right<=width*dpi/96,"{label}: language {language}, DPI {dpi}, width {}",rect.right);
                        assert!(rect.bottom<=32*dpi/96);
                    }
                }
                SelectObject(dc,old);let _=DeleteObject(font);
            }
            let _=DeleteDC(dc);
        }
    }
    #[test] fn preview_background_never_touches_hardware(){
        let dir=std::env::temp_dir().join(format!("a865r-startup-test-{}",std::process::id()));
        assert_eq!(background(&dir,true),0);
        let v:serde_json::Value=serde_json::from_slice(&fs::read(dir.join("receiver-startup.json")).unwrap()).unwrap();
        assert_eq!(v["status"],"preview-skipped-hardware");assert_eq!(v["ui_created"],false);
        fs::remove_file(dir.join("receiver-startup.json")).unwrap();fs::remove_dir(dir).unwrap();
    }
}
