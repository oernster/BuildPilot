//! Starting one step of a run; what a run checks before its first step (SRS 3.17, 3.18).

use std::path::{Path, PathBuf};

use crate::domain::environment::{Need, environments_among, resolve, step_variables};
use crate::domain::launch_plan::{LaunchPlan, ScriptKind, plan};
use crate::domain::lifecycle::LaunchError;
use crate::domain::operation::OperationConfig;
use crate::domain::output::Stream;

use super::App;
use super::ports::{ProcessHandle, RunKey};

impl App {
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
        let dir = config.working_dir();
        if !paths.is_dir(dir) {
            return Err(LaunchError::WorkingDirNotFound(dir.to_path_buf()));
        }
        let found = environments_among(dir, &paths.subfolders(dir), |path| paths.is_file(path));
        resolve(config.environment_need(), config.environment(), &found)
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
        let activation = environment.filter(|_| step.kind().environment_need() != Need::None);
        let paths = &self.ports.paths;
        let variables = step_variables(&self.ports.variables.inherited(), activation, |path| {
            paths.is_file(path)
        });
        let launch = plan(
            step,
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
            let interpreter = if step.kind() == ScriptKind::Python {
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

/// `launch` as the operator would type it, for a launch failure message (LCH-009) and the
/// step's line (STEP-006).
pub(super) fn display_command(launch: &LaunchPlan) -> String {
    std::iter::once(launch.program.to_string_lossy().into_owned())
        .chain(launch.arguments.iter().cloned())
        .map(|part| {
            if part.is_empty() || part.contains(char::is_whitespace) {
                format!("\"{part}\"")
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
