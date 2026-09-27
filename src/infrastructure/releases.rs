//! The newest published release, from GitHub's API (UI-010). `releases/latest` answers only a
//! published release, never a draft or a pre-release, so a tag pushed during development can
//! never prompt anyone. Everything in the answer is foreign input: each field is checked for its
//! type before use and anything malformed is left out.

use serde_json::Value;

use crate::application::ports::{Release, ReleaseAsset, ReleaseSource};

/// The repository's address on GitHub, from Cargo.toml: the one home of owner and name.
pub const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
/// The start of a repository address on GitHub.
const GITHUB_PREFIX: &str = "https://github.com/";
/// GitHub's API server.
const API_HOST: &str = "api.github.com";
/// The media type GitHub's API asks callers to accept.
const ACCEPT: &str = "application/vnd.github+json";
/// How long each stage of the request may take, in milliseconds.
const TIMEOUT_MS: i32 = 5000;
/// The largest answer accepted; a release description is far smaller.
const MAX_ANSWER_BYTES: usize = 1024 * 1024;

/// The API path of `repository`'s newest release; `None` unless it is a GitHub address.
pub fn latest_release_path(repository: &str) -> Option<String> {
    let rest = repository
        .strip_prefix(GITHUB_PREFIX)?
        .trim_end_matches('/');
    let mut parts = rest.split('/');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(owner), Some(name), None) if !owner.is_empty() && !name.is_empty() => {
            Some(format!("/repos/{owner}/{name}/releases/latest"))
        }
        _ => None,
    }
}

fn string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)?
        .as_str()
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

/// The release in a `releases/latest` answer; `None` unless it names a tag and a page.
pub fn parse_release(answer: &[u8]) -> Option<Release> {
    let document: Value = serde_json::from_slice(answer).ok()?;
    let assets = document
        .get("assets")
        .and_then(Value::as_array)
        .map(|assets| {
            assets
                .iter()
                .filter_map(|asset| {
                    Some(ReleaseAsset {
                        name: string(asset, "name")?,
                        download_url: string(asset, "browser_download_url")?,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Some(Release {
        tag: string(&document, "tag_name")?,
        page_url: string(&document, "html_url")?,
        assets,
    })
}

/// Asks GitHub, through WinHTTP, for BuildPilot's newest release.
pub struct GitHubReleases {
    /// What the request calls itself: GitHub refuses a request without a user agent.
    user_agent: String,
}

impl GitHubReleases {
    /// A source that identifies itself as `product` at `version`.
    pub fn new(product: &str, version: &str) -> Self {
        Self {
            user_agent: format!("{product}/{version}"),
        }
    }
}

impl ReleaseSource for GitHubReleases {
    fn latest_release(&self) -> Option<Release> {
        let path = latest_release_path(REPOSITORY)?;
        let headers = format!("Accept: {ACCEPT}\r\n");
        let answer = super::win32::http::get(&super::win32::http::Get {
            host: API_HOST,
            path: &path,
            headers: &headers,
            user_agent: &self.user_agent,
            timeout_ms: TIMEOUT_MS,
            max_body: MAX_ANSWER_BYTES,
        })
        .ok()?;
        parse_release(&answer)
    }
}
