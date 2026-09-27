//! Why an action was refused. Every message names the thing and what the operator can do.

use std::error::Error;
use std::fmt;
use std::path::PathBuf;

use crate::domain::deck::DeckError;
use crate::domain::lifecycle::TransitionError;
use crate::domain::operation::OperationError;

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
        }
    }
}

impl Error for AppError {}

impl From<OperationError> for AppError {
    fn from(error: OperationError) -> Self {
        Self::Invalid(error)
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
