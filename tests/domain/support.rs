//! Builders shared by the domain tests, written once.

use std::path::PathBuf;

use buildpilot::domain::operation::{
    IconRef, Operation, OperationConfig, OperationId, OperationSpec,
};
use buildpilot::domain::step::StepSpec;

/// A valid spec with one step, `script`, run in its folder with no arguments.
pub fn spec(script: &str) -> OperationSpec {
    let script = PathBuf::from(script);
    OperationSpec {
        name: "app build".to_owned(),
        working_dir: script
            .parent()
            .expect("test script has a folder")
            .to_path_buf(),
        steps: vec![StepSpec::for_script(&script)],
        environment: None,
        icon: IconRef::Placeholder,
    }
}

/// `spec(first)` with a further step for each of `later`, in order.
pub fn spec_with_steps(first: &str, later: &[&str]) -> OperationSpec {
    let mut spec = spec(first);
    spec.steps.extend(
        later
            .iter()
            .map(|script| StepSpec::for_script(script.as_ref())),
    );
    spec
}

/// A validated configuration for `script`.
pub fn config(script: &str) -> OperationConfig {
    OperationConfig::try_from(spec(script)).expect("test spec is valid")
}

/// An identity from `value`.
pub fn id(value: &str) -> OperationId {
    OperationId::new(value).expect("test id is not blank")
}

/// An operation with identity `value` running `C:\src\<value>\build.ps1`.
pub fn operation(value: &str) -> Operation {
    Operation::new(id(value), config(&format!(r"C:\src\{value}\build.ps1")))
}
