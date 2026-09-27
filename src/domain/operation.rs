//! A configured build operation: what BuildPilot remembers between runs (SRS 3.1 to 3.4).
//!
//! `OperationSpec` is what the operator typed; it may be wrong. `OperationConfig` is a spec that
//! has passed validation, so code holding one never has to check it again.

use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

use super::launch_plan::ScriptKind;

/// Stable identity of an operation (CFG-003). Assigned once by infrastructure, never reused.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OperationId(String);

impl OperationId {
    /// Wraps `value`; refuses an empty or blank identifier.
    pub fn new(value: impl Into<String>) -> Result<Self, OperationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(OperationError::EmptyId);
        }
        Ok(Self(value))
    }

    /// The identifier as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OperationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Where an operation's icon comes from (ICON-001 to ICON-004).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum IconRef {
    /// No icon; the row shows BuildPilot's placeholder.
    #[default]
    Placeholder,
    /// Found by convention beside the script, referenced by path (OQ-6).
    Discovered(PathBuf),
    /// Chosen by the operator and copied into the data folder (OQ-6).
    Chosen(PathBuf),
}

/// An operation's editable fields, as entered. Not yet validated.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OperationSpec {
    /// Display name.
    pub name: String,
    /// The script or executable to run; must be absolute.
    pub script_path: PathBuf,
    /// The directory to run it in; must be absolute.
    pub working_dir: PathBuf,
    /// Arguments, one entry each (OQ-8).
    pub arguments: Vec<String>,
    /// The icon.
    pub icon: IconRef,
}

/// An operation's fields after validation. Construct with `OperationConfig::try_from`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationConfig {
    name: String,
    script_path: PathBuf,
    working_dir: PathBuf,
    arguments: Vec<String>,
    icon: IconRef,
    kind: ScriptKind,
}

impl OperationConfig {
    /// Display name, trimmed.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Absolute path of the script or executable.
    pub fn script_path(&self) -> &Path {
        &self.script_path
    }
    /// Absolute working directory.
    pub fn working_dir(&self) -> &Path {
        &self.working_dir
    }
    /// Arguments, one entry each.
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }
    /// The icon.
    pub fn icon(&self) -> &IconRef {
        &self.icon
    }
    /// What kind of file the script is.
    pub fn kind(&self) -> ScriptKind {
        self.kind
    }
    /// The same configuration with a different icon; an icon never affects a run.
    pub fn with_icon(&self, icon: IconRef) -> Self {
        Self {
            icon,
            ..self.clone()
        }
    }
    /// The fields back as an editable spec, for the Edit dialog.
    pub fn to_spec(&self) -> OperationSpec {
        OperationSpec {
            name: self.name.clone(),
            script_path: self.script_path.clone(),
            working_dir: self.working_dir.clone(),
            arguments: self.arguments.clone(),
            icon: self.icon.clone(),
        }
    }
    /// True when running with `other` would launch something different (EDIT-002).
    pub fn differs_in_execution(&self, other: &Self) -> bool {
        self.script_path != other.script_path
            || self.working_dir != other.working_dir
            || self.arguments != other.arguments
    }
}

impl TryFrom<OperationSpec> for OperationConfig {
    type Error = OperationError;

    /// Validates `spec` (ADD-005), reporting the first problem found.
    fn try_from(spec: OperationSpec) -> Result<Self, Self::Error> {
        let name = spec.name.trim().to_owned();
        if name.is_empty() {
            return Err(OperationError::EmptyName);
        }
        if spec.script_path.as_os_str().is_empty() {
            return Err(OperationError::EmptyScriptPath);
        }
        if !spec.script_path.is_absolute() {
            return Err(OperationError::ScriptPathNotAbsolute(spec.script_path));
        }
        let kind = ScriptKind::of(&spec.script_path)
            .ok_or_else(|| OperationError::UnsupportedScriptType(spec.script_path.clone()))?;
        if spec.working_dir.as_os_str().is_empty() {
            return Err(OperationError::EmptyWorkingDir);
        }
        if !spec.working_dir.is_absolute() {
            return Err(OperationError::WorkingDirNotAbsolute(spec.working_dir));
        }
        Ok(Self {
            name,
            script_path: spec.script_path,
            working_dir: spec.working_dir,
            arguments: spec.arguments,
            icon: spec.icon,
            kind,
        })
    }
}

