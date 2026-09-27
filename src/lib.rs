//! BuildPilot: a build flight deck for Windows.
//!
//! The crate is layered `ui -> application -> domain <- infrastructure` (SRS CON-003). The
//! layering is enforced by `tests/structural.rs`, not by convention.

pub mod application;
pub mod domain;
pub mod infrastructure;
