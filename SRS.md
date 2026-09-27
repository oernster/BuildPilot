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
- a dependency graph, pipeline or sequencing designer; an operation's own ordered steps (STEP-001)
  are the one sequence, with no branching, no condition and no link between operations
  (Amendment 4);
- an environment builder: BuildPilot never creates, installs into, upgrades or repairs a Python
  environment or any other; it uses one that already exists (ENV-007, Amendment 4);
- an "autopilot" that decides what to run;
- a CPU-core or thread scheduler for the underlying tools;
- a build-artifact manager;
- a terminal emulator (no cursor addressing, no interactive input to a running script);
- Run All / Stop All; no action of any kind on checked rows (§4.2 of the spec).

### 1.5 Definitions

| Term | Meaning |
|---|---|
| Operation | One configured build command: one or more steps, its working directory, its name and its icon. Persisted. |
| Step | One script or executable with its arguments. An operation runs its steps in order (Amendment 4). |
| Environment | A Python virtual environment that already exists: a direct subfolder of the working directory holding `pyvenv.cfg` and `Scripts\python.exe` (ENV-001). |
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

A new, standalone, local-only desktop application. It depends on nothing networked: its one
network request is the update check (UI-010), whose failure changes nothing. It interacts with
the operating system in five ways only: launching processes, terminating process trees, opening
a file with its associated application, revealing a file in Explorer, reading and writing its
own data folder.

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
(UUID, assigned at creation, never reused), display name, its steps in order (each a script path,
absolute, plus arguments), working directory (absolute), the chosen environment's folder name
where ENV-002 asked for one, icon reference, position in the flight deck. Globally it shall hold
the schema version, the theme preference and the operator's host table (HOST-001). Source: spec
§12; amended by Amendment 4. An operation saved by an earlier schema migrates to one step (CFG-009).
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

**ADD-001 (M) Add by browsing.** When the operator chooses to add a script (SCAN-002 offers a
folder as well; Amendment 5), BuildPilot shall open a file picker filtered to the supported types
(LCH-001). When a file is chosen, BuildPilot shall open the
operation dialog pre-filled with the derived defaults, the chosen file as its only step.
Source: spec §5.
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
operation dialog showing the name, the steps (each a script path with its arguments, STEP-008),
the working directory, the environment where ENV-002 applies and the icon, all editable. Edit
never touches a script. Source: spec §6; amended by Amendment 4.

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
Remove. Source: spec §4.1. For an operation with several steps, the script shown, opened and
located is the first step's, followed by `+N` for the rest (Amendment 4).

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

**ROW-008 (M) New rows slot in by name.** When an operation is added (Add or a folder scan), the
flight deck shall place it before the first row whose name sorts after its own, ignoring case; an
equal name goes after the existing one. A deck in name order therefore stays in order. Rows the
operator arranged (ROW-003, ROW-004) are never moved; a stored deck loads in its saved order.
Source: owner, 2026-09-27. Acceptance: adding Stellody, axisdb, PigeonPost, zebra then BuildPilot
to an empty deck lists axisdb, BuildPilot, PigeonPost, Stellody, zebra. Verified by:
`domain::deck` tests; `application::deck` test.

### 3.7 Launching (LCH)

**LCH-001 (M) Host by file type.** The launcher shall choose the host from one table keyed by
file extension:

| Extension | Launched as |
|---|---|
| `.ps1` | `pwsh.exe` if found on PATH, else `powershell.exe`; with `-NoProfile -NonInteractive -ExecutionPolicy Bypass -File <script> <arguments>` |
| `.bat`, `.cmd` | the script itself, with `<arguments>`; the standard library runs it through `cmd.exe` (Amendment 1) |
| `.exe`, `.com` | directly |
| `.py` | the environment's `Scripts\python.exe` with `<script> <arguments>`, activated as ENV-006 states (Amendment 4) |

An extension in the operator's host table is launched as HOST-002 states, ahead of this table.
A file of any other type is refused at Add with a message listing the supported types, the
operator's own included.
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

**UI-003 (M) Help/About.** The About surface shall show the icon, the name, the version read
from `VERSION`, a one-sentence purpose, the author, the copyright, the repository URL, the
GPL-3.0 licence and the open source credits of UI-009. Everything down to the licence stays
in place; only the credits scroll. Amended by Amendment 3.

