//! Getting work from other threads onto the UI thread: run events from the launcher's threads
//! (without flooding it), a later BuildPilot's summons (DATA-001) and the update check's answer
//! (UI-010).
//!
//! The first event after a drain schedules one drain on the event loop; events arriving before
//! it runs ride along. Measured in spike R-2: 50,000 lines in about 0.75 s produced roughly 4,000
//! drains, none longer than 3.6 ms.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Work another thread can ask the UI thread to do.
#[derive(Clone, Copy)]
pub enum Hook {
    /// Drain the waiting run events.
    Drain,
    /// Bring the window forward.
    Summon,
    /// Show what an update check found (UI-010).
    Update,
}

/// How many hooks there are.
const HOOK_COUNT: usize = 3;

/// What one hook does, once installed.
type Act = Option<Rc<dyn Fn()>>;

thread_local! {
    /// The UI thread's hooks, installed by `install`.
    static HOOKS: RefCell<[Act; HOOK_COUNT]> = RefCell::new([None, None, None]);
}

/// Wakes the UI thread when events are waiting.
#[derive(Clone, Default)]
pub struct Waker {
    pending: Arc<AtomicBool>,
}

impl Waker {
    /// A waker with nothing pending.
    pub fn new() -> Self {
        Self::default()
    }

    /// The callback the launcher calls after each event it sends.
    pub fn callback(&self) -> Arc<dyn Fn() + Send + Sync> {
        let pending = self.pending.clone();
        Arc::new(move || {
            if !pending.swap(true, Ordering::AcqRel) {
                schedule(Hook::Drain);
            }
        })
    }

    /// Called at the start of a drain, so an event arriving during it schedules another.
    pub fn clear(&self) {
        self.pending.store(false, Ordering::Release);
    }
}

/// Runs `hook` on the UI thread. Fails only once the event loop has ended, when nothing is left
/// to do it for.
pub fn schedule(hook: Hook) {
    let _ = slint::invoke_from_event_loop(move || run(hook));
}

/// Installs what `hook` does on this (the UI) thread.
pub fn install(hook: Hook, act: Rc<dyn Fn()>) {
    HOOKS.with(|hooks| hooks.borrow_mut()[hook as usize] = Some(act));
}

fn run(hook: Hook) {
    let act = HOOKS.with(|hooks| hooks.borrow()[hook as usize].clone());
    if let Some(act) = act {
        act();
    }
}
