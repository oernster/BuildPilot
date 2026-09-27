use buildpilot::domain::follow::Follow;

// OUT-006
#[test]
fn tray_follows_until_scrolled_up() {
    assert!(Follow::default().follows());
    assert_eq!(Follow::after_scroll(false), Follow::Paused);
    assert!(!Follow::after_scroll(false).follows());
    assert_eq!(Follow::after_scroll(true), Follow::Following);
    assert_eq!(Follow::jump_to_latest(), Follow::Following);
}
