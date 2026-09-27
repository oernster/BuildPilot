use std::io::{self, Read};
use std::sync::Arc;
use std::sync::mpsc;

use buildpilot::application::ports::{RunEventKind, RunKey};
use buildpilot::domain::operation::OperationId;
use buildpilot::domain::output::Stream;
use buildpilot::infrastructure::launcher::{EventSink, READER_FAILED, read_guarded};

/// A pipe that gives one line, then panics.
struct PanickingPipe {
    served: bool,
}

impl Read for PanickingPipe {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.served {
            panic!("planted reader panic");
        }
        self.served = true;
        let line = b"before\n";
        buffer[..line.len()].copy_from_slice(line);
        Ok(line.len())
    }
}

// NFR-REL-001: a panic while reading output ends that reading rather than BuildPilot; the run's
// output says so.
#[test]
fn a_panicking_reader_is_contained_and_shown() {
    let (sender, events) = mpsc::channel();
    let sink = EventSink::new(sender, Arc::new(|| {}));
    let key = RunKey {
        operation: OperationId::new("op").unwrap(),
        run: 1,
    };
    read_guarded(
        Box::new(PanickingPipe { served: false }),
        Stream::Stdout,
        &key,
        &sink,
    );

    let lines: Vec<(Stream, String)> = events
        .try_iter()
        .map(|event| match event.kind {
            RunEventKind::Line { stream, text } => (stream, text),
            RunEventKind::Exited { .. } => panic!("no exit expected"),
        })
        .collect();
    assert_eq!(
        lines,
        [
            (Stream::Stdout, "before".to_owned()),
            (Stream::Stderr, READER_FAILED.to_owned()),
        ]
    );
}
