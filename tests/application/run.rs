use std::path::PathBuf;
use std::time::Duration;

use buildpilot::application::{AppError, Notice, OverdueStop, STOP_TIMEOUT};
use buildpilot::domain::lifecycle::{Failure, LaunchError, RunState, TransitionError};
use buildpilot::domain::operation::OperationId;
use buildpilot::domain::output::Stream;

use super::fakes::{FIRST_PID, World};
use super::support::{SCRIPT_A, SCRIPT_B, SCRIPT_C, add, exited, line, output_texts, world};

// LCH-002, LIFE-004: Run starts the planned command and the row is Running at once.
#[test]
fn run_launches_and_is_running() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    // Never run: nothing has started, so there is no time to tell.
    assert_eq!(app.elapsed(&a), None);
    app.run(&a).unwrap();
    assert!(app.is_running(&a));
    let (key, plan) = world.state.borrow().launches[0].clone();
    assert_eq!(key.operation, a);
    assert_eq!(plan.program, PathBuf::from("pwsh.exe"));
    assert_eq!(plan.working_dir, PathBuf::from(r"C:\src\alpha"));
    assert_eq!(app.elapsed(&a), Some(Duration::ZERO));
    world.advance(Duration::from_secs(42));
    assert_eq!(app.elapsed(&a), Some(Duration::from_secs(42)));
}

// LCH-005
#[test]
fn second_run_is_refused() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    app.run(&a).unwrap();
    assert_eq!(
        app.run(&a),
        Err(AppError::Transition(TransitionError::AlreadyRunning))
    );
    assert_eq!(world.state.borrow().launches.len(), 1);
    let ghost = OperationId::new("ghost").unwrap();
    assert!(app.run(&ghost).is_err());
}

// LCH-007
#[test]
fn missing_script_is_not_launched() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    world.state.borrow_mut().files.clear();
    app.run(&a).unwrap();
    assert_eq!(
        app.run_state(&a),
        &RunState::Failed(Failure::FailedToStart(LaunchError::ScriptNotFound(
            PathBuf::from(SCRIPT_A)
        )))
    );
    assert!(world.state.borrow().launches.is_empty());
    assert_eq!(app.elapsed(&a), None);
}

// LCH-008
#[test]
fn missing_working_dir_is_not_launched() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    world.state.borrow_mut().dirs.clear();
    app.run(&a).unwrap();
    assert_eq!(
        app.run_state(&a),
        &RunState::Failed(Failure::FailedToStart(LaunchError::WorkingDirNotFound(
            PathBuf::from(r"C:\src\alpha")
        )))
    );
}

// LCH-009: the operating system's refusal names the command attempted.
#[test]
fn launch_refusal_names_the_command() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    world.state.borrow_mut().spawn_error = Some("Access is denied.".to_owned());
    app.run(&a).unwrap();
    let RunState::Failed(Failure::FailedToStart(error)) = app.run_state(&a) else {
        panic!("expected a launch failure");
    };
    assert_eq!(
        error.to_string(),
        "Could not start pwsh.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File \
         C:\\src\\alpha\\build.ps1: Access is denied."
    );
}

#[test]
fn display_command_quotes_spaced_and_empty_parts() {
    let world = World::new().with_script(r"C:\my src\tool.exe");
    let mut app = world.app();
    let mut spec = app.draft_for(r"C:\my src\tool.exe".as_ref()).unwrap();
    spec.arguments = vec!["two words".to_owned()];
    let id = app.add(spec).unwrap();
    world.state.borrow_mut().spawn_error = Some("No.".to_owned());
    app.run(&id).unwrap();
    let RunState::Failed(Failure::FailedToStart(error)) = app.run_state(&id) else {
        panic!("expected a launch failure");
    };
    assert_eq!(
        error.to_string(),
        "Could not start \"C:\\my src\\tool.exe\" \"two words\": No."
    );
}

// LIFE-002, LIFE-007: the exit code decides; the duration stops at exit.
#[test]
fn exit_code_decides_and_duration_freezes() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    let b = add(&mut app, SCRIPT_B);
    app.run(&a).unwrap();
    app.run(&b).unwrap();
    world.advance(Duration::from_secs(3));
    app.handle_event(exited(&world.launch_key(0), 0));
    app.handle_event(exited(&world.launch_key(1), 3));
    assert_eq!(app.run_state(&a), &RunState::Succeeded);
    assert_eq!(app.run_state(&b), &RunState::Failed(Failure::ExitCode(3)));
    world.advance(Duration::from_secs(60));
    assert_eq!(app.elapsed(&a), Some(Duration::from_secs(3)));
}

// OUT-002, LCH-006: interleaved output from concurrent runs stays with its operation.
#[test]
fn output_is_attributed_to_its_operation() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    let b = add(&mut app, SCRIPT_B);
    app.run(&a).unwrap();
    app.run(&b).unwrap();
    let (ka, kb) = (world.launch_key(0), world.launch_key(1));
    app.handle_event(line(&ka, Stream::Stdout, "a1"));
    app.handle_event(line(&kb, Stream::Stderr, "b1"));
    app.handle_event(line(&ka, Stream::Stdout, "a2"));
    assert_eq!(output_texts(&app, &a), ["a1", "a2"]);
    assert_eq!(output_texts(&app, &b), ["b1"]);
    let b_line = app.output(&b).unwrap().lines().next().unwrap().clone();
    assert_eq!(b_line.stream, Stream::Stderr);
}

