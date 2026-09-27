use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};

use buildpilot::infrastructure::win32::{known_folders, processes};
use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

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
