use std::path::PathBuf;

use buildpilot::application::{AppError, LocateOutcome};
use buildpilot::domain::deck::DeckError;
use buildpilot::domain::operation::OperationId;

use super::fakes::DATA_FOLDER;
use super::support::{SCRIPT_A, add, world};

// NAV-001
#[test]
fn open_script_uses_the_association() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    app.open_script(&a).unwrap();
    assert_eq!(
        world.state.borrow().shell_calls,
        [("open", PathBuf::from(SCRIPT_A))]
    );
}

// NAV-003, NAV-004
#[test]
fn open_script_reports_missing_and_shell_failures() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    world.state.borrow_mut().shell_error = Some("No application is associated.".to_owned());
    let error = app.open_script(&a).unwrap_err();
    assert_eq!(
        error,
        AppError::Shell("No application is associated.".to_owned())
    );
    assert!(error.to_string().contains("No application is associated."));

    world.state.borrow_mut().files.clear();
    let missing = app.open_script(&a).unwrap_err();
    assert_eq!(missing, AppError::ScriptMissing(PathBuf::from(SCRIPT_A)));
    assert!(missing.to_string().contains("Edit"));
}

// NAV-002
#[test]
fn locate_reveals_the_script() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    assert_eq!(app.locate_script(&a), Ok(LocateOutcome::Revealed));
    assert_eq!(
        world.state.borrow().shell_calls,
        [("reveal", PathBuf::from(SCRIPT_A))]
    );
}

// NAV-003: a missing script opens the nearest folder that still exists.
#[test]
fn locate_missing_script_opens_nearest_folder() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    {
        let mut state = world.state.borrow_mut();
        state.files.clear();
        state.dirs.clear();
        state.dirs.insert(PathBuf::from(r"C:\src"));
    }
    assert_eq!(
        app.locate_script(&a),
        Ok(LocateOutcome::ScriptMissingFolderOpened {
            script: PathBuf::from(SCRIPT_A),
            folder: PathBuf::from(r"C:\src"),
        })
    );
    assert_eq!(
        world.state.borrow().shell_calls,
        [("open_folder", PathBuf::from(r"C:\src"))]
    );

    world.state.borrow_mut().dirs.clear();
    assert_eq!(
        app.locate_script(&a),
        Ok(LocateOutcome::ScriptMissingNothingOpened(PathBuf::from(
            SCRIPT_A
        )))
    );
}

#[test]
fn locate_passes_shell_failures_on() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    world.state.borrow_mut().shell_error = Some("Explorer is not running.".to_owned());
    assert!(matches!(app.locate_script(&a), Err(AppError::Shell(_))));
    world.state.borrow_mut().files.clear();
    assert!(matches!(app.locate_script(&a), Err(AppError::Shell(_))));
}

#[test]
fn navigation_refuses_unknown_operations() {
    let app = world().app();
    let ghost = OperationId::new("ghost").unwrap();
    let not_found = AppError::Deck(DeckError::NotFound(ghost.clone()));
    assert_eq!(app.open_script(&ghost), Err(not_found.clone()));
    assert_eq!(app.locate_script(&ghost), Err(not_found));
}

// UI-004
#[test]
fn data_folder_is_shown_and_opened() {
    let world = world();
    let app = world.app();
    assert_eq!(app.data_folder(), PathBuf::from(DATA_FOLDER));
    app.open_data_folder().unwrap();
    assert_eq!(
        world.state.borrow().shell_calls,
        [("open_folder", PathBuf::from(DATA_FOLDER))]
    );
}
