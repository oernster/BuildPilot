//! Which program runs an operation and with what command line (SRS LCH-001).
//!
//! The table of script types lives here once. Everything else asks this module.

use std::path::{Path, PathBuf};

use super::environment::{Need, PYTHON_PROGRAM, VariableEdits, python_in};
use super::step::Step;

/// The kinds of file BuildPilot can launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptKind {
    /// A PowerShell script, run by a PowerShell host.
    PowerShell,
    /// A `.bat` or `.cmd` batch file.
    Batch,
    /// A native executable.
    Executable,
    /// A Python script, run by an existing environment's interpreter (ENV-006).
    Python,
}

/// Supported extensions, lower case, in the order they are listed to the operator.
const EXTENSIONS: &[(&str, ScriptKind)] = &[
    ("ps1", ScriptKind::PowerShell),
    ("bat", ScriptKind::Batch),
    ("cmd", ScriptKind::Batch),
    ("exe", ScriptKind::Executable),
    ("com", ScriptKind::Executable),
    ("py", ScriptKind::Python),
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

    /// How much a step of this kind depends on an environment (ENV-004, OQ-18).
    pub fn environment_need(self) -> Need {
        match self {
            Self::Python => Need::Required,
            Self::PowerShell => Need::Optional,
            Self::Batch | Self::Executable => Need::None,
        }
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

/// Everything infrastructure needs to start one step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchPlan {
    /// The program to start.
    pub program: PathBuf,
    /// The arguments, one entry each; infrastructure quotes them.
    pub arguments: Vec<String>,
    /// The directory the program starts in.
    pub working_dir: PathBuf,
    /// Changes to BuildPilot's own variables for this process alone (ENV-006, ENV-009).
    pub variables: VariableEdits,
}

/// Builds the launch plan for `step`, run in `working_dir`. `environment` is the folder of the
/// environment the run resolved, if any; `variables` are the step's edits from
/// `environment::step_variables`.
///
/// A batch file is handed to the standard library as the program itself. Rust then runs it
/// through `cmd.exe` with `cmd.exe` escaping and refuses an argument it cannot escape safely.
/// Starting `cmd.exe /c` by hand would quote the arguments by the wrong rules (SRS
/// Amendment 1).
pub fn plan(
    step: &Step,
    working_dir: &Path,
    powershell: PowerShellHost,
    environment: Option<&Path>,
    variables: VariableEdits,
) -> LaunchPlan {
    let script = step.script_path().to_path_buf();
    let script_argument = script.to_string_lossy().into_owned();
    let (program, mut arguments) = match step.kind() {
        ScriptKind::PowerShell => {
            let mut leading: Vec<String> = POWERSHELL_ARGUMENTS
                .iter()
                .map(|argument| (*argument).to_owned())
                .collect();
            leading.push(script_argument);
            (PathBuf::from(powershell.program()), leading)
        }
        // Run refuses a .py step with no environment (ENV-004); the bare name is what a caller
        // outside Run gets.
        ScriptKind::Python => (
            environment.map_or_else(|| PathBuf::from(PYTHON_PROGRAM), python_in),
            vec![script_argument],
        ),
        ScriptKind::Batch | ScriptKind::Executable => (script, Vec::new()),
    };
    arguments.extend(step.arguments().iter().cloned());
    LaunchPlan {
        program,
        arguments,
        working_dir: working_dir.to_path_buf(),
        variables,
    }
}
