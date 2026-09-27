//! Real processes, started hidden. Fixtures live in `tests/fixtures` and are copied to a folder
//! whose name holds a space and a non-ASCII character, so every launch also proves LCH-003.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use buildpilot::application::ports::{Launcher, ProcessHandle, RunEvent, RunEventKind, RunKey};
use buildpilot::domain::environment::{VariableEdits, step_variables};
use buildpilot::domain::launch_plan::{LaunchPlan, plan};
use buildpilot::domain::operation::{OperationConfig, OperationId, draft_for_script};
use buildpilot::domain::output::Stream;
use buildpilot::infrastructure::launcher::{EventSink, PIPE_DRAIN_TIMEOUT, WindowsLauncher};
use buildpilot::infrastructure::powershell::detect_powershell;
use buildpilot::infrastructure::win32::job::process_is_alive;

/// Longest any single event may take to arrive, PowerShell start-up included.
const EVENT_WAIT: Duration = Duration::from_secs(30);
/// How often to look again while waiting for a process to disappear.
const POLL: Duration = Duration::from_millis(100);
/// PowerShell start-up allowance on top of the pipe drain timeout.
const STARTUP_ALLOWANCE: Duration = Duration::from_secs(15);

const ARGUMENTS: [&str; 3] = ["plain", "two words", r"C:\with space\x"];

struct Rig {
    folder: tempfile::TempDir,
    launcher: WindowsLauncher,
    events: Receiver<RunEvent>,
}

/// Kills a leftover grandchild if a test fails before cleaning up.
struct Reap(u32);

impl Drop for Reap {
    fn drop(&mut self) {
        if process_is_alive(self.0) {
            let _ = Command::new("taskkill")
                .args(["/F", "/PID", &self.0.to_string()])
                .output();
        }
    }
}

fn rig() -> Rig {
    let folder = tempfile::Builder::new()
        .prefix("with space \u{e9} ")
        .tempdir()
        .unwrap();
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    for entry in fs::read_dir(fixtures).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), folder.path().join(entry.file_name())).unwrap();
    }
    let (sender, events) = mpsc::channel();
    let launcher = WindowsLauncher::new(EventSink::new(sender, Arc::new(|| {})));
    Rig {
        folder,
        launcher,
        events,
    }
}

impl Rig {
    fn plan(&self, fixture: &str, arguments: &[&str]) -> LaunchPlan {
        self.plan_in(fixture, arguments, None, VariableEdits::default())
    }

    /// `fixture`'s one step, run in `environment` where given, with `variables`.
    fn plan_in(
        &self,
        fixture: &str,
        arguments: &[&str],
        environment: Option<&Path>,
        variables: VariableEdits,
    ) -> LaunchPlan {
        let mut spec = draft_for_script(&self.folder.path().join(fixture)).unwrap();
        spec.steps[0].arguments = arguments
            .iter()
            .map(|argument| (*argument).to_owned())
            .collect();
        let config = OperationConfig::try_from(spec).unwrap();
        plan(
            config.first_step(),
            config.working_dir(),
            detect_powershell(env::var_os("PATH").as_deref()),
            environment,
            variables,
        )
    }

    fn start(&mut self, run: u64, fixture: &str, arguments: &[&str]) -> Box<dyn ProcessHandle> {
        let key = RunKey {
            operation: OperationId::new(fixture).unwrap(),
            run,
        };
        let plan = self.plan(fixture, arguments);
        self.launcher.spawn(key, &plan).unwrap()
    }

    fn next(&self) -> RunEvent {
        self.events
            .recv_timeout(EVENT_WAIT)
            .expect("an event in time")
    }

    /// Lines until run `run` exits, with its exit code; other runs' events are skipped.
    fn until_exit(&self, run: u64) -> (Vec<(Stream, String)>, i32) {
        let mut lines = Vec::new();
        loop {
            let event = self.next();
            if event.key.run != run {
                continue;
            }
            match event.kind {
                RunEventKind::Line { stream, text } => lines.push((stream, text)),
                RunEventKind::Exited { code } => return (lines, code),
            }
        }
    }

    /// The grandchild's process id, from the fixture's `child:` line.
    fn child_pid(&self, run: u64) -> u32 {
        loop {
            let event = self.next();
            if let (true, RunEventKind::Line { text, .. }) = (event.key.run == run, &event.kind)
                && let Some(pid) = text.strip_prefix("child:")
            {
                return pid.trim().parse().unwrap();
            }
        }
    }
}

fn wait_until_gone(pid: u32) -> bool {
    let deadline = Instant::now() + EVENT_WAIT;
    while Instant::now() < deadline {
        if !process_is_alive(pid) {
            return true;
        }
        thread::sleep(POLL);
    }
    false
}

fn stdout_texts(lines: &[(Stream, String)]) -> Vec<&str> {
    lines
        .iter()
        .filter(|(stream, _)| *stream == Stream::Stdout)
        .map(|(_, text)| text.as_str())
        .collect()
}

// LCH-002, LCH-003, LIFE-002, OUT-001, OUT-007: a .ps1 gets its arguments and folder intact,
// both streams arrive attributed and the exit code passes through -File.
#[test]
fn powershell_script_arguments_streams_and_exit_code() {
    let mut rig = rig();
    let _process = rig.start(1, "echo_args.ps1", &ARGUMENTS);
    let (lines, code) = rig.until_exit(1);
    let folder = rig.folder.path().to_string_lossy().into_owned();
    let mut expected: Vec<String> = ARGUMENTS.iter().map(|a| format!("arg:{a}")).collect();
    expected.push(format!("cwd:{folder}"));
    assert_eq!(stdout_texts(&lines), expected);
    assert!(lines.contains(&(Stream::Stderr, "to stderr".to_owned())));
    assert_eq!(code, 3);
}

