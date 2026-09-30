//! What a row or the tray says, worked out from application state. Pure: no Slint
//! types, no window, so every wording is tested without one.

use std::path::Path;
use std::time::Duration;

use crate::application::IconStatus;
use crate::domain::elapsed::format_elapsed;
use crate::domain::installer::InstallerBlock;
use crate::domain::lifecycle::{Failure, RunState};

/// Frames of the running indicator; one advances per UI tick (LIFE-005).
pub const SPINNER: [&str; 4] = ["\u{25D0}", "\u{25D3}", "\u{25D1}", "\u{25D2}"];

const GLYPH_IDLE: &str = "\u{25CB}";
const GLYPH_SUCCEEDED: &str = "\u{2713}";
const GLYPH_FAILED: &str = "\u{2717}";
const GLYPH_STOPPED: &str = "\u{25A0}";

/// A row's state class, which decides its glyph colour. The glyph and the words carry the same
/// meaning, so colour is never the only signal (A11Y, spec §19).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusClass {
    /// Not run.
    Idle,
    /// Running.
    Running,
    /// Exited 0.
    Succeeded,
    /// Did not succeed.
    Failed,
    /// Stopped by the operator.
    Stopped,
}

/// What the view knows about one operation.
pub struct RowFacts<'a> {
    /// Display name.
    pub name: &'a str,
    /// The first step's script.
    pub script: &'a Path,
    /// How many steps follow the first (ROW-001).
    pub later_steps: usize,
    /// Its latest run.
    pub state: &'a RunState,
    /// How long the latest run has run.
    pub elapsed: Option<Duration>,
    /// How long its build typically takes (LIFE-008).
    pub typical: Option<Duration>,
    /// The process still alive after a Stop that timed out (STOP-003).
    pub overdue_pid: Option<u32>,
    /// Another operation running in this one's folder, which holds its Run back (LCH-010).
    pub folder_busy_with: Option<&'a str>,
    /// Why Launch installer is not available; `None` when it is (PKG-003, PKG-004).
    pub installer_block: Option<InstallerBlock>,
    /// Which spinner frame to show.
    pub tick: usize,
}

/// The words and switches of one row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowText {
    /// Script file name and folder, enough to identify it (ROW-001).
    pub detail: String,
    /// The state glyph.
    pub glyph: &'static str,
    /// The state in words.
    pub status: String,
    /// Under the status in every state, how long the build typically takes; empty until one
    /// has succeeded (LIFE-008).
    pub typical: String,
    /// The state class.
    pub class: StatusClass,
    /// Something wrong the operator can act on; empty when nothing is.
    pub problem: String,
    /// Run is available (LCH-005, LCH-010).
    pub can_run: bool,
    /// Why Run is not, for its tooltip.
    pub run_blocked: String,
    /// Stop is available.
    pub can_stop: bool,
    /// Remove is available (REM-003).
    pub can_remove: bool,
    /// Launch installer is available (PKG-003, PKG-004).
    pub can_install: bool,
    /// Why it is not, for its tooltip.
    pub install_blocked: String,
}

/// The row for `facts`.
pub fn row_text(facts: &RowFacts<'_>) -> RowText {
    let running = facts.state.is_running();
    let (glyph, class, status, mut problem) = state_words(facts.state, facts.elapsed, facts.tick);
    if let Some(pid) = facts.overdue_pid {
        problem = format!(
            "Stop has not finished: process {pid} is still running. Press Stop again or end it in Task Manager."
        );
    }
    let run_blocked = match facts.folder_busy_with {
        _ if running => "already running".to_owned(),
        Some(other) => format!("{other} is building in this folder"),
        None => String::new(),
    };
    let typical = facts
        .typical
        .map(|took| format!("Typical build time: {}", format_elapsed(took)))
        .unwrap_or_default();
    RowText {
        detail: detail(facts.script, facts.later_steps),
        glyph,
        status,
        typical,
        class,
        problem,
        can_run: run_blocked.is_empty(),
        run_blocked,
        can_stop: running,
        can_remove: !running,
        can_install: facts.installer_block.is_none(),
        install_blocked: facts
            .installer_block
            .map(|block| block.to_string())
            .unwrap_or_default(),
    }
}

