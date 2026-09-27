//! A configured build operation: what BuildPilot remembers between runs (SRS 3.1 to 3.4).
//!
//! `OperationSpec` is what the operator typed; it may be wrong. `OperationConfig` is a spec that
//! has passed validation, so code holding one never has to check it again.

use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

use super::environment::Need;
use super::host::HostTable;
use super::launch_plan::need_of;
use super::step::{Step, StepSpec};

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
    /// The steps, in the order they run (STEP-001).
    pub steps: Vec<StepSpec>,
    /// The directory every step runs in; must be absolute.
    pub working_dir: PathBuf,
    /// The environment's folder name, where the operator chose one (ENV-002).
    pub environment: Option<String>,
    /// The icon.
    pub icon: IconRef,
}

/// An operation's fields after validation. Construct with `OperationConfig::try_from`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationConfig {
    name: String,
    steps: Vec<Step>,
    working_dir: PathBuf,
    environment: Option<String>,
    icon: IconRef,
}

impl OperationConfig {
    /// Display name, trimmed.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// The steps in order; never empty.
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }
    /// The first step, which the row shows, opens and locates (ROW-001).
    pub fn first_step(&self) -> &Step {
        &self.steps[0]
    }
    /// Absolute working directory.
    pub fn working_dir(&self) -> &Path {
        &self.working_dir
    }
    /// The chosen environment's folder name, if the operator chose one (ENV-002).
    pub fn environment(&self) -> Option<&str> {
        self.environment.as_deref()
    }
    /// The icon.
    pub fn icon(&self) -> &IconRef {
        &self.icon
    }
    /// How much the steps depend on an environment, run by `hosts`: the most any one step
    /// needs. A step nothing runs needs nothing; Run refuses it on its own account.
    pub fn environment_need(&self, hosts: &HostTable) -> Need {
        need_of(
            self.steps
                .iter()
                .filter_map(|step| hosts.host_for(step.script_path())),
        )
    }
    /// Refuses the first step whose type nothing in `hosts` runs (LCH-001). Add and Edit ask
    /// this; loading does not, so removing a host never loses an operation.
    pub fn check_hosts(&self, hosts: &HostTable) -> Result<(), OperationError> {
        self.steps
            .iter()
            .try_for_each(|step| hosts.check(step.script_path()).map(|_| ()))
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
            steps: self.steps.iter().map(Step::to_spec).collect(),
            working_dir: self.working_dir.clone(),
            environment: self.environment.clone(),
            icon: self.icon.clone(),
        }
    }
    /// True when running with `other` would launch something different (EDIT-002).
    pub fn differs_in_execution(&self, other: &Self) -> bool {
        self.steps != other.steps
            || self.working_dir != other.working_dir
            || self.environment != other.environment
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
        if spec.steps.is_empty() {
            return Err(OperationError::EmptyScriptPath);
        }
        let steps = spec
            .steps
            .into_iter()
            .map(Step::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        if spec.working_dir.as_os_str().is_empty() {
            return Err(OperationError::EmptyWorkingDir);
        }
        if !spec.working_dir.is_absolute() {
            return Err(OperationError::WorkingDirNotAbsolute(spec.working_dir));
        }
        Ok(Self {
            name,
            steps,
            working_dir: spec.working_dir,
            environment: spec
                .environment
                .filter(|environment| !environment.trim().is_empty()),
            icon: spec.icon,
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
pub fn draft_for_script(script: &Path, hosts: &HostTable) -> Result<OperationSpec, OperationError> {
    hosts.check(script)?;
    let working_dir = script
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| OperationError::NoParentDirectory(script.to_path_buf()))?;
    Ok(OperationSpec {
        name: default_name(script, working_dir),
        steps: vec![StepSpec::for_script(script)],
        working_dir: working_dir.to_path_buf(),
        environment: None,
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
    /// Nothing runs the script's type: neither a built-in rule nor the operator's table.
    UnsupportedScriptType {
        /// The script.
        path: PathBuf,
        /// The types that can run, as the operator reads them.
        supported: String,
    },
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
            Self::UnsupportedScriptType { path, supported } => write!(
                f,
                "{} is not a type BuildPilot can run. Supported types: {supported}. \
                 Add a host for its type in Settings to run it.",
                path.display()
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
