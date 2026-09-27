//! The window's callbacks for the toolbar, the rows, the tray, Settings, About and the confirm
//! dialog. Each is one `App` call plus a refresh.

use std::rc::Rc;

use slint::platform::{Key, WindowEvent};
use slint::{ComponentHandle, Model, SharedString};

use crate::application::LocateOutcome;
use crate::domain::follow::Follow;
use crate::domain::operation::OperationId;
use crate::domain::preferences::Theme;

use super::layout::theme_from_index;
use super::{MainWindow, Pending, Ui};

/// Connects every callback except the Add/Edit dialog's.
pub(super) fn wire(ui: &Rc<Ui>, window: &MainWindow) {
    wire_toolbar(ui, window);
    wire_rows(ui, window);
    wire_tray(ui, window);
    wire_confirm(ui, window);
    let weak = window.as_weak();
    window.on_step_ring(move |forward| {
        if let Some(window) = weak.upgrade() {
            step_ring(window.window(), forward);
        }
    });
}

/// Moves keyboard focus one stop, exactly as Tab or Shift+Tab would, so Right and Left walk the
/// same ring Slint walks (A11Y-003).
pub fn step_ring(window: &slint::Window, forward: bool) {
    let key = SharedString::from(if forward { Key::Tab } else { Key::Backtab });
    window.dispatch_event(WindowEvent::KeyPressed { text: key.clone() });
    window.dispatch_event(WindowEvent::KeyReleased { text: key });
}

/// An operation identity from a row; rows only ever carry valid ones.
fn id(text: &SharedString) -> Option<OperationId> {
    OperationId::new(text.as_str()).ok()
}

fn wire_toolbar(ui: &Rc<Ui>, window: &MainWindow) {
    let this = ui.clone();
    window.on_toggle_theme(move || {
        let windows_dark = this.environment.windows_uses_dark;
        let now_dark = this.app.borrow().preferences().theme.resolve(windows_dark) == Theme::Dark;
        let next = if now_dark { 0 } else { 1 };
        this.app.borrow_mut().set_theme(theme_from_index(next));
        this.apply_theme();
        this.refresh();
    });
    let this = ui.clone();
    window.on_choose_theme(move |index| {
        this.app.borrow_mut().set_theme(theme_from_index(index));
        this.apply_theme();
        this.refresh();
    });
    let this = ui.clone();
    window.on_open_settings(move || {
        let folder = this.app.borrow().data_folder();
        this.show_hosts();
        this.with_window(|window| {
            window.set_data_folder(SharedString::from(folder.display().to_string()));
            window.set_show_settings(true);
        });
    });
    let this = ui.clone();
    window.on_close_settings(move || this.with_window(|window| window.set_show_settings(false)));
    let this = ui.clone();
    window.on_open_data_folder(move || {
        let opened = this.app.borrow().open_data_folder();
        this.report(opened);
    });
    let this = ui.clone();
    window.on_dismiss_notice(move |index| {
        if let Ok(index) = usize::try_from(index)
            && index < this.notices.row_count()
        {
            this.notices.remove(index);
        }
    });
}

