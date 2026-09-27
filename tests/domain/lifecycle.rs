use std::path::PathBuf;

use buildpilot::domain::lifecycle::{
    Failure, LaunchError, Percent, Progress, RunState, TransitionError,
};

fn running() -> RunState {
    RunState::Idle.launched().unwrap()
}

fn finished_states() -> [RunState; 4] {
    [
        RunState::Idle,
        RunState::Succeeded,
        RunState::Failed(Failure::ExitCode(1)),
        RunState::Stopped,
    ]
}

// LIFE-001, LCH-002: every state that is not running can launch.
#[test]
fn every_idle_or_finished_state_can_launch() {
    for state in finished_states() {
        let next = state.launched().unwrap();
        assert!(next.is_running(), "{state:?}");
        assert!(!state.is_running());
    }
}

// LCH-005
#[test]
fn second_launch_is_refused() {
    assert_eq!(running().launched(), Err(TransitionError::AlreadyRunning));
    let error = LaunchError::ScriptNotFound(PathBuf::from("x"));
    assert_eq!(
        running().launch_failed(error),
        Err(TransitionError::AlreadyRunning)
    );
}

// LCH-007
#[test]
fn launch_failure_is_a_failed_state() {
    let error = LaunchError::ScriptNotFound(PathBuf::from(r"C:\gone\build.ps1"));
    assert_eq!(
        RunState::Idle.launch_failed(error.clone()),
        Ok(RunState::Failed(Failure::FailedToStart(error)))
    );
}

// LIFE-002
#[test]
fn exit_code_decides_success() {
    assert_eq!(running().exited(0), Ok(RunState::Succeeded));
    assert_eq!(
        running().exited(3),
        Ok(RunState::Failed(Failure::ExitCode(3)))
    );
    assert_eq!(
        running().exited(-1),
        Ok(RunState::Failed(Failure::ExitCode(-1)))
    );
}

// LIFE-003: a stopped run is Stopped whatever the code, including 0.
#[test]
fn stop_wins_over_exit_code() {
    let stopping = running().stop_requested().unwrap();
    for code in [0, 1, -1] {
        assert_eq!(stopping.exited(code), Ok(RunState::Stopped));
    }
}

// STOP-003: Stop can be asked again.
#[test]
fn stop_request_is_repeatable() {
    let stopping = running().stop_requested().unwrap();
    assert_eq!(stopping.stop_requested(), Ok(stopping.clone()));
    let RunState::Running(details) = stopping else {
        panic!("still running");
    };
    assert!(details.stop_requested());
}

#[test]
fn stop_and_exit_need_a_running_process() {
    for state in finished_states() {
        assert_eq!(state.stop_requested(), Err(TransitionError::NotRunning));
        assert_eq!(state.exited(0), Err(TransitionError::NotRunning));
    }
}

// LIFE-006: v1 runs are indeterminate.
#[test]
fn new_run_is_indeterminate() {
    let RunState::Running(details) = running() else {
        panic!("running");
    };
    assert_eq!(details.progress(), Progress::Indeterminate);
    assert!(!details.stop_requested());
}

#[test]
fn percent_is_bounded() {
    assert_eq!(Percent::new(100).map(Percent::value), Some(100));
    assert_eq!(Percent::new(0).map(Percent::value), Some(0));
    assert_eq!(Percent::new(101), None);
    let determinate = Progress::Determinate(Percent::new(40).unwrap());
    assert_ne!(determinate, Progress::Indeterminate);
}

// LCH-007 to LCH-009: every launch error says what to do or what failed.
#[test]
fn launch_errors_are_actionable() {
    let script = LaunchError::ScriptNotFound(PathBuf::from(r"C:\gone\build.ps1")).to_string();
    assert!(script.contains(r"C:\gone\build.ps1") && script.contains("Edit"));
    let folder = LaunchError::WorkingDirNotFound(PathBuf::from(r"C:\gone")).to_string();
    assert!(folder.contains(r"C:\gone") && folder.contains("Edit"));
    let os = LaunchError::Os {
        command: "pwsh.exe -File build.ps1".to_owned(),
        message: "Access is denied.".to_owned(),
    }
    .to_string();
    assert_eq!(
        os,
        "Could not start pwsh.exe -File build.ps1: Access is denied."
    );
}

#[test]
fn transition_errors_have_messages() {
    assert!(
        TransitionError::AlreadyRunning
            .to_string()
            .contains("already running")
    );
    assert!(
        TransitionError::NotRunning
            .to_string()
            .contains("not running")
    );
}
