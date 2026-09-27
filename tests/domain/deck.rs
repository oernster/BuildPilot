use buildpilot::domain::deck::{DeckError, FlightDeck};
use buildpilot::domain::operation::{Operation, OperationConfig};

use super::support::{config, id, operation, spec};

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

/// An operation with identity `value` named `name`.
fn named(value: &str, name: &str) -> Operation {
    let mut spec = spec(&format!(r"C:\src\{value}\build.ps1"));
    spec.name = name.to_owned();
    Operation::new(id(value), OperationConfig::try_from(spec).unwrap())
}

fn names(deck: &FlightDeck) -> Vec<&str> {
    deck.operations()
        .iter()
        .map(|op| op.config().name())
        .collect()
}

// ROW-008: a new row goes before the first whose name sorts after its own, whatever the case;
// an equal name goes after. A deck in name order stays in order.
#[test]
fn insert_by_name_keeps_a_sorted_deck_sorted() {
    let mut deck = FlightDeck::default();
    for (value, name) in [
        ("s", "Stellody"),
        ("a", "axisdb"),
        ("p", "PigeonPost"),
        ("z", "zebra"),
        ("b", "BuildPilot"),
        ("s2", "stellody"),
    ] {
        deck.insert_by_name(named(value, name)).unwrap();
    }
    assert_eq!(
        names(&deck),
        [
            "axisdb",
            "BuildPilot",
            "PigeonPost",
            "Stellody",
            "stellody",
            "zebra"
        ]
    );
    assert_eq!(
        deck.insert_by_name(named("a", "again")).unwrap_err(),
        DeckError::DuplicateId(id("a"))
    );
}

// ROW-008: rows the operator arranged are never moved; the new one lands before the first row
// whose name sorts after it.
#[test]
fn insert_by_name_leaves_an_arranged_deck_alone() {
    let mut deck = FlightDeck::new(vec![named("z", "zebra"), named("a", "alpha")]).unwrap();
    assert_eq!(deck.insert_by_name(named("m", "middle")), Ok(0));
    assert_eq!(names(&deck), ["middle", "zebra", "alpha"]);
    assert_eq!(deck.insert_by_name(named("zz", "zz top")), Ok(3));
    assert_eq!(names(&deck), ["middle", "zebra", "alpha", "zz top"]);
}

// EDIT-001: an edit keeps identity and position.
#[test]
fn replace_config_keeps_position() {
    let mut deck = deck(&["a", "b"]);
    let replacement = config(r"C:\src\elsewhere\build.ps1");
    let index = deck.position(&id("b")).unwrap();
    deck.replace_config_at(index, replacement.clone());
    assert_eq!(order(&deck), ["a", "b"]);
    assert_eq!(deck.get(&id("b")).unwrap().config(), &replacement);
    // A position with no operation changes nothing.
    let before = deck.clone();
    deck.replace_config_at(deck.operations().len(), replacement);
    assert_eq!(deck, before);
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
