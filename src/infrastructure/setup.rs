//! The setup program's work on the real machine (INST-001 to INST-005). Everything is per user:
//! files under `%LOCALAPPDATA%\Programs\BuildPilot`, the Apps list entry under
//! `HKEY_CURRENT_USER`, so Windows never asks for administrator rights.

use std::env;
use std::fs;
use std::io;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

use crate::domain::version::Version;
use crate::setup::plan::Step;
use crate::setup::route::Installed;

use super::build_info::{NOTICES, NOTICES_FILE};
use super::locations::{
    APP_EXE, AUTHOR, PRODUCT_NAME, SETUP_EXE, SHORTCUT, install_folder, installed_data_folder,
};
use super::win32::{known_folders, processes, registry};

/// The Apps list entry.
pub const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\BuildPilot";
/// The argument Uninstall in the Apps list passes to setup.
pub const UNINSTALL_ARGUMENT: &str = "--uninstall";
/// How long to wait for a closed BuildPilot to go, in milliseconds.
const CLOSE_WAIT_MS: u32 = 5000;
/// Bytes in the kilobyte the Apps list measures size in.
const KILOBYTE: u64 = 1024;
/// Registry true and false, as the Apps list reads NoModify and NoRepair.
const REGISTRY_FALSE: u32 = 0;
/// Makes a shortcut through Windows PowerShell; the paths arrive in environment variables so
/// nothing needs quoting.
const SHORTCUT_SCRIPT: &str = "$s = (New-Object -ComObject WScript.Shell).CreateShortcut($env:BP_LINK); \
     $s.TargetPath = $env:BP_TARGET; $s.WorkingDirectory = $env:BP_FOLDER; \
     $s.IconLocation = $env:BP_TARGET + ',0'; $s.Save()";

/// What is installed, read once from the Apps list entry.
pub fn installed() -> Installed {
    match registry::read_string(UNINSTALL_KEY, "DisplayName") {
        None => Installed::Nothing,
        Some(_) => Installed::Recorded(
            registry::read_string(UNINSTALL_KEY, "DisplayVersion")
                .as_deref()
                .and_then(Version::parse),
        ),
    }
}

/// The desktop shortcut's path; `None` when Windows cannot say where the desktop is.
pub fn desktop_shortcut() -> Option<PathBuf> {
    known_folders::desktop().map(|folder| folder.join(SHORTCUT))
}

fn start_menu_shortcut() -> Option<PathBuf> {
    known_folders::start_menu_programs().map(|folder| folder.join(SHORTCUT))
}

/// True when BuildPilot is running (INST-004).
pub fn app_running() -> bool {
    !processes::running(APP_EXE).is_empty()
}

/// Ends every running BuildPilot; true when none is left.
pub fn close_app() -> bool {
    processes::running(APP_EXE)
        .into_iter()
        .all(|id| processes::end(id, CLOSE_WAIT_MS))
}

/// Does `step`, installing `version` from `payload`; the error is the operating system's words.
pub fn perform(step: Step, payload: &[u8], version: Version) -> io::Result<()> {
    let folder = install_folder();
    match step {
        Step::CopyFiles => copy_files(&folder, payload),
        Step::StartMenuShortcut => make_shortcut(start_menu_shortcut(), &folder),
        Step::AddDesktopShortcut => make_shortcut(desktop_shortcut(), &folder),
        Step::RemoveDesktopShortcut => remove_file(desktop_shortcut()),
        Step::Register => register(&folder, payload, version),
        Step::RemoveShortcuts => {
            remove_file(start_menu_shortcut())?;
            remove_file(desktop_shortcut())
        }
        Step::Unregister => registry::delete_tree(UNINSTALL_KEY),
        Step::RemoveFiles => remove_folder(&folder),
        Step::RemoveData => remove_folder(&installed_data_folder()),
    }
}

/// Writes the application beside a copy of setup. The application goes in under a temporary
/// name first, so a failed write never leaves half an executable where the old one was.
fn copy_files(folder: &Path, payload: &[u8]) -> io::Result<()> {
    fs::create_dir_all(folder)?;
    let app = folder.join(APP_EXE);
    let fresh = folder.join(format!("{APP_EXE}.new"));
    fs::write(&fresh, payload)?;
    fs::rename(&fresh, &app)?;
    // The licence text of every crate built in, offered from the program's Licence (UI-008).
    fs::write(folder.join(NOTICES_FILE), NOTICES)?;
    let setup = folder.join(SETUP_EXE);
    let this = env::current_exe()?;
    if !same_file(&this, &setup) {
        fs::copy(&this, &setup)?;
    }
    Ok(())
}

/// Deletes `copy`, the copy of setup an earlier uninstall ran from in the temporary folder, unless
/// `this` process is that copy. True when a copy was deleted; one already gone is not an error,
/// and one still running cannot be deleted, so it answers the error and is left for next time.
pub fn remove_stale_copy(this: &Path, copy: &Path) -> io::Result<bool> {
    if same_file(this, copy) {
        return Ok(false);
    }
    match fs::remove_file(copy) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn make_shortcut(link: Option<PathBuf>, folder: &Path) -> io::Result<()> {
    let link = link.ok_or_else(|| io::Error::other("Windows did not say where the folder is"))?;
    let status = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", SHORTCUT_SCRIPT])
        .env("BP_LINK", &link)
        .env("BP_TARGET", folder.join(APP_EXE))
        .env("BP_FOLDER", folder)
        .creation_flags(CREATE_NO_WINDOW)
        .status()?;
    if status.success() && link.exists() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "the shortcut {} was not written",
            link.display()
        )))
    }
}

fn register(folder: &Path, payload: &[u8], version: Version) -> io::Result<()> {
    let setup = folder.join(SETUP_EXE).display().to_string();
    let app = folder.join(APP_EXE).display().to_string();
    let strings = [
        ("DisplayName", PRODUCT_NAME.to_owned()),
        ("DisplayVersion", version.to_string()),
        ("Publisher", AUTHOR.to_owned()),
        ("DisplayIcon", format!("{app},0")),
        ("InstallLocation", folder.display().to_string()),
        (
            "UninstallString",
            format!("\"{setup}\" {UNINSTALL_ARGUMENT}"),
        ),
        ("ModifyPath", format!("\"{setup}\"")),
    ];
    for (name, text) in &strings {
        registry::write_string(UNINSTALL_KEY, name, text)?;
    }
    registry::write_number(UNINSTALL_KEY, "NoModify", REGISTRY_FALSE)?;
    registry::write_number(UNINSTALL_KEY, "NoRepair", REGISTRY_FALSE)?;
    let size = u32::try_from(payload.len() as u64 / KILOBYTE).unwrap_or(u32::MAX);
    registry::write_number(UNINSTALL_KEY, "EstimatedSize", size)
}

/// Deletes `path`; one already gone is not an error.
fn remove_file(path: Option<PathBuf>) -> io::Result<()> {
    match path.map(fs::remove_file) {
        Some(Err(error)) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

/// Deletes `folder` and everything in it; one already gone is not an error.
fn remove_folder(folder: &Path) -> io::Result<()> {
    match fs::remove_dir_all(folder) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

/// Starts the installed BuildPilot and waits, up to `wait_ms`, until its window is ready, so it
/// keeps the foreground once setup closes.
pub fn launch(wait_ms: u32) -> io::Result<()> {
    let folder = install_folder();
    processes::allow_any_foreground();
    let child = Command::new(folder.join(APP_EXE))
        .current_dir(&folder)
        .spawn()?;
    processes::wait_until_ready(&child, wait_ms);
    Ok(())
}
