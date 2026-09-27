//! The tray's rows, read straight from the application's output buffer rather than copied, so a
//! 100,000-line run costs nothing extra to show (OUT-004, OUT-005).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use slint::{Model, ModelNotify, ModelTracker, SharedString};

use crate::application::App;
use crate::domain::operation::OperationId;
use crate::domain::output::Stream;

use super::OutputLineData;

/// The selected operation's latest output, as a Slint model.
pub struct OutputModel {
    app: Rc<RefCell<App>>,
    selected: RefCell<Option<OperationId>>,
    notify: ModelNotify,
    /// Lines dropped and next line number at the last sync.
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

    /// Catches up with lines added or dropped since the last sync. Answers true when lines were
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

    /// Lines dropped and the next line number of the selected operation's output.
    fn counts(&self) -> (u64, u64) {
        let selected = self.selected.borrow();
        let app = self.app.borrow();
        selected
            .as_ref()
            .and_then(|id| app.output(id))
            .map_or((0, 0), |buffer| {
                (buffer.dropped(), buffer.next_line_number())
            })
    }
}

impl Model for OutputModel {
    type Data = OutputLineData;

    fn row_count(&self) -> usize {
        let selected = self.selected.borrow();
        let app = self.app.borrow();
        selected
            .as_ref()
            .and_then(|id| app.output(id))
            .map_or(0, |buffer| buffer.len())
    }

    fn row_data(&self, row: usize) -> Option<Self::Data> {
        let selected = self.selected.borrow();
        let app = self.app.borrow();
        let line = app.output(selected.as_ref()?)?.get(row)?;
        Some(OutputLineData {
            text: SharedString::from(line.text.as_str()),
            err: line.stream == Stream::Stderr,
        })
    }

    fn model_tracker(&self) -> &dyn ModelTracker {
        &self.notify
    }
}
