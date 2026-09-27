//! What an install, a repair or a removal will do, as one ordered list of steps.

/// The choices the route screen offers, opened on what the machine already holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Choices {
    /// A shortcut on the desktop.
    pub desktop_shortcut: bool,
    /// Start BuildPilot when setup finishes.
    pub launch_after: bool,
}

/// One piece of work setup does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Write the application and a copy of setup into the install folder.
    CopyFiles,
    /// The Start menu shortcut (INST-002); always present.
    StartMenuShortcut,
    /// Put the desktop shortcut in place.
    AddDesktopShortcut,
    /// Take the desktop shortcut away.
    RemoveDesktopShortcut,
    /// The Apps list entry with Modify, Repair and Uninstall (INST-002).
    Register,
    /// Take both shortcuts away.
    RemoveShortcuts,
    /// Take the Apps list entry away.
    Unregister,
    /// Delete the install folder.
    RemoveFiles,
    /// Delete the data folder: operations, settings, icons and the log (INST-005).
    RemoveData,
}

/// How much of the progress bar `step` takes: its measured time in milliseconds, so the bar
/// moves with the work rather than with the count of steps. Measured on 2026-09-27 from the
/// setup log of a real install, repair and uninstall of 0.1.0. A desktop shortcut is the same
/// call as the Start menu one; deleting the data folder is the same work as deleting the
/// program files. A step that took under a millisecond still takes one.
pub fn step_weight(step: Step) -> u32 {
    match step {
        Step::CopyFiles => 496,
        Step::StartMenuShortcut | Step::AddDesktopShortcut => 637,
        Step::Register => 5,
        Step::RemoveShortcuts => 3,
        Step::RemoveFiles | Step::RemoveData => 15,
        Step::RemoveDesktopShortcut | Step::Unregister => 1,
    }
}

/// The whole bar for `steps`; never zero, so a fraction of it is always defined.
pub fn weight(steps: &[Step]) -> u32 {
    steps
        .iter()
        .map(|step| step_weight(*step))
        .sum::<u32>()
        .max(1)
}

/// A fresh install, an update, a downgrade or a reinstall with `choices`.
pub fn install(choices: Choices) -> Vec<Step> {
    let desktop = if choices.desktop_shortcut {
        Step::AddDesktopShortcut
    } else {
        Step::RemoveDesktopShortcut
    };
    vec![
        Step::CopyFiles,
        Step::StartMenuShortcut,
        desktop,
        Step::Register,
    ]
}

/// A repair: the program files and what points at them are put back; nothing else changes.
pub fn repair() -> Vec<Step> {
    vec![Step::CopyFiles, Step::StartMenuShortcut, Step::Register]
}

/// A removal, keeping the data folder when `keep_data` (the default, INST-005).
pub fn uninstall(keep_data: bool) -> Vec<Step> {
    let mut steps = vec![Step::RemoveShortcuts, Step::Unregister, Step::RemoveFiles];
    if !keep_data {
        steps.push(Step::RemoveData);
    }
    steps
}
