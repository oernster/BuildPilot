//! The UI layer: the Slint window over the application layer. A client of `App` and nothing
//! below it; every action goes through one `App` method.
//!
//! All state lives on the UI thread in `Ui`. Process events arrive through `events::Waker` and
//! are drained here; a 250 ms tick refreshes elapsed times and the running indicator.

// The Slint compiler generates public items without doc comments.
#![allow(missing_docs)]

slint::include_modules!();

mod actions;
mod dialog;
mod events;
mod layout;
mod output_model;
mod row_sync;
pub mod rows;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::num::NonZeroIsize;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::{
    CloseRequestResponse, ComponentHandle, Image, ModelRc, PhysicalPosition, PhysicalSize,
    SharedString, Timer, TimerMode, VecModel,
};

use crate::application::{App, AppError, RunEvent};
use crate::domain::follow::Follow;
use crate::domain::operation::OperationId;

use events::Hook;
pub use events::Waker;
use output_model::OutputModel;

/// How often elapsed times and the running indicator refresh.
const TICK: Duration = Duration::from_millis(250);
/// Tray height before the operator first resizes it, in logical pixels.
const DEFAULT_TRAY_HEIGHT: f32 = 260.0;

/// What the composition root tells the UI about the machine and the build.
pub struct Environment {
    /// The version, from VERSION.
    pub version: &'static str,
    /// Whether Windows currently asks apps to be dark.
    pub windows_uses_dark: bool,
    /// Restores and raises a native window, given its handle (DATA-001).
    pub bring_forward: fn(NonZeroIsize),
}

/// What a later BuildPilot's summons calls, from any thread: brings the window forward.
pub fn summons() -> impl Fn() + Send + 'static {
    || events::schedule(Hook::Summon)
}

/// A question waiting on the confirm dialog.
enum Pending {
    Remove(OperationId),
    Close,
}

/// The UI's state.
pub(crate) struct Ui {
    app: Rc<RefCell<App>>,
    window: slint::Weak<MainWindow>,
    rows: Rc<VecModel<RowData>>,
    output: Rc<OutputModel>,
    notices: Rc<VecModel<SharedString>>,
    icons: RefCell<HashMap<PathBuf, Option<Image>>>,
    follow: Cell<Follow>,
    tick: Cell<usize>,
    tray_expanded: Cell<bool>,
    tray_height: Cell<f32>,
    dialog: RefCell<Option<dialog::DialogState>>,
    pending: RefCell<Option<Pending>>,
    environment: Environment,
    waker: Waker,
    events: Receiver<RunEvent>,
}

/// Opens the window and runs until it closes.
pub fn run(
    app: App,
    events: Receiver<RunEvent>,
    waker: Waker,
    environment: Environment,
) -> Result<(), slint::PlatformError> {
    let window = MainWindow::new()?;
    let preferences = *app.preferences();
    let app = Rc::new(RefCell::new(app));
    let ui = Rc::new(Ui {
        output: Rc::new(OutputModel::new(app.clone())),
        app,
        window: window.as_weak(),
        rows: Rc::new(VecModel::default()),
        notices: Rc::new(VecModel::default()),
        icons: RefCell::new(HashMap::new()),
        follow: Cell::new(Follow::default()),
        tick: Cell::new(0),
        tray_expanded: Cell::new(preferences.tray.expanded),
        tray_height: Cell::new(
            preferences
                .tray
                .height
                .map_or(DEFAULT_TRAY_HEIGHT, |h| h as f32),
        ),
        dialog: RefCell::new(None),
        pending: RefCell::new(None),
        environment,
        waker,
        events,
    });

    window.set_rows(ModelRc::from(ui.rows.clone()));
    window.set_lines(ModelRc::from(ui.output.clone()));
    window.set_notices(ModelRc::from(ui.notices.clone()));
    window.set_version(SharedString::from(ui.environment.version));
    if let Some(geometry) = preferences.window {
        window
            .window()
            .set_position(PhysicalPosition::new(geometry.x, geometry.y));
        window
            .window()
            .set_size(PhysicalSize::new(geometry.width, geometry.height));
    }
    ui.show_tray();
    ui.apply_theme();
    actions::wire(&ui, &window);
    dialog::wire(&ui, &window);

    let drain_ui = ui.clone();
    events::install(Hook::Drain, Rc::new(move || drain_ui.drain()));
    let summon_ui = ui.clone();
    events::install(Hook::Summon, Rc::new(move || summon_ui.come_forward()));
    let tick_ui = ui.clone();
    let ticker = Timer::default();
    ticker.start(TimerMode::Repeated, TICK, move || {
        tick_ui.tick.set(tick_ui.tick.get().wrapping_add(1));
        tick_ui.sync_rows();
    });
    let close_ui = ui.clone();
    window
        .window()
        .on_close_requested(move || close_ui.close_requested());

    ui.refresh();
    window.run()
}

