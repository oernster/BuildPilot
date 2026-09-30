//! The config file on disk (SRS 3.1): `buildpilot.json` in the data folder.

use std::ffi::OsString;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::application::ports::{ConfigStore, LoadProblem, LoadedConfig, StoreError};
use crate::domain::operation::Operation;
use crate::domain::preferences::Preferences;

use super::config_format::{CURRENT_SCHEMA, parse, render};

/// The config file's name inside the data folder.
pub const CONFIG_FILE: &str = "buildpilot.json";
/// Appended to the name of a config file that could not be read (CFG-006).
pub const SET_ASIDE_SUFFIX: &str = ".unreadable";
/// Appended to the name of the file written before it replaces the config file (CFG-002).
const TEMPORARY_SUFFIX: &str = ".tmp";

/// Reads and writes `buildpilot.json`.
pub struct JsonConfigStore {
    folder: PathBuf,
    unreadable: Vec<Value>,
    /// Why saving is refused this session, when it is.
    frozen: Option<String>,
}

impl JsonConfigStore {
    /// A store for the data folder `folder`, which need not exist yet.
    pub fn new(folder: PathBuf) -> Self {
        Self {
            folder,
            unreadable: Vec::new(),
            frozen: None,
        }
    }

    fn file(&self) -> PathBuf {
        self.folder.join(CONFIG_FILE)
    }

    /// Moves an unreadable config file aside so the next save does not destroy it.
    fn set_aside(&mut self, file: &Path, loaded: &mut LoadedConfig) {
        let aside = with_suffix(file, SET_ASIDE_SUFFIX);
        match fs::rename(file, &aside) {
            Ok(()) => loaded.problems.push(LoadProblem::SetAside(aside)),
            Err(error) => self.freeze(file, error.to_string(), loaded),
        }
    }

    fn freeze(&mut self, file: &Path, message: String, loaded: &mut LoadedConfig) {
        self.frozen = Some(format!(
            "{} could not be read ({message}), so it is left untouched",
            file.display()
        ));
        loaded.problems.push(LoadProblem::Unreadable {
            path: file.to_path_buf(),
            message,
        });
    }
}

impl ConfigStore for JsonConfigStore {
    fn load(&mut self) -> LoadedConfig {
        let file = self.file();
        let mut loaded = LoadedConfig::default();
        let text = match fs::read_to_string(&file) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => return loaded,
            Err(error) if error.kind() == ErrorKind::InvalidData => {
                self.set_aside(&file, &mut loaded);
                return loaded;
            }
            Err(error) => {
                self.freeze(&file, error.to_string(), &mut loaded);
                return loaded;
            }
        };
        let Some(document) = parse(&text) else {
            self.set_aside(&file, &mut loaded);
            return loaded;
        };
        if document.schema > CURRENT_SCHEMA {
            self.frozen = Some(format!(
                "{} was written by a newer BuildPilot, so it is left untouched",
                file.display()
            ));
            loaded.problems.push(LoadProblem::NewerSchema);
        }
        if !document.unreadable.is_empty() {
            loaded
                .problems
                .push(LoadProblem::UnreadableEntries(document.unreadable.len()));
        }
        self.unreadable = document.unreadable;
        loaded.operations = document.operations;
        loaded.preferences = document.preferences;
        loaded
    }

    fn save(
        &mut self,
        operations: &[Operation],
        preferences: &Preferences,
    ) -> Result<(), StoreError> {
        let file = self.file();
        let failed = |message: String| StoreError {
            path: file.clone(),
            message,
        };
        if let Some(reason) = &self.frozen {
            return Err(failed(reason.clone()));
        }
        let text = render(operations, preferences, &self.unreadable);
        write_atomically(&self.folder, &file, &text).map_err(|error| failed(error.to_string()))
    }

    fn data_folder(&self) -> PathBuf {
        self.folder.clone()
    }
}

/// Writes `text` to `file` inside `folder` (made if missing) by writing a temporary file beside
/// it and renaming that over it (CFG-002). std::fs::rename replaces an existing file on Windows
/// (MoveFileExW with MOVEFILE_REPLACE_EXISTING), so the old file is intact until the new one is
/// whole.
pub(super) fn write_atomically(folder: &Path, file: &Path, text: &str) -> io::Result<()> {
    fs::create_dir_all(folder)?;
    let temporary = with_suffix(file, TEMPORARY_SUFFIX);
    fs::write(&temporary, text)?;
    fs::rename(&temporary, file)
}

fn with_suffix(file: &Path, suffix: &str) -> PathBuf {
    let mut name = OsString::from(file.as_os_str());
    name.push(suffix);
    PathBuf::from(name)
}
