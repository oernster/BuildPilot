//! The setup program (INST-001 to INST-006): its composition root and what its window does.
//! The route, the plan and every word come from `buildpilot::setup`; the work from
//! `infrastructure::setup`. Every path ends in a verdict or in BuildPilot running.

// A windowed program: no console window opens alongside it.
#![windows_subsystem = "windows"]

use std::cell::Cell;
use std::env;
use std::fs;
use std::process::{Command, ExitCode};
use std::rc::Rc;
use std::thread;

use buildpilot::domain::credits;
use buildpilot::domain::version::Version;
use buildpilot::infrastructure::build_info::{CREDITS, LICENCE, NOTICES_FILE};
use buildpilot::infrastructure::locations::{
    PRODUCT_NAME, SETUP_EXE, install_folder, setup_log_folder,
};
use buildpilot::infrastructure::log_file::{LogFile, ROTATE_AT_BYTES};
use buildpilot::infrastructure::setup::{self as work, UNINSTALL_ARGUMENT};
use buildpilot::infrastructure::win32::diagnostics::show_error;
use buildpilot::infrastructure::win32::theme::windows_uses_dark;
use buildpilot::setup::plan::{self, Choices, Step};
use buildpilot::setup::route::{Installed, Route, route};
use buildpilot::setup::wording::{self, route_words, step_words, verdict};
use buildpilot::ui::{
    FooterAction, Reading, Ring, SetupScreen, SetupWindow, step_ring, wire_reading,
};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

/// The application this setup program installs, built into it by build.ps1.
#[cfg(buildpilot_payload)]
static PAYLOAD: &[u8] = include_bytes!(env!("BUILDPILOT_PAYLOAD"));
#[cfg(not(buildpilot_payload))]
static PAYLOAD: &[u8] = &[];

/// How long to wait for a started BuildPilot's window before setup closes, in milliseconds.
const LAUNCH_WAIT_MS: u32 = 5000;

struct Setup {
    window: slint::Weak<SetupWindow>,
    route: Route,
    carried: Version,
    log: LogFile,
    // True when setup was opened only to uninstall, so cancelling closes it.
    uninstall_only: bool,
    pending: Cell<Option<(Vec<Step>, bool)>>,
}

fn action(label: &str, id: &str, primary: bool, danger: bool) -> FooterAction {
    FooterAction {
        label: label.into(),
        id: id.into(),
        primary,
        danger,
    }
}

impl Setup {
    fn with(&self, act: impl FnOnce(&SetupWindow)) {
        if let Some(window) = self.window.upgrade() {
            act(&window);
        }
    }

    fn show(
        &self,
        screen: SetupScreen,
        heading: &str,
        flow: &str,
        lead: &str,
        actions: Vec<FooterAction>,
    ) {
        self.with(|window| {
            window.set_screen(screen);
            window.set_heading(heading.into());
            window.set_flow(flow.into());
            window.set_lead(lead.into());
            window.set_actions(ModelRc::new(VecModel::from(actions)));
        });
    }

    fn show_route(&self) {
        let words = route_words(
            self.route,
            self.carried,
            &install_folder().display().to_string(),
        );
        let mut actions = Vec::new();
        if self.route != Route::Install {
            actions.push(action(wording::UNINSTALL_LINK, "uninstall", false, false));
        }
        actions.push(action(wording::CANCEL, "cancel", false, false));
        if !words.second.is_empty() {
            actions.push(action(&words.second, "second", false, false));
        }
        actions.push(action(&words.go, "go", true, false));
        self.show(
            SetupScreen::Route,
            &words.heading,
            &words.flow,
            &words.lead,
            actions,
        );
    }

    fn show_uninstall(&self) {
        let actions = vec![
            action(wording::CANCEL, "cancel-uninstall", false, false),
            action(wording::UNINSTALL_GO, "do-uninstall", false, true),
        ];
        self.show(
            SetupScreen::Uninstall,
            wording::UNINSTALL_HEADING,
            "",
            wording::UNINSTALL_LEAD,
            actions,
        );
    }

