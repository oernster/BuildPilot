//! Runs of several steps and runs in an existing environment (SRS 3.17, 3.18).

use std::path::{Path, PathBuf};

use buildpilot::application::App;
use buildpilot::domain::environment::EnvironmentProblem;
use buildpilot::domain::launch_plan::LaunchPlan;
use buildpilot::domain::lifecycle::{Failure, LaunchError, RunState};
use buildpilot::domain::operation::OperationId;
use buildpilot::domain::output::Stream;
use buildpilot::domain::step::StepSpec;

use super::fakes::World;
use super::support::{draft, exited};

const APP: &str = r"C:\src\app";
const BUILD_EXE: &str = r"C:\src\app\buildexe.py";
const BUILD_INSTALLER: &str = r"C:\src\app\buildinstaller.py";
const BUILD_PS1: &str = r"C:\src\app\build.ps1";
const BUILD_CMD: &str = r"C:\src\app\build.cmd";
const VENV: &str = r"C:\src\app\venv";

/// A world holding every script above in `C:\src\app`.
fn scripts() -> World {
    [BUILD_EXE, BUILD_INSTALLER, BUILD_PS1, BUILD_CMD]
        .into_iter()
        .fold(World::new(), World::with_script)
}

/// Adds one operation running `steps` in order, in `C:\src\app`.
fn add_steps(app: &mut App, steps: &[&str]) -> OperationId {
    let mut spec = draft(app, steps[0]);
    spec.steps.extend(
        steps[1..]
            .iter()
            .map(|script| StepSpec::for_script(script.as_ref())),
    );
    app.add(spec).unwrap()
}

fn plan(world: &World, nth: usize) -> LaunchPlan {
    world.state.borrow().launches[nth].1.clone()
}

fn set(plan: &LaunchPlan, name: &str) -> Option<String> {
    plan.variables
        .set
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.clone())
}

fn notes(app: &App, id: &OperationId) -> Vec<String> {
    app.output(id)
        .unwrap()
        .lines()
        .filter(|line| line.stream == Stream::Note)
        .map(|line| line.text.clone())
        .collect()
}

// STEP-002, STEP-006, STEP-007: steps start in order in one run, each announced.
#[test]
fn steps_run_in_order() {
    let world = scripts().with_environment(VENV);
    let mut app = world.app();
    let id = add_steps(&mut app, &[BUILD_EXE, BUILD_INSTALLER]);
    app.run(&id).unwrap();
    assert_eq!(plan(&world, 0).arguments, [BUILD_EXE]);
    let RunState::Running(running) = app.run_state(&id) else {
        panic!("running");
    };
    assert_eq!((running.step(), running.steps()), (1, 2));
    app.handle_event(exited(&world.launch_key(0), 0));
    assert_eq!(world.launch_key(1), world.launch_key(0));
    assert_eq!(plan(&world, 1).arguments, [BUILD_INSTALLER]);
    assert!(app.is_running(&id));
    app.handle_event(exited(&world.launch_key(1), 0));
    assert_eq!(app.run_state(&id), &RunState::Succeeded);
    let interpreter = r"C:\src\app\venv\Scripts\python.exe";
    assert_eq!(
        notes(&app, &id),
        [
            format!(r"Step 1 of 2: {interpreter} {BUILD_EXE}"),
            format!("Environment: {VENV}, interpreter {interpreter}"),
            format!(r"Step 2 of 2: {interpreter} {BUILD_INSTALLER}"),
            format!("Environment: {VENV}, interpreter {interpreter}"),
        ]
    );
}

// STEP-003: a failing step ends the run; the next never starts.
#[test]
fn a_failing_step_ends_the_run() {
    let world = scripts();
    let mut app = world.app();
    let id = add_steps(&mut app, &[BUILD_CMD, BUILD_PS1]);
    app.run(&id).unwrap();
    app.handle_event(exited(&world.launch_key(0), 3));
    assert_eq!(
        app.run_state(&id),
        &RunState::Failed(Failure::StepExitCode { step: 1, code: 3 })
    );
    assert_eq!(world.state.borrow().launches.len(), 1);
}

