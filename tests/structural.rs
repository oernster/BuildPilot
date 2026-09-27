//! Structural rules checked by reading the source, not by trusting convention.
//!
//! Each rule was proved to bite by planting a violation and watching it fail.

use std::fs;
use std::path::{Path, PathBuf};

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// A module may not exceed this many lines (NFR-MAINT-003).
const MAX_LINES: usize = 400;
/// A file within this share of the cap must be cut well below it, not trimmed to fit.
const DANGER_BAND_PERCENT: usize = 5;
const DANGER_BAND_START: usize = MAX_LINES - MAX_LINES * DANGER_BAND_PERCENT / 100;

/// The layers that do no I/O, each with the only parts of the standard library it may name.
/// Neither may touch the file system, processes, threads, the environment or the network.
/// The domain may not hold an instant at all (`Duration` is a length of time, not a reading of
/// one); the application may hold one but only ever gets it from its `Clock` port.
const PURE_LAYERS: &[(&str, &[&str])] = &[
    (
        "domain",
        &[
            "std::collections",
            "std::error",
            "std::fmt",
            "std::ops",
            "std::path",
            "std::str",
            "std::time::Duration",
        ],
    ),
    (
        "application",
        &[
            "std::collections",
            "std::error",
            "std::fmt",
            "std::iter",
            "std::mem",
            "std::path",
            "std::time::Duration",
            "std::time::Instant",
        ],
    ),
];

/// Ways of reading the clock directly; a pure layer must be handed the time instead.
const CLOCK_READS: &[&str] = &["Instant::now", "SystemTime"];

/// Which layers each layer may not name (CON-003): `ui -> application -> domain <- infrastructure`.
const FORBIDDEN_DEPENDENCIES: &[(&str, &[&str])] = &[
    ("domain", &["application", "infrastructure", "ui"]),
    ("application", &["infrastructure", "ui"]),
    // Infrastructure implements the application's port traits, so it may name the application.
    ("infrastructure", &["ui"]),
];

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    files_with_extension(dir, "rs")
}

fn files_with_extension(dir: &Path, wanted: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return files;
    };
    for entry in entries {
        let path = entry.expect("directory entry is readable").path();
        if path.is_dir() {
            files.extend(files_with_extension(&path, wanted));
        } else if path
            .extension()
            .is_some_and(|extension| extension == wanted)
        {
            files.push(path);
        }
    }
    files
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn relative(path: &Path) -> String {
    path.strip_prefix(ROOT)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Every `prefix` path mentioned in `source`, e.g. every `std::...` path.
fn paths_after<'a>(source: &'a str, prefix: &'a str) -> impl Iterator<Item = &'a str> {
    source.match_indices(prefix).map(move |(start, _)| {
        let rest = &source[start..];
        let end = rest
            .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':' || c == '{'))
            .unwrap_or(rest.len());
        &rest[..end]
    })
}

