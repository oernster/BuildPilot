use buildpilot::domain::line_assembler::{LineAssembler, MAX_PENDING_BYTES};

#[test]
fn complete_lines_are_released_at_once() {
    let mut assembler = LineAssembler::new();
    assert_eq!(assembler.push(b"one\ntwo\r\n"), ["one", "two"]);
    assert_eq!(assembler.finish(), None);
}

#[test]
fn a_line_split_across_chunks_is_joined() {
    let mut assembler = LineAssembler::new();
    assert!(assembler.push(b"Compil").is_empty());
    assert_eq!(assembler.push(b"ing\nFin"), ["Compiling"]);
    assert_eq!(assembler.finish(), Some("Fin".to_owned()));
    assert_eq!(assembler.finish(), None);
}

// OUT-008: a character split across chunks is not damaged.
#[test]
fn a_character_split_across_chunks_survives() {
    let bytes = "caf\u{e9}\n".as_bytes();
    let mut assembler = LineAssembler::new();
    assert!(assembler.push(&bytes[..4]).is_empty());
    assert_eq!(assembler.push(&bytes[4..]), ["caf\u{e9}"]);
}

#[test]
fn empty_lines_are_kept() {
    let mut assembler = LineAssembler::new();
    assert_eq!(assembler.push(b"\n\n"), ["", ""]);
}

// Robustness: a line with no newline is released in pieces, never held without limit.
#[test]
fn a_runaway_line_is_released_at_the_cap() {
    let mut assembler = LineAssembler::new();
    let released = assembler.push(&vec![b'x'; MAX_PENDING_BYTES * 2 + 1]);
    assert_eq!(released.len(), 2);
    assert!(
        released
            .iter()
            .all(|piece| piece.len() == MAX_PENDING_BYTES)
    );
    assert_eq!(assembler.finish(), Some("x".to_owned()));
}

// The cap backs off to a character boundary rather than splitting one.
#[test]
fn the_cap_never_splits_a_character() {
    let mut bytes = vec![b'x'; MAX_PENDING_BYTES - 1];
    bytes.extend_from_slice("\u{e9}\u{e9}".as_bytes());
    let mut assembler = LineAssembler::new();
    let released = assembler.push(&bytes);
    assert_eq!(released.len(), 1);
    assert!(!released[0].contains('\u{fffd}'));
    assert_eq!(released[0].len(), MAX_PENDING_BYTES - 1);
    assert_eq!(assembler.finish(), Some("\u{e9}\u{e9}".to_owned()));
}

// Bytes that are all continuation bytes have no boundary; the cut falls at the cap.
#[test]
fn invalid_run_is_cut_at_the_cap() {
    let mut assembler = LineAssembler::new();
    let released = assembler.push(&vec![0x80; MAX_PENDING_BYTES + 1]);
    assert_eq!(released.len(), 1);
    assert_eq!(released[0].chars().count(), MAX_PENDING_BYTES);
}
