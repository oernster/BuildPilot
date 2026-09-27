//! The Add and Edit dialog (SRS 3.2 to 3.4) and its file pickers.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use slint::{ComponentHandle, Image, SharedString};

use crate::domain::launch_plan::ScriptKind;
use crate::domain::operation::{IconRef, OperationId, OperationSpec};
use crate::domain::step::StepList;

use super::{MainWindow, StepEditor, Ui};

/// Image types an icon may be (ICON-004).
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "ico"];

/// Whether the dialog adds or edits; the icon and steps it currently holds.
pub(super) struct DialogState {
    editing: Option<OperationId>,
    icon: IconRef,
    pub(super) steps: StepList,
}

pub(super) fn wire(ui: &Rc<Ui>, window: &MainWindow) {
    let this = ui.clone();
    window.on_add(move || this.open_add_choice());
    let this = ui.clone();
    window.on_edit(move |row| {
        if let Ok(id) = OperationId::new(row.as_str()) {
            this.open_edit(id);
        }
    });
    let this = ui.clone();
    window.on_browse_script(move || {
        if let Some(script) = pick_script(None) {
            this.with_window(|w| {
                w.set_field_script(SharedString::from(script.display().to_string()))
            });
        }
    });
    let this = ui.clone();
    window.on_browse_folder(move || {
        if let Some(folder) = rfd::FileDialog::new()
            .set_title("Working directory")
            .pick_folder()
        {
            this.with_window(|w| {
                w.set_field_working_dir(SharedString::from(folder.display().to_string()))
            });
            this.refresh_environment();
        }
    });
    let this = ui.clone();
    window.on_choose_icon(move || {
        let picked = rfd::FileDialog::new()
            .set_title("Choose an icon")
            .add_filter("Images", IMAGE_EXTENSIONS)
            .pick_file();
        if let Some(image) = picked {
            this.set_dialog_icon(IconRef::Chosen(image));
        }
    });
    let this = ui.clone();
    window.on_use_placeholder_icon(move || this.set_dialog_icon(IconRef::Placeholder));
    let this = ui.clone();
    window.on_save_operation(move || this.save_dialog());
    let this = ui.clone();
    window.on_cancel_operation(move || this.close_dialog());
}

/// Asks for a build script, starting in `folder` when given (ADD-001, STEP-008).
pub(super) fn pick_script(folder: Option<&Path>) -> Option<PathBuf> {
    let extensions: Vec<&str> = ScriptKind::supported_extensions().collect();
    let mut picker = rfd::FileDialog::new()
        .set_title("Choose a build script")
        .add_filter("Build scripts", &extensions);
    if let Some(folder) = folder {
        picker = picker.set_directory(folder);
    }
    picker.pick_file()
}

impl Ui {
    fn open_edit(&self, id: OperationId) {
        let (spec, running) = {
            let app = self.app.borrow();
            let Some(operation) = app.deck().get(&id) else {
                return;
            };
            (operation.config().to_spec(), app.is_running(&id))
        };
        let note = if running {
            "This operation is running. Changes to its steps, folder or environment take effect on its next run."
                .to_owned()
        } else {
            String::new()
        };
        self.open_dialog(Some(id), spec, "Edit build operation", "Save", note);
    }

    pub(super) fn open_dialog(
        &self,
        editing: Option<OperationId>,
        spec: OperationSpec,
        heading: &str,
        save_label: &str,
        note: String,
    ) {
        self.with_window(|w| {
            w.set_operation_heading(SharedString::from(heading));
            w.set_operation_save_label(SharedString::from(save_label));
            w.set_field_name(SharedString::from(spec.name.as_str()));
            w.set_field_working_dir(SharedString::from(spec.working_dir.display().to_string()));
            w.set_operation_note(SharedString::from(note));
            w.set_operation_error(SharedString::default());
        });
        *self.dialog.borrow_mut() = Some(DialogState {
            editing,
            icon: IconRef::Placeholder,
            steps: StepList::new(spec.steps),
        });
        self.show_steps();
        self.reset_environment(spec.environment);
        // A folder scan sets its own warning after this (SCAN-009); anything else has none.
        self.with_window(|w| {
            w.global::<StepEditor>()
                .set_warning(SharedString::default())
        });
        self.set_dialog_icon(spec.icon);
        self.with_window(|w| w.set_show_operation_dialog(true));
    }

    fn set_dialog_icon(&self, icon: IconRef) {
        let (image, placeholder, note) = match &icon {
            IconRef::Placeholder => (Image::default(), true, "Placeholder".to_owned()),
            IconRef::Discovered(path) | IconRef::Chosen(path) => match self.load_image(path) {
                Some(image) => {
                    let found = matches!(icon, IconRef::Discovered(_));
                    (
                        image,
                        false,
                        if found {
                            "Found beside the script"
                        } else {
                            "Chosen image"
                        }
                        .to_owned(),
                    )
                }
                None => (
                    Image::default(),
                    true,
                    format!("Cannot show {}", path.display()),
                ),
            },
        };
        if let Some(state) = self.dialog.borrow_mut().as_mut() {
            state.icon = icon;
        }
        self.with_window(|w| {
            w.set_field_icon(image);
            w.set_field_icon_is_placeholder(placeholder);
            w.set_field_icon_note(SharedString::from(note));
        });
    }

    fn save_dialog(&self) {
        let Some(window) = self.window.upgrade() else {
            return;
        };
        self.keep_typed_step();
        // The folder may have been typed rather than browsed: the choice must match it now.
        self.refresh_environment();
        let Some((editing, icon, steps)) = self.dialog.borrow().as_ref().map(|state| {
            (
                state.editing.clone(),
                state.icon.clone(),
                state.steps.steps().to_vec(),
            )
        }) else {
            return;
        };
        let spec = OperationSpec {
            name: window.get_field_name().to_string(),
            steps,
            working_dir: PathBuf::from(window.get_field_working_dir().trim()),
            environment: self.chosen_environment(),
            icon,
        };
        let outcome = match &editing {
            None => self.app.borrow_mut().add(spec).map(|_| None),
            Some(id) => self.app.borrow_mut().edit(id, spec).map(Some),
        };
        match outcome {
            Ok(edited) => {
                self.close_dialog();
                if let (Some(outcome), Some(id)) = (edited, &editing)
                    && outcome.applies_next_run
                {
                    let name = self.name_of(id);
                    self.notify(format!(
                        "{name} is running: your changes take effect on its next run."
                    ));
                }
                self.refresh();
            }
            Err(error) => window.set_operation_error(SharedString::from(error.to_string())),
        }
    }

    fn close_dialog(&self) {
        self.dialog.borrow_mut().take();
        self.with_window(|w| w.set_show_operation_dialog(false));
    }

    fn name_of(&self, id: &OperationId) -> String {
        self.app
            .borrow()
            .deck()
            .get(id)
            .map_or_else(|| id.to_string(), |op| op.config().name().to_owned())
    }
}
