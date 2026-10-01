# BuildPilot Architecture

BuildPilot is a Windows desktop application that remembers build scripts, starts them as child
processes, shows their output and stops their process trees. It keeps everything on the local
machine: one settings file, one run times file, one icons folder and one log, all in the
operator's own data folder.
Its one network request asks GitHub whether a newer release is published (UI-010).
The requirements it answers to are in [SRS.md](SRS.md); requirement IDs below point there.

## Invariants

`UI -> Application -> Domain <- Infrastructure`

Every rule below is enforced by a test, not by convention. Each was proved to bite by planting a
violation and watching the test fail.

| Invariant | Enforced by |
|---|---|
| The domain names only `std::collections`, `error`, `fmt`, `ops`, `path`, `str` and `time::Duration`: no files, processes, threads, environment or clock. | `pure_layers_use_only_allowed_std` in `tests/structural.rs` |
| The application adds `iter`, `mem` and `time::Instant`; it gets every instant from its `Clock` port. | `pure_layers_use_only_allowed_std` |
| The setup program's policy (`src/setup`) names only `std::fmt` plus one layer: the domain (for `Version`). | `pure_layers_use_only_allowed_std`, `layers_depend_inwards_only` |
| No pure layer names an external crate. | `pure_layers_use_no_external_crates` |
| Layers depend inwards only: the domain names no other layer, the application names neither infrastructure nor UI, infrastructure never names the UI. | `layers_depend_inwards_only` |
| No Rust source or test file and no Slint file exceeds 400 lines; none sits in the danger band of 381 to 399. | `no_module_exceeds_the_line_limit_or_sits_in_the_danger_band` |
| No source names a command that builds a Python environment (`venv`, `virtualenv`, `pip`, `uv`, `poetry`, `conda`; ENV-007). | `no_source_builds_an_environment` |
| The interface draws only the small copies in `assets/ui`, none over 256 pixels (UI-013). | `the_interface_draws_only_small_copies` in `tests/assets.rs` |
| `unsafe` appears only under `src/infrastructure/win32`, each block with its SAFETY reasoning. | `unsafe_code_lives_only_in_win32` |
| `Cargo.toml` carries the version in `VERSION` and no other. | `cargo_version_matches_the_version_file` |
| Slint's frame counter never shows: each executable's `main` first removes `SLINT_DEBUG_PERFORMANCE`, before any window exists. | `every_main_hides_the_frame_counter_first` in `tests/frame_counter.rs` |
| The site under `docs/` names the version in `VERSION` and no other. | `the_site_names_the_version_file` in `tests/site.rs` |
| Every control wears the house ring: no standard Slint `Button` or `CheckBox`, no border in the accent colour. | `every_control_follows_the_ring_model` |
| Every text colour reaches 4.5:1 and every ring 3:1 against the surfaces it is drawn on, in both themes. | `tests/contrast.rs` |
| Every toolbar and row control is named in the Guide; a label that changes with state needs an entry for each state. | `the_guide_names_every_control` in `tests/guide.rs` |
| The open source credits are the crates a release build compiles in: nothing dev-only, build-only or procedural. | `the_credits_are_the_shipped_graph` in `tests/infrastructure/build_info.rs` |

## Layers

- **Domain** (`src/domain`). BuildPilot's nouns and rules: an operation, its ordered steps and
  their validation, folders compared as Windows compares them, the flight deck with its order
  and where a new row slots in by name, the run state machine, the launch plan for each kind of
  script, where a project's installer is found and when it may be launched, the operator's host
  table, finding a Python environment and the variables that deactivate and activate one, the
  folder-scan patterns and what a scan proposes, cutting output bytes into lines and decoding
  them, the output buffer and its caps, each operation's recent successful run times and the
  typical one among them (their median), the tray's follow state, selection and preferences,
  release versions, the open source credits and the self-reading cycle of a surface of text. No
  I/O, no clock, no framework.
- **Application** (`src/application`). `App` owns all state and is driven from one thread. It
  offers one method per thing the operator can do and never waits on a process: a launch
  answers at once; output and the exit arrive later through `App::handle_event`. What it needs
  from the machine is stated as traits in `ports.rs`: `ConfigStore`, `RunTimesStore`,
  `IdSource`, `Clock`, `PathProbe`, `Variables`, `IconLibrary`, `Launcher` and `ProcessHandle`, `Shell`, `Log` and
  `ReleaseSource`. A folder scan reads the folder through `PathProbe` and proposes; nothing is
  added until the operator confirms through `App::add`.
  `updates.rs` decides what an update check found and what to say about it. Every refusal
  is an `AppError` whose message names the thing and what the operator can do; everything else
  worth saying is a `Notice`, worded once and shown and logged in the same words.
