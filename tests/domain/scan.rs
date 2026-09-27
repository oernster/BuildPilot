use buildpilot::domain::scan::{PATTERNS, Proposal, looked_for, propose};

fn names(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

// SCAN-001: the two measured patterns, PowerShell first.
#[test]
fn the_patterns_are_the_measured_two() {
    let files: Vec<&[&str]> = PATTERNS.iter().map(|pattern| pattern.files).collect();
    assert_eq!(
        files,
        [
            &["build.ps1"][..],
            &["buildexe.py", "buildinstaller.py"][..]
        ]
    );
    assert!(PATTERNS.iter().all(|pattern| !pattern.name.is_empty()));
    assert_eq!(
        looked_for(),
        ["build.ps1", "buildexe.py", "buildinstaller.py"]
    );
}

// SCAN-003, its acceptance cases.
#[test]
fn the_best_pattern_is_proposed() {
    let python = propose(&names(&["README.md", "buildinstaller.py", "buildexe.py"])).unwrap();
    assert_eq!(python.steps, ["buildexe.py", "buildinstaller.py"]);
    assert!(python.missing.is_empty());
    assert_eq!(python.warning(), None);

    let partial = propose(&names(&["buildruntime.py", "buildinstaller.py"])).unwrap();
    assert_eq!(partial.steps, ["buildinstaller.py"]);
    assert_eq!(partial.missing, ["buildexe.py"]);

    let powershell = propose(&names(&["Build.PS1", "go.mod"])).unwrap();
    assert_eq!(powershell.steps, ["Build.PS1"]);

    assert_eq!(propose(&names(&["Makefile"])), None);
}

// SCAN-003: on a tie the earlier pattern wins; more files present beat fewer.
#[test]
fn ties_go_to_the_earlier_pattern() {
    let tie = propose(&names(&["build.ps1", "buildinstaller.py"])).unwrap();
    assert_eq!(tie.steps, ["build.ps1"]);
    let more = propose(&names(&["build.ps1", "buildexe.py", "buildinstaller.py"])).unwrap();
    assert_eq!(more.steps, ["buildexe.py", "buildinstaller.py"]);
}

// SCAN-009: the words name what is missing and what will run.
#[test]
fn a_partial_match_says_what_is_missing() {
    let one = Proposal {
        steps: names(&["buildinstaller.py"]),
        missing: names(&["buildexe.py"]),
    };
    assert_eq!(
        one.warning().unwrap(),
        "buildexe.py was not found: only buildinstaller.py will run"
    );
    let two = Proposal {
        steps: names(&["c.py"]),
        missing: names(&["a.py", "b.py"]),
    };
    assert_eq!(
        two.warning().unwrap(),
        "a.py, b.py were not found: only c.py will run"
    );
}
