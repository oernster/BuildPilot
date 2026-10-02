# Decisions and trade-offs

The deliberate choices BuildPilot rests on: what was chosen, what was given up
for it and why. Each entry is the decision as the product makes it today.
The detail behind each one, with the tests that hold it, lives in
[ARCHITECTURE.md](ARCHITECTURE.md) and the requirements specification
([SRS.md](SRS.md)), whose amendments record each change of mind and whose
Appendix D holds what is deliberately not planned.

## The product as a whole

### It runs the scripts; it never becomes the build system

BuildPilot remembers build scripts, starts them, shows their output and stops
them. The scripts stay the source of truth and stay runnable without it. It
never writes to, renames or deletes a script.

- **Rather than:** a build tool, a pipeline designer or a script editor.
- **Gains:** nothing about a project depends on BuildPilot; dropping it costs
  a project nothing.
- **Costs:** every build has to be expressed as a script first.

### What BuildPilot deliberately is not

There is no dependency graph, no condition, no Run All and no Stop All. A
running script cannot be typed into. The one sequence is an operation's own
steps, run in order; the one action on ticked rows is Run the ticked builds.

- **Rather than:** a CI server, a terminal or an autopilot that decides what
  to run.
- **Gains:** a small surface that can be held to a high bar.
- **Costs:** builds that depend on each other are started by hand in the right
  order.

### Rust and Slint

The application is written in Rust with a Slint interface, chosen from four
candidates. The two doubts about Slint (dragging rows and a very long output
view) were settled by measured spikes before any design depended on them.

- **Rather than:** Tauri with a TypeScript front end, egui or iced.
- **Gains:** the whole application is one language; layout lives in its own
  markup apart from the logic; Slint offers a GPLv3 licence that matches the
  repository.
- **Costs:** row dragging, the scroll bars and the tooltips had to be built
  by hand.

### Windows only

BuildPilot is a Windows application. An earlier plan for a macOS package and a
Flatpak was ruled out.

- **Rather than:** three platforms.
- **Gains:** one platform to build, test and package.
- **Costs:** nobody on macOS or Linux can use it. The Windows calls still sit
  behind adapters, so the door is not closed in the code.

### Specification before code

Every behaviour is a numbered requirement in the SRS naming the test that
verifies it. A change arrives as a numbered amendment with its reason, never
as a silent edit.

- **Rather than:** building first and describing afterwards.
- **Gains:** a ruled-out idea stays ruled out; each test names the requirement
  it holds.
- **Costs:** keeping the specification true is work of its own.

## Privacy and the network

### One request, to ask about updates

The only network request BuildPilot makes asks GitHub for the latest
published release. It names BuildPilot and its version and nothing about the
operator or their scripts. It goes through WinHTTP, which is part of Windows.

- **Rather than:** no update check at all; an HTTP and TLS library among the
  dependencies.
- **Gains:** updates are found; no networking crate is compiled in.
- **Costs:** GitHub learns which version is asking.

### The update check: quiet unless there is news

A check runs three seconds after the window opens and then once a day; an
automatic check that fails or finds nothing says nothing. A check the
operator asks for ignores any skipped release and always says what it found.
A tag that is not a version is never newer. Each stage of the request waits
at most five seconds and the request is never retried.

- **Rather than:** a check that reports every outcome; retries.
- **Gains:** updates are found without nagging; a malformed release never
  tells somebody their copy is stale.
- **Costs:** one unprompted request a day.

### The answer is treated as foreign input

Every field of GitHub's answer is checked and its size is capped at a
megabyte. A panic on the worker that asks is still answered, as unreachable.

- **Rather than:** trusting the release service.
- **Gains:** a malformed or hostile answer cannot crash the window; the
  operator's own check always gets a reply.
- **Costs:** none recorded.

### Donations go through the browser

The donate button hands the payment address to Windows to open. BuildPilot
never fetches it; a browser that will not open is reported as a notice.
Nothing is held back behind a donation.

- **Rather than:** making the request itself.
- **Gains:** no second network route inside the application.
- **Costs:** BuildPilot never learns what happened next.

### Plain files in the operator's own folder

The settings, the run times, chosen icons and the log are ordinary files in
the operator's data folder. None is encrypted. The log records launches,
exits, stops, notices and refusals, rotating at one megabyte with one
previous file kept.

