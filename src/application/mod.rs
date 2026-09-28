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
mod installer_actions;
mod navigation;
pub mod ports;
mod run_actions;
mod runtime;
mod scan_actions;
mod step_actions;
pub mod updates;

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
    Clock, ConfigStore, IconLibrary, IdSource, Launcher, LoadProblem, Log, PathProbe,
    ProcessHandle, Release, ReleaseAsset, ReleaseSource, RunEvent, RunEventKind, RunKey, Shell,
    StoreError, Variables,
};
pub use run_actions::{OverdueStop, STOP_TIMEOUT};
pub use scan_actions::{FolderScan, ScannedFolder};

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
    /// The log file.
    pub log: Box<dyn Log>,
    /// BuildPilot's own environment variables.
    pub variables: Box<dyn Variables>,
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
    /// Each operation's installer as last looked for (PKG-001, PKG-002). Looked for at start, on
    /// add, edit and select and when a run ends, never on every redraw: the rows redraw four
    /// times a second and a look reads two folders.
    installers: HashMap<OperationId, Option<PathBuf>>,
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
        let mut app = Self {
            deck: FlightDeck::default(),
            selection: DeckSelection::default(),
            preferences: loaded.preferences,
            runtimes: HashMap::new(),
            icon_status: HashMap::new(),
            installers: HashMap::new(),
            notices: Vec::new(),
            powershell,
            runs_started: 0,
            ports,
        };
        for problem in loaded.problems {
            app.raise(Notice::Load(problem));
        }
        for operation in loaded.operations {
            let id = operation.id().clone();
            if app.deck.add(operation).is_err() {
                app.raise(Notice::DuplicateDropped(id));
            }
        }
        let icons: Vec<(OperationId, IconRef)> = app
            .deck
            .operations()
            .iter()
            .map(|operation| (operation.id().clone(), operation.config().icon().clone()))
            .collect();
        for (id, icon) in &icons {
            app.record_icon_status(id, icon);
            app.record_installer(id);
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
            self.raise(Notice::SaveFailed(error));
        }
    }

    /// Logs `notice` and holds it for the notice area.
    fn raise(&mut self, notice: Notice) {
        self.ports.log.record(&notice.to_string());
        self.notices.push(notice);
    }

    /// Logs a refusal the operator has been shown (NFR-OBS-001).
    pub fn record_refusal(&self, error: &AppError) {
        self.ports.log.record(&format!("Refused: {error}"));
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
