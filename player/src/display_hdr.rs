//! Read-only display capabilities. This module never changes Windows HDR.
use windows::Win32::{Devices::Display::*,Foundation::*,Graphics::Gdi::*};
use std::mem::size_of;
use windows::Win32::System::SystemInformation::OSVERSIONINFOW;
#[link(name="ntdll")] extern "system" {fn RtlGetVersion(info:*mut OSVERSIONINFOW)->i32;}

// Windows SDK wingdi.h (26100): HDR-specific queries avoid confusing WCG with HDR.
#[repr(C)] #[derive(Default)] struct Info2 {header:DISPLAYCONFIG_DEVICE_INFO_HEADER,value:u32,encoding:u32,bits:u32,mode:u32}
#[repr(C)] struct White {header:DISPLAYCONFIG_DEVICE_INFO_HEADER,value:u32}
fn white_scale(mut header:DISPLAYCONFIG_DEVICE_INFO_HEADER)->f32 {unsafe{header.r#type=DISPLAYCONFIG_DEVICE_INFO_TYPE(11);header.size=size_of::<White>() as u32;let mut info=White{header,value:1000};if DisplayConfigGetDeviceInfo(&mut info.header)==0 {(info.value as f32/1000.).clamp(1.,12.5)}else{1.}}}
pub struct State {pub enabled:bool,pub supported:bool,pub limited:bool,pub white_scale:f32}
pub unsafe fn query(window:HWND)->Result<State,String> {
    let mut monitor=MONITORINFOEXW::default();monitor.monitorInfo.cbSize=size_of::<MONITORINFOEXW>() as u32;
    if !GetMonitorInfoW(MonitorFromWindow(window,MONITOR_DEFAULTTONEAREST),&mut monitor.monitorInfo).as_bool(){return Err("Cannot identify the player's display.".into());}
    for _ in 0..3 {
        let(mut n,mut m)=(0,0);let result=GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS,&mut n,&mut m);
        if result!=ERROR_SUCCESS{return Err(format!("Display query failed ({})",result.0));}
        let mut paths=vec![DISPLAYCONFIG_PATH_INFO::default();n as usize];let mut modes=vec![DISPLAYCONFIG_MODE_INFO::default();m as usize];
        let result=QueryDisplayConfig(QDC_ONLY_ACTIVE_PATHS,&mut n,paths.as_mut_ptr(),&mut m,modes.as_mut_ptr(),None);
        if result==ERROR_INSUFFICIENT_BUFFER{continue;}
        if result!=ERROR_SUCCESS{return Err(format!("Display query failed ({})",result.0));}
        for path in &paths[..n as usize] {
            let mut source=DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
            source.header=DISPLAYCONFIG_DEVICE_INFO_HEADER{r#type:DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,size:size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,adapterId:path.sourceInfo.adapterId,id:path.sourceInfo.id};
            if DisplayConfigGetDeviceInfo(&mut source.header)!=0 || source.viewGdiDeviceName!=monitor.szDevice{continue;}
            let header=DISPLAYCONFIG_DEVICE_INFO_HEADER{r#type:DISPLAYCONFIG_DEVICE_INFO_TYPE(15),size:size_of::<Info2>() as u32,adapterId:path.targetInfo.adapterId,id:path.targetInfo.id};
            let mut info=Info2{header,..Default::default()};
            if DisplayConfigGetDeviceInfo(&mut info.header)==0{return Ok(State{enabled:info.mode==2,supported:info.value&16!=0,limited:info.value&8!=0,white_scale:white_scale(header)});}
            let mut version=OSVERSIONINFOW::default();version.dwOSVersionInfoSize=size_of::<OSVERSIONINFOW>() as u32;
            if RtlGetVersion(&mut version)!=0 || version.dwBuildNumber>=22000 {return Err("This Windows version does not expose the HDR-specific display control.".into());}
            let mut legacy=DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO::default();legacy.header=header;legacy.header.r#type=DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO;legacy.header.size=size_of::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>() as u32;
            let code=DisplayConfigGetDeviceInfo(&mut legacy.header);
            if code!=0{return Err(format!("Windows HDR query failed ({code})"));}
            let flags=legacy.Anonymous.value;
            return Ok(State{enabled:flags&2!=0,supported:flags&1!=0,limited:flags&8!=0,white_scale:white_scale(header)});
        }
        return Err("No active display path for the player.".into());
    }
    Err("Display configuration is changing. Try again.".into())
}