/// The tray's last line once a run has ended: the row's own glyph and words with the class that
/// colours it (OUT-007). `None` while the operation is idle or running.
pub fn closing_line(state: &RunState, elapsed: Option<Duration>) -> Option<(String, StatusClass)> {
    if matches!(state, RunState::Idle | RunState::Running(_)) {
        return None;
    }
    let (glyph, class, status, problem) = state_words(state, elapsed, 0);
    let line = if problem.is_empty() {
        format!("{glyph} {status}")
    } else {
        format!("{glyph} {status}: {problem}")
    };
    Some((line, class))
}

/// A run state's glyph, class, words and problem, as a row and the tray's last line say them.
fn state_words(
    state: &RunState,
    elapsed: Option<Duration>,
    tick: usize,
) -> (&'static str, StatusClass, String, String) {
    let took = elapsed.map(format_elapsed);
    match state {
        RunState::Idle => (
            GLYPH_IDLE,
            StatusClass::Idle,
            "Not run".to_owned(),
            String::new(),
        ),
        RunState::Running(details) => {
            let action = if details.stop_requested() {
                "Stopping"
            } else {
                "Running"
            };
            // STEP-007: which step, only where there is more than one.
            let verb = if details.steps() > 1 {
                format!("{action} step {} of {}", details.step(), details.steps())
            } else {
                action.to_owned()
            };
            let status = match &took {
                Some(took) => format!("{verb} {took}"),
                None => verb,
            };
            (
                SPINNER[tick % SPINNER.len()],
                StatusClass::Running,
                status,
                String::new(),
            )
        }
        RunState::Succeeded => (
            GLYPH_SUCCEEDED,
            StatusClass::Succeeded,
            after("Succeeded", &took),
            String::new(),
        ),
        RunState::Failed(Failure::ExitCode(code)) => {
            let status = match &took {
                Some(took) => format!("Failed, exit code {code}, after {took}"),
                None => format!("Failed, exit code {code}"),
            };
            (GLYPH_FAILED, StatusClass::Failed, status, String::new())
        }
        RunState::Failed(Failure::StepExitCode { step, code }) => {
            let failed = format!("Failed, step {step} exited with code {code}");
            let status = match &took {
                Some(took) => format!("{failed}, after {took}"),
                None => failed,
            };
            (GLYPH_FAILED, StatusClass::Failed, status, String::new())
        }
        RunState::Failed(Failure::FailedToStart(error)) => (
            GLYPH_FAILED,
            StatusClass::Failed,
            "Failed to start".to_owned(),
            error.to_string(),
        ),
        RunState::Stopped => (
            GLYPH_STOPPED,
            StatusClass::Stopped,
            after("Stopped", &took),
            String::new(),
        ),
    }
}

/// `label` followed by "after m:ss" when the duration is known.
fn after(label: &str, took: &Option<String>) -> String {
    match took {
        Some(took) => format!("{label} after {took}"),
        None => label.to_owned(),
    }
}

/// `build.ps1 in C:\src\app`; `buildexe.py +1 in C:\src\app` when one step follows it (ROW-001).
pub fn detail(script: &Path, later_steps: usize) -> String {
    let mut file = script
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    if later_steps > 0 {
        file = format!("{file} +{later_steps}");
    }
    match script.parent() {
        Some(folder) if !folder.as_os_str().is_empty() => format!("{file} in {}", folder.display()),
        _ => file,
    }
}

/// What to say about an icon that cannot be shown (ICON-005); empty when it can.
pub fn icon_problem(status: &IconStatus) -> String {
    match status {
        IconStatus::Missing(path) => format!("Icon not found or not an image: {}", path.display()),
        IconStatus::Placeholder | IconStatus::Ready(_) => String::new(),
    }
}

/// The tray's note about dropped lines (OUT-004); empty when none were dropped.
pub fn dropped_note(dropped: u64) -> String {
    if dropped == 0 {
        String::new()
    } else {
        format!("Oldest {dropped} lines not kept")
    }
}
