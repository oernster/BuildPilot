//! Fake ports. Each fake shares its state with the test through `Rc<RefCell<..>>`, so a test
//! arranges the world, drives `App` and then inspects what `App` asked of the world.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use buildpilot::application::ports::LoadedConfig;
use buildpilot::application::{
    App, Clock, ConfigStore, IconLibrary, IdSource, Launcher, Log, PathProbe, Ports, ProcessHandle,
    RunKey, RunTimesStore, Shell, StoreError, Variables,
};
use buildpilot::domain::launch_plan::{LaunchPlan, PowerShellHost};
use buildpilot::domain::operation::{Operation, OperationId};
use buildpilot::domain::preferences::Preferences;
use buildpilot::domain::run_times::RunTimes;

/// Everything the fakes record and everything a test can arrange.
#[derive(Default)]
pub struct WorldState {
    pub to_load: LoadedConfig,
    pub saves: Vec<(Vec<Operation>, Preferences)>,
    pub save_error: Option<String>,
    pub next_id: u32,
    pub files: HashSet<PathBuf>,
    pub dirs: HashSet<PathBuf>,
    pub discoverable: HashMap<PathBuf, PathBuf>,
    pub readable_icons: HashSet<PathBuf>,
    pub imports: Vec<(OperationId, PathBuf)>,
    pub import_error: Option<String>,
    pub released: Vec<OperationId>,
    pub launches: Vec<(RunKey, LaunchPlan)>,
    pub spawn_error: Option<String>,
    pub stops: Vec<u32>,
    pub stop_error: Option<String>,
    pub shell_calls: Vec<(&'static str, PathBuf)>,
    pub shell_error: Option<String>,
    pub log: Vec<String>,
    pub inherited: Vec<(String, String)>,
    /// What the run times store hands `App` at start; an `Err` is an unreadable file.
    pub run_times_to_load: Option<Result<RunTimes, String>>,
    /// Every run times save, in order.
    pub run_times_saves: Vec<RunTimes>,
    pub run_times_save_error: Option<String>,
}

/// A test's whole world: shared state plus a clock the test moves.
pub struct World {
    pub state: Rc<RefCell<WorldState>>,
    pub now: Rc<Cell<Instant>>,
}

pub const DATA_FOLDER: &str = r"C:\Users\op\AppData\Roaming\BuildPilot";
pub const FIRST_PID: u32 = 4000;

impl World {
    pub fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(WorldState::default())),
            now: Rc::new(Cell::new(Instant::now())),
        }
    }

    /// Records that a script exists, together with its folder.
    pub fn with_script(self, script: &str) -> Self {
        let script = PathBuf::from(script);
        {
            let mut state = self.state.borrow_mut();
            state.dirs.insert(script.parent().unwrap().to_path_buf());
            state.files.insert(script);
        }
        self
    }

    /// Records an existing environment at `folder`: its marker and its interpreter (ENV-001).
    pub fn with_environment(self, folder: &str) -> Self {
        let folder = PathBuf::from(folder);
        {
            let mut state = self.state.borrow_mut();
            state.dirs.insert(folder.clone());
            state.files.insert(folder.join("pyvenv.cfg"));
            state.files.insert(folder.join(r"Scripts\python.exe"));
        }
        self
    }

    /// BuildPilot's own variables, as the fake reports them (ENV-009).
    pub fn with_inherited(self, pairs: &[(&str, &str)]) -> Self {
        self.state.borrow_mut().inherited = pairs
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect();
        self
    }

    pub fn advance(&self, by: Duration) {
        self.now.set(self.now.get() + by);
    }

    /// Starts an `App` over this world, preferring pwsh for `.ps1`.
    pub fn app(&self) -> App {
        let ports = Ports {
            store: Box::new(FakeStore(self.state.clone())),
            run_times: Box::new(FakeRunTimes(self.state.clone())),
            ids: Box::new(FakeIds(self.state.clone())),
            clock: Box::new(FakeClock(self.now.clone())),
            paths: Box::new(FakePaths(self.state.clone())),
            icons: Box::new(FakeIcons(self.state.clone())),
            launcher: Box::new(FakeLauncher(self.state.clone())),
            shell: Box::new(FakeShell(self.state.clone())),
            log: Box::new(FakeLog(self.state.clone())),
            variables: Box::new(FakeVariables(self.state.clone())),
        };
        App::start(ports, PowerShellHost::Pwsh)
    }

    /// Every line logged so far.
    pub fn logged(&self) -> Vec<String> {
        self.state.borrow().log.clone()
    }

    pub fn save_count(&self) -> usize {
        self.state.borrow().saves.len()
    }

    /// The key of the `nth` launch (from zero).
    pub fn launch_key(&self, nth: usize) -> RunKey {
        self.state.borrow().launches[nth].0.clone()
    }
}

struct FakeStore(Rc<RefCell<WorldState>>);

impl ConfigStore for FakeStore {
    fn load(&mut self) -> LoadedConfig {
        std::mem::take(&mut self.0.borrow_mut().to_load)
    }
    fn save(
        &mut self,
        operations: &[Operation],
        preferences: &Preferences,
    ) -> Result<(), StoreError> {
        let mut state = self.0.borrow_mut();
        if let Some(message) = state.save_error.clone() {
            return Err(StoreError {
                path: PathBuf::from(DATA_FOLDER).join("buildpilot.json"),
                message,
            });
        }
        state.saves.push((operations.to_vec(), preferences.clone()));
        Ok(())
    }
    fn data_folder(&self) -> PathBuf {
        PathBuf::from(DATA_FOLDER)
    }
}

