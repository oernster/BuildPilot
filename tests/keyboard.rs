//! A11Y-002 and A11Y-003, the house keyboard model, driven headless: real key events through
//! Slint's own focus handling, read back through the `Ring.focused` instrument each stop keeps.
//! What a ring LOOKS like is not measured here; only which control holds focus.

use std::cell::RefCell;
use std::rc::Rc;

use buildpilot::ui::{MainWindow, Ring, RowData, step_ring};
use slint::platform::{Key, WindowEvent};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

/// The toolbar, left to right.
const TOOLBAR: [&str; 4] = [
    "Add a build script",
    "Switch to dark theme",
    "Settings",
    "Help and About",
];
const SHOW_OUTPUT: &str = "Show output";

fn window() -> MainWindow {
    i_slint_backend_testing::init_no_event_loop();
    let window = MainWindow::new().unwrap();
    let weak = window.as_weak();
    window.on_step_ring(move |forward| step_ring(weak.upgrade().unwrap().window(), forward));
    window.show().unwrap();
    window
}

/// One event loop iteration's housekeeping, which runs the window's change handlers.
fn settle() {
    slint::platform::update_timers_and_animations();
}

fn press(window: &MainWindow, key: impl Into<SharedString>) {
    let text = key.into();
    window
        .window()
        .dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    window
        .window()
        .dispatch_event(WindowEvent::KeyReleased { text });
    settle();
}

fn focused(window: &MainWindow) -> String {
    window.global::<Ring>().get_focused().to_string()
}

/// The stops `count` presses of `key` visit, in order.
fn walk(window: &MainWindow, key: Key, count: usize) -> Vec<String> {
    (0..count)
        .map(|_| {
            press(window, key);
            focused(window)
        })
        .collect()
}

