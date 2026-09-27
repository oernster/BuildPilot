//! Turning a script's raw output bytes into readable text (OUT-008). BuildPilot is not a
//! terminal emulator, so escape sequences are removed rather than interpreted.

const ESC: char = '\u{1b}';
const BEL: char = '\u{7}';
/// Control Sequence Introducer, the character after ESC that opens a CSI sequence.
const CSI_OPEN: char = '[';
/// Operating System Command opener, the character after ESC that opens an OSC sequence.
const OSC_OPEN: char = ']';
/// ESC followed by this ends an OSC sequence (the "string terminator").
const STRING_TERMINATOR: char = '\\';
/// A CSI sequence ends at the first character in this range.
const CSI_FINAL: std::ops::RangeInclusive<char> = '@'..='~';

/// One line of output bytes (without its `\n`) as text: invalid UTF-8 replaced with U+FFFD, a
/// trailing `\r` removed, only the text after the last remaining `\r` kept (so a progress
/// counter that rewrites itself shows its final value) and ANSI escape sequences removed.
pub fn decode_line(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let text = text.strip_suffix('\r').unwrap_or(&text);
    let visible = match text.rfind('\r') {
        Some(index) => &text[index + 1..],
        None => text,
    };
    strip_ansi(visible)
}

/// `text` with ANSI escape sequences removed: CSI sequences (colours, cursor movement), OSC
/// sequences (window titles, links) and two-character escapes.
pub fn strip_ansi(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != ESC {
            plain.push(c);
            continue;
        }
        match chars.next() {
            Some(CSI_OPEN) => {
                for c in chars.by_ref() {
                    if CSI_FINAL.contains(&c) {
                        break;
                    }
                }
            }
            Some(OSC_OPEN) => {
                while let Some(c) = chars.next() {
                    if c == BEL {
                        break;
                    }
                    if c == ESC {
                        chars.next_if_eq(&STRING_TERMINATOR);
                        break;
                    }
                }
            }
            Some(_) | None => {}
        }
    }
    plain
}