fn wire_rows(ui: &Rc<Ui>, window: &MainWindow) {
    let this = ui.clone();
    window.on_select(move |row| {
        let Some(id) = id(&row) else { return };
        this.select(&id);
    });
    let this = ui.clone();
    window.on_toggle_checked(move |row| {
        let Some(id) = id(&row) else { return };
        let toggled = this.app.borrow_mut().toggle_checked(&id);
        this.report(toggled);
    });
    let this = ui.clone();
    window.on_run(move |row| {
        let Some(id) = id(&row) else { return };
        this.select(&id);
        let ran = this.app.borrow_mut().run(&id);
        this.output.show(Some(id));
        this.report(ran);
    });
    let this = ui.clone();
    window.on_stop(move |row| {
        let Some(id) = id(&row) else { return };
        let stopped = this.app.borrow_mut().stop(&id);
        this.report(stopped);
    });
    let this = ui.clone();
    window.on_open_script(move |row| {
        let Some(id) = id(&row) else { return };
        let opened = this.app.borrow().open_script(&id);
        this.report(opened);
    });
    let this = ui.clone();
    window.on_locate_script(move |row| {
        let Some(id) = id(&row) else { return };
        let located = this.app.borrow().locate_script(&id);
        match this.report(located) {
            Some(LocateOutcome::ScriptMissingFolderOpened { script, folder }) => this.notify(format!(
                "Script not found: {}. Explorer shows {}, the nearest folder that still exists. Use Edit to point at the script.",
                script.display(),
                folder.display()
            )),
            Some(LocateOutcome::ScriptMissingNothingOpened(script)) => this.notify(format!(
                "Script not found: {}, nor any folder above it. Use Edit to point at the script.",
                script.display()
            )),
            Some(LocateOutcome::Revealed) | None => {}
        }
    });
    let this = ui.clone();
    window.on_remove(move |row| {
        let Some(id) = id(&row) else { return };
        let name = this
            .app
            .borrow()
            .deck()
            .get(&id)
            .map(|op| op.config().name().to_owned());
        let Some(name) = name else { return };
        *this.pending.borrow_mut() = Some(Pending::Remove(id));
        this.ask(
            "Remove from BuildPilot?",
            &format!("Remove {name} from BuildPilot? The script file itself is not touched."),
            "Remove",
        );
    });
    let this = ui.clone();
    window.on_move_up(move |row| {
        let Some(id) = id(&row) else { return };
        let moved = this.app.borrow_mut().move_up(&id);
        this.report(moved);
    });
    let this = ui.clone();
    window.on_move_down(move |row| {
        let Some(id) = id(&row) else { return };
        let moved = this.app.borrow_mut().move_down(&id);
        this.report(moved);
    });
    let this = ui.clone();
    window.on_move_to(move |row, target| {
        let (Some(id), Ok(target)) = (id(&row), usize::try_from(target)) else {
            return;
        };
        let moved = this.app.borrow_mut().move_to(&id, target);
        this.report(moved);
    });
}

fn wire_tray(ui: &Rc<Ui>, window: &MainWindow) {
    let this = ui.clone();
    window.on_toggle_tray(move || {
        this.tray_expanded.set(!this.tray_expanded.get());
        this.show_tray();
        this.jump_to_latest();
    });
    let this = ui.clone();
    window.on_resize_tray(move |by| this.resize_tray(by));
    let this = ui.clone();
    window.on_tray_scrolled(move |at_end| {
        this.follow.set(Follow::after_scroll(at_end));
        this.with_window(|window| window.set_following(at_end));
    });
    let this = ui.clone();
    window.on_jump_to_latest(move || this.jump_to_latest());
}

fn wire_confirm(ui: &Rc<Ui>, window: &MainWindow) {
    let this = ui.clone();
    window.on_confirm(move || {
        this.with_window(|window| window.set_show_confirm(false));
        match this.pending.borrow_mut().take() {
            Some(Pending::Remove(id)) => {
                let removed = this.app.borrow_mut().remove(&id);
                if this.report(removed).is_some() {
                    this.output
                        .show(this.app.borrow().selection().selected().cloned());
                    this.refresh();
                }
            }
            Some(Pending::Close) => {
                this.app.borrow_mut().stop_all();
                this.with_window(|window| {
                    // Hiding the last window ends the event loop.
                    let _ = window.hide();
                });
            }
            None => {}
        }
    });
    let this = ui.clone();
    window.on_cancel_confirm(move || {
        this.pending.borrow_mut().take();
        this.with_window(|window| window.set_show_confirm(false));
    });
}

impl Ui {
    /// Selects operation `id` and shows its output, following the end (ROW-007).
    fn select(&self, id: &OperationId) {
        let selected = self.app.borrow_mut().select(id);
        if selected.is_ok() {
            self.output.show(Some(id.clone()));
            self.jump_to_latest();
        }
        self.report(selected);
    }

    fn jump_to_latest(&self) {
        self.follow.set(Follow::jump_to_latest());
        self.with_window(|window| {
            window.set_following(true);
            window.invoke_scroll_output_to_end();
        });
    }
}
