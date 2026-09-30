//! The domain: BuildPilot's nouns and rules, with no I/O, no clock and no framework.
//!
//! Everything here is plain data plus pure functions over it. It may use a small allow-listed
//! part of the standard library (see `tests/structural.rs`) and nothing outside it: no file
//! system, no processes, no threads, no reading of the time.

#![forbid(unsafe_code)]

pub mod auto_scroll;
pub mod credits;
pub mod deck;
pub mod elapsed;
pub mod environment;
pub mod follow;
pub mod host;
pub mod installer;
pub mod launch_plan;
pub mod lifecycle;
pub mod line_assembler;
pub mod operation;
pub mod output;
pub mod preferences;
pub mod run_times;
pub mod scan;
pub mod selection;
pub mod step;
pub mod text;
pub mod version;
