//! BuildPilot: a build flight deck for Windows.
//!
//! The crate is layered `ui -> application -> domain <- infrastructure` (SRS CON-003). The
//! layering is enforced by `tests/structural.rs`, not by convention. The setup program's policy
//! in `setup` is pure in the same way the domain is.

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod setup;
pub mod ui;
