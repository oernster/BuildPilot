use std::fs;
use std::time::Duration;

use buildpilot::application::ports::RunTimesStore;
use buildpilot::domain::operation::OperationId;
use buildpilot::domain::run_times::RunTimes;
use buildpilot::infrastructure::run_times_store::{JsonRunTimesStore, RUN_TIMES_FILE};

// LIFE-008: no file yet means no times, not a problem.
#[test]
fn missing_file_is_empty() {
    let folder = tempfile::tempdir().unwrap();
    let loaded = JsonRunTimesStore::new(folder.path().join("BuildPilot")).load();
    assert_eq!(loaded, Ok(RunTimes::default()));
}

// LIFE-008: what is saved is what the next session loads, to the millisecond.
#[test]
fn times_round_trip() {
    let folder = tempfile::tempdir().unwrap();
    let data = folder.path().join("BuildPilot");
    let times = RunTimes::from_recent([(
        OperationId::new("a").unwrap(),
        vec![Duration::from_millis(1_500), Duration::from_secs(200)],
    )]);
    JsonRunTimesStore::new(data.clone()).save(&times).unwrap();
    assert_eq!(JsonRunTimesStore::new(data).load(), Ok(times));
}

// LIFE-008: a damaged file is reported with its path; a blank identity is passed over.
#[test]
fn bad_files_are_reported_or_skipped() {
    let folder = tempfile::tempdir().unwrap();
    let file = folder.path().join(RUN_TIMES_FILE);
    fs::write(&file, "not json").unwrap();
    let mut store = JsonRunTimesStore::new(folder.path().to_path_buf());
    let error = store.load().unwrap_err();
    assert!(error.contains(RUN_TIMES_FILE), "{error}");

    fs::write(&file, r#"{"schema":1,"operations":{" ":[5],"b":[7000]}}"#).unwrap();
    let loaded = store.load().unwrap();
    let ids: Vec<_> = loaded
        .recent()
        .map(|(id, _)| id.as_str().to_owned())
        .collect();
    assert_eq!(ids, vec!["b".to_owned()]);
}

// LIFE-008: a file that exists but cannot be read is an error, never a silent empty.
#[test]
fn unreadable_file_is_an_error() {
    let folder = tempfile::tempdir().unwrap();
    fs::create_dir(folder.path().join(RUN_TIMES_FILE)).unwrap();
    assert!(
        JsonRunTimesStore::new(folder.path().to_path_buf())
            .load()
            .is_err()
    );
}

// LIFE-008: a folder that cannot be created is a save error naming the file.
#[test]
fn unwritable_folder_is_a_save_error() {
    let folder = tempfile::tempdir().unwrap();
    let blocker = folder.path().join("BuildPilot");
    fs::write(&blocker, "a file where the folder should be").unwrap();
    let error = JsonRunTimesStore::new(blocker.clone())
        .save(&RunTimes::default())
        .unwrap_err();
    assert_eq!(error.path, blocker.join(RUN_TIMES_FILE));
}
