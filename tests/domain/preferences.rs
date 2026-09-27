use buildpilot::domain::preferences::{Preferences, Theme, ThemeChoice};

// UI-001: first run follows Windows.
#[test]
fn defaults_follow_windows_with_nothing_remembered() {
    let preferences = Preferences::default();
    assert_eq!(preferences.theme, ThemeChoice::FollowWindows);
    assert_eq!(preferences.window, None);
    assert!(!preferences.tray.expanded);
    assert_eq!(preferences.tray.height, None);
}

#[test]
fn theme_choice_resolves() {
    for windows_uses_dark in [false, true] {
        assert_eq!(ThemeChoice::Light.resolve(windows_uses_dark), Theme::Light);
        assert_eq!(ThemeChoice::Dark.resolve(windows_uses_dark), Theme::Dark);
    }
    assert_eq!(ThemeChoice::FollowWindows.resolve(true), Theme::Dark);
    assert_eq!(ThemeChoice::FollowWindows.resolve(false), Theme::Light);
}
