//! The open source credits (UI-009): one line per crate built into BuildPilot, as the build
//! script writes them from `cargo metadata`. Nothing here is written by hand.

/// Between one crate's fields on a line.
pub const FIELD_SEPARATOR: char = '\t';

/// A crate built into BuildPilot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credit {
    /// The crate's name.
    pub name: String,
    /// Its version.
    pub version: String,
    /// Its licence expression, as the crate states it; empty when it states none.
    pub licence: String,
}

/// Each SPDX licence identifier a crate built into BuildPilot states, with the name a reader
/// knows it by. `tests/infrastructure/build_info.rs` fails when a shipped crate states one that
/// is not here, so no identifier reaches About unexplained.
pub const LICENCE_NAMES: &[(&str, &str)] = &[
    ("0BSD", "BSD Zero Clause"),
    ("Apache-2.0", "Apache 2.0"),
    ("BSD-2-Clause", "BSD 2-Clause"),
    ("BSD-3-Clause", "BSD 3-Clause"),
    ("BSL-1.0", "Boost 1.0"),
    ("CC0-1.0", "CC0 1.0"),
    ("GPL-3.0-only", "GPL 3.0"),
    ("ISC", "ISC"),
    (
        "LicenseRef-Slint-Royalty-free-2.0",
        "Slint Royalty-free 2.0",
    ),
    ("LicenseRef-Slint-Software-3.0", "Slint Software 3.0"),
    ("MIT", "MIT"),
    ("Unicode-3.0", "Unicode License v3"),
    ("Unlicense", "Unlicense"),
    ("Zlib", "zlib"),
];

/// The readable name of licence `identifier`; `None` when the table does not know it.
pub fn licence_name(identifier: &str) -> Option<&'static str> {
    LICENCE_NAMES
        .iter()
        .find(|(id, _)| *id == identifier)
        .map(|(_, name)| *name)
}

/// The identifiers in a licence `expression`, operators and brackets left out.
pub fn licence_identifiers(expression: &str) -> Vec<&str> {
    expression
        .split(|c: char| c.is_whitespace() || matches!(c, '(' | ')' | '/'))
        .filter(|word| !word.is_empty() && !matches!(*word, "OR" | "AND" | "WITH"))
        .collect()
}

/// A licence `expression` in words: each identifier by its name, `OR` and `AND` in lower case and
/// the old `MIT/Apache-2.0` form as "or". An identifier the table does not know stays as written.
pub fn readable_licence(expression: &str) -> String {
    let mut out = String::new();
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        if word.is_empty() {
            return;
        }
        let text = match word.as_str() {
            "OR" => "or",
            "AND" => "and",
            "WITH" => "with",
            other => licence_name(other).unwrap_or(other),
        };
        out.push_str(text);
        word.clear();
    };
    for c in expression.chars() {
        match c {
            '/' => {
                flush(&mut word, &mut out);
                out.push_str(" or ");
            }
            '(' | ')' => {
                flush(&mut word, &mut out);
                out.push(c);
            }
            c if c.is_whitespace() => {
                flush(&mut word, &mut out);
                if !out.ends_with(' ') {
                    out.push(' ');
                }
            }
            c => word.push(c),
        }
    }
    flush(&mut word, &mut out);
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The credits in `text`, one per line of name, version and licence. A line without all three
/// fields is skipped rather than guessed at.
pub fn parse(text: &str) -> Vec<Credit> {
    text.lines()
        .filter_map(|line| {
            let mut fields = line.splitn(3, FIELD_SEPARATOR);
            match (fields.next(), fields.next(), fields.next()) {
                (Some(name), Some(version), Some(licence)) if !name.is_empty() => Some(Credit {
                    name: name.to_owned(),
                    version: version.to_owned(),
                    licence: licence.to_owned(),
                }),
                _ => None,
            }
        })
        .collect()
}
