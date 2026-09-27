//! The UI's pure wording: what rows, notices and the tray say. No window is opened.

use std::path::{Path, PathBuf};
use std::time::Duration;

use buildpilot::application::{IconStatus, LoadProblem, Notice, StoreError};
use buildpilot::domain::lifecycle::{Failure, LaunchError, RunState};
use buildpilot::domain::operation::OperationId;
use buildpilot::ui::rows::{
    RowFacts, SPINNER, StatusClass, detail, dropped_note, icon_problem, notice_text, row_text,
};

const SCRIPT: &str = r"C:\src\app\build.ps1";

fn text_for(
    state: &RunState,
    elapsed: Option<Duration>,
    overdue_pid: Option<u32>,
    tick: usize,
) -> buildpilot::ui::rows::RowText {
    row_text(&RowFacts {
        name: "app build",
        script: Path::new(SCRIPT),
        state,
        elapsed,
        overdue_pid,
        tick,
    })
}

fn running(stop_requested: bool) -> RunState {
    let state = RunState::Idle.launched().unwrap();
    if stop_requested {
        state.stop_requested().unwrap()
    } else {
        state
    }
}

// ROW-001
#[test]
fn detail_names_the_file_and_its_folder() {
    assert_eq!(detail(Path::new(SCRIPT)), r"build.ps1 in C:\src\app");
    assert_eq!(detail(Path::new("build.ps1")), "build.ps1");
}

// LIFE-005: every state has a glyph and words, so colour is never the only signal.
#[test]
fn each_state_reads_in_words() {
    let took = Some(Duration::from_secs(65));
    let cases = [
        (RunState::Idle, None, StatusClass::Idle, "Not run"),
        (running(false), took, StatusClass::Running, "Running 1:05"),
        (running(true), took, StatusClass::Running, "Stopping 1:05"),
        (running(false), None, StatusClass::Running, "Running"),
        (
            RunState::Succeeded,
            took,
            StatusClass::Succeeded,
            "Succeeded after 1:05",
        ),
        (
            RunState::Succeeded,
            None,
            StatusClass::Succeeded,
            "Succeeded",
        ),
        (
            RunState::Failed(Failure::ExitCode(3)),
            took,
            StatusClass::Failed,
            "Failed, exit code 3, after 1:05",
        ),
        (
            RunState::Failed(Failure::ExitCode(3)),
            None,
            StatusClass::Failed,
            "Failed, exit code 3",
        ),
        (
            RunState::Stopped,
            took,
            StatusClass::Stopped,
            "Stopped after 1:05",
        ),
    ];
    for (state, elapsed, class, status) in cases {
        let text = text_for(&state, elapsed, None, 0);
        assert_eq!(text.class, class, "{status}");
        assert_eq!(text.status, status);
        assert!(!text.glyph.is_empty());
        assert!(text.problem.is_empty());
    }
}

// LCH-005, REM-003: the controls that apply; only those.
#[test]
fn controls_follow_the_state() {
    let idle = text_for(&RunState::Idle, None, None, 0);
    assert!(idle.can_run && !idle.can_stop && idle.can_remove);
    let busy = text_for(&running(false), None, None, 0);
    assert!(!busy.can_run && busy.can_stop && !busy.can_remove);
}

// LIFE-005: the running indicator moves with the tick and wraps.
#[test]
fn spinner_advances_and_wraps() {
    let state = running(false);
    let frames: Vec<&str> = (0..=SPINNER.len())
        .map(|tick| text_for(&state, None, None, tick).glyph)
        .collect();
    assert_eq!(frames[..SPINNER.len()], SPINNER);
    assert_eq!(frames[SPINNER.len()], SPINNER[0]);
}

// LCH-007, STOP-003: problems are named where the operator can act on them.
#[test]
fn problems_are_named() {
    let error = LaunchError::ScriptNotFound(PathBuf::from(r"C:\gone\build.ps1"));
    let failed = RunState::Failed(Failure::FailedToStart(error.clone()));
    let text = text_for(&failed, None, None, 0);
    assert_eq!(text.status, "Failed to start");
    assert_eq!(text.problem, error.to_string());
    let overdue = text_for(&running(true), None, Some(4242), 0);
    assert!(overdue.problem.contains("4242") && overdue.problem.contains("Task Manager"));
}

// ICON-005
#[test]
fn icon_problem_only_for_missing_icons() {
    let path = PathBuf::from(r"C:\icons\a.png");
    assert!(icon_problem(&IconStatus::Missing(path.clone())).contains(r"C:\icons\a.png"));
    assert!(icon_problem(&IconStatus::Ready(path)).is_empty());
    assert!(icon_problem(&IconStatus::Placeholder).is_empty());
}

// OUT-004
#[test]
fn dropped_note_appears_only_after_drops() {
    assert!(dropped_note(0).is_empty());
    assert_eq!(dropped_note(12), "Oldest 12 lines not kept");
}

#[test]
fn every_notice_has_words() {
    let path = PathBuf::from(r"C:\data\buildpilot.json");
    let notices = [
        Notice::Load(LoadProblem::UnreadableEntries(2)),
        Notice::Load(LoadProblem::SetAside(path.clone())),
        Notice::Load(LoadProblem::NewerSchema),
        Notice::Load(LoadProblem::Unreadable {
            path: path.clone(),
            message: "denied".to_owned(),
        }),
        Notice::DuplicateDropped(OperationId::new("x").unwrap()),
        Notice::SaveFailed(StoreError {
            path,
            message: "full".to_owned(),
        }),
        Notice::StopFailed {
            name: "app".to_owned(),
            message: "denied".to_owned(),
        },
    ];
    for notice in &notices {
        assert!(notice_text(notice).len() > 20, "{notice:?}");
    }
    assert!(notice_text(&notices[0]).starts_with('2'));
}
