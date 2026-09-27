//! The application layer: one entry point per thing the operator can do.
//!
//! `App` owns all state and is driven from one thread. It never waits on a process: launches
//! return at once and a process's output and exit come back through `App::handle_event`. That
//! makes every use case here a plain function call a test can make.
//!
//! The `impl App` blocks are split by concern across the files of this module.

#![forbid(unsafe_code)]

mod deck_actions;
mod errors;
mod navigation;
pub mod ports;
mod run_actions;
mod runtime;

use std::collections::HashMap;
use std::path::PathBuf;

use crate::domain::deck::FlightDeck;
use crate::domain::launch_plan::PowerShellHost;
use crate::domain::operation::{IconRef, OperationId};
use crate::domain::preferences::Preferences;
use crate::domain::selection::DeckSelection;

pub use deck_actions::EditOutcome;
pub use errors::AppError;
pub use navigation::LocateOutcome;
pub use ports::{
    Clock, ConfigStore, IconLibrary, IdSource, Launcher, LoadProblem, PathProbe, ProcessHandle,
    RunEvent, RunEventKind, RunKey, Shell, StoreError,
};
pub use run_actions::{OverdueStop, STOP_TIMEOUT};

use runtime::OperationRuntime;

/// The collaborators `App` is given at start; the composition root builds them.
pub struct Ports {
    /// Config file.
    pub store: Box<dyn ConfigStore>,
    /// New identities.
    pub ids: Box<dyn IdSource>,
    /// The time.
    pub clock: Box<dyn Clock>,
    /// Path existence.
    pub paths: Box<dyn PathProbe>,
    /// Icons.
    pub icons: Box<dyn IconLibrary>,
    /// Process starting.
    pub launcher: Box<dyn Launcher>,
    /// Opening files and folders.
    pub shell: Box<dyn Shell>,
}

/// Something the operator should be told that is not the answer to their last action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    /// A load problem (CFG-005, CFG-006, CFG-009).
    Load(LoadProblem),
    /// Two stored operations shared an identity; the later one was left out.
    DuplicateDropped(OperationId),
    /// The config file could not be written (CFG-008).
    SaveFailed(StoreError),
    /// Stop could not terminate an operation's processes (STOP-003).
    StopFailed {
        /// The operation's name.
        name: String,
        /// Why.
        message: String,
    },
}

/// How an operation's icon can be shown (ICON-002, ICON-005).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IconStatus {
    /// No icon configured: show the placeholder.
    Placeholder,
    /// Show this image.
    Ready(PathBuf),
    /// The configured image is missing or unreadable: show the placeholder and name this path.
    Missing(PathBuf),
}

/// The application's state and its use cases.
pub struct App {
    deck: FlightDeck,
    selection: DeckSelection,
    preferences: Preferences,
    runtimes: HashMap<OperationId, OperationRuntime>,
    icon_status: HashMap<OperationId, IconStatus>,
    notices: Vec<Notice>,
    powershell: PowerShellHost,
    runs_started: u64,
    ports: Ports,
}

impl App {
    /// Loads the configuration and starts with every operation idle (CFG-004). A problem while
    /// loading becomes a notice; it never stops BuildPilot starting.
    pub fn start(mut ports: Ports, powershell: PowerShellHost) -> Self {
        let loaded = ports.store.load();
        let mut notices: Vec<Notice> = loaded.problems.into_iter().map(Notice::Load).collect();
        let mut deck = FlightDeck::default();
        for operation in loaded.operations {
            let id = operation.id().clone();
            if deck.add(operation).is_err() {
                notices.push(Notice::DuplicateDropped(id));
            }
        }
        let mut app = Self {
            deck,
            selection: DeckSelection::default(),
            preferences: loaded.preferences,
            runtimes: HashMap::new(),
            icon_status: HashMap::new(),
            notices,
            powershell,
            runs_started: 0,
            ports,
        };
        let icons: Vec<(OperationId, IconRef)> = app
            .deck
            .operations()
            .iter()
            .map(|operation| (operation.id().clone(), operation.config().icon().clone()))
            .collect();
        for (id, icon) in &icons {
            app.record_icon_status(id, icon);
        }
        app
    }

    /// The flight deck.
    pub fn deck(&self) -> &FlightDeck {
        &self.deck
    }

    /// The selected and checked rows.
    pub fn selection(&self) -> &DeckSelection {
        &self.selection
    }

    /// The global preferences.
    pub fn preferences(&self) -> &Preferences {
        &self.preferences
    }

    /// How operation `id`'s icon can be shown.
    pub fn icon_status(&self, id: &OperationId) -> IconStatus {
        self.icon_status
            .get(id)
            .cloned()
            .unwrap_or(IconStatus::Placeholder)
    }

    /// Hands over every notice raised since the last call.
    pub fn take_notices(&mut self) -> Vec<Notice> {
        std::mem::take(&mut self.notices)
    }

    /// Writes the configuration; a failure becomes a notice and the change stays in memory
    /// (CFG-008).
    fn persist(&mut self) {
        let saved = self
            .ports
            .store
            .save(self.deck.operations(), &self.preferences);
        if let Err(error) = saved {
            self.notices.push(Notice::SaveFailed(error));
        }
    }

    /// Checks whether `icon`, operation `id`'s icon, can be shown and records the answer.
    fn record_icon_status(&mut self, id: &OperationId, icon: &IconRef) {
        let status = match icon {
            IconRef::Placeholder => IconStatus::Placeholder,
            IconRef::Discovered(path) | IconRef::Chosen(path) => {
                if self.ports.icons.is_readable(path) {
                    IconStatus::Ready(path.clone())
                } else {
                    IconStatus::Missing(path.clone())
                }
            }
        };
        self.icon_status.insert(id.clone(), status);
    }
}
