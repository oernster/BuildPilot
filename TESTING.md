# Testing

What is tested, what is not and why the line falls where it does.

This document exists because a coverage figure on its own is a number without a claim behind it.
Every figure here was measured on 2026-09-27, when it was written.

## The standard

**A floor is a measurement, never an aspiration.** A gate set to a number the code does not
reach teaches people to lower it.

**A gap is named or it is closed.** Where something cannot be tested, this document says what it
is and what stops it.

## The gate

`test.ps1` at the repository root runs three steps, cheapest first. It stops at the first one
that fails:

1. `cargo fmt --check`: the formatting.
2. `cargo clippy --all-targets -- -D warnings`: every lint, with warnings as errors.
3. `cargo llvm-cov` over every test, failing below 100% of lines or of regions in the floor below.

```powershell
./test.ps1
```

```powershell
$LASTEXITCODE
```

Read the exit code rather than the last line of output: `0` means every step passed. With
`-Html` the script also opens the coverage report.

## The coverage floor

| Scope | Floor | Why |
|---|---|---|
| `src/domain`, `src/application` and `src/setup` | 100% of lines and of regions | The correctness core and the setup program's policy: no files, no processes, no clock, no window, so every line and every branch is reachable from a test. |
| `src/infrastructure`, `src/ui`, `src/main.rs`, `src/bin` | measured, not floored | They need real processes, files, Explorer, the registry or a window; a number over them would mean little. Their behaviour is covered by the suites below and the checks by hand. |

Every line and every region in the floor is covered: 3,060 regions over 2,124 lines. A region is a branch as
well as a line, so an untaken `?` fails the gate even when its line ran. Where a branch could
not be taken in production it was deleted rather than tested (an operation looked up twice, a
refusal repeated after it had already been made). The region floor was proved by planting a
one-line `if` whose branch never runs: lines stayed at 100% and the gate failed.

## What the suites prove

339 tests, counted from the source.

| Suite | Tests | What it proves |
|---|---|---|
| `tests/domain` | 135 | The rules: validation, steps, folders compared as Windows compares them, deck order and slotting in by name, the run state machine, launch plans per kind of script, the host table, environment discovery with the deactivate and activate variables, the scan patterns and proposals, line assembly and decoding, the output buffer's caps, follow state, selection, preferences, elapsed-time wording, reading the credits and naming their licences, the self-reading cycle tick by tick. |
| `tests/application` | 91 | Every use case through `App` against hand-written fake ports: adding, editing, removing, reordering (including during a run), running steps in order, one build per folder, running the ticked rows, stopping, environments, hosts, folder scans, stale events, notices, refusals and what is logged; the update check's decisions and wording, the saved skip. |
| `tests/infrastructure` | 58 | The real machine: the settings file in a temporary folder (including unreadable and newer files), icons, PowerShell detection, real processes started and stopped, the single-instance event, the log's rotation, a reader thread that panics, the known folders, finding and ending a process by name and clearing setup's stale copy; the donation address; the generated credits against the shipped graph, with a readable name for every licence; reading GitHub's release answer as foreign input. |
| `tests/keyboard` | 14 | The keyboard ring, driven headless by real key events through Slint's own focus handling: neutral start, Tab and Shift+Tab with wrap, Left and Right, the rows stop, dialogs opening on their first control, owning the ring, closing on Escape and handing focus back, the output stop only while it overflows, the Help menu walked with Up and Down, a folder already on the deck off the ring. |
| `tests/ui.rs` | 11 | What rows and the tray say, without a window. |
| `tests/structural.rs` | 8 | The invariants in [ARCHITECTURE.md](ARCHITECTURE.md), by reading the source. |
| `tests/guide.rs` | 1 | Every toolbar and row control has a Guide entry, one per state where its label changes. |
| `tests/geometry` | 9 | Where Slint laid things out, headless: every toolbar tooltip inside the window, row tooltips not clipped by the list, a button's icon level with its words and centred with them, every credit's licence inside About's pane with the real list loaded, About's header above the scrolling credits, Licence opening at the top of its text, the scroll bars and the counts of rows out of sight, the selected row kept in sight when the tray opens. |
| `tests/setup.rs` | 6 | The setup program's policy: version order, the route for each installed state, each plan's steps and weights, every screen's words. |
| `tests/contrast.rs` | 3 | Text at 4.5:1 and rings at 3:1 against their surfaces, in both themes, read from `ui/theme.slint`. |
| `tests/assets.rs` | 1 | Every image a Slint file names is a small copy in `assets/ui`, none over 256 pixels. |
| `tests/frame_counter.rs` | 1 | Each executable removes Slint's frame counter variable before anything else. |
| `tests/site.rs` | 1 | The site under `docs/` names the version in `VERSION` and no other. |

## How each layer is tested

| Layer | Kind of test | Touches |
|---|---|---|
| Domain | Pure unit tests | Nothing |
| Application | Use cases against fake ports | Nothing |
| Infrastructure | Integration tests | Temporary folders, real hidden processes, a named event |
| UI | Wording tests without a window; keyboard tests on Slint's headless backend | Nothing |
| Structure | Source scans | The source tree, read only |

