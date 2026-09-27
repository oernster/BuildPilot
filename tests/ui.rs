//! The UI's pure wording: what rows and the tray say. No window is opened.

use std::path::{Path, PathBuf};
use std::time::Duration;

use buildpilot::application::IconStatus;
use buildpilot::domain::lifecycle::{Failure, LaunchError, RunState};
use buildpilot::ui::icon_image::load_scaled;
use buildpilot::ui::rows::{
    RowFacts, SPINNER, StatusClass, closing_line, detail, dropped_note, icon_problem, row_text,
};

const SCRIPT: &str = r"C:\src\app\build.ps1";

// OUT-007: a run that has ended closes the tray with the row's glyph and words, classed by how
// it ended so the tray colours it; nothing closes a run that is idle or still going.
#[test]
fn a_finished_run_closes_the_tray_in_its_outcome() {
    let took = Some(Duration::from_secs(83));
    let cases = [
        (
            RunState::Succeeded,
            "\u{2713} Succeeded after 1:23",
            StatusClass::Succeeded,
        ),
        (
            RunState::Failed(Failure::ExitCode(1)),
            "\u{2717} Failed, exit code 1, after 1:23",
            StatusClass::Failed,
        ),
        (
            RunState::Failed(Failure::StepExitCode { step: 2, code: 3 }),
            "\u{2717} Failed, step 2 exited with code 3, after 1:23",
            StatusClass::Failed,
        ),
        (
            RunState::Stopped,
            "\u{25A0} Stopped after 1:23",
            StatusClass::Stopped,
        ),
    ];
    for (state, text, class) in cases {
        assert_eq!(
            closing_line(&state, took),
            Some((text.to_owned(), class)),
            "{state:?}"
        );
    }
    let unstarted = RunState::Failed(Failure::FailedToStart(LaunchError::ScriptNotFound(
        PathBuf::from(SCRIPT),
    )));
    let (text, class) = closing_line(&unstarted, None).unwrap();
    assert_eq!(class, StatusClass::Failed);
    assert!(
        text.starts_with("\u{2717} Failed to start: Script not found"),
        "{text}"
    );
    assert_eq!(closing_line(&RunState::Idle, None), None);
    let running = RunState::Idle.launched(1).unwrap();
    assert_eq!(closing_line(&running, took), None);
}

/// The pixels a test icon is shrunk to fit.
const ICON_PIXELS: u32 = 144;

/// A file in the repository's assets folder.
fn asset(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(name)
}

// ICON-004, UI-013: an ICO icon draws as well as a PNG, both shrunk to the pixels asked for. The
// ICO is BuildPilot's own, holding 16 to 256 px images; the largest is the one drawn.
#[test]
fn ico_and_png_icons_load_shrunk() {
    for name in ["application-icon.ico", "application-icon.png"] {
        let image = load_scaled(&asset(name), ICON_PIXELS).unwrap_or_else(|| panic!("{name}"));
        let size = image.size();
        assert_eq!(size.width.max(size.height), ICON_PIXELS, "{name}: {size:?}");
    }
    assert!(load_scaled(&asset("..").join("Cargo.toml"), ICON_PIXELS).is_none());
}

fn text_for(
    state: &RunState,
    elapsed: Option<Duration>,
    overdue_pid: Option<u32>,
    tick: usize,
) -> buildpilot::ui::rows::RowText {
    row_text(&RowFacts {
        name: "app build",
        script: Path::new(SCRIPT),
        later_steps: 0,
        state,
        elapsed,
        overdue_pid,
        folder_busy_with: None,
        tick,
    })
}

fn running(stop_requested: bool) -> RunState {
    let state = RunState::Idle.launched(1).unwrap();
    if stop_requested {
        state.stop_requested().unwrap()
    } else {
        state
    }
}

// ROW-001
#[test]
fn detail_names_the_file_and_its_folder() {
    assert_eq!(detail(Path::new(SCRIPT), 0), r"build.ps1 in C:\src\app");
    assert_eq!(detail(Path::new("build.ps1"), 0), "build.ps1");
    // ROW-001, Amendment 4: the first step, then how many follow.
    assert_eq!(
        detail(Path::new(r"C:\src\app\buildexe.py"), 1),
        r"buildexe.py +1 in C:\src\app"
    );
}

// STEP-007, STEP-003: a run of several steps says which step it is on or which one failed.
#[test]
fn steps_read_in_words() {
    let second = RunState::Idle.launched(2).unwrap().exited(0).unwrap();
    let took = Some(Duration::from_secs(65));
    assert_eq!(
        text_for(&second, took, None, 0).status,
        "Running step 2 of 2 1:05"
    );
    let stopping = second.stop_requested().unwrap();
    assert_eq!(
        text_for(&stopping, None, None, 0).status,
        "Stopping step 2 of 2"
    );
    let failed = RunState::Failed(Failure::StepExitCode { step: 1, code: 3 });
    assert_eq!(
        text_for(&failed, took, None, 0).status,
        "Failed, step 1 exited with code 3, after 1:05"
    );
    assert_eq!(
        text_for(&failed, None, None, 0).status,
        "Failed, step 1 exited with code 3"
    );
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
    assert_eq!(busy.run_blocked, "already running");
    assert_eq!(idle.run_blocked, "");
}

// LCH-010: another build in the same folder holds Run back, saying which.
#[test]
fn a_busy_folder_holds_run_back() {
    let text = row_text(&RowFacts {
        name: "app build",
        script: Path::new(SCRIPT),
        later_steps: 0,
        state: &RunState::Succeeded,
        elapsed: None,
        overdue_pid: None,
        folder_busy_with: Some("app release"),
        tick: 0,
    });
    assert!(!text.can_run && !text.can_stop && text.can_remove);
    assert_eq!(text.run_blocked, "app release is building in this folder");
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
