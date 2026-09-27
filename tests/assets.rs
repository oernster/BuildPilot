//! UI-013: the interface draws small copies of its images, never the 1254 px masters. Drawn
//! from the masters, twenty rows held a drag at 20 frames a second; from the copies, at 60.

use std::fs;
use std::path::{Path, PathBuf};

const ROOT: &str = env!("CARGO_MANIFEST_DIR");
/// Where the copies live, as the Slint files name them.
const COPIES: &str = "../assets/ui/";
/// The largest a copy may be on its longest side: twice the largest size the interface draws an
/// image (setup's 126 px mark), as tools/uiicons.py makes them.
const LARGEST_COPY: u32 = 256;
/// A PNG's width and height sit at these byte offsets of its header, big-endian.
const WIDTH_AT: usize = 16;
const HEIGHT_AT: usize = 20;

/// Every image a Slint file names, as `(file, path as written)`.
fn image_urls() -> Vec<(PathBuf, String)> {
    let marker = "@image-url(\"";
    let mut found = Vec::new();
    for entry in fs::read_dir(Path::new(ROOT).join("ui")).expect("ui/ is readable") {
        let path = entry.expect("a ui/ entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("slint") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("a .slint file is readable");
        for (at, _) in source.match_indices(marker) {
            let rest = &source[at + marker.len()..];
            let end = rest.find('"').expect("an image URL is closed");
            found.push((path.clone(), rest[..end].to_owned()));
        }
    }
    found
}

/// A PNG's longest side, read from its header.
fn longest_side(png: &Path) -> u32 {
    let bytes = fs::read(png).expect("the copy is readable");
    let read = |at: usize| u32::from_be_bytes(bytes[at..at + 4].try_into().expect("four bytes"));
    read(WIDTH_AT).max(read(HEIGHT_AT))
}

#[test]
fn the_interface_draws_only_small_copies() {
    let urls = image_urls();
    assert!(!urls.is_empty(), "the Slint files name images");
    let mut problems = Vec::new();
    for (file, url) in urls {
        let name = file.file_name().unwrap().to_string_lossy().into_owned();
        let Some(copy) = url.strip_prefix(COPIES) else {
            problems.push(format!("{name} draws {url}, not a copy in assets/ui"));
            continue;
        };
        let path = Path::new(ROOT).join("assets").join("ui").join(copy);
        if !path.is_file() {
            problems.push(format!(
                "{name} draws {url}, which is missing: run tools/uiicons.py"
            ));
            continue;
        }
        let side = longest_side(&path);
        if side > LARGEST_COPY {
            problems.push(format!("{url} is {side} px, over {LARGEST_COPY}"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
