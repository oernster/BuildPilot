//! The self-reading cycle of a surface of text (UI-011): hold still on opening, read down
//! slowly, hold at the end, rewind fast, hold, repeat; step aside the moment the reader takes
//! over and carry on from where they left it. The state machine only, ported from PigeonPost's
//! `autoScroll.ts`; `ui/reading.slint` drives it on a timer against a real scroll view.
//!
//! The pace belongs to the application, never to one surface: if a surface seems to need a
//! different pace, the pace is wrong everywhere.

/// The clock. Every hold counts down in whole ticks, so a wait and a movement share granularity.
pub const TICK_MS: i32 = 40;
/// Stillness before the first descent: the reader orients before anything moves.
pub const START_HOLD_MS: i32 = 5000;
/// The descent: this many pixels every `DESCENT_TICKS_PER_STEP` ticks.
pub const DESCENT_PX: f32 = 1.0;
/// Ticks per descent step; a countdown rather than a slower timer, so holds keep `TICK_MS`.
pub const DESCENT_TICKS_PER_STEP: i32 = 2;
/// Long enough to finish reading the tail before the rewind takes it away.
pub const BOTTOM_HOLD_MS: i32 = 5000;
/// The rewind is a reposition, not a reading pass, so it travels fast.
pub const REWIND_PX: f32 = 15.0;
/// The breath before the next pass.
pub const TOP_HOLD_MS: i32 = 2000;
/// Stillness after any manual reading input before the cycle picks up again.
pub const MANUAL_HOLD_MS: i32 = 2500;

/// Where the cycle is: two movements and three holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Reading down.
    Down,
    /// Holding at the end.
    PauseBottom,
    /// Rewinding to the top.
    Up,
    /// Holding at the top; while opening, this is the start hold.
    PauseTop,
    /// Holding after the reader moved it by hand.
    Manual,
}

/// The cycle's state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct State {
    /// The phase.
    pub phase: Phase,
    /// What is left of the current hold, in milliseconds; ignored while moving.
    pub wait_ms: i32,
    /// Ticks left until the next descent step.
    pub ticks_to_step: i32,
    /// True until the start hold is spent: focus arriving while the surface opens is the
    /// dialog placing it, not a reader taking over.
    pub opening: bool,
}

/// Where a scrollable surface sits: how far down and how far it can go.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// Pixels scrolled from the top.
    pub top: f32,
    /// The furthest `top` can go; zero or less when the content fits.
    pub max_top: f32,
}

/// One tick's outcome: the next state and how far to move, already kept within bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Step {
    /// The next state.
    pub state: State,
    /// Pixels to move down; negative moves up.
    pub delta: f32,
}

impl Default for State {
    /// A fresh surface stands still for the start hold before it first moves.
    fn default() -> Self {
        Self {
            phase: Phase::PauseTop,
            wait_ms: START_HOLD_MS,
            ticks_to_step: DESCENT_TICKS_PER_STEP,
            opening: true,
        }
    }
}

fn still(state: State) -> Step {
    Step { state, delta: 0.0 }
}

impl State {
    /// The reader moved the surface by hand: hold, then carry on from their position.
    pub fn suspended(self) -> Self {
        Self {
            phase: Phase::Manual,
            wait_ms: MANUAL_HOLD_MS,
            opening: false,
            ..self
        }
    }

    /// Keyboard focus arrived in the surface. While it is still opening that is the dialog
    /// placing focus, so nothing changes; afterwards it is a reader and suspends the cycle.
    pub fn focused(self) -> Self {
        if self.opening { self } else { self.suspended() }
    }

    /// Advances one tick. Content that fits consumes nothing: no wait counts down and no phase
    /// changes, so a surface that fits today is ready for one that overflows tomorrow.
    pub fn tick(self, view: View) -> Step {
        if view.max_top <= 0.0 {
            return still(self);
        }
        match self.phase {
            Phase::Down => self.descend(view),
            Phase::Up => self.rewind(view),
            Phase::PauseBottom | Phase::PauseTop | Phase::Manual => self.hold(view),
        }
    }

    fn hold(self, view: View) -> Step {
        let wait_ms = self.wait_ms - TICK_MS;
        if wait_ms > 0 {
            return still(Self { wait_ms, ..self });
        }
        // The hold is spent; whatever opened the surface is over.
        let done = Self {
            wait_ms: 0,
            opening: false,
            ..self
        };
        let rewind = self.phase == Phase::PauseBottom
            || (self.phase == Phase::Manual && view.top >= view.max_top);
        if rewind {
            return still(Self {
                phase: Phase::Up,
                ..done
            });
        }
        still(Self {
            phase: Phase::Down,
            ticks_to_step: DESCENT_TICKS_PER_STEP,
            ..done
        })
    }

    fn descend(self, view: View) -> Step {
        let ticks_to_step = self.ticks_to_step - 1;
        if ticks_to_step > 0 {
            return still(Self {
                ticks_to_step,
                ..self
            });
        }
        let reset = Self {
            ticks_to_step: DESCENT_TICKS_PER_STEP,
            ..self
        };
        let remaining = view.max_top - view.top;
        if remaining <= DESCENT_PX {
            return Step {
                state: Self {
                    phase: Phase::PauseBottom,
                    wait_ms: BOTTOM_HOLD_MS,
                    ..reset
                },
                delta: remaining.max(0.0),
            };
        }
        Step {
            state: reset,
            delta: DESCENT_PX,
        }
    }

    fn rewind(self, view: View) -> Step {
        if view.top <= REWIND_PX {
            return Step {
                state: Self {
                    phase: Phase::PauseTop,
                    wait_ms: TOP_HOLD_MS,
                    ..self
                },
                delta: -view.top,
            };
        }
        Step {
            state: self,
            delta: -REWIND_PX,
        }
    }
}
