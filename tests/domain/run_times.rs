use std::time::Duration;

use buildpilot::domain::operation::OperationId;
use buildpilot::domain::run_times::{RECENT_RUNS_KEPT, RunTimes};

fn id(value: &str) -> OperationId {
    OperationId::new(value).unwrap()
}

fn seconds(values: &[u64]) -> Vec<Duration> {
    values.iter().copied().map(Duration::from_secs).collect()
}

// LIFE-008: the typical time is the median, so one outlier does not move it.
#[test]
fn typical_is_the_median_of_recent_successes() {
    let a = id("a");
    let mut times = RunTimes::default();
    assert_eq!(times.typical(&a), None);
    times.record(&a, Duration::from_secs(100));
    assert_eq!(times.typical(&a), Some(Duration::from_secs(100)));
    times.record(&a, Duration::from_secs(900));
    // Even count: the lower middle, a time that was actually seen.
    assert_eq!(times.typical(&a), Some(Duration::from_secs(100)));
    times.record(&a, Duration::from_secs(110));
    assert_eq!(times.typical(&a), Some(Duration::from_secs(110)));
}

// LIFE-008: only the newest RECENT_RUNS_KEPT count, so the typical time follows the build.
#[test]
fn oldest_runs_fall_away() {
    let a = id("a");
    let mut times = RunTimes::default();
    for took in seconds(&[1, 2, 3, 4, 5, 6, 7]) {
        times.record(&a, took);
    }
    let kept: Vec<_> = times.recent().map(|(_, d)| d.to_vec()).collect();
    assert_eq!(kept, vec![seconds(&[3, 4, 5, 6, 7])]);
    assert_eq!(kept[0].len(), RECENT_RUNS_KEPT);
}

// LIFE-008: stored lists are trimmed to the newest and empty ones are dropped.
#[test]
fn from_recent_trims_and_drops_empty() {
    let times = RunTimes::from_recent([
        (id("a"), seconds(&[1, 2, 3, 4, 5, 6])),
        (id("b"), Vec::new()),
    ]);
    let recent: Vec<_> = times
        .recent()
        .map(|(id, d)| (id.as_str().to_owned(), d.to_vec()))
        .collect();
    assert_eq!(recent, vec![("a".to_owned(), seconds(&[2, 3, 4, 5, 6]))]);
}

// LIFE-008, REM-001: a removed operation's times go with it; retain keeps only the named.
#[test]
fn forget_and_retain() {
    let mut times = RunTimes::default();
    times.record(&id("a"), Duration::from_secs(1));
    times.record(&id("b"), Duration::from_secs(2));
    times.record(&id("c"), Duration::from_secs(3));
    times.forget(&id("a"));
    times.retain(|id| id.as_str() != "c");
    let ids: Vec<_> = times
        .recent()
        .map(|(id, _)| id.as_str().to_owned())
        .collect();
    assert_eq!(ids, vec!["b".to_owned()]);
}