struct FakeRunTimes(Rc<RefCell<WorldState>>);

impl RunTimesStore for FakeRunTimes {
    fn load(&mut self) -> Result<RunTimes, String> {
        self.0
            .borrow_mut()
            .run_times_to_load
            .take()
            .unwrap_or_else(|| Ok(RunTimes::default()))
    }
    fn save(&mut self, times: &RunTimes) -> Result<(), StoreError> {
        let mut state = self.0.borrow_mut();
        if let Some(message) = state.run_times_save_error.clone() {
            return Err(StoreError {
                path: PathBuf::from(DATA_FOLDER).join("run-times.json"),
                message,
            });
        }
        state.run_times_saves.push(times.clone());
        Ok(())
    }
}

struct FakeIds(Rc<RefCell<WorldState>>);

impl IdSource for FakeIds {
    fn next_id(&mut self) -> OperationId {
        let mut state = self.0.borrow_mut();
        state.next_id += 1;
        OperationId::new(format!("op-{}", state.next_id)).unwrap()
    }
}

struct FakeClock(Rc<Cell<Instant>>);

impl Clock for FakeClock {
    fn now(&self) -> Instant {
        self.0.get()
    }
}

struct FakePaths(Rc<RefCell<WorldState>>);

impl PathProbe for FakePaths {
    fn is_file(&self, path: &Path) -> bool {
        self.0.borrow().files.contains(path)
    }
    fn is_dir(&self, path: &Path) -> bool {
        self.0.borrow().dirs.contains(path)
    }
    fn subfolders(&self, dir: &Path) -> Vec<String> {
        names_inside(&self.0.borrow().dirs, dir)
    }
    fn files(&self, dir: &Path) -> Vec<String> {
        names_inside(&self.0.borrow().files, dir)
    }
}

/// The sorted names of the `paths` directly inside `dir`.
fn names_inside(paths: &HashSet<PathBuf>, dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = paths
        .iter()
        .filter(|path| path.parent() == Some(dir))
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

struct FakeVariables(Rc<RefCell<WorldState>>);

impl Variables for FakeVariables {
    fn inherited(&self) -> Vec<(String, String)> {
        self.0.borrow().inherited.clone()
    }
}

struct FakeIcons(Rc<RefCell<WorldState>>);

impl IconLibrary for FakeIcons {
    fn discover(&self, script_dir: &Path) -> Option<PathBuf> {
        self.0.borrow().discoverable.get(script_dir).cloned()
    }
    fn is_readable(&self, path: &Path) -> bool {
        self.0.borrow().readable_icons.contains(path)
    }
    fn import(&mut self, id: &OperationId, source: &Path) -> Result<PathBuf, String> {
        let mut state = self.0.borrow_mut();
        if let Some(message) = state.import_error.clone() {
            return Err(message);
        }
        state.imports.push((id.clone(), source.to_path_buf()));
        let stored = PathBuf::from(DATA_FOLDER)
            .join("icons")
            .join(format!("{id}.png"));
        state.readable_icons.insert(stored.clone());
        Ok(stored)
    }
    fn release(&mut self, id: &OperationId) {
        self.0.borrow_mut().released.push(id.clone());
    }
}

struct FakeLauncher(Rc<RefCell<WorldState>>);

impl Launcher for FakeLauncher {
    fn spawn(&mut self, key: RunKey, plan: &LaunchPlan) -> Result<Box<dyn ProcessHandle>, String> {
        let mut state = self.0.borrow_mut();
        if let Some(message) = state.spawn_error.clone() {
            return Err(message);
        }
        state.launches.push((key, plan.clone()));
        let pid = FIRST_PID + state.launches.len() as u32;
        Ok(Box::new(FakeProcess {
            pid,
            state: self.0.clone(),
        }))
    }
}

struct FakeProcess {
    pid: u32,
    state: Rc<RefCell<WorldState>>,
}

impl ProcessHandle for FakeProcess {
    fn pid(&self) -> u32 {
        self.pid
    }
    fn stop(&mut self) -> Result<(), String> {
        let mut state = self.state.borrow_mut();
        state.stops.push(self.pid);
        match state.stop_error.clone() {
            Some(message) => Err(message),
            None => Ok(()),
        }
    }
}

struct FakeShell(Rc<RefCell<WorldState>>);

impl FakeShell {
    fn record(&self, action: &'static str, path: &Path) -> Result<(), String> {
        let mut state = self.0.borrow_mut();
        if let Some(message) = state.shell_error.clone() {
            return Err(message);
        }
        state.shell_calls.push((action, path.to_path_buf()));
        Ok(())
    }
}

struct FakeLog(Rc<RefCell<WorldState>>);

impl Log for FakeLog {
    fn record(&self, line: &str) {
        self.0.borrow_mut().log.push(line.to_owned());
    }
}

impl Shell for FakeShell {
    fn open(&self, file: &Path) -> Result<(), String> {
        self.record("open", file)
    }
    fn reveal(&self, file: &Path) -> Result<(), String> {
        self.record("reveal", file)
    }
    fn open_folder(&self, folder: &Path) -> Result<(), String> {
        self.record("open_folder", folder)
    }
    fn open_address(&self, address: &str) -> Result<(), String> {
        self.record("open_address", Path::new(address))
    }
}