**UI-006 (M) Help menu.** The toolbar's Help button shall open a menu holding, in order: Guide, a
separator, About BuildPilot, Licence, Check for Updates. Up and Down walk it, Enter or Space
chooses, Escape closes it and returns focus to the Help button. Source: house model (PigeonPost).

**UI-007 (M) Guide.** Guide shall explain every toolbar control and every row control, each as
its own icon with its name and one line on what it does; then the rules behind BuildPilot's
behaviour; then keyboard use at the foot. Verified by: a structural test that every toolbar and
row control's label is named in the Guide.

**UI-008 (M) Licence.** Licence shall show the text of `LICENSE` (GPL-3.0), opening at its
top. The licence texts of the crates built in are installed beside the program as
`THIRD-PARTY-NOTICES.txt` (4.1); Licence does not open them, as the owner ruled a second window
of licence text pointless.

**UI-009 (M) Open source credits.** The credits shall list every crate built into BuildPilot
with its version and licence, generated at build time; none is written by hand. Each licence
is shown by the name a reader knows it by ("Unicode License v3", "MIT or Apache 2.0"), not as
its SPDX identifier; a shipped crate stating an identifier with no name fails a test. The set is what
`cargo tree` says a release build compiles in for the Windows target (normal dependencies,
procedural macros left out); the licence of each comes from `cargo metadata`. Verified by: a
test that the list holds `slint` and `windows-sys` and holds no dev-only, build-only or
procedural-macro crate such as `tempfile`, `slint-build` or `syn`.

**UI-010 (M) Update check.** BuildPilot shall ask
`https://api.github.com/repos/oernster/BuildPilot/releases/latest` whether a newer release is
published: 3 seconds after the window opens, then every 24 hours while it runs; also whenever
Help > Check for Updates is chosen. The endpoint returns only a published release, never a draft or
a pre-release. A release is newer when its tag (an optional leading `v` removed) is a version
greater than `VERSION`; a tag that is not a version is never newer. When newer, BuildPilot shall
say "BuildPilot {latest} is available. You are running {current}." with Download, Skip This
Version and Later. Download opens the release's `.exe` asset, else the release page. Skip This
Version records the tag in the config file and that release never prompts unbidden again. An
automatic check that fails or finds nothing newer says nothing. A chosen check ignores the skip
and reports every outcome: the prompt, "You are running the latest version." or "The update
check could not reach GitHub. Please try again later." Each stage of the request (resolving, connecting, sending, receiving) waits at most 5 seconds;
the request is never retried. Source: house model; Amendment 3.

**UI-011 (M) Reading surfaces read themselves.** Guide, About, Licence and the installer's licence
page shall scroll their text when it overflows: still for 5 seconds on opening, then down 1 pixel
every 80 ms, still for 5 seconds at the end, back to the top at 15 pixels every 40 ms, still for 2
seconds, then again. Any scroll by the operator suspends the cycle for 2.5 seconds, after which it
carries on from where they left it; it is never switched off. The dialog's buttons stay in place
below the text. Source: house `/scroll` model.

**UI-012 (M) Scrolling shows itself.** Every surface that scrolls shall show a scroll bar
whenever its content overflows: at least 12 px wide, in theme colours whose thumb reaches 3:1
against its track and the surfaces beside it, draggable, with a click on the track moving a page.
The row list shall also count the rows wholly out of sight at each edge ("▼ 16 more below",
"▲ 3 more above"), each count scrolling a page that way when clicked. Rationale: Fluent's own bar
is a 2 px line until the pointer finds it (measured in Slint 1.18.1's
`widgets/fluent/scrollview.slint`), so with the output tray open 20 rows showed 4 and gave no
sign of the rest (owner, 2026-09-27). Verified by: `geometry` tests for the bar and the counts;
the UI-002 contrast test for the thumb; Manual for the look.

