//! Explorer and file associations (SRS 3.11).

use std::ffi::OsStr;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

use crate::application::ports::Shell;

use super::win32::shell::shell_open;

/// Explorer's executable, resolved through the system path.
const EXPLORER: &str = "explorer.exe";

/// Opens files and folders the way Explorer does.
pub struct ExplorerShell;

impl Shell for ExplorerShell {
    fn open(&self, file: &Path) -> Result<(), String> {
        shell_open(file.as_os_str()).map_err(|error| error.to_string())
    }

    /// Starts Explorer with `/select,` so the file is shown selected. Explorer's exit code
    /// means nothing, so only a failure to start it is reported.
    fn reveal(&self, file: &Path) -> Result<(), String> {
        Command::new(EXPLORER)
            .raw_arg(format!("/select,\"{}\"", file.display()))
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn open_folder(&self, folder: &Path) -> Result<(), String> {
        shell_open(folder.as_os_str()).map_err(|error| error.to_string())
    }

    /// Windows hands an address to the operator's default browser.
    fn open_address(&self, address: &str) -> Result<(), String> {
        shell_open(OsStr::new(address)).map_err(|error| error.to_string())
    }
}
