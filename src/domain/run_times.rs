//! How long each operation's builds usually take (LIFE-008): the durations of its most recent
//! successful runs and the typical one among them. The durations are handed in; this module
//! never reads a clock.

use std::collections::BTreeMap;
use std::time::Duration;

use super::operation::OperationId;

/// How many recent successful runs an operation keeps; older ones fall away, so the typical time
/// follows a build that has grown quicker or slower.
pub const RECENT_RUNS_KEPT: usize = 5;

/// Every operation's recent successful run durations, oldest first.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RunTimes {
    recent: BTreeMap<OperationId, Vec<Duration>>,
}

impl RunTimes {
    /// Run times from stored lists, oldest first. A list longer than `RECENT_RUNS_KEPT` keeps
    /// only its newest; an empty one is dropped.
    pub fn from_recent(recent: impl IntoIterator<Item = (OperationId, Vec<Duration>)>) -> Self {
        let recent = recent
            .into_iter()
            .filter(|(_, durations)| !durations.is_empty())
            .map(|(id, durations)| {
                let surplus = durations.len().saturating_sub(RECENT_RUNS_KEPT);
                (id, durations[surplus..].to_vec())
            })
            .collect();
        Self { recent }
    }

    /// Adds `took`, the duration of a successful run of operation `id`, dropping its oldest
    /// once more than `RECENT_RUNS_KEPT` are held.
    pub fn record(&mut self, id: &OperationId, took: Duration) {
        let durations = self.recent.entry(id.clone()).or_default();
        durations.push(took);
        if durations.len() > RECENT_RUNS_KEPT {
            durations.remove(0);
        }
    }

    /// Forgets operation `id`, as when it is removed from BuildPilot.
    pub fn forget(&mut self, id: &OperationId) {
        self.recent.remove(id);
    }

    /// Keeps only the operations `keep` answers true for.
    pub fn retain(&mut self, keep: impl Fn(&OperationId) -> bool) {
        self.recent.retain(|id, _| keep(id));
    }

    /// The median of operation `id`'s recent successful runs, so one unusually slow or quick
    /// build does not move it; `None` before any has succeeded. With an even count it is the
    /// lower of the middle two, a duration that was actually seen.
    pub fn typical(&self, id: &OperationId) -> Option<Duration> {
        let mut durations = self.recent.get(id)?.clone();
        durations.sort();
        durations
            .get(durations.len().saturating_sub(1) / 2)
            .copied()
    }

    /// Every operation's recent durations, oldest first, in identity order.
    pub fn recent(&self) -> impl Iterator<Item = (&OperationId, &[Duration])> {
        self.recent
            .iter()
            .map(|(id, durations)| (id, durations.as_slice()))
    }
}