**UI-004 (M) Settings.** Settings shall hold only the theme choice (Light, Dark), the data folder
path with an Open Folder button and the operator's host table (HOST-001). The theme button shown
chosen is the theme in effect, including while the first-run default (UI-001) still follows the
system. Source: OQ-9; amended by Amendments 4 and 6.

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
| NFR-SEC-001 | BuildPilot shall make no network connection except the update check of UI-010, a single HTTPS GET to api.github.com carrying nothing about the operator or their operations. It shall use the HTTP client built into Windows (WinHTTP), adding no HTTP or TLS crate. Amended by Amendment 3. | Inspection of dependencies; Manual with a network monitor. |
| NFR-SEC-002 | BuildPilot shall never write to, rename or delete a script. | Inspection: the shell and process adapters expose no write operation on scripts. |
| NFR-MAINT-001 | Domain and application code shall hold 100% line and region coverage (`cargo llvm-cov`), with the gate failing the build below either. | `test.ps1`. |
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

### 3.17 Steps (STEP)

Added by Amendment 4. The case that asked for it, measured 2026-09-27: 15 repositories under the
owner's Development folder document running `python buildexe.py` and then
`python buildinstaller.py`; none of their `buildinstaller.py` files runs `buildexe.py` itself.

**STEP-001 (M) Ordered steps.** Each operation shall hold an ordered list of one or more steps,
each a script path plus its arguments. The working directory, name and icon stay per operation.
Acceptance: an operation in `C:\src\app` holds `buildexe.py` then `buildinstaller.py`.
Verified by: `domain::operation` tests.

**STEP-002 (M) Run in order.** When a step exits with code 0 and a later step exists, the run use
case shall start the next step. Verified by: `application::run` test with a two-step fake.

**STEP-003 (M) Stop on failure.** If a step exits with a non-zero code, then the run use case shall
start no further step; the run shall end Failed with that code and the step's number.
Acceptance: step 1 exits 3, so step 2 never starts and the row reads "Failed, step 1 exited
with code 3". Verified by: `application::run` test.

**STEP-004 (M) Stop ends the sequence.** When the operator activates Stop during a step, the run
use case shall stop that step's process tree (STOP-001) and start no further step; the run
shall end Stopped. Verified by: `application::run` test.

**STEP-005 (M) Checked before any step starts.** When Run is activated, the run use case shall
check every step's script (LCH-007) and resolve the environment (ENV-003) before step 1 starts.
If any check fails, then no step starts; the row shall name the failing step. Rationale: a
missing second script found only after a ten minute first step wastes the first.
Verified by: `application::run` test with step 2's script absent.

**STEP-006 (M) One output per run.** The output store shall hold every step of a run as one
output. Where an operation has more than one step, a line before each step shall name its number,
its script and the program started. Verified by: `application::output` test.

**STEP-007 (M) Step shown while running.** While an operation with more than one step is running,
its row shall show which step is running as `step n of N`. Verified by: `ui` test.

**STEP-008 (M) Editing steps.** The operation dialog shall let the operator add a step by
browsing, remove a step and move a step up or down. It shall refuse to remove the only step.
Verified by: `domain::operation` tests; Manual for the dialog.

### 3.18 Python environments (ENV)

Added by Amendment 4. BuildPilot uses an environment that already exists; it never makes one.
What activation does was read from `venv\Scripts\activate.bat` on the reference machine on
2026-09-27: it sets `VIRTUAL_ENV`, clears `PYTHONHOME` and puts `Scripts` first on `PATH`; the
rest changes only an interactive prompt.

**ENV-001 (M) Discovery.** The environment finder shall treat a direct subfolder of the
operation's working directory as an environment when it holds both `pyvenv.cfg` and
`Scripts\python.exe`, whatever the folder's name.
Acceptance: in `C:\src\app`, `venv` and `.venv` holding both files are found; `tools` holding
`Scripts\python.exe` without `pyvenv.cfg` is not. Verified by: `infrastructure` test in a
temporary folder.

**ENV-002 (M) Choice when several.** While the working directory holds more than one environment
and the operation has a `.py` or `.ps1` step, the operation dialog shall list them and refuse to save until
one is chosen. Where exactly one is named `venv` or `.venv`, the dialog shall preselect it. The
choice is saved by folder name. Measured case: AxisDB holds `venv` and `venv_smoke`.
Verified by: `domain` test for the preselection; Manual for the dialog.

**ENV-003 (M) Resolution at Run.** When a run of an operation with a `.py` or `.ps1` step starts,
the run use case shall use the environment the operation names; where it names none, the only
one found.
Verified by: `application::run` tests.

