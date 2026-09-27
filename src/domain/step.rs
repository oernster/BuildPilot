//! One step of an operation: a script with its arguments (SRS 3.17).

use std::path::{Path, PathBuf};

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

/// A step after validation. Construct with `Step::try_from`. Its type is not judged here: the
/// host table does that. A row removed after the step was saved must not stop it loading
/// (HOST-001).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    script_path: PathBuf,
    arguments: Vec<String>,
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
    /// The fields back as an editable spec.
    pub fn to_spec(&self) -> StepSpec {
        StepSpec {
            script_path: self.script_path.clone(),
            arguments: self.arguments.clone(),
        }
    }
}

/// The dialog's steps with one of them selected, whose fields the dialog shows (STEP-008).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepList {
    steps: Vec<StepSpec>,
    selected: usize,
}

impl StepList {
    /// `steps` with the first selected; one empty step when there are none.
    pub fn new(mut steps: Vec<StepSpec>) -> Self {
        if steps.is_empty() {
            steps.push(StepSpec::default());
        }
        Self { steps, selected: 0 }
    }
    /// Every step in order.
    pub fn steps(&self) -> &[StepSpec] {
        &self.steps
    }
    /// The selected step's position, from zero.
    pub fn selected(&self) -> usize {
        self.selected
    }
    /// The selected step.
    pub fn current(&self) -> &StepSpec {
        &self.steps[self.selected]
    }
    /// Replaces the selected step with what the operator typed.
    pub fn set_current(&mut self, step: StepSpec) {
        self.steps[self.selected] = step;
    }
    /// Selects step `index`; false, changing nothing, when there is no such step.
    pub fn select(&mut self, index: usize) -> bool {
        let exists = index < self.steps.len();
        if exists {
            self.selected = index;
        }
        exists
    }
    /// Adds `step` as the last and selects it.
    pub fn add(&mut self, step: StepSpec) {
        self.steps.push(step);
        self.selected = self.steps.len() - 1;
    }
    /// Removes the selected step and selects the one now in its place, else the one before;
    /// refused (false) for the only step.
    pub fn remove(&mut self) -> bool {
        if self.steps.len() == 1 {
            return false;
        }
        self.steps.remove(self.selected);
        self.selected = self.selected.min(self.steps.len() - 1);
        true
    }
    /// Swaps the selected step with the one before, keeping it selected; false at the top.
    pub fn move_up(&mut self) -> bool {
        if self.selected == 0 {
            return false;
        }
        self.steps.swap(self.selected, self.selected - 1);
        self.selected -= 1;
        true
    }
    /// Swaps the selected step with the one after, keeping it selected; false at the bottom.
    pub fn move_down(&mut self) -> bool {
        if self.selected + 1 == self.steps.len() {
            return false;
        }
        self.steps.swap(self.selected, self.selected + 1);
        self.selected += 1;
        true
    }
    /// Each step as the list shows it: `1. buildexe.py`; `2. (no script)` when empty.
    pub fn labels(&self) -> Vec<String> {
        self.steps
            .iter()
            .enumerate()
            .map(|(index, step)| {
                let name = step.script_path.file_name().map_or_else(
                    || NO_SCRIPT.to_owned(),
                    |name| name.to_string_lossy().into_owned(),
                );
                format!("{}. {name}", index + 1)
            })
            .collect()
    }
}

/// How a step with no script yet is listed.
const NO_SCRIPT: &str = "(no script)";

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
        Ok(Self {
            script_path: spec.script_path,
            arguments: spec.arguments,
        })
    }
}
