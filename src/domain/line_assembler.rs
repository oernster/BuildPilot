//! Cutting a stream of output bytes into lines. Bytes arrive in chunks that ignore line breaks,
//! so a partial line is held until its `\n` arrives.
//!
//! The held partial line is capped. The size of a script's output is foreign input: a script
//! that writes megabytes without a newline must not make BuildPilot hold megabytes waiting.

use super::output::MAX_LINE_CHARS;
use super::text::decode_line;

/// The most bytes one UTF-8 character can take.
const UTF8_MAX_CHAR_BYTES: usize = 4;

/// Most bytes held while waiting for a newline: enough for a whole `MAX_LINE_CHARS` line.
pub const MAX_PENDING_BYTES: usize = MAX_LINE_CHARS * UTF8_MAX_CHAR_BYTES;

/// A UTF-8 continuation byte matches `CONTINUATION_PATTERN` under `CONTINUATION_MASK`.
const CONTINUATION_MASK: u8 = 0b1100_0000;
const CONTINUATION_PATTERN: u8 = 0b1000_0000;

/// Assembles lines from one pipe's chunks.
#[derive(Debug, Clone, Default)]
pub struct LineAssembler {
    pending: Vec<u8>,
}

impl LineAssembler {
    /// An assembler holding nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes the next chunk and answers every line it completes, decoded. A partial line longer
    /// than `MAX_PENDING_BYTES` is released in pieces rather than held.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        let mut lines = Vec::new();
        let mut rest = chunk;
        while let Some(newline) = rest.iter().position(|&byte| byte == b'\n') {
            self.pending.extend_from_slice(&rest[..newline]);
            lines.push(decode_line(&self.pending));
            self.pending.clear();
            rest = &rest[newline + 1..];
        }
        self.pending.extend_from_slice(rest);
        while self.pending.len() > MAX_PENDING_BYTES {
            let cut = char_boundary_at_or_before(&self.pending, MAX_PENDING_BYTES);
            let head: Vec<u8> = self.pending.drain(..cut).collect();
            lines.push(decode_line(&head));
        }
        lines
    }

    /// The pipe has closed: answers the final line if it had no `\n`.
    pub fn finish(&mut self) -> Option<String> {
        if self.pending.is_empty() {
            return None;
        }
        let last = decode_line(&self.pending);
        self.pending.clear();
        Some(last)
    }
}

/// The largest index at or before `limit` that does not split a UTF-8 character; `limit` itself
/// when the bytes before it are not valid UTF-8 at all.
fn char_boundary_at_or_before(bytes: &[u8], limit: usize) -> usize {
    let mut cut = limit;
    while cut > 0 && bytes[cut] & CONTINUATION_MASK == CONTINUATION_PATTERN {
        cut -= 1;
    }
    if cut == 0 { limit } else { cut }
}
