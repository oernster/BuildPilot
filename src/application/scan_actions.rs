//! Adding from a folder: what a scan of it proposes (SRS 3.20). Nothing is added here; the
//! operator confirms through `App::add`.

use std::path::Path;

use crate::domain::environment::Offer;
use crate::domain::operation::{IconRef, OperationSpec};
use crate::domain::scan::{looked_for, propose};
use crate::domain::step::StepSpec;

use super::App;

/// One folder a scan recognised, ready for the operation dialog or the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedFolder {
    /// The operation it would add (SCAN-004).
    pub spec: OperationSpec,
    /// The partial-match words, when the pattern was not whole (SCAN-009).
    pub warning: Option<String>,
    /// Why the list starts it unticked; `None` when it starts ticked (SCAN-006).
    pub unticked_because: Option<String>,
    /// An operation on the deck already runs it, so the list never lets it be ticked (SCAN-006).
    pub already_added: bool,
}

/// What a scan of a chosen folder found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolderScan {
    /// The folder itself is a project (SCAN-004).
    One(ScannedFolder),
    /// Folders directly inside it are projects, in name order (SCAN-005).
    Several(Vec<ScannedFolder>),
    /// Nothing was recognised; these are the file names looked for (SCAN-007).
    Nothing(Vec<&'static str>),
}

/// SCAN-006's two reasons.
const ALREADY_ADDED: &str = "Already on the flight deck";
const CHOOSE_ALONE: &str = "Several environments: add this one on its own to choose";

impl App {
    /// Scans `folder`, then the folders directly inside it; never deeper (SCAN-008).
    pub fn scan_folder(&self, folder: &Path) -> FolderScan {
        if let Some(found) = self.scan_one(folder) {
            return FolderScan::One(found);
        }
        let inside: Vec<ScannedFolder> = self
            .ports
            .paths
            .subfolders(folder)
            .iter()
            .filter_map(|name| self.scan_one(&folder.join(name)))
            .collect();
        if inside.is_empty() {
            FolderScan::Nothing(looked_for())
        } else {
            FolderScan::Several(inside)
        }
    }

    /// The proposal for `folder` alone, filled as the dialog would fill it (SCAN-004).
    fn scan_one(&self, folder: &Path) -> Option<ScannedFolder> {
        let proposal = propose(&self.ports.paths.files(folder))?;
        let steps: Vec<StepSpec> = proposal
            .steps
            .iter()
            .map(|file| StepSpec::for_script(&folder.join(file)))
            .collect();
        let (environment, unchosen) = match self.environment_offer(folder, &steps) {
            Offer::Choose {
                preselected: Some(name),
                ..
            } => (Some(name), false),
            Offer::Choose {
                preselected: None, ..
            } => (None, true),
            Offer::Note(_) | Offer::Nothing => (None, false),
        };
        let spec = OperationSpec {
            name: folder
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            steps,
            working_dir: folder.to_path_buf(),
            environment,
            icon: self
                .ports
                .icons
                .discover(folder)
                .map_or(IconRef::Placeholder, IconRef::Discovered),
        };
        let already_added = self.already_added(&spec);
        let unticked_because = if already_added {
            Some(ALREADY_ADDED.to_owned())
        } else if unchosen {
            Some(CHOOSE_ALONE.to_owned())
        } else {
            None
        };
        Some(ScannedFolder {
            spec,
            warning: proposal.warning(),
            unticked_because,
            already_added,
        })
    }

    /// True when an operation on the deck runs the same steps in the same folder (SCAN-006).
    fn already_added(&self, spec: &OperationSpec) -> bool {
        self.deck.operations().iter().any(|operation| {
            let existing = operation.config().to_spec();
            existing.steps == spec.steps && existing.working_dir == spec.working_dir
        })
    }
}
