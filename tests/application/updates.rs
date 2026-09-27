use std::path::PathBuf;

use buildpilot::application::updates::{
    Asked, UpdateOutcome, check, download_url, message, tag_version,
};
use buildpilot::application::{Release, ReleaseAsset, ReleaseSource};
use buildpilot::domain::version::Version;

use super::support::world;

const PAGE: &str = "https://github.com/oernster/BuildPilot/releases/tag/v1.2.0";
const SETUP: &str =
    "https://github.com/oernster/BuildPilot/releases/download/v1.2.0/BuildPilotSetup.exe";

/// A release source that answers one release; else nothing.
struct Fixed(Option<Release>);

impl ReleaseSource for Fixed {
    fn latest_release(&self) -> Option<Release> {
        self.0.clone()
    }
}

fn version(text: &str) -> Version {
    Version::parse(text).unwrap()
}

fn release(tag: &str, assets: &[(&str, &str)]) -> Release {
    Release {
        tag: tag.to_owned(),
        page_url: PAGE.to_owned(),
        assets: assets
            .iter()
            .map(|(name, url)| ReleaseAsset {
                name: (*name).to_owned(),
                download_url: (*url).to_owned(),
            })
            .collect(),
    }
}

fn offered(latest: &str, url: &str) -> UpdateOutcome {
    UpdateOutcome::Available {
        latest: version(latest),
        download_url: url.to_owned(),
    }
}

// UI-010: a tag is a version with or without a leading v; anything else is never newer.
#[test]
fn a_tag_reads_as_a_version() {
    assert_eq!(tag_version("v1.2.0"), Some(version("1.2.0")));
    assert_eq!(tag_version("V1.2.0"), Some(version("1.2.0")));
    assert_eq!(tag_version(" 1.2.0 "), Some(version("1.2.0")));
    for not_a_version in ["", "v", "latest", "v1.2", "1.2.0-beta", "v1.2.0.1"] {
        assert_eq!(tag_version(not_a_version), None, "{not_a_version}");
    }
}

// UI-010: Download opens the setup program when the release has one, else its page.
#[test]
fn download_prefers_the_setup_program() {
    let with = release(
        "v1.2.0",
        &[("notes.txt", "n"), ("BuildPilotSetup.EXE", SETUP)],
    );
    assert_eq!(download_url(&with), SETUP);
    assert_eq!(
        download_url(&release("v1.2.0", &[("notes.txt", "n")])),
        PAGE
    );
}

// UI-010: newer is offered; same, older, unparseable and unreachable are not.
#[test]
fn only_a_newer_release_is_offered() {
    let current = version("1.1.0");
    let ask = |tag: &str| {
        check(
            &Fixed(Some(release(tag, &[]))),
            current,
            None,
            Asked::Automatically,
        )
    };
    assert_eq!(ask("v1.2.0"), offered("1.2.0", PAGE));
    assert_eq!(ask("v1.1.0"), UpdateOutcome::UpToDate);
    assert_eq!(ask("v1.0.9"), UpdateOutcome::UpToDate);
    assert_eq!(ask("nightly"), UpdateOutcome::UpToDate);
    assert_eq!(
        check(&Fixed(None), current, None, Asked::Automatically),
        UpdateOutcome::Unreachable
    );
}

// UI-010: a skipped release stays quiet unbidden; the operator's own check still offers it.
// A release newer than the skipped one is offered either way.
#[test]
fn the_skip_silences_only_the_automatic_check() {
    let source = Fixed(Some(release("v1.2.0", &[])));
    let (current, skipped) = (version("1.1.0"), Some(version("1.2.0")));
    assert_eq!(
        check(&source, current, skipped, Asked::Automatically),
        UpdateOutcome::UpToDate
    );
    assert_eq!(
        check(&source, current, skipped, Asked::ByOperator),
        offered("1.2.0", PAGE)
    );
    let newer = Fixed(Some(release("v1.3.0", &[])));
    assert_eq!(
        check(&newer, current, skipped, Asked::Automatically),
        offered("1.3.0", PAGE)
    );
}

// UI-010: an automatic check speaks only of a newer release; the operator's reports everything.
#[test]
fn what_each_outcome_says() {
    let current = version("1.1.0");
    let available = offered("1.2.0", PAGE);
    let said = "BuildPilot 1.2.0 is available. You are running 1.1.0.";
    for asked in [Asked::Automatically, Asked::ByOperator] {
        assert_eq!(message(&available, current, asked).as_deref(), Some(said));
    }
    assert_eq!(
        message(&UpdateOutcome::UpToDate, current, Asked::Automatically),
        None
    );
    assert_eq!(
        message(&UpdateOutcome::Unreachable, current, Asked::Automatically),
        None
    );
    assert_eq!(
        message(&UpdateOutcome::UpToDate, current, Asked::ByOperator).as_deref(),
        Some("You are running the latest version.")
    );
    assert_eq!(
        message(&UpdateOutcome::Unreachable, current, Asked::ByOperator).as_deref(),
        Some("The update check could not reach GitHub. Please try again later.")
    );
}

// UI-010: Skip This Version is saved.
#[test]
fn a_skip_is_saved() {
    let world = world();
    let mut app = world.app();
    app.skip_update(version("1.2.0"));
    let saved = world.state.borrow().saves.last().unwrap().1;
    assert_eq!(saved.skipped_update, Some(version("1.2.0")));
    assert_eq!(app.preferences().skipped_update, Some(version("1.2.0")));
}

// UI-010: Download goes through the shell.
#[test]
fn download_opens_through_the_shell() {
    let world = world();
    let app = world.app();
    app.open_address(SETUP).unwrap();
    assert_eq!(
        world.state.borrow().shell_calls,
        [("open_address", PathBuf::from(SETUP))]
    );
}
