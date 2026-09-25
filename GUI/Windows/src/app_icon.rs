//! Explicit native window icons, independent of the shell's executable thumbnail cache.
use windows::{core::{PCWSTR, Result}, Win32::{Foundation::*, System::LibraryLoader::GetModuleHandleW,
    UI::{HiDpi::*, WindowsAndMessaging::*}}};

pub unsafe fn load(dpi: u32) -> Result<(HICON, HICON)> {
    let instance = GetModuleHandleW(None)?;
    let load = |x, y| -> Result<HICON> {
        Ok(HICON(LoadImageW(instance, PCWSTR(201usize as *const u16), IMAGE_ICON,
            GetSystemMetricsForDpi(x, dpi), GetSystemMetricsForDpi(y, dpi), LR_SHARED)?.0))
    };
    // LR_SHARED retains these resource handles for the process lifetime.
    Ok((load(SM_CXICON, SM_CYICON)?, load(SM_CXSMICON, SM_CYSMICON)?))
}
pub unsafe fn apply(hwnd: HWND) -> Result<()> {
    let (large, small) = load(GetDpiForWindow(hwnd))?;
    SendMessageW(hwnd, WM_SETICON, WPARAM(ICON_BIG as usize), LPARAM(large.0 as isize));
    SendMessageW(hwnd, WM_SETICON, WPARAM(ICON_SMALL as usize), LPARAM(small.0 as isize));
    Ok(())
}
