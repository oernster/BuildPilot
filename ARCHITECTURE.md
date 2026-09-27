# BuildPilot Architecture

BuildPilot is a Windows desktop application that remembers build scripts, starts them as child
processes, shows their output and stops their process trees. It keeps everything on the local
machine: one settings file, one icons folder and one log, all in the operator's own data folder.
The requirements it answers to are in [SRS.md](SRS.md); requirement IDs below point there.

## Invariants

`UI -> Application -> Domain <- Infrastructure`

Every rule below is enforced by a test, not by convention. Each was proved to bite by planting a
violation and watching the test fail.

| Invariant | Enforced by |
|---|---|
| The domain names only `std::collections`, `error`, `fmt`, `ops`, `path`, `str` and `time::Duration`: no files, processes, threads, environment or clock. | `pure_layers_use_only_allowed_std` in `tests/structural.rs` |
| The application adds `iter`, `mem` and `time::Instant`; it gets every instant from its `Clock` port. | `pure_layers_use_only_allowed_std` |
| The setup program's policy (`src/setup`) names only `std::fmt` and no other layer. | `pure_layers_use_only_allowed_std`, `layers_depend_inwards_only` |
| No pure layer names an external crate. | `pure_layers_use_no_external_crates` |
| Layers depend inwards only: the domain names no other layer, the application names neither infrastructure nor UI, infrastructure never names the UI. | `layers_depend_inwards_only` |
| No source file exceeds 400 lines; none sits in the danger band of 381 to 399. | `no_module_exceeds_the_line_limit_or_sits_in_the_danger_band` |
| `unsafe` appears only under `src/infrastructure/win32`, each block with its SAFETY reasoning. | `unsafe_code_lives_only_in_win32` |
| `Cargo.toml` carries the version in `VERSION` and no other. | `cargo_version_matches_the_version_file` |
| Every control wears the house ring: no standard Slint `Button` or `CheckBox`, no border in the accent colour. | `every_control_follows_the_ring_model` |
| Every text colour reaches 4.5:1 and every ring 3:1 against the surfaces it is drawn on, in both themes. | `tests/contrast.rs` |

## Layers

- **Domain** (`src/domain`). BuildPilot's nouns and rules: an operation and its validation, the
  flight deck and its order, the run state machine, the launch plan for each kind of script,
  cutting output bytes into lines and decoding them, the output buffer and its caps, the tray's
  follow state, selection and preferences. No I/O, no clock, no framework.
- **Application** (`src/application`). `App` owns all state and is driven from one thread. It
  offers one method per thing the operator can do and never waits on a process: a launch
  answers at once; output and the exit arrive later through `App::handle_event`. What it needs
  from the machine is stated as traits in `ports.rs`: `ConfigStore`, `IdSource`, `Clock`,
  `PathProbe`, `IconLibrary`, `Launcher` and `ProcessHandle`, `Shell` and `Log`. Every refusal
  is an `AppError` whose message names the thing and what the operator can do; everything else
  worth saying is a `Notice`, worded once and shown and logged in the same words.
- **Infrastructure** (`src/infrastructure`). The ports implemented against the real machine: the
  JSON settings file, icons on disk, process launch, Explorer, the system clock, UUIDs and the
  log file. Every direct Windows call lives in `win32/`: job objects, the OEM code page, the
  shell, the theme, the single-instance event and the error box.
- **UI** (`src/ui` and `ui/*.slint`). The Slint window over `App` and nothing below it. The
  wording rows and the tray show is worked out in `rows.rs`, which has no Slint types so it is
  tested without a window.
- **Composition root** (`src/main.rs`). The one place the real ports are built and wired. It
  settles the single instance, opens the log, installs the panic hook, then starts `App` and the
  window.

## Running a script

1. `App::run` checks the script and its folder exist (LCH-007, LCH-008), builds the launch plan
   and asks the `Launcher` to start it. A missing file is not an error: the row reads Failed
   with the reason.
2. `WindowsLauncher` starts the program suspended with no console window, puts it in a fresh
   job object, then resumes it. Nothing the script starts can escape the job, so Stop ends the
   whole tree (STOP-001).
3. Each run gets three threads: one reading stdout, one reading stderr, one waiting for the
   exit. None touches application state; they send `RunEvent`s down a channel.
4. The first event after a drain schedules one drain on the UI thread; events arriving before it
   runs ride along. Measured in spike R-2: 50,000 lines in about 0.75 s produced roughly 4,000
   drains, none longer than 3.6 ms.
5. `App::handle_event` appends lines to the run's buffer and moves the state machine on the
   exit. Events for an earlier run of the same operation are ignored (OUT-002).
6. After the exit, the waiter gives the pipes two seconds to drain, since a background process
   the script left running can hold them open indefinitely.

**Stopping.** Stop terminates the job. A tree still alive five seconds later is reported on its
row with the process id (STOP-003). Once a script has exited by itself, anything it deliberately
left running (a compiler server, a build daemon) is released rather than killed with the job.

**Output.** Bytes are cut into lines by `LineAssembler`. A line is decoded as UTF-8; one that is
not valid UTF-8 is decoded in the OEM code page, which hidden console programs write in.
Terminal escape sequences are removed rather than interpreted (OUT-008). A run keeps its latest
100,000 lines and splits a longer line into pieces of 16,384 characters (OUT-004). The tray
reads the buffer directly rather than a copy.

