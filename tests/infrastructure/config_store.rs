use std::fs;
use std::path::{Path, PathBuf};

use buildpilot::application::ports::{ConfigStore, LoadProblem};
use buildpilot::domain::host::{HostRow, HostTable};
use buildpilot::domain::operation::{
    IconRef, Operation, OperationConfig, OperationId, draft_for_script,
};
use buildpilot::domain::preferences::{Preferences, ThemeChoice, TrayLayout, WindowGeometry};
use buildpilot::domain::step::StepSpec;
use buildpilot::domain::version::Version;
use buildpilot::infrastructure::config_format::CURRENT_SCHEMA;
use buildpilot::infrastructure::config_store::{CONFIG_FILE, JsonConfigStore, SET_ASIDE_SUFFIX};
use serde_json::{Value, json};

fn operation(id: &str, script: &str) -> Operation {
    let mut spec = draft_for_script(Path::new(script), &HostTable::default()).unwrap();
    spec.steps[0].arguments = vec!["--release".to_owned(), "two words".to_owned()];
    spec.icon = IconRef::Chosen(PathBuf::from(r"C:\data\icons\x.png"));
    Operation::new(
        OperationId::new(id).unwrap(),
        OperationConfig::try_from(spec).unwrap(),
    )
}

fn preferences() -> Preferences {
    Preferences {
        theme: ThemeChoice::Dark,
        window: Some(WindowGeometry {
            x: -8,
            y: 20,
            width: 1400,
            height: 900,
        }),
        tray: TrayLayout {
            expanded: true,
            height: Some(260),
        },
        skipped_update: Version::parse("1.2.0"),
        // HOST-001: the host table round-trips, leading arguments and all.
        hosts: HostTable::new(vec![
            HostRow::new(
                "rb",
                PathBuf::from(r"C:\Ruby33\bin\ruby.exe"),
                vec!["-W0".to_owned(), "two words".to_owned()],
            )
            .unwrap(),
            HostRow::new("sh", PathBuf::from("bash.exe"), Vec::new()).unwrap(),
        ])
        .unwrap(),
    }
}

fn file_json(folder: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(folder.join(CONFIG_FILE)).unwrap()).unwrap()
}

// CFG-007: first run has nothing and reports nothing.
#[test]
fn missing_file_is_a_first_run() {
    let folder = tempfile::tempdir().unwrap();
    let loaded = JsonConfigStore::new(folder.path().join("BuildPilot")).load();
    assert!(loaded.operations.is_empty());
    assert!(loaded.problems.is_empty());
    assert_eq!(loaded.preferences, Preferences::default());
}

// CFG-001, CFG-003: every field and the order survive a save and reload.
#[test]
fn every_field_and_the_order_round_trip() {
    let folder = tempfile::tempdir().unwrap();
    let data = folder.path().join("BuildPilot");
    // Amendment 4: several steps and a chosen environment survive too.
    let mut python =
        draft_for_script(Path::new(r"C:\src\py\buildexe.py"), &HostTable::default()).unwrap();
    python.steps.push(StepSpec::for_script(Path::new(
        r"C:\src\py\buildinstaller.py",
    )));
    python.environment = Some("venv_smoke".to_owned());
    // PKG-001: a set installer survives too.
    python.installer = Some(PathBuf::from(r"C:\src\py\dist-installer\PySetup.exe"));
    let operations = vec![
        operation("b", r"C:\src\beta\build.cmd"),
        operation("a", r"C:\src\alpha\build.ps1"),
        Operation::new(
            OperationId::new("p").unwrap(),
            OperationConfig::try_from(python).unwrap(),
        ),
    ];
    let mut store = JsonConfigStore::new(data.clone());
    store.save(&operations, &preferences()).unwrap();
    store.save(&operations, &preferences()).unwrap();
    let loaded = JsonConfigStore::new(data.clone()).load();
    assert_eq!(loaded.operations, operations);
    assert_eq!(loaded.preferences, preferences());
    assert!(loaded.problems.is_empty());
    assert_eq!(store.data_folder(), data);
}

// CFG-004: nothing about a run is stored.
#[test]
fn serialised_form_has_no_runtime_fields() {
    let folder = tempfile::tempdir().unwrap();
    let mut store = JsonConfigStore::new(folder.path().to_path_buf());
    store
        .save(&[operation("a", r"C:\src\alpha\build.ps1")], &preferences())
        .unwrap();
    let document = file_json(folder.path());
    let keys: Vec<&str> = document["operations"][0]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys, ["icon", "id", "name", "steps", "working_dir"]);
    assert_eq!(document["schema"], CURRENT_SCHEMA);
}

