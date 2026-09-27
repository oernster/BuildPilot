use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use buildpilot::infrastructure::win32::instance::{Claim, claim, instance_key};

/// Long enough for a waiting thread to be scheduled on a loaded machine.
const SUMMONS_WAIT: Duration = Duration::from_secs(5);

/// A key no other test or running BuildPilot uses.
fn unique_key(test: &str) -> String {
    format!("test/{}/{test}", std::process::id())
}

// DATA-001: the first claim is the instance; a second one finds it running.
#[test]
fn a_second_claim_finds_the_first_running() {
    let key = unique_key("second");
    let Ok(Claim::First(_held)) = claim(&key) else {
        panic!("the first claim should hold the instance");
    };
    assert!(matches!(claim(&key), Ok(Claim::Running)));
}

// DATA-001: once the instance ends, the next BuildPilot becomes the instance.
#[test]
fn the_claim_is_released_with_the_instance() {
    let key = unique_key("release");
    let first = claim(&key).unwrap();
    assert!(matches!(first, Claim::First(_)));
    drop(first);
    assert!(matches!(claim(&key), Ok(Claim::First(_))));
}

// DATA-001: a later BuildPilot's claim summons the running one, once per launch.
#[test]
fn a_later_claim_summons_the_instance() {
    let key = unique_key("summons");
    let Ok(Claim::First(instance)) = claim(&key) else {
        panic!("the first claim should hold the instance");
    };
    let (sender, summoned) = mpsc::channel();
    instance.on_summons(move || sender.send(()).unwrap());

    for _ in 0..2 {
        assert!(matches!(claim(&key), Ok(Claim::Running)));
        summoned.recv_timeout(SUMMONS_WAIT).unwrap();
    }
    assert!(summoned.try_recv().is_err());
}

// DATA-001: the key ignores case and holds no backslash, which a kernel object name cannot.
#[test]
fn the_key_folds_case_and_separators() {
    let key = instance_key(Path::new(r"C:\Users\Someone\AppData\Roaming\BuildPilot"));
    assert_eq!(key, "c:/users/someone/appdata/roaming/buildpilot");
    assert_eq!(
        key,
        instance_key(Path::new(r"c:\USERS\someone\appdata\roaming\buildpilot"))
    );
}