- **Rather than:** encrypted settings; no record at all.
- **Gains:** inspectable files; a failure leaves a trail to read.
- **Costs:** anyone with access to the account can read which scripts are
  configured and what ran.

## Running a build

### A job object per run, joined before the first instruction

Each run starts suspended with no console window, joins a job object of its
own and only then resumes. Stop terminates the job. The job is set to kill
everything in it if BuildPilot ends without stopping it, including a crash.

- **Rather than:** killing the top process; joining the job after it starts.
- **Gains:** nothing a script starts can escape before it is caught, so Stop
  ends the whole tree and a crash leaves no orphans.
- **Costs:** a process that deliberately breaks away from its job survives
  Stop. A tree still alive five seconds after Stop is reported on its row by
  process id rather than hidden.

### Hard stop only

Stop ends the tree at once. No interrupt is sent first and there is no grace
period.

- **Rather than:** Ctrl+Break, then waiting.
- **Gains:** Stop always means stopped.
- **Costs:** a script gets no chance to tidy up after itself.

### What a finished script leaves running is released

Once a script has exited by itself, anything it deliberately left running (a
compiler server, a build daemon) is released from the job rather than killed
with it. After the exit, the pipes are given two seconds to drain, since such
a process can hold them open.

- **Rather than:** ending every process the run ever started; waiting for the
  pipes indefinitely.
- **Gains:** tools that keep a warm server between builds keep it; a run
  always finishes.
- **Costs:** those processes outlive the run and BuildPilot.

### The exit code alone decides success

A run succeeds when its process exits with code zero and fails otherwise.
Output is never inspected. Standard input is closed at launch.

- **Rather than:** reading the output for signs of failure; an interactive
  console.
- **Gains:** one rule that every build tool already follows; no guessing.
- **Costs:** a script that fails yet exits zero shows as succeeded; a script
  that prompts reads end of input.

### Many builds at once; one per folder

Any number of different operations run at once with no queue. A second run of
the same operation is refused, as is a run in a folder another running build
holds, compared as Windows compares folders.

- **Rather than:** no limit; one build at a time.
- **Gains:** projects build side by side; two builds never overwrite each
  other's output.
- **Costs:** the same project added twice with different arguments cannot run
  both at once.

### Steps in order, every one checked first

An operation holds one or more steps run in order in one output; the first
failure or a Stop ends the sequence. Every step's script, the folder and the
environment are checked before step one starts.

- **Rather than:** one script per operation; finding a missing second script
  only after the first has run. Measured across the owner's projects: fifteen
  documented running two Python scripts in turn.
- **Gains:** a two-script build is one row; a long first step is never wasted
  on a missing second.
- **Costs:** no branching or condition between steps.

### Batch files handed to the standard library

A batch file is given to Rust's process launcher as the program. It quotes
the arguments by the rules of the Windows command interpreter and refuses one
it cannot escape safely.

- **Rather than:** starting the command interpreter by hand, which would quote
  by the wrong rules.
- **Gains:** arguments with spaces and quotes arrive intact.
- **Costs:** the behaviour belongs to the standard library, which documents
  that it may change; a test with a real batch fixture watches for that.

### PowerShell 7 when present, without profile or prompts

A PowerShell script runs under PowerShell 7 when it is on PATH and Windows
PowerShell otherwise, with no profile, no interaction and the execution
policy bypassed.

- **Rather than:** always Windows PowerShell; whatever the machine's profile
  and policy say.
- **Gains:** the newer host where it exists; the same behaviour on every
  machine.
- **Costs:** a script that relies on its profile does not get it.

## Python environments and other script types

### Use an environment that exists; never build one

A Python step runs inside an environment already in its working directory.
BuildPilot runs no command that creates, installs into or repairs one; a
structural test refuses source that names such a command. A Python step with
no environment is refused rather than run on whatever Python is on PATH.

- **Rather than:** creating environments; falling back to the system Python.
- **Gains:** a build runs on the interpreter its project chose or not at all.
- **Costs:** a project without an environment has to make one first.

### Undo any inherited activation, every step

Before any step gets its variables, an activation BuildPilot inherited from
the shell that started it is undone as deactivation would undo it. Any other
environment's scripts folder is then taken off PATH.

- **Rather than:** passing BuildPilot's own environment straight through.
- **Gains:** two environments never conflict inside one build.
- **Costs:** a build that relied on an environment from the shell no longer
  sees it.

### Activated as the activation script would

