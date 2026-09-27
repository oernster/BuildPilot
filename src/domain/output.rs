//! One run's output, kept for the tray (SRS 3.10).
//!
//! Lines are numbered from the start of the run. When old lines are dropped (OUT-004) the
//! numbers of the survivors do not change, so a reader that has shown lines up to number N can
//! ask for everything after N without caring what was dropped in between.

use std::collections::VecDeque;

/// Most lines kept per operation; the oldest go first (OUT-004, OQ-2).
pub const MAX_LINES: usize = 100_000;

/// Longest line kept whole; a longer one is split into pieces of this many characters (OUT-004).
pub const MAX_LINE_CHARS: usize = 16_384;

/// Which pipe a line came from (OUT-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    /// Standard output.
    Stdout,
    /// Standard error.
    Stderr,
    /// BuildPilot's own line: which step started and with what (STEP-006, ENV-008).
    Note,
}

/// One line of output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputLine {
    /// The pipe it came from.
    pub stream: Stream,
    /// The text, already decoded.
    pub text: String,
}

/// The retained output of one run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputBuffer {
    lines: VecDeque<OutputLine>,
    dropped: u64,
    capacity: usize,
}

impl Default for OutputBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputBuffer {
    /// An empty buffer holding up to `MAX_LINES`.
    pub fn new() -> Self {
        Self::with_capacity(MAX_LINES)
    }

    /// An empty buffer holding up to `capacity` lines (at least one).
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            lines: VecDeque::new(),
            dropped: 0,
            capacity: capacity.max(1),
        }
    }

    /// Appends `text`, split into pieces of at most `MAX_LINE_CHARS` characters, dropping the
    /// oldest lines to stay within capacity.
    pub fn push(&mut self, stream: Stream, text: &str) {
        for piece in split_long_line(text) {
            if self.lines.len() == self.capacity {
                self.lines.pop_front();
                self.dropped += 1;
            }
            self.lines.push_back(OutputLine {
                stream,
                text: piece.to_owned(),
            });
        }
    }

    /// How many lines are held.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// True when nothing is held.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// How many lines have been dropped; also the number of the oldest line held.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// The number the next line pushed will get.
    pub fn next_line_number(&self) -> u64 {
        self.dropped + self.lines.len() as u64
    }

    /// The line at `index` among those held (0 is the oldest held), for a view that shows
    /// one row per line.
    pub fn get(&self, index: usize) -> Option<&OutputLine> {
        self.lines.get(index)
    }

    /// Every line held, oldest first.
    pub fn lines(&self) -> impl Iterator<Item = &OutputLine> {
        self.lines.iter()
    }

    /// The lines numbered `number` onwards that are still held.
    pub fn lines_from(&self, number: u64) -> impl Iterator<Item = &OutputLine> {
        let skip = number.saturating_sub(self.dropped);
        let skip = usize::try_from(skip).unwrap_or(usize::MAX);
        self.lines.iter().skip(skip)
    }
}

/// `text` in pieces of at most `MAX_LINE_CHARS` characters, split on character boundaries.
fn split_long_line(text: &str) -> Vec<&str> {
    let mut pieces = Vec::new();
    let mut start = 0;
    for (count, (index, _)) in text.char_indices().enumerate() {
        if count > 0 && count % MAX_LINE_CHARS == 0 {
            pieces.push(&text[start..index]);
            start = index;
        }
    }
    pieces.push(&text[start..]);
    pieces
}
