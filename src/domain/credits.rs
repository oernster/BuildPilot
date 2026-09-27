//! The open source credits (UI-009): one line per crate built into BuildPilot, as the build
//! script writes them from `cargo metadata`. Nothing here is written by hand.

/// Between one crate's fields on a line.
pub const FIELD_SEPARATOR: char = '\t';

/// A crate built into BuildPilot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credit {
    /// The crate's name.
    pub name: String,
    /// Its version.
    pub version: String,
    /// Its licence expression, as the crate states it; empty when it states none.
    pub licence: String,
}

/// The credits in `text`, one per line of name, version and licence. A line without all three
/// fields is skipped rather than guessed at.
pub fn parse(text: &str) -> Vec<Credit> {
    text.lines()
        .filter_map(|line| {
            let mut fields = line.splitn(3, FIELD_SEPARATOR);
            match (fields.next(), fields.next(), fields.next()) {
                (Some(name), Some(version), Some(licence)) if !name.is_empty() => Some(Credit {
                    name: name.to_owned(),
                    version: version.to_owned(),
                    licence: licence.to_owned(),
                }),
                _ => None,
            }
        })
        .collect()
}
