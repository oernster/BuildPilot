//! Python environments that already exist; the variables a step runs with (SRS 3.18).
//!
//! BuildPilot never makes an environment (ENV-007). It finds one (ENV-001), decides which one a
//! run uses (ENV-002 to ENV-005) and works out what `activate.bat` would have changed
//! (ENV-006), after undoing what `deactivate.bat` would undo (ENV-009, ENV-010). Everything that
//! looks at the disk is handed in, so the rules here are pure.

use std::fmt;
use std::path::{Path, PathBuf};

/// The file that marks a folder as an environment (ENV-001).
pub const MARKER_FILE: &str = "pyvenv.cfg";
/// The environment's folder of programs, which activation puts first on `PATH`.
pub const SCRIPTS_FOLDER: &str = "Scripts";
/// The environment's interpreter, inside `SCRIPTS_FOLDER`.
pub const PYTHON_PROGRAM: &str = "python.exe";
/// Folder names preselected when several environments are found (ENV-002).
pub const PREFERRED_NAMES: &[&str] = &["venv", ".venv"];

const PATH: &str = "PATH";
const VIRTUAL_ENV: &str = "VIRTUAL_ENV";
const VIRTUAL_ENV_PROMPT: &str = "VIRTUAL_ENV_PROMPT";
const PYTHONHOME: &str = "PYTHONHOME";
const OLD_PATH: &str = "_OLD_VIRTUAL_PATH";
const OLD_PYTHONHOME: &str = "_OLD_VIRTUAL_PYTHONHOME";
const OLD_PROMPT: &str = "_OLD_VIRTUAL_PROMPT";
/// What `deactivate.bat` clears, read on the reference machine on 2026-09-27.
const ACTIVATION_VARIABLES: &[&str] = &[
    VIRTUAL_ENV,
    VIRTUAL_ENV_PROMPT,
    OLD_PATH,
    OLD_PYTHONHOME,
    OLD_PROMPT,
];
/// Python's output reaches the tray live and in UTF-8 only with these (R-5, measured).
const PYTHON_OUTPUT: &[(&str, &str)] = &[("PYTHONUNBUFFERED", "1"), ("PYTHONIOENCODING", "utf-8")];

const PATH_SEPARATOR: char = ';';
const PATH_QUOTE: char = '"';

/// Environment folder names in `folders` that are environments, judged by `is_file` (ENV-001).
pub fn environments_among(
    working_dir: &Path,
    folders: &[String],
    is_file: impl Fn(&Path) -> bool,
) -> Vec<String> {
    folders
        .iter()
        .filter(|folder| is_environment(&working_dir.join(folder), &is_file))
        .cloned()
        .collect()
}

/// True when `folder` holds both the marker and the interpreter (ENV-001).
pub fn is_environment(folder: &Path, is_file: impl Fn(&Path) -> bool) -> bool {
    is_file(&folder.join(MARKER_FILE)) && is_file(&python_in(folder))
}

/// The interpreter of the environment at `folder`.
pub fn python_in(folder: &Path) -> PathBuf {
    folder.join(SCRIPTS_FOLDER).join(PYTHON_PROGRAM)
}

/// The environment the dialog offers first among `found` (ENV-002): the only one; else the one
/// with a preferred name when exactly one has such a name; else none.
pub fn preselect(found: &[String]) -> Option<String> {
    if let [only] = found {
        return Some(only.clone());
    }
    let mut preferred = found
        .iter()
        .filter(|name| PREFERRED_NAMES.contains(&name.as_str()));
    match (preferred.next(), preferred.next()) {
        (Some(name), None) => Some(name.clone()),
        _ => None,
    }
}

/// What the operation dialog shows about the environment (ENV-002).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Offer {
    /// Nothing: no step is activated; else only `.ps1` steps are, with none to use.
    Nothing,
    /// A line of text, for one environment or for a `.py` step with none.
    Note(String),
    /// A choice among several, with the one to start on where there is one.
    Choose {
        /// The environments found.
        found: Vec<String>,
        /// The one preselected (ENV-002).
        preselected: Option<String>,
    },
}

/// The offer for steps needing `need` in a folder holding `found`.
pub fn offer(need: Need, found: &[String]) -> Offer {
    match (need, found) {
        (Need::None, _) | (Need::Optional, []) => Offer::Nothing,
        (Need::Required, []) => Offer::Note(
            "No Python environment in this folder: the .py steps will not run until one exists."
                .to_owned(),
        ),
        (_, [only]) => Offer::Note(format!("Environment: {only}")),
        (_, several) => Offer::Choose {
            found: several.to_vec(),
            preselected: preselect(several),
        },
    }
}

/// How much an operation's steps depend on an environment, least first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Need {
    /// No step is activated, so no environment is looked for.
    None,
    /// A `.ps1` step is activated when there is one; it runs without one otherwise (ENV-004).
    Optional,
    /// A `.py` step cannot run without one (ENV-004).
    Required,
}

/// Why a run found no environment to use (ENV-004, ENV-005).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvironmentProblem {
    /// None was found where a `.py` step needs one.
    NoneFound,
    /// The environment the operation names is not there.
    NamedMissing(String),
    /// Several were found and the operation names none of them.
    Several(Vec<String>),
}

