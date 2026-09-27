//! DATA-001: one BuildPilot per data folder. The first process creates a named event and waits
//! on it. A later process finds the event already there, sets it and exits; the first then brings
//! its window forward.
//!
//! The default data folder is `%APPDATA%\BuildPilot`, which is per user, so this is one instance
//! per Windows user. A `BUILDPILOT_DATA_DIR` copy is a separate instance, since it writes a
//! separate config file. The `Local\` namespace is the logon session's.

use std::io;
use std::iter::once;
use std::num::NonZeroIsize;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::ptr::null;
use std::thread;

use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError, HWND, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{
    CreateEventW, INFINITE, SetEvent, WaitForSingleObject,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    IsIconic, SW_RESTORE, SetForegroundWindow, ShowWindow,
};

use super::processes::allow_any_foreground;

/// What every BuildPilot event name starts with, including the namespace it lives in.
const NAME_PREFIX: &str = r"Local\BuildPilot.";
/// Kernel object names may not contain this after the namespace.
const FORBIDDEN_IN_NAME: char = '\\';
/// What stands in for it.
const NAME_SEPARATOR: &str = "/";
/// Win32 BOOL false: the event resets itself once a waiter has seen it.
const AUTO_RESET: i32 = 0;
/// Win32 BOOL false: the event starts unset.
const UNSET: i32 = 0;
/// Win32 BOOL false, as answered by a call that did not happen.
const FALSE: i32 = 0;

/// What starting up found.
pub enum Claim {
    /// No other BuildPilot is using this data folder; this one is the instance.
    First(Instance),
    /// Another BuildPilot is using it and has been asked to come forward.
    Running,
}

/// The claim held by the running instance, released when the process ends.
pub struct Instance {
    event: OwnedHandle,
}

/// The key naming the instance for `data_folder`: case folded, since Windows paths are not
/// case sensitive, with the separators a kernel object name cannot hold replaced.
pub fn instance_key(data_folder: &Path) -> String {
    data_folder
        .to_string_lossy()
        .to_lowercase()
        .replace(FORBIDDEN_IN_NAME, NAME_SEPARATOR)
}

/// Claims the instance named by `key`. When another process holds it, asks that process to
/// come forward and answers `Running`.
pub fn claim(key: &str) -> io::Result<Claim> {
    let name: Vec<u16> = format!("{NAME_PREFIX}{key}")
        .encode_utf16()
        .chain(once(0))
        .collect();
    // SAFETY: `name` is NUL-terminated UTF-16 that outlives the call; no security attributes.
    let raw = unsafe { CreateEventW(null(), AUTO_RESET, UNSET, name.as_ptr()) };
    if raw.is_null() {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: read straight after the call it describes, on the same thread.
    let already_there = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    // SAFETY: `raw` is a fresh, valid handle this process owns and nothing else will close.
    let event = unsafe { OwnedHandle::from_raw_handle(raw) };
    if !already_there {
        return Ok(Claim::First(Instance { event }));
    }
    // The running instance is not the foreground process, so Windows would refuse its request to
    // come forward; this one was just started by the operator and may pass that right on.
    allow_any_foreground();
    // SAFETY: `event` is a valid event handle opened with full access by CreateEventW.
    if unsafe { SetEvent(event.as_raw_handle()) } == FALSE {
        return Err(io::Error::last_os_error());
    }
    Ok(Claim::Running)
}

impl Instance {
    /// Calls `summoned` on a thread of its own each time a later process asks this one to come
    /// forward. The claim is held for the rest of the process's life.
    pub fn on_summons(self, summoned: impl Fn() + Send + 'static) {
        thread::spawn(move || {
            // SAFETY: the handle is owned by this closure, so it stays valid while it is waited on.
            while unsafe { WaitForSingleObject(self.event.as_raw_handle(), INFINITE) }
                == WAIT_OBJECT_0
            {
                // NFR-REL-001: the panic hook has logged it; the next summons is still answered.
                let _ = catch_unwind(AssertUnwindSafe(&summoned));
            }
        });
    }
}

/// Restores `window` if it is minimised and makes it the foreground window.
pub fn bring_forward(window: NonZeroIsize) {
    let hwnd = window.get() as HWND;
    // SAFETY: plain calls on a window handle; Windows answers failure for a handle that has gone.
    unsafe {
        if IsIconic(hwnd) != FALSE {
            ShowWindow(hwnd, SW_RESTORE);
        }
        SetForegroundWindow(hwnd);
    }
}
