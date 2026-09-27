//! Opening and revealing scripts; opening the data folder (SRS 3.11, UI-004).

use std::path::PathBuf;

use crate::domain::deck::DeckError;
use crate::domain::operation::OperationId;

use super::{App, AppError};

/// What Locate Script did (NAV-002, NAV-003).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocateOutcome {
    /// Explorer shows the script, selected.
    Revealed,
    /// The script is missing; Explorer shows this, the nearest folder that still exists.
    ScriptMissingFolderOpened {
        /// The script that was not found.
        script: PathBuf,
        /// The folder opened instead.
        folder: PathBuf,
    },
    /// The script is missing and so is every folder above it.
    ScriptMissingNothingOpened(PathBuf),
}

impl App {
    /// Opens operation `id`'s script in its associated application (NAV-001).
    pub fn open_script(&self, id: &OperationId) -> Result<(), AppError> {
        let script = self.script_of(id)?;
        if !self.ports.paths.is_file(&script) {
            return Err(AppError::ScriptMissing(script));
        }
        self.ports.shell.open(&script).map_err(AppError::Shell)
    }

    /// Shows operation `id`'s script in Explorer; when it is missing, opens the nearest folder
    /// above it that still exists (NAV-003).
    pub fn locate_script(&self, id: &OperationId) -> Result<LocateOutcome, AppError> {
        let script = self.script_of(id)?;
        if self.ports.paths.is_file(&script) {
            self.ports.shell.reveal(&script).map_err(AppError::Shell)?;
            return Ok(LocateOutcome::Revealed);
        }
        let nearest = script
            .ancestors()
            .skip(1)
            .find(|folder| self.ports.paths.is_dir(folder))
            .map(PathBuf::from);
        match nearest {
            Some(folder) => {
                self.ports
                    .shell
                    .open_folder(&folder)
                    .map_err(AppError::Shell)?;
                Ok(LocateOutcome::ScriptMissingFolderOpened { script, folder })
            }
            None => Ok(LocateOutcome::ScriptMissingNothingOpened(script)),
        }
    }

    /// The folder holding BuildPilot's configuration (UI-004).
    pub fn data_folder(&self) -> PathBuf {
        self.ports.store.data_folder()
    }

    /// Opens the data folder in Explorer (UI-004).
    pub fn open_data_folder(&self) -> Result<(), AppError> {
        self.ports
            .shell
            .open_folder(&self.data_folder())
            .map_err(AppError::Shell)
    }

    fn script_of(&self, id: &OperationId) -> Result<PathBuf, AppError> {
        let operation = self
            .deck
            .get(id)
            .ok_or_else(|| DeckError::NotFound(id.clone()))?;
        Ok(operation.config().script_path().to_path_buf())
    }
}
