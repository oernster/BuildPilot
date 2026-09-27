//! Everything setup says. Pure, so every screen's words are a test rather than a screenshot.

use super::plan::Step;
use super::route::{Installed, Route};
use crate::domain::version::Version;

/// What the route screen says and offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteWords {
    /// The heading; a single version appears here only where the route is about one.
    pub heading: String,
    /// `v1 → v2` for an update or a downgrade; empty otherwise.
    pub flow: String,
    /// The sentence under the heading.
    pub lead: String,
    /// The go-ahead, named for the route.
    pub go: String,
    /// A second go-ahead where the route has one (Reinstall beside Repair); empty otherwise.
    pub second: String,
}

/// The arrow between two versions in a flow line.
pub const FLOW_ARROW: &str = "\u{2192}";

/// What the route screen says for `route`, installing `carried` into `folder`.
pub fn route_words(route: Route, carried: Version, folder: &str) -> RouteWords {
    let flow = |from: String| format!("{from} {FLOW_ARROW} v{carried}");
    match route {
        Route::Install => RouteWords {
            heading: format!("Install BuildPilot {carried}"),
            flow: String::new(),
            lead: format!(
                "BuildPilot will be installed for your Windows account in {folder}. \
                 It needs no administrator rights."
            ),
            go: "Install".to_owned(),
            second: String::new(),
        },
        Route::Update { from } => RouteWords {
            heading: "Update BuildPilot".to_owned(),
            flow: flow(from.map_or_else(|| "an earlier version".to_owned(), |v| format!("v{v}"))),
            lead: "Your operations and settings are kept.".to_owned(),
            go: "Update".to_owned(),
            second: String::new(),
        },
        Route::Downgrade { from } => RouteWords {
            heading: "Go back to an earlier BuildPilot".to_owned(),
            flow: flow(format!("v{from}")),
            lead: "Your operations and settings are kept. A newer BuildPilot may have saved \
                   settings this one cannot read; if so it says so and leaves them untouched."
                .to_owned(),
            go: "Go back".to_owned(),
            second: String::new(),
        },
        Route::Manage => RouteWords {
            heading: format!("BuildPilot {carried} is installed"),
            flow: String::new(),
            lead: "Repair puts the program files back and leaves everything else alone. \
                   Reinstall writes them again with the choices below."
                .to_owned(),
            go: "Repair".to_owned(),
            second: "Reinstall".to_owned(),
        },
    }
}

/// The uninstall screen's heading.
pub const UNINSTALL_HEADING: &str = "Uninstall BuildPilot";
/// The uninstall screen's lead.
pub const UNINSTALL_LEAD: &str =
    "The program files, the shortcuts and the Apps list entry are removed.";
/// The uninstall screen's one option, ticked by default (INST-005).
pub const KEEP_DATA: &str = "Keep my operations, settings and log";
/// The uninstall screen's go-ahead.
pub const UNINSTALL_GO: &str = "Uninstall";

/// The heading while BuildPilot is open (INST-004).
pub const RUNNING_HEADING: &str = "BuildPilot is running";
/// The lead while BuildPilot is open.
pub const RUNNING_LEAD: &str = "Setup waits until it is closed. Closing it stops any build it \
    is running.";
/// The go-ahead while BuildPilot is open.
pub const RUNNING_GO: &str = "Close it and continue";

/// Leaves the screen without doing anything.
pub const CANCEL: &str = "Cancel";
/// Ends setup from the verdict.
pub const CLOSE: &str = "Close";
/// Opens the uninstall screen from the route screen.
pub const UNINSTALL_LINK: &str = "Uninstall";
/// The route screen's desktop shortcut option.
pub const DESKTOP_SHORTCUT: &str = "Put a shortcut on the desktop";
/// The route screen's start-when-finished option.
pub const LAUNCH_AFTER: &str = "Start BuildPilot when setup finishes";

/// The progress screen's heading.
pub fn progress_heading(removing: bool) -> String {
    if removing {
        "Uninstalling BuildPilot".to_owned()
    } else {
        "Installing BuildPilot".to_owned()
    }
}

/// What the progress screen says while `step` runs.
pub fn step_words(step: Step) -> &'static str {
    match step {
        Step::CopyFiles => "Copying the program files",
        Step::StartMenuShortcut => "Adding the Start menu shortcut",
        Step::AddDesktopShortcut => "Adding the desktop shortcut",
        Step::RemoveDesktopShortcut => "Removing the desktop shortcut",
        Step::Register => "Adding BuildPilot to the Apps list",
        Step::RemoveShortcuts => "Removing the shortcuts",
        Step::Unregister => "Removing BuildPilot from the Apps list",
        Step::RemoveFiles => "Removing the program files",
        Step::RemoveData => "Removing your operations, settings and log",
    }
}

/// What the log says is installed as setup starts.
pub fn installed_words(installed: Installed) -> String {
    match installed {
        Installed::Nothing => "nothing".to_owned(),
        Installed::Recorded(Some(version)) => format!("version {version}"),
        Installed::Recorded(None) => "a version the Apps list does not state".to_owned(),
    }
}

/// The verdict's title and line: done; failed at a step with the reason and the log.
pub fn verdict(failure: Option<(Step, &str)>, removing: bool, log: &str) -> (String, String) {
    match failure {
        None if removing => (
            "BuildPilot is uninstalled".to_owned(),
            "Nothing of it is left running.".to_owned(),
        ),
        None => (
            "BuildPilot is ready".to_owned(),
            "Find it in the Start menu.".to_owned(),
        ),
        Some((step, reason)) => (
            "Setup did not finish".to_owned(),
            format!(
                "{} failed: {reason}. What happened is written in {log}.",
                step_words(step)
            ),
        ),
    }
}
