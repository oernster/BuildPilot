//! One operation's transient run state (SRS spec §21 "OperationRuntime"). Never persisted
//! (CFG-004).

use std::time::Instant;

use crate::domain::lifecycle::RunState;
use crate::domain::output::OutputBuffer;

use super::ports::ProcessHandle;

/// The latest run of one operation.
#[derive(Default)]
pub(super) struct OperationRuntime {
    /// Where the run is in its lifecycle.
    pub state: RunState,
    /// The number of the latest run; events for any other number are stale.
    pub run: Option<u64>,
    /// The live process tree, while one is running.
    pub process: Option<Box<dyn ProcessHandle>>,
    /// The latest run's output (OUT-003).
    pub output: OutputBuffer,
    /// When the process started; `None` if it never did.
    pub started_at: Option<Instant>,
    /// When the process exited.
    pub finished_at: Option<Instant>,
    /// When Stop was first asked for (STOP-003).
    pub stop_requested_at: Option<Instant>,
}

impl OperationRuntime {
    /// Clears the previous run's output and times for a new attempt numbered `run`.
    pub fn begin_attempt(&mut self, run: u64) {
        self.run = Some(run);
        self.process = None;
        self.output = OutputBuffer::new();
        self.started_at = None;
        self.finished_at = None;
        self.stop_requested_at = None;
    }
}
