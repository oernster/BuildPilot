//! The update check's decisions (UI-010): which release is newer, what to download and what to
//! say. The request itself is a `ReleaseSource`; when it is made is the window's business.

use crate::domain::version::Version;

use super::ports::{Release, ReleaseSource};

/// The file type a Windows release offers to download.
const WINDOWS_ASSET_SUFFIX: &str = ".exe";

/// What a check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateOutcome {
    /// A newer release: its version and what Download opens.
    Available {
        /// The newer version.
        latest: Version,
        /// The setup program's download, else the release's page.
        download_url: String,
    },
    /// Nothing newer; else only a release the operator skipped.
    UpToDate,
    /// The release source could not be reached or answered nonsense.
    Unreachable,
}

/// Who asked: the window on its own or the operator through Help.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asked {
    /// At launch or on the daily timer: honours the skip and speaks only of a newer release.
    Automatically,
    /// Help > Check for Updates: ignores the skip and reports every outcome.
    ByOperator,
}

/// The version a release tag names: `1.2.0` or `v1.2.0`; `None` for anything else, so a tag
/// that is not a version is never newer.
pub fn tag_version(tag: &str) -> Option<Version> {
    let tag = tag.trim();
    let bare = tag.strip_prefix(['v', 'V']).unwrap_or(tag);
    Version::parse(bare)
}

/// The release's setup program when it has one, else its page.
pub fn download_url(release: &Release) -> String {
    release
        .assets
        .iter()
        .find(|asset| asset.name.to_lowercase().ends_with(WINDOWS_ASSET_SUFFIX))
        .map_or_else(
            || release.page_url.clone(),
            |asset| asset.download_url.clone(),
        )
}

/// Asks `source` and decides: newer than `current`, not the `skipped` release when asked
/// automatically.
pub fn check(
    source: &dyn ReleaseSource,
    current: Version,
    skipped: Option<Version>,
    asked: Asked,
) -> UpdateOutcome {
    let Some(release) = source.latest_release() else {
        return UpdateOutcome::Unreachable;
    };
    match tag_version(&release.tag) {
        Some(latest)
            if latest > current && (asked == Asked::ByOperator || skipped != Some(latest)) =>
        {
            UpdateOutcome::Available {
                latest,
                download_url: download_url(&release),
            }
        }
        _ => UpdateOutcome::UpToDate,
    }
}

/// What the window says about `outcome`; `None` when an automatic check should say nothing.
pub fn message(outcome: &UpdateOutcome, current: Version, asked: Asked) -> Option<String> {
    match (outcome, asked) {
        (UpdateOutcome::Available { latest, .. }, _) => Some(format!(
            "BuildPilot {latest} is available. You are running {current}."
        )),
        (_, Asked::Automatically) => None,
        (UpdateOutcome::UpToDate, Asked::ByOperator) => {
            Some("You are running the latest version.".to_owned())
        }
        (UpdateOutcome::Unreachable, Asked::ByOperator) => {
            Some("The update check could not reach GitHub. Please try again later.".to_owned())
        }
    }
}
