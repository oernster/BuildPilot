//! The dialogs' and the Help menu's rings.

use std::cell::RefCell;
use std::rc::Rc;

use buildpilot::ui::{ScanList, ScannedRow, StepEditor};
use slint::platform::Key;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use super::support::*;

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

    let settings = TOOLBAR.iter().position(|stop| *stop == "Settings").unwrap();
    walk(&window, Key::Tab, settings + 1);
    assert_eq!(focused(&window), "Settings");
    press(&window, " ");
    assert_eq!(focused(&window), "Light theme");

    let dialog = [
        "Dark theme",
        "Open the data folder",
        "File type",
        "Program",
        "Browse for the host program",
        "Host arguments before the script, one per line",
        "Add this host to the table",
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
    // STEP-008: two steps, the first selected, so Up is disabled and left off the ring.
    let labels: Vec<SharedString> = vec!["1. buildexe.py".into(), "2. buildinstaller.py".into()];
    window
        .global::<StepEditor>()
        .set_labels(ModelRc::new(VecModel::from(labels)));
    let opener = window.as_weak();
    window.on_add(move || opener.upgrade().unwrap().set_show_operation_dialog(true));
    press(&window, Key::Tab);
    press(&window, " ");
    assert_eq!(focused(&window), "Name");
    press(&window, Key::RightArrow);
    assert_eq!(focused(&window), "Name");

    let ring = [
        "Step 1. buildexe.py",
        "Step 2. buildinstaller.py",
        "Add a step after the last",
        "Remove the selected step",
        "Run the selected step later",
        "Script of step 1",
        "Browse for the script",
        "Working directory",
        "Browse for the working directory",
        "Arguments of step 1, one per line",
        "Choose image",
        "Use placeholder",
        "Cancel",
        "Add",
        "Name",
    ];
    assert_eq!(walk(&window, Key::Tab, ring.len()), ring);
}

// ENV-002: where several environments are offered, the choice is a stop on the ring, between
// the working directory and the arguments.
#[test]
fn the_environment_choice_is_on_the_ring() {
    let window = window();
    let editor = window.global::<StepEditor>();
    let names: Vec<SharedString> = vec!["venv".into(), "venv_smoke".into()];
    editor.set_environments(ModelRc::new(VecModel::from(names)));
    editor.set_environment("venv".into());
    editor.set_choose_environment(true);
    let opener = window.as_weak();
    window.on_add(move || opener.upgrade().unwrap().set_show_operation_dialog(true));
    press(&window, Key::Tab);
    press(&window, " ");
    let stops = walk(&window, Key::Tab, 7);
    assert_eq!(
        stops[4..],
        [
            "Browse for the working directory",
            "Environment",
            "Arguments of step 1, one per line"
        ]
    );
}

// SCAN-002: Add's choice opens on Script and rings Cancel, Script and Folder.
#[test]
fn the_add_choice_opens_on_script() {
    let window = window();
    let opener = window.as_weak();
    window.on_add(move || {
        let window = opener.upgrade().unwrap();
        window.global::<ScanList>().set_choosing(true);
    });
    press(&window, Key::Tab);
    press(&window, " ");
    assert_eq!(focused(&window), "Choose a build script");
    assert_eq!(
        walk(&window, Key::Tab, 3),
        [
            "Choose a folder to recognise",
            "Cancel",
            "Choose a build script"
        ]
    );
}

// SCAN-005, SCAN-006: the list opens on Cancel; each project's tick box is a stop, except one
// already on the flight deck, which cannot be ticked.
#[test]
fn the_project_list_rings_its_tick_boxes() {
    let window = window();
    let row = |name: &str, warning: &str| ScannedRow {
        name: name.into(),
        steps: "build.ps1".into(),
        ticked: true,
        warning: warning.into(),
        note: SharedString::default(),
        locked: false,
    };
    let list = window.global::<ScanList>();
    list.set_rows(ModelRc::new(VecModel::from(vec![
        row(
            "edca",
            "buildexe.py was not found: only buildinstaller.py will run",
        ),
        ScannedRow {
            ticked: false,
            note: "Already on the flight deck".into(),
            locked: true,
            ..row("stellody", "")
        },
        row("pigeonpost", ""),
    ])));
    let opener = window.as_weak();
    window.on_add(move || {
        let window = opener.upgrade().unwrap();
        window.global::<ScanList>().set_listing(true);
    });
    press(&window, Key::Tab);
    press(&window, " ");
    assert_eq!(focused(&window), "Cancel");
    assert_eq!(
        walk(&window, Key::Tab, 4),
        [
            "Add the ticked projects",
            "Add edca",
            "Add pigeonpost",
            "Cancel"
        ]
    );
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
    window.on_help(move |entry| {
        if entry == "about" {
            opener.upgrade().unwrap().set_show_about(true);
        }
    });
    press(&window, Key::Backtab);
    assert_eq!(focused(&window), "Help");
    press(&window, Key::Return);
    assert_eq!(focused(&window), "Guide");
    press(&window, Key::DownArrow);
    assert_eq!(focused(&window), "About BuildPilot");
    press(&window, Key::Return);
    assert_eq!(focused(&window), "Close");
    window.set_show_about(false);
    settle();
    assert_eq!(focused(&window), "Help");
}

// UI-006: the Help menu opens on its first entry, Up and Down walk it and wrap, Escape closes it
// and hands focus back to the Help button.
#[test]
fn the_help_menu_walks_with_up_and_down() {
    let window = window();
    walk(&window, Key::Tab, TOOLBAR.len());
    assert_eq!(focused(&window), "Help");
    press(&window, Key::Return);
    assert_eq!(focused(&window), "Guide");
    assert_eq!(
        walk(&window, Key::DownArrow, 4),
        ["About BuildPilot", "Licence", "Check for Updates", "Guide"]
    );
    assert_eq!(walk(&window, Key::UpArrow, 1), ["Check for Updates"]);
    press(&window, Key::Escape);
    assert!(!window.get_show_help_menu());
    assert_eq!(focused(&window), "Help");
}
