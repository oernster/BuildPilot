//! Recent successful run times on disk (LIFE-008): `run-times.json` in the data folder, kept
//! apart from the config file, which holds no run data (CFG-004).

use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::application::ports::{RunTimesStore, StoreError};
use crate::domain::operation::OperationId;
use crate::domain::run_times::RunTimes;

use super::config_store::write_atomically;

/// The run times file's name inside the data folder.
pub const RUN_TIMES_FILE: &str = "run-times.json";
/// The shape of the file this build writes.
const CURRENT_SCHEMA: u32 = 1;

/// The file as written: each operation's identity against its recent durations in
/// milliseconds, oldest first.
#[derive(Serialize, Deserialize)]
struct Document {
    schema: u32,
    operations: BTreeMap<String, Vec<u64>>,
}

/// Reads and writes `run-times.json`.
pub struct JsonRunTimesStore {
    folder: PathBuf,
}

impl JsonRunTimesStore {
    /// A store for the data folder `folder`, which need not exist yet.
    pub fn new(folder: PathBuf) -> Self {
        Self { folder }
    }

    fn file(&self) -> PathBuf {
        self.folder.join(RUN_TIMES_FILE)
    }
}

impl RunTimesStore for JsonRunTimesStore {
    fn load(&mut self) -> Result<RunTimes, String> {
        let file = self.file();
        let unreadable = |message: String| format!("{}: {message}", file.display());
        let text = match fs::read_to_string(&file) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(RunTimes::default()),
            Err(error) => return Err(unreadable(error.to_string())),
        };
        let document: Document =
            serde_json::from_str(&text).map_err(|error| unreadable(error.to_string()))?;
        // A blank identity names no operation, so its entry is passed over.
        Ok(RunTimes::from_recent(
            document
                .operations
                .into_iter()
                .filter_map(|(id, milliseconds)| {
                    let durations = milliseconds.into_iter().map(Duration::from_millis);
                    Some((OperationId::new(id).ok()?, durations.collect()))
                }),
        ))
    }

    fn save(&mut self, times: &RunTimes) -> Result<(), StoreError> {
        let file = self.file();
        let document = Document {
            schema: CURRENT_SCHEMA,
            operations: times
                .recent()
                .map(|(id, durations)| {
                    let milliseconds = durations
                        .iter()
                        .map(|took| u64::try_from(took.as_millis()).unwrap_or(u64::MAX))
                        .collect();
                    (id.as_str().to_owned(), milliseconds)
                })
                .collect(),
        };
        let failed = |message: String| StoreError {
            path: file.clone(),
            message,
        };
        let text =
            serde_json::to_string_pretty(&document).map_err(|error| failed(error.to_string()))?;
        write_atomically(&self.folder, &file, &text).map_err(|error| failed(error.to_string()))
    }
}