## Keyboard

The house model, applied to every surface (A11Y-002, A11Y-003).

- Slint walks the focus ring on Tab and Shift+Tab and wraps. A disabled top-level `FocusScope`
  in `main.slint` hears every Left and Right a control does not use and turns it into the same
  step through `step_ring` in `src/ui/actions.rs`.
- The window opens on a zero-size sink that holds focus but is never a stop.
- The rows are one stop (`ui/rows_list.slint`). Up and Down walk them; the selected row's own
  controls follow the stop, the other rows' do not.
- The output is a stop only while it overflows, reached by Tab alone and never ringed.
- A dialog opens on its first control. While one is open, `Ring.modal-open` takes every other
  stop off the ring. Escape closes it and focus returns to the control that opened it.
- Every stop reports itself to the `Ring.focused` global, which is how `tests/keyboard.rs` reads
  the ring without pixels.

## Failing without falling over

- **A second launch** finds the named event the first created, hands over its right to take the
  foreground, sets the event and exits; the first restores and raises its window (DATA-001).
- **The log opens before anything else can fail.** Standard error is pointed at it, so what the
  Rust runtime writes there is kept.
- **Every panic is logged** by a hook, whatever thread it happens on (NFR-REL-001).
- **A panic reading output** ends that reading, not BuildPilot; the run's output says so.
- **A failure that ends the run**, a panic on the UI thread or a window that cannot open, shows
  a Windows error box naming the log, then exits with a failure code.
- **A settings file that cannot be read** is set aside; where it cannot even be moved, it is
  left alone and not saved over. The notice area says which.

## The setup program

A second binary in the same crate, `src/bin/buildpilotsetup.rs`, with the release application
built into it by `build.ps1` (INST-001 to INST-006).

- **Policy is pure.** `src/setup` decides the route from one reading of the Apps list entry
  (install, update, downgrade or manage), what each run does as an ordered list of steps and
  every word it shows. It is tested without a window or a machine.
- **Work is infrastructure.** `infrastructure/setup.rs` does each step: the files, the shortcuts
  (through Windows PowerShell's `WScript.Shell`, started with no window), the Apps list entry
  under `HKEY_CURRENT_USER` and removal. Names and folders come from `locations.rs`, which the
  application uses too, so the two cannot disagree.
- **Screens, not greyed controls.** One screen shows at a time: the route, uninstall, running,
  progress and the verdict. The footer holds that screen's actions; progress offers none.
- **Nothing is touched while BuildPilot runs** (INST-004). Setup finds it by executable name
  (never by process tree) then offers to close it.
- **The bar moves with the work.** Each step is weighted by its time measured on a real install.
- **Removing its own folder.** Started from the install folder, as the Apps list does, setup
  copies itself to the temporary folder and runs from there, so the install folder can go.
- **Every path ends in a verdict** or in BuildPilot running, with each step written to a log as
  it happens.

## Data locations

| What | Where |
|---|---|
| Settings and operations | `%APPDATA%\BuildPilot\buildpilot.json` |
| A settings file that could not be read | `buildpilot.json.unreadable` beside it |
| Chosen icons | `%APPDATA%\BuildPilot\icons\` |
| Log | `%APPDATA%\BuildPilot\buildpilot.log` and `buildpilot.previous.log` |
| The program and setup's copy of itself | `%LOCALAPPDATA%\Programs\BuildPilot` |
| The Apps list entry | `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Uninstall\BuildPilot` |
| Setup's log and its temporary copy of itself | `%TEMP%\BuildPilot Setup\` |

`BUILDPILOT_DATA_DIR` names a different data folder, for testing without touching the real one.
BuildPilot itself never writes to a script, a working directory or anywhere outside its data
folder; only setup writes the program folder, the shortcuts and the Apps list entry.

## Design decisions

| Decision | Why | Rejected alternative |
|---|---|---|
| A job object per run | Ends the whole process tree, including anything the script started. | Killing the top process, which leaves its children running. |
| Start suspended, then join the job | Nothing can start before it is in the job. | Joining after start, which leaves a window for escape. |
| Batch files handed to the standard library as the program | Rust quotes their arguments by `cmd.exe` rules and refuses one it cannot escape safely (Amendment 1). | Starting `cmd.exe /c` by hand, which quotes by the wrong rules. |
| Coalesced drains | One UI update per burst keeps the window responsive under a flood of output (spike R-2). | One UI update per line. |
| The rows as one keyboard stop | Ten rows of seven controls would cost seventy Tab presses. | Every row's controls on the ring. |
| One named event for the single instance | The same object answers "is one running?" and "come forward". | A mutex plus a second signal. |
| The instance keyed on the data folder | The default folder is per user, which is DATA-001; a test copy on its own folder runs alongside. | One instance per user whatever the folder. |
| Notice wording in the application layer | The window and the log say the same words. | Wording in the UI, which the log cannot reach. |

See also [TESTING.md](TESTING.md) for how each layer is tested and [DEVELOPMENT.md](DEVELOPMENT.md)
for building it.
