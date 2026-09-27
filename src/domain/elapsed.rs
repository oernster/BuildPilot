//! How a run's elapsed time or duration is written (LIFE-005, LIFE-007). The time itself is
//! measured elsewhere and handed in; this module never reads a clock.

use std::time::Duration;

const SECONDS_PER_MINUTE: u64 = 60;

/// `duration` as `m:ss`, whole seconds, rounded down. Minutes are not capped, so a run of an
/// hour and a quarter reads `75:00`.
pub fn format_elapsed(duration: Duration) -> String {
    let seconds = duration.as_secs();
    format!(
        "{}:{:02}",
        seconds / SECONDS_PER_MINUTE,
        seconds % SECONDS_PER_MINUTE
    )
}