fn row(id: &str, name: &str, selected: bool, running: bool) -> RowData {
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

fn with_rows(window: &MainWindow, rows: Vec<RowData>) {
    let selected = rows
        .iter()
        .position(|row| row.selected)
        .map_or(-1, |at| at as i32);
    window.set_rows(ModelRc::new(VecModel::from(rows)));
    window.set_selected_index(selected);
}

/// The stops of the empty window, in reading order.
fn empty_ring() -> Vec<String> {
    TOOLBAR
        .iter()
        .chain(&["Add a build script", SHOW_OUTPUT])
        .map(|label| (*label).to_owned())
        .collect()
}

// A11Y-003 and the house model: the main window opens with nothing focused.
#[test]
fn the_window_opens_with_nothing_focused() {
    let window = window();
    assert_eq!(focused(&window), "");
}

// A11Y-003: Tab reaches every control in reading order and wraps from the last to the first.
#[test]
fn tab_walks_every_control_and_wraps() {
    let window = window();
    let ring = empty_ring();
    let mut expected = ring.clone();
    expected.push(ring[0].clone());
    assert_eq!(walk(&window, Key::Tab, ring.len() + 1), expected);
}

// A11Y-003: Shift+Tab walks the same ring backwards, wrapping from the first to the last.
#[test]
fn shift_tab_walks_back_and_wraps() {
    let window = window();
    press(&window, Key::Tab);
    // From the first stop, back past the start to the last and on round to the first again.
    let ring = empty_ring();
    let expected: Vec<String> = ring.iter().rev().cloned().collect();
    assert_eq!(walk(&window, Key::Backtab, ring.len()), expected);
}

// The house model: Right is Tab and Left is Shift+Tab, everywhere a control does not use them.
#[test]
fn right_and_left_step_the_ring() {
    let window = window();
    let forward = walk(&window, Key::RightArrow, 3);
    assert_eq!(forward, TOOLBAR[..3]);
    assert_eq!(
        walk(&window, Key::LeftArrow, 2),
        TOOLBAR[..2].iter().rev().copied().collect::<Vec<_>>()
    );
}

// The rows are one stop; the selected row's enabled controls follow it; other rows' do not.
#[test]
fn the_selected_rows_controls_follow_the_list() {
    let window = window();
    with_rows(
        &window,
        vec![
            row("a", "Alpha", true, false),
            row("b", "Beta", false, false),
        ],
    );
    let expected: Vec<String> = TOOLBAR
        .iter()
        .map(|label| (*label).to_owned())
        .chain(
            [
                "Build scripts",
                "Select Alpha",
                "Run Alpha",
                // Stop Alpha is disabled (not running), so it is passed over.
                "Edit Alpha",
                "Open script",
                "Show script in Explorer",
                "Remove Alpha from BuildPilot",
                SHOW_OUTPUT,
                TOOLBAR[0],
            ]
            .map(str::to_owned),
        )
        .collect();
    assert_eq!(walk(&window, Key::Tab, expected.len()), expected);
}

// Up and Down walk the rows, wrapping; Space checks the selected row; Enter edits it.
#[test]
fn up_and_down_walk_the_rows_and_wrap() {
    let window = window();
    with_rows(
        &window,
        vec![
            row("a", "Alpha", true, false),
            row("b", "Beta", false, false),
        ],
    );
    let chosen = Rc::new(RefCell::new(Vec::new()));
    let log = chosen.clone();
    window.on_select(move |id| log.borrow_mut().push(format!("select {id}")));
    let log = chosen.clone();
    window.on_toggle_checked(move |id| log.borrow_mut().push(format!("check {id}")));
    let log = chosen.clone();
    window.on_edit(move |id| log.borrow_mut().push(format!("edit {id}")));

    walk(&window, Key::Tab, TOOLBAR.len() + 1);
    assert_eq!(focused(&window), "Build scripts");
    press(&window, Key::DownArrow);
    press(&window, Key::UpArrow);
    window.set_selected_index(1);
    press(&window, Key::DownArrow);
    press(&window, " ");
    press(&window, Key::Return);
    assert_eq!(
        *chosen.borrow(),
        ["select b", "select b", "select a", "check b", "edit b"]
    );
}

// A dialog opens on its first control, keeps the ring to itself, closes on Escape and hands
// focus back to the control that opened it.
#[test]
fn a_dialog_owns_the_ring_while_open() {
    let window = window();
    let closed = Rc::new(RefCell::new(0));
    let count = closed.clone();
    window.on_close_settings(move || *count.borrow_mut() += 1);
    let opener = window.as_weak();
    window.on_open_settings(move || opener.upgrade().unwrap().set_show_settings(true));

    walk(&window, Key::Tab, 3);
    assert_eq!(focused(&window), "Settings");
    press(&window, " ");
    assert_eq!(focused(&window), "Light theme");

    let dialog = [
        "Dark theme",
        "Follow the Windows theme",
        "Open the data folder",
        "Close",
        "Light theme",
    ];
    assert_eq!(walk(&window, Key::Tab, dialog.len()), dialog);

    press(&window, Key::Escape);
    assert_eq!(*closed.borrow(), 1);
    window.set_show_settings(false);
    settle();
    assert_eq!(focused(&window), "Settings");
}

// The Add and Edit dialog opens in its first field; a field keeps Left and Right for its caret
// and is left with Tab; the ring runs through every field and button and wraps.
#[test]
fn the_operation_dialog_walks_fields_and_buttons() {
    let window = window();
    window.set_operation_save_label("Add".into());
    let opener = window.as_weak();
    window.on_add(move || opener.upgrade().unwrap().set_show_operation_dialog(true));
    press(&window, Key::Tab);
    press(&window, " ");
    assert_eq!(focused(&window), "Name");
    press(&window, Key::RightArrow);
    assert_eq!(focused(&window), "Name");

    let ring = [
        "Script",
        "Browse for the script",
        "Working directory",
        "Browse for the working directory",
        "Arguments, one per line",
        "Choose image",
        "Use placeholder",
        "Cancel",
        "Add",
        "Name",
    ];
    assert_eq!(walk(&window, Key::Tab, ring.len()), ring);
}

// Every other dialog opens already focused on its first control, the safe one where it asks.
#[test]
fn dialogs_open_on_their_first_control() {
    let window = window();
    with_rows(&window, vec![row("a", "Alpha", true, false)]);
    window.set_confirm_label("Remove".into());
    let opener = window.as_weak();
    window.on_remove(move |_| opener.upgrade().unwrap().set_show_confirm(true));
    walk(&window, Key::Backtab, 2);
    assert_eq!(focused(&window), "Remove Alpha from BuildPilot");
    press(&window, " ");
    assert_eq!(focused(&window), "Cancel");

    // Closing hands focus back to the rows the removal was asked from.
    window.set_show_confirm(false);
    settle();
    assert_eq!(focused(&window), "Build scripts");

    let opener = window.as_weak();
    window.on_open_about(move || opener.upgrade().unwrap().set_show_about(true));
    press(&window, Key::Backtab);
    assert_eq!(focused(&window), "Help and About");
    press(&window, Key::Return);
    assert_eq!(focused(&window), "Close");
}

/// The window's preferred size, from ui/main.slint.
const WINDOW_WIDTH: f32 = 1100.0;
const WINDOW_HEIGHT: f32 = 760.0;
/// Output lines enough to overflow any tray.
const OVERFLOWING_LINES: usize = 200;

// The output is a stop only while it has more lines than fit; it follows the tray's buttons.
#[test]
fn the_output_is_a_stop_only_while_it_overflows() {
    let window = window();
    window
        .window()
        .set_size(slint::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT));
    window.set_tray_expanded(true);
    let line = |text: &str| buildpilot::ui::OutputLineData {
        text: text.into(),
        err: false,
    };
    window.set_lines(ModelRc::new(VecModel::from(vec![line("one line")])));
    settle();
    assert!(!walk(&window, Key::Tab, empty_ring().len() + 1).contains(&"Output".to_owned()));

    let many: Vec<_> = (0..OVERFLOWING_LINES)
        .map(|n| line(&n.to_string()))
        .collect();
    window.set_lines(ModelRc::new(VecModel::from(many)));
    settle();
    let stops = walk(&window, Key::Tab, empty_ring().len() + 1);
    let output = stops.iter().position(|stop| stop == "Output");
    let show = stops.iter().position(|stop| stop == "Hide output");
    assert!(
        matches!((show, output), (Some(s), Some(o)) if o == s + 1),
        "{stops:?}"
    );
}
