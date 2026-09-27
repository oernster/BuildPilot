//! Starting processes and carrying what they do back to the application (SRS 3.7 to 3.10).
//!
//! Each run gets three threads: one reading stdout, one reading stderr and one waiting for the
//! exit. None of them touches application state; they only send `RunEvent`s down a channel the
//! UI thread drains. Reading a pipe therefore never waits on the UI (OUT-005).

use std::io::Read;
use std::os::windows::io::AsHandle;
use std::os::windows::process::CommandExt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use windows_sys::Win32::System::Threading::{CREATE_NO_WINDOW, CREATE_SUSPENDED};

use crate::application::ports::{Launcher, ProcessHandle, RunEvent, RunEventKind, RunKey};
use crate::domain::launch_plan::LaunchPlan;
use crate::domain::line_assembler::LineAssembler;
use crate::domain::output::Stream;

use super::win32::codepage::decode_oem;
use super::win32::job::{Job, ProcessWatch, resume_main_thread};

/// How long to wait for the pipes to close after the process exits. A background process the
/// script left running can hold them open indefinitely; the exit is reported regardless.
pub const PIPE_DRAIN_TIMEOUT: Duration = Duration::from_secs(2);

/// The exit code reported when the real one could not be learned.
pub const UNKNOWN_EXIT_CODE: i32 = -1;

/// The exit code a stopped process tree is given; the run reads Stopped whatever it is.
const STOP_EXIT_CODE: u32 = 1;

/// Bytes read from a pipe at a time.
const READ_CHUNK_BYTES: usize = 8 * 1024;

/// Pipes read per run: stdout and stderr.
const PIPES_PER_RUN: usize = 2;

/// Delivers run events to the UI thread and wakes it.
#[derive(Clone)]
pub struct EventSink {
    sender: Sender<RunEvent>,
    wake: Arc<dyn Fn() + Send + Sync>,
}

impl EventSink {
    /// Sends down `sender`, then calls `wake` so the receiving thread knows to drain it.
    pub fn new(sender: Sender<RunEvent>, wake: Arc<dyn Fn() + Send + Sync>) -> Self {
        Self { sender, wake }
    }

    fn send(&self, key: &RunKey, kind: RunEventKind) {
        let event = RunEvent {
            key: key.clone(),
            kind,
        };
        // A closed receiver means BuildPilot is shutting down; the event has nowhere to go.
        if self.sender.send(event).is_ok() {
            (self.wake)();
        }
    }

    fn line(&self, key: &RunKey, stream: Stream, text: String) {
        self.send(key, RunEventKind::Line { stream, text });
    }
}

/// Launches processes on Windows, each in its own job object.
pub struct WindowsLauncher {
    sink: EventSink,
}

impl WindowsLauncher {
    /// A launcher reporting through `sink`.
    pub fn new(sink: EventSink) -> Self {
        Self { sink }
    }
}

impl Launcher for WindowsLauncher {
    fn spawn(&mut self, key: RunKey, plan: &LaunchPlan) -> Result<Box<dyn ProcessHandle>, String> {
        let mut command = Command::new(&plan.program);
        // ENV-006, ENV-009: this process's variables only; BuildPilot's own are untouched.
        for name in &plan.variables.remove {
            command.env_remove(name);
        }
        command.envs(plan.variables.set.iter().map(|(name, value)| (name, value)));
        let mut child = command
            .args(&plan.arguments)
            .current_dir(&plan.working_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED)
            .spawn()
            .map_err(|error| error.to_string())?;
        let job = match Job::new().and_then(|job| job.assign(child.as_handle()).map(|()| job)) {
            Ok(job) => job,
            Err(error) => {
                // The child is suspended and has run no code; make sure it never does.
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.to_string());
            }
        };
        // From here the child is in the job: any failure drops `job`, whose kill-on-close ends
        // the child and anything it started.
        let pid = child.id();
        resume_main_thread(pid).map_err(|error| error.to_string())?;
        start_threads(child, key, &self.sink)?;
        Ok(Box::new(WindowsProcess {
            pid,
            job,
            watch: ProcessWatch::open(pid),
        }))
    }
}

