//! Opening a file or folder the way Explorer would (NAV-001, UI-004).

use std::ffi::OsStr;
use std::io;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::ptr::{null, null_mut};

use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

/// `ShellExecuteW` answers a value above this on success.
const SHELL_EXECUTE_SUCCESS_ABOVE: isize = 32;

/// Opens `target` with the "open" verb: a file in its associated application, a folder in
/// Explorer.
pub fn shell_open(target: &OsStr) -> io::Result<()> {
    let verb = wide(OsStr::new("open"));
    let target = wide(target);
    // SAFETY: both strings are NUL-terminated UTF-16 that outlive the call; the other pointer
    // arguments may be null.
    let result = unsafe {
        ShellExecuteW(
            null_mut(),
            verb.as_ptr(),
            target.as_ptr(),
            null(),
            null(),
            SW_SHOWNORMAL,
        )
    };
    if result as isize > SHELL_EXECUTE_SUCCESS_ABOVE {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

fn wide(text: &OsStr) -> Vec<u16> {
    text.encode_wide().chain(once(0)).collect()
}
