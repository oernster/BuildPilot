use buildpilot::domain::output::{MAX_LINE_CHARS, MAX_LINES, OutputBuffer, Stream};

fn texts<'a>(
    lines: impl Iterator<Item = &'a buildpilot::domain::output::OutputLine>,
) -> Vec<&'a str> {
    lines.map(|line| line.text.as_str()).collect()
}

#[test]
fn new_buffer_is_empty_with_the_standard_capacity() {
    let buffer = OutputBuffer::default();
    assert!(buffer.is_empty());
    assert_eq!(buffer.next_line_number(), 0);
    let mut full = OutputBuffer::new();
    for number in 0..=MAX_LINES {
        full.push(Stream::Stdout, &number.to_string());
    }
    assert_eq!(full.len(), MAX_LINES);
    assert_eq!(full.dropped(), 1);
}

// OUT-007: each line keeps its stream.
#[test]
fn lines_keep_their_stream() {
    let mut buffer = OutputBuffer::new();
    buffer.push(Stream::Stdout, "building");
    buffer.push(Stream::Stderr, "warning");
    let streams: Vec<Stream> = buffer.lines().map(|line| line.stream).collect();
    assert_eq!(streams, [Stream::Stdout, Stream::Stderr]);
    assert!(!buffer.is_empty());
}

// OUT-004: oldest dropped first; the count of drops is reported.
#[test]
fn oldest_lines_are_dropped_at_capacity() {
    let mut buffer = OutputBuffer::with_capacity(3);
    for text in ["a", "b", "c", "d", "e"] {
        buffer.push(Stream::Stdout, text);
    }
    assert_eq!(texts(buffer.lines()), ["c", "d", "e"]);
    assert_eq!(buffer.dropped(), 2);
    assert_eq!(buffer.len(), 3);
    assert_eq!(buffer.next_line_number(), 5);
}

// Line numbers survive drops, so a reader can resume after the last line it showed.
#[test]
fn lines_from_counts_from_the_start_of_the_run() {
    let mut buffer = OutputBuffer::with_capacity(3);
    for text in ["a", "b", "c", "d", "e"] {
        buffer.push(Stream::Stdout, text);
    }
    assert_eq!(texts(buffer.lines_from(3)), ["d", "e"]);
    assert_eq!(texts(buffer.lines_from(0)), ["c", "d", "e"]);
    assert!(texts(buffer.lines_from(5)).is_empty());
    assert!(texts(buffer.lines_from(u64::MAX)).is_empty());
}

#[test]
fn get_indexes_the_lines_held() {
    let mut buffer = OutputBuffer::with_capacity(2);
    for text in ["a", "b", "c"] {
        buffer.push(Stream::Stdout, text);
    }
    assert_eq!(buffer.get(0).map(|line| line.text.as_str()), Some("b"));
    assert_eq!(buffer.get(1).map(|line| line.text.as_str()), Some("c"));
    assert!(buffer.get(2).is_none());
}

#[test]
fn zero_capacity_still_holds_one_line() {
    let mut buffer = OutputBuffer::with_capacity(0);
    buffer.push(Stream::Stdout, "a");
    buffer.push(Stream::Stdout, "b");
    assert_eq!(texts(buffer.lines()), ["b"]);
}

// OUT-004: a long line is split on character boundaries.
#[test]
fn long_lines_are_split() {
    let mut buffer = OutputBuffer::new();
    let long: String = "\u{e9}".repeat(MAX_LINE_CHARS * 2 + 1);
    buffer.push(Stream::Stderr, &long);
    let lengths: Vec<usize> = buffer
        .lines()
        .map(|line| line.text.chars().count())
        .collect();
    assert_eq!(lengths, [MAX_LINE_CHARS, MAX_LINE_CHARS, 1]);
    assert!(buffer.lines().all(|line| line.stream == Stream::Stderr));
}

#[test]
fn a_line_at_the_limit_is_kept_whole() {
    let mut buffer = OutputBuffer::new();
    buffer.push(Stream::Stdout, &"x".repeat(MAX_LINE_CHARS));
    buffer.push(Stream::Stdout, "");
    assert_eq!(buffer.len(), 2);
}
