//! Every direct Windows API call BuildPilot makes. The only place in the crate that may use
//! `unsafe`, which `tests/structural.rs` enforces; each block carries its SAFETY reasoning.

pub mod codepage;
pub mod instance;
pub mod job;
pub mod shell;
pub mod theme;