**ENV-004 (M) No environment.** If a `.py` step has no environment at Run (none found; the named
one gone), then no step shall start; the row shall show Failed naming the working directory
searched and saying BuildPilot uses an existing environment and does not create one. A `.ps1` step with no environment found runs without one
(deactivated as ENV-009 states), so a PowerShell build with no Python keeps working.
Verified by: `application::run` tests for both kinds.

**ENV-005 (M) Several found, none chosen.** If Run finds several environments where the operation
names none (one appeared after it was saved), then no step shall start; the row shall name the
environments found and say to choose one in Edit. Verified by: `application::run` test.

**ENV-006 (M) Activation.** When the launcher starts a `.py` or `.ps1` step with an environment,
it shall give that process the variables `activate.bat` would give it, applied to the
deactivated variables of ENV-009: `VIRTUAL_ENV` set to the environment folder,
`VIRTUAL_ENV_PROMPT` set to its folder name, `_OLD_VIRTUAL_PATH` set to the deactivated `PATH`,
`_OLD_VIRTUAL_PYTHONHOME` set to `PYTHONHOME` where that was set, `PYTHONHOME` removed and
`<environment>\Scripts` put first on `PATH`. For OUT-001 and OUT-008 it shall also set
`PYTHONUNBUFFERED=1` and `PYTHONIOENCODING=utf-8` (R-5). A `.py` step's program is
`<environment>\Scripts\python.exe` with the script and its arguments; a `.ps1` step's program is
the PowerShell host of LCH-001. A script that activates another environment itself then
deactivates this one correctly, as the `_OLD_` variables are present. BuildPilot's own
environment is unchanged. `.bat` and `.cmd` steps are not activated (OQ-18).
Acceptance: given `C:\src\app\venv`, a fixture `.py` printing `sys.executable` and
`VIRTUAL_ENV` prints `C:\src\app\venv\Scripts\python.exe` and `C:\src\app\venv`; its line
`café`, printed 2 s before it exits, reaches the tray as `café` before it exits.
Verified by: `infrastructure::process` test with a real environment made by the test setup.

**ENV-007 (M) Never builds an environment.** BuildPilot shall not create, install into, upgrade or
repair any environment; it runs no `venv`, `virtualenv`, `pip`, `uv`, `poetry` or `conda` command
of its own. Source: owner, 2026-09-27. Verified by: inspection; a structural test that the source
names none of those commands.

**ENV-008 (M) Environment named.** When a `.py` or `.ps1` step starts with an environment, the
output tray's line for it shall name the environment folder (plus the interpreter path for `.py`),
including for a single-step operation. Verified by: `application::output` test.

**ENV-009 (M) Deactivate first, every step.** Before the launcher gives any step its variables,
activated or not, it shall undo an activation inherited from BuildPilot's own environment, as
`deactivate.bat` does: `PATH` taken from `_OLD_VIRTUAL_PATH` where that is set; `PYTHONHOME`
taken from `_OLD_VIRTUAL_PYTHONHOME` where that is set; `VIRTUAL_ENV`, `VIRTUAL_ENV_PROMPT`,
`_OLD_VIRTUAL_PATH`, `_OLD_VIRTUAL_PYTHONHOME` and `_OLD_VIRTUAL_PROMPT` removed. Source: owner,
2026-09-27 ("environments may be wildly different and conflict"); `deactivate.bat` and
`Activate.ps1` read on the reference machine the same day.
Rationale: every step is its own process built from BuildPilot's environment (read in
`infrastructure::launcher`), so no step inherits another step's activation; the one route for a
foreign environment is a BuildPilot started from a shell where one was active.
Acceptance: BuildPilot started with `VIRTUAL_ENV=C:\other\venv`, `PATH` beginning
`C:\other\venv\Scripts` and `_OLD_VIRTUAL_PATH=C:\Windows`; a step in `C:\src\app` with
`C:\src\app\venv` sees `VIRTUAL_ENV=C:\src\app\venv` and `PATH=C:\src\app\venv\Scripts;C:\Windows`;
a `.bat` step sees no `VIRTUAL_ENV` and `PATH=C:\Windows`.
Verified by: `domain` tests over the variable transformation; `infrastructure::process` test
with a fixture printing its variables.

