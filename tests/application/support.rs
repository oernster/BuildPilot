//! Shorthands shared by the application tests, written once.

use buildpilot::application::{App, RunEvent, RunEventKind, RunKey};
use buildpilot::domain::operation::{OperationId, OperationSpec};
use buildpilot::domain::output::Stream;

use super::fakes::World;

pub const SCRIPT_A: &str = r"C:\src\alpha\build.ps1";
pub const SCRIPT_B: &str = r"C:\src\beta\build.cmd";
pub const SCRIPT_C: &str = r"C:\src\gamma\make.exe";

/// A world where the three scripts exist.
pub fn world() -> World {
    World::new()
        .with_script(SCRIPT_A)
        .with_script(SCRIPT_B)
        .with_script(SCRIPT_C)
}

/// The Add dialog's defaults for `script`, unchanged.
pub fn draft(app: &App, script: &str) -> OperationSpec {
    app.draft_for(std::path::Path::new(script)).unwrap()
}

/// Adds `script` with its defaults.
pub fn add(app: &mut App, script: &str) -> OperationId {
    let spec = draft(app, script);
    app.add(spec).unwrap()
}

pub fn line(key: &RunKey, stream: Stream, text: &str) -> RunEvent {
    RunEvent {
        key: key.clone(),
        kind: RunEventKind::Line {
            stream,
            text: text.to_owned(),
        },
    }
}

pub fn exited(key: &RunKey, code: i32) -> RunEvent {
    RunEvent {
        key: key.clone(),
        kind: RunEventKind::Exited { code },
    }
}

/// The texts of operation `id`'s latest output.
pub fn output_texts(app: &App, id: &OperationId) -> Vec<String> {
    app.output(id)
        .map(|buffer| buffer.lines().map(|line| line.text.clone()).collect())
        .unwrap_or_default()
}
