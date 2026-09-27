//! Finding and ending processes by their executable's name (INST-004). By name and never by
//! process tree: a tree is judged from recorded parent ids, which Windows reuses, so a tree kill
//! can reach a process that merely inherited a dead parent's number.

use std::mem::{size_of, zeroed};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::process::Child;

use windows_sys::Win32::System::Threading::WaitForInputIdle;
use windows_sys::Win32::UI::WindowsAndMessaging::{ASFW_ANY, AllowSetForegroundWindow};

use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE, TerminateProcess, WaitForSingleObject,
};

/// Every process in the snapshot, not one.
const ALL_PROCESSES: u32 = 0;
/// Win32 BOOL false.
const FALSE: i32 = 0;
/// The exit code given to a process setup ends.
const ENDED_BY_SETUP: u32 = 1;

/// The ids of running processes whose executable is `image` (compared without regard to case).
pub fn running(image: &str) -> Vec<u32> {
    // SAFETY: a plain call; the handle is checked before use.
    let raw = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, ALL_PROCESSES) };
    if raw == INVALID_HANDLE_VALUE {
        return Vec::new();
    }
    // SAFETY: `raw` is a fresh handle owned here and closed when `snapshot` drops.
    let snapshot = unsafe { OwnedHandle::from_raw_handle(raw) };
    let wanted = image.to_lowercase();
    let mut ids = Vec::new();
    // SAFETY: PROCESSENTRY32W is plain data; dwSize is set as the API requires.
    let mut entry: PROCESSENTRY32W = unsafe { zeroed() };
    entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
    let handle = raw as HANDLE;
    // SAFETY: `handle` is the live snapshot; `entry` is correctly sized.
    let mut more = unsafe { Process32FirstW(handle, &mut entry) } != FALSE;
    while more {
        let length = entry
            .szExeFile
            .iter()
            .position(|&unit| unit == 0)
            .unwrap_or(0);
        let name = String::from_utf16_lossy(&entry.szExeFile[..length]).to_lowercase();
        if name == wanted {
            ids.push(entry.th32ProcessID);
        }
        // SAFETY: as above.
        more = unsafe { Process32NextW(handle, &mut entry) } != FALSE;
    }
    drop(snapshot);
    ids
}

/// Ends process `id` and waits up to `wait_ms` for it to go. True when it has gone.
pub fn end(id: u32, wait_ms: u32) -> bool {
    // SAFETY: a plain call; the handle is checked before use.
    let raw = unsafe { OpenProcess(PROCESS_TERMINATE | PROCESS_SYNCHRONIZE, FALSE, id) };
    if raw.is_null() {
        // Already gone or not ours to end; either way it cannot be waited on.
        return running_id(id).is_none();
    }
    // SAFETY: `raw` is a fresh handle owned here.
    let process = unsafe { OwnedHandle::from_raw_handle(raw) };
    // SAFETY: the handle carries PROCESS_TERMINATE and PROCESS_SYNCHRONIZE.
    unsafe {
        TerminateProcess(raw, ENDED_BY_SETUP);
        let gone = WaitForSingleObject(raw, wait_ms) == WAIT_OBJECT_0;
        drop(process);
        gone
    }
}

/// Lets whichever process asks next take the foreground. This process was started by the
/// operator, so it holds that right and may pass it on; a refusal only means a window flashes
/// in the taskbar instead of coming forward.
pub fn allow_any_foreground() {
    // SAFETY: plain call with a constant argument.
    unsafe { AllowSetForegroundWindow(ASFW_ANY) };
}

/// Waits up to `wait_ms` for `child` to finish starting and wait for input, which for a windowed
/// program means its window is up.
pub fn wait_until_ready(child: &Child, wait_ms: u32) {
    // SAFETY: the handle belongs to `child`, alive for the call.
    unsafe { WaitForInputIdle(child.as_raw_handle(), wait_ms) };
}

fn running_id(id: u32) -> Option<u32> {
    // SAFETY: a plain call; the handle is closed at once.
    let raw = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, FALSE, id) };
    if raw.is_null() {
        return None;
    }
    // SAFETY: `raw` is a fresh handle owned here.
    drop(unsafe { OwnedHandle::from_raw_handle(raw) });
    Some(id)
}
