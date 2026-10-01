# Testing

What is tested, what is not and why the line falls where it does.

This document exists because a coverage figure on its own is a number without a claim behind it.
Every figure here was measured on 2026-10-01, when it was last revised.

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

Every line and every region in the floor is covered: 3,405 regions over 2,320 lines. A region is a
branch as well as a line, so an untaken `?` fails the gate even when its line ran. Where a branch
could not be taken in production it was deleted rather than tested (an operation looked up twice, a
refusal repeated after it had already been made). The region floor was proved by planting a one-line
`if` whose branch never runs: lines stayed at 100% and the gate failed.

## What the suites prove

373 tests, counted from the gate's own run.

| Suite | Tests | What it proves |
|---|---|---|
| `tests/domain` | 148 | The rules: validation, steps, folders compared as Windows compares them, deck order and slotting in by name, the run state machine, launch plans per kind of script, where a project's installer is found and when it may be launched, the host table, environment discovery with the deactivate and activate variables, the scan patterns and proposals, line assembly and decoding (tabs expanded, other control characters dropped), the output buffer's caps, follow state, selection, preferences, elapsed-time wording, the typical build time as the median of the last five successes, reading the credits and naming their licences, the self-reading cycle tick by tick. |
| `tests/application` | 105 | Every use case through `App` against hand-written fake ports: adding, editing, removing, reordering (including during a run), running steps in order, one build per folder, running the ticked rows, stopping, run times recorded and saved on success alone, loaded at start and forgotten on remove, launching an installer (found, set, missing, held back by a run, a stop or a failure), environments, hosts, folder scans, stale events, notices, refusals and what is logged; the update check's decisions and wording, the saved skip. |
| `tests/infrastructure` | 63 | The real machine: the settings file in a temporary folder (including unreadable and newer files), the run times file (round trip, damaged, unreadable, unwritable), icons, PowerShell detection, real processes started and stopped, the single-instance event, the log's rotation, a reader thread that panics, the known folders, finding and ending a process by name and clearing setup's stale copy; the donation address; the generated credits against the shipped graph, with a readable name for every licence; reading GitHub's release answer as foreign input. |
| `tests/keyboard` | 14 | The keyboard ring, driven headless by real key events through Slint's own focus handling: neutral start, Tab and Shift+Tab with wrap, Left and Right, the rows stop, dialogs opening on their first control, owning the ring, closing on Escape and handing focus back, the output stop only while it overflows, the Help menu walked with Up and Down, a folder already on the deck off the ring. |
| `tests/ui.rs` | 13 | What rows and the tray say, without a window, including the typical build time in every state. |
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

- **Explorer.** Opening a script, revealing it, opening the data folder and launching an
  installer call the Windows shell, which would open windows (or start a real setup program) on
  the machine running the tests. The application tests drive each through a fake shell instead.
- **What the rings look like.** The keyboard tests read which control holds focus, not what is
  painted. The colours are held by the contrast test; how they look on screen is a check by
  hand.
- **Escape on the real window.** The headless keyboard suite passes Escape closing a dialog;
  on the real window Escape leaves dialogs open. A known limitation, not pursued: close a
  dialog with its own button.
- **Other code pages.** The OEM decoding test assumes a code page where byte `0x82` is `é`:
  850 (measured on the reference machine) or 437.

## Checked by hand

What only a real desktop and a real person can settle (how things look, Narrator, the file
picker, the installer run end to end) is checked by hand in a real build. The SRS marks each
such requirement Manual.

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