impl fmt::Display for EnvironmentProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoneFound => f.write_str("No Python environment was found"),
            Self::NamedMissing(name) => write!(f, "The environment {name} was not found"),
            Self::Several(found) => write!(
                f,
                "Several Python environments were found ({}): choose one in Edit",
                found.join(", ")
            ),
        }
    }
}

/// The environment a run uses (ENV-003): the named one; else the only one found. `Ok(None)`
/// means the run goes without one.
pub fn resolve(
    need: Need,
    named: Option<&str>,
    found: &[String],
) -> Result<Option<String>, EnvironmentProblem> {
    if need == Need::None {
        return Ok(None);
    }
    if let Some(name) = named {
        return found
            .iter()
            .find(|candidate| *candidate == name)
            .map(|candidate| Some(candidate.clone()))
            .ok_or_else(|| EnvironmentProblem::NamedMissing(name.to_owned()));
    }
    match found {
        [] if need == Need::Required => Err(EnvironmentProblem::NoneFound),
        [] => Ok(None),
        [only] => Ok(Some(only.clone())),
        several => Err(EnvironmentProblem::Several(several.to_vec())),
    }
}

/// Changes to the inherited variables for one step's process. Names compare without regard to
/// case, as Windows compares them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VariableEdits {
    /// Variables to remove.
    pub remove: Vec<String>,
    /// Variables to set, replacing any inherited value.
    pub set: Vec<(String, String)>,
}

/// The edits for one step (ENV-009, ENV-010, then ENV-006 when `activation` names an
/// environment folder). `inherited` is BuildPilot's own environment; `is_file` looks at the
/// disk, to tell an environment's `Scripts` folder on `PATH`.
pub fn step_variables(
    inherited: &[(String, String)],
    activation: Option<&Path>,
    is_file: impl Fn(&Path) -> bool,
) -> VariableEdits {
    let lookup = |name: &str| {
        inherited
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.clone())
    };
    // ENV-009: what deactivate.bat restores, then what it clears.
    let path = lookup(OLD_PATH)
        .or_else(|| lookup(PATH))
        .unwrap_or_default();
    let python_home = lookup(OLD_PYTHONHOME).or_else(|| lookup(PYTHONHOME));
    // ENV-010: no environment's Scripts folder stays on PATH.
    let path = join_path(
        split_path(&path)
            .into_iter()
            .filter(|entry| !is_scripts_of_environment(Path::new(entry), &is_file)),
    );
    let mut set: Vec<(&str, String)> = Vec::new();
    match activation {
        None => {
            set.push((PATH, path));
            if let Some(home) = python_home {
                set.push((PYTHONHOME, home));
            }
        }
        // ENV-006: what activate.bat sets, from the deactivated values.
        Some(folder) => {
            if let Some(home) = python_home {
                set.push((OLD_PYTHONHOME, home));
            }
            let prompt = folder
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            let scripts = folder.join(SCRIPTS_FOLDER).display().to_string();
            set.push((VIRTUAL_ENV, folder.display().to_string()));
            set.push((VIRTUAL_ENV_PROMPT, prompt));
            set.push((
                PATH,
                join_path([scripts].into_iter().chain(split_path(&path))),
            ));
            set.push((OLD_PATH, path));
            set.extend(
                PYTHON_OUTPUT
                    .iter()
                    .map(|(name, value)| (*name, (*value).to_owned())),
            );
        }
    }
    // Whatever deactivation clears, plus PYTHONHOME, goes unless it was just given a value.
    let remove = ACTIVATION_VARIABLES
        .iter()
        .chain([&PYTHONHOME])
        .filter(|name| !set.iter().any(|(key, _)| key == *name))
        .map(|name| (*name).to_owned())
        .collect();
    VariableEdits {
        remove,
        set: set
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    }
}

/// True when `entry` is the `Scripts` folder of an environment (ENV-010).
fn is_scripts_of_environment(entry: &Path, is_file: impl Fn(&Path) -> bool) -> bool {
    let named_scripts = entry
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case(SCRIPTS_FOLDER));
    named_scripts
        && entry
            .parent()
            .is_some_and(|parent| is_file(&parent.join(MARKER_FILE)))
}

/// `PATH`'s entries, as Windows reads them: `;` separates, except inside double quotes, which
/// are not part of the entry.
pub fn split_path(path: &str) -> Vec<String> {
    let mut entries = vec![String::new()];
    let mut quoted = false;
    for character in path.chars() {
        match character {
            PATH_QUOTE => quoted = !quoted,
            PATH_SEPARATOR if !quoted => entries.push(String::new()),
            other => entries.last_mut().expect("never empty").push(other),
        }
    }
    entries.retain(|entry| !entry.is_empty());
    entries
}

/// `entries` as one `PATH`, quoting an entry that holds a separator.
pub fn join_path(entries: impl IntoIterator<Item = String>) -> String {
    entries
        .into_iter()
        .map(|entry| {
            if entry.contains(PATH_SEPARATOR) {
                format!("{PATH_QUOTE}{entry}{PATH_QUOTE}")
            } else {
                entry
            }
        })
        .collect::<Vec<_>>()
        .join(&PATH_SEPARATOR.to_string())
}
