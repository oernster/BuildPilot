use std::path::{Path, PathBuf};

use buildpilot::application::{AppError, EditOutcome, IconStatus, Notice, StoreError};
use buildpilot::domain::deck::DeckError;
use buildpilot::domain::lifecycle::TransitionError;
use buildpilot::domain::operation::{IconRef, OperationError, OperationId};
use buildpilot::domain::preferences::{ThemeChoice, TrayLayout, WindowGeometry};

use super::fakes::DATA_FOLDER;
use super::support::{SCRIPT_A, SCRIPT_B, SCRIPT_C, add, draft, world};

// ICON-001: the conventional icon beside the script is used by default.
#[test]
fn draft_uses_the_discovered_icon() {
    let world = world();
    let icon = PathBuf::from(r"C:\src\alpha\assets\application-icon.png");
    world
        .state
        .borrow_mut()
        .discoverable
        .insert(PathBuf::from(r"C:\src\alpha"), icon.clone());
    let app = world.app();
    assert_eq!(draft(&app, SCRIPT_A).icon, IconRef::Discovered(icon));
    assert_eq!(draft(&app, SCRIPT_B).icon, IconRef::Placeholder);
    assert!(matches!(
        app.draft_for(Path::new(r"C:\src\x\notes.txt")),
        Err(AppError::Invalid(OperationError::UnsupportedScriptType(_)))
    ));
}

// CFG-001, ADD-006: add appends, assigns a fresh identity and saves.
#[test]
fn add_appends_and_saves() {
    let world = world();
    let mut app = world.app();
    let first = add(&mut app, SCRIPT_A);
    let second = add(&mut app, SCRIPT_A);
    assert_ne!(first, second);
    assert_eq!(world.save_count(), 2);
    let saved = &world.state.borrow().saves[1].0;
    assert_eq!(saved.len(), 2);
    assert_eq!(saved[1].id(), &second);
}

// ADD-005: an invalid spec is refused before anything is stored.
#[test]
fn invalid_add_stores_nothing() {
    let world = world();
    let mut app = world.app();
    let mut spec = draft(&app, SCRIPT_A);
    spec.name = String::new();
    assert_eq!(
        app.add(spec),
        Err(AppError::Invalid(OperationError::EmptyName))
    );
    assert!(app.deck().is_empty());
    assert_eq!(world.save_count(), 0);
}

// ICON-004, OQ-6: a chosen image is copied into the data folder.
#[test]
fn chosen_icon_is_imported() {
    let world = world();
    let mut app = world.app();
    let mut spec = draft(&app, SCRIPT_A);
    let source = PathBuf::from(r"C:\Pictures\rocket.png");
    spec.icon = IconRef::Chosen(source.clone());
    let id = app.add(spec).unwrap();
    let stored = PathBuf::from(DATA_FOLDER)
        .join("icons")
        .join(format!("{id}.png"));
    assert_eq!(world.state.borrow().imports, [(id.clone(), source)]);
    let config = app.deck().get(&id).unwrap().config();
    assert_eq!(config.icon(), &IconRef::Chosen(stored.clone()));
    assert_eq!(app.icon_status(&id), IconStatus::Ready(stored));
}

#[test]
fn failed_icon_import_adds_nothing() {
    let world = world();
    world.state.borrow_mut().import_error = Some("Access is denied.".to_owned());
    let mut app = world.app();
    let mut spec = draft(&app, SCRIPT_A);
    spec.icon = IconRef::Chosen(PathBuf::from(r"C:\Pictures\rocket.png"));
    let error = app.add(spec).unwrap_err();
    assert!(error.to_string().contains("rocket.png") && error.to_string().contains("denied"));
    assert!(app.deck().is_empty());
}

// EDIT-001: an edit keeps identity and position and saves.
#[test]
fn edit_replaces_in_place() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    let b = add(&mut app, SCRIPT_B);
    let mut spec = app.deck().get(&a).unwrap().config().to_spec();
    spec.name = "renamed".to_owned();
    assert_eq!(
        app.edit(&a, spec),
        Ok(EditOutcome {
            applies_next_run: false
        })
    );
    let ids: Vec<&OperationId> = app.deck().operations().iter().map(|op| op.id()).collect();
    assert_eq!(ids, [&a, &b]);
    assert_eq!(app.deck().get(&a).unwrap().config().name(), "renamed");
    assert_eq!(world.save_count(), 3);
}

