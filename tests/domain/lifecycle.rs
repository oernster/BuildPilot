use std::path::PathBuf;

use buildpilot::domain::environment::EnvironmentProblem;
use buildpilot::domain::lifecycle::{
    Failure, LaunchError, Percent, Progress, RunState, TransitionError,
};

fn running() -> RunState {
    RunState::Idle.launched(1).unwrap()
}

/// A run of `steps` steps, moved on to its `step`th by exits of 0.
fn at_step(step: usize, steps: usize) -> RunState {
    (1..step).fold(RunState::Idle.launched(steps).unwrap(), |state, _| {
        state.exited(0).unwrap()
    })
}

fn position(state: &RunState) -> (usize, usize) {
    let RunState::Running(details) = state else {
        panic!("running: {state:?}");
    };
    (details.step(), details.steps())
}

// STEP-002: 0 moves to the next step; after the last it is success.
#[test]
fn zero_moves_through_the_steps() {
    let first = RunState::Idle.launched(2).unwrap();
    assert_eq!(position(&first), (1, 2));
    let second = first.exited(0).unwrap();
    assert_eq!(position(&second), (2, 2));
    assert_eq!(second.exited(0), Ok(RunState::Succeeded));
}

// STEP-003: a failing step ends the run, naming the step.
#[test]
fn a_failing_step_ends_the_run() {
    assert_eq!(
        at_step(1, 3).exited(3),
        Ok(RunState::Failed(Failure::StepExitCode { step: 1, code: 3 }))
    );
    assert_eq!(
        at_step(2, 3).exited(-1),
        Ok(RunState::Failed(Failure::StepExitCode {
            step: 2,
            code: -1
        }))
    );
}

// STEP-004: Stop during a step ends the run as Stopped, even when the step exits 0.
#[test]
fn stop_during_a_step_ends_the_sequence() {
    let stopping = at_step(1, 2).stop_requested().unwrap();
    assert_eq!(stopping.exited(0), Ok(RunState::Stopped));
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
        let next = state.launched(1).unwrap();
        assert!(next.is_running(), "{state:?}");
        assert!(!state.is_running());
    }
}

// LCH-005
#[test]
fn second_launch_is_refused() {
    assert_eq!(running().launched(1), Err(TransitionError::AlreadyRunning));
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

// ENV-004, ENV-005: an environment refusal names the folder searched and never offers to make
// one.
#[test]
fn environment_refusals_name_the_folder() {
    let problems = [
        (
            EnvironmentProblem::NoneFound,
            "No Python environment was found",
        ),
        (
            EnvironmentProblem::NamedMissing("venv2".to_owned()),
            "The environment venv2 was not found",
        ),
        (
            EnvironmentProblem::Several(vec!["venv".to_owned(), "venv_smoke".to_owned()]),
            "Several Python environments were found (venv, venv_smoke): choose one in Edit",
        ),
    ];
    for (problem, opening) in problems {
        let text = LaunchError::Environment {
            problem,
            searched: PathBuf::from(r"C:\src\app"),
        }
        .to_string();
        assert_eq!(
            text,
            format!(
                r"{opening} in C:\src\app. BuildPilot uses an existing environment and does not create one."
            )
        );
    }
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
