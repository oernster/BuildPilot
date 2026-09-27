//! The operator's host table through the application (SRS 3.19).

use std::path::{Path, PathBuf};

use buildpilot::application::{App, AppError};
use buildpilot::domain::environment::Offer;
use buildpilot::domain::host::{HostError, HostRow};
use buildpilot::domain::lifecycle::{Failure, LaunchError, RunState};
use buildpilot::domain::operation::{OperationError, OperationId};
use buildpilot::domain::step::StepSpec;

use super::fakes::World;
use super::support::{add, draft, exited};

const APP: &str = r"C:\src\app";
const BUILD_RB: &str = r"C:\src\app\build.rb";
const BUILD_PY: &str = r"C:\src\app\buildexe.py";
const BUILD_CMD: &str = r"C:\src\app\build.cmd";
const RUBY: &str = r"C:\Ruby33\bin\ruby.exe";
const PYTHON: &str = r"C:\Python313\python.exe";

fn row(extension: &str, program: &str) -> HostRow {
    HostRow::new(extension, PathBuf::from(program), Vec::new()).unwrap()
}

fn scripts() -> World {
    [BUILD_RB, BUILD_PY, BUILD_CMD]
        .into_iter()
        .fold(World::new(), World::with_script)
}

/// An app whose table runs `.rb` with Ruby.
fn with_ruby(world: &World) -> App {
    let mut app = world.app();
    app.set_hosts(vec![row("rb", RUBY)]).unwrap();
    app
}

fn failure(app: &App, id: &OperationId) -> LaunchError {
    let RunState::Failed(Failure::FailedToStart(error)) = app.run_state(id) else {
        panic!("expected a failure to start: {:?}", app.run_state(id));
    };
    error.clone()
}

// LCH-001, HOST-001: a type is refused at Add until the table runs it; the table is saved.
#[test]
fn add_follows_the_table() {
    let world = scripts();
    let mut app = world.app();
    assert!(matches!(
        app.draft_for(Path::new(BUILD_RB)),
        Err(AppError::Invalid(
            OperationError::UnsupportedScriptType { .. }
        ))
    ));
    let saves = world.save_count();
    app.set_hosts(vec![row(".rb", RUBY)]).unwrap();
    assert_eq!(world.save_count(), saves + 1);
    let saved = world.state.borrow().saves.last().unwrap().1.clone();
    assert_eq!(saved.hosts.rows(), [row("rb", RUBY)]);
    assert_eq!(app.preferences().hosts, saved.hosts);
    add(&mut app, BUILD_RB);
    assert_eq!(app.deck().operations().len(), 1);
}

// HOST-001: two rows for one extension are refused and nothing changes.
#[test]
fn duplicate_rows_change_nothing() {
    let world = scripts();
    let mut app = with_ruby(&world);
    let saves = world.save_count();
    let refused = app.set_hosts(vec![row("sh", "bash.exe"), row("SH", "zsh.exe")]);
    assert_eq!(
        refused,
        Err(AppError::Host(HostError::Duplicate("sh".to_owned())))
    );
    assert!(refused.unwrap_err().to_string().contains(".sh"));
    assert_eq!(app.preferences().hosts.rows(), [row("rb", RUBY)]);
    assert_eq!(world.save_count(), saves);
}

// HOST-003: the pickers offer the built-in types then the operator's.
#[test]
fn pickers_offer_the_operator_types() {
    let world = scripts();
    let app = with_ruby(&world);
    assert_eq!(
        app.script_extensions(),
        ["ps1", "bat", "cmd", "exe", "com", "py", "rb"]
    );
}

// LCH-001: Edit refuses a step nothing runs, like Add.
#[test]
fn edit_refuses_a_type_nothing_runs() {
    let world = scripts();
    let mut app = world.app();
    let id = add(&mut app, BUILD_CMD);
    let mut spec = app.deck().get(&id).unwrap().config().to_spec();
    spec.steps.push(StepSpec::for_script(Path::new(BUILD_RB)));
    assert!(matches!(
        app.edit(&id, spec),
        Err(AppError::Invalid(
            OperationError::UnsupportedScriptType { .. }
        ))
    ));
}

// HOST-002 acceptance: `.rb` mapped to ruby.exe runs `build.rb --release` as
// `ruby.exe build.rb --release`, with no environment even where one exists.
#[test]
fn operator_row_runs_the_step() {
    let world = scripts().with_environment(r"C:\src\app\venv");
    let mut app = with_ruby(&world);
    let mut spec = draft(&app, BUILD_RB);
    spec.steps[0].arguments = vec!["--release".to_owned()];
    let id = app.add(spec).unwrap();
    app.run(&id).unwrap();
    let launch = world.state.borrow().launches[0].1.clone();
    assert_eq!(launch.program, PathBuf::from(RUBY));
    assert_eq!(launch.arguments, [BUILD_RB, "--release"]);
    assert!(
        launch
            .variables
            .set
            .iter()
            .all(|(name, _)| name != "VIRTUAL_ENV")
    );
}

// HOST-002: a .py row replaces activation entirely: no environment offered, demanded or used.
#[test]
fn python_row_needs_no_environment() {
    let world = scripts()
        .with_environment(r"C:\src\app\venv")
        .with_environment(r"C:\src\app\venv_other");
    let mut app = world.app();
    app.set_hosts(vec![row("py", PYTHON)]).unwrap();
    let steps = [StepSpec::for_script(Path::new(BUILD_PY))];
    assert_eq!(
        app.environment_offer(Path::new(APP), &steps),
        Offer::Nothing
    );
    let id = add(&mut app, BUILD_PY);
    app.run(&id).unwrap();
    let launch = world.state.borrow().launches[0].1.clone();
    assert_eq!(launch.program, PathBuf::from(PYTHON));
    let notes = app.output(&id).unwrap().lines().count();
    assert_eq!(notes, 0, "no Environment line for an operator's program");
}

// HOST-001: removing a row never loses the operation; its run fails before step 1, saying why.
#[test]
fn a_removed_row_fails_the_run_before_it_starts() {
    let world = scripts();
    let mut app = with_ruby(&world);
    let mut spec = draft(&app, BUILD_CMD);
    spec.steps.push(StepSpec::for_script(Path::new(BUILD_RB)));
    let id = app.add(spec).unwrap();
    app.set_hosts(Vec::new()).unwrap();
    app.run(&id).unwrap();
    let error = failure(&app, &id);
    assert_eq!(error, LaunchError::NoHost(PathBuf::from(BUILD_RB)));
    assert!(
        error
            .to_string()
            .contains("add a host for its type in Settings")
    );
    assert!(world.state.borrow().launches.is_empty());
}

// HOST-001: a row removed while step 1 runs fails the run when step 2 would start.
#[test]
fn a_row_removed_mid_run_fails_the_next_step() {
    let world = scripts();
    let mut app = with_ruby(&world);
    let mut spec = draft(&app, BUILD_CMD);
    spec.steps.push(StepSpec::for_script(Path::new(BUILD_RB)));
    let id = app.add(spec).unwrap();
    app.run(&id).unwrap();
    app.set_hosts(Vec::new()).unwrap();
    app.handle_event(exited(&world.launch_key(0), 0));
    assert_eq!(
        failure(&app, &id),
        LaunchError::NoHost(PathBuf::from(BUILD_RB))
    );
    assert_eq!(world.state.borrow().launches.len(), 1);
}
