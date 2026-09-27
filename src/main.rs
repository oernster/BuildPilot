//! BuildPilot's composition root: the one place the real ports are built and wired together.

// A windowed application: no console window opens alongside it.
#![windows_subsystem = "windows"]

use std::env;
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc;

use buildpilot::application::{App, Ports};
use buildpilot::infrastructure::config_store::JsonConfigStore;
use buildpilot::infrastructure::icons::FsIconLibrary;
use buildpilot::infrastructure::launcher::{EventSink, WindowsLauncher};
use buildpilot::infrastructure::log_file::LogFile;
use buildpilot::infrastructure::powershell::detect_powershell;
use buildpilot::infrastructure::shell::ExplorerShell;
use buildpilot::infrastructure::system::{FsPaths, SystemClock, UuidIds};
use buildpilot::infrastructure::win32::diagnostics::show_error;
use buildpilot::infrastructure::win32::instance::{self, Claim};
use buildpilot::infrastructure::win32::theme::windows_uses_dark;
use buildpilot::ui::{self, Environment, Waker};

/// The product's name, as Windows shows it in an error box's title.
const PRODUCT_NAME: &str = "BuildPilot";
/// The data folder's name under %APPDATA%.
const DATA_FOLDER_NAME: &str = PRODUCT_NAME;
/// The folder under the data folder where chosen icons are copied.
const ICONS_FOLDER_NAME: &str = "icons";

/// Names a different data folder, for testing without touching the real one.
const DATA_FOLDER_OVERRIDE: &str = "BUILDPILOT_DATA_DIR";

/// `BUILDPILOT_DATA_DIR` when set, else `%APPDATA%\BuildPilot`; the temporary folder if Windows
/// gives no APPDATA, so BuildPilot still opens and Settings shows where it is saving.
fn data_folder() -> PathBuf {
    if let Some(folder) = env::var_os(DATA_FOLDER_OVERRIDE) {
        return PathBuf::from(folder);
    }
    env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir)
        .join(DATA_FOLDER_NAME)
}

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
    };
    let app = App::start(ports, detect_powershell(env::var_os("PATH").as_deref()));
    let environment = Environment {
        version: env!("BUILDPILOT_VERSION"),
        windows_uses_dark: windows_uses_dark(),
        bring_forward: instance::bring_forward,
    };
    ui::run(app, events, waker, environment)
}
