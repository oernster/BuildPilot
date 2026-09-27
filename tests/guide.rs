//! UI-007: every toolbar and row control is named in the Guide, so a new control cannot ship
//! unexplained.
//!
//! A label built from a row's name ("Run " + name) passes when one of its words is a Guide
//! entry's name. A label that changes with state (the theme toggle's "Switch to light theme" or
//! "Switch to dark theme") is several controls in one place, so every alternative needs its own
//! entry.

use std::fs;
use std::path::Path;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");
/// The files holding the toolbar's and the rows' controls; then the Guide that explains them.
const CONTROL_FILES: [&str; 2] = ["ui/main.slint", "ui/row.slint"];
const GUIDE_FILE: &str = "ui/guide.slint";
/// What marks a label whose words depend on state.
const CONDITIONAL: &str = " ? ";

fn read(relative: &str) -> String {
    fs::read_to_string(Path::new(ROOT).join(relative)).expect("the file is readable")
}

/// The string literals on `line`, trimmed.
fn literals(line: &str) -> Vec<String> {
    line.split('"')
        .skip(1)
        .step_by(2)
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
        .collect()
}

#[test]
fn the_guide_names_every_control() {
    let names: Vec<String> = read(GUIDE_FILE)
        .lines()
        .filter_map(|line| line.split("name: \"").nth(1))
        .filter_map(|rest| rest.split('"').next())
        .map(str::to_owned)
        .collect();
    let named = |label: &String| names.contains(label);
    let mut missing = Vec::new();
    for file in CONTROL_FILES {
        let mut in_button = false;
        for line in read(file).lines() {
            if line.contains("IconButton {") {
                in_button = true;
            }
            if in_button && line.trim_start().starts_with("label:") {
                in_button = false;
                let labels = literals(line);
                let explained = if line.contains(CONDITIONAL) {
                    labels.iter().all(named)
                } else {
                    labels.iter().any(named)
                };
                if !explained {
                    missing.push(format!("{file}: {}", line.trim()));
                }
            }
        }
    }
    assert!(!names.is_empty(), "no Guide entries found in {GUIDE_FILE}");
    assert!(
        missing.is_empty(),
        "not in the Guide:\n{}",
        missing.join("\n")
    );
}