#[test]
fn pure_layers_use_only_allowed_std() {
    let mut violations = Vec::new();
    for (layer, allowed_paths) in PURE_LAYERS {
        for file in rust_files(&Path::new(ROOT).join("src").join(layer)) {
            let source = read(&file);
            for path in paths_after(&source, "std::") {
                let allowed = allowed_paths
                    .iter()
                    .any(|allowed| path == *allowed || path.starts_with(&format!("{allowed}::")));
                if !allowed {
                    violations.push(format!("{}: {path}", relative(&file)));
                }
            }
            for read_of_clock in CLOCK_READS {
                if source.contains(read_of_clock) {
                    violations.push(format!("{}: {read_of_clock}", relative(&file)));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "pure layers name forbidden std paths:\n{}",
        violations.join("\n")
    );
}

/// The modules a layer declares in its `mod.rs`, which its files may `use` by bare name.
fn declared_modules(layer_dir: &Path) -> Vec<String> {
    read(&layer_dir.join("mod.rs"))
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let rest = line
                .strip_prefix("pub mod ")
                .or_else(|| line.strip_prefix("mod "))?;
            Some(format!("{}::", rest.trim_end_matches(';')))
        })
        .collect()
}

#[test]
fn pure_layers_use_no_external_crates() {
    let mut violations = Vec::new();
    for (layer, _) in PURE_LAYERS {
        let layer_dir = Path::new(ROOT).join("src").join(layer);
        let mut own_roots: Vec<String> = ["std::", "crate::", "super::", "self::"]
            .map(str::to_owned)
            .to_vec();
        own_roots.extend(declared_modules(&layer_dir));
        for file in rust_files(&layer_dir) {
            for line in read(&file).lines() {
                let line = line.trim_start();
                let Some(used) = line
                    .strip_prefix("use ")
                    .or_else(|| line.strip_prefix("pub use "))
                else {
                    continue;
                };
                if !own_roots.iter().any(|root| used.starts_with(root.as_str())) {
                    violations.push(format!("{}: {line}", relative(&file)));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "pure layers use external crates:\n{}",
        violations.join("\n")
    );
}

#[test]
fn layers_depend_inwards_only() {
    let mut violations = Vec::new();
    for (layer, forbidden) in FORBIDDEN_DEPENDENCIES {
        for file in rust_files(&Path::new(ROOT).join("src").join(layer)) {
            let source = read(&file);
            for target in *forbidden {
                if source.contains(&format!("crate::{target}")) {
                    violations.push(format!("{} names crate::{target}", relative(&file)));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "layering broken:\n{}",
        violations.join("\n")
    );
}

#[test]
fn no_module_exceeds_the_line_limit_or_sits_in_the_danger_band() {
    let mut violations = Vec::new();
    for dir in ["src", "tests"] {
        for file in rust_files(&Path::new(ROOT).join(dir)) {
            let lines = read(&file).lines().count();
            if lines > MAX_LINES {
                violations.push(format!(
                    "{}: {lines} lines, over {MAX_LINES}",
                    relative(&file)
                ));
            } else if lines > DANGER_BAND_START {
                violations.push(format!(
                    "{}: {lines} lines, inside the danger band above {DANGER_BAND_START}",
                    relative(&file)
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "module size:\n{}",
        violations.join("\n")
    );
}

/// The one folder whose files may contain `unsafe` code.
const UNSAFE_HOME: &str = "src/infrastructure/win32";
/// How `unsafe` code begins.
const UNSAFE_MARKERS: &[&str] = &["unsafe {", "unsafe fn", "unsafe impl", "unsafe extern"];

#[test]
fn unsafe_code_lives_only_in_win32() {
    let home = Path::new(ROOT).join(UNSAFE_HOME);
    let mut violations = Vec::new();
    for file in rust_files(&Path::new(ROOT).join("src")) {
        if file.starts_with(&home) {
            continue;
        }
        let source = read(&file);
        for marker in UNSAFE_MARKERS {
            if source.contains(marker) {
                violations.push(format!("{}: {marker}", relative(&file)));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "unsafe code outside {UNSAFE_HOME}:\n{}",
        violations.join("\n")
    );
}

// CON-005, OQ-12
#[test]
fn cargo_version_matches_the_version_file() {
    let version = read(&Path::new(ROOT).join("VERSION")).trim().to_owned();
    let manifest = read(&Path::new(ROOT).join("Cargo.toml"));
    let declared = manifest
        .lines()
        .find_map(|line| line.trim().strip_prefix("version = \""))
        .and_then(|rest| rest.strip_suffix('"'))
        .expect("Cargo.toml declares a package version");
    assert_eq!(declared, version, "Cargo.toml version differs from VERSION");
}

/// Where the Slint UI lives.
const UI_DIR: &str = "ui";
/// Standard widgets that draw focus in their own style rather than the house ring; the UI uses
/// the controls in ui/widgets.slint instead.
const OWN_FOCUS_WIDGETS: &[&str] = &["Button", "CheckBox"];

// A11Y-002: every control wears the house ring (green on hover or focus, red when disabled) and
// the accent is never a ring.
#[test]
fn every_control_follows_the_ring_model() {
    let mut violations = Vec::new();
    for path in files_with_extension(&Path::new(ROOT).join(UI_DIR), "slint") {
        let source = read(&path);
        for (number, line) in source.lines().enumerate() {
            let place = format!("{}:{}", relative(&path), number + 1);
            if line.contains("std-widgets.slint") {
                let imported = line
                    .split(['{', '}'])
                    .nth(1)
                    .unwrap_or_default()
                    .split(',')
                    .map(str::trim);
                for widget in imported.filter(|name| OWN_FOCUS_WIDGETS.contains(name)) {
                    violations.push(format!("{place}: imports the standard {widget}"));
                }
            }
            if line.contains("border-color") && line.contains("Theme.accent") {
                violations.push(format!("{place}: draws a border in the accent"));
            }
        }
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}