// STEP-004: Stop during a step ends the sequence.
#[test]
fn stop_ends_the_sequence() {
    let world = scripts();
    let mut app = world.app();
    let id = add_steps(&mut app, &[BUILD_CMD, BUILD_PS1]);
    app.run(&id).unwrap();
    app.stop(&id).unwrap();
    app.handle_event(exited(&world.launch_key(0), 0));
    assert_eq!(app.run_state(&id), &RunState::Stopped);
    assert_eq!(world.state.borrow().launches.len(), 1);
}

// STEP-005: a missing later script is found before the first step starts.
#[test]
fn every_script_is_checked_first() {
    let world = scripts();
    let mut app = world.app();
    let id = add_steps(&mut app, &[BUILD_CMD, BUILD_PS1]);
    world.state.borrow_mut().files.remove(Path::new(BUILD_PS1));
    app.run(&id).unwrap();
    assert_eq!(
        app.run_state(&id),
        &RunState::Failed(Failure::FailedToStart(LaunchError::ScriptNotFound(
            PathBuf::from(BUILD_PS1)
        )))
    );
    assert!(world.state.borrow().launches.is_empty());
}

// LCH-009: a later step Windows refuses ends the run as failed to start, its time frozen.
#[test]
fn a_refused_later_step_fails_the_run() {
    let world = scripts();
    let mut app = world.app();
    let id = add_steps(&mut app, &[BUILD_CMD, BUILD_PS1]);
    app.run(&id).unwrap();
    world.state.borrow_mut().spawn_error = Some("Access is denied.".to_owned());
    app.handle_event(exited(&world.launch_key(0), 0));
    let RunState::Failed(Failure::FailedToStart(LaunchError::Os { message, .. })) =
        app.run_state(&id)
    else {
        panic!("expected a refused start: {:?}", app.run_state(&id));
    };
    assert_eq!(message, "Access is denied.");
    let took = app.elapsed(&id);
    world.advance(std::time::Duration::from_secs(9));
    assert_eq!(app.elapsed(&id), took);
}

// EDIT-002: an edit during a run waits for the next run, later steps included.
#[test]
fn later_steps_come_from_the_run_as_started() {
    let world = scripts();
    let mut app = world.app();
    let id = add_steps(&mut app, &[BUILD_CMD, BUILD_PS1]);
    app.run(&id).unwrap();
    let mut edited = app.deck().get(&id).unwrap().config().to_spec();
    edited.steps[1] = StepSpec::for_script(Path::new(BUILD_EXE));
    app.edit(&id, edited).unwrap();
    app.handle_event(exited(&world.launch_key(0), 0));
    assert_eq!(plan(&world, 1).arguments.last().unwrap(), BUILD_PS1);
}

/// The launch error a run of `steps` ended with.
fn refusal(world: &World, steps: &[&str], environment: Option<&str>) -> LaunchError {
    let mut app = world.app();
    let mut spec = draft(&app, steps[0]);
    spec.environment = environment.map(str::to_owned);
    let id = app.add(spec).unwrap();
    app.run(&id).unwrap();
    assert!(world.state.borrow().launches.is_empty());
    let RunState::Failed(Failure::FailedToStart(error)) = app.run_state(&id) else {
        panic!("expected a refusal");
    };
    error.clone()
}

fn environment_error(problem: EnvironmentProblem) -> LaunchError {
    LaunchError::Environment {
        problem,
        searched: PathBuf::from(APP),
    }
}