    fn choices(&self) -> Choices {
        let window = self.window.upgrade();
        Choices {
            desktop_shortcut: window.as_ref().is_some_and(|w| w.get_desktop_shortcut()),
            launch_after: window.as_ref().is_some_and(|w| w.get_launch_after()),
        }
    }

    /// INST-004: nothing is touched while BuildPilot is open.
    fn begin(self: &Rc<Self>, steps: Vec<Step>, removing: bool) {
        if work::app_running() {
            self.pending.set(Some((steps, removing)));
            let actions = vec![
                action(wording::CANCEL, "cancel-running", false, false),
                action(wording::RUNNING_GO, "close-app", true, false),
            ];
            self.show(
                SetupScreen::Running,
                wording::RUNNING_HEADING,
                "",
                wording::RUNNING_LEAD,
                actions,
            );
            return;
        }
        self.run(steps, removing);
    }

    fn run(self: &Rc<Self>, steps: Vec<Step>, removing: bool) {
        let launch = !removing && self.choices().launch_after;
        self.show(
            SetupScreen::Progress,
            &wording::progress_heading(removing),
            "",
            "",
            Vec::new(),
        );
        let (weak, log, carried) = (self.window.clone(), self.log.clone(), self.carried);
        let log_path = log.path().display().to_string();
        thread::spawn(move || {
            let total = plan::weight(&steps);
            let mut done = 0;
            let mut failure = None;
            for step in &steps {
                let (words, fraction) = (step_words(*step), done as f32 / total as f32);
                let _ = weak.upgrade_in_event_loop(move |w| {
                    w.set_step(words.into());
                    w.set_progress(fraction);
                });
                log.write(words);
                if let Err(error) = work::perform(*step, PAYLOAD, carried) {
                    log.write(&format!("{words} failed: {error}"));
                    failure = Some((*step, error.to_string()));
                    break;
                }
                done += plan::step_weight(*step);
            }
            if failure.is_none() && launch {
                log.write("Starting BuildPilot");
                if work::launch(LAUNCH_WAIT_MS).is_ok() {
                    let _ = slint::quit_event_loop();
                    return;
                }
            }
            let reason = failure
                .as_ref()
                .map(|(step, error)| (*step, error.as_str()));
            let (title, line) = verdict(reason, removing, &log_path);
            log.write(&title);
            let ok = failure.is_none();
            let _ = weak.upgrade_in_event_loop(move |w| {
                w.set_succeeded(ok);
                w.set_screen(SetupScreen::Verdict);
                w.set_heading(SharedString::from(title));
                w.set_flow("".into());
                w.set_lead(SharedString::from(line));
                w.set_actions(ModelRc::new(VecModel::from(vec![action(
                    wording::CLOSE,
                    "close",
                    true,
                    false,
                )])));
            });
        });
    }

    fn act(self: &Rc<Self>, id: &str) {
        match id {
            "go" if self.route == Route::Manage => self.begin(plan::repair(), false),
            "go" | "second" => self.begin(plan::install(self.choices()), false),
            "uninstall" => self.show_uninstall(),
            "cancel-uninstall" if !self.uninstall_only => self.show_route(),
            "do-uninstall" => {
                let keep = self.window.upgrade().is_none_or(|w| w.get_keep_data());
                self.begin(plan::uninstall(keep), true);
            }
            "close-app" => {
                if work::close_app()
                    && let Some((steps, removing)) = self.pending.take()
                {
                    self.run(steps, removing);
                }
            }
            "cancel-running" => self.show_route(),
            "desktop-changed" if self.route == Route::Manage => {
                let wanted = self.choices().desktop_shortcut;
                let step = if wanted {
                    Step::AddDesktopShortcut
                } else {
                    Step::RemoveDesktopShortcut
                };
                self.log.write(step_words(step));
                if let Err(error) = work::perform(step, PAYLOAD, self.carried) {
                    self.log
                        .write(&format!("{} failed: {error}", step_words(step)));
                }
            }
            "desktop-changed" => {}
            "licence" | "close-licence" => self.with(|w| {
                let open = id == "licence";
                w.global::<Ring>().set_modal_open(open);
                w.set_show_licence(open);
            }),
            _ => {
                let _ = slint::quit_event_loop();
            }
        }
    }
}

