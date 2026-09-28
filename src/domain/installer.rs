//! A project's installer: where BuildPilot looks for it when none is set and when Launch
//! installer is available (SRS 3.21).

use std::fmt;
use std::path::{Path, PathBuf};

use super::lifecycle::RunState;

/// The folders inside the working directory where a build leaves its installer, in the order
/// they are searched (PKG-002).
pub const INSTALLER_FOLDERS: [&str; 2] = ["dist-installer", "dist"];

/// What follows the project's name in its installer's file name (PKG-002).
const INSTALLER_SUFFIX: &str = "Setup.exe";

/// The extension an installer must carry, compared without regard to case.
const INSTALLER_EXTENSION: &str = "exe";

/// The installer found for a project in `working_dir` when none is set (PKG-002): the first file
/// in `dist-installer`, then in `dist`, that `is_default_installer` accepts. `files` lists the
/// names of the files directly inside a folder; `None` when neither folder holds one.
pub fn find_default_installer(
    working_dir: &Path,
    files: &dyn Fn(&Path) -> Vec<String>,
) -> Option<PathBuf> {
    let project = working_dir.file_name()?.to_string_lossy().into_owned();
    INSTALLER_FOLDERS.iter().find_map(|folder| {
        let folder = working_dir.join(folder);
        files(&folder)
            .into_iter()
            .find(|name| is_default_installer(name, &project))
            .map(|name| folder.join(name))
    })
}

/// True when `file_name` is the installer of `project`: the project's name followed by
/// `Setup.exe`, compared by letters and digits alone without regard to case. So `fulcrum` finds
/// `FulcrumSetup.exe`, `postal-gambit` finds `PostalGambitSetup.exe` and `HCS-Plugin-BridgeTalk`
/// finds `HCS-Plugin-BridgeTalk-Setup.exe` (PKG-002).
pub fn is_default_installer(file_name: &str, project: &str) -> bool {
    let is_program = Path::new(file_name)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(INSTALLER_EXTENSION));
    let project_key = key(project);
    is_program && !project_key.is_empty() && key(file_name) == project_key + &key(INSTALLER_SUFFIX)
}

/// `text` as its letters and digits alone, in lower case.
fn key(text: &str) -> String {
    text.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|character| character.to_ascii_lowercase())
        .collect()
}

/// The installer Launch installer may start for a run in `state`, when `installer` is the one on
/// disk; else why not (PKG-003, PKG-004). Launching is open before the first run and after one
/// that succeeded; never otherwise. A run in progress comes first, then a stopped one, then a
/// failed one, then a missing installer, so the reason given is the one to see to first.
pub fn launchable<'a>(
    state: &RunState,
    installer: Option<&'a Path>,
) -> Result<&'a Path, InstallerBlock> {
    match state {
        RunState::Running(_) => Err(InstallerBlock::Running),
        RunState::Stopped => Err(InstallerBlock::Stopped),
        RunState::Failed(_) => Err(InstallerBlock::Failed),
        RunState::Idle | RunState::Succeeded => installer.ok_or(InstallerBlock::NotFound),
    }
}

/// Why Launch installer is not available (PKG-003, PKG-004).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallerBlock {
    /// None was found where PKG-002 looks; else the one set is missing.
    NotFound,
    /// The operation is running or stopping, so its installer may be half written.
    Running,
    /// The latest run was stopped before it finished, so its installer may be incomplete.
    Stopped,
    /// The latest run failed, so its installer may be incomplete or stale.
    Failed,
}

/// The reason as the tooltip and a refusal say it, after the control's name.
impl fmt::Display for InstallerBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotFound => "no installer found; set one in Edit",
            Self::Running => "wait for the build to finish",
            Self::Stopped => "the last build was stopped; run it to success first",
            Self::Failed => "the last build failed; run it to success first",
        })
    }
}
