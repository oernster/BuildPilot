//! A release version, `major.minor.patch`, ordered the way releases are.

use std::fmt;

/// A release version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    major: u32,
    minor: u32,
    patch: u32,
}

/// How many dot-separated numbers a version has.
const PARTS: usize = 3;

impl Version {
    /// `text` as a version; `None` unless it is three numbers joined by dots.
    pub fn parse(text: &str) -> Option<Self> {
        let parts: Vec<u32> = text
            .trim()
            .split('.')
            .map(|part| part.parse().ok())
            .collect::<Option<_>>()?;
        match parts[..] {
            [major, minor, patch] if parts.len() == PARTS => Some(Self {
                major,
                minor,
                patch,
            }),
            _ => None,
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}