// CFG-009, Amendment 4: a schema 1 entry's script and arguments become its one step, written
// back as steps.
#[test]
fn older_schema_migrates() {
    let folder = tempfile::tempdir().unwrap();
    let text = r#"{"schema": 1, "operations": [{"id": "a", "name": "alpha",
        "script": "C:\\src\\alpha\\build.ps1", "arguments": ["-x"],
        "working_dir": "C:\\src\\alpha"}]}"#;
    fs::write(folder.path().join(CONFIG_FILE), text).unwrap();
    let mut store = JsonConfigStore::new(folder.path().to_path_buf());
    let loaded = store.load();
    assert!(loaded.problems.is_empty());
    let step = loaded.operations[0].config().first_step();
    assert_eq!(step.script_path(), Path::new(r"C:\src\alpha\build.ps1"));
    assert_eq!(step.arguments(), ["-x"]);
    store.save(&loaded.operations, &preferences()).unwrap();
    let saved = file_json(folder.path());
    assert_eq!(
        saved["operations"][0]["steps"],
        json!([{"script": "C:\\src\\alpha\\build.ps1", "arguments": ["-x"]}])
    );
    assert!(saved["operations"][0].get("script").is_none());
}

// CFG-005: a malformed entry is reported, the rest load and the entry survives later saves.
#[test]
fn malformed_entry_is_preserved_and_reported() {
    let folder = tempfile::tempdir().unwrap();
    let bad = json!({"id": "bad", "name": 42, "script": "C:\\x.ps1", "working_dir": "C:\\"});
    let document = json!({
        "schema": 1,
        "operations": [
            {"id": "a", "name": "alpha", "script": "C:\\src\\alpha\\build.ps1",
             "working_dir": "C:\\src\\alpha"},
            bad,
            {"id": "c", "name": "gamma", "script": "C:\\src\\gamma\\go.exe",
             "working_dir": "C:\\src\\gamma"},
        ]
    });
    fs::write(folder.path().join(CONFIG_FILE), document.to_string()).unwrap();
    let mut store = JsonConfigStore::new(folder.path().to_path_buf());
    let loaded = store.load();
    assert_eq!(loaded.operations.len(), 2);
    assert_eq!(loaded.problems, [LoadProblem::UnreadableEntries(1)]);
    assert_eq!(loaded.preferences, Preferences::default());

    store.save(&loaded.operations[..1], &preferences()).unwrap();
    let saved = file_json(folder.path());
    let entries = saved["operations"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[1], bad);
}

// CFG-006: a file that is not a config at all is set aside; the next save starts fresh.
#[test]
fn corrupt_file_is_set_aside() {
    let cases: [&[u8]; 4] = [
        b"not json",
        b"[1, 2]",
        b"{\"operations\": []}",
        b"\xff\xfe not utf-8",
    ];
    for bytes in cases {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join(CONFIG_FILE);
        fs::write(&file, bytes).unwrap();
        let mut store = JsonConfigStore::new(folder.path().to_path_buf());
        let loaded = store.load();
        let aside = folder
            .path()
            .join(format!("{CONFIG_FILE}{SET_ASIDE_SUFFIX}"));
        assert_eq!(loaded.problems, [LoadProblem::SetAside(aside.clone())]);
        assert_eq!(fs::read(&aside).unwrap(), bytes);
        store.save(&[], &Preferences::default()).unwrap();
        assert!(file.is_file());
    }
}

#[test]
fn operations_that_are_not_a_list_set_the_file_aside() {
    let folder = tempfile::tempdir().unwrap();
    fs::write(
        folder.path().join(CONFIG_FILE),
        r#"{"schema": 1, "operations": {}}"#,
    )
    .unwrap();
    let loaded = JsonConfigStore::new(folder.path().to_path_buf()).load();
    assert!(matches!(loaded.problems[..], [LoadProblem::SetAside(_)]));
}

#[test]
fn unreadable_preferences_fall_back_to_defaults() {
    let folder = tempfile::tempdir().unwrap();
    fs::write(
        folder.path().join(CONFIG_FILE),
        r#"{"schema": 1, "preferences": {"theme": "purple"}}"#,
    )
    .unwrap();
    let loaded = JsonConfigStore::new(folder.path().to_path_buf()).load();
    assert_eq!(loaded.preferences, Preferences::default());
    assert!(loaded.problems.is_empty());
}

