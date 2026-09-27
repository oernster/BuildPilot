//! Recognising a folder's build from the files in it (SRS 3.20). A scan proposes; nothing is
//! added until the operator confirms.

/// One recognised kind of build: the file names that become steps, in run order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pattern {
    /// What the build is, for the reader of this table.
    pub name: &'static str,
    /// The files, in the order they run.
    pub files: &'static [&'static str],
}

/// The patterns in order of preference (SCAN-001). A further one is one more entry here.
pub const PATTERNS: &[Pattern] = &[
    Pattern {
        name: "Go, Rust and other PowerShell builds",
        files: &["build.ps1"],
    },
    Pattern {
        name: "Python builds",
        files: &["buildexe.py", "buildinstaller.py"],
    },
];

/// What a scan proposes for one folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    /// The files present, as the folder names them, in run order.
    pub steps: Vec<String>,
    /// The pattern's files the folder lacks (SCAN-009).
    pub missing: Vec<String>,
}

impl Proposal {
    /// The words that flag a partial match (SCAN-009); `None` when nothing is missing.
    pub fn warning(&self) -> Option<String> {
        if self.missing.is_empty() {
            return None;
        }
        let verb = if self.missing.len() == 1 {
            "was"
        } else {
            "were"
        };
        Some(format!(
            "{} {verb} not found: only {} will run",
            self.missing.join(", "),
            self.steps.join(" then ")
        ))
    }
}

/// The proposal for a folder holding the files `present` (SCAN-003): the pattern with the most
/// of its files there, the earlier on a tie; `None` when no pattern has any. Names compare
/// without regard to case.
pub fn propose(present: &[String]) -> Option<Proposal> {
    let found = |wanted: &str| {
        present
            .iter()
            .find(|name| name.eq_ignore_ascii_case(wanted))
            .cloned()
    };
    let mut best: Option<Proposal> = None;
    for pattern in PATTERNS {
        let steps: Vec<String> = pattern
            .files
            .iter()
            .filter_map(|file| found(file))
            .collect();
        let better = best
            .as_ref()
            .is_none_or(|current| steps.len() > current.steps.len());
        if !steps.is_empty() && better {
            let missing = pattern
                .files
                .iter()
                .filter(|file| found(file).is_none())
                .map(|file| (*file).to_owned())
                .collect();
            best = Some(Proposal { steps, missing });
        }
    }
    best
}

/// Every file name a scan looks for, in table order, for SCAN-007's message.
pub fn looked_for() -> Vec<&'static str> {
    PATTERNS
        .iter()
        .flat_map(|pattern| pattern.files.iter().copied())
        .collect()
}
