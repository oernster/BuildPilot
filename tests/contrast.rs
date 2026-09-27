//! UI-002 and A11Y-002: every text colour reaches WCAG 2.2 AA contrast (4.5:1) against the
//! background it is drawn on, in both themes; the rings reach the non-text minimum (3:1). The
//! colours are read from ui/theme.slint, their one home.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// WCAG 2.2 AA minimum for normal text.
const MINIMUM_RATIO: f64 = 4.5;

/// Text token drawn on background token, as the UI actually pairs them.
const PAIRS: &[(&str, &str)] = &[
    ("text", "background"),
    ("text", "surface"),
    ("text", "surface-hover"),
    ("text", "surface-selected"),
    ("text-muted", "background"),
    ("text-muted", "surface"),
    ("text-muted", "surface-selected"),
    ("accent", "surface"),
    ("success", "surface"),
    ("failure", "surface"),
    ("failure", "surface-selected"),
    ("caution", "surface"),
    ("text", "warning-surface"),
    ("text-muted", "warning-surface"),
    ("tooltip-text", "tooltip-background"),
];

/// WCAG 2.2 AA minimum for the parts of a control that mark its state (1.4.11), such as a
/// focus ring.
const MINIMUM_NON_TEXT_RATIO: f64 = 3.0;

/// State marks drawn on the surfaces controls sit on (A11Y-002).
const NON_TEXT_PAIRS: &[(&str, &str)] = &[
    ("ring", "background"),
    ("ring", "surface"),
    ("ring", "surface-hover"),
    ("ring", "surface-selected"),
    ("danger", "background"),
    ("danger", "surface"),
    ("danger", "surface-selected"),
];

/// token -> (light colour, dark colour), from lines of the form
/// `out property <color> name: dark ? #dark : #light;`; a line `name: other;` names another token.
fn theme_colours() -> HashMap<String, ([u8; 3], [u8; 3])> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("ui")
        .join("theme.slint");
    let source = fs::read_to_string(path).expect("theme.slint is readable");
    let mut colours = HashMap::new();
    let mut aliases = Vec::new();
    for line in source.lines() {
        let Some(rest) = line.trim().strip_prefix("out property <color> ") else {
            continue;
        };
        let Some((name, value)) = rest.split_once(':') else {
            continue;
        };
        let hexes: Vec<[u8; 3]> = value
            .split('#')
            .skip(1)
            .map(|hex| parse_hex(&hex[..6]))
            .collect();
        if let [dark, light] = hexes[..] {
            colours.insert(name.trim().to_owned(), (light, dark));
        } else if hexes.is_empty() {
            let target = value.trim().trim_end_matches(';').trim().to_owned();
            aliases.push((name.trim().to_owned(), target));
        }
    }
    for (name, target) in aliases {
        let colour = colours
            .get(&target)
            .unwrap_or_else(|| panic!("{name} names {target}, which is not a colour token"));
        colours.insert(name, *colour);
    }
    colours
}

/// Every pair in `pairs` below `minimum` in either theme, in words.
fn failing_pairs(pairs: &[(&str, &str)], minimum: f64) -> Vec<String> {
    let colours = theme_colours();
    let mut failures = Vec::new();
    for (front, back) in pairs {
        let (front_light, front_dark) = colours[*front];
        let (back_light, back_dark) = colours[*back];
        for (theme, a, b) in [
            ("light", front_light, back_light),
            ("dark", front_dark, back_dark),
        ] {
            let ratio = contrast(a, b);
            if ratio < minimum {
                failures.push(format!("{theme}: {front} on {back} is {ratio:.2}:1"));
            }
        }
    }
    failures
}

fn parse_hex(hex: &str) -> [u8; 3] {
    let channel = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16).expect("hex colour");
    [channel(0), channel(2), channel(4)]
}

/// WCAG relative luminance.
fn luminance(colour: [u8; 3]) -> f64 {
    let linear = |channel: u8| {
        let value = f64::from(channel) / 255.0;
        if value <= 0.039_28 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(colour[0]) + 0.7152 * linear(colour[1]) + 0.0722 * linear(colour[2])
}

/// WCAG contrast ratio.
fn contrast(a: [u8; 3], b: [u8; 3]) -> f64 {
    let (lighter, darker) = {
        let (la, lb) = (luminance(a), luminance(b));
        if la > lb { (la, lb) } else { (lb, la) }
    };
    (lighter + 0.05) / (darker + 0.05)
}

#[test]
fn contrast_formula_matches_known_values() {
    assert!((contrast([0, 0, 0], [255, 255, 255]) - 21.0).abs() < 0.01);
    assert!((contrast([119, 119, 119], [255, 255, 255]) - 4.48).abs() < 0.01);
}

#[test]
fn every_text_pair_reaches_aa_in_both_themes() {
    let failures = failing_pairs(PAIRS, MINIMUM_RATIO);
    assert!(
        failures.is_empty(),
        "below {MINIMUM_RATIO}:1:\n{}",
        failures.join("\n")
    );
}

// A11Y-002: the focus ring and the disabled ring stand out from every surface a control sits on.
#[test]
fn every_state_mark_reaches_aa_in_both_themes() {
    let failures = failing_pairs(NON_TEXT_PAIRS, MINIMUM_NON_TEXT_RATIO);
    assert!(
        failures.is_empty(),
        "below {MINIMUM_NON_TEXT_RATIO}:1:\n{}",
        failures.join("\n")
    );
}
