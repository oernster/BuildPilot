//! The log file (NFR-OBS-001): `buildpilot.log` in the data folder, one timestamped line per
//! entry. At `ROTATE_AT_BYTES` it becomes `buildpilot.previous.log`, replacing any earlier one,
//! and a fresh file starts. Panics on any thread are recorded here too (NFR-REL-001).
//!
//! Logging never stops BuildPilot: a log that cannot be opened or written is simply not kept.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::panic::{self, PanicHookInfo};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, TryLockError};
use std::thread;

use crate::application::ports::Log;

use super::win32::diagnostics::{local_timestamp, redirect_stderr};

/// The log's name in the data folder.
pub const LOG_FILE_NAME: &str = "buildpilot.log";
/// What the log is renamed to when it rotates.
pub const PREVIOUS_LOG_FILE_NAME: &str = "buildpilot.previous.log";
/// The size at which the log rotates: 1 MB.
pub const ROTATE_AT_BYTES: u64 = 1024 * 1024;
/// How each entry ends, so the file reads properly in Notepad.
const LINE_END: &str = "\r\n";

/// The log, shared by every thread that writes to it.
#[derive(Clone)]
pub struct LogFile {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    path: PathBuf,
    previous: PathBuf,
    rotate_at: u64,
    holds_stderr: bool,
    file: Option<File>,
    size: u64,
}

impl LogFile {
    /// The log in `folder`, rotating at `rotate_at` bytes.
    pub fn open(folder: &Path, rotate_at: u64) -> Self {
        Self::start(folder, rotate_at, false)
    }

    /// The process's own log in `folder`. Standard error is pointed at it; again at each
    /// fresh file after a rotation, so nothing the Rust runtime writes there is lost.
    pub fn open_for_process(folder: &Path) -> Self {
        Self::start(folder, ROTATE_AT_BYTES, true)
    }

    fn start(folder: &Path, rotate_at: u64, holds_stderr: bool) -> Self {
        let mut inner = Inner {
            path: folder.join(LOG_FILE_NAME),
            previous: folder.join(PREVIOUS_LOG_FILE_NAME),
            rotate_at,
            holds_stderr,
            file: None,
            size: 0,
        };
        // A folder that cannot be made leaves the log closed rather than stopping BuildPilot.
        let _ = fs::create_dir_all(folder);
        inner.reopen();
        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    /// Where the log is written.
    pub fn path(&self) -> PathBuf {
        self.lock().path.clone()
    }

    /// Writes `line` with the time.
    pub fn write(&self, line: &str) {
        self.lock().write(line);
    }

    /// Records every panic, on any thread, before it unwinds (NFR-REL-001).
    pub fn record_panics(&self) {
        let log = self.clone();
        panic::set_hook(Box::new(move |info| {
            let line = panic_line(info);
            // The panic may have happened while this thread held the log; waiting for it would
            // never end, so standard error takes the line instead.
            if !log.try_write(&line) {
                eprintln!("{line}");
            }
        }));
    }

    fn try_write(&self, line: &str) -> bool {
        match self.inner.try_lock() {
            Ok(mut inner) => inner.write(line),
            Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner().write(line),
            Err(TryLockError::WouldBlock) => return false,
        }
        true
    }

    /// The log; a panic while another thread held it leaves nothing half done that matters.
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Log for LogFile {
    fn record(&self, line: &str) {
        self.write(line);
    }
}

impl Inner {
    fn reopen(&mut self) {
        let opened = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path);
        self.file = opened.ok().inspect(|file| {
            if self.holds_stderr {
                redirect_stderr(file);
            }
        });
        self.size = self
            .file
            .as_ref()
            .and_then(|file| file.metadata().ok())
            .map_or(0, |metadata| metadata.len());
    }

    fn write(&mut self, line: &str) {
        let entry = format!("{} {line}{LINE_END}", local_timestamp());
        let length = entry.len() as u64;
        if self.size > 0 && self.size + length > self.rotate_at {
            self.rotate();
        }
        if let Some(file) = self.file.as_mut()
            && file.write_all(entry.as_bytes()).is_ok()
        {
            self.size += length;
        }
    }

    /// Renames the log under its open handle (Rust opens files so they may be renamed while
    /// open), opens a fresh one and only then lets the old handle go, so standard error never
    /// points at a closed file.
    fn rotate(&mut self) {
        let _ = fs::rename(&self.path, &self.previous);
        let old = self.file.take();
        self.reopen();
        drop(old);
    }
}

/// A panic in one line: the thread, where and why.
fn panic_line(info: &PanicHookInfo<'_>) -> String {
    let thread = thread::current();
    let name = thread.name().unwrap_or("unnamed");
    let payload = info.payload();
    let message = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("no message");
    let location = info
        .location()
        .map_or_else(|| "an unknown place".to_owned(), ToString::to_string);
    format!("Internal error on thread '{name}' at {location}: {message}")
}
