//! The main window's ring: toolbar, rows and output.

use std::cell::RefCell;
use std::rc::Rc;

use slint::platform::Key;
use slint::{ComponentHandle, ModelRc, VecModel};

use super::support::*;

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
                "Build Alpha",
                // Stop Alpha is disabled (not running), so it is passed over; so is Launch
                // installer, which has none (PKG-003).
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
        kind: buildpilot::ui::LineKind::Plain,
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
