//! The tray's size, the theme and the window's remembered position (CFG-010, UI-001).

use slint::ComponentHandle;

use crate::domain::preferences::{ThemeChoice, TrayLayout, WindowGeometry};

use super::Ui;

/// The tray never gets shorter than this, in logical pixels.
const MIN_TRAY_HEIGHT: f32 = 120.0;
/// Nor taller than this share of the window.
const MAX_TRAY_SHARE: f32 = 0.8;
/// Positions below this mean Windows parked a minimised window off screen.
const OFF_SCREEN: i32 = -10_000;

impl Ui {
    pub(super) fn show_tray(&self) {
        let expanded = self.tray_expanded.get();
        let height = self.tray_height.get();
        self.with_window(|window| {
            window.set_tray_expanded(expanded);
            window.set_tray_height(height);
        });
    }

    pub(super) fn resize_tray(&self, by: f32) {
        let window_height = self.window.upgrade().map_or(f32::MAX, |window| {
            window.window().size().height as f32 / window.window().scale_factor()
        });
        let wanted = (self.tray_height.get() + by)
            .min(window_height * MAX_TRAY_SHARE)
            .max(MIN_TRAY_HEIGHT);
        self.tray_height.set(wanted);
        self.show_tray();
    }

    pub(super) fn apply_theme(&self) {
        let choice = self.app.borrow().preferences().theme;
        let dark = choice.resolve(self.environment.windows_uses_dark)
            == crate::domain::preferences::Theme::Dark;
        self.with_window(|window| window.invoke_apply_theme(dark));
    }

    pub(super) fn save_layout(&self) {
        let Some(window) = self.window.upgrade() else {
            return;
        };
        let position = window.window().position();
        let size = window.window().size();
        let geometry = (!window.window().is_minimized() && position.x > OFF_SCREEN).then_some(
            WindowGeometry {
                x: position.x,
                y: position.y,
                width: size.width,
                height: size.height,
            },
        );
        let tray = TrayLayout {
            expanded: self.tray_expanded.get(),
            height: Some(self.tray_height.get().round() as u32),
        };
        let mut app = self.app.borrow_mut();
        let window = geometry.or(app.preferences().window);
        app.set_layout(window, tray);
    }
}

/// Settings' theme buttons, left to right: 0 Light, 1 Dark. Following the system theme is the
/// first-run default only; Settings no longer offers it (UI-004, Amendment 6).
pub(super) fn theme_from_index(index: i32) -> ThemeChoice {
    if index == 0 {
        ThemeChoice::Light
    } else {
        ThemeChoice::Dark
    }
}
