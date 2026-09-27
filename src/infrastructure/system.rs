//! The plain operating-system answers: the time, fresh identities and whether paths exist.

use std::path::Path;
use std::time::Instant;

use crate::application::ports::{Clock, IdSource, PathProbe};
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
}
