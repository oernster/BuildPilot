//! The tray's rows, read straight from the application's output buffer rather than copied, so a
//! 100,000-line run costs nothing extra to show (OUT-004, OUT-005). Once the run has ended, one
//! more row follows the output: how it ended, in its outcome's colour (OUT-007).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use slint::{Model, ModelNotify, ModelTracker, SharedString};

use crate::application::App;
use crate::domain::operation::OperationId;

use super::rows::{StatusClass, closing_line};
use super::{LineKind, OutputLineData};

/// The selected operation's latest output, as a Slint model.
pub struct OutputModel {
    app: Rc<RefCell<App>>,
    selected: RefCell<Option<OperationId>>,
    notify: ModelNotify,
    /// Lines dropped and next row number at the last sync.
    shown: Cell<(u64, u64)>,
}

impl OutputModel {
    /// A model showing nothing until an operation is selected.
    pub fn new(app: Rc<RefCell<App>>) -> Self {
        Self {
            app,
            selected: RefCell::new(None),
            notify: ModelNotify::default(),
            shown: Cell::new((0, 0)),
        }
    }

    /// Shows operation `id`'s output from the start.
    pub fn show(&self, id: Option<OperationId>) {
        *self.selected.borrow_mut() = id;
        self.shown.set(self.counts());
        self.notify.reset();
    }

    /// Catches up with rows added or dropped since the last sync. Answers true when rows were
    /// added.
    pub fn sync(&self) -> bool {
        let (dropped, next) = self.counts();
        let (shown_dropped, shown_next) = self.shown.get();
        self.shown.set((dropped, next));
        if next < shown_next || dropped > shown_dropped {
            // A new run; else old lines fell off the front. Either way, re-read everything.
            self.notify.reset();
            return next > 0;
        }
        if next > shown_next {
            let start = usize::try_from(shown_next - dropped).unwrap_or(usize::MAX);
            let count = usize::try_from(next - shown_next).unwrap_or(usize::MAX);
            self.notify.row_added(start, count);
            return true;
        }
        false
    }

    /// Lines dropped and the next row number of the selected operation's output; the closing
    /// line, once there is one, counts as the last row.
    fn counts(&self) -> (u64, u64) {
        let selected = self.selected.borrow();
        let app = self.app.borrow();
        let Some(id) = selected.as_ref() else {
            return (0, 0);
        };
        let closing = u64::from(self.closing(&app, id).is_some());
        app.output(id).map_or((0, closing), |buffer| {
            (buffer.dropped(), buffer.next_line_number() + closing)
        })
    }

    /// The closing line for operation `id`'s latest run, once it has ended.
    fn closing(&self, app: &App, id: &OperationId) -> Option<OutputLineData> {
        let (text, class) = closing_line(app.run_state(id), app.elapsed(id))?;
        Some(OutputLineData {
            text: SharedString::from(text),
            kind: match class {
                StatusClass::Succeeded => LineKind::Succeeded,
                StatusClass::Failed => LineKind::Failed,
                StatusClass::Stopped | StatusClass::Idle | StatusClass::Running => {
                    LineKind::Stopped
                }
            },
        })
    }
}

impl Model for OutputModel {
    type Data = OutputLineData;

    fn row_count(&self) -> usize {
        let selected = self.selected.borrow();
        let app = self.app.borrow();
        let Some(id) = selected.as_ref() else {
            return 0;
        };
        let lines = app.output(id).map_or(0, |buffer| buffer.len());
        lines + usize::from(self.closing(&app, id).is_some())
    }

    fn row_data(&self, row: usize) -> Option<Self::Data> {
        let selected = self.selected.borrow();
        let app = self.app.borrow();
        let id = selected.as_ref()?;
        match app.output(id).and_then(|buffer| buffer.get(row)) {
            Some(line) => Some(OutputLineData {
                text: SharedString::from(line.text.as_str()),
                kind: LineKind::Plain,
            }),
            None if row == app.output(id).map_or(0, |buffer| buffer.len()) => {
                self.closing(&app, id)
            }
            None => None,
        }
    }

    fn model_tracker(&self) -> &dyn ModelTracker {
        &self.notify
    }
}
