# Development

How to go from a bare Windows machine to a running build of BuildPilot.

Every command here is PowerShell, run from the repository root, one command per block, meant to
be pasted as it is. `README.md` is for somebody using the application; this is for somebody
building it.

## Tools

| Tool | Version | Why |
|---|---|---|
| Rust, through rustup | 1.98.1, target `x86_64-pc-windows-msvc` | The compiler, cargo, rustfmt and clippy. |
| Microsoft C++ Build Tools | any current release | The MSVC target links with Microsoft's linker. |
| rustup's `llvm-tools` component | matches the compiler | The coverage measurement. |
| `cargo-llvm-cov` | 0.9.1 | Runs the tests under coverage for the gate. |
| Git | any | The source. |

Rust comes from [rustup.rs](https://rustup.rs); the C++ Build Tools from Microsoft's Visual
Studio downloads page, with the "Desktop development with C++" workload. Then:

```powershell
rustup component add llvm-tools
```

```powershell
cargo install cargo-llvm-cov
```

Check each is there before starting; a missing one fails the gate part way rather than at the
start:

```powershell
rustc --version
```

```powershell
cargo llvm-cov --version
```

## Getting the source

```powershell
git clone https://github.com/oernster/BuildPilot.git
```

```powershell
cd BuildPilot
```

The first build downloads the crates in `Cargo.lock`; after that nothing needs the network.

## Building

```powershell
./build.ps1
```

It does six things in order and stops at the first failure:

1. Reads `VERSION`, refusing anything that is not three numbers joined by dots.
2. Runs `stamp_version.ps1`, which writes that version into `Cargo.toml` (the package's own
   line only) and into every version token on the site under `docs/`.
3. Runs the gate, `test.ps1`. There is no switch to skip it: a gate that can be skipped is a
   gate that is skipped on the day it would have caught something.
4. Builds the application.
5. Builds the setup program with the application inside it, through the `BUILDPILOT_PAYLOAD`
   variable `build.rs` reads.
6. Copies the setup program into `dist\`.

| Output | What it is |
|---|---|
| `target\release\buildpilot.exe` | The application, one file with its images inside |
| `dist\BuildPilotSetup.exe` | The setup program, carrying the application |

For the application alone, skipping steps 5 and 6:

```powershell
./build.ps1 -SkipInstaller
```

`build.rs` hands `VERSION` to the code, compiles the Slint interface and puts the icon plus the
name and version Windows shows on every executable. Through `build_credits.rs` it also asks
`cargo tree` and `cargo metadata` (offline, against `Cargo.lock`) which crates a release build
compiles in, then writes the open source credits About lists and `THIRD-PARTY-NOTICES.txt`, the
licence texts setup installs beside the program. Outside a release build it keeps the Slint
element names, so the geometry tests can find elements and measure where they landed. A setup
program built without a payload (by `cargo build` alone) refuses to run and says why. Neither
executable is signed.

## Installing what you built

Run `dist\BuildPilotSetup.exe`. Everything it writes is for your own account:
`%LOCALAPPDATA%\Programs\BuildPilot`, the shortcuts and the Apps list entry under
`HKEY_CURRENT_USER`. Its log is `%TEMP%\BuildPilot Setup\buildpilot.log`.

## Regenerating the icon

Only needed after changing `assets\application-icon.png`; the result is committed.

```powershell
python tools/genicons.py
```

It writes `assets\application-icon.ico` at every size Windows asks for, from 16 to 256 pixels.
It needs Pillow.

## Regenerating the interface images

Only needed after changing a PNG master in `assets`; the results are committed.

```powershell
python tools/uiicons.py
```

The interface never draws a master: drawn from those 1254 pixel files, twenty rows held a drag at
20 frames a second. The script writes a small copy of each master to `assets\ui`, twice the
largest size the interface draws it (128 pixels; 256 for the application icon). `donate.png` is
a picture rather than an icon: it is cropped to its drawn pixels and scaled by height alone to
196 pixels, four times the toolbar's 49 pixel box. The `assets` test refuses a Slint file that
names anything else. It needs Pillow.

## Running from source

```powershell
cargo run
```

To keep a development run away from your real settings, point it at a folder of its own first:

```powershell
$env:BUILDPILOT_DATA_DIR = "$env:TEMP\BuildPilot-dev"
```

The log is `buildpilot.log` in whichever data folder is in use. A second run on the same data
folder brings the first window forward and exits, so a copy on its own folder can run beside
your everyday one.

## Testing

```powershell
./test.ps1
```

[TESTING.md](TESTING.md) covers the gate, the floor and what each suite proves.

## Versioning

The version lives in `VERSION` and nowhere else. The running application reads it through
`build.rs`. Two places cannot read it, so `stamp_version.ps1` writes it into them. One is
`Cargo.toml`, which `tests/structural.rs` checks. The other is the site under `docs/`, where each
mention is a `<!--VERSION-->` token that `tests/site.rs` checks. No other document names a version. After
changing `VERSION`, stamp before running the tests:

```powershell
./stamp_version.ps1
```

It is idempotent: a second run writes nothing.

## The website

`docs/` is the GitHub Pages site: `index.html`, `styles.css`, the icon, the donate artwork and
three captures in `screenshots/` (the window, the Edit dialog and Settings). It carries no
dates. Its colours are the application's tokens from
`ui/theme.slint`, light or dark as the reader's system is set.

## Where things live

| Path | What it holds |
|---|---|
| `src/domain` | The rules, with no I/O |
| `src/application` | `App`, its use cases and its ports |
| `src/infrastructure` | The ports against the real machine; `win32/` holds every Windows call |
| `src/ui` | The Rust side of the window |
| `src/main.rs` | The application's composition root |
| `src/setup` | The setup program's policy: routes, plans and words, with no I/O |
| `src/bin/buildpilotsetup.rs` | The setup program's composition root |
| `src/infrastructure/locations.rs` | Every name and folder the application and setup share, plus the author and copyright |
| `build_credits.rs` | Part of the build script: generates the credits and the third-party notices |
| `stamp_version.ps1` | Writes `VERSION` into `Cargo.toml` and the site |
| `tools/genicons.py` | Makes the `.ico` from the PNG master |
| `tools/uiicons.py` | Makes the small copies in `assets/ui` the interface draws |
| `ui/` | The Slint interface: `theme.slint` holds every colour and size |
| `assets/` | The application icon and every button image, as masters |
| `assets/ui` | The small copies of those masters, the only images the interface names |
| `docs/` | The GitHub Pages site |
| `tests/` | Every suite; `tests/fixtures` holds the scripts the process tests run |
| `SRS.md` | The requirements |

[ARCHITECTURE.md](ARCHITECTURE.md) explains how the pieces fit together.

## House rules worth knowing before a first change

- **Layers depend inwards.** `UI -> Application -> Domain <- Infrastructure`, checked by the
  structural tests.
- **400 lines per file at most.** A file in the band from 381 to 399 is cut to 350 or below, not
  trimmed to fit.
- **No magic numbers.** A literal that needs a comment to say what it represents is a named
  constant or derived from data.
- **One home for everything.** Colours and sizes live in `ui/theme.slint`, notice wording in the
  application layer, the version in `VERSION`.
- **`unsafe` only in `src/infrastructure/win32`,** each block with the reason it is sound.
- **Every guard is proved by planting a violation** and reading the exit code before it is
  trusted.

## See also

- [README.md](README.md) for what the application is and how to use it.
- [TESTING.md](TESTING.md) for the test suite in full.
- [ARCHITECTURE.md](ARCHITECTURE.md) for the invariants and the design decisions.
