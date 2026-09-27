use std::collections::HashSet;
use std::env;
use std::fs;

use buildpilot::application::ports::{Clock, IdSource, PathProbe};
use buildpilot::domain::launch_plan::PowerShellHost;
use buildpilot::infrastructure::powershell::detect_powershell;
use buildpilot::infrastructure::system::{FsPaths, SystemClock, UuidIds};

// LCH-001: pwsh when it is on PATH, Windows PowerShell otherwise.
#[test]
fn powershell_host_follows_path() {
    let root = tempfile::tempdir().unwrap();
    let empty = root.path().join("empty");
    let with_pwsh = root.path().join("with pwsh");
    fs::create_dir_all(&empty).unwrap();
    fs::create_dir_all(&with_pwsh).unwrap();
    fs::write(with_pwsh.join("pwsh.exe"), b"").unwrap();

    let found = env::join_paths([&empty, &with_pwsh]).unwrap();
    assert_eq!(detect_powershell(Some(&found)), PowerShellHost::Pwsh);
    let missing = env::join_paths([&empty]).unwrap();
    assert_eq!(
        detect_powershell(Some(&missing)),
        PowerShellHost::WindowsPowerShell
    );
    assert_eq!(detect_powershell(None), PowerShellHost::WindowsPowerShell);
}

// CFG-003: identities are never repeated.
#[test]
fn ids_are_unique() {
    let mut ids = UuidIds;
    let issued: HashSet<String> = (0..1000)
        .map(|_| ids.next_id().as_str().to_owned())
        .collect();
    assert_eq!(issued.len(), 1000);
}

#[test]
fn paths_answer_from_the_file_system() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("build.ps1");
    fs::write(&file, b"").unwrap();
    let paths = FsPaths;
    assert!(paths.is_file(&file) && !paths.is_dir(&file));
    assert!(paths.is_dir(root.path()) && !paths.is_file(root.path()));
    assert!(!paths.is_file(&root.path().join("gone.ps1")));
}

// Amendment 2. Assumes an OEM code page where 0x82 is é: 850 (measured on the reference
// machine) or 437.
#[cfg(windows)]
#[test]
fn oem_bytes_decode_in_the_console_code_page() {
    use buildpilot::infrastructure::win32::codepage::decode_oem;
    assert_eq!(decode_oem(b"caf\x82"), "caf\u{e9}");
    assert_eq!(decode_oem(b""), "");
}

#[test]
fn clock_moves_forwards() {
    let clock = SystemClock;
    let first = clock.now();
    assert!(clock.now() >= first);
}
