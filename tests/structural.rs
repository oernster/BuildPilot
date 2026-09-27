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

/// The only parts of the standard library the domain may name. No file system, processes,
/// threads, environment, network or clock (`std::time::Duration` is a length of time, not a
/// reading of one).
const DOMAIN_STD_ALLOWED: &[&str] = &[
    "std::collections",
    "std::error",
    "std::fmt",
    "std::ops",
    "std::path",
    "std::time::Duration",
];

/// Which layers each layer may not name (CON-003): `ui -> application -> domain <- infrastructure`.
const FORBIDDEN_DEPENDENCIES: &[(&str, &[&str])] = &[
    ("domain", &["application", "infrastructure", "ui"]),
    ("application", &["infrastructure", "ui"]),
    ("infrastructure", &["application", "ui"]),
];

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return files;
    };
    for entry in entries {
        let path = entry.expect("directory entry is readable").path();
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
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
fn domain_uses_only_allowed_std() {
    let mut violations = Vec::new();
    for file in rust_files(&Path::new(ROOT).join("src/domain")) {
        let source = read(&file);
        for path in paths_after(&source, "std::") {
            let allowed = DOMAIN_STD_ALLOWED
                .iter()
                .any(|allowed| path == *allowed || path.starts_with(&format!("{allowed}::")));
            if !allowed {
                violations.push(format!("{}: {path}", relative(&file)));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "domain names forbidden std paths:\n{}",
        violations.join("\n")
    );
}

#[test]
fn domain_uses_no_external_crates() {
    let own_roots = ["std::", "crate::", "super::", "self::"];
    let mut violations = Vec::new();
    for file in rust_files(&Path::new(ROOT).join("src/domain")) {
        for line in read(&file).lines() {
            let line = line.trim_start();
            let Some(used) = line
                .strip_prefix("use ")
                .or_else(|| line.strip_prefix("pub use "))
            else {
                continue;
            };
            if !own_roots.iter().any(|root| used.starts_with(root)) {
                violations.push(format!("{}: {line}", relative(&file)));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "domain uses external crates:\n{}",
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
