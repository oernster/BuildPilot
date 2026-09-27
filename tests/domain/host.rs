use std::path::{Path, PathBuf};

use buildpilot::domain::environment::Need;
use buildpilot::domain::host::{Host, HostError, HostRow, HostTable};
use buildpilot::domain::launch_plan::{ScriptKind, need_of};
use buildpilot::domain::operation::OperationError;

use super::support::hosts;

fn row(extension: &str, program: &str) -> Result<HostRow, HostError> {
    HostRow::new(extension, PathBuf::from(program), Vec::new())
}

// HOST-001: the extension is stored lower case without its dot, however it was typed.
#[test]
fn extension_is_normalised() {
    for typed in ["rb", ".rb", " .RB ", "Rb"] {
        assert_eq!(row(typed, "ruby.exe").unwrap().extension(), "rb", "{typed}");
    }
}

// HOST-001: a row keeps its program and leading arguments as given.
#[test]
fn row_keeps_program_and_arguments() {
    let arguments = vec!["-W0".to_owned(), "two words".to_owned()];
    let row = HostRow::new(
        "rb",
        PathBuf::from(r"C:\Ruby33\bin\ruby.exe"),
        arguments.clone(),
    )
    .unwrap();
    assert_eq!(row.program(), Path::new(r"C:\Ruby33\bin\ruby.exe"));
    assert_eq!(row.arguments(), arguments);
}

// HOST-001: each refusal names the row and what to do.
#[test]
fn invalid_rows_are_refused() {
    let cases = [
        (
            row("", "ruby.exe"),
            HostError::EmptyExtension,
            "no file type",
        ),
        (
            row(".", "ruby.exe"),
            HostError::EmptyExtension,
            "no file type",
        ),
        (
            row("r b", "ruby.exe"),
            HostError::BadExtension("r b".to_owned()),
            "letters and digits",
        ),
        (
            row("rb", ""),
            HostError::EmptyProgram("rb".to_owned()),
            "The host for .rb has no program",
        ),
        (
            row("rb", r"bin\ruby.exe"),
            HostError::ProgramNotAbsolute(PathBuf::from(r"bin\ruby.exe")),
            "relative",
        ),
    ];
    for (result, expected, wording) in cases {
        let error = result.unwrap_err();
        assert_eq!(error, expected);
        assert!(error.to_string().contains(wording), "{error}");
    }
}

// HOST-001: a full path or a bare name found on PATH, like pwsh.exe in LCH-001.
#[test]
fn program_is_a_full_path_or_a_bare_name() {
    assert!(row("rb", r"C:\Ruby33\bin\ruby.exe").is_ok());
    assert!(row("rb", "ruby.exe").is_ok());
}

// HOST-001: one row per extension.
#[test]
fn second_row_for_an_extension_is_refused() {
    let error = HostTable::new(vec![
        row("rb", "ruby.exe").unwrap(),
        row("sh", "bash.exe").unwrap(),
        row(".RB", "jruby.exe").unwrap(),
    ])
    .unwrap_err();
    assert_eq!(error, HostError::Duplicate("rb".to_owned()));
    assert!(error.to_string().contains("Two hosts are set for .rb"));
}

// HOST-001: rows keep the order given.
#[test]
fn rows_keep_their_order() {
    let table = hosts(&[("sh", "bash.exe"), ("rb", "ruby.exe")]);
    let extensions: Vec<&str> = table.rows().iter().map(HostRow::extension).collect();
    assert_eq!(extensions, ["sh", "rb"]);
}

// HOST-002: the row is found whatever the case of the script's extension.
#[test]
fn find_ignores_case() {
    let table = hosts(&[("rb", "ruby.exe")]);
    let found = table.find(Path::new(r"C:\src\app\BUILD.RB")).unwrap();
    assert_eq!(found.program(), Path::new("ruby.exe"));
    assert!(table.find(Path::new(r"C:\src\app\build")).is_none());
}

// A Windows path may hold unpaired UTF-16 surrogates; such an extension has no row, not a
// crash.
#[cfg(windows)]
#[test]
fn non_unicode_extension_has_no_row() {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    const LONE_SURROGATE: u16 = 0xD800;
    let mut wide: Vec<u16> = "a.".encode_utf16().collect();
    wide.push(LONE_SURROGATE);
    let path = PathBuf::from(OsString::from_wide(&wide));
    assert!(hosts(&[("rb", "ruby.exe")]).find(&path).is_none());
}

// HOST-002: the operator's row first, even for a type BuildPilot knows; then the built-in rule.
#[test]
fn operator_row_wins_over_the_built_in_rule() {
    let table = hosts(&[("rb", "ruby.exe"), ("py", r"C:\Python313\python.exe")]);
    let ruby = table.host_for(Path::new(r"C:\a\build.rb")).unwrap();
    assert!(matches!(ruby, Host::Operator(row) if row.extension() == "rb"));
    let python = table.host_for(Path::new(r"C:\a\buildexe.py")).unwrap();
    assert!(matches!(python, Host::Operator(row) if row.extension() == "py"));
    assert_eq!(
        table.host_for(Path::new(r"C:\a\build.ps1")),
        Some(Host::BuiltIn(ScriptKind::PowerShell))
    );
    assert_eq!(table.host_for(Path::new(r"C:\a\notes.txt")), None);
}

// LCH-001: a type nothing runs is refused, listing the operator's types too.
#[test]
fn check_refuses_a_type_nothing_runs() {
    let table = hosts(&[("rb", "ruby.exe")]);
    assert_eq!(
        table.check(Path::new(r"C:\a\build.cmd")),
        Ok(Host::BuiltIn(ScriptKind::Batch))
    );
    let error = table.check(Path::new(r"C:\a\notes.txt")).unwrap_err();
    assert_eq!(
        error,
        OperationError::UnsupportedScriptType {
            path: PathBuf::from(r"C:\a\notes.txt"),
            supported: ".ps1, .bat, .cmd, .exe, .com, .py, .rb".to_owned(),
        }
    );
    assert!(
        error
            .to_string()
            .contains("Add a host for its type in Settings")
    );
}

// HOST-003: the pickers offer the built-in types then the operator's, each once.
#[test]
fn extensions_list_built_in_then_operator_each_once() {
    let table = hosts(&[("rb", "ruby.exe"), ("py", "python.exe"), ("sh", "bash.exe")]);
    assert_eq!(
        table.extensions(),
        ["ps1", "bat", "cmd", "exe", "com", "py", "rb", "sh"]
    );
    assert_eq!(
        HostTable::default().supported_list(),
        ".ps1, .bat, .cmd, .exe, .com, .py"
    );
}

// HOST-002: an operator's program is never activated, so it needs no environment.
#[test]
fn operator_host_needs_no_environment() {
    let table = hosts(&[("py", "python.exe")]);
    let row = &table.rows()[0];
    assert_eq!(Host::Operator(row).environment_need(), Need::None);
    assert_eq!(
        Host::BuiltIn(ScriptKind::Python).environment_need(),
        Need::Required
    );
    assert_eq!(
        need_of([Host::Operator(row), Host::BuiltIn(ScriptKind::PowerShell)]),
        Need::Optional
    );
    assert_eq!(need_of([]), Need::None);
}
