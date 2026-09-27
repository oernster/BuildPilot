//! The config file's JSON shape and its translation to and from domain types (SRS 3.16).
//!
//! The file is parsed entry by entry rather than as one document, so one bad operation cannot
//! stop the rest loading (CFG-005). An entry that cannot be read is kept as raw JSON and written
//! back unchanged.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::domain::operation::{IconRef, Operation, OperationConfig, OperationId, OperationSpec};
use crate::domain::preferences::{Preferences, ThemeChoice, TrayLayout, WindowGeometry};

/// The schema this build writes (CFG-009).
pub const CURRENT_SCHEMA: u64 = 1;

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
struct OperationDto {
    id: String,
    name: String,
    script: PathBuf,
    working_dir: PathBuf,
    #[serde(default)]
    arguments: Vec<String>,
    #[serde(default)]
    icon: IconDto,
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
    let config = OperationConfig::try_from(OperationSpec {
        name: dto.name,
        script_path: dto.script,
        working_dir: dto.working_dir,
        arguments: dto.arguments,
        icon: match dto.icon {
            IconDto::Placeholder => IconRef::Placeholder,
            IconDto::Discovered { path } => IconRef::Discovered(path),
            IconDto::Chosen { path } => IconRef::Chosen(path),
        },
    })
    .ok()?;
    Some(Operation::new(id, config))
}

fn operation_to_dto(operation: &Operation) -> OperationDto {
    let config = operation.config();
    OperationDto {
        id: operation.id().as_str().to_owned(),
        name: config.name().to_owned(),
        script: config.script_path().to_path_buf(),
        working_dir: config.working_dir().to_path_buf(),
        arguments: config.arguments().to_vec(),
        icon: match config.icon() {
            IconRef::Placeholder => IconDto::Placeholder,
            IconRef::Discovered(path) => IconDto::Discovered { path: path.clone() },
            IconRef::Chosen(path) => IconDto::Chosen { path: path.clone() },
        },
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
    }
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
    }
}
