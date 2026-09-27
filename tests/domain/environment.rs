use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use buildpilot::domain::environment::{
    EnvironmentProblem, Need, VariableEdits, environments_among, is_environment, join_path,
    preselect, resolve, split_path, step_variables,
};

fn names(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn vars(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

/// The variables a child sees: `inherited` with `edits` applied, names upper-cased as Windows
/// compares them.
fn child_sees(inherited: &[(String, String)], edits: &VariableEdits) -> BTreeMap<String, String> {
    let mut seen: BTreeMap<String, String> = inherited
        .iter()
        .map(|(name, value)| (name.to_ascii_uppercase(), value.clone()))
        .collect();
    for name in &edits.remove {
        seen.remove(&name.to_ascii_uppercase());
    }
    for (name, value) in &edits.set {
        seen.insert(name.to_ascii_uppercase(), value.clone());
    }
    seen
}

/// A disk holding exactly `files`.
fn disk(files: &[&str]) -> impl Fn(&Path) -> bool {
    let files: BTreeSet<String> = files.iter().map(|file| (*file).to_owned()).collect();
    move |path: &Path| files.contains(&path.display().to_string())
}

// ENV-001: marker plus interpreter, whatever the folder is called.
#[test]
fn environments_are_found_by_their_files() {
    let is_file = disk(&[
        r"C:\src\app\venv\pyvenv.cfg",
        r"C:\src\app\venv\Scripts\python.exe",
        r"C:\src\app\.venv\pyvenv.cfg",
        r"C:\src\app\.venv\Scripts\python.exe",
        r"C:\src\app\tools\Scripts\python.exe",
        r"C:\src\app\half\pyvenv.cfg",
    ]);
    let folders = names(&["venv", ".venv", "tools", "half", "src"]);
    assert_eq!(
        environments_among(Path::new(r"C:\src\app"), &folders, &is_file),
        ["venv", ".venv"]
    );
    assert!(!is_environment(Path::new(r"C:\src\app\tools"), &is_file));
}

// ENV-002: the only one; else the one preferred name; else nothing.
#[test]
fn preselection() {
    assert_eq!(preselect(&names(&["env"])), Some("env".to_owned()));
    assert_eq!(
        preselect(&names(&["venv", "venv_smoke"])),
        Some("venv".to_owned())
    );
    assert_eq!(preselect(&names(&["a", ".venv"])), Some(".venv".to_owned()));
    assert_eq!(preselect(&names(&["venv", ".venv"])), None);
    assert_eq!(preselect(&names(&["a", "b"])), None);
    assert_eq!(preselect(&[]), None);
}

// ENV-003 to ENV-005.
#[test]
fn resolution() {
    let two = names(&["venv", "venv_smoke"]);
    let one = names(&["venv"]);
    assert_eq!(resolve(Need::None, Some("gone"), &[]), Ok(None));
    assert_eq!(
        resolve(Need::Required, Some("venv_smoke"), &two),
        Ok(Some("venv_smoke".to_owned()))
    );
    assert_eq!(
        resolve(Need::Optional, Some("gone"), &one),
        Err(EnvironmentProblem::NamedMissing("gone".to_owned()))
    );
    assert_eq!(
        resolve(Need::Required, None, &[]),
        Err(EnvironmentProblem::NoneFound)
    );
    assert_eq!(resolve(Need::Optional, None, &[]), Ok(None));
    assert_eq!(
        resolve(Need::Optional, None, &one),
        Ok(Some("venv".to_owned()))
    );
    assert_eq!(
        resolve(Need::Required, None, &two),
        Err(EnvironmentProblem::Several(two.clone()))
    );
}

// ENV-009, its acceptance case: an activation BuildPilot inherited is undone for every step.
#[test]
fn an_inherited_activation_is_undone() {
    let inherited = vars(&[
        ("VIRTUAL_ENV", r"C:\other\venv"),
        ("VIRTUAL_ENV_PROMPT", "venv"),
        ("Path", r"C:\other\venv\Scripts;C:\Windows"),
        ("_OLD_VIRTUAL_PATH", r"C:\Windows"),
        ("_OLD_VIRTUAL_PROMPT", "$P$G"),
        ("TEMP", r"C:\Temp"),
    ]);
    let is_file = disk(&[r"C:\other\venv\pyvenv.cfg"]);
    let activated = step_variables(&inherited, Some(Path::new(r"C:\src\app\venv")), &is_file);
    let seen = child_sees(&inherited, &activated);
    assert_eq!(seen["VIRTUAL_ENV"], r"C:\src\app\venv");
    assert_eq!(seen["VIRTUAL_ENV_PROMPT"], "venv");
    assert_eq!(seen["PATH"], r"C:\src\app\venv\Scripts;C:\Windows");
    assert_eq!(seen["_OLD_VIRTUAL_PATH"], r"C:\Windows");
    assert_eq!(seen["PYTHONUNBUFFERED"], "1");
    assert_eq!(seen["PYTHONIOENCODING"], "utf-8");
    assert!(!seen.contains_key("_OLD_VIRTUAL_PROMPT"));
    assert_eq!(seen["TEMP"], r"C:\Temp");

    let plain = child_sees(&inherited, &step_variables(&inherited, None, &is_file));
    assert_eq!(plain["PATH"], r"C:\Windows");
    for gone in [
        "VIRTUAL_ENV",
        "VIRTUAL_ENV_PROMPT",
        "_OLD_VIRTUAL_PATH",
        "PYTHONUNBUFFERED",
    ] {
        assert!(!plain.contains_key(gone), "{gone}");
    }
}

// ENV-010: an environment's Scripts folder left on PATH with no record is taken off; an
// ordinary folder called Scripts stays.
#[test]
fn stray_environment_folders_leave_path() {
    let inherited = vars(&[("PATH", r"C:\stray\venv\Scripts;C:\Tools\Scripts;C:\Windows")]);
    let is_file = disk(&[r"C:\stray\venv\pyvenv.cfg"]);
    let seen = child_sees(&inherited, &step_variables(&inherited, None, &is_file));
    assert_eq!(seen["PATH"], r"C:\Tools\Scripts;C:\Windows");
}

// ENV-006, ENV-009: PYTHONHOME is put back by deactivation, then set aside by activation.
#[test]
fn python_home_follows_deactivate_then_activate() {
    let no_disk = disk(&[]);
    let venv = Some(Path::new(r"C:\src\app\venv"));

    let restored = vars(&[("_OLD_VIRTUAL_PYTHONHOME", r"C:\Python")]);
    let seen = child_sees(&restored, &step_variables(&restored, None, &no_disk));
    assert_eq!(seen["PYTHONHOME"], r"C:\Python");
    assert!(!seen.contains_key("_OLD_VIRTUAL_PYTHONHOME"));

    let set = vars(&[("PYTHONHOME", r"C:\Python")]);
    let seen = child_sees(&set, &step_variables(&set, venv, &no_disk));
    assert!(!seen.contains_key("PYTHONHOME"));
    assert_eq!(seen["_OLD_VIRTUAL_PYTHONHOME"], r"C:\Python");

    let seen = child_sees(&[], &step_variables(&[], venv, &no_disk));
    assert!(!seen.contains_key("PYTHONHOME"));
    assert!(!seen.contains_key("_OLD_VIRTUAL_PYTHONHOME"));
    assert_eq!(seen["PATH"], r"C:\src\app\venv\Scripts");
}

// PATH is read as Windows reads it: quotes hold a separator and are not part of the entry.
#[test]
fn path_splits_and_joins() {
    let entries = split_path(r#"C:\a;;"C:\b;c";C:\d"#);
    assert_eq!(entries, [r"C:\a", r"C:\b;c", r"C:\d"]);
    assert_eq!(join_path(entries), r#"C:\a;"C:\b;c";C:\d"#);
    assert!(split_path("").is_empty());
}
