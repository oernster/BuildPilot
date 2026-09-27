//! Builders shared by the domain tests, written once.

use std::path::PathBuf;

use buildpilot::domain::operation::{
    IconRef, Operation, OperationConfig, OperationId, OperationSpec,
};

/// A valid spec for `script`, run in its folder with no arguments.
pub fn spec(script: &str) -> OperationSpec {
    let script = PathBuf::from(script);
    OperationSpec {
        name: "app build".to_owned(),
        working_dir: script
            .parent()
            .expect("test script has a folder")
            .to_path_buf(),
        script_path: script,
        arguments: Vec::new(),
        icon: IconRef::Placeholder,
    }
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
