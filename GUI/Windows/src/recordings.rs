use std::path::{Path, PathBuf};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::{Com::*, SystemInformation::GetLocalTime},
        UI::Shell::*,
    },
};
pub fn default_folder() -> PathBuf {
    unsafe {
        if let Ok(p) = SHGetKnownFolderPath(&FOLDERID_Documents, KF_FLAG_DEFAULT, None) {
            let value = p.to_string().ok();
            CoTaskMemFree(Some(p.0.cast()));
            if let Some(value) = value {
                return PathBuf::from(value).join("A865R").join("recordings");
            }
        }
    }
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Documents").join("A865R").join("recordings")
}
pub fn load(settings: &serde_json::Value) -> PathBuf {
    settings["recording_folder"]
        .as_str()
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(default_folder)
}
pub fn validate_folder(text:&str)->std::result::Result<PathBuf,String> {
    let path=PathBuf::from(text.trim());
    if !path.is_absolute() {return Err("Enter a full folder path or choose a folder.".into());}
    if path.exists() && !path.is_dir() {return Err("Choose a folder, not a file.".into());}
    Ok(path)
}
fn safe_name(name: &str) -> String {
    let s: String = name
        .chars()
        .take(64)
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.is_empty() {
        "TV".into()
    } else {
        s
    }
}
pub fn new_path(folder: &Path, channel: &str) -> PathBuf {
    let t = unsafe { GetLocalTime() };
    folder.join(format!(
        "{:04}-{:02}-{:02}_{:02}-{:02}-{:02}-{:03}_{}.ts",
        t.wYear,
        t.wMonth,
        t.wDay,
        t.wHour,
        t.wMinute,
        t.wSecond,
        t.wMilliseconds,
        safe_name(channel)
    ))
}
pub unsafe fn choose(hwnd: HWND, current: &Path, title: &str) -> Option<PathBuf> {
    let initialized = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
    let result = (|| -> Result<PathBuf> {
        let dialog: IFileOpenDialog =
            CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
        dialog.SetOptions(
            dialog.GetOptions()?
                | FOS_PICKFOLDERS
                | FOS_FORCEFILESYSTEM
                | FOS_PATHMUSTEXIST
                | FOS_NOCHANGEDIR,
        )?;
        let title:Vec<u16>=title.encode_utf16().chain(Some(0)).collect();
        dialog.SetTitle(PCWSTR(title.as_ptr()))?;
        let name: Vec<u16> = current
            .to_string_lossy()
            .encode_utf16()
            .chain(Some(0))
            .collect();
        if let Ok(item) =
            SHCreateItemFromParsingName::<_, _, IShellItem>(PCWSTR(name.as_ptr()), None)
        {
            let _ = dialog.SetFolder(&item);
        }
        dialog.Show(hwnd)?;
        let p = dialog.GetResult()?.GetDisplayName(SIGDN_FILESYSPATH)?;
        let path = p.to_string().map(PathBuf::from);
        CoTaskMemFree(Some(p.0.cast()));
        Ok(path?)
    })();
    if initialized {
        CoUninitialize();
    }
    result.ok()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths_use_documents_and_preserve_saved_absolute_locations() {
        let default = default_folder();
        assert!(default.is_absolute());
        assert!(default.ends_with("A865R/recordings"));
        assert_eq!(load(&serde_json::json!({})), default);
        assert_eq!(
            load(&serde_json::json!({"recording_folder":"relative"})),
            default
        );
        let custom = std::env::temp_dir().join("TV recordings");
        assert_eq!(
            load(&serde_json::json!({"recording_folder":custom})),
            custom
        );
        let p = new_path(&custom, "TV / Câmara: HD");
        assert_eq!(p.parent(), Some(custom.as_path()));
        assert!(!p.file_name().unwrap().to_string_lossy().contains(':'));
    }
}
