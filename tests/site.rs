//! The GitHub Pages site under `docs/` names the version in `VERSION` and no other (CON-005).
//!
//! The site cannot read `VERSION` when it is shown, so each mention is a token that
//! `stamp_version.ps1` rewrites. This fails when a token was left behind or the stamp never ran.

use std::fs;
use std::path::{Path, PathBuf};

const ROOT: &str = env!("CARGO_MANIFEST_DIR");
/// Where the site lives.
const SITE_DIR: &str = "docs";
/// The page that must name the version at least once.
const HOME_PAGE: &str = "index.html";
const OPEN: &str = "<!--VERSION-->";
const CLOSE: &str = "<!--/VERSION-->";

fn pages(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).expect("the site folder is readable") {
        let path = entry.expect("a site entry is readable").path();
        if path.is_dir() {
            found.extend(pages(&path));
        } else if path
            .extension()
            .is_some_and(|ext| ext == "html" || ext == "md")
        {
            found.push(path);
        }
    }
    found
}

/// Every version a page's tokens hold, in order.
fn stamped(page: &str) -> Vec<String> {
    page.split(OPEN)
        .skip(1)
        .map(|rest| rest.split(CLOSE).next().unwrap_or(rest).to_owned())
        .collect()
}

#[test]
fn the_site_names_the_version_file() {
    let root = Path::new(ROOT);
    let version = fs::read_to_string(root.join("VERSION"))
        .expect("VERSION is readable")
        .trim()
        .to_owned();
    let site = root.join(SITE_DIR);
    let home = fs::read_to_string(site.join(HOME_PAGE)).expect("the home page is readable");
    assert!(
        !stamped(&home).is_empty(),
        "{SITE_DIR}/{HOME_PAGE} names no version"
    );

    let mut wrong = Vec::new();
    for page in pages(&site) {
        let text = fs::read_to_string(&page).expect("a site page is readable");
        for found in stamped(&text) {
            if found != version {
                wrong.push(format!("{}: {found}", page.display()));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "the site names a version other than {version}:\n{}",
        wrong.join("\n")
    );
}