A Python or PowerShell step with an environment gets the same variables the
environment's own activation script would set. Python output is also set
unbuffered and UTF-8. Batch steps are not activated.

- **Rather than:** activation only for Python scripts; Python's defaults.
  Measured: a piped Python held three lines printed two seconds apart until
  its exit at 4.06 seconds; unbuffered, they arrived at 0.04, 2.04 and 4.04.
  Its accented letters arrived in the ANSI code page.
- **Gains:** a PowerShell build that calls Python finds its environment;
  Python output arrives live and readable.
- **Costs:** a batch build that calls Python must activate the environment
  itself.

### Other script types through one table in Settings

Any other file type is run through a table in Settings mapping an extension
to a program and its leading arguments. A row there replaces the built-in
rule for that type.

- **Rather than:** a program field on every operation; more built-in types.
- **Gains:** a new script type is one row, shared by every operation.
- **Costs:** an operation whose type loses its row still loads but refuses to
  run until Settings is mended.

## Output

### The readers never wait on the window

Each run has a thread per pipe and one for the exit; none touches the
application's state. The first event after a drain schedules one drain on the
window's thread and anything arriving before it runs rides along.

- **Rather than:** one window update per line.
- **Gains:** the window stays responsive under a flood. Measured in the
  spike: fifty thousand lines in about three quarters of a second gave about
  four thousand drains, none longer than 3.6 milliseconds.
- **Costs:** none recorded.

### UTF-8 where it is valid; the OEM code page otherwise

Each line is decoded as UTF-8 when it is valid UTF-8 and in the OEM code page
when it is not. Terminal escape sequences are removed rather than
interpreted.

- **Rather than:** UTF-8 alone. Measured: PowerShell, Windows PowerShell and
  the command interpreter, started hidden, all wrote an accented letter as one
  byte invalid in UTF-8.
- **Gains:** accented paths and messages read correctly from tools of either
  kind.
- **Costs:** a line in some third encoding is still misread; no colour from
  the tools themselves.

### The outcome is coloured, not the stream

Every output line is drawn plainly whichever stream it came on. The run ends
with one closing line in the success, failure or muted colour, carrying the
same glyph and words as the row.

- **Rather than:** errors in red. Many build tools log to standard error,
  PyInstaller among them, which made a good build look broken.
- **Gains:** the colour means what the exit code means.
- **Costs:** a genuine warning on standard error is not singled out.

### Output is capped

A run keeps its latest hundred thousand lines and splits a line longer than
16,384 characters. Only the latest run of each operation is kept, until it
runs again or BuildPilot closes.

- **Rather than:** unbounded output; keeping earlier runs.
- **Gains:** a runaway build cannot exhaust memory.
- **Costs:** the start of a very long log is dropped (the tray says so);
  earlier runs' output is gone.

### Tabs become spaces; other control characters go

A tab becomes the spaces reaching the next eight-column stop. Any other
control character is dropped.

- **Rather than:** drawing them, which the font showed as boxes.
- **Gains:** columns a tool lines up still line up.
- **Costs:** none recorded.

### Long lines wrap, with the scroll bar's strip always kept

A line wider than the tray wraps, at a space where it can. The space for the
scroll bar is kept whether or not the bar shows.

- **Rather than:** cutting lines off; scrolling sideways; a width that follows
  the bar, which would loop, since the wrapped height decides whether the bar
  shows.
- **Gains:** every part of every line is in reach.
- **Costs:** a strip of the tray's width is always reserved.

### Following pins the tray to the end

While the tray follows, it pins itself to the last line whenever the end
moves. Scrolling up stops following; returning to the end or Jump to latest
resumes it.

- **Rather than:** scrolling once after each drain, which stopped short before
  new rows were measured and hid the closing line.
- **Gains:** a finished run's outcome is always in view.
- **Costs:** none recorded.

## Adding and remembering builds

### A folder is recognised, never added unseen

Add takes a script or a folder. A folder is matched against an ordered list
of two built-in patterns: a PowerShell build script; the two Python scripts
in turn. A parent folder lists its projects with tick boxes. A
partial match says which file is missing. Nothing is added until the operator
confirms.

- **Rather than:** assembling every operation by hand; adding what a scan
  finds automatically. The patterns are the two measured across the owner's
  projects: of thirty with a build script, twelve and fourteen.