**ENV-010 (M) No stray environment on PATH.** After ENV-009, the launcher shall remove from the
step's `PATH` every entry that is the `Scripts` folder of an environment (its parent holds
`pyvenv.cfg`), other than the step's own. Rationale: ENV-009 cannot see two cases. One is an
activation that left no `_OLD_VIRTUAL_PATH` behind; the other is an environment added to the
Windows `PATH` by hand. Either would still put a foreign `python.exe` ahead of the system one. Verified by: `infrastructure` test in a temporary
folder.

### 3.19 Operator hosts (HOST)

Added by Amendment 4, for script types BuildPilot has no built-in rule for, such as `.rb`.

**HOST-001 (S) Host table.** Settings shall hold a table in which each row maps one file
extension to a program and its leading arguments. An extension is letters and digits, stored
lower case without its dot; it has at most one row. A program is a full path or a bare file name
found on PATH. A row that breaks these rules is refused when added, naming what to change; one
read from a hand-edited file is dropped. Removing a row never removes an operation: a step of
that type still loads; its run fails before step 1 with a message naming Settings.
Verified by: `infrastructure::config_store` round-trip and load tests; `domain::host` row tests;
`application::hosts` tests.

**HOST-002 (S) Operator first.** When a step's extension has a row in the operator's host table,
the launcher shall start that row's program with its leading arguments, then the script path, then
the step's arguments, in place of the built-in rule of LCH-001. Acceptance: `.rb` mapped to
`C:\Ruby33\bin\ruby.exe` runs `build.rb --release` as `ruby.exe build.rb --release`. A `.py` row
replaces environment activation for `.py` entirely. Verified by: `domain::launch_plan` and
`domain::host` tests; `application::hosts` acceptance test.

**HOST-003 (S) Picker follows the table.** The Add and step pickers shall offer the operator's
extensions beside the built-in ones. Verified by: `domain::host` test.

A host program that cannot be started is a launch failure (LCH-009).

### 3.20 Adding from a folder (SCAN)

Added by Amendment 5. Measured 2026-09-27 over the owner's Development folder: of 30 projects
with a build script, 12 hold `build.ps1` (Go and Rust, none with an environment); 14 hold
`buildexe.py` and `buildinstaller.py` (each with one environment); none holds both. The other 4
match neither pattern, one of them in part (EDColonisationAsst: `buildinstaller.py` without
`buildexe.py`). A scan proposes; it never adds or runs anything the operator has not seen, so it
is not the "autopilot" §1.4 rules out.

**SCAN-001 (M) Patterns.** The folder scanner shall take its patterns from one ordered list, each
an ordered list of file names that become steps:

| Order | Pattern | Steps |
|---|---|---|
| 1 | Go, Rust and other PowerShell builds | `build.ps1` |
| 2 | Python builds | `buildexe.py`, then `buildinstaller.py` |

A further pattern is one entry with no change elsewhere (as ICON-003). File names compare without
regard to case. Verified by: `domain` test over the list.

**SCAN-002 (M) Add takes a script or a folder.** When the operator activates Add, BuildPilot shall
offer to choose a script (ADD-001) or a folder. Verified by: Manual.

**SCAN-003 (M) Best pattern.** When a folder is scanned, the folder scanner shall choose the
pattern with the most of its files present directly in that folder (the earlier pattern on a
tie). It shall propose those present files as steps in the pattern's order. A pattern with none of its
files present is never chosen.
Acceptance: `buildexe.py` and `buildinstaller.py` present gives both steps in that order;
`buildinstaller.py` alone (EDColonisationAsst) gives that one step; `build.ps1` gives one step;
a folder holding only `Makefile` gives no proposal. Verified by: `domain` tests.

**SCAN-004 (M) One folder.** When the chosen folder has a proposal, BuildPilot shall open the
operation dialog pre-filled with it: the proposed steps, the folder as working directory, the
folder's name as the name, the environment preselected as ENV-002 states and the icon found as
ICON-001 states. Nothing is added until the operator confirms. Verified by: `application` test;
Manual for the dialog.

