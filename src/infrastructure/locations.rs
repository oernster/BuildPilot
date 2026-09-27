//! Where BuildPilot and its setup program keep things: one home for every name and folder, so
//! the application, setup and uninstall cannot disagree.

use std::env;
use std::path::PathBuf;

/// The product's name, as Windows shows it.
pub const PRODUCT_NAME: &str = "BuildPilot";
/// Who wrote it and publishes it: About says so and the Apps list shows it.
pub const AUTHOR: &str = "Oliver Ernster";
/// The copyright line About shows.
pub const COPYRIGHT: &str = "Copyright \u{a9} 2026 Oliver Ernster";
/// Where the toolbar's donate button sends a browser (UI-014). BuildPilot hands it to Windows,
/// which opens it in the operator's browser; BuildPilot itself never fetches it.
pub const DONATE_URL: &str = "https://www.paypal.com/ncp/payment/XC9S6VZ96K9Q4";
/// The application's executable.
pub const APP_EXE: &str = "buildpilot.exe";
/// The setup program's copy of itself in the install folder, run by Modify, Repair and Uninstall.
pub const SETUP_EXE: &str = "BuildPilotSetup.exe";
/// The shortcut's file name, in the Start menu and on the desktop.
pub const SHORTCUT: &str = "BuildPilot.lnk";
/// Names a different data folder, for testing without touching the real one.
pub const DATA_FOLDER_OVERRIDE: &str = "BUILDPILOT_DATA_DIR";
/// The folder under %LOCALAPPDATA% that per-user programs are installed into.
const PROGRAMS_FOLDER: &str = "Programs";
/// Where setup keeps its log, under the temporary folder.
const SETUP_LOG_FOLDER: &str = "BuildPilot Setup";

/// The folder named by `variable`; the temporary folder when Windows gives none.
fn folder_from(variable: &str) -> PathBuf {
    env::var_os(variable)
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir)
}

/// `BUILDPILOT_DATA_DIR` when set, else the installed data folder. BuildPilot still opens when
/// Windows gives no APPDATA; Settings shows where it is saving.
pub fn data_folder() -> PathBuf {
    env::var_os(DATA_FOLDER_OVERRIDE)
        .map(PathBuf::from)
        .unwrap_or_else(installed_data_folder)
}

/// `%APPDATA%\BuildPilot`, whatever the override says: the folder uninstall offers to remove.
pub fn installed_data_folder() -> PathBuf {
    folder_from("APPDATA").join(PRODUCT_NAME)
}

/// `%LOCALAPPDATA%\Programs\BuildPilot` (INST-001).
pub fn install_folder() -> PathBuf {
    folder_from("LOCALAPPDATA")
        .join(PROGRAMS_FOLDER)
        .join(PRODUCT_NAME)
}

/// Where setup writes its log.
pub fn setup_log_folder() -> PathBuf {
    env::temp_dir().join(SETUP_LOG_FOLDER)
}