// LCH-001, Amendment 1: a .cmd run through the standard library gets its arguments intact.
#[test]
fn batch_script_arguments_streams_and_exit_code() {
    let mut rig = rig();
    let _process = rig.start(1, "echo_args.cmd", &ARGUMENTS);
    let (lines, code) = rig.until_exit(1);
    let folder = rig.folder.path().to_string_lossy().into_owned();
    let mut expected: Vec<String> = ARGUMENTS.iter().map(|a| format!("arg:{a}")).collect();
    expected.push(format!("cwd:{folder}"));
    assert_eq!(stdout_texts(&lines), expected);
    assert!(lines.contains(&(Stream::Stderr, "to stderr ".to_owned())));
    assert_eq!(code, 4);
}

// LCH-009: a program that does not exist is refused with the operating system's message.
#[test]
fn missing_program_is_refused() {
    let mut rig = rig();
    let key = RunKey {
        operation: OperationId::new("ghost").unwrap(),
        run: 1,
    };
    let ghost = LaunchPlan {
        program: PathBuf::from(r"C:\no such folder\ghost.exe"),
        arguments: Vec::new(),
        working_dir: rig.folder.path().to_path_buf(),
        variables: VariableEdits::default(),
    };
    let error = rig.launcher.spawn(key, &ghost).err().expect("refused");
    assert!(!error.is_empty());
}

// ENV-006, ENV-009: the step's variables reach the process with activation first on PATH;
// setting PATH replaces the inherited Path rather than adding a second one.
#[test]
fn an_activated_step_sees_its_environment() {
    let mut rig = rig();
    let venv = rig.folder.path().join("venv");
    fs::create_dir_all(venv.join("Scripts")).unwrap();
    fs::write(venv.join("pyvenv.cfg"), "").unwrap();
    fs::write(venv.join(r"Scripts\python.exe"), "").unwrap();
    let inherited: Vec<(String, String)> = env::vars_os()
        .map(|(name, value)| {
            (
                name.to_string_lossy().into_owned(),
                value.to_string_lossy().into_owned(),
            )
        })
        .collect();
    let variables = step_variables(&inherited, Some(&venv), |path| path.is_file());
    let plan = rig.plan_in("print_env.ps1", &[], Some(&venv), variables);
    let key = RunKey {
        operation: OperationId::new("env").unwrap(),
        run: 1,
    };
    let _process = rig.launcher.spawn(key, &plan).unwrap();
    let (lines, code) = rig.until_exit(1);
    assert_eq!(code, 0);
    let texts: Vec<String> = lines.into_iter().map(|(_, text)| text).collect();
    assert_eq!(
        texts,
        [
            format!("VIRTUAL_ENV:{}", venv.display()),
            format!("PATH0:{}", venv.join("Scripts").display()),
            format!("PYTHON:{}", venv.join(r"Scripts\python.exe").display()),
            "PYTHONIOENCODING:utf-8".to_owned(),
            "PYTHONUNBUFFERED:1".to_owned(),
            "PATHS:1".to_owned(),
        ]
    );
}

// STOP-001: Stop ends the script and the process it started.
#[test]
fn stop_ends_the_whole_tree() {
    let mut rig = rig();
    let mut process = rig.start(1, "spawn_tree.ps1", &[]);
    let child = rig.child_pid(1);
    let _reap = Reap(child);
    assert!(process_is_alive(child));
    process.stop().unwrap();
    let (_, _code) = rig.until_exit(1);
    assert!(wait_until_gone(child), "grandchild {child} survived Stop");
    assert!(wait_until_gone(process.pid()));
}

// STOP-002: stopping one run leaves another running.
#[test]
fn stopping_one_leaves_another_running() {
    let mut rig = rig();
    let mut first = rig.start(1, "spawn_tree.ps1", &[]);
    let first_child = rig.child_pid(1);
    let _reap_first = Reap(first_child);
    let mut second = rig.start(2, "spawn_tree.ps1", &[]);
    let second_child = rig.child_pid(2);
    let _reap_second = Reap(second_child);

    first.stop().unwrap();
    rig.until_exit(1);
    assert!(wait_until_gone(first_child));
    assert!(process_is_alive(second_child));
    assert!(process_is_alive(second.pid()));
    second.stop().unwrap();
    rig.until_exit(2);
}

// STOP-005: dropping the handle while the script still runs ends the tree, as a crash would.
#[test]
fn dropping_a_running_handle_ends_the_tree() {
    let mut rig = rig();
    let process = rig.start(1, "spawn_tree.ps1", &[]);
    let child = rig.child_pid(1);
    let _reap = Reap(child);
    drop(process);
    rig.until_exit(1);
    assert!(
        wait_until_gone(child),
        "grandchild {child} survived its job closing"
    );
}

// A script that exits leaving a background process is reported finished within the drain
// timeout; the background process is released rather than killed.
#[test]
fn a_finished_script_releases_what_it_left_running() {
    let mut rig = rig();
    let started = Instant::now();
    let process = rig.start(1, "leave_daemon.ps1", &[]);
    let child = rig.child_pid(1);
    let _reap = Reap(child);
    let (_, code) = rig.until_exit(1);
    assert_eq!(code, 0);
    assert!(started.elapsed() < PIPE_DRAIN_TIMEOUT + STARTUP_ALLOWANCE);
    drop(process);
    thread::sleep(POLL);
    assert!(
        process_is_alive(child),
        "the left-behind process was killed"
    );
}
