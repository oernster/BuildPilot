use std::path::{Path, PathBuf};

use buildpilot::domain::installer::{
    INSTALLER_FOLDERS, InstallerBlock, find_default_installer, is_default_installer, launchable,
};
use buildpilot::domain::lifecycle::{Failure, RunState};
use buildpilot::domain::operation::OperationConfig;

use super::support::spec;

const PROJECT: &str = r"C:\src\AudioDeck";

/// A listing that answers `names` for `folder` and nothing for any other.
fn listing(folder: &str, names: &[&str]) -> impl Fn(&Path) -> Vec<String> {
    let folder = PathBuf::from(folder);
    let names: Vec<String> = names.iter().map(|name| (*name).to_owned()).collect();
    move |asked: &Path| {
        if asked == folder {
            names.clone()
        } else {
            Vec::new()
        }
    }
}

// PKG-002: the cases measured over the owner's projects, compared by letters and digits alone.
#[test]
fn the_default_name_is_the_folder_then_setup() {
    for (file, project) in [
        ("AudioDeckSetup.exe", "AudioDeck"),
        ("FulcrumSetup.exe", "fulcrum"),
        ("PostalGambitSetup.exe", "postal-gambit"),
        ("HCS-Plugin-BridgeTalk-Setup.exe", "HCS-Plugin-BridgeTalk"),
        ("audiodecksetup.EXE", "AudioDeck"),
    ] {
        assert!(is_default_installer(file, project), "{file} for {project}");
    }
    for (file, project) in [
        ("EDColonisationAsstInstaller.exe", "EDColonisationAsst"),
        ("AudioDeck.exe", "AudioDeck"),
        ("AudioDeckSetup.msi", "AudioDeck"),
        ("AudioDeckSetupexe", "AudioDeck"),
        ("Setup.exe", "---"),
    ] {
        assert!(!is_default_installer(file, project), "{file} for {project}");
    }
}

// PKG-002: dist-installer is searched first, then dist; the file's own spelling is kept.
#[test]
fn dist_installer_comes_before_dist() {
    assert_eq!(INSTALLER_FOLDERS, ["dist-installer", "dist"]);
    let files = |folder: &Path| {
        let mut names = listing(r"C:\src\AudioDeck\dist", &["AudioDeckSetup.exe"])(folder);
        names.extend(listing(
            r"C:\src\AudioDeck\dist-installer",
            &["readme.txt", "audiodecksetup.exe"],
        )(folder));
        names
    };
    assert_eq!(
        find_default_installer(Path::new(PROJECT), &files),
        Some(PathBuf::from(
            r"C:\src\AudioDeck\dist-installer\audiodecksetup.exe"
        ))
    );
}

// PKG-002: dist is the fallback, as for BuildPilot itself.
#[test]
fn dist_is_the_fallback() {
    let files = listing(
        r"C:\src\AudioDeck\dist",
        &["AudioDeck.exe", "AudioDeckSetup.exe"],
    );
    assert_eq!(
        find_default_installer(Path::new(PROJECT), &files),
        Some(PathBuf::from(r"C:\src\AudioDeck\dist\AudioDeckSetup.exe"))
    );
}

// PKG-002: with neither, nothing is found; a drive root has no name to look for.
#[test]
fn nothing_found_leaves_it_unset() {
    let nothing = |_: &Path| Vec::<String>::new();
    assert_eq!(find_default_installer(Path::new(PROJECT), &nothing), None);
    let everything = |_: &Path| vec!["Setup.exe".to_owned()];
    assert_eq!(find_default_installer(Path::new(r"C:\"), &everything), None);
}

// PKG-003, PKG-004: running comes first, then a stopped run, then a failed one, then a missing
// installer.
#[test]
fn launch_follows_the_run_state() {
    let installer = Path::new(r"C:\src\AudioDeck\dist\AudioDeckSetup.exe");
    let running = RunState::Idle.launched(1).unwrap();
    let stopping = running.stop_requested().unwrap();
    for state in [RunState::Idle, RunState::Succeeded] {
        assert_eq!(launchable(&state, Some(installer)), Ok(installer));
        assert_eq!(launchable(&state, None), Err(InstallerBlock::NotFound));
    }
    for state in [&running, &stopping] {
        assert_eq!(
            launchable(state, Some(installer)),
            Err(InstallerBlock::Running)
        );
        assert_eq!(launchable(state, None), Err(InstallerBlock::Running));
    }
    assert_eq!(
        launchable(&RunState::Stopped, Some(installer)),
        Err(InstallerBlock::Stopped)
    );
    // A failed run holds it back, whichever step failed.
    for failure in [
        Failure::ExitCode(1),
        Failure::StepExitCode { step: 2, code: 3 },
    ] {
        assert_eq!(
            launchable(&RunState::Failed(failure), Some(installer)),
            Err(InstallerBlock::Failed)
        );
    }
}

// PKG-003, PKG-004: each reason in the words the tooltip shows.
#[test]
fn every_reason_says_what_to_do() {
    assert_eq!(
        InstallerBlock::NotFound.to_string(),
        "no installer found; set one in Edit"
    );
    assert_eq!(
        InstallerBlock::Running.to_string(),
        "wait for the build to finish"
    );
    assert_eq!(
        InstallerBlock::Stopped.to_string(),
        "the last build was stopped; run it to success first"
    );
    assert_eq!(
        InstallerBlock::Failed.to_string(),
        "the last build failed; run it to success first"
    );
}

// PKG-001: blank is none, relative sits in the working directory, absolute is kept.
#[test]
fn a_set_installer_is_held_in_full() {
    let with = |installer: &str| {
        let mut spec = spec(r"C:\src\AudioDeck\build.ps1");
        spec.installer = Some(PathBuf::from(installer));
        OperationConfig::try_from(spec).unwrap()
    };
    assert_eq!(with("").installer(), None);
    assert_eq!(
        with(r".\dist-installer\AudioDeckSetup.exe").installer(),
        Some(Path::new(
            r"C:\src\AudioDeck\dist-installer\AudioDeckSetup.exe"
        ))
    );
    let absolute = with(r"D:\out\Setup.exe");
    assert_eq!(absolute.installer(), Some(Path::new(r"D:\out\Setup.exe")));
    assert_eq!(
        absolute.to_spec().installer,
        Some(PathBuf::from(r"D:\out\Setup.exe"))
    );
}