**SCAN-005 (S) A parent of several projects.** When the chosen folder has no proposal but folders
directly inside it do, BuildPilot shall list those folders with each one's proposed steps and a
tick box, then add an operation (as SCAN-004 fills it) for each ticked folder when the operator
confirms. Acceptance: choosing `C:\Users\Oliver\Development` lists 27 folders (12 PowerShell,
14 Python, EDColonisationAsst in part) and adds the ticked ones in the order listed.
Verified by: `application` test; Manual for the list.

**SCAN-006 (S) Unticked where a choice is owed.** In the SCAN-005 list, a folder shall start unticked
with a note saying why when its proposed steps and working directory match an operation already
on the deck; likewise when ENV-002 cannot preselect its environment. A folder already on the deck
cannot be ticked at all: its tick box is shown dimmed and is no stop on the ring. Rationale:
ADD-006 allows a duplicate through Add, one at a time; a bulk add never makes one (owner,
2026-09-27). An environment choice needs the single-folder dialog. Verified by: `application`
test; `keyboard` test for the ring; Manual for the dimmed box.

**SCAN-007 (M) Nothing found.** If neither the folder nor any folder directly inside it has a
proposal, then BuildPilot shall say which file names it looked for and open the script picker in
that folder. Verified by: `application` test; Manual.

**SCAN-009 (M) A partial match is flagged.** Where a proposal holds fewer steps than its pattern
(SCAN-003), BuildPilot shall say so in words naming the missing files and the steps that will
run, e.g. "buildexe.py was not found: only buildinstaller.py will run". In the SCAN-005 list that
folder's row shall also take the warning background and pulse twice when the list opens, then
stay on the warning background; in the SCAN-004 dialog the words sit above the steps on the same
background. Source: owner, 2026-09-27, so the EDColonisationAsst case is obvious rather than
hidden. The pulse is two cycles of 1 s each, below the three flashes per second of WCAG 2.2
success criterion 2.3.1; the words carry the meaning, so colour and motion are never the only
signal (LIFE-005, A11Y). The warning background is a theme token whose text reaches 4.5:1 in
both themes (UI-002). Verified by: `domain` test for the words; the UI-002 contrast test for the
token; Manual for the pulse.

**SCAN-008 (M) Depth.** The folder scanner shall read only the chosen folder plus (for
SCAN-005) the folders directly inside it; never deeper. Rationale: a deeper search would find build scripts
of dependencies and tools inside a project. Verified by: `infrastructure` test.

---

## 4. Other requirements

### 4.1 Legal

GPL-3.0 for BuildPilot. Slint is used under GPLv3. Every crate built in is credited in About with
its licence (UI-009); their licence texts are in `THIRD-PARTY-NOTICES.txt`, generated by
`build.ps1` and installed beside the program. The installer's licence page lists the crates.

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
| R-5 | Python writing to a pipe holds its output back and encodes it in the ANSI code page. | **Retired 2026-09-27 by measurement.** A venv's Python 3 started with piped output and no window printed three lines 2 s apart: all three arrived together at 4.06 s; with `PYTHONUNBUFFERED=1` at 0.04, 2.04 and 4.04 s. `é` arrived as the single byte 0xE9 (stdout encoding cp1252), which OUT-008's OEM fallback (code page 850) reads as `Ú`; with `PYTHONIOENCODING=utf-8` it arrived as C3 A9. ENV-006 sets both. |

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
accepting the proposed default in each case. OQ-14 to OQ-18 arose with Amendment 4, OQ-19
with Amendment 5 and OQ-20 ahead of the macOS and Linux port; all are decided. No question is open.

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
| OQ-14 | How is a sequence such as `buildexe.py` then `buildinstaller.py` expressed? | Ordered steps inside one operation, stopping on the first failure (STEP). Decided 2026-09-27. |
| OQ-15 | Several environments in one working directory? | The operation dialog asks; the choice is saved (ENV-002). Decided 2026-09-27. |
| OQ-16 | No environment for a `.py` step? | Refuse to run; never fall back to a Python on PATH and never create one (ENV-004, ENV-007). Decided 2026-09-27. |
| OQ-17 | Where does a host for another script type live? | One table in Settings keyed by extension (HOST). Decided 2026-09-27. |
| OQ-20 | macOS and Linux: how delivered, which build scripts, where checked, when? | DMG and Flatpak as the owner's other apps; the setup program stays Windows-only. Patterns: macOS runs `builddmg.py` (Python) or `builddmg.sh`; Linux runs `cleanup_flatpak.sh` or `clean_flatpak.sh`, then `build_flatpak.sh`. Checked on the owner's own Mac and Linux machines. Started once the Windows work is finished; its requirements become an amendment then. Decided 2026-09-27. |
| OQ-19 | Add from a folder: how offered, how far, partial matches, where the patterns live? | The one Add button takes a script or a folder; a parent folder may be scanned for several projects at once; a partial match proposes the files present; the patterns are a built-in list (SCAN). Decided 2026-09-27. |
| OQ-18 | Should a `.ps1`, `.bat` or `.cmd` step also run inside the environment, for a script that calls `python` itself? | `.ps1` steps yes, running without one when none is found; `.bat` and `.cmd` no. Every step is deactivated first (ENV-009). Decided 2026-09-27. |

