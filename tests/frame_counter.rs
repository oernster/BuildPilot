//! Slint's frame counter never shows, whatever environment BuildPilot is started from.
//!
//! Slint draws it when `SLINT_DEBUG_PERFORMANCE` is set as a window is made, so each executable
//! removes the variable before anything else happens. This fails when a `main` stops doing so
//! first.

use std::fs;
use std::path::Path;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");
/// Every executable that opens a Slint window.
const COMPOSITION_ROOTS: &[&str] = &["src/main.rs", "src/bin/buildpilotsetup.rs"];
const MAIN: &str = "fn main() -> ExitCode {";
const FIRST_LINE: &str = "hide_frame_counter();";

#[test]
fn every_main_hides_the_frame_counter_first() {
    let mut wrong = Vec::new();
    for root in COMPOSITION_ROOTS {
        let source =
            fs::read_to_string(Path::new(ROOT).join(root)).expect("the source is readable");
        let first = source
            .split(MAIN)
            .nth(1)
            .and_then(|body| body.lines().map(str::trim).find(|line| !line.is_empty()));
        if first != Some(FIRST_LINE) {
            wrong.push(format!("{root}: main begins {first:?}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "main must call {FIRST_LINE} first:\n{}",
        wrong.join("\n")
    );
}
