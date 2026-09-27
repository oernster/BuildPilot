# BuildPilot: Software Requirements Specification

Status: **Baseline 1.0, 2026-09-27.** Every question in Appendix B is closed. From here, changes
arrive as numbered amendments with a reason, never as silent edits.

Source: `BuildPilot-SPEC.md` (initial product specification), plus decisions taken on
2026-09-27. Each requirement's Source line points at the spec section it comes from.

Priorities use MoSCoW: **M** Must, **S** Should, **C** Could. The Won't-this-time list is
Appendix D. Every requirement names the test that will verify it, including tests not yet
written. "Manual" means a check in TESTING.md that needs a real desktop and a real person.

---

## 1. Introduction

### 1.1 Purpose

BuildPilot is a Windows desktop build flight deck. It remembers the build scripts an operator
runs often. It launches them as independent child processes, several at once. It shows each
one's state and live output. It stops them cleanly, including their child process trees.
BuildPilot orchestrates existing commands; it never becomes the build system. The scripts stay
the source of truth and stay runnable without BuildPilot.

### 1.2 Intended audience

The author (Oliver Ernster), as product owner and developer, plus any AI assistant working on the
code. Tests name requirement IDs from this document.

### 1.3 Scope

In scope for v1: configuring, remembering, ordering, launching, monitoring and stopping build
operations; showing live output per operation; opening and revealing scripts; icons; light and
dark themes; Help/About; a bespoke per-user Windows installer.

### 1.4 Out of scope (v1)

BuildPilot v1 is none of the following; it gains no feature of any of them:

- a replacement for PowerShell, cmd, Make, Cargo, npm, Gradle or any build tool;
- a CI/CD server, a cloud service or a networked service of any kind;
- a build-script editor (BuildPilot never writes to a script);
- a dependency graph, pipeline or sequencing designer;
- an "autopilot" that decides what to run;
- a CPU-core or thread scheduler for the underlying tools;
- a build-artifact manager;
- a terminal emulator (no cursor addressing, no interactive input to a running script);
- Run All / Stop All; no action of any kind on checked rows (§4.2 of the spec).

### 1.5 Definitions

| Term | Meaning |
|---|---|
| Operation | One configured build command: a script or executable, its working directory, its arguments, its name and its icon. Persisted. |
| Run | One execution of an operation, from launch to termination. Transient. |
| Flight deck | The ordered list of operations shown in the main window. |
| Row | The visual representation of one operation in the flight deck. |
| Selected row | The row whose output the output tray shows. Exactly zero or one. Distinct from a checked row. |
| Checked row | A row whose checkbox is ticked. Has no action attached in v1. |
| Output tray | The collapsible panel at the foot of the main window showing one run's output. |
| Process tree | The launched process plus every process it starts, directly or indirectly. |
| Host | The program that executes a script, e.g. `pwsh.exe` for a `.ps1`. |
| Config file | The single file holding all persisted operations and global preferences. |
| Data folder | `%APPDATA%\BuildPilot`. |

### 1.6 References

