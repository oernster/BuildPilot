use buildpilot::domain::selection::DeckSelection;

use super::support::id;

// ROW-007
#[test]
fn selecting_replaces_the_previous_selection() {
    let mut selection = DeckSelection::default();
    assert_eq!(selection.selected(), None);
    selection.select(id("a"));
    selection.select(id("b"));
    assert_eq!(selection.selected(), Some(&id("b")));
}

// ROW-006: checking and selecting are independent.
#[test]
fn checking_toggles_without_selecting() {
    let mut selection = DeckSelection::default();
    assert!(selection.toggle_checked(&id("a")));
    assert!(selection.toggle_checked(&id("b")));
    assert!(selection.is_checked(&id("a")));
    assert_eq!(selection.selected(), None);
    assert!(!selection.toggle_checked(&id("a")));
    assert!(!selection.is_checked(&id("a")));
    assert_eq!(selection.checked().collect::<Vec<_>>(), [&id("b")]);
}

// REM-002: a removed operation leaves nothing behind.
#[test]
fn forget_clears_selection_and_check() {
    let mut selection = DeckSelection::default();
    selection.select(id("a"));
    selection.toggle_checked(&id("a"));
    selection.forget(&id("a"));
    assert_eq!(selection.selected(), None);
    assert!(!selection.is_checked(&id("a")));
}

#[test]
fn forget_leaves_other_selection_alone() {
    let mut selection = DeckSelection::default();
    selection.select(id("a"));
    selection.forget(&id("b"));
    assert_eq!(selection.selected(), Some(&id("a")));
}