- **Gains:** a whole folder of projects joins the deck in one confirmation.
- **Costs:** a project following neither pattern is added by choosing its
  script.

### Scans go one level deep

A scan reads the chosen folder and the folders directly inside it, never
deeper.

- **Rather than:** a recursive search.
- **Gains:** build scripts of dependencies and tools inside a project are
  never proposed.
- **Costs:** projects nested further down need choosing directly.

### New rows slot in by name; arranged rows never move

A new operation goes before the first row whose name sorts after its own,
ignoring case. Rows the operator dragged or moved stay where they were put.

- **Rather than:** appending at the end; sorting the whole deck.
- **Gains:** a deck kept in name order stays in order; a hand-made order is
  never undone.
- **Costs:** a hand-ordered deck gets new rows by name, which may not be
  where the operator wants them.

### Icons found by convention; a chosen one is copied

An icon is looked for in an ordered list of conventional places beside the
project. One the operator chooses is copied into BuildPilot's data folder; a
found one is referred to where it lies.

- **Rather than:** referring to every chosen image where it lies.
- **Gains:** a chosen icon survives the original moving; a new convention is
  one entry in the list.
- **Costs:** a copied icon does not follow later edits to the original.

### A damaged settings file is never lost

An entry that cannot be read is kept in the file untouched and written back
on every save. A file that cannot be read at all is set aside rather than
overwritten. A file written by a newer schema is read but never saved over.
Every write goes to a temporary file renamed over the old one.

- **Rather than:** rewriting the file from what was understood.
- **Gains:** a hand edit gone wrong or a downgrade costs nothing.
- **Costs:** an unreadable entry stays in the file until it is mended by
  hand.

### The settings file holds across a major version

Every release of one major version reads a settings file any earlier release
of it wrote and installs to the same folder under the same Apps list key.
An installer path is stored only once saved from Edit, so adding it did not
change the file's schema.

- **Rather than:** writing a found default into every operation at load.
- **Gains:** any release of a major version updates any earlier one in place;
  an earlier release still reads the file.
- **Costs:** breaking either promise needs a new major version.

### Run times in a file of their own

The durations behind the typical build time are kept in a file beside the
settings, written the same atomic way. One that cannot be read or written is
logged and nothing more. The settings file holds no run data at all.

- **Rather than:** a field in the settings file.
- **Gains:** a damaged timings file can never put the configuration at risk;
  a restart never shows a row as running.
- **Costs:** two files to keep instead of one.

### The typical time is the median of the last five successes

A row shows the median of its operation's five most recent successful runs,
in every state, once one has succeeded. With an even count it is the lower of
the middle two, a duration that was actually seen. A failed or stopped run
does not count.

- **Rather than:** the mean; the last run alone.
- **Gains:** one cold build does not move it; it follows a build that has
  grown quicker or slower.
- **Costs:** a lasting change in build time takes a few runs to show.

## Launching a project's installer

### Found by the projects' own convention

Launch installer starts the one set in Edit. With none set, it uses the
working directory's name followed by Setup.exe in the installer output
folder, then the plain output folder, matched on letters and digits alone.

- **Rather than:** always asking for a path. Measured over the owner's deck:
  twenty of twenty-one setup programs follow that convention.
- **Gains:** most rows need no setting at all.
- **Costs:** a project named otherwise needs its path set once.

### Held back after a run that did not succeed

The control is disabled while the row runs or stops and after a run that was
stopped or failed, until a later run succeeds.

- **Rather than:** always launching whatever file is there.
- **Gains:** a half-written or stale installer from a broken build is never
  started by mistake.
- **Costs:** an installer from an earlier good build cannot be launched until
  the next success.

### Started as Explorer would; looked for at set moments

The installer is handed to Windows as a double click would hand it, so
Windows asks for administrator rights where it needs them. BuildPilot does
not wait on it, read its output or stop it. It looks for the file at start,
on add, edit and select, when a run ends and just before launching; never on
each redraw.

- **Rather than:** launching it as a tracked child; looking on every redraw,
  four times a second, across two folders.
- **Gains:** BuildPilot never asks for rights itself; the rows never read the
  disk.
- **Costs:** an installer that appears by other means shows only at the next
  of those moments.

## The interface

### Status in words and glyphs, never colour alone

Each state has a glyph and its words as well as a colour. Progress is never a
percentage: a running build shows an indeterminate indicator and its elapsed
time.

