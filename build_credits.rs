//! The open source credits (UI-009) and the third-party notices (UI-008), generated from
//! `cargo metadata` so no credit is ever written by hand. Part of the build script.
//!
//! The crates credited are those a release build compiles into the program for the Windows target.
//! Dev-only crates, build-only crates (`slint-build` and what it pulls in) and procedural macros
//! are left out, as nothing of them ships.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use super::credits_format::FIELD_SEPARATOR;

/// The target whose dependency graph ships.
const TARGET: &str = "x86_64-pc-windows-msvc";
/// The start of a file name that holds a crate's licence or notice.
const LICENCE_FILES: [&str; 4] = ["LICENSE", "LICENCE", "COPYING", "NOTICE"];
/// The credits file, one crate per line: name, version, licence.
const CREDITS_FILE: &str = "credits.tsv";
/// The notices file, as the program and setup name it; handed to the program by name.
const NOTICES_FILE: &str = "THIRD-PARTY-NOTICES.txt";
/// Between two blocks of the notices file.
const RULE: &str =
    "--------------------------------------------------------------------------------";

struct Crate {
    name: String,
    version: String,
    licence: String,
    folder: PathBuf,
}

fn cargo(arguments: &[&str]) -> Vec<u8> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = Command::new(cargo)
        .args(arguments)
        .args(["--locked", "--offline"])
        .output()
        .expect("cargo runs");
    assert!(
        output.status.success(),
        "cargo {} failed: {}",
        arguments.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn text(value: &Value, key: &str) -> String {
    value[key].as_str().unwrap_or_default().to_owned()
}

/// Name and version of every crate a release build compiles into the program. `cargo tree`
/// resolves features the way that build does; `cargo metadata` would merge in the features
/// dev-dependencies ask for and so name crates that never ship. Procedural macros run while
/// compiling and ship nothing, so they are left out too.
fn shipped_set() -> BTreeSet<(String, String)> {
    let tree = cargo(&[
        "tree",
        "--edges",
        "normal,no-proc-macro",
        "--target",
        TARGET,
        "--prefix",
        "none",
        "--format",
        "{p}",
    ]);
    let root = env::var("CARGO_PKG_NAME").unwrap_or_default();
    String::from_utf8_lossy(&tree)
        .lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let name = words.next()?;
            let version = words.next()?.strip_prefix('v')?;
            (name != root).then(|| (name.to_owned(), version.to_owned()))
        })
        .collect()
}

/// The shipped crates with their licence and source folder, sorted by name then version.
fn shipped() -> Vec<Crate> {
    let wanted = shipped_set();
    let metadata: Value = serde_json::from_slice(&cargo(&[
        "metadata",
        "--format-version",
        "1",
        "--filter-platform",
        TARGET,
    ]))
    .expect("cargo metadata answers JSON");
    let mut crates: Vec<Crate> = metadata["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .filter(|package| wanted.contains(&(text(package, "name"), text(package, "version"))))
        .map(|package| {
            let manifest = PathBuf::from(text(package, "manifest_path"));
            Crate {
                name: text(package, "name"),
                version: text(package, "version"),
                licence: text(package, "license"),
                folder: manifest.parent().map(Path::to_path_buf).unwrap_or_default(),
            }
        })
        .collect();
    assert_eq!(
        crates.len(),
        wanted.len(),
        "every shipped crate is in the metadata"
    );
    crates.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    crates
}

fn licence_texts(folder: &Path) -> Vec<String> {
    let mut names: Vec<PathBuf> = fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().to_uppercase())
                .unwrap_or_default();
            LICENCE_FILES.iter().any(|start| name.starts_with(start))
        })
        .collect();
    names.sort();
    names
        .iter()
        .filter_map(|path| fs::read(path).ok())
        .map(|bytes| String::from_utf8_lossy(&bytes).replace("\r\n", "\n"))
        .collect()
}

fn notices(crates: &[Crate]) -> String {
    // Identical texts are written once, headed by every crate that ships them.
    let mut by_text: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut without = Vec::new();
    for item in crates {
        let label = format!("{} {} ({})", item.name, item.version, item.licence);
        let texts = licence_texts(&item.folder);
        if texts.is_empty() {
            without.push(label);
            continue;
        }
        for body in texts {
            by_text.entry(body).or_default().push(label.clone());
        }
    }
    let mut blocks: Vec<(Vec<String>, String)> = by_text
        .into_iter()
        .map(|(body, labels)| (labels, body))
        .collect();
    blocks.sort();
    let mut out = format!(
        "Third-party notices for BuildPilot\n\nBuildPilot is built from {} crates. Their \
         licence texts follow, each written once beneath every crate that ships it.\n",
        crates.len()
    );
    for (labels, body) in blocks {
        out.push_str(&format!(
            "\n{RULE}\n{}\n{RULE}\n\n{}\n",
            labels.join("\n"),
            body.trim_end()
        ));
    }
    if !without.is_empty() {
        out.push_str(&format!(
            "\n{RULE}\nThese crates ship no licence file; each is used under the licence \
             named beside it.\n{RULE}\n\n{}\n",
            without.join("\n")
        ));
    }
    out
}

/// Writes the credits and the notices into `out_dir`.
pub fn generate(out_dir: &Path) {
    println!("cargo:rerun-if-changed=Cargo.lock");
    let crates = shipped();
    let credits: String = crates
        .iter()
        .map(|item| {
            format!(
                "{}{FIELD_SEPARATOR}{}{FIELD_SEPARATOR}{}\n",
                item.name, item.version, item.licence
            )
        })
        .collect();
    let (credits_path, notices_path) = (out_dir.join(CREDITS_FILE), out_dir.join(NOTICES_FILE));
    fs::write(&credits_path, credits).expect("the credits are written");
    fs::write(&notices_path, notices(&crates)).expect("the notices are written");
    println!(
        "cargo:rustc-env=BUILDPILOT_CREDITS={}",
        credits_path.display()
    );
    println!(
        "cargo:rustc-env=BUILDPILOT_NOTICES={}",
        notices_path.display()
    );
    println!("cargo:rustc-env=BUILDPILOT_NOTICES_FILE={NOTICES_FILE}");
}
