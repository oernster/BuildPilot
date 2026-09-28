//! Launching a project's installer (SRS 3.21).

use std::path::{Path, PathBuf};

use crate::domain::deck::DeckError;
use crate::domain::installer::{InstallerBlock, find_default_installer, launchable};
use crate::domain::operation::OperationId;

use super::{App, AppError};

impl App {
    /// The installer found in `working_dir` where PKG-002 looks; `None` when there is none. The
    /// operation dialog shows it when the operator has set none.
    pub fn default_installer(&self, working_dir: &Path) -> Option<PathBuf> {
        find_default_installer(working_dir, &|folder| self.ports.paths.files(folder))
    }

    /// The installer Launch installer would start for operation `id`, as last looked for: the
    /// one set in Edit, else the default; `None` when it is not on disk (PKG-001, PKG-002).
    pub fn installer(&self, id: &OperationId) -> Option<&Path> {
        self.installers.get(id)?.as_deref()
    }

    /// Why Launch installer is not available for operation `id`; `None` when it is (PKG-003,
    /// PKG-004).
    pub fn installer_block(&self, id: &OperationId) -> Option<InstallerBlock> {
        launchable(self.run_state(id), self.installer(id)).err()
    }

    /// Starts operation `id`'s installer the way Explorer would, so Windows asks for
    /// administrator rights where it needs them (PKG-005). Looks again first, since the file
    /// may have gone or appeared since the row was drawn.
    pub fn launch_installer(&mut self, id: &OperationId) -> Result<(), AppError> {
        self.deck
            .get(id)
            .ok_or_else(|| DeckError::NotFound(id.clone()))?;
        self.record_installer(id);
        let installer = launchable(self.run_state(id), self.installer(id))
            .map(Path::to_path_buf)
            .map_err(|block| AppError::Installer {
                name: self.name_of(id),
                block,
            })?;
        self.ports.shell.open(&installer).map_err(AppError::Shell)?;
        self.ports
            .log
            .record(&format!("Launched the installer {}", installer.display()));
        Ok(())
    }

    /// Looks for operation `id`'s installer and records what was found: the one set in Edit
    /// when it is on disk, else the default when none is set (PKG-001, PKG-002).
    pub(super) fn record_installer(&mut self, id: &OperationId) {
        let found = self.deck.get(id).and_then(|operation| {
            let config = operation.config();
            match config.installer() {
                Some(set) => self.ports.paths.is_file(set).then(|| set.to_path_buf()),
                None => self.default_installer(config.working_dir()),
            }
        });
        self.installers.insert(id.clone(), found);
    }
}
