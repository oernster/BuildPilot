//! The operator's host table: which program runs a script type (SRS 3.19, HOST-001 to HOST-003).
//!
//! A step's host is decided here once, from the table and the built-in rule of LCH-001. The
//! table wins, even for a type BuildPilot knows (HOST-002).

use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

use super::environment::Need;
use super::launch_plan::ScriptKind;
use super::operation::OperationError;

/// One row: the program that runs every file of one extension, with its leading arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostRow {
    extension: String,
    program: PathBuf,
    arguments: Vec<String>,
}

impl HostRow {
    /// A row for `extension` (with or without its dot, any case), run by `program` with
    /// `arguments` ahead of the script. `program` is a full path or a bare file name that
    /// Windows finds on PATH.
    pub fn new(
        extension: &str,
        program: PathBuf,
        arguments: Vec<String>,
    ) -> Result<Self, HostError> {
        let extension = extension.trim();
        let extension = extension
            .strip_prefix('.')
            .unwrap_or(extension)
            .to_ascii_lowercase();
        if extension.is_empty() {
            return Err(HostError::EmptyExtension);
        }
        if !extension.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(HostError::BadExtension(extension));
        }
        if program.as_os_str().is_empty() {
            return Err(HostError::EmptyProgram(extension));
        }
        let bare_name = program.components().count() == 1;
        if !program.is_absolute() && !bare_name {
            return Err(HostError::ProgramNotAbsolute(program));
        }
        Ok(Self {
            extension,
            program,
            arguments,
        })
    }
    /// The extension, lower case, without the dot.
    pub fn extension(&self) -> &str {
        &self.extension
    }
    /// The program to start.
    pub fn program(&self) -> &Path {
        &self.program
    }
    /// The arguments placed before the script path, one entry each.
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }
}

/// The operator's rows, at most one per extension.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HostTable {
    rows: Vec<HostRow>,
}

impl HostTable {
    /// The table of `rows`, in the order given; refuses two rows for one extension.
    pub fn new(rows: Vec<HostRow>) -> Result<Self, HostError> {
        for (index, row) in rows.iter().enumerate() {
            if rows[..index]
                .iter()
                .any(|earlier| earlier.extension == row.extension)
            {
                return Err(HostError::Duplicate(row.extension.clone()));
            }
        }
        Ok(Self { rows })
    }
    /// The rows in order.
    pub fn rows(&self) -> &[HostRow] {
        &self.rows
    }
    /// The row for `path`'s extension, judged without regard to case.
    pub fn find(&self, path: &Path) -> Option<&HostRow> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        self.rows.iter().find(|row| row.extension == extension)
    }
    /// What runs a file at `path`: the operator's row first, then the built-in rule (HOST-002);
    /// `None` when neither covers it.
    pub fn host_for(&self, path: &Path) -> Option<Host<'_>> {
        self.find(path)
            .map(Host::Operator)
            .or_else(|| ScriptKind::of(path).map(Host::BuiltIn))
    }
    /// The host for `script`, refusing a type nothing runs (LCH-001).
    pub fn check(&self, script: &Path) -> Result<Host<'_>, OperationError> {
        self.host_for(script)
            .ok_or_else(|| OperationError::UnsupportedScriptType {
                path: script.to_path_buf(),
                supported: self.supported_list(),
            })
    }
    /// Every extension a script may have, built-in first then the operator's, each once
    /// (HOST-003).
    pub fn extensions(&self) -> Vec<String> {
        let mut extensions: Vec<String> = ScriptKind::supported_extensions()
            .map(str::to_owned)
            .collect();
        for row in &self.rows {
            if !extensions.contains(&row.extension) {
                extensions.push(row.extension.clone());
            }
        }
        extensions
    }
    /// The extensions as the operator reads them, e.g. `.ps1, .bat, .rb`.
    pub fn supported_list(&self) -> String {
        self.extensions()
            .iter()
            .map(|extension| format!(".{extension}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// What runs one script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Host<'a> {
    /// BuildPilot's own rule for the type (LCH-001).
    BuiltIn(ScriptKind),
    /// The operator's row (HOST-002).
    Operator(&'a HostRow),
}

impl Host<'_> {
    /// How much a step with this host depends on an environment. The operator's program is run
    /// as given, never activated (HOST-002).
    pub fn environment_need(self) -> Need {
        match self {
            Self::BuiltIn(kind) => kind.environment_need(),
            Self::Operator(_) => Need::None,
        }
    }
}

/// Why a host row was refused. Each message names the row and what to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostError {
    /// No extension was given.
    EmptyExtension,
    /// The extension holds something other than letters and digits.
    BadExtension(String),
    /// No program was given for this extension.
    EmptyProgram(String),
    /// The program path was relative.
    ProgramNotAbsolute(PathBuf),
    /// Two rows named this extension.
    Duplicate(String),
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyExtension => f.write_str("A host has no file type: enter one, e.g. .rb."),
            Self::BadExtension(extension) => write!(
                f,
                "File type .{extension} is not valid: use letters and digits only, e.g. .rb."
            ),
            Self::EmptyProgram(extension) => write!(
                f,
                "The host for .{extension} has no program: choose the program that runs it."
            ),
            Self::ProgramNotAbsolute(path) => write!(
                f,
                "Host program {} is relative: give the full path or just a file name on PATH.",
                path.display()
            ),
            Self::Duplicate(extension) => {
                write!(f, "Two hosts are set for .{extension}: keep one of them.")
            }
        }
    }
}

impl Error for HostError {}