impl Ui {
    fn with_window(&self, act: impl FnOnce(&MainWindow)) {
        if let Some(window) = self.window.upgrade() {
            act(&window);
        }
    }

    /// Everything the window shows, brought up to date.
    fn refresh(&self) {
        let notices = self.app.borrow_mut().take_notices();
        for notice in &notices {
            self.notify(rows::notice_text(notice));
        }
        self.sync_rows();
        self.sync_output();
    }

    /// DATA-001: a later BuildPilot was started, so this window comes forward instead.
    fn come_forward(&self) {
        self.with_window(|window| {
            let handle = window.window().window_handle();
            if let Ok(handle) = handle.window_handle()
                && let RawWindowHandle::Win32(win32) = handle.as_raw()
            {
                (self.environment.bring_forward)(win32.hwnd);
            }
        });
    }

    /// Tells the operator something in the notice area.
    fn notify(&self, text: String) {
        self.notices.push(SharedString::from(text));
    }

    /// Runs `result` through the notice area when it is an error, then refreshes.
    fn report<T>(&self, result: Result<T, AppError>) -> Option<T> {
        let value = match result {
            Ok(value) => Some(value),
            Err(error) => {
                self.notify(error.to_string());
                None
            }
        };
        self.refresh();
        value
    }

    fn drain(&self) {
        self.waker.clear();
        {
            let mut app = self.app.borrow_mut();
            while let Ok(event) = self.events.try_recv() {
                app.handle_event(event);
            }
        }
        self.refresh();
    }

    fn sync_output(&self) {
        let added = self.output.sync();
        let (heading, dropped) = {
            let app = self.app.borrow();
            match app.selection().selected() {
                Some(id) => (
                    format!(
                        "Output: {}",
                        app.deck().get(id).map_or("", |op| op.config().name())
                    ),
                    app.output(id).map_or(0, |buffer| buffer.dropped()),
                ),
                None => ("Select a row to see its output".to_owned(), 0),
            }
        };
        let follows = self.follow.get().follows();
        self.with_window(|window| {
            window.set_tray_heading(SharedString::from(heading));
            window.set_dropped_note(SharedString::from(rows::dropped_note(dropped)));
            window.set_following(follows);
            if added && follows {
                window.invoke_scroll_output_to_end();
            }
        });
    }

    /// STOP-004: closing with builds running asks first.
    fn close_requested(&self) -> CloseRequestResponse {
        self.save_layout();
        let running = self.app.borrow().running_names();
        if running.is_empty() {
            return CloseRequestResponse::HideWindow;
        }
        *self.pending.borrow_mut() = Some(Pending::Close);
        let verb = if running.len() == 1 { "is" } else { "are" };
        self.ask(
            "Builds are still running",
            &format!(
                "{} {verb} still running. Stop and close BuildPilot?",
                running.join(", ")
            ),
            "Stop and close",
        );
        CloseRequestResponse::KeepWindowShown
    }

    fn ask(&self, heading: &str, message: &str, confirm_label: &str) {
        self.with_window(|window| {
            window.set_confirm_heading(SharedString::from(heading));
            window.set_confirm_message(SharedString::from(message));
            window.set_confirm_label(SharedString::from(confirm_label));
            window.set_show_confirm(true);
        });
    }
}
