//! BuildPilot's composition root: the one place the real ports are built and wired together.

// A windowed application: no console window opens alongside it.
#![windows_subsystem = "windows"]

use std::env;
use std::path::PathBuf;
use std::sync::mpsc;

use buildpilot::application::{App, Ports};
use buildpilot::infrastructure::config_store::JsonConfigStore;
use buildpilot::infrastructure::icons::FsIconLibrary;
use buildpilot::infrastructure::launcher::{EventSink, WindowsLauncher};
use buildpilot::infrastructure::powershell::detect_powershell;
use buildpilot::infrastructure::shell::ExplorerShell;
use buildpilot::infrastructure::system::{FsPaths, SystemClock, UuidIds};
use buildpilot::infrastructure::win32::theme::windows_uses_dark;
use buildpilot::ui::{self, Environment, Waker};

/// The data folder's name under %APPDATA%.
const DATA_FOLDER_NAME: &str = "BuildPilot";
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

fn main() {
    let data = data_folder();
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
    };
    let app = App::start(ports, detect_powershell(env::var_os("PATH").as_deref()));
    let environment = Environment {
        version: env!("BUILDPILOT_VERSION"),
        windows_uses_dark: windows_uses_dark(),
    };
    if let Err(error) = ui::run(app, events, waker, environment) {
        // No window could open, so there is nowhere else to say it.
        eprintln!("BuildPilot could not open its window: {error}");
    }
}
