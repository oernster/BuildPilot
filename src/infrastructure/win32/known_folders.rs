//! The folders Windows says the Desktop and the Start menu's programs are in. They can be
//! redirected (to OneDrive, say), so they are asked for rather than built from the profile path.

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::ptr::null_mut;

use windows_sys::Win32::Foundation::S_OK;
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::UI::Shell::{FOLDERID_Desktop, FOLDERID_Programs, SHGetKnownFolderPath};
use windows_sys::core::GUID;

/// No special behaviour asked of SHGetKnownFolderPath.
const DEFAULT_FLAGS: u32 = 0;

/// The operator's Desktop folder; `None` when Windows cannot say.
pub fn desktop() -> Option<PathBuf> {
    known_folder(&FOLDERID_Desktop)
}

/// The Start menu's Programs folder for the operator; `None` when Windows cannot say.
pub fn start_menu_programs() -> Option<PathBuf> {
    known_folder(&FOLDERID_Programs)
}

fn known_folder(id: &GUID) -> Option<PathBuf> {
    let mut raw: *mut u16 = null_mut();
    // SAFETY: `id` is a valid folder id; `raw` receives a string Windows allocates, freed below
    // whatever the result, as the API requires.
    let result = unsafe { SHGetKnownFolderPath(id, DEFAULT_FLAGS, null_mut(), &mut raw) };
    let path = (result == S_OK && !raw.is_null()).then(|| {
        // SAFETY: on success `raw` is a NUL-terminated UTF-16 string.
        let length = (0..).take_while(|&at| unsafe { *raw.add(at) } != 0).count();
        // SAFETY: `length` code units were just read from `raw`.
        let units = unsafe { std::slice::from_raw_parts(raw, length) };
        PathBuf::from(OsString::from_wide(units))
    });
    // SAFETY: `raw` came from SHGetKnownFolderPath; freeing null is allowed.
    unsafe { CoTaskMemFree(raw.cast()) };
    path
}
