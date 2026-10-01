use buildpilot::domain::text::{decode_line, lossy_utf8, strip_ansi};

fn decode(bytes: &[u8]) -> String {
    decode_line(bytes, lossy_utf8)
}

/// A stand-in for a code-page decoder: reads every byte as the Latin-1 character of that value.
fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&byte| char::from(byte)).collect()
}

// OUT-008
#[test]
fn plain_text_passes_through() {
    assert_eq!(decode(b"Compiling buildpilot"), "Compiling buildpilot");
    assert_eq!(decode("caf\u{e9}".as_bytes()), "caf\u{e9}");
}

// OUT-008: without a better decoder, invalid UTF-8 becomes U+FFFD.
#[test]
fn invalid_bytes_are_replaced() {
    assert_eq!(decode(b"bad \xff byte"), "bad \u{fffd} byte");
}

// Amendment 2: a line that is not UTF-8 goes to the fallback; a line that is never does.
#[test]
fn only_invalid_utf8_reaches_the_fallback() {
    assert_eq!(decode_line(b"caf\xe9\r", latin1), "caf\u{e9}");
    assert_eq!(decode_line("caf\u{e9}".as_bytes(), latin1), "caf\u{e9}");
    assert_eq!(decode_line(b"\x1b[31mcaf\xe9\x1b[0m", latin1), "caf\u{e9}");
}

#[test]
fn lossy_utf8_replaces_invalid_bytes() {
    assert_eq!(lossy_utf8(b"a\xffb"), "a\u{fffd}b");
}

#[test]
fn trailing_carriage_return_is_removed() {
    assert_eq!(decode(b"windows line\r"), "windows line");
}

#[test]
fn rewritten_line_shows_its_final_value() {
    assert_eq!(decode(b"10%\r50%\r100%"), "100%");
    assert_eq!(decode(b"10%\r100%\r"), "100%");
}

// OUT-008: colour codes removed.
#[test]
fn csi_sequences_are_removed() {
    assert_eq!(
        strip_ansi("\u{1b}[31;1merror\u{1b}[0m: failed"),
        "error: failed"
    );
    assert_eq!(strip_ansi("\u{1b}[2Kline"), "line");
}

#[test]
fn osc_sequences_are_removed_with_either_terminator() {
    assert_eq!(strip_ansi("\u{1b}]0;title\u{7}text"), "text");
    assert_eq!(strip_ansi("\u{1b}]8;;http://x\u{1b}\\link"), "link");
    assert_eq!(strip_ansi("\u{1b}]0;cut\u{1b}rest"), "rest");
}

#[test]
fn short_and_truncated_escapes_are_removed() {
    assert_eq!(strip_ansi("a\u{1b}=b"), "ab");
    assert_eq!(strip_ansi("end\u{1b}"), "end");
    assert_eq!(strip_ansi("end\u{1b}[31"), "end");
    assert_eq!(strip_ansi("end\u{1b}]title"), "end");
}

// OUT-008: the tray's font draws a tab as a box. `go test` writes these exact bytes (measured
// 2026-10-01), so each tab becomes the spaces that reach the next tab stop.
#[test]
fn tabs_are_expanded_to_tab_stops() {
    assert_eq!(
        decode(b"ok  \texample.com/x\t0.027s"),
        "ok      example.com/x   0.027s"
    );
    assert_eq!(decode(b"\tindented"), "        indented");
    assert_eq!(decode(b"12345678\tnext"), "12345678        next");
}

// OUT-008: any other control character would also be drawn as a box, so it is removed.
#[test]
fn other_control_characters_are_removed() {
    assert_eq!(
        decode(b"bell\x07 back\x08space \x00nul"),
        "bell backspace nul"
    );
}

#[test]
fn escapes_are_removed_after_decoding() {
    assert_eq!(decode(b"\x1b[32mok\x1b[0m\r"), "ok");
}
