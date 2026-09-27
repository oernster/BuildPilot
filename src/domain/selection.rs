//! Which row is selected and which rows are checked (ROW-006, ROW-007). Both are transient: they
//! are never persisted (OQ-7).

use std::collections::BTreeSet;

use super::operation::OperationId;

/// The selected row (whose output the tray shows) and the set of checked rows. The two are
/// independent: checking a row never selects it, selecting never checks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeckSelection {
    selected: Option<OperationId>,
    checked: BTreeSet<OperationId>,
}

impl DeckSelection {
    /// The selected row, if any.
    pub fn selected(&self) -> Option<&OperationId> {
        self.selected.as_ref()
    }

    /// Makes `id` the selected row.
    pub fn select(&mut self, id: OperationId) {
        self.selected = Some(id);
    }

    /// Ticks or unticks `id`; answers the new state.
    pub fn toggle_checked(&mut self, id: &OperationId) -> bool {
        if self.checked.remove(id) {
            return false;
        }
        self.checked.insert(id.clone());
        true
    }

    /// True when `id` is ticked.
    pub fn is_checked(&self, id: &OperationId) -> bool {
        self.checked.contains(id)
    }

    /// Every ticked row, in identity order.
    pub fn checked(&self) -> impl Iterator<Item = &OperationId> {
        self.checked.iter()
    }

    /// Drops every reference to `id`, for when its operation is removed.
    pub fn forget(&mut self, id: &OperationId) {
        self.checked.remove(id);
        if self.selected.as_ref() == Some(id) {
            self.selected = None;
        }
    }
}
