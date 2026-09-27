//! Help (UI-006 to UI-010): what the menu's entries open and the update check behind Check for
//! Updates, which also runs 3 seconds after the window opens and then once a day.

use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use slint::{ModelRc, SharedString, Timer, TimerMode, VecModel};

use crate::application::ReleaseSource;
use crate::application::updates::{self, Asked, UpdateOutcome};
use crate::domain::credits::{Credit, readable_licence};
use crate::domain::version::Version;

use super::events::{self, Hook};
use super::{CreditData, MainWindow, Ui};

/// How long after the window opens the first check waits, so it never competes with starting.
const FIRST_CHECK_AFTER: Duration = Duration::from_secs(3);
/// Seconds in the day between later checks.
const SECONDS_PER_DAY: u64 = 24 * 60 * 60;

/// What Help shows, gathered by the composition root.
pub struct HelpFacts {
    /// One sentence on what BuildPilot is.
    pub description: &'static str,
    /// Who wrote it.
    pub author: &'static str,
    /// The copyright line.
    pub copyright: &'static str,
    /// The repository's address.
    pub repository: &'static str,
    /// Every crate built in (UI-009).
    pub credits: Vec<Credit>,
    /// BuildPilot's licence text.
    pub licence: &'static str,
    /// Where newer releases are learned (UI-010).
    pub releases: Arc<dyn ReleaseSource>,
}

/// A newer release on offer, kept while its prompt is open.
struct Offer {
    latest: Version,
    download_url: String,
}

/// Help's state on the UI thread.
struct Help {
    results: Receiver<(UpdateOutcome, Asked)>,
    sender: Sender<(UpdateOutcome, Asked)>,
    offer: RefCell<Option<Offer>>,
}

/// Wires Help into `window`; the answer holds the check's timers, which run while it is kept.
pub fn wire(ui: &Rc<Ui>, window: &MainWindow) -> [Timer; 2] {
    let facts = &ui.environment.help;
    window.set_description(facts.description.into());
    window.set_author(facts.author.into());
    window.set_copyright(facts.copyright.into());
    window.set_repository(facts.repository.into());
    window.set_licence_text(facts.licence.into());
    let credits: Vec<CreditData> = facts
        .credits
        .iter()
        .map(|credit| CreditData {
            name: credit.name.as_str().into(),
            version: credit.version.as_str().into(),
            licence: readable_licence(&credit.licence).into(),
        })
        .collect();
    window.set_credits(ModelRc::new(VecModel::from(credits)));

    let (sender, results) = mpsc::channel();
    let help = Rc::new(Help {
        results,
        sender,
        offer: RefCell::new(None),
    });
    wire_menu(ui, &help, window);
    wire_prompt(ui, &help, window);
    let (this, state) = (ui.clone(), help.clone());
    events::install(Hook::Update, Rc::new(move || deliver(&this, &state)));

    let (this, state) = (ui.clone(), help.clone());
    let first = Timer::default();
    first.start(TimerMode::SingleShot, FIRST_CHECK_AFTER, move || {
        check(&this, &state, Asked::Automatically);
    });
    let (this, state) = (ui.clone(), help);
    let daily = Timer::default();
    daily.start(
        TimerMode::Repeated,
        Duration::from_secs(SECONDS_PER_DAY),
        move || check(&this, &state, Asked::Automatically),
    );
    [first, daily]
}

fn wire_menu(ui: &Rc<Ui>, help: &Rc<Help>, window: &MainWindow) {
    let (this, state) = (ui.clone(), help.clone());
    window.on_help(move |entry| match entry.as_str() {
        "guide" => this.with_window(|w| w.set_show_guide(true)),
        "about" => this.with_window(|w| w.set_show_about(true)),
        "licence" => this.with_window(|w| w.set_show_licence(true)),
        "updates" => check(&this, &state, Asked::ByOperator),
        _ => {}
    });
    let this = ui.clone();
    window.on_close_help(move || {
        this.with_window(|w| {
            w.set_show_guide(false);
            w.set_show_about(false);
            w.set_show_licence(false);
        });
    });
    let this = ui.clone();
    window.on_open_repository(move || {
        let opened = this
            .app
            .borrow()
            .open_address(this.environment.help.repository);
        this.report(opened);
    });
}

fn wire_prompt(ui: &Rc<Ui>, help: &Rc<Help>, window: &MainWindow) {
    let (this, state) = (ui.clone(), help.clone());
    window.on_download_update(move || {
        let offer = state.offer.borrow_mut().take();
        this.with_window(|w| w.set_show_update(false));
        if let Some(offer) = offer {
            let opened = this.app.borrow().open_address(&offer.download_url);
            this.report(opened);
        }
    });
    let (this, state) = (ui.clone(), help.clone());
    window.on_skip_update(move || {
        let offer = state.offer.borrow_mut().take();
        this.with_window(|w| w.set_show_update(false));
        if let Some(offer) = offer {
            this.app.borrow_mut().skip_update(offer.latest);
            this.refresh();
        }
    });
    let (this, state) = (ui.clone(), help.clone());
    window.on_close_update(move || {
        state.offer.borrow_mut().take();
        this.with_window(|w| w.set_show_update(false));
    });
}

fn current(ui: &Ui) -> Option<Version> {
    Version::parse(ui.environment.version)
}

/// Asks on a worker thread, so the window never waits on the network; the answer comes back
/// through the Update hook. A panic on the worker still answers, as unreachable.
fn check(ui: &Rc<Ui>, help: &Rc<Help>, asked: Asked) {
    let Some(current) = current(ui) else {
        return;
    };
    let skipped = ui.app.borrow().preferences().skipped_update;
    let source = ui.environment.help.releases.clone();
    let sender = help.sender.clone();
    thread::spawn(move || {
        let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
            updates::check(source.as_ref(), current, skipped, asked)
        }))
        .unwrap_or(UpdateOutcome::Unreachable);
        if sender.send((outcome, asked)).is_ok() {
            events::schedule(Hook::Update);
        }
    });
}

fn deliver(ui: &Rc<Ui>, help: &Rc<Help>) {
    let Some(current) = current(ui) else {
        return;
    };
    while let Ok((outcome, asked)) = help.results.try_recv() {
        let Some(message) = updates::message(&outcome, current, asked) else {
            continue;
        };
        let offer = match outcome {
            UpdateOutcome::Available {
                latest,
                download_url,
            } => Some(Offer {
                latest,
                download_url,
            }),
            UpdateOutcome::UpToDate | UpdateOutcome::Unreachable => None,
        };
        let offered = offer.is_some();
        *help.offer.borrow_mut() = offer;
        ui.with_window(|w| {
            w.set_update_message(SharedString::from(message));
            w.set_update_offer(offered);
            w.set_show_update(true);
        });
    }
}