- **Infrastructure** (`src/infrastructure`). The ports implemented against the real machine: the
  JSON settings file, the JSON run times file, icons on disk, process launch, Explorer, the
  system clock, UUIDs and the log file, plus the release source on GitHub and the credits the build script generated
  (`build_info.rs`). Every direct Windows call lives in `win32/`: job objects, the OEM code page,
  the shell, the theme, the single-instance event, the error box and the one HTTPS GET.
- **UI** (`src/ui` and `ui/*.slint`). The Slint window over `App` and nothing below it. The
  wording rows and the tray show is worked out in `rows.rs`, which has no Slint types so it is
  tested without a window.
- **Composition root** (`src/main.rs`). The one place the real ports are built and wired. It
  settles the single instance, opens the log, installs the panic hook, then starts `App` and the
  window.

## Running a script

1. `App::run` refuses a second run of the operation (LCH-005) and a run in a folder another
   running operation holds (LCH-010). It then checks every step's script, the folder and the
   environment before step 1 starts (STEP-005), builds the first step's launch plan and asks the
   `Launcher` to start it. A missing file is not an error: the row reads Failed with the reason.
   Each later step starts when the one before exits with code 0 (STEP-002).
2. `WindowsLauncher` starts the program suspended with no console window, puts it in a fresh
   job object, then resumes it. Nothing the script starts can escape the job, so Stop ends the
   whole tree (STOP-001).
3. Each run gets three threads: one reading stdout, one reading stderr, one waiting for the
   exit. None touches application state; they send `RunEvent`s down a channel.
4. The first event after a drain schedules one drain on the UI thread; events arriving before it
   runs ride along. Measured in spike R-2: 50,000 lines in about 0.75 s produced roughly 4,000
   drains, none longer than 3.6 ms.
5. `App::handle_event` appends lines to the run's buffer and moves the state machine on the
   exit; a success's duration joins the operation's run times, which are saved at once
   (LIFE-008). Events for an earlier run of the same operation are ignored (OUT-002).
6. After the exit, the waiter gives the pipes two seconds to drain, since a background process
   the script left running can hold them open indefinitely.

**Stopping.** Stop terminates the job. A tree still alive five seconds later is reported on its
row with the process id (STOP-003). Once a script has exited by itself, anything it deliberately
left running (a compiler server, a build daemon) is released rather than killed with the job.

**Output.** Bytes are cut into lines by `LineAssembler`. A line is decoded as UTF-8; one that is
not valid UTF-8 is decoded in the OEM code page, which hidden console programs write in.
Terminal escape sequences are removed rather than interpreted; a tab becomes the spaces reaching
the next 8-column tab stop and any other control character is dropped, since the font would draw
either as a box (OUT-008). A run keeps its latest
100,000 lines and splits a longer line into pieces of 16,384 characters (OUT-004). The tray
reads the buffer directly rather than a copy. Every line is drawn in the plain text colour
whichever stream it came on; the run's outcome is one closing line in its own colour (OUT-007).

## Launching an installer

Launch installer (PKG-001 to PKG-005) starts a project's setup program; BuildPilot never waits
on it, reads its output or stops it.

- **The rule is pure.** `domain/installer.rs` decides the default (the working directory's name
  plus `Setup.exe` in `dist-installer`, then `dist`, compared by letters and digits) from a
  listing it is handed. It also decides whether a run state allows a launch: not while running
  or stopping, not after a run that was stopped or failed.
- **The looking is cached.** `App` records each operation's installer through `PathProbe` at
  start, on add, edit and select and when a run ends; `application/installer_actions.rs` holds
  the use case. The rows redraw four times a second and a look reads two folders, so the rows read
  the record, never the disk. Launch looks again before it starts anything.
- **Starting it is the shell's job.** `Shell::open` hands the file to Windows as Explorer would,
  so an installer that needs administrator rights gets the usual prompt.

## Help, the credits and the update check

- **The menu** (UI-006) is `HelpMenu` in `ui/help.slint`, dropped from the Help button. It is a
  dialog as far as the keyboard ring is concerned: while it is open only its entries are stops;
  Up and Down step the ring through them.
- **The Guide** (UI-007) is data in `ui/guide.slint`: one entry per control with its own icon,
  then the rules, then keyboard use. `the_guide_names_every_control` reads every `IconButton`
  label in `ui/main.slint` and `ui/row.slint` and fails when the Guide has no entry for one; a
  label that changes with state (the theme toggle) needs an entry for every state.
- **The credits** (UI-009) are generated by `build_credits.rs`, part of the build script. It
  takes the set from `cargo tree --edges normal,no-proc-macro`, which resolves features the way
  a release build does (`cargo metadata` merges in what dev-dependencies ask for and so names
  crates that never ship), then each crate's licence and folder from `cargo metadata`. It writes
  one line per crate for About and `THIRD-PARTY-NOTICES.txt` with every licence file those
  crates ship, each distinct text once. Both reach the program through `build_info.rs`; setup
  writes the notices beside the program as part of copying the files.