No mocking library is used: every double is a hand-written fake in `tests/application/fakes.rs`.

**No test writes to the machine it runs on.** Every file test works under a temporary folder.
The process tests start hidden PowerShell and batch fixtures from `tests/fixtures`. A tree still
running when its handle is dropped is ended by its job object. A grandchild a test watches is
killed with `taskkill` afterwards, even when the test fails part way.

## What is not tested and why

- **Explorer.** Opening a script, revealing it and opening the data folder call the Windows
  shell, which would open windows on the machine running the tests.
- **What the rings look like.** The keyboard tests read which control holds focus, not what is
  painted. The colours are held by the contrast test; how they look on screen is a check by
  hand.
- **Other code pages.** The OEM decoding test assumes a code page where byte `0x82` is `é`:
  850 (measured on the reference machine) or 437.

## Checked by hand

These need a real desktop and a real person; the SRS marks them Manual.

| Check | How |
|---|---|
| CFG-007 first run | Start with an empty data folder: the empty state shows with its Add button. |
| ADD-001 add by browsing | Add opens the file picker, filtered to `.ps1`, `.bat`, `.cmd` and `.exe`. |
| ICON-006 no distortion | Every icon and asset keeps its proportions at every size. |
| REM-001 confirm before remove | Remove asks first, naming the operation and saying the script is not touched; Cancel is focused. |
| ROW-002 stable layout | Every control keeps its place in every state. |
| LCH-004 no console window | Running a `.ps1`, `.bat` and `.exe` opens no console. |
| OUT-006 auto-follow | The tray follows new output until scrolled up; Jump to latest resumes it. |
| A11Y-001 names | Narrator reads every control's name. |
| A11Y-002 focus visible | Green ring on hover or focus, red ring on a disabled control, none at rest, never round a pane. |
| A11Y-003 keyboard reach | Tab, Shift+Tab, Left and Right reach every control; Space and Enter activate. |
| A11Y-004 disabled | A disabled control looks disabled and its tooltip says why. |
| Escape closes a dialog | Known limitation, not pursued: on the real window Escape leaves dialogs open, although the headless suite passes it. Close a dialog with its own button. |
| DATA-001 single instance | Starting BuildPilot again brings the open window forward, restoring it if minimised. |
| INST-001 to INST-005 | Build with `build.ps1`, then install, run setup again to repair, then uninstall from the Apps list: check the folder, the Start menu shortcut, the Apps entry and that uninstall removes them. Driven through UI Automation on 2026-09-27; each step's time is in setup's log. |
| INST-004 running | Install with BuildPilot open: setup says so and offers to close it. Driven through UI Automation on 2026-09-27: the Running screen shows, Cancel returns to Install with BuildPilot still open, Close it and continue ends it and installs. |
| UI-006, UI-007 Help | The menu drops from Help; Guide, About and Licence open, each with its buttons held still below the text. |
| UI-011 reading | Guide, About, Licence and setup's licence page hold still 5 seconds, read down slowly, hold, rewind fast; a wheel or a key holds them 2.5 seconds, then they carry on. |
| UI-010 update check | With the network watched: one request to api.github.com 3 seconds after opening; Help > Check for Updates says one of the three answers; with the network off it says GitHub could not be reached. |
| INST-006 icon | The taskbar, the shortcuts and both executables show the application icon. |
| CFG-010 layout | Move and resize the window, resize the tray, restart: all come back as left. |
| ICON-005 missing icon | Delete a chosen icon's file, restart: the row shows the placeholder with a marker naming the path. |
| STEP-008, ENV-002 dialog | The operation dialog adds, removes and moves steps, refuses to remove the last and lists the environments when there are several. |
| SCAN-002 to SCAN-009 folders | Add a project folder, then a parent of several: the proposals, the ticks, a folder already on the deck dimmed, a partial match on the warning background pulsing twice. |
| OUT-007 outcome | A run that logs to stderr shows plain lines; its closing line is green on success, red on failure, muted when stopped. |
| UI-012 scroll bars | With the tray open and 20 rows, the bar shows, drags and pages; the counts read how many rows are out of sight. |
| UI-013 drag | With 20 rows, a drag follows the pointer smoothly. |
| UI-014 donate | The toolbar's drink button opens the donation page in the browser. |

## Running part of the suite

Every command below is PowerShell, run from the repository root.

One suite:

```powershell
cargo test --test keyboard
```

One test by name:

```powershell
cargo test --test infrastructure a_second_claim_finds_the_first_running
```

A count of the tests without running them:

```powershell
cargo test -- --list
```

## Keeping this honest

**Prove a new guard bites.** A test that has never been seen to fail is not yet a guard. Plant
the violation, read the exit code, then restore the file.

**Re-measure before quoting.** Every figure in this document was measured when it was written;
change the code and the figures are measured again, never carried forward.

**Read the exit code, never the last line.** The coverage table prints last, so the end of the
output says nothing about whether a test failed. The exit code is the only answer.

## See also

- [DEVELOPMENT.md](DEVELOPMENT.md) for the tools and the build.
- [ARCHITECTURE.md](ARCHITECTURE.md) for the invariants the structural tests enforce.
