//! Windows Job Objects: one per run, so a whole process tree can be ended together (STOP-001)
//! and cannot outlive BuildPilot (STOP-005).

use std::ffi::c_void;
use std::io;
use std::mem::{size_of, zeroed};
use std::os::windows::io::{AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle};
use std::ptr::null;

use windows_sys::Win32::Foundation::{INVALID_HANDLE_VALUE, WAIT_OBJECT_0};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, OpenThread, PROCESS_SYNCHRONIZE, ResumeThread, THREAD_SUSPEND_RESUME,
    WaitForSingleObject,
};

/// `ResumeThread` answers this on failure.
const RESUME_FAILED: u32 = u32::MAX;

/// A job object; closing the last handle to it kills every process in it while kill-on-close is
/// set.
pub struct Job(OwnedHandle);

impl Job {
    /// A new, empty job with kill-on-close set.
    pub fn new() -> io::Result<Self> {
        // SAFETY: both arguments may be null (default security, no name).
        let raw = unsafe { CreateJobObjectW(null(), null()) };
        if raw.is_null() {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: `raw` is a fresh handle that nothing else owns.
        let job = Self(unsafe { OwnedHandle::from_raw_handle(raw) });
        job.set_kill_on_close(true)?;
        Ok(job)
    }

    /// Sets or clears kill-on-close.
    pub fn set_kill_on_close(&self, kill: bool) -> io::Result<()> {
        // SAFETY: an all-zero JOBOBJECT_EXTENDED_LIMIT_INFORMATION is valid and means no limits.
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { zeroed() };
        if kill {
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        }
        // SAFETY: the job handle is live; the pointer and size describe `limits`.
        let ok = unsafe {
            SetInformationJobObject(
                self.0.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                (&raw const limits).cast::<c_void>(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        check(ok)
    }

    /// Puts `process` in the job; every process it starts from then on joins too.
    pub fn assign(&self, process: BorrowedHandle<'_>) -> io::Result<()> {
        // SAFETY: both handles are live for the duration of the call.
        check(unsafe { AssignProcessToJobObject(self.0.as_raw_handle(), process.as_raw_handle()) })
    }

    /// Ends every process in the job with `exit_code`.
    pub fn terminate(&self, exit_code: u32) -> io::Result<()> {
        // SAFETY: the job handle is live.
        check(unsafe { TerminateJobObject(self.0.as_raw_handle(), exit_code) })
    }
}

/// Resumes the one thread of process `pid`, which was created suspended so it could join its
/// job before running any code.
pub fn resume_main_thread(pid: u32) -> io::Result<()> {
    // SAFETY: plain call; the result is checked before use.
    let raw = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if raw == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `raw` is a fresh snapshot handle that nothing else owns.
    let snapshot = unsafe { OwnedHandle::from_raw_handle(raw) };
    // SAFETY: an all-zero THREADENTRY32 is valid once its size field is set.
    let mut entry: THREADENTRY32 = unsafe { zeroed() };
    entry.dwSize = size_of::<THREADENTRY32>() as u32;
    // SAFETY: the snapshot is live; `entry` is sized correctly.
    let mut more = unsafe { Thread32First(snapshot.as_raw_handle(), &mut entry) } != 0;
    while more {
        if entry.th32OwnerProcessID == pid {
            return resume_thread(entry.th32ThreadID);
        }
        // SAFETY: as for Thread32First.
        more = unsafe { Thread32Next(snapshot.as_raw_handle(), &mut entry) } != 0;
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("process {pid} has no thread to resume"),
    ))
}

fn resume_thread(thread_id: u32) -> io::Result<()> {
    // SAFETY: plain call; the result is checked before use.
    let raw = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, thread_id) };
    if raw.is_null() {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `raw` is a fresh thread handle that nothing else owns.
    let thread = unsafe { OwnedHandle::from_raw_handle(raw) };
    // SAFETY: the thread handle is live and has THREAD_SUSPEND_RESUME access.
    if unsafe { ResumeThread(thread.as_raw_handle()) } == RESUME_FAILED {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// A handle that can tell whether a process has exited.
pub struct ProcessWatch(OwnedHandle);

impl ProcessWatch {
    /// Watches process `pid`; `None` when it cannot be opened, including when it has gone.
    pub fn open(pid: u32) -> Option<Self> {
        // SAFETY: plain call; the result is checked before use.
        let raw = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
        if raw.is_null() {
            return None;
        }
        // SAFETY: `raw` is a fresh process handle that nothing else owns.
        Some(Self(unsafe { OwnedHandle::from_raw_handle(raw) }))
    }

    /// True once the process has exited.
    pub fn has_exited(&self) -> bool {
        // SAFETY: the handle is live and has SYNCHRONIZE access; a zero timeout never blocks.
        unsafe { WaitForSingleObject(self.0.as_raw_handle(), 0) == WAIT_OBJECT_0 }
    }
}

/// True while process `pid` exists and has not exited.
pub fn process_is_alive(pid: u32) -> bool {
    ProcessWatch::open(pid).is_some_and(|watch| !watch.has_exited())
}

fn check(ok: i32) -> io::Result<()> {
    if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
