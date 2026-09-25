//! Userspace DirectShow/BDA compatibility adapter. The USB binding remains WinUSB.
#![allow(non_snake_case)]
mod backend;
pub mod signal;
pub mod clock_recovery;
mod controls;
mod filter;
mod pin_control;
mod registration;
pub use backend::Replay;
pub use filter::{
    create_clocked_source, create_filter, create_observed_source, create_replay_filter, create_replay_source,
    create_video_filter, StreamObserver, VideoTransform, CAPTURE_CLSID, TUNER_CLSID,
};
use std::{
    ffi::c_void,
    sync::atomic::{AtomicUsize, Ordering},
};
use windows::{
    core::*,
    Win32::{Foundation::*, System::Com::*},
};

pub(crate) static OBJECTS: AtomicUsize = AtomicUsize::new(0);
pub(crate) fn trace(message: impl AsRef<str>) {
    use std::io::Write;
    let Some(root) = std::env::var_os("LOCALAPPDATA") else {
        return;
    };
    let dir = std::env::var_os("A865R_COMPAT_LOG")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(root).join("A865R/Compatibility"));
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("adapter.log"))
    {
        let _ = writeln!(
            file,
            "{:?} pid={} {}",
            std::time::SystemTime::now(),
            std::process::id(),
            message.as_ref()
        );
    }
}
pub(crate) fn unsupported() -> Error {
    Error::from_hresult(E_NOTIMPL)
}
pub(crate) fn invalid() -> Error {
    Error::from_hresult(E_INVALIDARG)
}
pub(crate) fn put<T: Copy>(p: *mut T, value: T) -> Result<()> {
    if p.is_null() {
        return Err(Error::from_hresult(E_POINTER));
    }
    unsafe {
        p.write(value);
    }
    Ok(())
}

#[implement(IClassFactory)]
struct Factory {
    capture: bool,
}
impl Drop for Factory {
    fn drop(&mut self) {
        OBJECTS.fetch_sub(1, Ordering::SeqCst);
    }
}
impl IClassFactory_Impl for Factory_Impl {
    fn CreateInstance(
        &self,
        outer: Option<&IUnknown>,
        iid: *const GUID,
        out: *mut *mut c_void,
    ) -> Result<()> {
        if out.is_null() || iid.is_null() {
            return Err(Error::from_hresult(E_POINTER));
        }
        unsafe {
            *out = std::ptr::null_mut();
        }
        if outer.is_some() {
            return Err(Error::from_hresult(CLASS_E_NOAGGREGATION));
        }
        let filter = filter::create_external_filter(self.capture);
        unsafe { filter.query(iid, out).ok() }
    }
    fn LockServer(&self, lock: BOOL) -> Result<()> {
        if lock.as_bool() {
            OBJECTS.fetch_add(1, Ordering::SeqCst);
        } else {
            OBJECTS.fetch_sub(1, Ordering::SeqCst);
        }
        Ok(())
    }
}
#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(
    clsid: *const GUID,
    iid: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    if clsid.is_null() || iid.is_null() || out.is_null() {
        return E_POINTER;
    }
    *out = std::ptr::null_mut();
    if *clsid != TUNER_CLSID && *clsid != CAPTURE_CLSID {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    trace(format!("DllGetClassObject {:?}", *clsid));
    OBJECTS.fetch_add(1, Ordering::SeqCst);
    let factory: IClassFactory = Factory {
        capture: *clsid == CAPTURE_CLSID,
    }
    .into();
    factory.query(iid, out)
}
#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    if OBJECTS.load(Ordering::SeqCst) == 0 {
        S_OK
    } else {
        S_FALSE
    }
}
#[no_mangle]
pub extern "system" fn DllRegisterServer() -> HRESULT {
    registration::register(false).into()
}
#[no_mangle]
pub extern "system" fn DllUnregisterServer() -> HRESULT {
    registration::unregister(false).into()
}
#[no_mangle]
pub extern "system" fn DllInstall(install: BOOL, command: PCWSTR) -> HRESULT {
    let user = unsafe { command.to_string().unwrap_or_default() }.eq_ignore_ascii_case("user");
    if install.as_bool() {
        registration::register(user).into()
    } else {
        registration::unregister(user).into()
    }
}
