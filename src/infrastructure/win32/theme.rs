//! Whether Windows is set to the dark app theme (UI-001's Follow Windows).

use std::ffi::c_void;
use std::iter::once;
use std::mem::size_of;
use std::ptr::null_mut;

use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};

const PERSONALIZE_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
const APPS_USE_LIGHT_THEME: &str = "AppsUseLightTheme";
/// `AppsUseLightTheme` holds this when apps should be dark.
const DARK: u32 = 0;

/// True when Windows asks apps to be dark; false when it asks for light or does not say.
pub fn windows_uses_dark() -> bool {
    let key: Vec<u16> = PERSONALIZE_KEY.encode_utf16().chain(once(0)).collect();
    let value: Vec<u16> = APPS_USE_LIGHT_THEME.encode_utf16().chain(once(0)).collect();
    let mut data: u32 = 1;
    let mut size = size_of::<u32>() as u32;
    // SAFETY: both names are NUL-terminated UTF-16; `data` and `size` describe a DWORD buffer.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_DWORD,
            null_mut(),
            (&raw mut data).cast::<c_void>(),
            &mut size,
        )
    };
    status == ERROR_SUCCESS && data == DARK
}
