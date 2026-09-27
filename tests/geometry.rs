//! Where things land on screen, measured headless from the laid-out element tree: a tooltip stays
//! inside the window and a button's icon sits centred on its words. What the pixels LOOK like is
//! not measured here; only the geometry Slint computed.

use std::collections::HashSet;

use buildpilot::ui::{CreditData, MainWindow, RowData, step_ring};
use i_slint_backend_testing::{ElementHandle, ElementQuery};
use slint::platform::WindowEvent;
use slint::{ComponentHandle, LogicalPosition, LogicalSize, ModelRc, VecModel};

/// The window size the tests lay out at: the smallest the window allows, the tightest case.
const WIDTH: f32 = 760.0;
const HEIGHT: f32 = 480.0;
/// Half a logical pixel: rounding in layout never moves anything further than this.
const TOLERANCE: f32 = 0.5;
/// The toolbar, left to right.
const TOOLBAR: [&str; 4] = [
    "Add a build script",
    "Switch to dark theme",
    "Settings",
    "Help",
];

fn window() -> MainWindow {
    i_slint_backend_testing::init_no_event_loop();
    let window = MainWindow::new().unwrap();
    let weak = window.as_weak();
    window.on_step_ring(move |forward| step_ring(weak.upgrade().unwrap().window(), forward));
    window.window().set_size(LogicalSize::new(WIDTH, HEIGHT));
    window.show().unwrap();
    settle();
    window
}

fn settle() {
    slint::platform::update_timers_and_animations();
}

/// The one `kind` of control labelled `label`; its inner focus stop shares the label.
fn only(window: &MainWindow, kind: &str, label: &str) -> ElementHandle {
    let wanted = label.to_owned();
    let found = ElementQuery::from_root(window)
        .match_descendants()
        .match_type_name(kind)
        .match_predicate(move |element| element.accessible_label().as_deref() == Some(&wanted))
        .find_all();
    // A component is reported once per element inlined into it, all at one place; more than one
    // place means more than one control.
    let places: HashSet<String> = found
        .iter()
        .map(|e| format!("{:?} {:?}", e.absolute_position(), e.size()))
        .collect();
    assert_eq!(
        places.len(),
        1,
        "expected one {kind} labelled {label}: {places:?}"
    );
    found.into_iter().next().unwrap()
}

fn hover(window: &MainWindow, element: &ElementHandle) {
    let (at, size) = (element.absolute_position(), element.size());
    let position = LogicalPosition::new(at.x + size.width / 2.0, at.y + size.height / 2.0);
    window
        .window()
        .dispatch_event(WindowEvent::PointerMoved { position });
    settle();
}

// A tooltip is never cut off by the window's edge, whichever end of the toolbar its control is.
#[test]
fn every_toolbar_tooltip_stays_inside_the_window() {
    let window = window();
    for label in TOOLBAR {
        hover(&window, &only(&window, "IconButton", label));
        let tips: Vec<_> = ElementHandle::find_by_element_type_name(&window, "Tooltip").collect();
        assert_eq!(tips.len(), 1, "hovering {label} shows one tooltip");
        let (at, size) = (tips[0].absolute_position(), tips[0].size());
        assert!(
            at.x >= -TOLERANCE && at.x + size.width <= WIDTH + TOLERANCE,
            "{label}'s tooltip spans x {} to {} in a window {WIDTH} wide",
            at.x,
            at.x + size.width
        );
    }
}

// A row's tooltips are drawn by the window, never inside the rows' scrolling view, which clips
// what it holds (the first row's tooltips were cut off there); each stays inside the window.
#[test]
fn row_tooltips_are_not_clipped_by_the_list() {
    let window = window();
    let row = RowData {
        id: "a".into(),
        name: "Alpha".into(),
        selected: true,
        can_run: true,
        can_remove: true,
        ..Default::default()
    };
    window.set_rows(ModelRc::new(VecModel::from(vec![row])));
    window.set_selected_index(0);
    settle();
    let list = ElementHandle::find_by_element_id(&window, "RowsList::rows-view")
        .next()
        .expect("the rows' scrolling view");
    let buttons: Vec<ElementHandle> = list
        .query_descendants()
        .match_type_name("IconButton")
        .find_all();
    assert!(!buttons.is_empty(), "the row has buttons");
    for button in &buttons {
        hover(&window, button);
        let tips: Vec<_> = ElementHandle::find_by_element_type_name(&window, "Tooltip").collect();
        let label = button.accessible_label().unwrap_or_default();
        assert_eq!(tips.len(), 1, "hovering {label} shows one tooltip");
        let (at, size) = (tips[0].absolute_position(), tips[0].size());
        assert!(
            at.x >= -TOLERANCE
                && at.y >= -TOLERANCE
                && at.x + size.width <= WIDTH + TOLERANCE
                && at.y + size.height <= HEIGHT + TOLERANCE,
            "{label}'s tooltip at {at:?} size {size:?} leaves the window"
        );
        let inside = list
            .query_descendants()
            .match_type_name("Tooltip")
            .find_all();
        assert!(
            inside.is_empty(),
            "{label}'s tooltip is inside the clipping list"
        );
    }
}

