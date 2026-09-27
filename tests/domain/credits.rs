use buildpilot::domain::credits::{Credit, licence_identifiers, parse, readable_licence};

fn credit(name: &str, version: &str, licence: &str) -> Credit {
    Credit {
        name: name.to_owned(),
        version: version.to_owned(),
        licence: licence.to_owned(),
    }
}

// UI-009
#[test]
fn each_line_is_one_crate() {
    let text = "serde\t1.0.228\tMIT OR Apache-2.0\nslint\t1.18.1\tGPL-3.0-only\n";
    assert_eq!(
        parse(text),
        vec![
            credit("serde", "1.0.228", "MIT OR Apache-2.0"),
            credit("slint", "1.18.1", "GPL-3.0-only"),
        ]
    );
}

// UI-009: a crate that states no licence keeps its place with an empty licence.
#[test]
fn an_empty_licence_is_kept() {
    assert_eq!(parse("quiet\t0.1.0\t"), vec![credit("quiet", "0.1.0", "")]);
}

// UI-009: a licence expression reads in words.
#[test]
fn a_licence_expression_reads_in_words() {
    let cases = [
        ("Unicode-3.0", "Unicode License v3"),
        ("MIT OR Apache-2.0", "MIT or Apache 2.0"),
        ("MIT/Apache-2.0", "MIT or Apache 2.0"),
        ("Apache-2.0 / MIT", "Apache 2.0 or MIT"),
        (
            "(MIT OR Apache-2.0) AND Unicode-3.0",
            "(MIT or Apache 2.0) and Unicode License v3",
        ),
        (
            "GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0 OR LicenseRef-Slint-Software-3.0",
            "GPL 3.0 or Slint Royalty-free 2.0 or Slint Software 3.0",
        ),
        (
            "Apache-2.0 WITH LLVM-exception",
            "Apache 2.0 with LLVM-exception",
        ),
        ("", ""),
    ];
    for (expression, words) in cases {
        assert_eq!(readable_licence(expression), words, "{expression}");
    }
}

// UI-009: the identifiers of an expression, without its operators and brackets.
#[test]
fn the_identifiers_of_an_expression() {
    assert_eq!(
        licence_identifiers("(MIT OR Apache-2.0) AND Unicode-3.0 / Zlib"),
        ["MIT", "Apache-2.0", "Unicode-3.0", "Zlib"]
    );
}

// UI-009: anything short of three fields is skipped, never guessed at.
#[test]
fn a_short_line_is_skipped() {
    assert_eq!(
        parse("half\t1.0\n\n\t1.0\tMIT\nfull\t2.0\tMIT"),
        vec![credit("full", "2.0", "MIT")]
    );
}
