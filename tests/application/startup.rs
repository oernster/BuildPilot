use std::path::PathBuf;

use buildpilot::application::ports::LoadedConfig;
use buildpilot::application::{IconStatus, LoadProblem, Notice};
use buildpilot::domain::lifecycle::RunState;
use buildpilot::domain::operation::{IconRef, Operation, OperationConfig, OperationId};
use buildpilot::domain::preferences::{Preferences, ThemeChoice};

use super::fakes::World;
use super::support::{SCRIPT_A, SCRIPT_B};

fn stored(id: &str, script: &str, icon: IconRef) -> Operation {
    let mut spec = buildpilot::domain::operation::draft_for_script(script.as_ref()).unwrap();
    spec.icon = icon;
    Operation::new(
        OperationId::new(id).unwrap(),
        OperationConfig::try_from(spec).unwrap(),
    )
}

// CFG-001, CFG-004: stored operations load in order, every one idle.
#[test]
fn loads_operations_in_order_and_idle() {
    let world = World::new();
    world.state.borrow_mut().to_load = LoadedConfig {
        operations: vec![
            stored("b", SCRIPT_B, IconRef::Placeholder),
            stored("a", SCRIPT_A, IconRef::Placeholder),
        ],
        preferences: Preferences {
            theme: ThemeChoice::Dark,
            ..Preferences::default()
        },
        problems: Vec::new(),
    };
    let app = world.app();
    let order: Vec<&str> = app
        .deck()
        .operations()
        .iter()
        .map(|op| op.id().as_str())
        .collect();
    assert_eq!(order, ["b", "a"]);
    for operation in app.deck().operations() {
        assert_eq!(app.run_state(operation.id()), &RunState::Idle);
    }
    assert_eq!(app.preferences().theme, ThemeChoice::Dark);
    assert_eq!(world.save_count(), 0);
}

// CFG-005, CFG-006, CFG-009: load problems reach the operator; the rest still loads.
#[test]
fn load_problems_become_notices() {
    let world = World::new();
    let set_aside = PathBuf::from(r"C:\data\buildpilot.json.unreadable");
    world.state.borrow_mut().to_load = LoadedConfig {
        operations: vec![stored("a", SCRIPT_A, IconRef::Placeholder)],
        preferences: Preferences::default(),
        problems: vec![
            LoadProblem::UnreadableEntries(1),
            LoadProblem::SetAside(set_aside.clone()),
            LoadProblem::NewerSchema,
        ],
    };
    let mut app = world.app();
    assert_eq!(app.deck().operations().len(), 1);
    assert_eq!(
        app.take_notices(),
        [
            Notice::Load(LoadProblem::UnreadableEntries(1)),
            Notice::Load(LoadProblem::SetAside(set_aside)),
            Notice::Load(LoadProblem::NewerSchema),
        ]
    );
    assert!(app.take_notices().is_empty());
}

// One bad entry never blocks the rest: a repeated identity drops only the repeat.
#[test]
fn a_repeated_identity_is_dropped_with_a_notice() {
    let world = World::new();
    world.state.borrow_mut().to_load.operations = vec![
        stored("a", SCRIPT_A, IconRef::Placeholder),
        stored("a", SCRIPT_B, IconRef::Placeholder),
    ];
    let mut app = world.app();
    assert_eq!(app.deck().operations().len(), 1);
    assert_eq!(
        app.take_notices(),
        [Notice::DuplicateDropped(OperationId::new("a").unwrap())]
    );
}

// ICON-002, ICON-005
#[test]
fn icon_status_reflects_what_can_be_shown() {
    let world = World::new();
    let present = PathBuf::from(r"C:\src\alpha\assets\application-icon.png");
    let gone = PathBuf::from(r"C:\data\icons\b.png");
    world
        .state
        .borrow_mut()
        .readable_icons
        .insert(present.clone());
    world.state.borrow_mut().to_load.operations = vec![
        stored("a", SCRIPT_A, IconRef::Discovered(present.clone())),
        stored("b", SCRIPT_B, IconRef::Chosen(gone.clone())),
        stored("c", SCRIPT_A, IconRef::Placeholder),
    ];
    let app = world.app();
    let id = |value: &str| OperationId::new(value).unwrap();
    assert_eq!(app.icon_status(&id("a")), IconStatus::Ready(present));
    assert_eq!(app.icon_status(&id("b")), IconStatus::Missing(gone));
    assert_eq!(app.icon_status(&id("c")), IconStatus::Placeholder);
    assert_eq!(app.icon_status(&id("zz")), IconStatus::Placeholder);
}