/// The rows' view and scroll bar once `window` shows `count` rows; no bar while it is hidden.
fn rows_scrolling(window: &MainWindow, count: usize) -> (ElementHandle, Option<ElementHandle>) {
    let rows: Vec<RowData> = (0..count)
        .map(|index| RowData {
            id: index.to_string().into(),
            name: format!("Project {index}").into(),
            ..Default::default()
        })
        .collect();
    window.set_rows(ModelRc::new(VecModel::from(rows)));
    settle();
    let view = ElementHandle::find_by_element_id(window, "RowsList::rows-view")
        .next()
        .expect("the rows' scrolling view");
    // Found by type, since a lookup by id does not reach a component's own root element; the
    // search passes over a hidden element.
    let bar = ElementHandle::find_by_element_type_name(window, "ScrollBar")
        .find(|element| element.id().as_deref() == Some("RowsList::rows-bar"));
    (view, bar)
}

// UI-012: rows that run past the window bring a bar a hand can find: the view gives it its own
// strip at the right edge, full height, with a thumb shorter than the track. Rows that fit do
// not: the view keeps the whole width.
#[test]
fn overflowing_rows_show_a_scroll_bar() {
    let window = window();
    let (view, bar) = rows_scrolling(&window, 20);
    let bar = bar.expect("twenty rows show the bar");
    let (view_at, view_size) = (view.absolute_position(), view.size());
    let (bar_at, bar_size) = (bar.absolute_position(), bar.size());
    assert!(bar_size.width >= 12.0 - TOLERANCE, "bar is {bar_size:?}");
    assert!(
        (view_at.x + view_size.width - bar_at.x).abs() <= TOLERANCE,
        "the view ends at {} where the bar starts at {}",
        view_at.x + view_size.width,
        bar_at.x
    );
    assert!((bar_size.height - view_size.height).abs() <= TOLERANCE);
    let thumb = bar
        .query_descendants()
        .match_id("ScrollBar::thumb")
        .find_first()
        .expect("the thumb");
    assert!(
        thumb.size().height < bar_size.height,
        "thumb {:?} in a bar {bar_size:?}",
        thumb.size()
    );

    let (view, bar) = rows_scrolling(&window, 1);
    assert!(bar.is_none(), "one row shows no bar");
    let right = view.absolute_position().x + view.size().width;
    assert!(
        (right - WIDTH).abs() <= TOLERANCE,
        "a list that fits keeps the whole width: it ends at {right}"
    );
}

/// The names of the edge cues showing, in order.
fn cues(window: &MainWindow) -> Vec<String> {
    let names: std::collections::BTreeSet<String> =
        ElementHandle::find_by_element_type_name(window, "MoreCue")
            .filter_map(|cue| cue.accessible_label().map(|label| label.to_string()))
            .collect();
    names.into_iter().collect()
}

// UI-012: rows wholly out of sight are counted at the edge they lie beyond; a click on the
// count scrolls a page that way. Twenty rows 102 px apart in a view 356 px tall show four.
#[test]
fn hidden_rows_are_counted_at_each_edge() {
    let window = window();
    rows_scrolling(&window, 20);
    assert_eq!(cues(&window), ["16 more rows below"]);
    let below = ElementHandle::find_by_element_type_name(&window, "MoreCue")
        .next()
        .expect("the cue below");
    let (at, size) = (below.absolute_position(), below.size());
    let position = LogicalPosition::new(at.x + size.width / 2.0, at.y + size.height / 2.0);
    let button = slint::platform::PointerEventButton::Left;
    let adapter = window.window();
    adapter.dispatch_event(WindowEvent::PointerPressed { position, button });
    adapter.dispatch_event(WindowEvent::PointerReleased { position, button });
    settle();
    assert_eq!(cues(&window), ["13 more rows below", "3 more rows above"]);
    rows_scrolling(&window, 2);
    assert!(cues(&window).is_empty(), "rows that fit are not counted");
}

