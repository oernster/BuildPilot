//! Which program runs an operation and with what command line (SRS LCH-001).
//!
//! The table of script types lives here once. Everything else asks this module.

use std::path::{Path, PathBuf};

use super::operation::OperationConfig;

/// The kinds of file BuildPilot can launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptKind {
    /// A PowerShell script, run by a PowerShell host.
    PowerShell,
    /// A `.bat` or `.cmd` batch file.
    Batch,
    /// A native executable.
    Executable,
}

/// Supported extensions, lower case, in the order they are listed to the operator.
const EXTENSIONS: &[(&str, ScriptKind)] = &[
    ("ps1", ScriptKind::PowerShell),
    ("bat", ScriptKind::Batch),
    ("cmd", ScriptKind::Batch),
    ("exe", ScriptKind::Executable),
    ("com", ScriptKind::Executable),
];

impl ScriptKind {
    /// The kind of `path`, judged by its extension without regard to case; `None` when the
    /// extension is missing or unsupported.
    pub fn of(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        EXTENSIONS
            .iter()
            .find(|(known, _)| *known == extension)
            .map(|(_, kind)| *kind)
    }

    /// Every supported extension, lower case, without the dot.
    pub fn supported_extensions() -> impl Iterator<Item = &'static str> {
        EXTENSIONS.iter().map(|(extension, _)| *extension)
    }

    /// The supported extensions as the operator reads them, e.g. `.ps1, .bat, .cmd`.
    pub fn supported_list() -> String {
        Self::supported_extensions()
            .map(|extension| format!(".{extension}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Which PowerShell is available to run `.ps1` scripts. Infrastructure finds out; the domain
/// only decides what to do with the answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerShellHost {
    /// PowerShell 7 or later (`pwsh.exe`), preferred when present.
    Pwsh,
    /// Windows PowerShell 5.1 (`powershell.exe`), present on every supported Windows.
    WindowsPowerShell,
}

impl PowerShellHost {
    /// The executable name, resolved through PATH by the operating system.
    pub fn program(self) -> &'static str {
        match self {
            Self::Pwsh => "pwsh.exe",
            Self::WindowsPowerShell => "powershell.exe",
        }
    }
}

/// Arguments placed before the script path when a PowerShell host runs a `.ps1`. `-File` makes
/// the host's exit code the script's exit code, which LIFE-002 depends on.
pub const POWERSHELL_ARGUMENTS: &[&str] = &[
    "-NoProfile",
    "-NonInteractive",
    "-ExecutionPolicy",
    "Bypass",
    "-File",
];

/// Everything infrastructure needs to start one run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchPlan {
    /// The program to start.
    pub program: PathBuf,
    /// The arguments, one entry each; infrastructure quotes them.
    pub arguments: Vec<String>,
    /// The directory the program starts in.
    pub working_dir: PathBuf,
}

/// Builds the launch plan for `config`.
///
/// A batch file is handed to the standard library as the program itself. Rust then runs it
/// through `cmd.exe` with `cmd.exe` escaping and refuses an argument it cannot escape safely.
/// Starting `cmd.exe /c` by hand would quote the arguments by the wrong rules (SRS
/// Amendment 1).
pub fn plan(config: &OperationConfig, powershell: PowerShellHost) -> LaunchPlan {
    let script = config.script_path().to_path_buf();
    let (program, mut arguments) = match config.kind() {
        ScriptKind::PowerShell => {
            let mut leading: Vec<String> = POWERSHELL_ARGUMENTS
                .iter()
                .map(|argument| (*argument).to_owned())
                .collect();
            leading.push(script.to_string_lossy().into_owned());
            (PathBuf::from(powershell.program()), leading)
        }
        ScriptKind::Batch | ScriptKind::Executable => (script, Vec::new()),
    };
    arguments.extend(config.arguments().iter().cloned());
    LaunchPlan {
        program,
        arguments,
        working_dir: config.working_dir().to_path_buf(),
    }
}
