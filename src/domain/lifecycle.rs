//! The run state machine (SRS 3.8, 3.9). Every change of state goes through one of the
//! transition methods here, which refuse the transitions the SRS does not allow.

use std::error::Error;
use std::fmt;
use std::path::PathBuf;

/// The state of an operation's latest run (LIFE-001).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RunState {
    /// Not run since BuildPilot started.
    #[default]
    Idle,
    /// A process is running.
    Running(Running),
    /// The process exited with code 0.
    Succeeded,
    /// The run did not succeed.
    Failed(Failure),
    /// The operator stopped the run.
    Stopped,
}

/// Details of a run in progress.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Running {
    stop_requested: bool,
    progress: Progress,
}

impl Running {
    /// True once the operator has asked for this run to stop.
    pub fn stop_requested(&self) -> bool {
        self.stop_requested
    }
    /// How far through the run is known to be.
    pub fn progress(&self) -> Progress {
        self.progress
    }
}

/// Why a run failed (LIFE-001).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// No process was started.
    FailedToStart(LaunchError),
    /// The process exited with this non-zero code.
    ExitCode(i32),
}

/// Why no process was started (LCH-007 to LCH-009).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchError {
    /// The script file does not exist.
    ScriptNotFound(PathBuf),
    /// The working directory does not exist.
    WorkingDirNotFound(PathBuf),
    /// The operating system refused to start the program.
    Os {
        /// The command that was attempted, as the operator would type it.
        command: String,
        /// The operating system's message.
        message: String,
    },
}

impl fmt::Display for LaunchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ScriptNotFound(path) => write!(
                f,
                "Script not found: {}. Use Edit to point at its new location.",
                path.display()
            ),
            Self::WorkingDirNotFound(path) => write!(
                f,
                "Working directory not found: {}. Use Edit to choose another.",
                path.display()
            ),
            Self::Os { command, message } => {
                write!(f, "Could not start {command}: {message}")
            }
        }
    }
}

impl Error for LaunchError {}

/// A whole percentage from 0 to 100.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Percent(u8);

impl Percent {
    /// The largest value.
    pub const MAX: u8 = 100;

    /// `value` as a percentage; `None` above 100.
    pub fn new(value: u8) -> Option<Self> {
        (value <= Self::MAX).then_some(Self(value))
    }
    /// The value.
    pub fn value(self) -> u8 {
        self.0
    }
}

/// How much is known about a run's progress (LIFE-006). v1 only ever produces `Indeterminate`:
/// a percentage exists only when real progress information backs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Progress {
    /// Running, with no trustworthy measure of how far.
    #[default]
    Indeterminate,
    /// Running, with a measured percentage.
    Determinate(Percent),
}

/// A transition the state machine refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionError {
    /// The operation is already running (LCH-005).
    AlreadyRunning,
    /// The operation is not running, so it cannot be stopped or exit.
    NotRunning,
}

impl fmt::Display for TransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::AlreadyRunning => "The operation is already running.",
            Self::NotRunning => "The operation is not running.",
        })
    }
}

impl Error for TransitionError {}

impl RunState {
    /// True while a process is running.
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running(_))
    }

    /// A process was started (LCH-002). Refused while one is already running (LCH-005).
    pub fn launched(&self) -> Result<Self, TransitionError> {
        self.refuse_if_running()?;
        Ok(Self::Running(Running::default()))
    }

    /// No process could be started (LCH-007 to LCH-009).
    pub fn launch_failed(&self, error: LaunchError) -> Result<Self, TransitionError> {
        self.refuse_if_running()?;
        Ok(Self::Failed(Failure::FailedToStart(error)))
    }

    /// The operator asked for the run to stop. Asking again is allowed and changes nothing, so
    /// Stop stays usable after a stop that did not finish (STOP-003).
    pub fn stop_requested(&self) -> Result<Self, TransitionError> {
        match self {
            Self::Running(running) => Ok(Self::Running(Running {
                stop_requested: true,
                ..running.clone()
            })),
            _ => Err(TransitionError::NotRunning),
        }
    }

    /// The process exited with `code`. A stopped run is Stopped whatever the code
    /// (LIFE-003); otherwise 0 is success and anything else failure (LIFE-002).
    pub fn exited(&self, code: i32) -> Result<Self, TransitionError> {
        match self {
            Self::Running(running) if running.stop_requested => Ok(Self::Stopped),
            Self::Running(_) if code == 0 => Ok(Self::Succeeded),
            Self::Running(_) => Ok(Self::Failed(Failure::ExitCode(code))),
            _ => Err(TransitionError::NotRunning),
        }
    }

    fn refuse_if_running(&self) -> Result<(), TransitionError> {
        if self.is_running() {
            return Err(TransitionError::AlreadyRunning);
        }
        Ok(())
    }
}
