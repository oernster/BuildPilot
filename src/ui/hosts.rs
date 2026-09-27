//! The host table in Settings (HOST-001). The rules are in `domain::host`; this moves the table
//! to and from the window and hands every change to `App::set_hosts`.

use std::path::PathBuf;
use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::domain::host::HostRow;
use crate::domain::launch_plan::command_line;
use crate::domain::operation::parse_argument_lines;

use super::{HostEditor, MainWindow, Ui};

/// Program types a host may be.
const PROGRAM_EXTENSIONS: &[&str] = &["exe", "com", "bat", "cmd"];

pub(super) fn wire(ui: &Rc<Ui>, window: &MainWindow) {
    let editor = window.global::<HostEditor>();
    let this = ui.clone();
    editor.on_add(move || this.add_host());
    let this = ui.clone();
    editor.on_remove(move |index| {
        if let Ok(index) = usize::try_from(index) {
            this.remove_host(index);
        }
    });
    let this = ui.clone();
    editor.on_browse(move || {
        if let Some(program) = rfd::FileDialog::new()
            .set_title("Choose the program that runs these scripts")
            .add_filter("Programs", PROGRAM_EXTENSIONS)
            .pick_file()
        {
            this.with_window(|w| {
                w.global::<HostEditor>()
                    .set_program(SharedString::from(program.display().to_string()));
            });
        }
    });
}

impl Ui {
    /// Shows the table as it stands, with an empty row to write and no refusal.
    pub(super) fn show_hosts(&self) {
        let labels: Vec<SharedString> = self
            .app
            .borrow()
            .preferences()
            .hosts
            .rows()
            .iter()
            .map(|row| {
                let command = command_line(row.program(), row.arguments());
                SharedString::from(format!(".{}: {command}", row.extension()))
            })
            .collect();
        self.with_window(|w| {
            let editor = w.global::<HostEditor>();
            editor.set_rows(ModelRc::new(VecModel::from(labels)));
            editor.set_extension(SharedString::default());
            editor.set_program(SharedString::default());
            editor.set_arguments(SharedString::default());
            editor.set_error(SharedString::default());
        });
    }

    /// Adds the written row at the end of the table.
    fn add_host(&self) {
        let Some(typed) = self.with_window_value(|w| {
            let editor = w.global::<HostEditor>();
            HostRow::new(
                &editor.get_extension(),
                PathBuf::from(editor.get_program().trim()),
                parse_argument_lines(&editor.get_arguments()),
            )
        }) else {
            return;
        };
        let row = match typed {
            Ok(row) => row,
            Err(error) => return self.refuse_host(&error.to_string()),
        };
        let mut rows = self.app.borrow().preferences().hosts.rows().to_vec();
        rows.push(row);
        self.save_hosts(rows);
    }

    /// Removes row `index` from the table.
    fn remove_host(&self, index: usize) {
        let mut rows = self.app.borrow().preferences().hosts.rows().to_vec();
        if index < rows.len() {
            rows.remove(index);
            self.save_hosts(rows);
        }
    }

    /// Hands `rows` to the application; shows the table on success, the refusal otherwise.
    fn save_hosts(&self, rows: Vec<HostRow>) {
        let saved = self.app.borrow_mut().set_hosts(rows);
        match saved {
            Ok(()) => self.show_hosts(),
            Err(error) => {
                self.app.borrow().record_refusal(&error);
                self.refuse_host(&error.to_string());
            }
        }
    }

    fn refuse_host(&self, message: &str) {
        self.with_window(|w| {
            w.global::<HostEditor>()
                .set_error(SharedString::from(message));
        });
    }
}
