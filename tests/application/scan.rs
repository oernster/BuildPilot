//! Adding from a folder (SRS 3.20).

use std::path::{Path, PathBuf};

use buildpilot::application::{FolderScan, ScannedFolder};
use buildpilot::domain::operation::IconRef;
use buildpilot::domain::step::StepSpec;

use super::fakes::World;

const PARENT: &str = r"C:\dev";

/// A world holding, under `C:\dev`: a PowerShell project, a Python project with one
/// environment, the partial EDColonisationAsst shape and a folder with nothing recognised.
fn projects() -> World {
    [
        r"C:\dev\pigeonpost\build.ps1",
        r"C:\dev\stellody\buildexe.py",
        r"C:\dev\stellody\buildinstaller.py",
        r"C:\dev\edca\buildruntime.py",
        r"C:\dev\edca\buildinstaller.py",
        r"C:\dev\notes\readme.md",
    ]
    .into_iter()
    .fold(World::new(), World::with_script)
    .with_environment(r"C:\dev\stellody\venv")
}

fn steps(scripts: &[&str]) -> Vec<StepSpec> {
    scripts
        .iter()
        .map(|script| StepSpec::for_script(script.as_ref()))
        .collect()
}

fn several(scan: FolderScan) -> Vec<ScannedFolder> {
    let FolderScan::Several(found) = scan else {
        panic!("expected several: {scan:?}");
    };
    found
}

// SCAN-004: a project folder fills the dialog: steps in order, the folder, its name, its icon.
#[test]
fn a_project_folder_fills_one_operation() {
    let world = projects();
    world.state.borrow_mut().discoverable.insert(
        PathBuf::from(r"C:\dev\stellody"),
        PathBuf::from(r"C:\dev\stellody\assets\application-icon.png"),
    );
    let app = world.app();
    let FolderScan::One(found) = app.scan_folder(Path::new(r"C:\dev\stellody")) else {
        panic!("expected one");
    };
    assert_eq!(found.spec.name, "stellody");
    assert_eq!(
        found.spec.steps,
        steps(&[
            r"C:\dev\stellody\buildexe.py",
            r"C:\dev\stellody\buildinstaller.py"
        ])
    );
    assert_eq!(found.spec.working_dir, PathBuf::from(r"C:\dev\stellody"));
    assert_eq!(found.spec.environment, None);
    assert_eq!(
        found.spec.icon,
        IconRef::Discovered(PathBuf::from(
            r"C:\dev\stellody\assets\application-icon.png"
        ))
    );
    assert_eq!(found.warning, None);
    assert_eq!(found.unticked_because, None);
}

// SCAN-005, SCAN-009: a parent lists its projects by name, the partial one flagged.
#[test]
fn a_parent_lists_its_projects() {
    let app = projects().app();
    let found = several(app.scan_folder(Path::new(PARENT)));
    let names: Vec<&str> = found
        .iter()
        .map(|folder| folder.spec.name.as_str())
        .collect();
    assert_eq!(names, ["edca", "pigeonpost", "stellody"]);
    assert_eq!(
        found[0].spec.steps,
        steps(&[r"C:\dev\edca\buildinstaller.py"])
    );
    assert_eq!(
        found[0].warning.as_deref(),
        Some("buildexe.py was not found: only buildinstaller.py will run")
    );
    assert!(found.iter().all(|folder| folder.unticked_because.is_none()));
}

// SCAN-006: a project already on the deck starts unticked; so does one owing an environment choice.
#[test]
fn a_choice_owed_starts_unticked() {
    let world = projects()
        .with_environment(r"C:\dev\edca\env_a")
        .with_environment(r"C:\dev\edca\env_b");
    let mut app = world.app();
    let FolderScan::One(pigeonpost) = app.scan_folder(Path::new(r"C:\dev\pigeonpost")) else {
        panic!("expected one");
    };
    app.add(pigeonpost.spec).unwrap();
    let found = several(app.scan_folder(Path::new(PARENT)));
    let reasons: Vec<Option<&str>> = found
        .iter()
        .map(|folder| folder.unticked_because.as_deref())
        .collect();
    assert_eq!(
        reasons,
        [
            Some("Several environments: add this one on its own to choose"),
            Some("Already on the flight deck"),
            None
        ]
    );
}

// ENV-002 through SCAN-004: a preferred name among several is chosen for the operator.
#[test]
fn a_preferred_environment_is_preselected() {
    let world = projects().with_environment(r"C:\dev\stellody\venv_smoke");
    let app = world.app();
    let FolderScan::One(found) = app.scan_folder(Path::new(r"C:\dev\stellody")) else {
        panic!("expected one");
    };
    assert_eq!(found.spec.environment.as_deref(), Some("venv"));
    assert_eq!(found.unticked_because, None);
}

// SCAN-007: nothing recognised names the files looked for.
#[test]
fn nothing_recognised_names_what_was_looked_for() {
    let app = projects().app();
    assert_eq!(
        app.scan_folder(Path::new(r"C:\dev\notes")),
        FolderScan::Nothing(vec!["build.ps1", "buildexe.py", "buildinstaller.py"])
    );
}
