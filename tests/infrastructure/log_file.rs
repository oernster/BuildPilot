use std::fs;

use buildpilot::application::ports::Log;
use buildpilot::infrastructure::log_file::{LOG_FILE_NAME, LogFile, PREVIOUS_LOG_FILE_NAME};

/// A rotation size small enough to cross in a few entries.
const SMALL_ROTATION: u64 = 200;
/// Longer than a third of `SMALL_ROTATION` once timestamped, so three entries never fit.
const ENTRY_TEXT_LENGTH: usize = 60;

fn entry(number: usize) -> String {
    format!("entry {number:0>width$}", width = ENTRY_TEXT_LENGTH)
}

// NFR-OBS-001: each entry is one timestamped line in the data folder's log.
#[test]
fn entries_are_timestamped_lines() {
    let folder = tempfile::tempdir().unwrap();
    let log = LogFile::open(folder.path(), SMALL_ROTATION * 100);
    log.write("first");
    log.record("second");

    assert_eq!(log.path(), folder.path().join(LOG_FILE_NAME));
    let text = fs::read_to_string(log.path()).unwrap();
    let lines: Vec<&str> = text.split_terminator("\r\n").collect();
    assert_eq!(lines.len(), 2, "{text:?}");
    assert!(lines[0].ends_with(" first"), "{text:?}");
    assert!(lines[1].ends_with(" second"), "{text:?}");
    // yyyy-mm-dd hh:mm:ss.mmm
    let stamp = &lines[0][..lines[0].len() - " first".len()];
    assert_eq!(stamp.len(), "2026-01-01 00:00:00.000".len(), "{stamp}");
}

// NFR-OBS-001: the log rotates at its size, keeping one previous file.
#[test]
fn the_log_rotates_keeping_one_previous_file() {
    let folder = tempfile::tempdir().unwrap();
    let log = LogFile::open(folder.path(), SMALL_ROTATION);
    let previous = folder.path().join(PREVIOUS_LOG_FILE_NAME);
    for number in 0..6 {
        log.write(&entry(number));
    }

    let current = fs::read_to_string(log.path()).unwrap();
    let kept = fs::read_to_string(&previous).unwrap();
    // Two entries fit, so the third rotates: 0 and 1 went first, then 2 and 3 were kept.
    assert!(current.contains(&entry(4)) && current.contains(&entry(5)));
    assert!(!current.contains(&entry(3)), "{current:?}");
    assert!(kept.contains(&entry(2)) && kept.contains(&entry(3)));
    assert!(
        !kept.contains(&entry(1)),
        "only one previous file: {kept:?}"
    );
    assert!(fs::metadata(log.path()).unwrap().len() <= SMALL_ROTATION);
}

// NFR-OBS-001: a log already over its size rotates on the next entry.
#[test]
fn an_oversized_log_rotates_on_its_next_entry() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join(LOG_FILE_NAME);
    fs::write(&path, "x".repeat(SMALL_ROTATION as usize)).unwrap();
    let log = LogFile::open(folder.path(), SMALL_ROTATION);
    log.write("fresh");

    let current = fs::read_to_string(&path).unwrap();
    assert!(current.ends_with(" fresh\r\n") && !current.contains('x'));
    assert!(folder.path().join(PREVIOUS_LOG_FILE_NAME).exists());
}

// NFR-OBS-001: a log that cannot be written is not kept; nothing fails.
#[test]
fn an_unwritable_folder_keeps_no_log() {
    let root = tempfile::tempdir().unwrap();
    let not_a_folder = root.path().join("file");
    fs::write(&not_a_folder, b"").unwrap();
    let log = LogFile::open(&not_a_folder, SMALL_ROTATION);
    log.write("goes nowhere");
    assert!(!log.path().exists());
}
