use buildpilot::domain::credits::parse;
use buildpilot::infrastructure::build_info::{CREDITS, NOTICES};

fn credited(name: &str) -> bool {
    parse(CREDITS).iter().any(|credit| credit.name == name)
}

// UI-009: generated from the real graph: what is built in is there, what only tests use is not.
#[test]
fn the_credits_are_the_shipped_graph() {
    for built_in in ["slint", "serde", "serde_json", "uuid", "windows-sys", "rfd"] {
        assert!(
            credited(built_in),
            "{built_in} is built in but not credited"
        );
    }
    // The testing backend is a dev-dependency whose features `cargo metadata` would merge in;
    // `syn` is only ever used by procedural macros while compiling.
    let not_shipped = [
        "tempfile",
        "slint-build",
        "winresource",
        "buildpilot",
        "i-slint-backend-testing",
        "syn",
    ];
    for not_shipped in not_shipped {
        assert!(
            !credited(not_shipped),
            "{not_shipped} does not ship but is credited"
        );
    }
}

// UI-009: every credit names a version and a licence.
#[test]
fn every_credit_names_its_version_and_licence() {
    let credits = parse(CREDITS);
    let bare: Vec<_> = credits
        .iter()
        .filter(|credit| credit.version.is_empty() || credit.licence.is_empty())
        .map(|credit| credit.name.as_str())
        .collect();
    assert!(
        bare.is_empty(),
        "credits with no version or licence: {bare:?}"
    );
}

// UI-008: the notices carry every credited crate by name.
#[test]
fn the_notices_name_every_credited_crate() {
    let missing: Vec<_> = parse(CREDITS)
        .into_iter()
        .filter(|credit| !NOTICES.contains(&format!("{} {} (", credit.name, credit.version)))
        .map(|credit| credit.name)
        .collect();
    assert!(missing.is_empty(), "not in the notices: {missing:?}");
}
