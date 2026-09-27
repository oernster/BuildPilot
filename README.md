# <img width="128" height="128" alt="application-icon" src="https://github.com/user-attachments/assets/3f97eec9-1056-4f01-a55f-93454b69a342" /> BuildPilot

A build flight deck control tool for systematically launching multiple (generally build) scripts

BuildPilot remembers the build scripts you run often. It starts them as independent processes,
several at once, shows each one's state and live output, then stops them cleanly with everything
they started. It orchestrates the commands you already have; it never becomes the build system.
Every script stays runnable without it.

## Who it is for

- Developers on Windows who run the same handful of build, package or release scripts every day.
- Anyone juggling several projects whose builds they want to start side by side and watch at a
  glance.
- People who work from the keyboard: every control is reachable by Tab and the arrow keys.

## Who it is not for

- Anyone wanting a build system. BuildPilot runs your scripts; it does not replace PowerShell,
  cmd, Make, Cargo, npm or any other tool.
- Anyone wanting a pipeline. There is no sequencing, no dependency graph and no Run All.
- Anyone wanting a terminal. Output is shown as it arrives; a running script cannot be typed
  into.
- Anyone on macOS or Linux. BuildPilot is Windows only.

## What it does

- **Remembers your scripts.** Each operation holds a script or executable, its working directory,
  its arguments (one per line) and a name. `.ps1`, `.bat`, `.cmd` and `.exe` are supported.
- **Runs several at once.** Each run is its own process. Rows show the state (running, succeeded,
  failed, stopped) with a glyph and in words, plus the elapsed time.
- **Shows output live.** The tray at the foot of the window shows the selected run's output as it
  arrives, following the last line until you scroll up. It keeps the latest 100,000 lines of a
  run.
- **Stops the whole tree.** Stop ends the script and everything it started, through a Windows job
  object. A tree still alive five seconds after Stop is reported by process id.
- **Opens and reveals scripts.** Open a script with its associated application or show it in
  Explorer, from its row.
- **Uses PowerShell sensibly.** A `.ps1` runs under `pwsh` when it is on PATH and Windows
  PowerShell otherwise, with no profile, no prompts and `-ExecutionPolicy Bypass`. No console
  window opens.
- **Keeps its order.** Drag a row by its handle (Alt+Up and Alt+Down do the same) to reorder
  the deck; the order is saved.
- **Shows icons.** An icon beside a script is found for you; any image can be chosen instead.
- **Looks after its settings file.** An entry it cannot read is kept in the file untouched. A
  file it cannot read at all is set aside as `buildpilot.json.unreadable` rather than
  overwritten.
- **Runs once.** Starting BuildPilot while it is already running brings the open window forward.
- **Follows your theme.** Light, dark or whatever Windows is set to.
- **Keeps a log.** Launches, exits, stops and errors go to `buildpilot.log`. A crash is recorded
  there too and never ends the application silently.
- **Explains itself.** Help holds a Guide to every control, About with the open source credits,
  the licence and Check for Updates. Long text in them reads itself slowly and stops the moment
  you scroll it.
- **Makes one request, to ask about updates.** Shortly after it opens and once a day, BuildPilot
  asks GitHub whether a newer release is published; Help > Check for Updates asks on demand. The
  request carries nothing about you or your scripts, a release can be skipped and a failed check
  says nothing. It goes through WinHTTP, which is part of Windows, so no HTTP or TLS library is
  among BuildPilot's dependencies. Nothing else it does touches the network.

## Where it keeps things

| What | Where |
|---|---|
| Settings and operations | `%APPDATA%\BuildPilot\buildpilot.json` |
| Chosen icons | `%APPDATA%\BuildPilot\icons\` |
| Log | `%APPDATA%\BuildPilot\buildpilot.log`, with one previous file kept as `buildpilot.previous.log` |
| The program | `%LOCALAPPDATA%\Programs\BuildPilot` |

Settings shows the folder and opens it. The settings file is not encrypted.

## Built with

| Part | Choice |
|---|---|
| Language | Rust, edition 2024 |
| Interface | Slint, used under the GPLv3 |
| Windows calls | `windows-sys` |
| File pickers | `rfd` |
| Settings file | `serde` and `serde_json` |

## Getting it

Run `BuildPilotSetup.exe`. It installs BuildPilot for your own Windows account in
`%LOCALAPPDATA%\Programs\BuildPilot`, so it never asks for administrator rights. It adds a
Start menu shortcut, offers a desktop one and adds BuildPilot to the Apps list, where Modify,
Repair and Uninstall all reopen it. Run it again to update, go back to an earlier version,
repair or reinstall; it reads what is installed and offers the one that fits. Uninstalling
keeps your operations and settings unless you untick that. Neither executable is signed.

## Testing

The gate checks formatting, runs clippy with warnings as errors, then runs every test under
coverage with a floor of 100% of lines over the domain and application layers.

```powershell
./test.ps1
```

[TESTING.md](TESTING.md) covers what is tested, what is not and why.

## Building

```powershell
./build.ps1
```

It runs the gate, then writes the application to `target\release\buildpilot.exe` and the setup
program to `dist\BuildPilotSetup.exe`.
[DEVELOPMENT.md](DEVELOPMENT.md) sets up a machine to build it;
[ARCHITECTURE.md](ARCHITECTURE.md) explains how it is put together and why.

## Licence

BuildPilot is distributed under the GNU General Public License v3.0; see [LICENSE](LICENSE).
Slint is used under its GPLv3 licence. Every crate built into BuildPilot is credited in Help >
About; the list is generated at build time, never written by hand. Their licence texts are
written to `THIRD-PARTY-NOTICES.txt`, which setup installs beside the program.
