use std::fs;
use std::path::Path;

use buildpilot::application::ports::IconLibrary;
use buildpilot::domain::operation::OperationId;
use buildpilot::infrastructure::icons::{FsIconLibrary, IconConvention};

const PNG: &[u8] = b"\x89PNG\r\n\x1a\nrest of image";
const JPEG: &[u8] = b"\xff\xd8\xff\xe0rest";
const ICO: &[u8] = b"\x00\x00\x01\x00rest";

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn library(root: &Path) -> FsIconLibrary {
    FsIconLibrary::new(root.join("data").join("icons"))
}

// ICON-001
#[test]
fn finds_conventional_icon() {
    let root = tempfile::tempdir().unwrap();
    let icon = root
        .path()
        .join("app")
        .join("assets")
        .join("application-icon.png");
    write(&icon, PNG);
    assert_eq!(
        library(root.path()).discover(&root.path().join("app")),
        Some(icon)
    );
}

// ICON-001, ICON-003: the conventions in order: assets\application-icon.png, then .ico, then a
// PNG named after the folder in the folder itself. The first readable image wins.
#[test]
fn conventions_are_tried_in_order() {
    let root = tempfile::tempdir().unwrap();
    let app = root.path().join("Stellody");
    let png = app.join("assets").join("application-icon.png");
    let ico = app.join("assets").join("application-icon.ico");
    let named = app.join("Stellody.png");
    let library = library(root.path());
    assert_eq!(library.discover(&app), None);
    write(&named, PNG);
    assert_eq!(library.discover(&app), Some(named.clone()));
    write(&ico, ICO);
    assert_eq!(library.discover(&app), Some(ico.clone()));
    write(&png, PNG);
    assert_eq!(library.discover(&app), Some(png));
    // A convention whose file is not an image passes to the next.
    write(&app.join("assets").join("application-icon.png"), b"hello");
    assert_eq!(library.discover(&app), Some(ico));
}

// ICON-001: the folder-named PNG is the folder's own name in full, dots and all; another PNG in
// the folder is not an icon, nor is one when the folder is a drive's root.
#[test]
fn only_the_folder_named_png_is_found() {
    let root = tempfile::tempdir().unwrap();
    let dotted = root.path().join("my.app");
    write(&dotted.join("my.png"), PNG);
    write(&dotted.join("logo.png"), PNG);
    assert_eq!(library(root.path()).discover(&dotted), None);
    write(&dotted.join("my.app.png"), PNG);
    assert_eq!(
        library(root.path()).discover(&dotted),
        Some(dotted.join("my.app.png"))
    );
    assert_eq!(
        IconConvention::NamedForFolder("png").candidate(Path::new(r"C:\")),
        None
    );
}

// ICON-001: a file with the right name that is not an image is not used.
#[test]
fn ignores_undecodable_file() {
    let root = tempfile::tempdir().unwrap();
    let icon = root
        .path()
        .join("app")
        .join("assets")
        .join("application-icon.png");
    write(&icon, b"hello");
    assert_eq!(
        library(root.path()).discover(&root.path().join("app")),
        None
    );
}

#[test]
fn absent_icon_gives_none() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(library(root.path()).discover(root.path()), None);
}

// ICON-005
#[test]
fn readable_means_a_known_image_signature() {
    let root = tempfile::tempdir().unwrap();
    let library = library(root.path());
    for (name, bytes, readable) in [
        ("a.png", PNG, true),
        ("a.jpg", JPEG, true),
        ("a.ico", ICO, true),
        ("a.gif", b"GIF89a".as_slice(), false),
        ("empty.png", b"".as_slice(), false),
    ] {
        let path = root.path().join(name);
        write(&path, bytes);
        assert_eq!(library.is_readable(&path), readable, "{name}");
    }
    assert!(!library.is_readable(&root.path().join("missing.png")));
    assert!(!library.is_readable(root.path()));
}

// ICON-004, OQ-6: import copies, replaces an earlier copy of another type and release deletes.
#[test]
fn import_replace_and_release() {
    let root = tempfile::tempdir().unwrap();
    let mut library = library(root.path());
    let id = OperationId::new("op-1").unwrap();
    let other = OperationId::new("op-2").unwrap();
    let png = root.path().join("pics").join("Rocket.PNG");
    let jpeg = root.path().join("pics").join("photo.jpg");
    let bare = root.path().join("pics").join("noextension");
    write(&png, PNG);
    write(&jpeg, JPEG);
    write(&bare, ICO);

    let first = library.import(&id, &png).unwrap();
    assert_eq!(first.file_name().unwrap(), "op-1.png");
    assert_eq!(fs::read(&first).unwrap(), PNG);
    let kept = library.import(&other, &jpeg).unwrap();

    let second = library.import(&id, &jpeg).unwrap();
    assert_eq!(second.file_name().unwrap(), "op-1.jpg");
    assert!(!first.exists());
    let third = library.import(&id, &bare).unwrap();
    assert_eq!(third.file_name().unwrap(), "op-1.png");

    library.release(&id);
    assert!(!third.exists() && !second.exists());
    assert!(kept.exists());
    library.release(&id);
}

#[test]
fn import_refuses_what_is_not_an_image() {
    let root = tempfile::tempdir().unwrap();
    let text = root.path().join("notes.png");
    write(&text, b"hello");
    let error = library(root.path())
        .import(&OperationId::new("op-1").unwrap(), &text)
        .unwrap_err();
    assert!(error.contains("PNG, JPEG or ICO"));
}

#[test]
fn import_reports_an_unwritable_folder() {
    let root = tempfile::tempdir().unwrap();
    let png = root.path().join("a.png");
    write(&png, PNG);
    fs::write(
        root.path().join("data"),
        "a file where the folder should be",
    )
    .unwrap();
    assert!(
        library(root.path())
            .import(&OperationId::new("op-1").unwrap(), &png)
            .is_err()
    );
}
