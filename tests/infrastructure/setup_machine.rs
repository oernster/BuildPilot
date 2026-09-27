use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};

use buildpilot::infrastructure::win32::{known_folders, processes};
use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

use std::fs;

use buildpilot::infrastructure::setup::remove_stale_copy;

// INST-003: the copy an uninstall ran from is cleared by the next setup that is not that copy.
#[test]
fn a_stale_copy_is_removed() {
    let folder = tempfile::tempdir().unwrap();
    let (this, copy) = (
        folder.path().join("dist.exe"),
        folder.path().join("copy.exe"),
    );
    fs::write(&this, b"setup").unwrap();
    fs::write(&copy, b"setup").unwrap();
    assert!(remove_stale_copy(&this, &copy).unwrap());
    assert!(!copy.exists());
}

// INST-003: the copy that is running keeps itself; one already gone is not an error.
#[test]
fn the_running_copy_and_a_missing_one_are_left_alone() {
    let folder = tempfile::tempdir().unwrap();
    let copy = folder.path().join("copy.exe");
    fs::write(&copy, b"setup").unwrap();
    assert!(!remove_stale_copy(&copy, &copy).unwrap());
    assert!(copy.exists());
    let gone = folder.path().join("gone.exe");
    assert!(!remove_stale_copy(&copy, &gone).unwrap());
}

/// Long enough for a test to see the process, far shorter than it would run on its own.
const WAIT_MS: u32 = 5000;

// INST-002: Windows says where the desktop and the Start menu's programs are.
#[test]
fn the_known_folders_are_found() {
    assert!(known_folders::desktop().is_some_and(|folder| folder.is_dir()));
    assert!(known_folders::start_menu_programs().is_some_and(|folder| folder.is_dir()));
}

// INST-004: a process is found by its executable's name and ended by its id.
#[test]
fn a_process_is_found_by_name_and_ended() {
    let mut child = Command::new("cmd.exe")
        .args(["/c", "ping", "-n", "30", "127.0.0.1"])
        .stdout(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .unwrap();
    let id = child.id();
    assert!(processes::running("CMD.EXE").contains(&id));
    assert!(processes::end(id, WAIT_MS));
    assert!(!processes::running("cmd.exe").contains(&id));
    let _ = child.wait();
}
