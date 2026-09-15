//! Registers only our own CLSIDs and category entries, in the selected registry scope.
use crate::filter::{CAPTURE_CLSID, TUNER_CLSID};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::{LibraryLoader::*, Registry::*},
    },
};
const TUNER_CATEGORY: &str = "{71985F48-1CA1-11D3-9CC8-00C04F7971E0}";
const CAPTURE_CATEGORY: &str = "{FD0A5AF4-B41D-11D2-9C95-00C04F7971E0}";
fn set(root: HKEY, path: &str, name: &str, value: &str) -> Result<()> {
    unsafe {
        let mut key = HKEY::default();
        RegCreateKeyExW(
            root,
            &HSTRING::from(path),
            0,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut key,
            None,
        )
        .ok()?;
        let data: Vec<u8> = value
            .encode_utf16()
            .chain(Some(0))
            .flat_map(u16::to_le_bytes)
            .collect();
        let r = RegSetValueExW(key, &HSTRING::from(name), 0, REG_SZ, Some(&data)).ok();
        let _ = RegCloseKey(key);
        r
    }
}
pub fn register(user: bool) -> Result<()> {
    let root = if user {
        HKEY_CURRENT_USER
    } else {
        HKEY_LOCAL_MACHINE
    };
    let mut module = HMODULE::default();
    unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            PCWSTR(register as *const () as *const u16),
            &mut module,
        )?;
    }
    let mut path = [0u16; 32768];
    let n = unsafe { GetModuleFileNameW(module, &mut path) };
    if n == 0 || n as usize == path.len() {
        return Err(Error::from_win32());
    }
    let path = String::from_utf16_lossy(&path[..n as usize]);
    for (clsid, cat, label) in [
        (TUNER_CLSID, TUNER_CATEGORY, "Tuner"),
        (CAPTURE_CLSID, CAPTURE_CATEGORY, "Capture"),
    ] {
        let clsid = format!("{{{clsid:?}}}");
        let base = format!("Software\\Classes\\CLSID\\{clsid}");
        set(root, &base, "", &format!("A865R Open {label}"))?;
        set(root, &format!("{base}\\InprocServer32"), "", &path)?;
        set(
            root,
            &format!("{base}\\InprocServer32"),
            "ThreadingModel",
            "Both",
        )?;
        let instance = format!("Software\\Classes\\CLSID\\{cat}\\Instance\\{clsid}");
        // Discovery uses the physical WinUSB device's BDA interfaces. A duplicate
        // software moniker has no real device identity and confuses legacy clients.
        let r = unsafe { RegDeleteTreeW(root, &HSTRING::from(instance)) };
        if r != ERROR_FILE_NOT_FOUND {
            r.ok()?;
        }
    }
    crate::trace(format!(
        "registered scope={} path={path}",
        if user { "user" } else { "system" }
    ));
    Ok(())
}
pub fn unregister(user: bool) -> Result<()> {
    let root = if user {
        HKEY_CURRENT_USER
    } else {
        HKEY_LOCAL_MACHINE
    };
    for (clsid, cat) in [
        (TUNER_CLSID, TUNER_CATEGORY),
        (CAPTURE_CLSID, CAPTURE_CATEGORY),
    ] {
        let clsid = format!("{{{clsid:?}}}");
        for path in [
            format!("Software\\Classes\\CLSID\\{clsid}"),
            format!("Software\\Classes\\CLSID\\{cat}\\Instance\\{clsid}"),
        ] {
            let r = unsafe { RegDeleteTreeW(root, &HSTRING::from(path)) };
            if r != ERROR_FILE_NOT_FOUND {
                r.ok()?;
            }
        }
    }
    Ok(())
}
