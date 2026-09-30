//! Running and stopping operations; taking in what their processes do (SRS 3.7 to 3.10).

use std::time::Duration;

use crate::domain::deck::DeckError;
use crate::domain::lifecycle::{Failure, RunState, TransitionError};
use crate::domain::operation::{OperationConfig, OperationId};
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
        // LCH-005, settled once: the run's state from here on is this transition or a failure to
        // start, both allowed exactly when this one is.
        let started = self.run_state(id).launched(config.steps().len())?;
        if let Some(by) = self.folder_busy(id, &config) {
            return Err(AppError::FolderInUse {
                folder: config.working_dir().to_path_buf(),
                by,
            });
        }

        self.runs_started += 1;
        let run = self.runs_started;
        let key = RunKey {
            operation: id.clone(),
            run,
        };
        self.runtimes
            .entry(id.clone())
            .or_default()
            .begin_attempt(run, config.clone());
        let outcome = match self.prepare(&config) {
            Ok(environment) => {
                let started = self.start_step(&key, &config, 0, environment.as_deref());
                self.runtimes.entry(id.clone()).or_default().environment = environment;
                started
            }
            Err(error) => {
                let name = config.name();
                self.ports
                    .log
                    .record(&format!("{name} could not start: {error}"));
                Err(error)
            }
        };

        let now = self.ports.clock.now();
        let runtime = self.runtimes.entry(id.clone()).or_default();
        runtime.state = match outcome {
            Ok(process) => {
                runtime.process = Some(process);
                runtime.started_at = Some(now);
                started
            }
            Err(error) => RunState::Failed(Failure::FailedToStart(error)),
        };
        Ok(())
    }

    /// Runs every ticked operation, top row first, each as `run` would (ROW-006, LCH-011). One
    /// that `run` refuses is skipped and answered with its name, so the rest still start.
    pub fn run_checked(&mut self) -> Vec<(String, AppError)> {
        let ticked: Vec<OperationId> = self
            .deck
            .operations()
            .iter()
            .map(|operation| operation.id().clone())
            .filter(|id| self.selection.is_checked(id))
            .collect();
        ticked
            .iter()
            .filter_map(|id| self.run(id).err().map(|error| (self.name_of(id), error)))
            .collect()
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
        let refused = runtime
            .process
            .as_mut()
            .and_then(|process| process.stop().err());
        self.ports.log.record(&format!("Stop asked for {name}"));
        if let Some(message) = refused {
            self.raise(Notice::StopFailed { name, message });
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
        let code = match event.kind {
            RunEventKind::Line { stream, text } => {
                runtime.output.push(stream, &text);
                return;
            }
            RunEventKind::Exited { code } => code,
        };
        let Ok(next) = runtime.state.exited(code) else {
            return;
        };
        runtime.process = None;
        if let RunState::Running(running) = &next {
            // STEP-002: the step before succeeded, so the next one starts in the same run.
            let index = running.step() - 1;
            runtime.state = next.clone();
            let config = runtime
                .config
                .clone()
                .expect("a run in progress holds the configuration it started with");
            let environment = runtime.environment.clone();
            let started = self.start_step(&event.key, &config, index, environment.as_deref());
            let runtime = self.runtimes.entry(event.key.operation).or_default();
            match started {
                Ok(process) => runtime.process = Some(process),
                Err(error) => {
                    runtime.state = RunState::Failed(Failure::FailedToStart(error));
                    runtime.finished_at = Some(now);
                }
            }
            return;
        }
        let stopped = next == RunState::Stopped;
        // LIFE-008: only a success says how long the build takes.
        let succeeded_in = (next == RunState::Succeeded)
            .then_some(runtime.started_at)
            .flatten()
            .map(|started| now.saturating_duration_since(started));
        runtime.state = next;
        runtime.finished_at = Some(now);
        if let Some(took) = succeeded_in {
            self.run_times.record(&event.key.operation, took);
            self.persist_run_times();
        }
        // The run may have written the installer (PKG-002).
        self.record_installer(&event.key.operation);
        let name = self.name_of(&event.key.operation);
        let line = if stopped {
            format!("{name} stopped (exit code {code})")
        } else {
            format!("{name} exited with code {code}")
        };
        self.ports.log.record(&line);
    }

    /// Operation `id`'s run state; Idle when it has not run (LIFE-001).
    pub fn run_state(&self, id: &OperationId) -> &RunState {
        self.runtimes
            .get(id)
            .map_or(&IDLE, |runtime| &runtime.state)
    }

    /// The name of another operation running in the working directory of operation `id`,
    /// configured as `wanted`, which holds its Run back (LCH-010); `None` when the folder is
    /// free. A running operation is judged by the configuration its run started with, not by an
    /// edit made since.
    pub fn folder_busy(&self, id: &OperationId, wanted: &OperationConfig) -> Option<String> {
        self.runtimes
            .iter()
            .filter(|(other, runtime)| *other != id && runtime.state.is_running())
            .find(|(_, runtime)| {
                runtime
                    .config
                    .as_ref()
                    .is_some_and(|running| running.shares_folder_with(wanted))
            })
            .map(|(other, _)| self.name_of(other))
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

    /// How long operation `id`'s build typically takes: the median of its recent successful
    /// runs, remembered across sessions; `None` before any has succeeded (LIFE-008).
    pub fn typical_duration(&self, id: &OperationId) -> Option<Duration> {
        self.run_times.typical(id)
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
            // A running run always holds its process; only the Stop may be missing.
            .filter_map(
                |(id, runtime)| match (runtime.stop_requested_at, &runtime.process) {
                    (Some(asked), Some(process)) => (now.saturating_duration_since(asked)
                        >= STOP_TIMEOUT)
                        .then(|| OverdueStop {
                            id: id.clone(),
                            pid: process.pid(),
                        }),
                    _ => None,
                },
            )
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
