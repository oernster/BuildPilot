//! Why an action was refused. Every message names the thing and what the operator can do.

use std::error::Error;
use std::fmt;
use std::path::PathBuf;

use crate::domain::deck::DeckError;
use crate::domain::host::HostError;
use crate::domain::lifecycle::TransitionError;
use crate::domain::operation::OperationError;

use super::Notice;
use super::ports::LoadProblem;

/// An action the application refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    /// The operation's fields are invalid.
    Invalid(OperationError),
    /// The deck refused the change.
    Deck(DeckError),
    /// The run state does not allow it.
    Transition(TransitionError),
    /// A running operation cannot be removed (REM-003).
    RemoveWhileRunning(String),
    /// The chosen icon could not be copied into the data folder.
    IconImport {
        /// The image chosen.
        source: PathBuf,
        /// Why it failed.
        message: String,
    },
    /// The script is not where the operation says (NAV-003).
    ScriptMissing(PathBuf),
    /// Windows could not open or show something (NAV-004).
    Shell(String),
    /// Several environments were found and none was chosen (ENV-002).
    EnvironmentNotChosen(Vec<String>),
    /// A row of the host table is invalid (HOST-001).
    Host(HostError),
    /// Another operation is running in this working directory (LCH-010).
    FolderInUse {
        /// The working directory.
        folder: PathBuf,
        /// The operation running there.
        by: String,
    },
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(error) => error.fmt(f),
            Self::Deck(error) => error.fmt(f),
            Self::Transition(error) => error.fmt(f),
            Self::RemoveWhileRunning(name) => {
                write!(f, "{name} is running. Stop it before removing it.")
            }
            Self::IconImport { source, message } => write!(
                f,
                "Could not use {} as the icon: {message}",
                source.display()
            ),
            Self::ScriptMissing(path) => write!(
                f,
                "Script not found: {}. Use Edit to point at its new location.",
                path.display()
            ),
            Self::Shell(message) => write!(f, "Windows could not open it: {message}"),
            Self::EnvironmentNotChosen(found) => write!(
                f,
                "Choose the environment to use: this folder holds several ({}).",
                found.join(", ")
            ),
            Self::Host(error) => error.fmt(f),
            Self::FolderInUse { folder, by } => write!(
                f,
                "{by} is building in {} now. Run this when it has finished, so the two do not \
                 overwrite each other's files.",
                folder.display()
            ),
        }
    }
}

impl Error for AppError {}

/// What the notice area and the log say about a notice.
impl fmt::Display for Notice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Load(LoadProblem::UnreadableEntries(count)) => write!(
                f,
                "{count} saved operation(s) could not be read. They are kept in the settings file, untouched."
            ),
            Self::Load(LoadProblem::SetAside(path)) => write!(
                f,
                "The settings file could not be read, so BuildPilot started empty. The old file was kept as {}.",
                path.display()
            ),
            Self::Load(LoadProblem::NewerSchema) => f.write_str(
                "The settings file was written by a newer BuildPilot. \
                It is shown here but changes will not be saved.",
            ),
            Self::Load(LoadProblem::Unreadable { path, message }) => write!(
                f,
                "The settings file {} could not be read ({message}). Changes will not be saved this session.",
                path.display()
            ),
            Self::DuplicateDropped(id) => write!(
                f,
                "Two saved operations shared the identity {id}; the second was left out."
            ),
            Self::SaveFailed(error) => error.fmt(f),
            Self::StopFailed { name, message } => write!(f, "Could not stop {name}: {message}"),
        }
    }
}

impl From<OperationError> for AppError {
    fn from(error: OperationError) -> Self {
        Self::Invalid(error)
    }
}

impl From<HostError> for AppError {
    fn from(error: HostError) -> Self {
        Self::Host(error)
    }
}

impl From<DeckError> for AppError {
    fn from(error: DeckError) -> Self {
        Self::Deck(error)
    }
}

impl From<TransitionError> for AppError {
    fn from(error: TransitionError) -> Self {
        Self::Transition(error)
    }
}
