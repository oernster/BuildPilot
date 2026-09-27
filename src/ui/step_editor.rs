//! The operation dialog's steps and environment (STEP-008, ENV-002). The rules are in
//! `domain::step::StepList` and `domain::environment::offer`; this moves them to and from the
//! window.

use std::path::PathBuf;
use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::domain::environment::Offer;
use crate::domain::operation::{format_argument_lines, parse_argument_lines};
use crate::domain::step::StepSpec;

use super::{MainWindow, StepEditor, Ui};

pub(super) fn wire(ui: &Rc<Ui>, window: &MainWindow) {
    let editor = window.global::<StepEditor>();
    let this = ui.clone();
    editor.on_select(move |index| {
        this.change_steps(|steps| usize::try_from(index).is_ok_and(|index| steps.select(index)));
    });
    let this = ui.clone();
    editor.on_add(move || {
        let folder = this.with_window_value(|w| PathBuf::from(w.get_field_working_dir().trim()));
        if let Some(script) = this.pick_script(folder.as_deref()) {
            this.change_steps(|steps| {
                steps.add(StepSpec::for_script(&script));
                true
            });
        }
    });
    let this = ui.clone();
    editor.on_remove(move || this.change_steps(|steps| steps.remove()));
    let this = ui.clone();
    editor.on_up(move || this.change_steps(|steps| steps.move_up()));
    let this = ui.clone();
    editor.on_down(move || this.change_steps(|steps| steps.move_down()));
}

impl Ui {
    /// Keeps what was typed for the selected step, applies `change` to the list and shows the
    /// result; the environment offer follows, since the kinds of step may have changed.
    fn change_steps(&self, change: impl FnOnce(&mut crate::domain::step::StepList) -> bool) {
        self.keep_typed_step();
        let changed = self
            .dialog
            .borrow_mut()
            .as_mut()
            .is_some_and(|state| change(&mut state.steps));
        if changed {
            self.show_steps();
            self.refresh_environment();
        }
    }

    /// Stores the Script and Arguments fields into the selected step.
    pub(super) fn keep_typed_step(&self) {
        let Some(typed) = self.with_window_value(|w| StepSpec {
            script_path: PathBuf::from(w.get_field_script().trim()),
            arguments: parse_argument_lines(&w.get_field_arguments()),
        }) else {
            return;
        };
        if let Some(state) = self.dialog.borrow_mut().as_mut() {
            state.steps.set_current(typed);
        }
    }

    /// Shows the step list and the selected step's fields.
    pub(super) fn show_steps(&self) {
        let Some((labels, selected, current)) = self.dialog.borrow().as_ref().map(|state| {
            (
                state.steps.labels(),
                state.steps.selected(),
                state.steps.current().clone(),
            )
        }) else {
            return;
        };
        self.with_window(|w| {
            let editor = w.global::<StepEditor>();
            let labels: Vec<SharedString> = labels.into_iter().map(SharedString::from).collect();
            editor.set_labels(ModelRc::new(VecModel::from(labels)));
            editor.set_selected(i32::try_from(selected).unwrap_or(i32::MAX));
            w.set_field_script(SharedString::from(
                current.script_path.display().to_string(),
            ));
            w.set_field_arguments(SharedString::from(format_argument_lines(
                &current.arguments,
            )));
        });
    }

    /// Asks which environments the working directory holds for these steps and shows the
    /// answer, keeping the operator's choice while it is still one of those found (ENV-002).
    pub(super) fn refresh_environment(&self) {
        let Some(steps) = self
            .dialog
            .borrow()
            .as_ref()
            .map(|state| state.steps.steps().to_vec())
        else {
            return;
        };
        let Some(folder) =
            self.with_window_value(|w| PathBuf::from(w.get_field_working_dir().trim()))
        else {
            return;
        };
        let offer = self.app.borrow().environment_offer(&folder, &steps);
        self.with_window(|w| {
            let editor = w.global::<StepEditor>();
            let kept = editor.get_environment().to_string();
            match offer {
                Offer::Choose { found, preselected } => {
                    let current = if found.contains(&kept) {
                        kept
                    } else {
                        preselected.unwrap_or_default()
                    };
                    let names: Vec<SharedString> =
                        found.into_iter().map(SharedString::from).collect();
                    editor.set_environments(ModelRc::new(VecModel::from(names)));
                    editor.set_environment(SharedString::from(current));
                    editor.set_environment_note(SharedString::default());
                    editor.set_choose_environment(true);
                }
                Offer::Note(note) => {
                    editor.set_choose_environment(false);
                    editor.set_environment_note(SharedString::from(note));
                }
                Offer::Nothing => {
                    editor.set_choose_environment(false);
                    editor.set_environment_note(SharedString::default());
                }
            }
        });
    }

    /// The environment the dialog holds: the one chosen where a choice is offered, else none.
    pub(super) fn chosen_environment(&self) -> Option<String> {
        self.with_window_value(|w| {
            let editor = w.global::<StepEditor>();
            let chosen = editor.get_environment().to_string();
            (editor.get_choose_environment() && !chosen.is_empty()).then_some(chosen)
        })
        .flatten()
    }

    /// Starts the environment afresh for a newly opened dialog, on `environment` when given.
    pub(super) fn reset_environment(&self, environment: Option<String>) {
        self.with_window(|w| {
            let editor = w.global::<StepEditor>();
            editor.set_environment(SharedString::from(environment.unwrap_or_default()));
            editor.set_choose_environment(false);
        });
        self.refresh_environment();
    }
}
