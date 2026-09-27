//! How BuildPilot reports trouble when there is no window to say it in (NFR-REL-001).

use std::fs::File;
use std::iter::once;
use std::os::windows::io::AsRawHandle;
use std::ptr::null_mut;

use windows_sys::Win32::Foundation::SYSTEMTIME;
use windows_sys::Win32::System::Console::{STD_ERROR_HANDLE, SetStdHandle};
use windows_sys::Win32::System::SystemInformation::GetLocalTime;
use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};

/// The variable that makes Slint count frames and draw the count over the window.
const SLINT_PERFORMANCE_VARIABLE: &str = "SLINT_DEBUG_PERFORMANCE";

/// Removes Slint's frame counter from this process, whatever environment started it. Slint reads
/// the variable each time a window's renderer is made, so this must run before any window
/// exists; it is the first line of each executable's `main`. The builds BuildPilot starts
/// inherit its environment, so they no longer see the variable either.
pub fn hide_frame_counter() {
    // SAFETY: called first in `main`, while the process has one thread, so nothing else can be
    // reading or writing the environment at the same time.
    unsafe { std::env::remove_var(SLINT_PERFORMANCE_VARIABLE) };
}

/// Points the process's standard error at `file`. A windowed program is started with none, so
/// without this whatever the Rust runtime writes there (a stack overflow, an abort) is lost.
/// `file` must stay open for as long as standard error points at it.
pub fn redirect_stderr(file: &File) {
    // SAFETY: `file`'s handle is valid; the caller keeps it open while standard error uses it.
    unsafe { SetStdHandle(STD_ERROR_HANDLE, file.as_raw_handle()) };
}

/// The local time as `yyyy-mm-dd hh:mm:ss.mmm`, for the log.
pub fn local_timestamp() -> String {
    // SAFETY: GetLocalTime fills the SYSTEMTIME it is given and cannot fail.
    let now: SYSTEMTIME = unsafe {
        let mut now = std::mem::zeroed();
        GetLocalTime(&mut now);
        now
    };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        now.wYear, now.wMonth, now.wDay, now.wHour, now.wMinute, now.wSecond, now.wMilliseconds
    )
}

/// Shows `text` in a Windows error box titled `title` and waits for the operator to close it.
pub fn show_error(title: &str, text: &str) {
    let title: Vec<u16> = title.encode_utf16().chain(once(0)).collect();
    let text: Vec<u16> = text.encode_utf16().chain(once(0)).collect();
    // SAFETY: both strings are NUL-terminated UTF-16 that outlive the call; no owner window.
    unsafe {
        MessageBoxW(
            null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        )
    };
}
