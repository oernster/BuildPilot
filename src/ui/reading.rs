//! Answers the `Reading` global in ui/reading.slint with the self-reading cycle of
//! `domain::auto_scroll` (UI-011). Every window with a reading surface wires it once.

use crate::domain::auto_scroll::{Phase, State, TICK_MS, View};

use super::{Reading, ReadingState, ReadingStep};

/// The phases in the order the window's integer stands for them.
const PHASES: [Phase; 5] = [
    Phase::Down,
    Phase::PauseBottom,
    Phase::Up,
    Phase::PauseTop,
    Phase::Manual,
];

fn to_domain(state: &ReadingState) -> State {
    let phase = usize::try_from(state.phase)
        .ok()
        .and_then(|index| PHASES.get(index).copied());
    match phase {
        Some(phase) => State {
            phase,
            wait_ms: state.wait_ms,
            ticks_to_step: state.ticks_to_step,
            opening: state.opening,
        },
        // A state the window never had from here: start again rather than guess.
        None => State::default(),
    }
}

fn to_window(state: State) -> ReadingState {
    let phase = PHASES
        .iter()
        .position(|p| *p == state.phase)
        .unwrap_or_default();
    ReadingState {
        phase: i32::try_from(phase).unwrap_or_default(),
        wait_ms: state.wait_ms,
        ticks_to_step: state.ticks_to_step,
        opening: state.opening,
    }
}

/// Answers `reading`'s callbacks and starts its clock.
pub fn wire_reading(reading: &Reading<'_>) {
    reading.on_start(|| to_window(State::default()));
    reading.on_tick(|state, top, max_top| {
        let step = to_domain(&state).tick(View { top, max_top });
        ReadingStep {
            state: to_window(step.state),
            delta: step.delta,
        }
    });
    reading.on_suspend(|state| to_window(to_domain(&state).suspended()));
    reading.on_focus(|state| to_window(to_domain(&state).focused()));
    reading.set_tick_ms(TICK_MS);
}