- `BuildPilot-SPEC.md`, initial product specification.
- ISO/IEC/IEEE 29148:2018 (requirement quality), EARS syntax (Mavin et al., RE'09).
- Slint licence, `github.com/slint-ui/slint/LICENSE.md` (read 2026-09-27).

---

## 2. Overall description

### 2.1 Product perspective

A new, standalone, local-only desktop application. It depends on nothing networked. It
interacts with the operating system in five ways only: launching processes, terminating process
trees, opening a file with its associated application, revealing a file in Explorer, reading
and writing its own data folder.

### 2.2 User classes

One class: **the operator**, a developer on their own Windows machine who runs their own build
scripts. There are no roles, accounts or permissions inside BuildPilot. BuildPilot runs with the
operator's normal user rights and never requests elevation.

### 2.3 Operating environment

- Windows 11 x64 (reference machine: the author's development PC). Windows 10 x64 is not tested
  in v1; see Appendix D.
- PowerShell 7 (`pwsh.exe`) optional; Windows PowerShell 5.1 (`powershell.exe`) is present on
  every supported Windows.

### 2.4 Constraints

| ID | Constraint | Rationale |
|---|---|---|
| CON-001 | Implementation language is Rust (stable toolchain). | Owner decision; first Rust project. |
| CON-002 | UI framework is Slint, used under its GPLv3 licence. | See 2.6. |
| CON-003 | Code layering is `ui → application → domain ← infrastructure`, enforced by a structural test. | House architecture invariant; keeps OS integration behind narrow adapters (spec §16, §17). |
| CON-004 | The supplied artwork in `assets/` is product artwork and is used as supplied. | Spec §15. |
| CON-005 | The version has one home, the `VERSION` file at the repo root; nothing else holds a version literal by hand. | House versioning rule. The build script stamps Cargo.toml from `VERSION`; a structural test fails when they differ (OQ-12). |
| CON-006 | Repository licence is GPL-3.0. | Existing `LICENSE`; compatible with Slint's GPLv3 option. |

### 2.5 Assumptions

| ID | Assumption | Owner | Confirm by |
|---|---|---|---|
| AS-001 | The operator's scripts report failure through a non-zero exit code. A script that fails yet exits 0 will show as Succeeded; BuildPilot does not parse output to second-guess it (spec §10). | Oliver | 2026-10-04 |
| AS-002 | Scripts do not need interactive input. Standard input is closed at launch, so a script that prompts receives end of input. | Oliver | 2026-10-04 |
| AS-003 | Rust will be installed by the operator (`rustup-init`). Measured 2026-09-27: `cargo` absent from PATH and from `~\.cargo\bin`; `~\.rustup` present. | Oliver | before implementation |

### 2.6 Framework decision record

**Decision:** Slint. **Decided:** 2026-09-27 by the owner, from four candidates.

| Candidate | For | Against |
|---|---|---|
| **Slint** (chosen) | The whole application stays Rust, which suits a first Rust project. Declarative markup keeps layout apart from logic. Offers a GPLv3 licence that matches the repo. Has accessibility support and theming. | Drag reordering of list rows must be hand built (risk R-1). A large scrolling output view must be virtualised by hand (risk R-2). |
| Tauri v2 + React/TS | Closest to the author's Wails experience; richest UI toolkit. | Most UI code would be TypeScript, not Rust. Adds a web toolchain and a second language. |
| egui/eframe | Pure Rust; quickest start. | Immediate-mode look reads less like a polished Windows app; accessibility less mature. |
| iced | Pure Rust, Elm architecture. | Weakest accessibility of the four, against spec §19. |

The Slint "against" points are hypotheses from general knowledge, not measurements. The first two
implementation spikes (R-1, R-2) settle them before the design depends on them.

---

## 3. Requirements

### 3.1 Persistence (CFG)

**CFG-001 (M) Save on change.** When the operator adds, edits, removes or reorders an operation
(or changes a global preference), the configuration store shall write the config file.
Source: spec §12. Acceptance: Given two operations, when the second is dragged above the first
and BuildPilot is restarted, then the order is second, first.
Verified by: `infrastructure::config_store` test `reorder_survives_reload`.

**CFG-002 (M) Atomic write.** The configuration store shall write the config file by writing a
temporary file in the data folder and renaming it over the old one.
Acceptance: a write interrupted after the temporary file is written leaves the previous config
file intact. Verified by: `config_store` test `interrupted_write_keeps_previous_file`.

**CFG-003 (M) Persisted fields.** For each operation the config file shall hold: a stable ID
(UUID, assigned at creation, never reused), display name, script path (absolute), working
directory (absolute), arguments, icon reference, position in the flight deck. Globally it shall
hold the schema version and the theme preference. Source: spec §12.
Verified by: `domain::config` round-trip test `every_field_round_trips`.

**CFG-004 (M) Runtime state is never persisted.** The config file shall hold no process ID,
run state, output or timestamp of a run. Source: spec §12.
Acceptance: after a restart during a run, the row shows Idle, never Running.
Verified by: `domain::config` test `serialised_form_has_no_runtime_fields`.

**CFG-005 (M) One bad operation does not block the rest.** If one operation entry in the config
file cannot be parsed, then the configuration store shall load every other operation. It shall
show a notice naming how many entries were unreadable. It shall write the unreadable entries back
unchanged on every later save. Source: spec §12, §18.
Acceptance: Given a file with three operations where the second has a numeric `name`, then two
rows load; the notice says one entry is unreadable; after an edit to row one the file still
contains the second entry byte for byte.
Verified by: `config_store` test `malformed_entry_is_preserved_and_reported`.

**CFG-006 (M) Unreadable config file.** If the whole config file cannot be parsed, then the
configuration store shall rename it to `buildpilot.json.unreadable` (replacing any older one),
start with an empty flight deck and show a notice naming the renamed file.
Verified by: `config_store` test `corrupt_file_is_set_aside`.

**CFG-007 (M) First run.** While the flight deck has no operations, the main window shall show
a prompt to add a build script in place of the row list.
Verified by: Manual.

**CFG-008 (M) Unwritable data folder.** If the config file cannot be written, then BuildPilot
shall keep running with the in-memory configuration and show a notice naming the path and the
operating-system error. Verified by: `config_store` test with a read-only directory.

**CFG-009 (M) Schema version.** The config file shall carry a schema version. When BuildPilot
reads an older schema version, it shall migrate it. If it reads a newer schema version, then it
shall refuse to write the file and say so, so a downgrade cannot destroy data.
Verified by: `config_store` tests `older_schema_migrates`, `newer_schema_is_read_only`.

**CFG-010 (S) Window layout.** BuildPilot shall persist the window size and position, the output
tray's expanded state and the tray height. It shall restore them at start.
Verified by: `domain::config` round-trip test; Manual for restore.

### 3.2 Adding operations (ADD)

**ADD-001 (M) Add by browsing.** When the operator activates Add, BuildPilot shall open a file
picker filtered to the supported types (LCH-001). When a file is chosen, BuildPilot shall open the
operation dialog pre-filled with the derived defaults. Source: spec §5.
Verified by: Manual (file picker); defaults per ADD-002 to ADD-004.

**ADD-002 (M) Default working directory.** The operation factory shall default the working
directory to the chosen script's parent directory. Source: spec §5.
Acceptance: choosing `C:\src\app\build.ps1` gives `C:\src\app`.
Verified by: `domain::operation` test `working_dir_defaults_to_script_parent`.

**ADD-003 (M) Default name.** The operation factory shall default the display name to the
script's parent folder name followed by a space and the file stem (OQ-5).
Acceptance: `C:\src\pigeonpost\build.ps1` gives `pigeonpost build`.
Verified by: `domain::operation` test `name_defaults_from_folder_and_stem`.

**ADD-004 (M) Default arguments.** The operation factory shall default arguments to none.

**ADD-005 (M) Required fields.** While the name or the script path is empty, the operation
dialog shall refuse to save; it shall say which field is missing.
Verified by: `domain::operation` validation tests.

**ADD-006 (M) Duplicates allowed.** The operation factory shall allow the same script to be added
more than once, since two operations may differ only in arguments.

### 3.3 Icons (ICON)

**ICON-001 (M) Discovery convention.** When a script is chosen, the icon resolver shall look for
`assets\application-icon.png` beneath the script's directory. Where that file exists and decodes
as an image, the resolver shall use it as the operation's icon. Source: spec §5.1.
Verified by: `infrastructure::icon_discovery` tests `finds_conventional_icon`,
`ignores_undecodable_file`, `absent_icon_gives_none`.

**ICON-002 (M) Placeholder.** While an operation has no usable icon, its row shall show the
BuildPilot default placeholder icon. Source: spec §5.1.

**ICON-003 (M) Isolated conventions.** The icon resolver shall take its conventions from one
ordered list, so a further convention is added as one entry with no change elsewhere.
Verified by: inspection; `icon_discovery` test iterates the list.

**ICON-004 (M) Choose icon.** When the operator activates Choose Icon in the operation dialog,
BuildPilot shall open an image picker (PNG, JPEG, ICO) and use the chosen image.
Per OQ-6, the chosen image is copied into `%APPDATA%\BuildPilot\icons\<operation id>.<ext>`
so it survives the original moving; a discovered icon is referenced by path.

**ICON-005 (M) Missing icon at start.** If an operation's icon file is missing or unreadable at
start, then its row shall show the placeholder plus a marker whose tooltip names the missing path.
Source: spec §18. Verified by: `icon_discovery` test; Manual for the marker.

**ICON-006 (M) No distortion.** The rendering of every icon and every supplied asset shall
preserve its aspect ratio. Source: spec §15. `negative.png` is 1278x1230 (measured), so this
matters. Verified by: Manual.

### 3.4 Editing (EDIT)

**EDIT-001 (M) Edit fields.** When the operator activates Edit, BuildPilot shall open the
operation dialog showing the name, script path, working directory, arguments and icon, all
editable. Edit never touches the script. Source: spec §6.

**EDIT-002 (M) Edit while running.** While an operation is running, the operation dialog shall
state that changes to the script path, working directory and arguments take effect on the next
run; the running process shall be unaffected. Name and icon changes apply at once.
Source: spec §6. Verified by: `application::deck` test `edit_during_run_does_not_touch_run`.

### 3.5 Removal (REM)

**REM-001 (M) Confirm before remove.** When the operator activates Remove, BuildPilot shall show
a confirmation naming the operation and stating that the script file is not deleted.
Verified by: Manual.

**REM-002 (M) Configuration only.** The remove use case shall delete the operation's config entry
plus any icon copied for it under the data folder. It shall delete nothing else.
Verified by: `application::deck` test `remove_leaves_script_on_disk`.

**REM-003 (M) Not while running.** While an operation is running, its Remove control shall be
disabled, with a tooltip saying to stop it first. Source: spec §13.
Verified by: `application::deck` test `remove_refused_while_running`.

### 3.6 Rows and ordering (ROW)

**ROW-001 (M) Row contents.** Each row shall show, left to right: reorder handle, checkbox, icon,
name, script file name with its folder, status, Run, Stop, Edit, Open Script, Locate Script,
Remove. Source: spec §4.1.

**ROW-002 (M) Stable layout.** The row shall keep every control in the same position in every
state; controls that do not apply are disabled, never hidden. Source: spec §4.1, §19.
Verified by: Manual.

**ROW-003 (M) Drag reorder.** When the operator drags a row by its handle and drops it, the
flight deck shall move the operation to the drop position and save the order. Source: spec §4.3.

**ROW-004 (M) Keyboard reorder.** When a row has focus and the operator presses Alt+Up or
Alt+Down, the flight deck shall move it one place and save the order. Source: spec §19.
Verified by: `domain::deck` tests `move_up`, `move_down`, `move_at_edge_is_no_op`.

**ROW-005 (M) Reorder does not touch runs.** Reordering shall not interrupt, restart or
re-attribute any running operation or its output. Source: spec §4.3.
Verified by: `application::deck` test `reorder_during_run_keeps_run`.

**ROW-006 (M) Checkboxes.** Each row shall have a checkbox whose state the flight deck holds as a
set of operation IDs. No action uses it in v1. Checkbox state is not persisted (OQ-7).

**ROW-007 (M) Selection.** When the operator clicks a row (or moves focus to it), the flight deck
shall make it the selected row and the output tray shall show its latest run. Source: spec §11.1.

### 3.7 Launching (LCH)

**LCH-001 (M) Host by file type.** The launcher shall choose the host from one table keyed by
file extension:

| Extension | Launched as |
|---|---|
| `.ps1` | `pwsh.exe` if found on PATH, else `powershell.exe`; with `-NoProfile -NonInteractive -ExecutionPolicy Bypass -File <script> <arguments>` |
| `.bat`, `.cmd` | the script itself, with `<arguments>`; the standard library runs it through `cmd.exe` (Amendment 1) |
| `.exe`, `.com` | directly |

A file of any other type is refused at Add with a message listing the supported types.
Verified by: `domain::launch_plan` table tests (one per row, plus the pwsh-absent case).

**LCH-002 (M) Launch.** When the operator activates Run on an idle, succeeded, failed or stopped
operation, the launcher shall start the host as a child process with the configured working
directory and arguments, with stdout and stderr captured and stdin closed. Source: spec §8.

**LCH-003 (M) Paths and arguments.** The launcher shall pass script paths, working directories
and arguments containing spaces, quotes or non-ASCII characters to the host intact.
Arguments are stored as a list (one per line in the dialog); the launcher quotes each one
(OQ-8). Verified by: `infrastructure::process` integration test using a fixture script that
echoes each argument it receives, with paths under a folder named `with space é`.

**LCH-004 (M) No console window.** The launcher shall start the host without opening a console
window. Verified by: Manual.

**LCH-005 (M) One run per operation.** While an operation is running, its Run control shall be
disabled and the run use case shall refuse a second launch of it. Source: spec §8.2.
Verified by: `application::runner` test `second_run_is_refused`.

**LCH-006 (M) Concurrent runs.** The run use case shall allow any number of different operations
to run at once, with no queueing. Source: spec §8.1. Verified by: `application::runner` test
running three fake processes that finish in reverse order of launch.

**LCH-007 (M) Missing script.** If the script file does not exist when Run is activated, then the
run use case shall not launch it; the row shall show Failed with the reason "Script not found"
plus the path. The row shall offer Edit to repair it. Source: spec §7, §18.
Verified by: `application::runner` test `missing_script_is_not_launched`.

**LCH-008 (M) Missing working directory.** If the working directory does not exist when Run is
activated, then the run use case shall not launch it and the row shall name the directory.

**LCH-009 (M) Launch failure.** If the operating system refuses to start the host (including
access denied and host not found), then the row shall show Failed with the operating-system
message and the command that was attempted. Source: spec §18.
Verified by: `infrastructure::process` test with a nonexistent host.

### 3.8 Lifecycle and progress (LIFE)

**LIFE-001 (M) States.** The run state machine shall have exactly these states: Idle, Running,
Succeeded, Failed, Stopped. Failed carries one of two reasons: failed to start; exited with code N.
Source: spec §10. Verified by: `domain::lifecycle` tests covering every allowed transition and
refusing every other.

**LIFE-002 (M) Success from exit status.** When a run's process exits by itself, the run state
machine shall move to Succeeded if the exit code is 0 and to Failed with the code otherwise.
Output content is never inspected. Source: spec §10.
Verified by: `domain::lifecycle` tests; `infrastructure::process` tests with fixture scripts
exiting 0 and 3.

**LIFE-003 (M) Stopped is not success.** When a run ends because the operator stopped it, the
run state machine shall move to Stopped whatever exit code the process reports.
Source: spec §9.

**LIFE-004 (M) Visible promptly.** When Run is activated, the row shall show Running within
100 ms, measured from the click event to the state change reaching the UI in a debug build.
Source: spec §8, "update the row immediately".

**LIFE-005 (M) Status not by colour alone.** Each state shall be shown by a text label and a
distinct glyph as well as colour. Running shows an animated indeterminate indicator and the
elapsed time as `m:ss`. Source: spec §10, §19.

**LIFE-006 (M) No invented percentage.** The progress model shall have an Indeterminate value and
a Determinate(percent) value; v1 shall only ever produce Indeterminate. Source: spec §10.
Verified by: inspection plus `domain::lifecycle` test.

**LIFE-007 (M) Finish details.** When a run ends, its row shall show the final state, the exit
code where there is one and the run's duration.

### 3.9 Stopping (STOP)

**STOP-001 (M) Stop the tree.** When the operator activates Stop on a running operation, the
process adapter shall terminate the operation's whole process tree. Each run is started inside
its own Windows Job Object; Stop terminates that job. Source: spec §9.
Verified by: `infrastructure::process` Windows test: a fixture script starts a grandchild
process; after Stop, neither process exists.

**STOP-002 (M) Isolation.** Stopping one operation shall leave every other running operation
running. Verified by: `infrastructure::process` test with two runs, one stopped.

**STOP-003 (M) Stop failure.** If the process tree cannot be terminated within 5 seconds of Stop,
then the row shall show a Stop failed notice naming the process ID still alive. Stop shall
remain available. Source: spec §18.

**STOP-004 (M) Closing BuildPilot with runs active.** When the operator closes the main window
while any operation is running, BuildPilot shall ask for confirmation naming the running
operations. On confirmation it shall stop them all and exit; on cancel it shall stay open.

**STOP-005 (M) No orphans on crash.** If BuildPilot ends without stopping its runs (including a
crash), then every run's process tree shall be terminated. The Job Objects are created with
kill-on-close. Verified by: `infrastructure::process` test that drops the job handle.

### 3.10 Output tray (OUT)

**OUT-001 (M) Live output.** While the selected row's operation is running, the output tray shall
append its stdout and stderr lines as they arrive. Source: spec §11.

**OUT-002 (M) Attribution.** The output store shall keep each run's output keyed by operation ID,
so no line is ever shown under another operation. Source: spec §11.1.
Verified by: `application::output` test with two runs interleaving lines.

**OUT-003 (M) Retained after the run.** The output store shall keep the latest run's output for
each operation until that operation is run again or BuildPilot exits. Source: spec §11.1.

**OUT-004 (M) Retention cap.** The output store shall keep at most the latest 100,000 lines per
operation, dropping the oldest. The tray shall say when lines have been dropped. A single line
longer than 16,384 characters shall be split. Verified by: `domain::output_buffer` tests.

**OUT-005 (M) Responsiveness under load.** While a run emits 50,000 lines within 5 seconds, the UI
shall keep responding to a click within 100 ms. Reading the process pipes shall never wait on the
UI. Measured on the reference machine with a debug-build event-loop latency readout and the
fixture script `tests\fixtures\flood.ps1`.

**OUT-006 (M) Auto-follow.** While the tray is scrolled to its last line, it shall follow new
output. When the operator scrolls up, it shall stop following and show a Jump to latest control.
When the operator returns to the last line or activates Jump to latest, following shall resume.
Source: spec §11. Verified by: `domain::follow` state tests; Manual.

**OUT-007 (M) stderr marked.** The tray shall mark stderr lines with a gutter marker as well as a
colour. Source: spec §11.1, §19.

**OUT-008 (M) Text decoding.** The output reader shall decode each line as UTF-8 when it is valid
UTF-8; otherwise it shall decode the line in the Windows OEM code page (Amendment 2). It shall
remove ANSI escape sequences. Verified by: `domain::text` tests; `infrastructure::process` tests
reading a folder name containing `é`.

**OUT-009 (M) Collapse and resize.** The operator shall be able to collapse, expand and resize
the tray. Collapsing it shall not affect any run.

### 3.11 Script navigation (NAV)

**NAV-001 (M) Open Script.** When the operator activates Open Script, the shell adapter shall
open the script with its associated application. Source: spec §7.

**NAV-002 (M) Locate Script.** When the operator activates Locate Script, the shell adapter shall
open Explorer with the script selected. Source: spec §7.

**NAV-003 (M) Missing script.** If the script is missing when Open or Locate is activated, then
BuildPilot shall say so, name the path and offer Edit. Locate shall open the nearest existing
parent folder instead. Source: spec §7. Verified by: `application::navigate` tests with a fake
shell adapter.

**NAV-004 (M) No association.** If Windows has no application associated with the script type,
then BuildPilot shall show the operating-system message.

### 3.12 Theme, Settings, Help/About (UI)

**UI-001 (M) Themes.** BuildPilot shall offer light and dark themes using the supplied
`light-mode.png` and `dark-mode.png` artwork, applied at once when chosen and persisted.
On first run the theme follows the Windows app theme. Source: spec §14.

**UI-002 (M) Contrast.** Every text colour in both themes shall reach a contrast ratio of at
least 4.5:1 against its background (WCAG 2.2 AA). Verified by: a unit test over the theme tokens.

**UI-003 (M) Help/About.** The About surface shall show the name, the version read from
`VERSION`, a one-sentence purpose, the repository URL, the GPL-3.0 licence and a Slint credit.

**UI-004 (M) Settings.** Settings shall hold only the theme choice (Light, Dark, Follow Windows)
plus the data folder path with an Open Folder button. Source: OQ-9.

**UI-005 (M) Theme toggle.** The toolbar shall carry a theme toggle using the light and dark
artwork; Settings shall show the same choice. Source: OQ-13.

### 3.13 Accessibility and keyboard (A11Y)

**A11Y-001 (M)** Every button shall have a tooltip and an accessible name. Source: spec §19.
**A11Y-002 (M)** Keyboard focus shall be visible on every focusable control and never drawn on a
container. Source: spec §19; house `/keeb` model.
**A11Y-003 (M)** Tab and Shift+Tab shall reach every control; Space and Enter shall activate the
focused control. Source: spec §19.
**A11Y-004 (M)** Disabled controls shall look disabled and state why in their tooltip.
Verified by (all four): Manual, with the Windows Narrator reading the names for A11Y-001.

### 3.14 Installer (INST)

The house `/installer` skill is loaded before any installer code is written.

**INST-001 (M)** The setup program shall install BuildPilot per user under
`%LOCALAPPDATA%\Programs\BuildPilot` without requesting elevation.
**INST-002 (M)** It shall create a Start menu shortcut and an Apps list entry offering Modify,
Repair and Uninstall.
**INST-003 (M)** It shall detect an existing install and offer update, downgrade, repair or
reinstall, according to the version comparison.
**INST-004 (M)** If BuildPilot is running, then the setup program shall say so and wait rather
than overwrite it.
**INST-005 (M)** Uninstall shall remove the program files and ask whether to keep the data
folder, defaulting to keep.
**INST-006 (M)** The application and the setup program shall carry the icon made from
`assets\application-icon.png`, which the taskbar and shortcuts also show. Source: spec §16.
Verified by (all): Manual build-and-launch; install policy logic unit-tested.

### 3.15 Non-functional requirements (NFR)

| ID | Requirement | Measured by |
|---|---|---|
| NFR-PERF-001 | The main window shall be visible within 1 second of launch with 50 configured operations. | Stopwatch on the reference machine, release build, 5 launches, worst case. |
| NFR-PERF-002 | See LIFE-004 and OUT-005. | As stated there. |
| NFR-CAP-001 | BuildPilot shall work with at least 50 configured operations and at least 8 concurrent runs. | `application::runner` test with 8 fake runs; Manual with 8 real scripts. |
| NFR-REL-001 | A panic on any thread BuildPilot owns shall be logged and shown to the operator, never end the application silently. | Test that panics a reader thread. |
| NFR-REL-002 | Every error shown shall name what failed and what the operator can do. | Review of each error string against spec §18. |
| NFR-OBS-001 | BuildPilot shall write a log file in the data folder, recording launches, exits, stops and errors, rotated at 1 MB with one previous file kept. | Inspection. |
| NFR-SEC-001 | BuildPilot shall make no network connection. | Inspection of dependencies; Manual with a network monitor. |
| NFR-SEC-002 | BuildPilot shall never write to, rename or delete a script. | Inspection: the shell and process adapters expose no write operation on scripts. |
| NFR-MAINT-001 | Domain and application code shall hold 100% line coverage (`cargo llvm-cov`), with the gate failing the build below it. | `test.ps1`. |
| NFR-MAINT-002 | `cargo fmt --check` and `cargo clippy -- -D warnings` shall pass. | `test.ps1`. |
| NFR-MAINT-003 | No source file shall exceed 400 lines, excluding build scripts. | Structural test. |
| NFR-MAINT-004 | README.md, ARCHITECTURE.md, TESTING.md and DEVELOPMENT.md shall exist and match the tree. | Handover gate. |
| NFR-PORT-001 | OS integration (process launch, tree termination, file association, Explorer reveal, paths) shall sit behind interfaces in infrastructure, so another platform means new adapters only. | Structural test on imports. |

**Non-claims.** BuildPilot does not sandbox the scripts it runs: they run with the operator's
full rights. It does not verify that a script is safe. It does not encrypt its config file.

### 3.16 Data requirements

The config file is `%APPDATA%\BuildPilot\buildpilot.json`, UTF-8 JSON, owned by BuildPilot alone.
Operations are held as an array in flight-deck order; the order is the array order. BuildPilot is
the single writer.

**DATA-001 (M) Single instance.** When BuildPilot is started while another instance is running for
the same Windows user, the new process shall bring the running instance's window forward and exit.
Source: OQ-10. Verified by: Manual.

---

## 4. Other requirements

### 4.1 Legal

GPL-3.0 for BuildPilot. Slint is used under GPLv3, credited in About. Third-party crate licences
are listed in the installer's licence page.

### 4.2 Internationalisation

English only in v1 (Appendix D). Paths and output with non-ASCII characters are handled (LCH-003,
OUT-008).

### 4.3 Risk register

Personal developer utility: a full FMEA is judged disproportionate. Named risks, highest first:

| ID | Risk | Mitigation |
|---|---|---|
| R-1 | Slint has no ready-made drag reordering for list rows. | **Retired 2026-09-27 by a throwaway spike (since removed).** A `TouchArea` on the handle keeps receiving `moved` after the pointer leaves it in any direction and receives the release, so the drag is built from it. Measured flaw to design out: a twitch of a few pixels retargeted the row; the target must change only when the pointer passes the middle of the next row. |
| R-2 | A plain Slint text view may not cope with 100,000 lines. | **Retired 2026-09-27 by a throwaway spike (since removed)**, on `ListView` (which instantiates only visible rows). Real launcher, `flood.ps1`, 50,000 lines in 0.73 to 0.78 s (faster than OUT-005's load): all shown, worst drain under 3.6 ms, no event-loop gap of 50 ms once output flowed. Open: one gap of 76 to 84 ms about 140 ms after each launch, before any output, cause not found (under the 100 ms target); 352 ms on the very first run, not reproduced. `spawn` measured at 48 to 51 ms, which counts against LIFE-004 when Run is clicked. |
| R-3 | `.bat`/`.cmd` argument quoting through `cmd.exe` is error-prone; Rust's standard library is believed to refuse arguments it cannot quote safely for batch files (to verify). | LCH-003 fixture test covers `.cmd` too; surface a refusal as a launch failure. |
| R-4 | A script that breaks away from its Job Object would survive Stop. | Accept for v1; STOP-003 reports survivors by PID. |

---

## Appendix A. Build order

Inside-out, following spec §22 but with the domain first:

1. Domain: operation, deck order, lifecycle state machine, launch plan table, output buffer,
   follow state, config model. All pure, all unit-tested.
2. Application: add, edit, remove, reorder, run, stop, navigate use cases over declared interfaces.
3. Infrastructure: config store, process adapter with Job Objects, shell adapter, icon discovery.
4. Spikes R-1 and R-2, then the Slint UI.
5. Theme, About, Settings; installer; documents.

Every use case is runnable from a test before any window exists.

## Appendix B. Decisions register

Every question raised against Draft 0.1 is closed. All were decided by Oliver on 2026-09-27,
accepting the proposed default in each case. No question is open.

| ID | Question | Decision |
|---|---|---|
| OQ-1 | Stop: hard terminate only? Or Ctrl+Break first with a grace period? | Hard terminate of the Job Object in v1; graceful stop is in Appendix D. |
| OQ-2 | Output retention cap and flood test figures (OUT-004, OUT-005). | 100,000 lines; 50,000 lines in 5 s; 100 ms click response. |
| OQ-3 | Row response time (LIFE-004). | 100 ms. |
| OQ-4 | Stop timeout before reporting failure (STOP-003). | 5 s. |
| OQ-5 | Default display name (ADD-003). | Folder name, space, file stem. |
| OQ-6 | Chosen icons: copy into the data folder? Or reference by path? | Copy chosen icons; reference discovered ones by path. |
| OQ-7 | Persist checkbox state? | No; transient. |
| OQ-8 | Arguments as a list or as one string? | List, one per line in the dialog. |
| OQ-9 | What does Settings hold in v1? | Theme (Light, Dark, Follow Windows) plus the data folder path with an Open Folder button. Nothing else until use earns it. |
| OQ-10 | Single instance per user? | Yes; a second launch brings the first window forward. |
| OQ-11 | Supported script types beyond `.ps1`, `.bat`, `.cmd`, `.exe`, `.com`? | None in v1 (see Appendix D). |
| OQ-12 | How Cargo.toml's version follows `VERSION` (CON-005). | The build script stamps Cargo.toml from `VERSION`; a structural test fails when they differ. |
| OQ-13 | Theme toggle location: toolbar buttons using the light/dark artwork? Settings? Both? | Toolbar toggle using the artwork; Settings mirrors it. |

## Appendix C. Traceability

Each requirement above carries its own Verified by line. The test names are planned; a test
that lands names its requirement ID in a comment above it, so traceability runs from code back to
this document as well as forwards.

## Appendix D. Won't this time (v1)

- Run All / Stop All and any action on checked rows.
- Graceful stop (Ctrl+Break then wait), per OQ-1.
- A progress protocol for scripts to report percentages (the model allows it: LIFE-006).
- Keeping output from runs before the latest.
- Script types beyond those in LCH-001, e.g. `.py`, `.sh`.
- Windows 10 testing; macOS and Linux builds.
- Localisation.
- Per-operation preferences (spec §12), until a real one is found.
- Environment variable overrides per operation.

## Appendix E. Amendments

**Amendment 1 (2026-09-27): batch files are launched directly.** Changes LCH-001, row `.bat`,
`.cmd`. Baseline 1.0 said to start `cmd.exe /d /c <script> <arguments>` by hand.

Reason, from the Rust standard library documentation (`std::process`, "Windows argument
splitting", read 2026-09-27): when `Command` is given a `.bat` file it runs it as `cmd.exe /c`
itself and "escapes the arguments according to `cmd.exe` rules"; an argument it cannot escape
safely makes the spawn fail with an error. Starting `cmd.exe` by hand would quote the arguments
by the standard C runtime rules instead, which `cmd.exe` does not follow: that is the fault risk
R-3 named. The same documentation says this batch handling "may be removed in the future", so an
infrastructure test launches a real `.cmd` fixture with spaced and quoted arguments; a change in
the standard library then fails that test rather than a build in the field.

**Amendment 2 (2026-09-27): output that is not UTF-8 is read in the OEM code page.** Changes
OUT-008, which said to decode as UTF-8 and replace invalid bytes with U+FFFD.

Reason, measured on the reference machine (ANSI code page 1252, OEM code page 850): started
hidden with piped output, `pwsh.exe`, `powershell.exe` and `cmd.exe` all wrote `é` as the single
byte 0x82, which is `é` in code page 850 and invalid as UTF-8. Decoding it as UTF-8 showed U+FFFD
in place of every accented character, including in paths. Tools such as Cargo write UTF-8, so no
single encoding fits every script. Each line is therefore decoded as UTF-8 when it is valid UTF-8
and in the OEM code page otherwise; a pure ASCII line reads the same either way.
