//! The GitHub Pages site under `docs/` names the version in `VERSION` and no other (CON-005).
//!
//! The site cannot read `VERSION` when it is shown, so each mention is a token that
//! `stamp_version.ps1` rewrites. This fails when a token was left behind or the stamp never ran.
//!
//! The same script puts each local stylesheet and script's content hash on the links to it, so a
//! browser holding the old file fetches the new one. This fails when a file changed and its links
//! still name the old hash.

use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const ROOT: &str = env!("CARGO_MANIFEST_DIR");
/// Where the site lives.
const SITE_DIR: &str = "docs";
/// The page that must name the version at least once.
const HOME_PAGE: &str = "index.html";
const OPEN: &str = "<!--VERSION-->";
const CLOSE: &str = "<!--/VERSION-->";
/// How many hex characters of the SHA-256 a link carries.
const ASSET_HASH_LENGTH: usize = 10;
/// The attributes that link a stylesheet or a script.
const LINK_ATTRIBUTES: [&str; 2] = ["href=", "src="];
/// The files a link is versioned for.
const ASSET_EXTENSIONS: [&str; 2] = [".css", ".js"];

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

/// SHA-256 as lowercase hex.
fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Every local stylesheet or script link in a page, as (path, query). Remote, protocol-relative
/// and root-absolute links are not the site's files and are left out.
fn asset_links(page: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for attribute in LINK_ATTRIBUTES {
        for (at, _) in page.match_indices(attribute) {
            let rest = &page[at + attribute.len()..];
            let Some(quote) = rest.chars().next().filter(|c| *c == '"' || *c == '\'') else {
                continue;
            };
            let value = rest[1..].split(quote).next().unwrap_or_default();
            let value = value.split('#').next().unwrap_or_default();
            let (path, query) = value.split_once('?').unwrap_or((value, ""));
            let is_asset = ASSET_EXTENSIONS.iter().any(|ext| path.ends_with(ext));
            if is_asset && !path.starts_with('/') && !path.contains(':') {
                found.push((path.to_owned(), format!("?{query}")));
            }
        }
    }
    found
}

#[test]
fn the_site_links_each_asset_by_its_content() {
    let site = Path::new(ROOT).join(SITE_DIR);
    let mut wrong = Vec::new();
    let mut checked = 0;
    for page in pages(&site) {
        if page.extension().is_none_or(|ext| ext != "html") {
            continue;
        }
        let text = fs::read_to_string(&page).expect("a site page is readable");
        let folder = page.parent().expect("a page sits in a folder");
        for (path, query) in asset_links(&text) {
            let Ok(content) = fs::read(folder.join(&path)) else {
                wrong.push(format!(
                    "{}: links {path}, which cannot be read",
                    page.display()
                ));
                continue;
            };
            // CRLF read as LF, as the stamp does, so a Windows checkout agrees with GitHub's blob.
            let content: Vec<u8> = content
                .iter()
                .enumerate()
                .filter(|(i, byte)| !(**byte == b'\r' && content.get(i + 1) == Some(&b'\n')))
                .map(|(_, byte)| *byte)
                .collect();
            let want = format!("?v={}", &sha256_hex(&content)[..ASSET_HASH_LENGTH]);
            if query != want {
                wrong.push(format!(
                    "{}: {path}{query}, content says {path}{want}",
                    page.display()
                ));
            }
            checked += 1;
        }
    }
    assert!(
        checked > 0,
        "no local stylesheet or script links found in {SITE_DIR}"
    );
    assert!(
        wrong.is_empty(),
        "asset links name a stale hash; run ./stamp_version.ps1:\n{}",
        wrong.join("\n")
    );
}