// ENV-004, ENV-005: a .py step with no environment to use does not start.
#[test]
fn python_without_a_usable_environment_is_refused() {
    assert_eq!(
        refusal(&scripts(), &[BUILD_EXE], None),
        environment_error(EnvironmentProblem::NoneFound)
    );
    let two = scripts()
        .with_environment(VENV)
        .with_environment(r"C:\src\app\venv_smoke");
    assert_eq!(
        refusal(&two, &[BUILD_EXE], None),
        environment_error(EnvironmentProblem::Several(vec![
            "venv".to_owned(),
            "venv_smoke".to_owned()
        ]))
    );
    assert_eq!(
        refusal(
            &scripts().with_environment(VENV),
            &[BUILD_EXE],
            Some("gone")
        ),
        environment_error(EnvironmentProblem::NamedMissing("gone".to_owned()))
    );
}

// ENV-003: the named environment is used when several exist.
#[test]
fn the_named_environment_is_used() {
    let world = scripts()
        .with_environment(VENV)
        .with_environment(r"C:\src\app\venv_smoke");
    let mut app = world.app();
    let mut spec = draft(&app, BUILD_EXE);
    spec.environment = Some("venv_smoke".to_owned());
    let id = app.add(spec).unwrap();
    app.run(&id).unwrap();
    assert_eq!(
        plan(&world, 0).program,
        PathBuf::from(r"C:\src\app\venv_smoke\Scripts\python.exe")
    );
}

// ENV-006, OQ-18: .ps1 is activated when there is an environment; .cmd never is.
#[test]
fn powershell_is_activated_and_batch_is_not() {
    let world = scripts().with_environment(VENV);
    let mut app = world.app();
    let id = add_steps(&mut app, &[BUILD_PS1, BUILD_CMD]);
    app.run(&id).unwrap();
    app.handle_event(exited(&world.launch_key(0), 0));
    let powershell = plan(&world, 0);
    assert_eq!(powershell.program, PathBuf::from("pwsh.exe"));
    assert_eq!(set(&powershell, "VIRTUAL_ENV").as_deref(), Some(VENV));
    let batch = plan(&world, 1);
    assert_eq!(set(&batch, "VIRTUAL_ENV"), None);
    assert!(batch.variables.remove.contains(&"VIRTUAL_ENV".to_owned()));
    assert_eq!(
        notes(&app, &id)[1],
        format!("Environment: {VENV}"),
        "a .ps1 step names no interpreter"
    );
}

// ENV-004: a .ps1 step with no environment runs without one.
#[test]
fn powershell_runs_without_an_environment() {
    let world = scripts();
    let mut app = world.app();
    let id = add_steps(&mut app, &[BUILD_PS1]);
    app.run(&id).unwrap();
    assert!(app.is_running(&id));
    assert_eq!(set(&plan(&world, 0), "VIRTUAL_ENV"), None);
    assert!(notes(&app, &id).is_empty());
}

// ENV-010: an environment's Scripts folder on PATH with no record of an activation is still
// taken off, judged by the disk.
#[test]
fn a_stray_environment_leaves_path() {
    let world = scripts().with_inherited(&[("Path", r"C:\stray\venv\Scripts;C:\Windows")]);
    world
        .state
        .borrow_mut()
        .files
        .insert(PathBuf::from(r"C:\stray\venv\pyvenv.cfg"));
    let mut app = world.app();
    let id = add_steps(&mut app, &[BUILD_CMD]);
    app.run(&id).unwrap();
    assert_eq!(
        set(&plan(&world, 0), "PATH").as_deref(),
        Some(r"C:\Windows")
    );
}

// ENV-009: an activation BuildPilot inherited never reaches a step.
#[test]
fn an_inherited_activation_is_undone() {
    let world = scripts().with_inherited(&[
        ("VIRTUAL_ENV", r"C:\other\venv"),
        ("PATH", r"C:\other\venv\Scripts;C:\Windows"),
        ("_OLD_VIRTUAL_PATH", r"C:\Windows"),
    ]);
    let mut app = world.app();
    let id = add_steps(&mut app, &[BUILD_CMD]);
    app.run(&id).unwrap();
    let launch = plan(&world, 0);
    assert_eq!(set(&launch, "PATH").as_deref(), Some(r"C:\Windows"));
    assert!(launch.variables.remove.contains(&"VIRTUAL_ENV".to_owned()));
}
