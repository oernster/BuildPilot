use std::path::PathBuf;

use buildpilot::application::{App, AppError, LoadProblem, Notice, StoreError};
use buildpilot::domain::operation::OperationId;

use super::fakes::{FIRST_PID, World};
use super::support::{SCRIPT_A, add, exited, world};

fn name(app: &App, id: &OperationId) -> String {
    app.deck().get(id).unwrap().config().name().to_owned()
}

// NFR-REL-002: every notice names what went wrong in words.
#[test]
fn every_notice_has_words() {
    let path = PathBuf::from(r"C:\data\buildpilot.json");
    let notices = [
        Notice::Load(LoadProblem::UnreadableEntries(2)),
        Notice::Load(LoadProblem::SetAside(path.clone())),
        Notice::Load(LoadProblem::NewerSchema),
        Notice::Load(LoadProblem::Unreadable {
            path: path.clone(),
            message: "denied".to_owned(),
        }),
        Notice::DuplicateDropped(OperationId::new("x").unwrap()),
        Notice::SaveFailed(StoreError {
            path,
            message: "full".to_owned(),
        }),
        Notice::StopFailed {
            name: "app".to_owned(),
            message: "denied".to_owned(),
        },
    ];
    for notice in &notices {
        assert!(notice.to_string().len() > 20, "{notice:?}");
    }
    assert!(notices[0].to_string().starts_with('2'));
}

// NFR-OBS-001: a launch is logged with its process and the command run.
#[test]
fn a_launch_is_logged_with_process_and_command() {
    let world = world();
    let mut app = world.app();
    let id = add(&mut app, SCRIPT_A);
    app.run(&id).unwrap();

    let logged = world.logged();
    let line = logged.last().unwrap();
    let expected = format!("Started {} (process {}): ", name(&app, &id), FIRST_PID + 1);
    assert!(line.starts_with(&expected), "{line}");
    assert!(line.contains("build.ps1"), "{line}");
}

// NFR-OBS-001: a launch Windows refuses is logged with the reason.
#[test]
fn a_refused_launch_is_logged() {
    let world = world();
    let mut app = world.app();
    let id = add(&mut app, SCRIPT_A);
    world.state.borrow_mut().spawn_error = Some("access denied".to_owned());
    app.run(&id).unwrap();

    let line = world.logged().last().unwrap().clone();
    assert!(
        line.starts_with(&format!("{} could not start: ", name(&app, &id))),
        "{line}"
    );
    assert!(line.contains("access denied"), "{line}");
}

// NFR-OBS-001: an exit is logged with its code.
#[test]
fn an_exit_is_logged_with_its_code() {
    let world = world();
    let mut app = world.app();
    let id = add(&mut app, SCRIPT_A);
    app.run(&id).unwrap();
    app.handle_event(exited(&world.launch_key(0), 3));

    let expected = format!("{} exited with code 3", name(&app, &id));
    assert_eq!(world.logged().last(), Some(&expected));
}

// NFR-OBS-001: a stop is logged when asked for and again when the run ends.
#[test]
fn a_stop_is_logged_when_asked_and_when_done() {
    let world = world();
    let mut app = world.app();
    let id = add(&mut app, SCRIPT_A);
    app.run(&id).unwrap();
    app.stop(&id).unwrap();
    app.handle_event(exited(&world.launch_key(0), 1));

    let name = name(&app, &id);
    let logged = world.logged();
    let tail = &logged[logged.len() - 2..];
    assert_eq!(
        tail,
        [
            format!("Stop asked for {name}"),
            format!("{name} stopped (exit code 1)")
        ]
    );
}

// NFR-OBS-001: every notice raised is logged in the words the operator sees.
#[test]
fn notices_are_logged_as_shown() {
    let world = world();
    world.state.borrow_mut().to_load.problems = vec![LoadProblem::NewerSchema];
    let mut app = world.app();
    assert_eq!(
        world.logged(),
        [Notice::Load(LoadProblem::NewerSchema).to_string()]
    );

    let id = add(&mut app, SCRIPT_A);
    app.run(&id).unwrap();
    world.state.borrow_mut().stop_error = Some("denied".to_owned());
    app.stop(&id).unwrap();
    let shown = app.take_notices();
    let logged = world.logged();
    assert!(logged.ends_with(&[shown.last().unwrap().to_string()]));
}

// NFR-OBS-001: a refusal the operator was shown is logged too.
#[test]
fn refusals_are_logged() {
    let world = World::new();
    let app = world.app();
    app.record_refusal(&AppError::Shell("no handler".to_owned()));
    assert_eq!(
        world.logged(),
        ["Refused: Windows could not open it: no handler"]
    );
}
