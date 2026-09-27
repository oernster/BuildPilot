//! Which conversation this run of setup is having (INST-003), decided once from one reading of
//! the machine, so the screen, its heading, its options and its buttons cannot disagree.

use super::version::Version;

/// What the machine holds, read once before anything is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Installed {
    /// No install is recorded.
    Nothing,
    /// An install is recorded; its version, when the record states one that can be read.
    Recorded(Option<Version>),
}

/// What setup is for this run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// Nothing is installed.
    Install,
    /// An older version is installed; also one whose version cannot be read.
    Update {
        /// The version installed; `None` when the record does not say.
        from: Option<Version>,
    },
    /// A newer version is installed.
    Downgrade {
        /// The version installed.
        from: Version,
    },
    /// This version is installed: repair, reinstall or uninstall.
    Manage,
}

/// The route for a machine holding `installed` when setup carries `carried`. An install whose
/// version cannot be read is offered the carried version over it rather than being refused.
pub fn route(installed: Installed, carried: Version) -> Route {
    match installed {
        Installed::Nothing => Route::Install,
        Installed::Recorded(None) => Route::Update { from: None },
        Installed::Recorded(Some(from)) if from < carried => Route::Update { from: Some(from) },
        Installed::Recorded(Some(from)) if from > carried => Route::Downgrade { from },
        Installed::Recorded(Some(_)) => Route::Manage,
    }
}