// HOST-001: a hand-edited mistake costs its own row, not the table; with no rows, nothing is
// written, so a file without hosts reads the same before and after.
#[test]
fn bad_host_rows_are_dropped_and_an_empty_table_is_not_written() {
    let folder = tempfile::tempdir().unwrap();
    let document = json!({"schema": 2, "preferences": {"hosts": [
        {"extension": "rb", "program": "C:\\Ruby33\\bin\\ruby.exe"},
        {"extension": "r b", "program": "x.exe"},
        {"extension": ".RB", "program": "jruby.exe"},
        {"extension": "sh", "program": "bash.exe", "arguments": ["-e"]},
    ]}});
    fs::write(folder.path().join(CONFIG_FILE), document.to_string()).unwrap();
    let mut store = JsonConfigStore::new(folder.path().to_path_buf());
    let loaded = store.load();
    let rows = loaded.preferences.hosts.rows();
    let kept: Vec<(&str, &Path)> = rows
        .iter()
        .map(|row| (row.extension(), row.program()))
        .collect();
    assert_eq!(
        kept,
        [
            ("rb", Path::new(r"C:\Ruby33\bin\ruby.exe")),
            ("sh", Path::new("bash.exe")),
        ]
    );
    assert_eq!(rows[1].arguments(), ["-e"]);

    store.save(&[], &Preferences::default()).unwrap();
    assert!(
        file_json(folder.path())["preferences"]
            .get("hosts")
            .is_none()
    );
}

// CFG-009: a file from a newer BuildPilot is read but never overwritten.
#[test]
fn newer_schema_is_read_only() {
    let folder = tempfile::tempdir().unwrap();
    let newer = CURRENT_SCHEMA + 1;
    let text = format!(
        r#"{{"schema": {newer}, "operations": [{{"id": "a", "name": "alpha",
        "steps": [{{"script": "C:\\src\\alpha\\build.ps1"}}], "working_dir": "C:\\src\\alpha"}}]}}"#
    );
    fs::write(folder.path().join(CONFIG_FILE), &text).unwrap();
    let mut store = JsonConfigStore::new(folder.path().to_path_buf());
    let loaded = store.load();
    assert_eq!(loaded.problems, [LoadProblem::NewerSchema]);
    assert_eq!(loaded.operations.len(), 1);
    let error = store
        .save(&loaded.operations, &Preferences::default())
        .unwrap_err();
    assert!(error.message.contains("newer BuildPilot"));
    assert_eq!(
        fs::read_to_string(folder.path().join(CONFIG_FILE)).unwrap(),
        text
    );
}

// CFG-008: a folder that cannot be created is a save error, never a crash.
#[test]
fn unwritable_folder_is_a_save_error() {
    let folder = tempfile::tempdir().unwrap();
    let blocker = folder.path().join("BuildPilot");
    fs::write(&blocker, "a file where the folder should be").unwrap();
    let mut store = JsonConfigStore::new(blocker.clone());
    let error = store.save(&[], &Preferences::default()).unwrap_err();
    assert_eq!(error.path, blocker.join(CONFIG_FILE));
    assert!(!error.message.is_empty());
}

// CFG-002: a write that fails part way leaves the previous file intact.
#[test]
fn interrupted_write_keeps_previous_file() {
    let folder = tempfile::tempdir().unwrap();
    let mut store = JsonConfigStore::new(folder.path().to_path_buf());
    store
        .save(&[operation("a", r"C:\src\alpha\build.ps1")], &preferences())
        .unwrap();
    let before = fs::read_to_string(folder.path().join(CONFIG_FILE)).unwrap();
    fs::create_dir(folder.path().join(format!("{CONFIG_FILE}.tmp"))).unwrap();
    assert!(store.save(&[], &Preferences::default()).is_err());
    assert_eq!(
        fs::read_to_string(folder.path().join(CONFIG_FILE)).unwrap(),
        before
    );
}

// A file that exists but cannot be read is left alone and saving is refused.
#[test]
fn a_file_that_cannot_be_opened_freezes_the_store() {
    let folder = tempfile::tempdir().unwrap();
    fs::create_dir(folder.path().join(CONFIG_FILE)).unwrap();
    let mut store = JsonConfigStore::new(folder.path().to_path_buf());
    let loaded = store.load();
    assert!(matches!(
        loaded.problems[..],
        [LoadProblem::Unreadable { .. }]
    ));
    assert!(store.save(&[], &Preferences::default()).is_err());
    assert!(folder.path().join(CONFIG_FILE).is_dir());
}