/// The licence page: BuildPilot's licence, then every crate it is built from (UI-009), whose
/// licence texts setup installs beside it.
fn licence_page() -> String {
    let credits = credits::parse(CREDITS);
    let mut page = format!(
        "{LICENCE}\n\nThird-party crates\n\nBuildPilot is built from these {} crates. Their \
         licence texts are installed beside it as {NOTICES_FILE}.\n\n",
        credits.len()
    );
    for credit in credits {
        page.push_str(&format!(
            "{} {}: {}\n",
            credit.name,
            credit.version,
            credits::readable_licence(&credit.licence)
        ));
    }
    page
}

/// Setup cannot delete the folder it runs from, so a copy started from there moves to the
/// temporary folder first and runs again from it. True when this process should now end.
fn relocated() -> bool {
    let (Ok(this), Ok(home)) = (env::current_exe(), install_folder().canonicalize()) else {
        return false;
    };
    if this.parent().and_then(|p| p.canonicalize().ok()) != Some(home) {
        return false;
    }
    let folder = setup_log_folder();
    let copy = folder.join(SETUP_EXE);
    fs::create_dir_all(&folder).is_ok()
        && fs::copy(&this, &copy).is_ok()
        && Command::new(&copy)
            .args(env::args().skip(1))
            .spawn()
            .is_ok()
}

fn main() -> ExitCode {
    if relocated() {
        return ExitCode::SUCCESS;
    }
    let log = LogFile::open(&setup_log_folder(), ROTATE_AT_BYTES);
    log.record_panics();
    let Some(carried) = Version::parse(env!("BUILDPILOT_VERSION")) else {
        show_error(
            PRODUCT_NAME,
            "This setup program carries no readable version.",
        );
        return ExitCode::FAILURE;
    };
    if PAYLOAD.is_empty() {
        show_error(
            PRODUCT_NAME,
            "This setup program was built without BuildPilot inside it. Build it with build.ps1.",
        );
        return ExitCode::FAILURE;
    }
    let installed = work::installed();
    log.write(&format!(
        "Setup {carried} started; installed: {installed:?}"
    ));
    let window = match SetupWindow::new() {
        Ok(window) => window,
        Err(error) => {
            show_error(
                PRODUCT_NAME,
                &format!("Setup could not open its window: {error}"),
            );
            return ExitCode::FAILURE;
        }
    };
    window.invoke_apply_theme(windows_uses_dark());
    window.set_desktop_label(wording::DESKTOP_SHORTCUT.into());
    window.set_launch_label(wording::LAUNCH_AFTER.into());
    window.set_keep_label(wording::KEEP_DATA.into());
    window.set_licence_text(licence_page().into());
    wire_reading(&window.global::<Reading>());
    // Options open on what is already true.
    let has_desktop = work::desktop_shortcut().is_some_and(|link| link.exists());
    window.set_desktop_shortcut(has_desktop);
    window.set_launch_after(installed == Installed::Nothing);
    let setup = Rc::new(Setup {
        window: window.as_weak(),
        route: route(installed, carried),
        carried,
        log: log.clone(),
        uninstall_only: env::args().any(|arg| arg == UNINSTALL_ARGUMENT),
        pending: Cell::new(None),
    });
    if setup.uninstall_only {
        setup.show_uninstall();
    } else {
        setup.show_route();
    }
    let handler = setup.clone();
    window.on_action(move |id| handler.act(&id));
    let weak = window.as_weak();
    window.on_toggle_theme(move || {
        if let Some(w) = weak.upgrade() {
            w.invoke_apply_theme(!w.global::<buildpilot::ui::Theme>().get_dark());
        }
    });
    let weak = window.as_weak();
    window.on_step_ring(move |forward| {
        if let Some(w) = weak.upgrade() {
            step_ring(w.window(), forward);
        }
    });
    if let Err(error) = window.run() {
        log.write(&format!("Setup's window failed: {error}"));
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
