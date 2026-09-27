//! Whether the output tray follows new output (OUT-006).

/// The tray's follow state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Follow {
    /// New output scrolls the tray to its last line.
    #[default]
    Following,
    /// The operator scrolled up; new output leaves the view where it is and the tray offers
    /// Jump to latest.
    Paused,
}

impl Follow {
    /// The state after the operator scrolls: following when the view reached the last line,
    /// paused otherwise.
    pub fn after_scroll(at_last_line: bool) -> Self {
        if at_last_line {
            Self::Following
        } else {
            Self::Paused
        }
    }

    /// The state after Jump to latest.
    pub fn jump_to_latest() -> Self {
        Self::Following
    }

    /// True when new output should scroll the tray.
    pub fn follows(self) -> bool {
        self == Self::Following
    }
}
