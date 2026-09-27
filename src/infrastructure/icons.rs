//! Operation icons on disk (SRS 3.3).

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::application::ports::IconLibrary;
use crate::domain::operation::OperationId;

/// Where to look for an icon beside a script, relative to the script's folder, in order. A new
/// convention is one more entry here (ICON-003).
pub const ICON_CONVENTIONS: &[&[&str]] = &[&["assets", "application-icon.png"]];

/// The file signatures of the image types an icon may be: PNG, JPEG, ICO.
const IMAGE_SIGNATURES: &[&[u8]] = &[b"\x89PNG\r\n\x1a\n", b"\xff\xd8\xff", b"\x00\x00\x01\x00"];

/// Enough bytes to hold the longest signature.
const SIGNATURE_BYTES: usize = 8;

/// The extension a copied icon keeps when its source has none.
const FALLBACK_EXTENSION: &str = "png";

/// Icons: found beside scripts, copied into `<data folder>\icons` when chosen.
pub struct FsIconLibrary {
    folder: PathBuf,
}

impl FsIconLibrary {
    /// A library storing chosen icons in `folder`, which need not exist yet.
    pub fn new(folder: PathBuf) -> Self {
        Self { folder }
    }

    /// Every stored copy for operation `id`, whatever its extension.
    fn copies_of(&self, id: &OperationId) -> Vec<PathBuf> {
        let Ok(entries) = fs::read_dir(&self.folder) else {
            return Vec::new();
        };
        entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.file_stem().is_some_and(|stem| stem == id.as_str()))
            .collect()
    }
}

impl IconLibrary for FsIconLibrary {
    fn discover(&self, script_dir: &Path) -> Option<PathBuf> {
        ICON_CONVENTIONS
            .iter()
            .map(|parts| {
                parts
                    .iter()
                    .fold(script_dir.to_path_buf(), |path, part| path.join(part))
            })
            .find(|candidate| self.is_readable(candidate))
    }

    /// True when `path` opens and starts with a PNG, JPEG or ICO signature. This recognises
    /// the file type; it does not decode the whole image.
    fn is_readable(&self, path: &Path) -> bool {
        let Ok(mut file) = File::open(path) else {
            return false;
        };
        let mut head = Vec::with_capacity(SIGNATURE_BYTES);
        if file
            .by_ref()
            .take(SIGNATURE_BYTES as u64)
            .read_to_end(&mut head)
            .is_err()
        {
            return false;
        }
        IMAGE_SIGNATURES
            .iter()
            .any(|signature| head.starts_with(signature))
    }

    fn import(&mut self, id: &OperationId, source: &Path) -> Result<PathBuf, String> {
        if !self.is_readable(source) {
            return Err("it is not a PNG, JPEG or ICO image that can be read".to_owned());
        }
        fs::create_dir_all(&self.folder).map_err(|error| error.to_string())?;
        let extension = source
            .extension()
            .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_else(|| FALLBACK_EXTENSION.to_owned());
        let stored = self.folder.join(format!("{id}.{extension}"));
        for old in self.copies_of(id) {
            if old != stored {
                // A leftover copy only wastes space; failing to delete it is not worth refusing.
                let _ = fs::remove_file(old);
            }
        }
        fs::copy(source, &stored).map_err(|error| error.to_string())?;
        Ok(stored)
    }

    fn release(&mut self, id: &OperationId) {
        for copy in self.copies_of(id) {
            // As above: a copy that survives is harmless.
            let _ = fs::remove_file(copy);
        }
    }
}
