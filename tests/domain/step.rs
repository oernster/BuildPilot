use std::path::Path;

use buildpilot::domain::step::{StepList, StepSpec};

fn step(script: &str) -> StepSpec {
    StepSpec::for_script(Path::new(script))
}

fn list(scripts: &[&str]) -> StepList {
    StepList::new(scripts.iter().map(|script| step(script)).collect())
}

// STEP-008: an empty list still holds one step to fill in.
#[test]
fn a_new_list_is_never_empty() {
    let empty = StepList::new(Vec::new());
    assert_eq!(empty.steps(), [StepSpec::default()]);
    assert_eq!(empty.labels(), ["1. (no script)"]);
}

// STEP-008: the list shows each step by number and file name.
#[test]
fn labels_number_the_steps() {
    let steps = list(&[r"C:\a\buildexe.py", r"C:\a\buildinstaller.py"]);
    assert_eq!(steps.labels(), ["1. buildexe.py", "2. buildinstaller.py"]);
}

// STEP-008: selection moves only to a step that exists; typing replaces the selected step.
#[test]
fn selecting_and_typing() {
    let mut steps = list(&[r"C:\a\one.ps1", r"C:\a\two.ps1"]);
    assert!(steps.select(1));
    assert!(!steps.select(2));
    assert_eq!(steps.selected(), 1);
    steps.set_current(step(r"C:\a\changed.ps1"));
    assert_eq!(steps.current(), &step(r"C:\a\changed.ps1"));
    assert_eq!(steps.steps()[0], step(r"C:\a\one.ps1"));
}

// STEP-008: a step is added last and selected.
#[test]
fn adding_selects_the_new_last_step() {
    let mut steps = list(&[r"C:\a\one.ps1"]);
    steps.add(step(r"C:\a\two.ps1"));
    assert_eq!(steps.selected(), 1);
    assert_eq!(steps.current(), &step(r"C:\a\two.ps1"));
}

// STEP-008: the only step cannot be removed; removing selects its neighbour.
#[test]
fn removing_keeps_at_least_one_step() {
    let mut steps = list(&[r"C:\a\one.ps1", r"C:\a\two.ps1", r"C:\a\three.ps1"]);
    steps.select(1);
    assert!(steps.remove());
    assert_eq!(steps.current(), &step(r"C:\a\three.ps1"));
    assert!(steps.remove());
    assert_eq!(steps.current(), &step(r"C:\a\one.ps1"));
    assert!(!steps.remove());
    assert_eq!(steps.steps().len(), 1);
}

// STEP-008: moving swaps with the neighbour and keeps the step selected; not past either end.
#[test]
fn moving_up_and_down() {
    let mut steps = list(&[r"C:\a\one.ps1", r"C:\a\two.ps1"]);
    assert!(!steps.move_up());
    assert!(steps.move_down());
    assert_eq!(steps.selected(), 1);
    assert_eq!(steps.labels(), ["1. two.ps1", "2. one.ps1"]);
    assert!(!steps.move_down());
    assert!(steps.move_up());
    assert_eq!(steps.labels(), ["1. one.ps1", "2. two.ps1"]);
}
