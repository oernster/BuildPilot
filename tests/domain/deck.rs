use buildpilot::domain::deck::{DeckError, FlightDeck};

use super::support::{config, id, operation};

fn deck(ids: &[&str]) -> FlightDeck {
    FlightDeck::new(ids.iter().map(|value| operation(value)).collect()).unwrap()
}

fn order(deck: &FlightDeck) -> Vec<&str> {
    deck.operations()
        .iter()
        .map(|op| op.id().as_str())
        .collect()
}

#[test]
fn new_keeps_the_given_order_and_refuses_repeats() {
    assert_eq!(order(&deck(&["a", "b", "c"])), ["a", "b", "c"]);
    let repeated = FlightDeck::new(vec![operation("a"), operation("a")]);
    assert_eq!(repeated.unwrap_err(), DeckError::DuplicateId(id("a")));
}

// CFG-007
#[test]
fn empty_deck_reports_empty() {
    assert!(FlightDeck::default().is_empty());
    assert!(!deck(&["a"]).is_empty());
}

#[test]
fn add_appends_as_the_bottom_row() {
    let mut deck = deck(&["a"]);
    deck.add(operation("b")).unwrap();
    assert_eq!(order(&deck), ["a", "b"]);
    assert_eq!(deck.position(&id("b")), Some(1));
    assert_eq!(
        deck.add(operation("a")).unwrap_err(),
        DeckError::DuplicateId(id("a"))
    );
}

// EDIT-001: an edit keeps identity and position.
#[test]
fn replace_config_keeps_position() {
    let mut deck = deck(&["a", "b"]);
    let replacement = config(r"C:\src\elsewhere\build.ps1");
    deck.replace_config(&id("b"), replacement.clone()).unwrap();
    assert_eq!(order(&deck), ["a", "b"]);
    assert_eq!(deck.get(&id("b")).unwrap().config(), &replacement);
    assert_eq!(
        deck.replace_config(&id("z"), replacement).unwrap_err(),
        DeckError::NotFound(id("z"))
    );
}

// REM-002
#[test]
fn remove_takes_out_one_row() {
    let mut deck = deck(&["a", "b", "c"]);
    assert_eq!(deck.remove(&id("b")).unwrap().id(), &id("b"));
    assert_eq!(order(&deck), ["a", "c"]);
    assert_eq!(
        deck.remove(&id("b")).unwrap_err(),
        DeckError::NotFound(id("b"))
    );
}

// ROW-003: drag to a row, both directions.
#[test]
fn move_to_places_the_row_at_the_target() {
    let mut deck = deck(&["a", "b", "c", "d"]);
    deck.move_to(&id("d"), 0).unwrap();
    assert_eq!(order(&deck), ["d", "a", "b", "c"]);
    deck.move_to(&id("d"), 3).unwrap();
    assert_eq!(order(&deck), ["a", "b", "c", "d"]);
    deck.move_to(&id("a"), 2).unwrap();
    assert_eq!(order(&deck), ["b", "c", "a", "d"]);
}

#[test]
fn move_to_refuses_a_row_past_the_end() {
    let mut deck = deck(&["a", "b"]);
    let error = deck.move_to(&id("a"), 2).unwrap_err();
    assert_eq!(error, DeckError::PositionOutOfRange { target: 2, rows: 2 });
    assert_eq!(order(&deck), ["a", "b"]);
    assert_eq!(
        deck.move_to(&id("z"), 0).unwrap_err(),
        DeckError::NotFound(id("z"))
    );
}

// ROW-004
#[test]
fn move_up_and_down() {
    let mut deck = deck(&["a", "b", "c"]);
    assert!(deck.move_up(&id("b")).unwrap());
    assert_eq!(order(&deck), ["b", "a", "c"]);
    assert!(deck.move_down(&id("b")).unwrap());
    assert_eq!(order(&deck), ["a", "b", "c"]);
}

// ROW-004: at an edge the move is a no-op, not an error.
#[test]
fn move_at_edge_is_no_op() {
    let mut deck = deck(&["a", "b"]);
    assert!(!deck.move_up(&id("a")).unwrap());
    assert!(!deck.move_down(&id("b")).unwrap());
    assert_eq!(order(&deck), ["a", "b"]);
    assert_eq!(
        deck.move_up(&id("z")).unwrap_err(),
        DeckError::NotFound(id("z"))
    );
    assert_eq!(
        deck.move_down(&id("z")).unwrap_err(),
        DeckError::NotFound(id("z"))
    );
}

#[test]
fn errors_have_messages() {
    let messages = [
        DeckError::DuplicateId(id("a")).to_string(),
        DeckError::NotFound(id("a")).to_string(),
        DeckError::PositionOutOfRange { target: 5, rows: 2 }.to_string(),
    ];
    assert!(messages[0].contains("already exists"));
    assert!(messages[1].contains("No operation"));
    assert!(messages[2].contains("row 5") && messages[2].contains("2 rows"));
}
