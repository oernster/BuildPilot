use std::path::{Path, PathBuf};

use buildpilot::application::AppError;
use buildpilot::domain::deck::DeckError;
use buildpilot::domain::installer::InstallerBlock;
use buildpilot::domain::operation::{Operation, OperationConfig, OperationId};

use super::fakes::World;
use super::support::{SCRIPT_A, add, draft, exited, world};

/// Where PKG-002 finds SCRIPT_A's installer.
const DEFAULT: &str = r"C:\src\alpha\dist-installer\alphaSetup.exe";
/// Where the fallback folder holds one.
const FALLBACK: &str = r"C:\src\alpha\dist\AlphaSetup.exe";
/// One the operator set in Edit.
const SET: &str = r"D:\out\AlphaInstaller.exe";

fn with_file(world: World, file: &str) -> World {
    world.state.borrow_mut().files.insert(PathBuf::from(file));
    world
}

fn opened(world: &World) -> Vec<(&'static str, PathBuf)> {
    world.state.borrow().shell_calls.clone()
}

// PKG-002, PKG-005: the default is found, then started as Explorer would and logged.
#[test]
fn the_default_installer_launches() {
    let world = with_file(world(), DEFAULT);
    let mut app = world.app();
    let id = add(&mut app, SCRIPT_A);
    assert_eq!(app.installer(&id), Some(Path::new(DEFAULT)));
    assert_eq!(app.installer_block(&id), None);
    app.launch_installer(&id).unwrap();
    assert_eq!(opened(&world), [("open", PathBuf::from(DEFAULT))]);
    assert!(
        world
            .logged()
            .contains(&format!("Launched the installer {DEFAULT}"))
    );
}

// PKG-002: dist is the fallback; the dialog is offered the same answer.
#[test]
fn dist_is_the_fallback() {
    let world = with_file(world(), FALLBACK);
    let mut app = world.app();
    let id = add(&mut app, SCRIPT_A);
    assert_eq!(app.installer(&id), Some(Path::new(FALLBACK)));
    assert_eq!(
        app.default_installer(Path::new(r"C:\src\alpha")),
        Some(PathBuf::from(FALLBACK))
    );
}

// PKG-003: with none found and none set, Launch installer is refused and nothing opens.
#[test]
fn nothing_found_is_refused() {
    let world = world();
    let mut app = world.app();
    let id = add(&mut app, SCRIPT_A);
    assert_eq!(app.installer(&id), None);
    assert_eq!(app.installer_block(&id), Some(InstallerBlock::NotFound));
    let refused = app.launch_installer(&id).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "The installer of alpha build was not launched: no installer found; set one in Edit."
    );
    assert!(opened(&world).is_empty());
}

// PKG-004: a run holds it back; a stop mid-run keeps it back until a later run ends.
#[test]
fn a_run_and_a_stop_hold_it_back() {
    let world = with_file(world(), DEFAULT);
    let mut app = world.app();
    let id = add(&mut app, SCRIPT_A);
    app.run(&id).unwrap();
    assert_eq!(app.installer_block(&id), Some(InstallerBlock::Running));
    assert!(matches!(
        app.launch_installer(&id),
        Err(AppError::Installer {
            block: InstallerBlock::Running,
            ..
        })
    ));
    app.stop(&id).unwrap();
    app.handle_event(exited(&world.launch_key(0), 1));
    assert_eq!(app.installer_block(&id), Some(InstallerBlock::Stopped));
    app.run(&id).unwrap();
    app.handle_event(exited(&world.launch_key(1), 0));
    assert_eq!(app.installer_block(&id), None);
    assert!(opened(&world).is_empty());
}

// PKG-004: a failed run does not hold it back; only a stop does.
#[test]
fn a_failed_run_leaves_it_available() {
    let world = with_file(world(), DEFAULT);
    let mut app = world.app();
    let id = add(&mut app, SCRIPT_A);
    app.run(&id).unwrap();
    app.handle_event(exited(&world.launch_key(0), 1));
    assert_eq!(app.installer_block(&id), None);
}

// PKG-002: an installer the run wrote is found when it ends; one built elsewhere on select.
#[test]
fn a_new_installer_is_found_after_a_run_or_on_select() {
    let world = world();
    let mut app = world.app();
    let id = add(&mut app, SCRIPT_A);
    app.run(&id).unwrap();
    world
        .state
        .borrow_mut()
        .files
        .insert(PathBuf::from(DEFAULT));
    assert_eq!(app.installer(&id), None);
    app.handle_event(exited(&world.launch_key(0), 0));
    assert_eq!(app.installer(&id), Some(Path::new(DEFAULT)));

    world.state.borrow_mut().files.remove(Path::new(DEFAULT));
    app.select(&id).unwrap();
    assert_eq!(app.installer(&id), None);
}

// PKG-001: a set installer wins over the default; set but missing, nothing is used.
#[test]
fn a_set_installer_is_used_and_never_second_guessed() {
    let world = with_file(with_file(world(), DEFAULT), SET);
    let mut app = world.app();
    let id = add(&mut app, SCRIPT_A);
    let mut spec = draft(&app, SCRIPT_A);
    spec.installer = Some(PathBuf::from(SET));
    app.edit(&id, spec.clone()).unwrap();
    assert_eq!(app.installer(&id), Some(Path::new(SET)));

    world.state.borrow_mut().files.remove(Path::new(SET));
    assert_eq!(
        app.launch_installer(&id).unwrap_err(),
        AppError::Installer {
            name: "alpha build".to_owned(),
            block: InstallerBlock::NotFound,
        }
    );
    assert_eq!(app.installer(&id), None);
}

// PKG-002: operations loaded at start are looked for too.
#[test]
fn loaded_operations_are_looked_for_at_start() {
    let world = with_file(world(), DEFAULT);
    let spec = draft(&world.app(), SCRIPT_A);
    let id = OperationId::new("loaded").unwrap();
    world.state.borrow_mut().to_load.operations = vec![Operation::new(
        id.clone(),
        OperationConfig::try_from(spec).unwrap(),
    )];
    let app = world.app();
    assert_eq!(app.installer(&id), Some(Path::new(DEFAULT)));
}

// PKG-005: Windows refusing to start it is said in its words; a removed row is forgotten.
#[test]
fn refusals_name_the_cause() {
    let world = with_file(world(), DEFAULT);
    let mut app = world.app();
    let id = add(&mut app, SCRIPT_A);
    world.state.borrow_mut().shell_error = Some("The operation was cancelled".to_owned());
    assert_eq!(
        app.launch_installer(&id).unwrap_err(),
        AppError::Shell("The operation was cancelled".to_owned())
    );
    app.remove(&id).unwrap();
    assert_eq!(app.installer(&id), None);
    assert_eq!(
        app.launch_installer(&id).unwrap_err(),
        AppError::Deck(DeckError::NotFound(id))
    );
}
