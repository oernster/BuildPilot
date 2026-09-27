//! Configuring the flight deck: add, edit, remove, reorder, select and preferences
//! (SRS 3.2 to 3.6, 3.12).

use std::path::Path;

use crate::domain::deck::DeckError;
use crate::domain::operation::{
    IconRef, Operation, OperationConfig, OperationId, OperationSpec, draft_for_script,
};
use crate::domain::preferences::{ThemeChoice, TrayLayout, WindowGeometry};
use crate::domain::version::Version;

use super::{App, AppError};

/// What an edit did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditOutcome {
    /// True when the operation is running and the change alters what a run launches, so it
    /// takes effect on the next run only (EDIT-002).
    pub applies_next_run: bool,
}

impl App {
    /// The Add dialog's starting values for `script` (ADD-001 to ADD-004), with the
    /// conventional icon when one is found beside it (ICON-001).
    pub fn draft_for(&self, script: &Path) -> Result<OperationSpec, AppError> {
        let mut spec = draft_for_script(script)?;
        if let Some(icon) = self.ports.icons.discover(&spec.working_dir) {
            spec.icon = IconRef::Discovered(icon);
        }
        Ok(spec)
    }

    /// Adds an operation as the bottom row and saves (CFG-001).
    pub fn add(&mut self, spec: OperationSpec) -> Result<OperationId, AppError> {
        let config = OperationConfig::try_from(spec)?;
        let id = self.ports.ids.next_id();
        let config = self.settle_icon(&id, config, None)?;
        self.record_icon_status(&id, config.icon());
        self.deck.add(Operation::new(id.clone(), config))?;
        self.persist();
        Ok(id)
    }

    /// Replaces operation `id`'s configuration and saves. A running process is never touched
    /// (EDIT-002).
    pub fn edit(&mut self, id: &OperationId, spec: OperationSpec) -> Result<EditOutcome, AppError> {
        let current = self
            .deck
            .get(id)
            .ok_or_else(|| DeckError::NotFound(id.clone()))?
            .config()
            .clone();
        let config = OperationConfig::try_from(spec)?;
        let config = self.settle_icon(id, config, Some(current.icon()))?;
        let applies_next_run = self.is_running(id) && current.differs_in_execution(&config);
        self.record_icon_status(id, config.icon());
        self.deck.replace_config(id, config)?;
        self.persist();
        Ok(EditOutcome { applies_next_run })
    }

    /// Removes operation `id` from BuildPilot and saves. The script is never touched; a copied
    /// icon is deleted (REM-002). Refused while it runs (REM-003).
    pub fn remove(&mut self, id: &OperationId) -> Result<(), AppError> {
        if self.is_running(id) {
            return Err(AppError::RemoveWhileRunning(self.name_of(id)));
        }
        let removed = self.deck.remove(id)?;
        if matches!(removed.config().icon(), IconRef::Chosen(_)) {
            self.ports.icons.release(id);
        }
        self.selection.forget(id);
        self.runtimes.remove(id);
        self.icon_status.remove(id);
        self.persist();
        Ok(())
    }

    /// Drops operation `id` at row `target` and saves (ROW-003).
    pub fn move_to(&mut self, id: &OperationId, target: usize) -> Result<(), AppError> {
        self.deck.move_to(id, target)?;
        self.persist();
        Ok(())
    }

    /// Moves operation `id` one row up, saving when it moved (ROW-004).
    pub fn move_up(&mut self, id: &OperationId) -> Result<bool, AppError> {
        let moved = self.deck.move_up(id)?;
        if moved {
            self.persist();
        }
        Ok(moved)
    }

    /// Moves operation `id` one row down, saving when it moved (ROW-004).
    pub fn move_down(&mut self, id: &OperationId) -> Result<bool, AppError> {
        let moved = self.deck.move_down(id)?;
        if moved {
            self.persist();
        }
        Ok(moved)
    }

    /// Makes operation `id` the selected row, whose output the tray shows (ROW-007).
    pub fn select(&mut self, id: &OperationId) -> Result<(), AppError> {
        self.deck
            .get(id)
            .ok_or_else(|| DeckError::NotFound(id.clone()))?;
        self.selection.select(id.clone());
        Ok(())
    }

    /// Ticks or unticks operation `id`'s checkbox; answers the new state (ROW-006).
    pub fn toggle_checked(&mut self, id: &OperationId) -> Result<bool, AppError> {
        self.deck
            .get(id)
            .ok_or_else(|| DeckError::NotFound(id.clone()))?;
        Ok(self.selection.toggle_checked(id))
    }

    /// Sets and saves the theme (UI-001).
    pub fn set_theme(&mut self, theme: ThemeChoice) {
        self.preferences.theme = theme;
        self.persist();
    }

    /// Records and saves the release the operator chose not to hear about unbidden (UI-010).
    pub fn skip_update(&mut self, version: Version) {
        self.preferences.skipped_update = Some(version);
        self.persist();
    }

    /// Records and saves the window and tray layout (CFG-010).
    pub fn set_layout(&mut self, window: Option<WindowGeometry>, tray: TrayLayout) {
        self.preferences.window = window;
        self.preferences.tray = tray;
        self.persist();
    }

    /// Settles the icon of a new or edited configuration. A newly chosen image is copied into
    /// the data folder (OQ-6); leaving a chosen image behind deletes its copy.
    fn settle_icon(
        &mut self,
        id: &OperationId,
        config: OperationConfig,
        previous: Option<&IconRef>,
    ) -> Result<OperationConfig, AppError> {
        let unchanged = previous == Some(config.icon());
        match config.icon() {
            IconRef::Chosen(_) if unchanged => Ok(config),
            IconRef::Chosen(source) => {
                let stored = self.ports.icons.import(id, source).map_err(|message| {
                    AppError::IconImport {
                        source: source.clone(),
                        message,
                    }
                })?;
                Ok(config.with_icon(IconRef::Chosen(stored)))
            }
            IconRef::Placeholder | IconRef::Discovered(_) => {
                if matches!(previous, Some(IconRef::Chosen(_))) {
                    self.ports.icons.release(id);
                }
                Ok(config)
            }
        }
    }
}