## Appendix C. Traceability

Each requirement above carries its own Verified by line. The test names are planned; a test
that lands names its requirement ID in a comment above it, so traceability runs from code back to
this document as well as forwards.

## Appendix D. Won't this time (v1)

- Run All / Stop All and any action on checked rows.
- Graceful stop (Ctrl+Break then wait), per OQ-1.
- A progress protocol for scripts to report percentages (the model allows it: LIFE-006).
- Keeping output from runs before the latest.
- Built-in script types beyond those in LCH-001, e.g. `.sh`; the operator's host table
  (HOST-001) covers them instead.
- A working directory per step; environments other than Python virtual environments.
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

**Amendment 3 (2026-09-27): Help becomes a menu; an update check is added.** Changes UI-003 and
NFR-SEC-001; adds UI-006 to UI-011. Baseline 1.0 had a Help and About button opening About
directly, credited Slint alone and made no network connection at all.

Reason: the owner's review of the first installed build asked for the house Help menu (Guide,
About, Licence, Check for Updates) and for reading surfaces that scroll themselves. The owner also ruled
that the house update check (automatic at launch and daily, plus on demand) is wanted even though
it means one request to GitHub. The About credit list was measured against the shipped graph:
292 crates under 25 licence expressions, where About named one. The update check uses WinHTTP,
built into Windows, so BuildPilot still carries no HTTP or TLS crate.

**Amendment 4 (2026-09-27): steps, Python environments and operator hosts.** Changes §1.4, §1.5,
ADD-001, CFG-003, LCH-001, UI-004 and Appendix D; adds STEP-001 to STEP-008, ENV-001 to ENV-010,
HOST-001 to HOST-003, R-5 and OQ-14 to OQ-18. Baseline 1.0 launched one script per operation and
listed `.py` as won't-this-time.

Reason: the owner's Python projects need an existing virtual environment active and two scripts
run in order (`buildexe.py`, then `buildinstaller.py`). Measured on the reference machine: 31
projects hold an environment, each a `venv` folder with `pyvenv.cfg`; one holds two; 15 document
the two-script order. The owner ruled that BuildPilot detects and activates an environment that
exists but never builds one. It also ruled that BuildPilot refuses a `.py` step with no environment rather than
guessing at a Python on PATH. `.ps1` steps are activated as well, so a PowerShell build that
calls Python finds its environment. Every step first undoes any activation BuildPilot inherited,
since two environments may conflict. A host table in Settings covers other script types without a
per-operation field. Existing config files migrate each operation to a single step.

**Amendment 5 (2026-09-27): adding from a folder.** Changes ADD-001 (Add now also takes a folder);
adds SCAN-001 to SCAN-009 and OQ-19. Baseline 1.0 added one chosen script at a time.

Reason: the owner asked for a folder's build to be recognised rather than assembled by hand. The
two patterns are the two measured across the owner's projects (3.20). The owner chose a built-in
pattern list, a partial match proposing what exists and a parent folder scanned for several
projects at once. Every proposal is shown before anything is added.

**Amendment 6 (2026-09-27): Settings no longer offers Follow Windows.** Changes UI-004 and OQ-9's
answer. Baseline 1.0 offered Light, Dark and Follow Windows.

Reason: the owner intends BuildPilot for macOS and Linux as well, where a button named after
Windows does not belong. The first run still starts in the system's theme (UI-001); choosing
Light or Dark replaces that for good.
