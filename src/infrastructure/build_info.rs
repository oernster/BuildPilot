//! What the build script generated for this build, carried inside the program: the open source
//! credits (UI-009) and the third-party notices (UI-008). See `build_credits.rs`, which names
//! both files and hands their paths here.

/// One line per crate built in: name, version, licence; read with `domain::credits::parse`.
pub const CREDITS: &str = include_str!(env!("BUILDPILOT_CREDITS"));

/// The licence text of every crate built in.
pub const NOTICES: &str = include_str!(env!("BUILDPILOT_NOTICES"));

/// The name the notices file goes by beside the program.
pub const NOTICES_FILE: &str = env!("BUILDPILOT_NOTICES_FILE");

/// BuildPilot's own licence, GPL-3.0: the program's Licence and setup's licence page.
pub const LICENCE: &str = include_str!("../../LICENSE");
