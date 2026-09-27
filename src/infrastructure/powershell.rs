//! Which PowerShell runs `.ps1` scripts (LCH-001).

use std::env;
use std::ffi::OsStr;

use crate::domain::launch_plan::PowerShellHost;

/// `Pwsh` when `pwsh.exe` is in one of the folders of `path_variable` (the value of PATH);
/// Windows PowerShell otherwise.
pub fn detect_powershell(path_variable: Option<&OsStr>) -> PowerShellHost {
    let pwsh = PowerShellHost::Pwsh.program();
    let found = path_variable
        .map(|value| env::split_paths(value).any(|folder| folder.join(pwsh).is_file()))
        .unwrap_or(false);
    if found {
        PowerShellHost::Pwsh
    } else {
        PowerShellHost::WindowsPowerShell
    }
}