#[test]
fn edit_refuses_unknown_or_invalid() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    let spec = draft(&app, SCRIPT_A);
    let ghost = OperationId::new("ghost").unwrap();
    assert_eq!(
        app.edit(&ghost, spec.clone()),
        Err(AppError::Deck(DeckError::NotFound(ghost)))
    );
    let mut blank = spec;
    blank.name = " ".to_owned();
    assert_eq!(
        app.edit(&a, blank),
        Err(AppError::Invalid(OperationError::EmptyName))
    );
}

// EDIT-002: while running, changes to what runs apply next run; the process is untouched.
#[test]
fn edit_during_run_does_not_touch_run() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    app.run(&a).unwrap();
    let mut renamed = app.deck().get(&a).unwrap().config().to_spec();
    renamed.name = "only a name".to_owned();
    assert!(!app.edit(&a, renamed.clone()).unwrap().applies_next_run);
    let mut argued = renamed;
    argued.steps[0].arguments = vec!["--release".to_owned()];
    assert!(app.edit(&a, argued).unwrap().applies_next_run);
    assert!(app.is_running(&a));
    assert_eq!(world.state.borrow().launches.len(), 1);
    assert!(world.state.borrow().stops.is_empty());
}

// OQ-6: an unchanged chosen icon is not copied again; dropping it deletes the copy.
#[test]
fn chosen_icon_lifecycle_through_edits() {
    let world = world();
    let mut app = world.app();
    let mut spec = draft(&app, SCRIPT_A);
    spec.icon = IconRef::Chosen(PathBuf::from(r"C:\Pictures\one.png"));
    let id = app.add(spec).unwrap();
    let kept = app.deck().get(&id).unwrap().config().to_spec();
    app.edit(&id, kept.clone()).unwrap();
    assert_eq!(world.state.borrow().imports.len(), 1);

    let mut replaced = kept;
    replaced.icon = IconRef::Chosen(PathBuf::from(r"C:\Pictures\two.png"));
    app.edit(&id, replaced.clone()).unwrap();
    assert_eq!(world.state.borrow().imports.len(), 2);

    let mut dropped = replaced;
    dropped.icon = IconRef::Placeholder;
    app.edit(&id, dropped).unwrap();
    assert_eq!(world.state.borrow().released, std::slice::from_ref(&id));
    assert_eq!(app.icon_status(&id), IconStatus::Placeholder);
}

// REM-002: removal takes the entry, its selection and its copied icon; nothing else.
#[test]
fn remove_leaves_script_on_disk() {
    let world = world();
    let mut app = world.app();
    let mut spec = draft(&app, SCRIPT_A);
    spec.icon = IconRef::Chosen(PathBuf::from(r"C:\Pictures\one.png"));
    let a = app.add(spec).unwrap();
    let b = add(&mut app, SCRIPT_B);
    app.select(&a).unwrap();
    app.toggle_checked(&a).unwrap();
    app.remove(&a).unwrap();
    assert_eq!(app.deck().operations().len(), 1);
    assert_eq!(app.selection().selected(), None);
    assert!(!app.selection().is_checked(&a));
    assert_eq!(world.state.borrow().released, std::slice::from_ref(&a));
    assert!(world.state.borrow().files.contains(Path::new(SCRIPT_A)));
    app.remove(&b).unwrap();
    assert_eq!(world.state.borrow().released.len(), 1);
    assert_eq!(app.remove(&a), Err(AppError::Deck(DeckError::NotFound(a))));
}

// REM-003
#[test]
fn remove_refused_while_running() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    app.run(&a).unwrap();
    let error = app.remove(&a).unwrap_err();
    assert_eq!(
        error,
        AppError::RemoveWhileRunning("alpha build".to_owned())
    );
    assert!(error.to_string().contains("Stop it before removing it"));
    assert_eq!(app.deck().operations().len(), 1);
}

// ROW-003, ROW-004, CFG-001: moves save only when something moved.
#[test]
fn reordering_saves_when_it_moves() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    let b = add(&mut app, SCRIPT_B);
    let c = add(&mut app, SCRIPT_C);
    let saves = world.save_count();
    app.move_to(&c, 0).unwrap();
    assert!(app.move_down(&c).unwrap());
    assert!(app.move_up(&b).unwrap());
    assert!(!app.move_up(&a).unwrap());
    assert!(!app.move_down(&c).unwrap());
    let order: Vec<&OperationId> = app.deck().operations().iter().map(|op| op.id()).collect();
    assert_eq!(order, [&a, &b, &c]);
    assert_eq!(world.save_count(), saves + 3);
    assert!(app.move_to(&a, 9).is_err());
}

