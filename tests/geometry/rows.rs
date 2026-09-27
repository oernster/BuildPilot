//! The rows' scrolling view: its tooltips, its scroll bar, the counts of rows out of sight and the
//! selected row kept in view.

use buildpilot::ui::{MainWindow, RowData};
use i_slint_backend_testing::ElementHandle;
use slint::platform::WindowEvent;
use slint::{ComponentHandle, LogicalPosition, ModelRc, VecModel};

use crate::{HEIGHT, TOLERANCE, WIDTH, hover, settle, window};

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

/// The row for `Project {index}`, found by its own Select control; `None` when the view has
/// clipped it out of sight, since an element query passes over what is not shown.
fn row_named(window: &MainWindow, index: usize) -> Option<ElementHandle> {
    let wanted = format!("Select Project {index}");
    ElementHandle::find_by_element_type_name(window, "OperationRow").find(|row| {
        let wanted = wanted.clone();
        row.query_descendants()
            .match_predicate(move |element| element.accessible_label().as_deref() == Some(&wanted))
            .find_first()
            .is_some()
    })
}

// ROW-009: expanding the tray shrinks the rows' view; the selected row stays wholly in sight
// rather than being left below the new foot of the view.
#[test]
fn expanding_the_tray_keeps_the_selected_row_in_view() {
    let window = window();
    let (view, _) = rows_scrolling(&window, 20);
    let selected = 2;
    window.set_selected_index(selected as i32);
    settle();
    let in_view = |view: &ElementHandle, row: &ElementHandle| {
        let (top, bottom) = (
            view.absolute_position().y,
            view.absolute_position().y + view.size().height,
        );
        let (from, to) = (
            row.absolute_position().y,
            row.absolute_position().y + row.size().height,
        );
        from >= top - TOLERANCE && to <= bottom + TOLERANCE
    };
    let before = row_named(&window, selected).expect("the row is drawn before the tray opens");
    assert!(in_view(&view, &before), "in sight before the tray opens");

    window.set_tray_expanded(true);
    settle();
    let row = row_named(&window, selected)
        .unwrap_or_else(|| panic!("row {selected} was left out of sight when the tray opened"));
    assert!(
        in_view(&view, &row),
        "row {selected} spans y {} to {} in a view from {} to {}",
        row.absolute_position().y,
        row.absolute_position().y + row.size().height,
        view.absolute_position().y,
        view.absolute_position().y + view.size().height
    );
}
