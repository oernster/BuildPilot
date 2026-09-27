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
cargo build --release
```

The build script, `build.rs`, does two things before the code compiles:

1. Reads `VERSION` and hands it to the code as `BUILDPILOT_VERSION`, falling back to
   `0.0.0-dev` if the file is missing, so no version literal lives in the source.
2. Compiles the Slint interface in `ui/` into Rust.

The application is written to `target\release\buildpilot.exe`. There is no setup program yet.

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

The version lives in `VERSION` and nowhere else. `Cargo.toml` must carry the same string, which
`tests/structural.rs` checks; the running application reads it through `build.rs`.

## Where things live

| Path | What it holds |
|---|---|
| `src/domain` | The rules, with no I/O |
| `src/application` | `App`, its use cases and its ports |
| `src/infrastructure` | The ports against the real machine; `win32/` holds every Windows call |
| `src/ui` | The Rust side of the window |
| `src/main.rs` | The composition root |
| `ui/` | The Slint interface: `theme.slint` holds every colour and size |
| `assets/` | The application icon and every button image |
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
