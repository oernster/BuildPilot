//! The config file's JSON shape and its translation to and from domain types (SRS 3.16).
//!
//! The file is parsed entry by entry rather than as one document, so one bad operation cannot
//! stop the rest loading (CFG-005). An entry that cannot be read is kept as raw JSON and written
//! back unchanged.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::domain::host::{HostRow, HostTable};
use crate::domain::operation::{IconRef, Operation, OperationConfig, OperationId, OperationSpec};
use crate::domain::preferences::{Preferences, ThemeChoice, TrayLayout, WindowGeometry};
use crate::domain::step::StepSpec;
use crate::domain::version::Version;

/// The schema this build writes (CFG-009). Schema 2 replaced one script with steps.
pub const CURRENT_SCHEMA: u64 = 2;

const SCHEMA_KEY: &str = "schema";
const PREFERENCES_KEY: &str = "preferences";
const OPERATIONS_KEY: &str = "operations";

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
enum ThemeDto {
    Light,
    Dark,
    #[default]
    FollowWindows,
}

#[derive(Serialize, Deserialize)]
struct WindowDto {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

#[derive(Serialize, Deserialize, Default)]
struct TrayDto {
    #[serde(default)]
    expanded: bool,
    #[serde(default)]
    height: Option<u32>,
}

#[derive(Serialize, Deserialize, Default)]
struct PreferencesDto {
    #[serde(default)]
    theme: ThemeDto,
    #[serde(default)]
    window: Option<WindowDto>,
    #[serde(default)]
    tray: TrayDto,
    // Written only once a release is skipped, so older files and newer ones read alike.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    skipped_update: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    hosts: Vec<HostDto>,
}

#[derive(Serialize, Deserialize)]
struct HostDto {
    extension: String,
    program: PathBuf,
    #[serde(default)]
    arguments: Vec<String>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum IconDto {
    #[default]
    Placeholder,
    Discovered {
        path: PathBuf,
    },
    Chosen {
        path: PathBuf,
    },
}

#[derive(Serialize, Deserialize)]
struct StepDto {
    script: PathBuf,
    #[serde(default)]
    arguments: Vec<String>,
}

/// An operation entry. Schema 1 held one `script` with its `arguments`; schema 2 holds `steps`.
/// Both read, as one step and as the list (CFG-009); only `steps` is written.
#[derive(Serialize, Deserialize)]
struct OperationDto {
    id: String,
    name: String,
    #[serde(default)]
    steps: Vec<StepDto>,
    #[serde(default, skip_serializing)]
    script: Option<PathBuf>,
    #[serde(default, skip_serializing)]
    arguments: Vec<String>,
    working_dir: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    environment: Option<String>,
    #[serde(default)]
    icon: IconDto,
    // Written only once an installer is set (PKG-001), so the schema stays 2: an earlier
    // BuildPilot reads the entry and ignores the field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    installer: Option<PathBuf>,
}

/// A config file as read.
#[derive(Debug, Default)]
pub struct ParsedDocument {
    /// The schema the file declares.
    pub schema: u64,
    /// The operations that could be read, in file order.
    pub operations: Vec<Operation>,
    /// The operation entries that could not be read, verbatim.
    pub unreadable: Vec<Value>,
    /// The preferences; defaults when absent or unreadable.
    pub preferences: Preferences,
}

/// Parses `text`. `None` means the file as a whole is not a BuildPilot config: not JSON, not an
/// object, no schema number or an operations field that is not a list (CFG-006).
pub fn parse(text: &str) -> Option<ParsedDocument> {
    let Value::Object(document) = serde_json::from_str::<Value>(text).ok()? else {
        return None;
    };
    let schema = document.get(SCHEMA_KEY)?.as_u64()?;
    let entries = match document.get(OPERATIONS_KEY) {
        None => Vec::new(),
        Some(Value::Array(entries)) => entries.clone(),
        Some(_) => return None,
    };
    let preferences = document
        .get(PREFERENCES_KEY)
        .and_then(|value| serde_json::from_value::<PreferencesDto>(value.clone()).ok())
        .map(preferences_from_dto)
        .unwrap_or_default();
    let mut parsed = ParsedDocument {
        schema,
        preferences,
        ..ParsedDocument::default()
    };
    for entry in entries {
        match operation_from_value(&entry) {
            Some(operation) => parsed.operations.push(operation),
            None => parsed.unreadable.push(entry),
        }
    }
    Some(parsed)
}

/// The file text for `operations` and `preferences`, followed by the `unreadable` entries
/// unchanged.
pub fn render(operations: &[Operation], preferences: &Preferences, unreadable: &[Value]) -> String {
    let mut entries: Vec<Value> = operations
        .iter()
        .map(|operation| serde_json::to_value(operation_to_dto(operation)))
        .collect::<Result<_, _>>()
        .expect("operation DTOs always serialise");
    entries.extend(unreadable.iter().cloned());
    let mut document = Map::new();
    document.insert(SCHEMA_KEY.to_owned(), Value::from(CURRENT_SCHEMA));
    document.insert(
        PREFERENCES_KEY.to_owned(),
        serde_json::to_value(preferences_to_dto(preferences)).expect("preferences serialise"),
    );
    document.insert(OPERATIONS_KEY.to_owned(), Value::Array(entries));
    serde_json::to_string_pretty(&Value::Object(document)).expect("a JSON value serialises")
}

fn operation_from_value(entry: &Value) -> Option<Operation> {
    let dto = serde_json::from_value::<OperationDto>(entry.clone()).ok()?;
    let id = OperationId::new(dto.id).ok()?;
    let mut steps: Vec<StepSpec> = dto
        .steps
        .into_iter()
        .map(|step| StepSpec {
            script_path: step.script,
            arguments: step.arguments,
        })
        .collect();
    // Schema 1: the one script and its arguments become the only step.
    if let Some(script) = dto.script.filter(|_| steps.is_empty()) {
        steps.push(StepSpec {
            script_path: script,
            arguments: dto.arguments,
        });
    }
    let config = OperationConfig::try_from(OperationSpec {
        name: dto.name,
        steps,
        working_dir: dto.working_dir,
        environment: dto.environment,
        icon: match dto.icon {
            IconDto::Placeholder => IconRef::Placeholder,
            IconDto::Discovered { path } => IconRef::Discovered(path),
            IconDto::Chosen { path } => IconRef::Chosen(path),
        },
        installer: dto.installer,
    })
    .ok()?;
    Some(Operation::new(id, config))
}

fn operation_to_dto(operation: &Operation) -> OperationDto {
    let config = operation.config();
    OperationDto {
        id: operation.id().as_str().to_owned(),
        name: config.name().to_owned(),
        steps: config
            .steps()
            .iter()
            .map(|step| StepDto {
                script: step.script_path().to_path_buf(),
                arguments: step.arguments().to_vec(),
            })
            .collect(),
        script: None,
        arguments: Vec::new(),
        working_dir: config.working_dir().to_path_buf(),
        environment: config.environment().map(str::to_owned),
        icon: match config.icon() {
            IconRef::Placeholder => IconDto::Placeholder,
            IconRef::Discovered(path) => IconDto::Discovered { path: path.clone() },
            IconRef::Chosen(path) => IconDto::Chosen { path: path.clone() },
        },
        installer: config.installer().map(Path::to_path_buf),
    }
}

fn preferences_from_dto(dto: PreferencesDto) -> Preferences {
    Preferences {
        theme: match dto.theme {
            ThemeDto::Light => ThemeChoice::Light,
            ThemeDto::Dark => ThemeChoice::Dark,
            ThemeDto::FollowWindows => ThemeChoice::FollowWindows,
        },
        window: dto.window.map(|window| WindowGeometry {
            x: window.x,
            y: window.y,
            width: window.width,
            height: window.height,
        }),
        tray: TrayLayout {
            expanded: dto.tray.expanded,
            height: dto.tray.height,
        },
        skipped_update: dto.skipped_update.as_deref().and_then(Version::parse),
        hosts: hosts_from_dtos(dto.hosts),
    }
}

/// The host table as stored. A row that fails validation is dropped, as is a second row for one
/// extension, so a hand-edited mistake costs that row and not the table.
fn hosts_from_dtos(dtos: Vec<HostDto>) -> HostTable {
    let mut rows: Vec<HostRow> = Vec::new();
    for dto in dtos {
        let Ok(row) = HostRow::new(&dto.extension, dto.program, dto.arguments) else {
            continue;
        };
        if !rows.iter().any(|kept| kept.extension() == row.extension()) {
            rows.push(row);
        }
    }
    HostTable::new(rows).expect("a second row for an extension was dropped")
}

fn preferences_to_dto(preferences: &Preferences) -> PreferencesDto {
    PreferencesDto {
        theme: match preferences.theme {
            ThemeChoice::Light => ThemeDto::Light,
            ThemeChoice::Dark => ThemeDto::Dark,
            ThemeChoice::FollowWindows => ThemeDto::FollowWindows,
        },
        window: preferences.window.map(|window| WindowDto {
            x: window.x,
            y: window.y,
            width: window.width,
            height: window.height,
        }),
        tray: TrayDto {
            expanded: preferences.tray.expanded,
            height: preferences.tray.height,
        },
        skipped_update: preferences
            .skipped_update
            .map(|version| version.to_string()),
        hosts: preferences
            .hosts
            .rows()
            .iter()
            .map(|row| HostDto {
                extension: row.extension().to_owned(),
                program: row.program().to_path_buf(),
                arguments: row.arguments().to_vec(),
            })
            .collect(),
    }
}