// ROW-006, ROW-007
#[test]
fn select_and_check_known_rows_only() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    app.select(&a).unwrap();
    assert_eq!(app.selection().selected(), Some(&a));
    assert!(app.toggle_checked(&a).unwrap());
    let ghost = OperationId::new("ghost").unwrap();
    assert!(app.select(&ghost).is_err());
    assert!(app.toggle_checked(&ghost).is_err());
}

// ROW-004: only a known row moves.
#[test]
fn moving_an_unknown_row_is_refused() {
    let world = world();
    let mut app = world.app();
    add(&mut app, SCRIPT_A);
    let ghost = OperationId::new("ghost").unwrap();
    let not_found = Err(AppError::Deck(DeckError::NotFound(ghost.clone())));
    assert_eq!(app.move_up(&ghost), not_found);
    assert_eq!(app.move_down(&ghost), not_found);
}

// CFG-003: an identity already in the deck is refused rather than doubled.
#[test]
fn a_repeated_identity_is_refused() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    world.state.borrow_mut().next_id = 0;
    let spec = draft(&app, SCRIPT_B);
    assert_eq!(
        app.add(spec),
        Err(AppError::Deck(DeckError::DuplicateId(a)))
    );
    assert_eq!(app.deck().operations().len(), 1);
}

// EDIT-001: an edit whose chosen icon cannot be copied changes nothing.
#[test]
fn failed_icon_import_on_edit_changes_nothing() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    let before = app.deck().get(&a).unwrap().config().clone();
    world.state.borrow_mut().import_error = Some("Access is denied.".to_owned());
    let mut spec = before.to_spec();
    spec.name = "renamed".to_owned();
    spec.icon = IconRef::Chosen(PathBuf::from(r"C:\Pictures\rocket.png"));
    assert!(app.edit(&a, spec).is_err());
    assert_eq!(app.deck().get(&a).unwrap().config(), &before);
}

// UI-001, CFG-010
#[test]
fn preferences_are_saved() {
    let world = world();
    let mut app = world.app();
    app.set_theme(ThemeChoice::Light);
    let window = WindowGeometry {
        x: 10,
        y: 20,
        width: 1200,
        height: 800,
    };
    let tray = TrayLayout {
        expanded: true,
        height: Some(240),
    };
    app.set_layout(Some(window), tray);
    let saved = world.state.borrow().saves.last().unwrap().1;
    assert_eq!(saved.theme, ThemeChoice::Light);
    assert_eq!(saved.window, Some(window));
    assert_eq!(saved.tray, tray);
    assert_eq!(app.preferences().tray, tray);
}

// Wrapped domain errors keep the domain's wording.
#[test]
fn wrapped_errors_read_as_the_domain_wrote_them() {
    let ghost = OperationId::new("ghost").unwrap();
    let cases = [
        (
            AppError::Invalid(OperationError::EmptyName),
            OperationError::EmptyName.to_string(),
        ),
        (
            AppError::Deck(DeckError::NotFound(ghost.clone())),
            DeckError::NotFound(ghost).to_string(),
        ),
        (
            AppError::Transition(TransitionError::AlreadyRunning),
            TransitionError::AlreadyRunning.to_string(),
        ),
    ];
    for (wrapped, expected) in cases {
        assert_eq!(wrapped.to_string(), expected);
    }
}

// CFG-008: a failed save keeps the change in memory and tells the operator.
#[test]
fn failed_save_keeps_the_change_and_raises_a_notice() {
    let world = world();
    world.state.borrow_mut().save_error = Some("Access is denied.".to_owned());
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    assert!(app.deck().get(&a).is_some());
    let notices = app.take_notices();
    let [Notice::SaveFailed(StoreError { path, message })] = notices.as_slice() else {
        panic!("one save failure expected, got {notices:?}");
    };
    assert!(path.ends_with("buildpilot.json"));
    assert_eq!(message, "Access is denied.");
    let text = StoreError {
        path: path.clone(),
        message: message.clone(),
    }
    .to_string();
    assert!(text.contains("kept until BuildPilot closes"));
}
