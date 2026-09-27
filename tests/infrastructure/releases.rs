use buildpilot::application::{Release, ReleaseAsset};
use buildpilot::infrastructure::releases::{REPOSITORY, latest_release_path, parse_release};

// UI-010: the address asked is derived from Cargo.toml's repository, the one home of its name.
#[test]
fn the_path_comes_from_the_repository() {
    assert_eq!(
        latest_release_path(REPOSITORY).as_deref(),
        Some("/repos/oernster/BuildPilot/releases/latest")
    );
    assert_eq!(
        latest_release_path("https://github.com/owner/name/").as_deref(),
        Some("/repos/owner/name/releases/latest")
    );
    for not_github in [
        "",
        "https://gitlab.com/a/b",
        "https://github.com/a",
        "https://github.com/a/b/c",
    ] {
        assert_eq!(latest_release_path(not_github), None, "{not_github}");
    }
}

// UI-010: a well-formed answer reads whole.
#[test]
fn a_release_is_read() {
    let answer = br#"{"tag_name": "v1.2.0", "html_url": "https://page",
        "assets": [{"name": "BuildPilotSetup.exe", "browser_download_url": "https://setup"}],
        "draft": false}"#;
    assert_eq!(
        parse_release(answer),
        Some(Release {
            tag: "v1.2.0".to_owned(),
            page_url: "https://page".to_owned(),
            assets: vec![ReleaseAsset {
                name: "BuildPilotSetup.exe".to_owned(),
                download_url: "https://setup".to_owned(),
            }],
        })
    );
}

// UI-010: the answer is foreign input: every field is checked and anything malformed left out.
#[test]
fn a_malformed_answer_is_refused_or_trimmed() {
    for refused in [
        &b"not json"[..],
        b"[]",
        b"{}",
        br#"{"tag_name": "v1.2.0"}"#,
        br#"{"html_url": "https://page"}"#,
        br#"{"tag_name": 12, "html_url": "https://page"}"#,
        br#"{"tag_name": "", "html_url": "https://page"}"#,
    ] {
        assert_eq!(
            parse_release(refused),
            None,
            "{}",
            String::from_utf8_lossy(refused)
        );
    }
    let trimmed = br#"{"tag_name": "v1", "html_url": "p", "assets": [
        {"name": "a.exe"}, {"browser_download_url": "u"}, 7, {"name": 3, "browser_download_url": "u"},
        {"name": "b.exe", "browser_download_url": "https://b"}]}"#;
    let release = parse_release(trimmed).unwrap();
    assert_eq!(release.assets.len(), 1);
    assert_eq!(release.assets[0].name, "b.exe");
    let no_assets = parse_release(br#"{"tag_name": "v1", "html_url": "p", "assets": null}"#);
    assert_eq!(no_assets.unwrap().assets, []);
}
