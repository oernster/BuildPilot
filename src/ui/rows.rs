//! What a row, a notice or the tray says, worked out from application state. Pure: no Slint
//! types, no window, so every wording is tested without one.

use std::path::Path;
use std::time::Duration;

use crate::application::{IconStatus, LoadProblem, Notice};
use crate::domain::elapsed::format_elapsed;
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
    /// The script.
    pub script: &'a Path,
    /// Its latest run.
    pub state: &'a RunState,
    /// How long the latest run has run.
    pub elapsed: Option<Duration>,
    /// The process still alive after a Stop that timed out (STOP-003).
    pub overdue_pid: Option<u32>,
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
    /// The state class.
    pub class: StatusClass,
    /// Something wrong the operator can act on; empty when nothing is.
    pub problem: String,
    /// Run is available (LCH-005).
    pub can_run: bool,
    /// Stop is available.
    pub can_stop: bool,
    /// Remove is available (REM-003).
    pub can_remove: bool,
}

/// The row for `facts`.
pub fn row_text(facts: &RowFacts<'_>) -> RowText {
    let running = facts.state.is_running();
    let took = facts.elapsed.map(format_elapsed);
    let (glyph, class, status, mut problem) = match facts.state {
        RunState::Idle => (
            GLYPH_IDLE,
            StatusClass::Idle,
            "Not run".to_owned(),
            String::new(),
        ),
        RunState::Running(details) => {
            let verb = if details.stop_requested() {
                "Stopping"
            } else {
                "Running"
            };
            let status = match &took {
                Some(took) => format!("{verb} {took}"),
                None => verb.to_owned(),
            };
            (
                SPINNER[facts.tick % SPINNER.len()],
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
    };
    if let Some(pid) = facts.overdue_pid {
        problem = format!(
            "Stop has not finished: process {pid} is still running. Press Stop again or end it in Task Manager."
        );
    }
    RowText {
        detail: detail(facts.script),
        glyph,
        status,
        class,
        problem,
        can_run: !running,
        can_stop: running,
        can_remove: !running,
    }
}

/// `label` followed by "after m:ss" when the duration is known.
fn after(label: &str, took: &Option<String>) -> String {
    match took {
        Some(took) => format!("{label} after {took}"),
        None => label.to_owned(),
    }
}

/// `build.ps1 in C:\src\app`.
pub fn detail(script: &Path) -> String {
    let file = script
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
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

/// A notice in words.
pub fn notice_text(notice: &Notice) -> String {
    match notice {
        Notice::Load(LoadProblem::UnreadableEntries(count)) => format!(
            "{count} saved operation(s) could not be read. They are kept in the settings file, untouched."
        ),
        Notice::Load(LoadProblem::SetAside(path)) => format!(
            "The settings file could not be read, so BuildPilot started empty. The old file was kept as {}.",
            path.display()
        ),
        Notice::Load(LoadProblem::NewerSchema) => {
            "The settings file was written by a newer BuildPilot. \
            It is shown here but changes will not be saved."
                .to_owned()
        }
        Notice::Load(LoadProblem::Unreadable { path, message }) => format!(
            "The settings file {} could not be read ({message}). Changes will not be saved this session.",
            path.display()
        ),
        Notice::DuplicateDropped(id) => {
            format!("Two saved operations shared the identity {id}; the second was left out.")
        }
        Notice::SaveFailed(error) => error.to_string(),
        Notice::StopFailed { name, message } => format!("Could not stop {name}: {message}"),
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