- **Reading surfaces** (UI-011). The cycle is the pure machine in `domain/auto_scroll.rs`, ported
  from PigeonPost. `ReadingPane` in `ui/reading.slint` drives it on a Slint timer through the
  `Reading` global, which `ui/reading.rs` answers. A hand is found by one watch: the position
  the pane last placed. Any other position means the wheel, the scroll bar or the keys moved it.
  While the update prompt is open, the surface beneath is frozen rather than read.
- **The update check** (UI-010). `application/updates.rs` decides: the tag read as a version, the
  skip honoured only when the window asked by itself, the setup program preferred to the release
  page. `GitHubReleases` in `infrastructure/releases.rs` asks `releases/latest`, which answers only
  a published release, through `win32/http.rs` (WinHTTP, so no HTTP or TLS crate). It treats
  the answer as foreign input: every field checked, the size capped. `ui/help.rs` asks from a
  worker thread 3 seconds after the window opens, then every 24 hours; also on demand. The answer
  returns through the `Update` hook in `ui/events.rs`; a panic on the worker still answers, as
  unreachable, so the operator's own check always gets a reply.

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
  stop off the ring. Closing it returns focus to the control that opened it. Escape closes it in
  the headless suite but not on the real window, a known limitation (TESTING.md).
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
- **A run times file that cannot be read or written** is logged and nothing more: an unreadable
  file means the typical times start afresh; a failed save leaves them in memory until the next. They only inform, so they never
  warrant a notice.

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
  copies itself to the temporary folder and runs from there, so the install folder can go. The
  next setup that is not that copy deletes it; a copy still running is left for the time after.
- **Every path ends in a verdict** or in BuildPilot running, with each step written to a log as
  it happens.

## Data locations

| What | Where |
|---|---|
| Settings and operations | `%APPDATA%\BuildPilot\buildpilot.json` |
| A settings file that could not be read | `buildpilot.json.unreadable` beside it |
| Each operation's recent successful run times (LIFE-008) | `%APPDATA%\BuildPilot\run-times.json` |
| Chosen icons | `%APPDATA%\BuildPilot\icons\` |
| Log | `%APPDATA%\BuildPilot\buildpilot.log` and `buildpilot.previous.log` |
| The program and setup's copy of itself | `%LOCALAPPDATA%\Programs\BuildPilot` |
| The Apps list entry | `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Uninstall\BuildPilot` |
| Setup's log and its temporary copy of itself | `%TEMP%\BuildPilot Setup\` |

`BUILDPILOT_DATA_DIR` names a different data folder, for testing without touching the real one.
BuildPilot itself never writes to a script, a working directory or anywhere outside its data
folder; only setup writes the program folder, the shortcuts and the Apps list entry. What a
script or a launched installer writes is that program's own doing.

## Design decisions

| Decision | Why | Rejected alternative |
|---|---|---|
| A job object per run | Ends the whole process tree, including anything the script started. | Killing the top process, which leaves its children running. |
| Start suspended, then join the job | Nothing can start before it is in the job. | Joining after start, which leaves a window for escape. |
| Batch files handed to the standard library as the program | Rust quotes their arguments by `cmd.exe` rules and refuses one it cannot escape safely (Amendment 1). | Starting `cmd.exe /c` by hand, which quotes by the wrong rules. |
| Coalesced drains | One UI update per burst keeps the window responsive under a flood of output (spike R-2). | One UI update per line. |
| The rows as one keyboard stop | Ten rows of eight controls would cost eighty Tab presses. | Every row's controls on the ring. |
| One named event for the single instance | The same object answers "is one running?" and "come forward". | A mutex plus a second signal. |
| The instance keyed on the data folder | The default folder is per user, which is DATA-001; a test copy on its own folder runs alongside. | One instance per user whatever the folder. |
| Notice wording in the application layer | The window and the log say the same words. | Wording in the UI, which the log cannot reach. |
| An installer is stored only once saved from Edit | The dialog shows the default found and Save keeps what its field holds; until then an operation follows the project's own convention. The config file stays schema 2, so an earlier release still reads it (DATA-002). | Writing the default into every operation at load, which rewrites every entry unasked. |
| Run times in a file of their own | The config file holds no run data (CFG-004); a damaged timings file can never put the operator's configuration at risk. Both files share one atomic write. | A field in `buildpilot.json`, which CFG-004 forbids. |
| The typical time is the median of the last five successes | One cold-cache build does not move it; it follows a build that has grown quicker or slower. A failed or stopped run says nothing about how long a build takes. | The mean, which one outlier drags; the last run alone, which a stopped run would blank. |
| The tray pins itself to the end while following | The list measures new rows only when it next lays out, so a scroll asked for as they arrive stopped short and hid the run's closing line (OUT-006). | Scrolling once from Rust after each drain. |

See also [TESTING.md](TESTING.md) for how each layer is tested and [DEVELOPMENT.md](DEVELOPMENT.md)
for building it.