// LCH-006: independent runs finish in any order.
#[test]
fn concurrent_runs_finish_in_any_order() {
    let world = world();
    let mut app = world.app();
    let ids = [
        add(&mut app, SCRIPT_A),
        add(&mut app, SCRIPT_B),
        add(&mut app, SCRIPT_C),
    ];
    for id in &ids {
        app.run(id).unwrap();
    }
    assert!(ids.iter().all(|id| app.is_running(id)));
    for nth in (0..ids.len()).rev() {
        app.handle_event(exited(&world.launch_key(nth), 0));
        assert!(!app.is_running(&ids[nth]));
        assert!(ids[..nth].iter().all(|id| app.is_running(id)));
    }
}

// OUT-003, OUT-002: output survives the run; a rerun starts clean; stale events are ignored.
#[test]
fn rerun_clears_output_and_ignores_the_old_run() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    assert!(app.output(&a).is_none());
    app.run(&a).unwrap();
    let first = world.launch_key(0);
    app.handle_event(line(&first, Stream::Stdout, "first run"));
    app.handle_event(exited(&first, 0));
    assert_eq!(output_texts(&app, &a), ["first run"]);

    app.run(&a).unwrap();
    assert!(output_texts(&app, &a).is_empty());
    app.handle_event(line(&first, Stream::Stdout, "late"));
    app.handle_event(exited(&first, 1));
    assert!(output_texts(&app, &a).is_empty());
    assert!(app.is_running(&a));

    let mut unknown = world.launch_key(1);
    unknown.operation = OperationId::new("ghost").unwrap();
    app.handle_event(exited(&unknown, 0));
    assert!(app.is_running(&a));
}

// A second exit for the same run changes nothing.
#[test]
fn a_repeated_exit_is_ignored() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    app.run(&a).unwrap();
    app.handle_event(exited(&world.launch_key(0), 0));
    app.handle_event(exited(&world.launch_key(0), 9));
    assert_eq!(app.run_state(&a), &RunState::Succeeded);
}

// STOP-001, LIFE-003, STOP-002
#[test]
fn stop_terminates_one_run_and_reads_stopped() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    let b = add(&mut app, SCRIPT_B);
    app.run(&a).unwrap();
    app.run(&b).unwrap();
    app.stop(&a).unwrap();
    assert_eq!(world.state.borrow().stops, [FIRST_PID + 1]);
    app.handle_event(exited(&world.launch_key(0), 0));
    assert_eq!(app.run_state(&a), &RunState::Stopped);
    assert!(app.is_running(&b));
}

#[test]
fn stop_needs_a_running_process() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    let not_running = Err(AppError::Transition(TransitionError::NotRunning));
    assert_eq!(app.stop(&a), not_running);
    app.run(&a).unwrap();
    app.handle_event(exited(&world.launch_key(0), 0));
    assert_eq!(app.stop(&a), not_running);
    assert_eq!(app.stop(&OperationId::new("ghost").unwrap()), not_running);
}

// STOP-003: several overdue stops are listed in a stable order.
#[test]
fn overdue_stops_are_listed_in_identity_order() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    let b = add(&mut app, SCRIPT_B);
    app.run(&b).unwrap();
    app.run(&a).unwrap();
    app.stop(&b).unwrap();
    app.stop(&a).unwrap();
    world.advance(STOP_TIMEOUT);
    let overdue: Vec<OperationId> = app
        .overdue_stops()
        .into_iter()
        .map(|stop| stop.id)
        .collect();
    assert_eq!(overdue, [a, b]);
}

// STOP-003: a failed stop is reported and Stop stays usable; a slow one becomes overdue.
#[test]
fn failed_or_slow_stop_is_reported() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    app.run(&a).unwrap();
    // Running with no Stop asked for is never overdue.
    world.advance(STOP_TIMEOUT);
    assert!(app.overdue_stops().is_empty());
    world.state.borrow_mut().stop_error = Some("Access is denied.".to_owned());
    app.stop(&a).unwrap();
    assert_eq!(
        app.take_notices(),
        [Notice::StopFailed {
            name: "alpha build".to_owned(),
            message: "Access is denied.".to_owned()
        }]
    );
    assert!(app.is_running(&a));
    world.advance(STOP_TIMEOUT - Duration::from_millis(1));
    assert!(app.overdue_stops().is_empty());
    app.stop(&a).unwrap();
    world.advance(Duration::from_millis(1));
    assert_eq!(
        app.overdue_stops(),
        [OverdueStop {
            id: a.clone(),
            pid: FIRST_PID + 1
        }]
    );
}

// STOP-004: closing lists and stops every running operation, in deck order.
#[test]
fn stop_all_stops_every_running_operation() {
    let world = world();
    let mut app = world.app();
    let a = add(&mut app, SCRIPT_A);
    let b = add(&mut app, SCRIPT_B);
    let c = add(&mut app, SCRIPT_C);
    app.run(&c).unwrap();
    app.run(&a).unwrap();
    assert_eq!(app.running_names(), ["alpha build", "gamma make"]);
    app.stop_all();
    let mut stops = world.state.borrow().stops.clone();
    stops.sort_unstable();
    assert_eq!(stops, [FIRST_PID + 1, FIRST_PID + 2]);
    assert!(!app.is_running(&b));
}
