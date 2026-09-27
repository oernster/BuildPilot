use std::time::Duration;

use buildpilot::domain::elapsed::format_elapsed;

// LIFE-005: m:ss, rounded down, minutes uncapped.
#[test]
fn formats_minutes_and_seconds() {
    let cases = [
        (Duration::ZERO, "0:00"),
        (Duration::from_millis(9_999), "0:09"),
        (Duration::from_secs(42), "0:42"),
        (Duration::from_secs(61), "1:01"),
        (Duration::from_secs(75 * 60), "75:00"),
    ];
    for (duration, expected) in cases {
        assert_eq!(format_elapsed(duration), expected);
    }
}
