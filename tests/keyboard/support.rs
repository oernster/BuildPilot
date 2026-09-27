//! The headless window and key presses every keyboard test drives, written once.

use buildpilot::ui::{MainWindow, Ring, RowData, step_ring};
use slint::platform::{Key, WindowEvent};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

/// The toolbar, left to right.
pub const TOOLBAR: [&str; 5] = [
    "Add a build script",
    "Buy the author a drink (opens your browser)",
    "Switch to dark theme",
    "Settings",
    "Help",
];
pub const SHOW_OUTPUT: &str = "Show output";

pub fn window() -> MainWindow {
    i_slint_backend_testing::init_no_event_loop();
    let window = MainWindow::new().unwrap();
    let weak = window.as_weak();
    window.on_step_ring(move |forward| step_ring(weak.upgrade().unwrap().window(), forward));
    window.show().unwrap();
    window
}

/// One event loop iteration's housekeeping, which runs the window's change handlers.
pub fn settle() {
    slint::platform::update_timers_and_animations();
}

pub fn press(window: &MainWindow, key: impl Into<SharedString>) {
    let text = key.into();
    window
        .window()
        .dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    window
        .window()
        .dispatch_event(WindowEvent::KeyReleased { text });
    settle();
}

pub fn focused(window: &MainWindow) -> String {
    window.global::<Ring>().get_focused().to_string()
}

/// The stops `count` presses of `key` visit, in order.
pub fn walk(window: &MainWindow, key: Key, count: usize) -> Vec<String> {
    (0..count)
        .map(|_| {
            press(window, key);
            focused(window)
        })
        .collect()
}

pub fn row(id: &str, name: &str, selected: bool, running: bool) -> RowData {
    RowData {
        id: id.into(),
        name: name.into(),
        selected,
        can_run: !running,
        can_stop: running,
        can_remove: !running,
        ..Default::default()
    }
}

pub fn with_rows(window: &MainWindow, rows: Vec<RowData>) {
    let selected = rows
        .iter()
        .position(|row| row.selected)
        .map_or(-1, |at| at as i32);
    window.set_rows(ModelRc::new(VecModel::from(rows)));
    window.set_selected_index(selected);
}

/// The stops of the empty window, in reading order.
pub fn empty_ring() -> Vec<String> {
    TOOLBAR
        .iter()
        .chain(&["Add a build script", SHOW_OUTPUT])
        .map(|label| (*label).to_owned())
        .collect()
}
