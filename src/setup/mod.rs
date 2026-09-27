//! The setup program's policy (INST-001 to INST-006): which route a run takes, what it will do
//! and everything it says. Pure: no I/O and no framework, so every state is a test. The work
//! itself is in `infrastructure::setup`; the window in `ui/setup.slint`.

pub mod plan;
pub mod route;
pub mod wording;
