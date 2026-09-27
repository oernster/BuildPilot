//! Several runs at once: one build per folder (LCH-010) and running the ticked rows (LCH-011).

use std::path::PathBuf;

use buildpilot::application::{App, AppError};
use buildpilot::domain::lifecycle::{RunState, TransitionError};
use buildpilot::domain::operation::OperationId;
use buildpilot::domain::output::Stream;

use super::support::{SCRIPT_A, SCRIPT_B, SCRIPT_C, add, draft, exited, line, output_texts, world};

// ROW-005: reordering while builds run neither stops, restarts nor re-attributes any of them;
// output arriving after the move lands under the operation that produced it.
#[test]
fn reorder_during_run_keeps_run() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    let b = add(&mut app, SCRIPT_B);
    let c = add(&mut app, SCRIPT_C);
    app.run(&a).unwrap();
    app.run(&b).unwrap();
    let (run_a, run_b) = (world.launch_key(0), world.launch_key(1));
    app.handle_event(line(&run_a, Stream::Stdout, "a before"));

    app.move_to(&a, 2).unwrap();
    assert!(app.move_down(&b).unwrap());
    let order: Vec<&OperationId> = app.deck().operations().iter().map(|op| op.id()).collect();
    assert_eq!(order, [&c, &b, &a]);

    app.handle_event(line(&run_a, Stream::Stdout, "a after"));
    app.handle_event(line(&run_b, Stream::Stderr, "b after"));
    assert!(app.is_running(&a) && app.is_running(&b));
    assert!(!app.is_running(&c));
    assert_eq!(world.state.borrow().launches.len(), 2, "nothing restarted");
    assert!(world.state.borrow().stops.is_empty(), "nothing stopped");
    assert!(output_texts(&app, &a).ends_with(&["a before".to_owned(), "a after".to_owned()]));
    assert!(output_texts(&app, &b).ends_with(&["b after".to_owned()]));
    assert!(
        !output_texts(&app, &b)
            .iter()
            .any(|text| text.starts_with("a "))
    );

    app.handle_event(exited(&run_a, 0));
    assert_eq!(app.run_state(&a), &RunState::Succeeded);
    assert!(app.is_running(&b));
}

/// Who holds operation `id`'s folder, asked as its row asks.
fn busy(app: &App, id: &OperationId) -> Option<String> {
    app.folder_busy(id, app.deck().get(id).unwrap().config())
}

// LCH-010: while one operation builds in a folder, another in the same folder is refused, saying
// which holds it; once the first ends, the second runs.
#[test]
fn one_build_per_folder() {
    let world = world();
    let mut app = world.app();
    let first = add(&mut app, SCRIPT_A);
    let second = add(&mut app, SCRIPT_A);
    let elsewhere = add(&mut app, SCRIPT_B);
    app.run(&first).unwrap();
    assert_eq!(busy(&app, &second).as_deref(), Some("alpha build"));
    let refused = app.run(&second).unwrap_err();
    assert_eq!(
        refused,
        AppError::FolderInUse {
            folder: PathBuf::from(r"C:\src\alpha"),
            by: "alpha build".to_owned(),
        }
    );
    assert!(
        refused
            .to_string()
            .contains(r"alpha build is building in C:\src\alpha")
    );
    assert_eq!(app.run_state(&second), &RunState::Idle);
    assert_eq!(busy(&app, &first), None, "a run never blocks itself");
    app.run(&elsewhere).unwrap();
    app.handle_event(exited(&world.launch_key(0), 0));
    assert_eq!(busy(&app, &second), None);
    app.run(&second).unwrap();
}

// LCH-010: the folder is compared as Windows compares it; the running build is judged by
// the folder its run started in, not by an edit made since.
#[test]
fn the_folder_is_the_one_the_run_started_in() {
    let world = world();
    let mut app = world.app();
    let first = add(&mut app, SCRIPT_A);
    let mut shouting = draft(&app, SCRIPT_A);
    shouting.working_dir = PathBuf::from(r"c:\SRC\Alpha\.\");
    let second = app.add(shouting).unwrap();
    app.run(&first).unwrap();
    assert!(busy(&app, &second).is_some());
    let mut moved = app.deck().get(&first).unwrap().config().to_spec();
    moved.working_dir = PathBuf::from(r"C:\src\beta");
    app.edit(&first, moved).unwrap();
    assert!(busy(&app, &second).is_some(), "the run is still in alpha");
}

// LCH-011: Run ticked starts every ticked row, top first; a refusal is named and the rest still
// start; an unticked row is left alone.
#[test]
fn run_ticked_starts_every_ticked_row() {
    let world = world();
    let mut app = world.app();
    let alpha = add(&mut app, SCRIPT_A);
    let beta = add(&mut app, SCRIPT_B);
    let gamma = add(&mut app, SCRIPT_C);
    let twin = add(&mut app, SCRIPT_A);
    assert!(app.run_checked().is_empty(), "nothing ticked, nothing run");
    assert!(world.state.borrow().launches.is_empty());
    for id in [&alpha, &gamma, &twin] {
        app.toggle_checked(id).unwrap();
    }
    app.run(&gamma).unwrap();
    let refused = app.run_checked();
    let names: Vec<&str> = refused.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["alpha build", "gamma make"]);
    assert!(matches!(refused[0].1, AppError::FolderInUse { .. }));
    assert_eq!(
        refused[1].1,
        AppError::Transition(TransitionError::AlreadyRunning)
    );
    assert!(app.is_running(&alpha) && app.is_running(&gamma));
    assert!(!app.is_running(&beta) && !app.is_running(&twin));
}
