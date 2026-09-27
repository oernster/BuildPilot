//! Getting run events from the launcher's threads onto the UI thread without flooding it.
//!
//! The first event after a drain schedules one drain on the event loop; events arriving before
//! it runs ride along. Measured in spike R-2: 50,000 lines in about 0.75 s produced roughly 4,000
//! drains, none longer than 3.6 ms.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

thread_local! {
    /// The UI thread's drain, installed by `install_drain`.
    static DRAIN: RefCell<Option<Rc<dyn Fn()>>> = RefCell::new(None);
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
                // Fails only once the event loop has ended, when nothing is left to wake.
                let _ = slint::invoke_from_event_loop(run_drain);
            }
        })
    }

    /// Called at the start of a drain, so an event arriving during it schedules another.
    pub fn clear(&self) {
        self.pending.store(false, Ordering::Release);
    }
}

/// Installs the function that drains events on this (the UI) thread.
pub fn install_drain(drain: Rc<dyn Fn()>) {
    DRAIN.with(|slot| *slot.borrow_mut() = Some(drain));
}

fn run_drain() {
    let drain = DRAIN.with(|slot| slot.borrow().clone());
    if let Some(drain) = drain {
        drain();
    }
}