- **Rather than:** coloured dots; a progress bar.
- **Gains:** readable for everybody; nothing invented about how far a script
  has got.
- **Costs:** more words on each row.

### One home for every colour, checked for contrast

Every colour and every size shared between files lives in one theme file,
in light and dark sets. A test reads it and requires text at 4.5 to one and
rings at three to one against the surfaces they are drawn on.

- **Rather than:** colours written where they are used.
- **Gains:** a colour that cannot be read fails the suite rather than
  shipping.
- **Costs:** a new colour has to earn its place.

### Light or dark, starting from Windows

The first run follows the Windows app theme. Choosing Light or Dark from the
toolbar or Settings then holds for good.

- **Rather than:** a third choice that follows Windows.
- **Gains:** two buttons that say what they do.
- **Costs:** once chosen, the theme no longer follows Windows.

### Its own controls, scroll bars and tooltips

No standard Slint button or tick box is used; every control wears the house
ring, which a structural test holds. Every scrolling surface has a bar at
least twelve pixels wide. The row list counts the rows out of sight at each
edge. The window draws one tooltip last, over everything.

- **Rather than:** the stock Fluent widgets. Their scroll bar is a two pixel
  line until the pointer finds it, so twenty rows showed four with no sign of
  the rest; each button's own tooltip was clipped by the list.
- **Gains:** every scroll shows itself; no tooltip is cut off.
- **Costs:** the controls are BuildPilot's own to maintain.

### The rows are one keyboard stop

Tab, Shift+Tab, Left and Right walk every control, wrapping. The rows are one
stop walked with Up and Down; only the selected row's own controls follow. Each
dialog opens on its first control and owns the ring while open. The focus
ring is drawn on controls, never on a pane.

- **Rather than:** every row's controls on the ring, which would cost eighty
  presses for ten rows.
- **Gains:** the whole application works without a mouse.
- **Costs:** Escape closes a dialog under the headless tests but not on the
  real window; dialogs are closed with their own buttons.

### Small copies of every image

The interface draws only small copies of the artwork, made by a script and
committed. A test refuses any image over 256 pixels. Operation icons are
scaled once when they load.

- **Rather than:** drawing the masters. Measured: twenty rows dragged at
  twenty frames a second from the full-size masters and sixty from the
  copies.
- **Gains:** dragging stays smooth.
- **Costs:** the copies must be regenerated when a master changes.

### Reading surfaces read themselves

The Guide, About, Licence and setup's licence page scroll slowly on their own
when they overflow. They pause the moment the reader scrolls, then carry on
from where the reader left them.

- **Rather than:** static pages.
- **Gains:** long text can be read hands free.
- **Costs:** none recorded.

### The Guide is held to the controls

A test reads every toolbar and row control's label and fails when the Guide
has no entry for it, one per state where a label changes.

- **Rather than:** a guide kept up by memory.
- **Gains:** the Guide cannot fall behind the window.
- **Costs:** a new control needs its Guide entry before the suite passes.

### One instance per data folder; closing asks while builds run

A second launch brings the running window forward and exits. Closing the
window while builds run asks first; confirming stops them all.

- **Rather than:** one instance per user whatever the folder; several copies
  running the same deck.
- **Gains:** one deck, one set of runs; a test copy on its own data folder
  runs beside the everyday one.
- **Costs:** one more question when closing mid-build.

### Every failure is said, never silent

The log opens before anything else can fail and standard error is pointed at
it. Every panic is logged on whatever thread it happens. A panic reading
output ends that reading, not BuildPilot. A failure that ends the program
shows an error box naming the log.

- **Rather than:** letting a crash end the window without a word.
- **Gains:** a failure always leaves something to read.
- **Costs:** none recorded.

## Building and installing

### The build runs the gate, with no way round it

The build script stamps the version, runs the full gate and only then builds
the application and the setup program carrying it. There is no switch to
skip the gate.

- **Rather than:** a build that can skip the tests.
- **Gains:** nothing ships that failed the gate.
- **Costs:** every release build waits for the whole suite.

### Installed for one user, without administrator rights

Setup installs into the user's own programs folder and registers in the
user's half of the registry.

- **Rather than:** a machine-wide install.
- **Gains:** no administrator prompt.
- **Costs:** each account on a machine installs separately.

### A setup program of its own

