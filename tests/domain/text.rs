use buildpilot::domain::text::{decode_line, strip_ansi};

// OUT-008
#[test]
fn plain_text_passes_through() {
    assert_eq!(decode_line(b"Compiling buildpilot"), "Compiling buildpilot");
    assert_eq!(decode_line("caf\u{e9}".as_bytes()), "caf\u{e9}");
}

// OUT-008: invalid UTF-8 becomes U+FFFD.
#[test]
fn invalid_bytes_are_replaced() {
    assert_eq!(decode_line(b"bad \xff byte"), "bad \u{fffd} byte");
}

#[test]
fn trailing_carriage_return_is_removed() {
    assert_eq!(decode_line(b"windows line\r"), "windows line");
}

#[test]
fn rewritten_line_shows_its_final_value() {
    assert_eq!(decode_line(b"10%\r50%\r100%"), "100%");
    assert_eq!(decode_line(b"10%\r100%\r"), "100%");
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

#[test]
fn escapes_are_removed_after_decoding() {
    assert_eq!(decode_line(b"\x1b[32mok\x1b[0m\r"), "ok");
}
