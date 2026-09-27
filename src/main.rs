//! BuildPilot's composition root: the one place the real ports are built and wired together.

// A windowed application: no console window opens alongside it.
#![windows_subsystem = "windows"]

use std::env;
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, mpsc};

use buildpilot::application::{App, Ports};
use buildpilot::domain::credits;
use buildpilot::infrastructure::build_info::{CREDITS, LICENCE};
use buildpilot::infrastructure::config_store::JsonConfigStore;
use buildpilot::infrastructure::icons::FsIconLibrary;
use buildpilot::infrastructure::launcher::{EventSink, WindowsLauncher};
use buildpilot::infrastructure::locations::{
    AUTHOR, COPYRIGHT, DONATE_URL, PRODUCT_NAME, data_folder,
};
use buildpilot::infrastructure::log_file::LogFile;
use buildpilot::infrastructure::powershell::detect_powershell;
use buildpilot::infrastructure::releases::{GitHubReleases, REPOSITORY};
use buildpilot::infrastructure::shell::ExplorerShell;
use buildpilot::infrastructure::system::{FsPaths, ProcessVariables, SystemClock, UuidIds};
use buildpilot::infrastructure::win32::diagnostics::show_error;
use buildpilot::infrastructure::win32::instance::{self, Claim};
use buildpilot::infrastructure::win32::theme::windows_uses_dark;
use buildpilot::ui::{self, Environment, HelpFacts, Waker};

/// The folder under the data folder where chosen icons are copied.
const ICONS_FOLDER_NAME: &str = "icons";

fn main() -> ExitCode {
    let data = data_folder();
    // DATA-001, settled before the config or the log is touched so each has one writer. A later
    // BuildPilot leaves both alone and exits.
    let claimed = instance::claim(&instance::instance_key(&data));
    if let Ok(Claim::Running) = claimed {
        return ExitCode::SUCCESS;
    }
    // The log comes next, before anything else can fail, so every failure has somewhere to go.
    let log = LogFile::open_for_process(&data);
    log.record_panics();
    log.write(&format!(
        "BuildPilot {} started, data folder {}",
        env!("BUILDPILOT_VERSION"),
        data.display()
    ));
    match claimed {
        Ok(Claim::First(instance)) => instance.on_summons(ui::summons()),
        Ok(Claim::Running) => {}
        // Nothing to compare against, so this run goes ahead without the guard.
        Err(error) => log.write(&format!("Could not check for another BuildPilot: {error}")),
    }

    // NFR-REL-001: a panic on the UI thread would end the run; the operator is told why first.
    let ran = panic::catch_unwind(AssertUnwindSafe(|| run(data, log.clone())));
    let failure = match ran {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(format!("BuildPilot could not open its window: {error}")),
        Err(_) => Some("BuildPilot hit an internal error and has to close.".to_owned()),
    };
    let Some(failure) = failure else {
        log.write("BuildPilot closed");
        return ExitCode::SUCCESS;
    };
    log.write(&failure);
    let details = format!("{failure}\n\nThe details are in {}.", log.path().display());
    show_error(PRODUCT_NAME, &details);
    log.write("BuildPilot closed after the error above");
    ExitCode::FAILURE
}

/// Builds the real ports, starts the application and runs the window until it closes.
fn run(data: PathBuf, log: LogFile) -> Result<(), slint::PlatformError> {
    let (sender, events) = mpsc::channel();
    let waker = Waker::new();
    let ports = Ports {
        store: Box::new(JsonConfigStore::new(data.clone())),
        ids: Box::new(UuidIds),
        clock: Box::new(SystemClock),
        paths: Box::new(FsPaths),
        icons: Box::new(FsIconLibrary::new(data.join(ICONS_FOLDER_NAME))),
        launcher: Box::new(WindowsLauncher::new(EventSink::new(
            sender,
            waker.callback(),
        ))),
        shell: Box::new(ExplorerShell),
        log: Box::new(log),
        variables: Box::new(ProcessVariables),
    };
    let app = App::start(ports, detect_powershell(env::var_os("PATH").as_deref()));
    let version = env!("BUILDPILOT_VERSION");
    let help = HelpFacts {
        description: env!("CARGO_PKG_DESCRIPTION"),
        author: AUTHOR,
        copyright: COPYRIGHT,
        repository: REPOSITORY,
        donate: DONATE_URL,
        credits: credits::parse(CREDITS),
        licence: LICENCE,
        releases: Arc::new(GitHubReleases::new(PRODUCT_NAME, version)),
    };
    let environment = Environment {
        version,
        windows_uses_dark: windows_uses_dark(),
        bring_forward: instance::bring_forward,
        help,
    };
    ui::run(app, events, waker, environment)
}