/// The longest licence expression among the crates built in, as a crate states it.
const LONG_LICENCE: &str = "(MIT OR Apache-2.0) AND Unicode-3.0";

// UI-009: every credit's licence ends inside the reading pane, however long another row's is.
#[test]
fn every_credit_licence_is_inside_the_pane() {
    let window = window();
    let credit = |name: &str, licence: &str| CreditData {
        name: name.into(),
        version: "1.0.0".into(),
        licence: licence.into(),
    };
    let mut credits = vec![
        credit("short", "MIT"),
        credit("a-crate-with-a-long-name", LONG_LICENCE),
    ];
    // The real list too: the two rows alone did not show the fault the whole list did.
    credits.extend(
        buildpilot::domain::credits::parse(buildpilot::infrastructure::build_info::CREDITS)
            .iter()
            .map(|built_in| credit(&built_in.name, &built_in.licence)),
    );
    window.set_credits(ModelRc::new(VecModel::from(credits)));
    window.set_show_about(true);
    settle();
    let panes: Vec<_> =
        ElementHandle::find_by_element_id(&window, "AboutDialog::credits-pane").collect();
    let pane = panes.first().expect("the About pane");
    let right = pane.absolute_position().x + pane.size().width;
    let licences: Vec<_> =
        ElementHandle::find_by_element_id(&window, "AboutDialog::credit-licence").collect();
    assert!(!licences.is_empty(), "the licences are shown");
    for licence in licences {
        let end = licence.absolute_position().x + licence.size().width;
        assert!(
            end <= right + TOLERANCE,
            "a licence ends at x {end}, beyond the pane's right edge at {right}"
        );
    }
}

/// The one element with `id`.
fn by_id(window: &MainWindow, id: &str) -> ElementHandle {
    let found: Vec<_> = ElementHandle::find_by_element_id(window, id).collect();
    found
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("no element {id}"))
}

// UI-003: About's header (version to licence) stays put above the credits, which alone scroll.
#[test]
fn the_about_header_sits_above_the_scrolling_credits() {
    let window = window();
    window.set_show_about(true);
    settle();
    let header = by_id(&window, "AboutDialog::about-header");
    let pane = by_id(&window, "AboutDialog::credits-pane");
    let header_bottom = header.absolute_position().y + header.size().height;
    assert!(
        header_bottom <= pane.absolute_position().y + TOLERANCE,
        "the header ends at y {header_bottom}, below the pane's top at {}",
        pane.absolute_position().y
    );
}

// UI-011: Licence opens at the top of its text; reading starts from there.
#[test]
fn the_licence_opens_at_its_top() {
    let window = window();
    window.set_licence_text(buildpilot::infrastructure::build_info::LICENCE.into());
    window.set_show_licence(true);
    settle();
    let pane = by_id(&window, "LicenceDialog::licence-pane");
    let body = by_id(&window, "LicenceDialog::licence-body");
    let (pane_top, body_top) = (pane.absolute_position().y, body.absolute_position().y);
    assert!(
        (body_top - pane_top).abs() <= TOLERANCE,
        "the text starts at y {body_top}, the pane at {pane_top}"
    );
}

fn centre_y(element: &ElementHandle) -> f32 {
    element.absolute_position().y + element.size().height / 2.0
}

fn centre_x(element: &ElementHandle) -> f32 {
    element.absolute_position().x + element.size().width / 2.0
}

// A button's icon sits level with its words; the pair sits centred in the button.
#[test]
fn a_button_icon_is_centred_on_its_words() {
    let window = window();
    window.set_show_settings(true);
    settle();
    for label in ["Light theme", "Dark theme"] {
        let button = only(&window, "TextButton", label);
        let icons: Vec<_> = button
            .query_descendants()
            .match_id("TextButton::icon-image")
            .find_all();
        let words: Vec<_> = button
            .query_descendants()
            .match_id("TextButton::words")
            .find_all();
        assert_eq!(
            (icons.len(), words.len()),
            (1, 1),
            "{label} has one icon and words"
        );
        let (icon, words) = (&icons[0], &words[0]);
        assert!(
            (centre_y(icon) - centre_y(words)).abs() <= TOLERANCE,
            "{label}: icon centre y {} against words centre y {}",
            centre_y(icon),
            centre_y(words)
        );
        let left = icon.absolute_position().x;
        let right = words.absolute_position().x + words.size().width;
        let group_centre = (left + right) / 2.0;
        assert!(
            (group_centre - centre_x(&button)).abs() <= TOLERANCE,
            "{label}: icon and words centre on x {group_centre}, the button on {}",
            centre_x(&button)
        );
    }
}
