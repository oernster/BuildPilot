use std::time::Duration;

use buildpilot::domain::operation::OperationId;
use buildpilot::domain::run_times::RunTimes;

use super::support::{SCRIPT_A, add, exited, world};

// LIFE-008: a success is recorded and saved; a failure neither replaces nor saves; the typical
// time stays in place while the next run is under way.
#[test]
fn successes_are_recorded_and_saved() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    assert_eq!(app.typical_duration(&a), None);
    app.run(&a).unwrap();
    world.advance(Duration::from_secs(3));
    app.handle_event(exited(&world.launch_key(0), 0));
    assert_eq!(app.typical_duration(&a), Some(Duration::from_secs(3)));
    assert_eq!(world.state.borrow().run_times_saves.len(), 1);

    app.run(&a).unwrap();
    assert_eq!(app.typical_duration(&a), Some(Duration::from_secs(3)));
    world.advance(Duration::from_secs(1));
    app.handle_event(exited(&world.launch_key(1), 3));
    assert_eq!(app.typical_duration(&a), Some(Duration::from_secs(3)));
    assert_eq!(world.state.borrow().run_times_saves.len(), 1);

    let saved = world.state.borrow().run_times_saves[0].clone();
    assert_eq!(saved.typical(&a), Some(Duration::from_secs(3)));
}

// LIFE-008: times stored by an earlier session are used; those of operations no longer on the
// deck are dropped.
#[test]
fn stored_times_are_loaded_for_known_operations() {
    let world = world();
    let a = add(&mut world.app(), SCRIPT_A);
    let ghost = OperationId::new("ghost").unwrap();
    let stored = RunTimes::from_recent([
        (a.clone(), vec![Duration::from_secs(40)]),
        (ghost.clone(), vec![Duration::from_secs(9)]),
    ]);
    {
        let mut state = world.state.borrow_mut();
        let saved = state.saves.last().unwrap().0.clone();
        state.to_load.operations = saved;
        state.run_times_to_load = Some(Ok(stored));
    }
    let app = world.app();
    assert_eq!(app.typical_duration(&a), Some(Duration::from_secs(40)));
    assert_eq!(app.typical_duration(&ghost), None);
}

// LIFE-008: unreadable run times are logged and start afresh; they never stop BuildPilot.
#[test]
fn unreadable_times_are_logged() {
    let world = world();
    world.state.borrow_mut().run_times_to_load = Some(Err("bad JSON".to_owned()));
    let mut app = world.app();
    assert!(app.take_notices().is_empty());
    assert!(
        world
            .logged()
            .iter()
            .any(|line| line.contains("bad JSON") && line.contains("start afresh"))
    );
}

// LIFE-008: a save that fails is logged; the time is still used this session.
#[test]
fn failed_save_is_logged() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    world.state.borrow_mut().run_times_save_error = Some("Access is denied.".to_owned());
    app.run(&a).unwrap();
    world.advance(Duration::from_secs(2));
    app.handle_event(exited(&world.launch_key(0), 0));
    assert_eq!(app.typical_duration(&a), Some(Duration::from_secs(2)));
    assert!(
        world
            .logged()
            .iter()
            .any(|line| line.contains("run-times.json") && line.contains("Access is denied."))
    );
}

// LIFE-008, REM-001: removing an operation forgets its times and saves that.
#[test]
fn remove_forgets_times() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    app.run(&a).unwrap();
    app.handle_event(exited(&world.launch_key(0), 0));
    app.remove(&a).unwrap();
    assert_eq!(app.typical_duration(&a), None);
    let last = world.state.borrow().run_times_saves.last().unwrap().clone();
    assert_eq!(last, RunTimes::default());
}
