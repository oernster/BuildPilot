use std::path::{Path, PathBuf};

use buildpilot::domain::launch_plan::{PowerShellHost, ScriptKind, plan};
use buildpilot::domain::operation::OperationConfig;

use super::support::spec;

// LCH-001: the table, one row per extension, regardless of case.
#[test]
fn kinds_by_extension() {
    let cases = [
        ("a.ps1", Some(ScriptKind::PowerShell)),
        ("a.PS1", Some(ScriptKind::PowerShell)),
        ("a.bat", Some(ScriptKind::Batch)),
        ("a.Cmd", Some(ScriptKind::Batch)),
        ("a.exe", Some(ScriptKind::Executable)),
        ("a.com", Some(ScriptKind::Executable)),
        ("a.py", None),
        ("a", None),
    ];
    for (name, expected) in cases {
        assert_eq!(ScriptKind::of(Path::new(name)), expected, "{name}");
    }
}

// A Windows path may hold unpaired UTF-16 surrogates; such an extension is unsupported, not a
// crash.
#[cfg(windows)]
#[test]
fn non_unicode_extension_is_unsupported() {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    const LONE_SURROGATE: u16 = 0xD800;
    let mut wide: Vec<u16> = "a.".encode_utf16().collect();
    wide.push(LONE_SURROGATE);
    let path = PathBuf::from(OsString::from_wide(&wide));
    assert_eq!(ScriptKind::of(&path), None);
}

#[test]
fn supported_list_reads_naturally() {
    assert_eq!(ScriptKind::supported_list(), ".ps1, .bat, .cmd, .exe, .com");
}

fn config_with_arguments(script: &str) -> OperationConfig {
    let mut spec = spec(script);
    spec.arguments = vec!["--release".to_owned(), "two words".to_owned()];
    OperationConfig::try_from(spec).unwrap()
}

// LCH-001: pwsh when found, PowerShell 5.1 otherwise; -File so the exit code passes through.
#[test]
fn powershell_script_runs_under_the_chosen_host() {
    let config = config_with_arguments(r"C:\my src\app\build.ps1");
    for (host, program) in [
        (PowerShellHost::Pwsh, "pwsh.exe"),
        (PowerShellHost::WindowsPowerShell, "powershell.exe"),
    ] {
        let launch = plan(&config, host);
        assert_eq!(launch.program, PathBuf::from(program));
        assert_eq!(
            launch.arguments,
            [
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                r"C:\my src\app\build.ps1",
                "--release",
                "two words",
            ]
        );
        assert_eq!(launch.working_dir, PathBuf::from(r"C:\my src\app"));
    }
}

// LCH-001, Amendment 1: batch files and executables are the program themselves.
#[test]
fn batch_and_executable_run_directly() {
    for script in [r"C:\src\app\build.cmd", r"C:\src\app\tool.exe"] {
        let launch = plan(&config_with_arguments(script), PowerShellHost::Pwsh);
        assert_eq!(launch.program, PathBuf::from(script));
        assert_eq!(launch.arguments, ["--release", "two words"]);
    }
}
