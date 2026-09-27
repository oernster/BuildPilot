//! Adding from a folder (SRS 3.20): Add's choice, the scan and the list of projects. What a
//! scan proposes is decided in `App::scan_folder`; this shows it and adds what is confirmed.

use std::path::Path;
use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::application::{FolderScan, ScannedFolder};

use super::dialog::pick_script;
use super::{MainWindow, ScanList, ScannedRow, StepEditor, Ui};

/// The projects a list shows, each with whether it is ticked.
pub(super) type Listed = Vec<(ScannedFolder, bool)>;

pub(super) fn wire(ui: &Rc<Ui>, window: &MainWindow) {
    let list = window.global::<ScanList>();
    let this = ui.clone();
    list.on_choose_script(move || {
        this.close_scan();
        this.add_script_from(None);
    });
    let this = ui.clone();
    list.on_choose_folder(move || {
        this.close_scan();
        if let Some(folder) = rfd::FileDialog::new()
            .set_title("Choose a folder to recognise")
            .pick_folder()
        {
            this.add_from_folder(&folder);
        }
    });
    let this = ui.clone();
    list.on_toggle(move |index| {
        if let Ok(index) = usize::try_from(index)
            && let Some(entry) = this.scanned.borrow_mut().get_mut(index)
        {
            entry.1 = !entry.1;
        }
        this.show_scanned();
    });
    let this = ui.clone();
    list.on_add_ticked(move || this.add_ticked());
    let this = ui.clone();
    list.on_cancel(move || this.close_scan());
}

impl Ui {
    /// SCAN-002: Add asks whether to take a script or a folder.
    pub(super) fn open_add_choice(&self) {
        self.with_window(|w| w.global::<ScanList>().set_choosing(true));
    }

    /// ADD-001: a chosen script opens the dialog with its defaults.
    pub(super) fn add_script_from(&self, folder: Option<&Path>) {
        let Some(script) = pick_script(folder) else {
            return;
        };
        let draft = self.app.borrow().draft_for(&script);
        if let Some(spec) = self.report(draft) {
            self.open_dialog(None, spec, "Add a build script", "Add", String::new());
        }
    }

    /// SCAN-004, SCAN-005, SCAN-007: what the scan found decides what opens.
    fn add_from_folder(&self, folder: &Path) {
        let scan = self.app.borrow().scan_folder(folder);
        match scan {
            FolderScan::One(found) => {
                let warning = found.warning.unwrap_or_default();
                self.open_dialog(None, found.spec, "Add a build", "Add", String::new());
                self.with_window(|w| {
                    w.global::<StepEditor>()
                        .set_warning(SharedString::from(warning))
                });
            }
            FolderScan::Several(found) => {
                *self.scanned.borrow_mut() = found
                    .into_iter()
                    .map(|folder| {
                        let ticked = folder.unticked_because.is_none();
                        (folder, ticked)
                    })
                    .collect();
                self.show_scanned();
                self.with_window(|w| {
                    let list = w.global::<ScanList>();
                    list.set_folder(SharedString::from(folder.display().to_string()));
                    list.set_listing(true);
                });
            }
            FolderScan::Nothing(looked_for) => {
                self.notify(format!(
                    "No build was recognised in {}: BuildPilot looked for {}. Choose its script instead.",
                    folder.display(),
                    looked_for.join(", ")
                ));
                self.refresh();
                self.add_script_from(Some(folder));
            }
        }
    }

    /// Shows the listed projects with their ticks.
    fn show_scanned(&self) {
        let rows: Vec<ScannedRow> = self
            .scanned
            .borrow()
            .iter()
            .map(|(folder, ticked)| ScannedRow {
                name: SharedString::from(folder.spec.name.as_str()),
                steps: SharedString::from(steps_in_words(folder)),
                ticked: *ticked,
                warning: SharedString::from(folder.warning.clone().unwrap_or_default()),
                note: SharedString::from(folder.unticked_because.clone().unwrap_or_default()),
            })
            .collect();
        self.with_window(|w| {
            w.global::<ScanList>()
                .set_rows(ModelRc::new(VecModel::from(rows)))
        });
    }

    /// Adds every ticked project in list order; a refusal is reported and the rest still added.
    fn add_ticked(&self) {
        let ticked: Vec<ScannedFolder> = self
            .scanned
            .borrow()
            .iter()
            .filter(|(_, ticked)| *ticked)
            .map(|(folder, _)| folder.clone())
            .collect();
        self.close_scan();
        for folder in ticked {
            let added = self.app.borrow_mut().add(folder.spec);
            self.report(added);
        }
        self.refresh();
    }

    fn close_scan(&self) {
        self.scanned.borrow_mut().clear();
        self.with_window(|w| {
            let list = w.global::<ScanList>();
            list.set_choosing(false);
            list.set_listing(false);
        });
    }
}

/// `buildexe.py then buildinstaller.py`.
fn steps_in_words(folder: &ScannedFolder) -> String {
    folder
        .spec
        .steps
        .iter()
        .filter_map(|step| step.script_path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(" then ")
}
