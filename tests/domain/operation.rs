use std::path::{Path, PathBuf};

use buildpilot::domain::environment::Need;
use buildpilot::domain::host::HostTable;
use buildpilot::domain::operation::{
    IconRef, Operation, OperationConfig, OperationError, OperationId, OperationSpec,
    draft_for_script, format_argument_lines, parse_argument_lines,
};
use buildpilot::domain::step::StepSpec;

use super::support::{config, hosts, id, spec, spec_with_steps};

// ADD-002
#[test]
fn working_dir_defaults_to_script_parent() {
    let draft =
        draft_for_script(Path::new(r"C:\src\app\build.ps1"), &HostTable::default()).unwrap();
    assert_eq!(draft.working_dir, PathBuf::from(r"C:\src\app"));
}

// ADD-003, OQ-5
#[test]
fn name_defaults_from_folder_and_stem() {
    let draft = draft_for_script(
        Path::new(r"C:\src\pigeonpost\build.ps1"),
        &HostTable::default(),
    )
    .unwrap();
    assert_eq!(draft.name, "pigeonpost build");
}

// ADD-003: a script at a drive root has no folder name to lead with.
#[test]
fn name_at_drive_root_is_the_stem() {
    let draft = draft_for_script(Path::new(r"C:\build.cmd"), &HostTable::default()).unwrap();
    assert_eq!(draft.name, "build");
    assert_eq!(draft.working_dir, PathBuf::from(r"C:\"));
}

// ADD-004, ICON-002
#[test]
fn draft_has_no_arguments_and_the_placeholder_icon() {
    let draft =
        draft_for_script(Path::new(r"C:\src\app\build.ps1"), &HostTable::default()).unwrap();
    assert_eq!(
        draft.steps,
        [StepSpec::for_script(Path::new(r"C:\src\app\build.ps1"))]
    );
    assert!(draft.steps[0].arguments.is_empty());
    assert_eq!(draft.environment, None);
    assert_eq!(draft.icon, IconRef::Placeholder);
}

// LCH-001: an unsupported type is refused at Add, naming the supported ones.
#[test]
fn draft_refuses_unsupported_type() {
    let error =
        draft_for_script(Path::new(r"C:\src\app\build.sh"), &HostTable::default()).unwrap_err();
    assert_eq!(
        error,
        OperationError::UnsupportedScriptType {
            path: PathBuf::from(r"C:\src\app\build.sh"),
            supported: ".ps1, .bat, .cmd, .exe, .com, .py".to_owned(),
        }
    );
    assert!(
        error
            .to_string()
            .contains(".ps1, .bat, .cmd, .exe, .com, .py")
    );
}

#[test]
fn draft_refuses_a_bare_file_name() {
    let error = draft_for_script(Path::new("build.ps1"), &HostTable::default()).unwrap_err();
    assert_eq!(
        error,
        OperationError::NoParentDirectory(PathBuf::from("build.ps1"))
    );
}

// ADD-006: the same script may be added twice.
#[test]
fn same_script_makes_two_distinct_operations() {
    let first = Operation::new(id("a"), config(r"C:\src\app\build.ps1"));
    let second = Operation::new(id("b"), config(r"C:\src\app\build.ps1"));
    assert_ne!(first, second);
    assert_eq!(first.config(), second.config());
}

#[test]
fn valid_spec_round_trips_through_config() {
    let mut original = spec_with_steps(r"C:\src\app\buildexe.py", &[r"C:\src\app\setup.ps1"]);
    original.steps[0].arguments = vec!["--release".to_owned()];
    original.environment = Some("venv".to_owned());
    original.icon = IconRef::Chosen(PathBuf::from(r"C:\data\icons\a.png"));
    let validated = OperationConfig::try_from(original.clone()).unwrap();
    assert_eq!(validated.to_spec(), original);
    assert_eq!(validated.name(), "app build");
    assert_eq!(validated.steps().len(), 2);
    assert_eq!(
        validated.first_step().script_path(),
        Path::new(r"C:\src\app\buildexe.py")
    );
    assert_eq!(validated.first_step().arguments(), ["--release"]);
    assert_eq!(
        validated.steps()[1].script_path(),
        Path::new(r"C:\src\app\setup.ps1")
    );
    assert_eq!(validated.environment(), Some("venv"));
}

// STEP-001: at least one step; each is validated, the first bad one reported.
#[test]
fn steps_are_required_and_each_validated() {
    let mut none = spec(r"C:\src\app\build.ps1");
    none.steps.clear();
    assert_eq!(
        OperationConfig::try_from(none).unwrap_err(),
        OperationError::EmptyScriptPath
    );
    let mut bad_second = spec_with_steps(r"C:\src\app\build.ps1", &[r"C:\src\app\x.ps1"]);
    bad_second.steps[1].script_path = PathBuf::from("x.ps1");
    assert_eq!(
        OperationConfig::try_from(bad_second).unwrap_err(),
        OperationError::ScriptPathNotAbsolute(PathBuf::from("x.ps1"))
    );
}

// LCH-001, HOST-001: a type nothing runs still loads, so removing a host never loses an
// operation; Add and Edit refuse it through `check_hosts`, naming the first such step.
#[test]
fn unsupported_type_loads_but_is_refused_by_check_hosts() {
    let config = OperationConfig::try_from(spec_with_steps(
        r"C:\src\app\build.ps1",
        &[r"C:\src\app\notes.txt", r"C:\src\app\run.rb"],
    ))
    .unwrap();
    assert_eq!(
        config.check_hosts(&HostTable::default()).unwrap_err(),
        OperationError::UnsupportedScriptType {
            path: PathBuf::from(r"C:\src\app\notes.txt"),
            supported: ".ps1, .bat, .cmd, .exe, .com, .py".to_owned(),
        }
    );
    let supported = hosts(&[("txt", "notepad.exe"), ("rb", "ruby.exe")]);
    assert_eq!(config.check_hosts(&supported), Ok(()));
}

// ENV-002: a blank environment name is no choice at all.
#[test]
fn blank_environment_is_none() {
    let mut blank = spec(r"C:\src\app\build.py");
    blank.environment = Some("  ".to_owned());
    assert_eq!(
        OperationConfig::try_from(blank).unwrap().environment(),
        None
    );
}

// ENV-004, OQ-18: the operation needs what its most demanding step needs.
#[test]
fn environment_need_is_the_greatest_step_need() {
    let need = |first: &str, later: &[&str]| {
        OperationConfig::try_from(spec_with_steps(first, later))
            .unwrap()
            .environment_need(&HostTable::default())
    };
    assert_eq!(need(r"C:\a\build.cmd", &[r"C:\a\x.exe"]), Need::None);
    assert_eq!(need(r"C:\a\build.cmd", &[r"C:\a\x.ps1"]), Need::Optional);
    assert_eq!(
        need(r"C:\a\x.ps1", &[r"C:\a\x.py", r"C:\a\y.bat"]),
        Need::Required
    );
}

// ADD-005: the name is trimmed; a blank one is refused.
#[test]
fn name_is_trimmed_and_blank_refused() {
    let mut padded = spec(r"C:\src\app\build.ps1");
    padded.name = "  app  ".to_owned();
    assert_eq!(OperationConfig::try_from(padded).unwrap().name(), "app");

    let mut blank = spec(r"C:\src\app\build.ps1");
    blank.name = "   ".to_owned();
    assert_eq!(
        OperationConfig::try_from(blank).unwrap_err(),
        OperationError::EmptyName
    );
}

/// One way to break a spec, the error it must produce and words the message must contain.
type BrokenSpecCase = (fn(&mut OperationSpec), OperationError, &'static str);

// ADD-005: each refusal names its field.
#[test]
fn each_invalid_field_is_named() {
    let cases: [BrokenSpecCase; 4] = [
        (
            |s| s.steps[0].script_path = PathBuf::new(),
            OperationError::EmptyScriptPath,
            "Script is missing",
        ),
        (
            |s| s.steps[0].script_path = PathBuf::from("build.ps1"),
            OperationError::ScriptPathNotAbsolute(PathBuf::from("build.ps1")),
            "relative",
        ),
        (
            |s| s.working_dir = PathBuf::new(),
            OperationError::EmptyWorkingDir,
            "Working directory is missing",
        ),
        (
            |s| s.working_dir = PathBuf::from("app"),
            OperationError::WorkingDirNotAbsolute(PathBuf::from("app")),
            "Working directory app is relative",
        ),
    ];
    for (break_it, expected, wording) in cases {
        let mut broken = spec(r"C:\src\app\build.ps1");
        break_it(&mut broken);
        let error = OperationConfig::try_from(broken).unwrap_err();
        assert_eq!(error, expected);
        assert!(error.to_string().contains(wording), "{error}");
    }
}

#[test]
fn every_error_has_a_message() {
    let errors = [
        OperationError::EmptyId,
        OperationError::EmptyName,
        OperationError::NoParentDirectory(PathBuf::from("x.ps1")),
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
    }
}

#[test]
fn blank_id_is_refused() {
    assert_eq!(OperationId::new(" ").unwrap_err(), OperationError::EmptyId);
    let id = OperationId::new("4f1c").unwrap();
    assert_eq!(id.as_str(), "4f1c");
    assert_eq!(id.to_string(), "4f1c");
}

// EDIT-002: only steps, folder and environment change what a run launches.
#[test]
fn execution_difference_ignores_name_and_icon() {
    let base = config(r"C:\src\app\build.ps1");
    let mut renamed = base.to_spec();
    renamed.name = "other".to_owned();
    let renamed = OperationConfig::try_from(renamed).unwrap();
    assert!(!base.differs_in_execution(&renamed));
    assert!(!base.differs_in_execution(&base.with_icon(IconRef::Placeholder)));

    let mut moved = base.to_spec();
    moved.working_dir = PathBuf::from(r"C:\elsewhere");
    assert!(base.differs_in_execution(&OperationConfig::try_from(moved).unwrap()));

    let mut argued = base.to_spec();
    argued.steps[0].arguments = vec!["-x".to_owned()];
    assert!(base.differs_in_execution(&OperationConfig::try_from(argued).unwrap()));

    let mut rescripted = base.to_spec();
    rescripted.steps[0].script_path = PathBuf::from(r"C:\src\app\other.ps1");
    assert!(base.differs_in_execution(&OperationConfig::try_from(rescripted).unwrap()));

    let mut stepped = base.to_spec();
    stepped
        .steps
        .push(StepSpec::for_script(Path::new(r"C:\src\app\two.ps1")));
    assert!(base.differs_in_execution(&OperationConfig::try_from(stepped).unwrap()));

    let mut chosen = base.to_spec();
    chosen.environment = Some("venv".to_owned());
    assert!(base.differs_in_execution(&OperationConfig::try_from(chosen).unwrap()));
}

#[test]
fn with_icon_changes_only_the_icon() {
    let base = config(r"C:\src\app\build.ps1");
    let icon = IconRef::Discovered(PathBuf::from(r"C:\src\app\assets\application-icon.png"));
    let changed = base.with_icon(icon.clone());
    assert_eq!(changed.icon(), &icon);
    assert_eq!(changed.steps(), base.steps());
}

#[test]
fn set_config_keeps_the_identity() {
    let mut operation = Operation::new(id("a"), config(r"C:\src\app\build.ps1"));
    let replacement = config(r"C:\src\other\build.ps1");
    operation.set_config(replacement.clone());
    assert_eq!(operation.id(), &id("a"));
    assert_eq!(operation.config(), &replacement);
}

// OQ-8: one argument per line, trimmed, blanks dropped.
#[test]
fn argument_lines_parse_and_format() {
    let parsed = parse_argument_lines("  --release \r\n\n-Target \"x y\"\n   \n");
    assert_eq!(parsed, ["--release", "-Target \"x y\""]);
    assert_eq!(format_argument_lines(&parsed), "--release\n-Target \"x y\"");
}
