//! Infrastructure: the application's ports implemented against the real machine. The file
//! system, processes, Explorer and the clock are reached from here and nowhere else.
//!
//! Process, shell and Win32 code is Windows-only (SRS 2.3); the rest is portable.

pub mod config_format;
pub mod config_store;
pub mod icons;
#[cfg(windows)]
pub mod launcher;
pub mod locations;
#[cfg(windows)]
pub mod log_file;
pub mod powershell;
#[cfg(windows)]
pub mod setup;
#[cfg(windows)]
pub mod shell;
pub mod system;
#[cfg(windows)]
pub mod win32;
