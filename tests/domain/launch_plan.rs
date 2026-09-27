use std::path::{Path, PathBuf};

use buildpilot::domain::environment::{Need, VariableEdits};
use buildpilot::domain::launch_plan::{LaunchPlan, PowerShellHost, ScriptKind, plan};
use buildpilot::domain::step::{Step, StepSpec};

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
        ("a.Py", Some(ScriptKind::Python)),
        ("a.sh", None),
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
    assert_eq!(
        ScriptKind::supported_list(),
        ".ps1, .bat, .cmd, .exe, .com, .py"
    );
}

// ENV-004, OQ-18: .py needs an environment, .ps1 uses one when there is one, the rest never.
#[test]
fn each_kind_states_its_environment_need() {
    assert_eq!(ScriptKind::Python.environment_need(), Need::Required);
    assert_eq!(ScriptKind::PowerShell.environment_need(), Need::Optional);
    assert_eq!(ScriptKind::Batch.environment_need(), Need::None);
    assert_eq!(ScriptKind::Executable.environment_need(), Need::None);
}

fn step_with_arguments(script: &str) -> Step {
    Step::try_from(StepSpec {
        script_path: PathBuf::from(script),
        arguments: vec!["--release".to_owned(), "two words".to_owned()],
    })
    .unwrap()
}

fn plan_of(script: &str, host: PowerShellHost, environment: Option<&Path>) -> LaunchPlan {
    let step = step_with_arguments(script);
    let folder = Path::new(script).parent().unwrap();
    plan(&step, folder, host, environment, VariableEdits::default())
}

// LCH-001: pwsh when found, PowerShell 5.1 otherwise; -File so the exit code passes through.
#[test]
fn powershell_script_runs_under_the_chosen_host() {
    for (host, program) in [
        (PowerShellHost::Pwsh, "pwsh.exe"),
        (PowerShellHost::WindowsPowerShell, "powershell.exe"),
    ] {
        let launch = plan_of(r"C:\my src\app\build.ps1", host, None);
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
        let launch = plan_of(script, PowerShellHost::Pwsh, None);
        assert_eq!(launch.program, PathBuf::from(script));
        assert_eq!(launch.arguments, ["--release", "two words"]);
    }
}

// ENV-006: a .py step runs under its environment's interpreter, the script first.
#[test]
fn python_runs_under_the_environment_interpreter() {
    let venv = Path::new(r"C:\src\app\venv");
    let launch = plan_of(r"C:\src\app\buildexe.py", PowerShellHost::Pwsh, Some(venv));
    assert_eq!(
        launch.program,
        PathBuf::from(r"C:\src\app\venv\Scripts\python.exe")
    );
    assert_eq!(
        launch.arguments,
        [r"C:\src\app\buildexe.py", "--release", "two words"]
    );
    // Outside Run, which refuses a .py step with no environment, the bare name is used.
    let bare = plan_of(r"C:\src\app\buildexe.py", PowerShellHost::Pwsh, None);
    assert_eq!(bare.program, PathBuf::from("python.exe"));
}

// ENV-006: the variable edits handed in are the plan's.
#[test]
fn plan_carries_the_variable_edits() {
    let edits = VariableEdits {
        remove: vec!["VIRTUAL_ENV".to_owned()],
        set: vec![("PATH".to_owned(), r"C:\Windows".to_owned())],
    };
    let step = step_with_arguments(r"C:\src\app\build.cmd");
    let launch = plan(
        &step,
        Path::new(r"C:\src\app"),
        PowerShellHost::Pwsh,
        None,
        edits.clone(),
    );
    assert_eq!(launch.variables, edits);
    assert_eq!(launch.working_dir, PathBuf::from(r"C:\src\app"));
}
