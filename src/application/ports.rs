//! What the application needs from the outside world, stated as traits. Infrastructure
//! implements them; tests implement them with fakes. Nothing here does I/O itself.

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::domain::launch_plan::LaunchPlan;
use crate::domain::operation::{Operation, OperationId};
use crate::domain::output::Stream;
use crate::domain::preferences::Preferences;

/// Reads and writes the config file (SRS 3.1).
pub trait ConfigStore {
    /// Loads everything that can be loaded. Never fails outright: a problem is reported in
    /// `LoadedConfig::problems` and the rest still loads (CFG-005, CFG-006).
    fn load(&mut self) -> LoadedConfig;
    /// Writes `operations` (in deck order) with `preferences` (CFG-001, CFG-002).
    fn save(
        &mut self,
        operations: &[Operation],
        preferences: &Preferences,
    ) -> Result<(), StoreError>;
    /// The data folder, shown in Settings (UI-004).
    fn data_folder(&self) -> PathBuf;
}

/// What a load produced.
#[derive(Debug, Clone, Default)]
pub struct LoadedConfig {
    /// The operations that could be read, in deck order.
    pub operations: Vec<Operation>,
    /// The preferences; defaults where none were stored.
    pub preferences: Preferences,
    /// Everything that went wrong.
    pub problems: Vec<LoadProblem>,
}

/// A problem found while loading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadProblem {
    /// This many operation entries could not be read; they are kept in the file (CFG-005).
    UnreadableEntries(usize),
    /// The whole file could not be read and was renamed to this path (CFG-006).
    SetAside(PathBuf),
    /// The file was written by a newer BuildPilot, so it will not be overwritten (CFG-009).
    NewerSchema,
    /// The file exists but could not be read or set aside; it is left untouched and changes
    /// are not saved this session.
    Unreadable {
        /// The config file.
        path: PathBuf,
        /// The operating system's message.
        message: String,
    },
}

/// A failed write (CFG-008).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreError {
    /// The file that could not be written.
    pub path: PathBuf,
    /// The operating system's message.
    pub message: String,
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Could not save {}: {}. Changes are kept until BuildPilot closes.",
            self.path.display(),
            self.message
        )
    }
}

/// Keeps the log file (NFR-OBS-001). Writing never fails from the caller's side: a log that
/// cannot be written must not stop a build.
pub trait Log {
    /// Records one line; the log adds the time.
    fn record(&self, line: &str);
}

/// Hands out new operation identities (CFG-003).
pub trait IdSource {
    /// A fresh identity, never handed out before.
    fn next_id(&mut self) -> OperationId;
}

/// Tells the time. The only way the application learns it.
pub trait Clock {
    /// The current instant.
    fn now(&self) -> Instant;
}

/// Asks whether paths exist (LCH-007, LCH-008, NAV-003).
pub trait PathProbe {
    /// True when `path` is an existing file.
    fn is_file(&self, path: &Path) -> bool;
    /// True when `path` is an existing directory.
    fn is_dir(&self, path: &Path) -> bool;
}

/// Finds, checks and stores operation icons (SRS 3.3).
pub trait IconLibrary {
    /// The conventional icon beside a script in `script_dir`, when present and readable
    /// (ICON-001).
    fn discover(&self, script_dir: &Path) -> Option<PathBuf>;
    /// True when `path` is an image that can be shown (ICON-005).
    fn is_readable(&self, path: &Path) -> bool;
    /// Copies `source` into the data folder as operation `id`'s icon, replacing any earlier
    /// copy. Answers where it now lives (OQ-6).
    fn import(&mut self, id: &OperationId, source: &Path) -> Result<PathBuf, String>;
    /// Deletes operation `id`'s copied icon, if there is one (REM-002).
    fn release(&mut self, id: &OperationId);
}

/// Starts processes (SRS 3.7). Output and exit arrive later as `RunEvent`s tagged with `key`.
pub trait Launcher {
    /// Starts `plan`. An `Err` carries the operating system's message.
    fn spawn(&mut self, key: RunKey, plan: &LaunchPlan) -> Result<Box<dyn ProcessHandle>, String>;
}

/// A started process tree.
pub trait ProcessHandle {
    /// The process identifier of the program started, for STOP-003's notice.
    fn pid(&self) -> u32;
    /// Terminates the whole process tree (STOP-001). The exit still arrives as a `RunEvent`.
    fn stop(&mut self) -> Result<(), String>;
}

/// Opens things with Windows (SRS 3.11).
pub trait Shell {
    /// Opens `file` with its associated application (NAV-001).
    fn open(&self, file: &Path) -> Result<(), String>;
    /// Opens Explorer with `file` selected (NAV-002).
    fn reveal(&self, file: &Path) -> Result<(), String>;
    /// Opens `folder` in Explorer.
    fn open_folder(&self, folder: &Path) -> Result<(), String>;
}

/// Which run of which operation an event belongs to. The run number keeps a late event from an
/// earlier run from ever being shown under a newer one (OUT-002).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunKey {
    /// The operation.
    pub operation: OperationId,
    /// The run, numbered from one per BuildPilot session.
    pub run: u64,
}

/// Something a running process did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunEvent {
    /// The run it belongs to.
    pub key: RunKey,
    /// What happened.
    pub kind: RunEventKind,
}

/// What a running process did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunEventKind {
    /// A line of output, already decoded.
    Line {
        /// The pipe it came from.
        stream: Stream,
        /// The text.
        text: String,
    },
    /// The process exited, after all its output was delivered.
    Exited {
        /// Its exit code.
        code: i32,
    },
}
