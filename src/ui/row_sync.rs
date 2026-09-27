//! Building the rows the window shows from application state, updated in place so a drag
//! in progress is never torn down by a refresh.

use std::collections::HashMap;
use std::path::Path;

use slint::{Image, Model, SharedString};

use crate::application::{App, IconStatus};
use crate::domain::operation::{Operation, OperationId};

use super::rows::{self, RowFacts, StatusClass};
use super::{RowData, StatusKind, Ui};

/// What `selected-index` holds when no row is selected; the window reads it that way.
const NONE_SELECTED: i32 = -1;

impl Ui {
    /// Updates rows in place, so a drag in progress is never torn down by a refresh.
    pub(super) fn sync_rows(&self) {
        let fresh = {
            let app = self.app.borrow();
            let overdue: HashMap<OperationId, u32> = app
                .overdue_stops()
                .into_iter()
                .map(|stop| (stop.id, stop.pid))
                .collect();
            app.deck()
                .operations()
                .iter()
                .map(|operation| {
                    self.row_data(&app, operation, overdue.get(operation.id()).copied())
                })
                .collect::<Vec<_>>()
        };
        let selected = fresh
            .iter()
            .position(|row| row.selected)
            .and_then(|index| i32::try_from(index).ok())
            .unwrap_or(NONE_SELECTED);
        self.with_window(|window| window.set_selected_index(selected));
        let same_rows = fresh.len() == self.rows.row_count()
            && fresh.iter().enumerate().all(|(index, row)| {
                self.rows
                    .row_data(index)
                    .is_some_and(|old| old.id == row.id)
            });
        if !same_rows {
            self.rows.set_vec(fresh);
            return;
        }
        for (index, row) in fresh.into_iter().enumerate() {
            if self.rows.row_data(index).as_ref() != Some(&row) {
                self.rows.set_row_data(index, row);
            }
        }
    }

    pub(super) fn row_data(
        &self,
        app: &App,
        operation: &Operation,
        overdue_pid: Option<u32>,
    ) -> RowData {
        let id = operation.id();
        let config = operation.config();
        let text = rows::row_text(&RowFacts {
            name: config.name(),
            script: config.script_path(),
            state: app.run_state(id),
            elapsed: app.elapsed(id),
            overdue_pid,
            tick: self.tick.get(),
        });
        let (icon, icon_is_placeholder, icon_problem) = self.icon_for(&app.icon_status(id));
        RowData {
            id: SharedString::from(id.as_str()),
            name: SharedString::from(config.name()),
            detail: SharedString::from(text.detail),
            icon,
            icon_is_placeholder,
            icon_problem: SharedString::from(icon_problem),
            status: SharedString::from(text.status),
            glyph: SharedString::from(text.glyph),
            kind: status_kind(text.class),
            problem: SharedString::from(text.problem),
            checked: app.selection().is_checked(id),
            selected: app.selection().selected() == Some(id),
            can_run: text.can_run,
            can_stop: text.can_stop,
            can_remove: text.can_remove,
        }
    }

    /// The image for an icon status: the image, whether to draw the placeholder instead and
    /// what to say about a problem.
    pub(super) fn icon_for(&self, status: &IconStatus) -> (Image, bool, String) {
        match status {
            IconStatus::Ready(path) => match self.load_image(path) {
                Some(image) => (image, false, String::new()),
                None => (
                    Image::default(),
                    true,
                    format!("Icon could not be loaded: {}", path.display()),
                ),
            },
            other => (Image::default(), true, rows::icon_problem(other)),
        }
    }

    /// Loads and caches an image; the same path is decoded only once per session.
    pub(super) fn load_image(&self, path: &Path) -> Option<Image> {
        self.icons
            .borrow_mut()
            .entry(path.to_path_buf())
            .or_insert_with(|| Image::load_from_path(path).ok())
            .clone()
    }
}

fn status_kind(class: StatusClass) -> StatusKind {
    match class {
        StatusClass::Idle => StatusKind::Idle,
        StatusClass::Running => StatusKind::Running,
        StatusClass::Succeeded => StatusKind::Succeeded,
        StatusClass::Failed => StatusKind::Failed,
        StatusClass::Stopped => StatusKind::Stopped,
    }
}
