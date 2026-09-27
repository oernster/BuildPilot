//! The plain operating-system answers: the time, fresh identities, whether paths exist and
//! BuildPilot's own variables.

use std::env;
use std::fs;
use std::path::Path;
use std::time::Instant;

use crate::application::ports::{Clock, IdSource, PathProbe, Variables};
use crate::domain::operation::OperationId;

/// The real clock.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// Random UUIDs as operation identities (CFG-003).
pub struct UuidIds;

impl IdSource for UuidIds {
    fn next_id(&mut self) -> OperationId {
        OperationId::new(uuid::Uuid::new_v4().to_string()).expect("a UUID is never blank")
    }
}

/// The real file system's answers about paths.
pub struct FsPaths;

impl PathProbe for FsPaths {
    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }
    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }
    fn subfolders(&self, dir: &Path) -> Vec<String> {
        entries_where(dir, Path::is_dir)
    }
    fn files(&self, dir: &Path) -> Vec<String> {
        entries_where(dir, Path::is_file)
    }
}

/// The sorted names of the entries directly inside `dir` whose path passes `keep`; none when
/// `dir` cannot be read.
fn entries_where(dir: &Path, keep: fn(&Path) -> bool) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|entry| keep(&entry.path()))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// The variables BuildPilot itself was started with (ENV-009).
pub struct ProcessVariables;

impl Variables for ProcessVariables {
    /// A value that is not valid Unicode is carried with its invalid parts replaced, since the
    /// domain edits text.
    fn inherited(&self) -> Vec<(String, String)> {
        env::vars_os()
            .map(|(name, value)| {
                (
                    name.to_string_lossy().into_owned(),
                    value.to_string_lossy().into_owned(),
                )
            })
            .collect()
    }
}