/// Starts the two pipe readers and the exit waiter for `child`, which moves to the waiter.
fn start_threads(mut child: Child, key: RunKey, sink: &EventSink) -> Result<(), String> {
    let (done_sender, done_receiver) = mpsc::channel();
    let pipes: [(Option<Box<dyn Read + Send>>, Stream); PIPES_PER_RUN] = [
        (
            child.stdout.take().map(|pipe| Box::new(pipe) as _),
            Stream::Stdout,
        ),
        (
            child.stderr.take().map(|pipe| Box::new(pipe) as _),
            Stream::Stderr,
        ),
    ];
    for (pipe, stream) in pipes {
        let pipe = pipe.ok_or("the process started without an output pipe")?;
        spawn_reader(pipe, stream, key.clone(), sink.clone(), done_sender.clone())?;
    }
    spawn_waiter(child, done_receiver, key, sink.clone())
}

fn spawn_reader(
    pipe: Box<dyn Read + Send>,
    stream: Stream,
    key: RunKey,
    sink: EventSink,
    done: Sender<()>,
) -> Result<(), String> {
    thread::Builder::new()
        .name(format!("run {} {stream:?}", key.run))
        .spawn(move || {
            read_guarded(pipe, stream, &key, &sink);
            // The waiter may already have stopped listening; that is fine.
            let _ = done.send(());
        })
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// What the output says when reading it failed inside BuildPilot (NFR-REL-001).
pub const READER_FAILED: &str = "BuildPilot stopped reading this output after an internal error.";

/// Reads `pipe` to its end as `key`'s `stream`. A panic while reading ends the reading, not
/// BuildPilot: the panic hook logs it and the run's output says so (NFR-REL-001).
pub fn read_guarded(pipe: Box<dyn Read + Send>, stream: Stream, key: &RunKey, sink: &EventSink) {
    let read = catch_unwind(AssertUnwindSafe(|| read_pipe(pipe, stream, key, sink)));
    if read.is_err() {
        sink.line(key, Stream::Stderr, READER_FAILED.to_owned());
    }
}

fn read_pipe(mut pipe: Box<dyn Read + Send>, stream: Stream, key: &RunKey, sink: &EventSink) {
    let mut assembler = LineAssembler::with_fallback(decode_oem);
    let mut chunk = vec![0; READ_CHUNK_BYTES];
    loop {
        match pipe.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(count) => {
                for text in assembler.push(&chunk[..count]) {
                    sink.line(key, stream, text);
                }
            }
        }
    }
    if let Some(text) = assembler.finish() {
        sink.line(key, stream, text);
    }
}

fn spawn_waiter(
    mut child: Child,
    done: Receiver<()>,
    key: RunKey,
    sink: EventSink,
) -> Result<(), String> {
    thread::Builder::new()
        .name(format!("run {} exit", key.run))
        .spawn(move || {
            let code = match catch_unwind(AssertUnwindSafe(|| child.wait())) {
                Ok(Ok(status)) => status.code().unwrap_or(UNKNOWN_EXIT_CODE),
                Ok(Err(error)) => {
                    let text = format!("BuildPilot could not learn how the process ended: {error}");
                    sink.line(&key, Stream::Stderr, text);
                    UNKNOWN_EXIT_CODE
                }
                Err(_) => {
                    let text = "BuildPilot could not learn how the process ended.".to_owned();
                    sink.line(&key, Stream::Stderr, text);
                    UNKNOWN_EXIT_CODE
                }
            };
            let deadline = Instant::now() + PIPE_DRAIN_TIMEOUT;
            for _ in 0..PIPES_PER_RUN {
                let left = deadline.saturating_duration_since(Instant::now());
                if done.recv_timeout(left).is_err() {
                    break;
                }
            }
            sink.send(&key, RunEventKind::Exited { code });
        })
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// A running process tree.
struct WindowsProcess {
    pid: u32,
    job: Job,
    watch: Option<ProcessWatch>,
}

impl ProcessHandle for WindowsProcess {
    fn pid(&self) -> u32 {
        self.pid
    }

    fn stop(&mut self) -> Result<(), String> {
        self.job
            .terminate(STOP_EXIT_CODE)
            .map_err(|error| error.to_string())
    }
}

impl Drop for WindowsProcess {
    /// Once the script itself has exited, anything it deliberately left running (a compiler
    /// server, a build daemon) is released rather than killed with the job. While the script
    /// is still running, dropping the handle kills the whole tree (STOP-005).
    fn drop(&mut self) {
        if self.watch.as_ref().is_some_and(ProcessWatch::has_exited) {
            let _ = self.job.set_kill_on_close(false);
        }
    }
}
