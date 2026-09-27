use buildpilot::domain::auto_scroll::{
    BOTTOM_HOLD_MS, DESCENT_TICKS_PER_STEP, MANUAL_HOLD_MS, Phase, REWIND_PX, START_HOLD_MS, State,
    TICK_MS, TOP_HOLD_MS, View,
};

/// A surface taller than its window by this much.
const OVERFLOW: f32 = 100.0;

/// The ticks a hold of `ms` lasts: whole ticks, so a hold that is not a multiple of the tick
/// (the manual hold's 2500 ms is 62.5) takes the next one too.
fn ticks(ms: i32) -> i32 {
    (ms + TICK_MS - 1) / TICK_MS
}

/// Runs `count` ticks against a surface that follows each step, answering the state and where
/// the surface ended up.
fn run(mut state: State, mut top: f32, max_top: f32, count: i32) -> (State, f32) {
    for _ in 0..count {
        let step = state.tick(View { top, max_top });
        top = (top + step.delta).clamp(0.0, max_top.max(0.0));
        state = step.state;
    }
    (state, top)
}

// UI-011: still for the whole start hold, then reading down begins.
#[test]
fn it_holds_still_on_opening() {
    let (state, top) = run(State::default(), 0.0, OVERFLOW, ticks(START_HOLD_MS) - 1);
    assert_eq!((state.phase, top), (Phase::PauseTop, 0.0));
    let (state, top) = run(state, top, OVERFLOW, 1);
    assert_eq!((state.phase, top), (Phase::Down, 0.0));
    assert!(!state.opening, "the start hold is spent");
}

// UI-011: one pixel every second tick.
#[test]
fn it_reads_down_one_pixel_per_step() {
    let (state, _) = run(State::default(), 0.0, OVERFLOW, ticks(START_HOLD_MS));
    let (_, top) = run(state, 0.0, OVERFLOW, DESCENT_TICKS_PER_STEP * 10);
    assert_eq!(top, 10.0);
}

// UI-011: at the end it holds, rewinds fast, holds at the top, then reads again.
#[test]
fn it_holds_rewinds_and_repeats() {
    let down = State {
        phase: Phase::Down,
        wait_ms: 0,
        ticks_to_step: 1,
        opening: false,
    };
    let (state, top) = run(down, OVERFLOW - 0.5, OVERFLOW, 1);
    assert_eq!(
        (state.phase, state.wait_ms, top),
        (Phase::PauseBottom, BOTTOM_HOLD_MS, OVERFLOW)
    );
    let (state, top) = run(state, top, OVERFLOW, ticks(BOTTOM_HOLD_MS));
    assert_eq!((state.phase, top), (Phase::Up, OVERFLOW));
    let (state, top) = run(state, top, OVERFLOW, 1);
    assert_eq!(top, OVERFLOW - REWIND_PX, "the rewind is fast");
    let rewind_ticks = (OVERFLOW / REWIND_PX).ceil() as i32;
    let (state, top) = run(state, top, OVERFLOW, rewind_ticks - 1);
    assert_eq!(
        (state.phase, state.wait_ms, top),
        (Phase::PauseTop, TOP_HOLD_MS, 0.0)
    );
    let (state, _) = run(state, top, OVERFLOW, ticks(TOP_HOLD_MS));
    assert_eq!(state.phase, Phase::Down);
}

// UI-011: a hand on the surface holds it, then the cycle carries on from there.
#[test]
fn a_manual_scroll_suspends_and_resumes_in_place() {
    let reading = State {
        phase: Phase::Down,
        wait_ms: 0,
        ticks_to_step: DESCENT_TICKS_PER_STEP,
        opening: false,
    };
    let held = reading.suspended();
    assert_eq!((held.phase, held.wait_ms), (Phase::Manual, MANUAL_HOLD_MS));
    let (state, top) = run(held, 40.0, OVERFLOW, ticks(MANUAL_HOLD_MS) - 1);
    assert_eq!((state.phase, top), (Phase::Manual, 40.0));
    let (state, top) = run(state, top, OVERFLOW, 1 + DESCENT_TICKS_PER_STEP);
    assert_eq!(
        (state.phase, top),
        (Phase::Down, 41.0),
        "it reads on from where it was left"
    );
}

// UI-011: a reader who scrolled to the very end has only the rewind left.
#[test]
fn a_manual_hold_at_the_end_rewinds() {
    let (state, _) = run(
        State::default().suspended(),
        OVERFLOW,
        OVERFLOW,
        ticks(MANUAL_HOLD_MS),
    );
    assert_eq!(state.phase, Phase::Up);
}

// UI-011: the dialog placing focus as it opens is not a reader taking over.
#[test]
fn opening_focus_is_not_a_reader() {
    let opening = State::default();
    assert_eq!(opening.focused(), opening);
    let (open, _) = run(opening, 0.0, OVERFLOW, ticks(START_HOLD_MS));
    assert_eq!(open.focused().phase, Phase::Manual);
}

// UI-011: a hand during the start hold ends the opening too.
#[test]
fn a_scroll_while_opening_ends_the_opening() {
    assert!(!State::default().suspended().opening);
}

// UI-011: content that fits consumes nothing, so no wait runs down while there is nothing to read.
#[test]
fn content_that_fits_stands_still() {
    let (state, top) = run(State::default(), 0.0, 0.0, ticks(START_HOLD_MS) * 2);
    assert_eq!((state, top), (State::default(), 0.0));
}
