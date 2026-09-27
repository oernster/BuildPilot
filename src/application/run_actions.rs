//! Running and stopping operations; taking in what their processes do (SRS 3.7 to 3.10).

use std::time::Duration;

use crate::domain::deck::DeckError;
use crate::domain::launch_plan::{LaunchPlan, plan};
use crate::domain::lifecycle::{LaunchError, RunState, TransitionError};
use crate::domain::operation::OperationId;
use crate::domain::output::OutputBuffer;

use super::ports::{RunEvent, RunEventKind, RunKey};
use super::{App, AppError, Notice};

/// How long a stopped process tree may take to die before the row says Stop failed (STOP-003,
/// OQ-4).
pub const STOP_TIMEOUT: Duration = Duration::from_secs(5);

/// The state of an operation that has never run.
static IDLE: RunState = RunState::Idle;

/// An operation whose processes outlived Stop by more than `STOP_TIMEOUT` (STOP-003).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverdueStop {
    /// The operation.
    pub id: OperationId,
    /// The process identifier still alive.
    pub pid: u32,
}

impl App {
    /// Starts operation `id` (LCH-002). Answers at once; the process runs on. A missing script
    /// or folder is not an error here, nor is a launch the operating system refuses: the row
    /// shows Failed with the reason (LCH-007 to LCH-009). Refused while the operation is
    /// already running (LCH-005).
    pub fn run(&mut self, id: &OperationId) -> Result<(), AppError> {
        let config = self
            .deck
            .get(id)
            .ok_or_else(|| DeckError::NotFound(id.clone()))?
            .config()
            .clone();
        self.run_state(id).launched()?;

        self.runs_started += 1;
        let run = self.runs_started;
        let key = RunKey {
            operation: id.clone(),
            run,
        };
        let outcome = if !self.ports.paths.is_file(config.script_path()) {
            Err(LaunchError::ScriptNotFound(
                config.script_path().to_path_buf(),
            ))
        } else if !self.ports.paths.is_dir(config.working_dir()) {
            Err(LaunchError::WorkingDirNotFound(
                config.working_dir().to_path_buf(),
            ))
        } else {
            let launch = plan(&config, self.powershell);
            self.ports
                .launcher
                .spawn(key, &launch)
                .map_err(|message| LaunchError::Os {
                    command: display_command(&launch),
                    message,
                })
        };

        let now = self.ports.clock.now();
        let runtime = self.runtimes.entry(id.clone()).or_default();
        runtime.begin_attempt(run);
        runtime.state = match outcome {
            Ok(process) => {
                runtime.process = Some(process);
                runtime.started_at = Some(now);
                runtime.state.launched()?
            }
            Err(error) => runtime.state.launch_failed(error)?,
        };
        Ok(())
    }

    /// Asks for operation `id`'s process tree to be terminated (STOP-001). Answers at once; the
    /// row shows Stopped when the exit arrives. Asking again is allowed (STOP-003).
    pub fn stop(&mut self, id: &OperationId) -> Result<(), AppError> {
        let now = self.ports.clock.now();
        let name = self.name_of(id);
        let Some(runtime) = self.runtimes.get_mut(id) else {
            return Err(TransitionError::NotRunning.into());
        };
        runtime.state = runtime.state.stop_requested()?;
        runtime.stop_requested_at.get_or_insert(now);
        if let Some(process) = runtime.process.as_mut()
            && let Err(message) = process.stop()
        {
            self.notices.push(Notice::StopFailed { name, message });
        }
        Ok(())
    }

    /// Stops every running operation, for closing BuildPilot (STOP-004).
    pub fn stop_all(&mut self) {
        for id in self.running_ids() {
            // Each id was running a moment ago, so stop cannot be refused.
            let _ = self.stop(&id);
        }
    }

    /// Takes in one event from a running process. Events for a run that is not the operation's
    /// latest are ignored (OUT-002).
    pub fn handle_event(&mut self, event: RunEvent) {
        let now = self.ports.clock.now();
        let Some(runtime) = self.runtimes.get_mut(&event.key.operation) else {
            return;
        };
        if runtime.run != Some(event.key.run) {
            return;
        }
        match event.kind {
            RunEventKind::Line { stream, text } => runtime.output.push(stream, &text),
            RunEventKind::Exited { code } => {
                if let Ok(next) = runtime.state.exited(code) {
                    runtime.state = next;
                    runtime.finished_at = Some(now);
                    runtime.process = None;
                }
            }
        }
    }

    /// Operation `id`'s run state; Idle when it has not run (LIFE-001).
    pub fn run_state(&self, id: &OperationId) -> &RunState {
        self.runtimes
            .get(id)
            .map_or(&IDLE, |runtime| &runtime.state)
    }

    /// True while operation `id` has a process running.
    pub fn is_running(&self, id: &OperationId) -> bool {
        self.run_state(id).is_running()
    }

    /// The names of the running operations in deck order, for the close confirmation
    /// (STOP-004).
    pub fn running_names(&self) -> Vec<String> {
        self.deck
            .operations()
            .iter()
            .filter(|operation| self.is_running(operation.id()))
            .map(|operation| operation.config().name().to_owned())
            .collect()
    }

    /// How long the latest run has run: up to now while running, start to exit once finished;
    /// `None` when no process started (LIFE-005, LIFE-007).
    pub fn elapsed(&self, id: &OperationId) -> Option<Duration> {
        let runtime = self.runtimes.get(id)?;
        let started = runtime.started_at?;
        let end = runtime
            .finished_at
            .unwrap_or_else(|| self.ports.clock.now());
        Some(end.saturating_duration_since(started))
    }

    /// The latest run's output (OUT-003); `None` when the operation has not run.
    pub fn output(&self, id: &OperationId) -> Option<&OutputBuffer> {
        self.runtimes.get(id).map(|runtime| &runtime.output)
    }

    /// Operations still running more than `STOP_TIMEOUT` after Stop (STOP-003).
    pub fn overdue_stops(&self) -> Vec<OverdueStop> {
        let now = self.ports.clock.now();
        let mut overdue: Vec<OverdueStop> = self
            .runtimes
            .iter()
            .filter(|(_, runtime)| runtime.state.is_running())
            .filter_map(|(id, runtime)| {
                let asked = runtime.stop_requested_at?;
                let process = runtime.process.as_ref()?;
                (now.saturating_duration_since(asked) >= STOP_TIMEOUT).then(|| OverdueStop {
                    id: id.clone(),
                    pid: process.pid(),
                })
            })
            .collect();
        overdue.sort_by(|a, b| a.id.cmp(&b.id));
        overdue
    }

    fn running_ids(&self) -> Vec<OperationId> {
        self.runtimes
            .iter()
            .filter(|(_, runtime)| runtime.state.is_running())
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Operation `id`'s display name; its identity when it is not on the deck.
    pub(super) fn name_of(&self, id: &OperationId) -> String {
        self.deck
            .get(id)
            .map(|operation| operation.config().name().to_owned())
            .unwrap_or_else(|| id.to_string())
    }
}

/// `launch` as the operator would type it, for a launch failure message (LCH-009).
fn display_command(launch: &LaunchPlan) -> String {
    std::iter::once(launch.program.to_string_lossy().into_owned())
        .chain(launch.arguments.iter().cloned())
        .map(|part| {
            if part.is_empty() || part.contains(char::is_whitespace) {
                format!("\"{part}\"")
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
