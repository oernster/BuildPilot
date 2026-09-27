use buildpilot::domain::credits::{Credit, parse};

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

// UI-009: anything short of three fields is skipped, never guessed at.
#[test]
fn a_short_line_is_skipped() {
    assert_eq!(
        parse("half\t1.0\n\n\t1.0\tMIT\nfull\t2.0\tMIT"),
        vec![credit("full", "2.0", "MIT")]
    );
}