/// A persisted operation: identity plus validated configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation {
    id: OperationId,
    config: OperationConfig,
}

impl Operation {
    /// Pairs an identity with a configuration.
    pub fn new(id: OperationId, config: OperationConfig) -> Self {
        Self { id, config }
    }
    /// The stable identity.
    pub fn id(&self) -> &OperationId {
        &self.id
    }
    /// The configuration.
    pub fn config(&self) -> &OperationConfig {
        &self.config
    }
    /// Replaces the configuration; the identity never changes.
    pub fn set_config(&mut self, config: OperationConfig) {
        self.config = config;
    }
}

/// The defaults for a newly chosen script (ADD-002 to ADD-004). The icon is left as the
/// placeholder; icon discovery is infrastructure's job and happens afterwards.
pub fn draft_for_script(script: &Path) -> Result<OperationSpec, OperationError> {
    if ScriptKind::of(script).is_none() {
        return Err(OperationError::UnsupportedScriptType(script.to_path_buf()));
    }
    let working_dir = script
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| OperationError::NoParentDirectory(script.to_path_buf()))?;
    Ok(OperationSpec {
        name: default_name(script, working_dir),
        script_path: script.to_path_buf(),
        working_dir: working_dir.to_path_buf(),
        arguments: Vec::new(),
        icon: IconRef::Placeholder,
    })
}

/// The folder name, a space and the file stem (OQ-5); just the stem when the script sits at the
/// root of a drive.
fn default_name(script: &Path, folder: &Path) -> String {
    let stem = script
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    match folder.file_name() {
        Some(folder_name) => format!("{} {stem}", folder_name.to_string_lossy()),
        None => stem,
    }
}

/// Parses the Edit dialog's argument box: one argument per line, each trimmed, blank lines
/// dropped (OQ-8).
pub fn parse_argument_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The reverse of `parse_argument_lines`, for showing stored arguments in the dialog.
pub fn format_argument_lines(arguments: &[String]) -> String {
    arguments.join("\n")
}

/// Why an operation's fields were refused. Each message names the field and what to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationError {
    /// The identifier was blank.
    EmptyId,
    /// The name was blank.
    EmptyName,
    /// No script was given.
    EmptyScriptPath,
    /// The script path was relative.
    ScriptPathNotAbsolute(PathBuf),
    /// The script's extension is not one BuildPilot can launch.
    UnsupportedScriptType(PathBuf),
    /// No working directory was given.
    EmptyWorkingDir,
    /// The working directory was relative.
    WorkingDirNotAbsolute(PathBuf),
    /// The script path has no folder to default the working directory from.
    NoParentDirectory(PathBuf),
}

impl fmt::Display for OperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId => f.write_str("The operation has no identifier."),
            Self::EmptyName => f.write_str("Name is missing: enter a display name."),
            Self::EmptyScriptPath => f.write_str("Script is missing: choose a script to run."),
            Self::ScriptPathNotAbsolute(path) => write!(
                f,
                "Script path {} is relative: give the full path, starting with the drive.",
                path.display()
            ),
            Self::UnsupportedScriptType(path) => write!(
                f,
                "{} is not a type BuildPilot can run. Supported types: {}.",
                path.display(),
                ScriptKind::supported_list()
            ),
            Self::EmptyWorkingDir => {
                f.write_str("Working directory is missing: choose the folder to run in.")
            }
            Self::WorkingDirNotAbsolute(path) => write!(
                f,
                "Working directory {} is relative: give the full path, starting with the drive.",
                path.display()
            ),
            Self::NoParentDirectory(path) => write!(
                f,
                "{} has no containing folder to run in: choose a working directory.",
                path.display()
            ),
        }
    }
}

impl Error for OperationError {}