Install, update, downgrade, repair and removal are one bespoke program in the
same interface toolkit. It reads the machine once to choose its route, shows
one screen at a time and ends in a verdict. Its policy is pure and tested;
its progress bar is weighted by the time each step took on a real install.
Removal keeps the operator's data unless asked otherwise.

- **Rather than:** a generic installer.
- **Gains:** one look throughout; every route is a test.
- **Costs:** the setup program is BuildPilot's own to maintain.

### BuildPilot is found by name, never by process tree

Setup finds a running BuildPilot by its executable's name and offers to close
it before touching anything.

- **Rather than:** walking the process tree, which is judged from recorded
  parent ids that Windows reuses.
- **Gains:** setup can never end a stranger that inherited a dead parent's
  number.
- **Costs:** none recorded.

### Removing itself through a temporary copy

Started from the install folder, as the Apps list does, setup copies itself
to the temporary folder and runs from there so the install folder can go.
The next setup that is not that copy deletes it.

- **Rather than:** leaving the install folder behind.
- **Gains:** uninstall removes the whole folder.
- **Costs:** a copy lingers in the temporary folder until the next setup.

### Unsigned executables

Neither the application nor setup is signed.

- **Rather than:** buying a code-signing certificate.
- **Gains:** no certificate to buy or renew.
- **Costs:** neither carries a verified publisher.

### Credits generated from what ships

The open source credits in About are generated at build time from the
crates a release build compiles in, procedural macros left out, with each
licence shown by its readable name. Their licence texts are installed beside
the program.

- **Rather than:** a hand-written list; the full metadata, which merges in
  what test-only crates ask for and names crates that never ship. Measured
  when the list arrived: 292 crates where About had named one.
- **Gains:** the credits cannot drift from the build; a crate whose licence
  has no readable name fails a test.
- **Costs:** the build depends on what Cargo reports, read offline against
  the lock file.

### GPL plus a commercial licence

BuildPilot is GPL-3.0 and uses Slint under its GPLv3 licence. A commercial
licence for BuildPilot's own code is offered separately.

- **Rather than:** one licence for every use.
- **Gains:** the source is open on the same terms Slint is used under;
  closed-source users still have a route.
- **Costs:** anyone building closed software on it needs that separate
  licence.

## Engineering

### Layers with ports and one composition root

The code is split into domain, application, infrastructure and interface,
each depending only inward. The application reaches the machine only through
traits it declares; one composition root wires the real ones in. Structural
tests hold the boundaries and the standard library parts each pure layer may
name.

- **Rather than:** convention alone.
- **Gains:** every use case runs in a test with no process, file, clock or
  window.
- **Costs:** more modules and explicit wiring.

### Complete coverage where it means something

Lines and regions must both be fully covered over the domain, the
application and setup's policy. A branch that could not happen in production
was deleted rather than tested. The layers that need a real machine are
measured but not floored.

- **Rather than:** one figure over everything; lines alone, which an untaken
  branch on a covered line passes.
- **Gains:** anything short of complete in the pure layers is a decision
  nobody made.
- **Costs:** the machine-facing code relies on targeted integration tests.

### Small files

No Rust or Slint file may exceed four hundred lines or sit in the band from
381 to 399; one that reaches the band is cut to 350 or below.

- **Rather than:** letting files grow; trimming a line or two to fit.
- **Gains:** files split at real seams.
- **Costs:** many small files.

### Unsafe code in one folder

Every unsafe block lives with the Windows calls in one folder, each with the
reasoning that makes it sound.

- **Rather than:** unsafe code wherever a call needs it.
- **Gains:** the code that can break memory safety is in one place to review.
- **Costs:** a thin wrapper for every Windows call.

### Every value has one home

The version lives in one file; the manifest and the website are stamped from
it and tests fail when either differs. The website links its stylesheet and
script by a hash of their content.

- **Rather than:** copies written where they are needed.
- **Gains:** a change is made once; a browser never pairs a new page with a
  stale stylesheet.
- **Costs:** a stamping step that must run after every change.

### Tests with real parts

No mocking library: the application is tested against hand-written fakes.
Process tests start real hidden scripts; file tests work in temporary
folders. The keyboard is driven through Slint's own headless backend, pinned
to the Slint in use. Every guard was proved by planting a violation.

- **Rather than:** mocks and assumed guards.
- **Gains:** a passing test means the real thing works; a guard is known to
  bite.
- **Costs:** fakes are written by hand; how things look is still checked by
  hand.
