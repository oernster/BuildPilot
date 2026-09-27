//! Starting one step of a run; what a run checks before its first step (SRS 3.17, 3.18).

use std::path::{Path, PathBuf};

use crate::domain::environment::{Need, Offer, environments_among, offer, resolve, step_variables};
use crate::domain::host::{Host, HostTable};
use crate::domain::launch_plan::{LaunchPlan, ScriptKind, command_line, need_of, plan};
use crate::domain::lifecycle::LaunchError;
use crate::domain::operation::OperationConfig;
use crate::domain::output::Stream;
use crate::domain::step::{Step, StepSpec};

use super::ports::{ProcessHandle, RunKey};
use super::{App, AppError};

impl App {
    /// The environments directly inside `dir` (ENV-001), as the dialog and a run both see them.
    pub(super) fn environments_found(&self, dir: &Path) -> Vec<String> {
        let paths = &self.ports.paths;
        environments_among(dir, &paths.subfolders(dir), |path| paths.is_file(path))
    }

    /// What the dialog shows about the environment for `steps` run in `working_dir` (ENV-002).
    pub fn environment_offer(&self, working_dir: &Path, steps: &[StepSpec]) -> Offer {
        // A step of no known kind yet (still being typed) needs nothing.
        let hosts = steps
            .iter()
            .filter_map(|step| self.preferences.hosts.host_for(&step.script_path));
        offer(need_of(hosts), &self.environments_found(working_dir))
    }

    /// Refuses a configuration with a step nothing runs (LCH-001) or that leaves open which of
    /// several environments to use (ENV-002).
    pub(super) fn check_config(&self, config: &OperationConfig) -> Result<(), AppError> {
        let hosts = &self.preferences.hosts;
        config.check_hosts(hosts)?;
        let found = self.environments_found(config.working_dir());
        let unchosen = config.environment_need(hosts) != Need::None
            && config.environment().is_none()
            && found.len() > 1;
        if unchosen {
            return Err(AppError::EnvironmentNotChosen(found));
        }
        Ok(())
    }

    /// Everything a run needs, checked before step 1 starts so a missing later script is not
    /// found after the earlier steps have run (STEP-005). Answers the environment folder every
    /// step uses (ENV-003), if any.
    pub(super) fn prepare(&self, config: &OperationConfig) -> Result<Option<PathBuf>, LaunchError> {
        let paths = &self.ports.paths;
        if let Some(step) = config
            .steps()
            .iter()
            .find(|step| !paths.is_file(step.script_path()))
        {
            return Err(LaunchError::ScriptNotFound(
                step.script_path().to_path_buf(),
            ));
        }
        let hosts = &self.preferences.hosts;
        for step in config.steps() {
            host_of(hosts, step)?;
        }
        let dir = config.working_dir();
        if !paths.is_dir(dir) {
            return Err(LaunchError::WorkingDirNotFound(dir.to_path_buf()));
        }
        let found = self.environments_found(dir);
        resolve(config.environment_need(hosts), config.environment(), &found)
            .map(|name| name.map(|name| dir.join(name)))
            .map_err(|problem| LaunchError::Environment {
                problem,
                searched: dir.to_path_buf(),
            })
    }

    /// Starts step `index` of run `key`, run with `environment` where the step's kind is
    /// activated (ENV-006, OQ-18). Writes the step's own lines to the run's output first
    /// (STEP-006, ENV-008) and logs the outcome.
    pub(super) fn start_step(
        &mut self,
        key: &RunKey,
        config: &OperationConfig,
        index: usize,
        environment: Option<&Path>,
    ) -> Result<Box<dyn ProcessHandle>, LaunchError> {
        let step = &config.steps()[index];
        // Checked by `prepare`; Settings may have changed the table since step 1.
        let host = host_of(&self.preferences.hosts, step)?;
        let activation = environment.filter(|_| host.environment_need() != Need::None);
        let paths = &self.ports.paths;
        let variables = step_variables(&self.ports.variables.inherited(), activation, |path| {
            paths.is_file(path)
        });
        let launch = plan(
            step,
            host,
            config.working_dir(),
            self.powershell,
            activation,
            variables,
        );
        let command = display_command(&launch);
        let steps = config.steps().len();
        let mut notes = Vec::new();
        if steps > 1 {
            notes.push(format!("Step {} of {steps}: {command}", index + 1));
        }
        if let Some(folder) = activation {
            let interpreter = if host == Host::BuiltIn(ScriptKind::Python) {
                format!(", interpreter {}", launch.program.display())
            } else {
                String::new()
            };
            notes.push(format!("Environment: {}{interpreter}", folder.display()));
        }
        let runtime = self.runtimes.entry(key.operation.clone()).or_default();
        for note in &notes {
            runtime.output.push(Stream::Note, note);
        }
        let outcome = self
            .ports
            .launcher
            .spawn(key.clone(), &launch)
            .map_err(|message| LaunchError::Os {
                command: command.clone(),
                message,
            });
        let name = config.name();
        let line = match &outcome {
            Ok(process) => format!("Started {name} (process {}): {command}", process.pid()),
            Err(error) => format!("{name} could not start: {error}"),
        };
        self.ports.log.record(&line);
        outcome
    }
}

/// What runs `step`, from `hosts` and the built-in rule; refused when nothing does.
fn host_of<'a>(hosts: &'a HostTable, step: &Step) -> Result<Host<'a>, LaunchError> {
    hosts
        .host_for(step.script_path())
        .ok_or_else(|| LaunchError::NoHost(step.script_path().to_path_buf()))
}

/// `launch` as the operator would type it, for a launch failure message (LCH-009) and the
/// step's line (STEP-006).
pub(super) fn display_command(launch: &LaunchPlan) -> String {
    command_line(&launch.program, &launch.arguments)
}
