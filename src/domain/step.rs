//! One step of an operation: a script with its arguments (SRS 3.17).

use std::path::{Path, PathBuf};

use super::launch_plan::ScriptKind;
use super::operation::OperationError;

/// A step's fields, as entered. Not yet validated.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StepSpec {
    /// The script or executable to run; must be absolute.
    pub script_path: PathBuf,
    /// Arguments, one entry each (OQ-8).
    pub arguments: Vec<String>,
}

impl StepSpec {
    /// A step running `script` with no arguments (ADD-004).
    pub fn for_script(script: &Path) -> Self {
        Self {
            script_path: script.to_path_buf(),
            arguments: Vec::new(),
        }
    }
}

/// A step after validation. Construct with `Step::try_from`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    script_path: PathBuf,
    arguments: Vec<String>,
    kind: ScriptKind,
}

impl Step {
    /// Absolute path of the script or executable.
    pub fn script_path(&self) -> &Path {
        &self.script_path
    }
    /// Arguments, one entry each.
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }
    /// What kind of file the script is.
    pub fn kind(&self) -> ScriptKind {
        self.kind
    }
    /// The fields back as an editable spec.
    pub fn to_spec(&self) -> StepSpec {
        StepSpec {
            script_path: self.script_path.clone(),
            arguments: self.arguments.clone(),
        }
    }
}

impl TryFrom<StepSpec> for Step {
    type Error = OperationError;

    /// Validates `spec` (ADD-005), reporting the first problem found.
    fn try_from(spec: StepSpec) -> Result<Self, Self::Error> {
        if spec.script_path.as_os_str().is_empty() {
            return Err(OperationError::EmptyScriptPath);
        }
        if !spec.script_path.is_absolute() {
            return Err(OperationError::ScriptPathNotAbsolute(spec.script_path));
        }
        let kind = ScriptKind::of(&spec.script_path)
            .ok_or_else(|| OperationError::UnsupportedScriptType(spec.script_path.clone()))?;
        Ok(Self {
            script_path: spec.script_path,
            arguments: spec.arguments,
            kind,
        })
    }
}
