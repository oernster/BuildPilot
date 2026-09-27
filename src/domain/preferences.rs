//! Global preferences (CFG-003, CFG-010, UI-001, UI-004, UI-010).

use super::version::Version;

/// The theme the operator chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeChoice {
    /// Always light.
    Light,
    /// Always dark.
    Dark,
    /// Whatever Windows uses for apps; the first-run default (UI-001).
    #[default]
    FollowWindows,
}

/// The theme actually shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    /// Light.
    Light,
    /// Dark.
    Dark,
}

impl ThemeChoice {
    /// The theme to show, given whether Windows currently uses the dark app theme.
    pub fn resolve(self, windows_uses_dark: bool) -> Theme {
        match self {
            Self::Light => Theme::Light,
            Self::Dark => Theme::Dark,
            Self::FollowWindows if windows_uses_dark => Theme::Dark,
            Self::FollowWindows => Theme::Light,
        }
    }
}

/// The main window's last position and size, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowGeometry {
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
}

/// The output tray's last layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TrayLayout {
    /// True when the tray was expanded.
    pub expanded: bool,
    /// The tray's height in physical pixels; `None` until the operator first resizes it.
    pub height: Option<u32>,
}

/// Everything global BuildPilot remembers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Preferences {
    /// The chosen theme.
    pub theme: ThemeChoice,
    /// The window's geometry; `None` until the window has been shown once.
    pub window: Option<WindowGeometry>,
    /// The tray's layout.
    pub tray: TrayLayout,
    /// The release the operator chose not to hear about unbidden (UI-010); `None` until one is.
    pub skipped_update: Option<Version>,
}
