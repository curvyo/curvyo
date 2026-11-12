//! Reads and writes one material record file (ADR 0055 §2, §3).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::machine_profile::{FeedRate, GENERIC_LASER_ID};
use crate::material::{
    CellIndex, HistoryEntry, HistoryEvent, MAX_HISTORY_ENTRIES, MAX_MATERIAL_NAME_CHARS,
    MachineTie, Material, MaterialSet, Passes, PowerPercent, SetKind, SetSource, Settings,
    Thickness,
};
use crate::record_envelope::{EnvelopeError, is_record_id, open, pretty, record_id};
use crate::test_grid::{GridRange, range_of};

/// The record format this build writes and the newest it reads.
pub const MATERIAL_FORMAT_VERSION: u64 = 1;
/// The largest record text that is read or written (ADR 0055 §3, §4).
pub const MAX_MATERIAL_BYTES: usize = 256 * 1024;
/// The only machine family a material is for.
const KIND_LASER: &str = "laser";

/// Why a material record was skipped (ADR 0055 §3).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MaterialFileError {
    /// Not a valid record: skipped and named with this reason, never rewritten by a read.
    #[error("{0}")]
    Damaged(String),
    /// Written by a newer build: skipped and never written (ADR 0004 §9).
    #[error("Needs a newer Curvyo")]
    Newer,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordJson {
    format_version: u64,
    id: String,
    kind: String,
    modified: u64,
    name: String,
    thickness_mm: f64,
    machine: MachineJson,
    sets: SetsJson,
    history: Vec<EntryJson>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MachineJson {
    id: String,
    name: String,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct SetsJson {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cut: Option<SetJson>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    score: Option<SetJson>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    engrave: Option<SetJson>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SetJson {
    power_pct: f64,
    speed_mm_min: u64,
    passes: u64,
    source: Value,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NumbersJson {
    power_pct: f64,
    speed_mm_min: u64,
    passes: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryJson {
    id: String,
    date: u64,
    event: String,
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    set: Option<NumbersJson>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    grid: Option<GridJson>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cell: Option<CellJson>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// The `grid` of a history entry, also the cells of a remembered grid.
pub(crate) struct GridJson {
    pub(crate) columns: u64,
    pub(crate) rows: u64,
    pub(crate) power_from_pct: f64,
    pub(crate) power_to_pct: f64,
    pub(crate) speed_from_mm_min: u64,
    pub(crate) speed_to_mm_min: u64,
    pub(crate) cell_mm: f64,
    pub(crate) gap_mm: f64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CellJson {
    column: u64,
    row: u64,
}

fn damaged(reason: impl Into<String>) -> MaterialFileError {
    MaterialFileError::Damaged(reason.into())
}

/// Whether `id` may name the machine of a material: a profile UUID or the built-in.
pub(crate) fn is_machine_id(id: &str) -> bool {
    id == GENERIC_LASER_ID || is_record_id(id)
}

/// The material the record file `file_name` holds.
///
/// # Errors
/// [`MaterialFileError::Newer`] for a `format_version` above 1; [`MaterialFileError::Damaged`]
/// with one reason text for every damage of ADR 0055 §3: a text over 256 KiB, a JSON or type
/// error, an unknown key, a value out of range, a `source` naming no fitting entry, more than 500
/// history entries, a kind other than `laser`, or an id that differs from the file name. Nothing
/// is written on read; a speed above a machine's maximum feed is not damage.
pub fn read_material(file_name: &str, text: &str) -> Result<Material, MaterialFileError> {
    let (value, _) = open(
        text,
        MAX_MATERIAL_BYTES,
        MATERIAL_FORMAT_VERSION,
        MATERIAL_FORMAT_VERSION,
    )
    .map_err(|error| match error {
        EnvelopeError::Newer => MaterialFileError::Newer,
        EnvelopeError::TooLarge => damaged("The file is over 256 KiB."),
        EnvelopeError::NotJson | EnvelopeError::BadVersion => {
            damaged("The file is not a material record.")
        }
    })?;
    let record: RecordJson = serde_json::from_value(value).map_err(|error| shape_error(&error))?;
    let thickness = check_fields(file_name, &record)?;
    let history = record
        .history
        .into_iter()
        .map(read_entry)
        .collect::<Result<Vec<_>, _>>()?;
    let mut sets = BTreeMap::new();
    for (kind, set) in [
        (SetKind::Cut, record.sets.cut),
        (SetKind::Score, record.sets.score),
        (SetKind::Engrave, record.sets.engrave),
    ] {
        if let Some(set) = set {
            sets.insert(kind, read_set(kind, &set, &history)?);
        }
    }
    Ok(Material {
        id: record.id,
        modified: record.modified,
        name: record.name,
        thickness,
        machine: MachineTie {
            id: record.machine.id,
            name: record.machine.name,
        },
        sets,
        history,
    })
}

/// The thickness of `record` once its id, kind, name, machine and history length are valid.
fn check_fields(file_name: &str, record: &RecordJson) -> Result<Thickness, MaterialFileError> {
    if record_id(file_name) != Some(record.id.as_str()) {
        return Err(damaged("The id differs from the file name."));
    }
    if record.kind != KIND_LASER {
        return Err(damaged(format!("Kind {} is not laser.", record.kind)));
    }
    let name_chars = record.name.chars().count();
    if record.name.trim().is_empty() || name_chars > MAX_MATERIAL_NAME_CHARS {
        return Err(damaged(format!(
            "The name has {name_chars} characters, not 1 to 60."
        )));
    }
    let thickness = Thickness::from_mm(record.thickness_mm).ok_or_else(|| {
        damaged(format!(
            "Thickness {} is outside 0.1 to 100 mm.",
            record.thickness_mm
        ))
    })?;
    if !is_machine_id(&record.machine.id) {
        return Err(damaged("The machine id is not valid."));
    }
    let machine_chars = record.machine.name.chars().count();
    if machine_chars > MAX_MATERIAL_NAME_CHARS {
        return Err(damaged(format!(
            "The machine name has {machine_chars} characters, not more than 60."
        )));
    }
    if record.history.len() > MAX_HISTORY_ENTRIES {
        return Err(damaged(format!(
            "The history has {} entries, more than 500.",
            record.history.len()
        )));
    }
    Ok(thickness)
}

/// The reason of a JSON shape error: the unknown key by name, else one text.
fn shape_error(error: &serde_json::Error) -> MaterialFileError {
    let message = error.to_string();
    if let Some(rest) = message.strip_prefix("unknown field `")
        && let Some(key) = rest.split('`').next()
    {
        return damaged(format!("Unknown key {key}."));
    }
    damaged("The file is not a material record.")
}

fn read_numbers(power: f64, speed: u64, passes: u64) -> Result<Settings, MaterialFileError> {
    Ok(Settings {
        power: PowerPercent::from_percent(power)
            .ok_or_else(|| damaged(format!("Power {power} is outside 1 to 100.")))?,
        speed: FeedRate::new(speed)
            .ok_or_else(|| damaged(format!("Speed {speed} is outside 1 to 100,000 mm/min.")))?,
        passes: Passes::new(passes)
            .ok_or_else(|| damaged(format!("Passes {passes} are outside 1 to 20.")))?,
    })
}

fn read_set(
    kind: SetKind,
    set: &SetJson,
    history: &[HistoryEntry],
) -> Result<MaterialSet, MaterialFileError> {
    let settings = read_numbers(set.power_pct, set.speed_mm_min, set.passes)?;
    let source = match &set.source {
        Value::String(text) if text == "edited" => SetSource::Edited,
        Value::Object(map) if map.len() == 1 => {
            let test = map.get("test").and_then(Value::as_str).ok_or_else(|| {
                damaged(format!("The {} set's source is not valid.", kind.label()))
            })?;
            let fits = history.iter().any(|entry| {
                entry.id == test
                    && entry.kind == kind
                    && matches!(entry.event, HistoryEvent::Tested { .. })
            });
            if !fits {
                return Err(damaged(format!(
                    "The {} set names no test in the history.",
                    kind.label()
                )));
            }
            SetSource::Test(test.to_owned())
        }
        _ => {
            return Err(damaged(format!(
                "The {} set's source is not valid.",
                kind.label()
            )));
        }
    };
    Ok(MaterialSet { settings, source })
}

fn read_entry(entry: EntryJson) -> Result<HistoryEntry, MaterialFileError> {
    if !is_record_id(&entry.id) {
        return Err(damaged("A history entry has no valid id."));
    }
    let kind = SetKind::parse(&entry.kind)
        .ok_or_else(|| damaged(format!("Unknown set kind {}.", entry.kind)))?;
    let numbers = |set: Option<NumbersJson>| match set {
        Some(set) => read_numbers(set.power_pct, set.speed_mm_min, set.passes),
        None => Err(damaged(format!("A {} entry has no set.", entry.event))),
    };
    let event = match (entry.event.as_str(), entry.grid, entry.cell) {
        ("tested", Some(grid), Some(cell)) => {
            let grid = read_grid(&grid)?;
            let cell = read_cell(&cell, &grid)?;
            HistoryEvent::Tested {
                grid,
                cell,
                set: numbers(entry.set)?,
            }
        }
        ("tested", _, _) => return Err(damaged("A tested entry has no grid or cell.")),
        ("edited", None, None) => HistoryEvent::Edited {
            set: numbers(entry.set)?,
        },
        ("removed", None, None) if entry.set.is_none() => HistoryEvent::Removed,
        ("edited" | "removed", _, _) => {
            return Err(damaged(format!(
                "An {} entry has keys it may not have.",
                entry.event
            )));
        }
        (other, _, _) => return Err(damaged(format!("Unknown history event {other}."))),
    };
    Ok(HistoryEntry {
        id: entry.id,
        date: entry.date,
        kind,
        event,
    })
}

fn read_grid(grid: &GridJson) -> Result<GridRange, MaterialFileError> {
    match serde_json::to_value(grid) {
        Ok(Value::Object(map)) => range_of(&map),
        _ => None,
    }
    .ok_or_else(|| damaged("A test's grid is not valid."))
}

fn read_cell(cell: &CellJson, grid: &GridRange) -> Result<CellIndex, MaterialFileError> {
    let index = |n: u64, count: u8| u8::try_from(n).ok().filter(|n| *n < count);
    match (index(cell.column, grid.columns), index(cell.row, grid.rows)) {
        (Some(column), Some(row)) => Ok(CellIndex { column, row }),
        _ => Err(damaged("A test's cell is outside its grid.")),
    }
}

/// The canonical text of `material` in record format 1 (ADR 0055 §2): fixed key order, two-space
/// indent, trailing newline, so read, write, read is the identity.
#[must_use]
pub fn write_material(material: &Material) -> String {
    let set = |kind| {
        material.sets.get(&kind).map(|set: &MaterialSet| SetJson {
            power_pct: set.settings.power.percent(),
            speed_mm_min: u64::from(set.settings.speed),
            passes: u64::from(set.settings.passes.get()),
            source: match &set.source {
                SetSource::Test(id) => json!({ "test": id }),
                SetSource::Edited => json!("edited"),
            },
        })
    };
    let record = RecordJson {
        format_version: MATERIAL_FORMAT_VERSION,
        id: material.id.clone(),
        kind: KIND_LASER.to_owned(),
        modified: material.modified,
        name: material.name.clone(),
        thickness_mm: material.thickness.mm(),
        machine: MachineJson {
            id: material.machine.id.clone(),
            name: material.machine.name.clone(),
        },
        sets: SetsJson {
            cut: set(SetKind::Cut),
            score: set(SetKind::Score),
            engrave: set(SetKind::Engrave),
        },
        history: material.history.iter().map(write_entry).collect(),
    };
    pretty(&record)
}

fn numbers_json(set: Settings) -> NumbersJson {
    NumbersJson {
        power_pct: set.power.percent(),
        speed_mm_min: u64::from(set.speed),
        passes: u64::from(set.passes.get()),
    }
}

pub(crate) fn grid_json(range: &GridRange) -> GridJson {
    GridJson {
        columns: u64::from(range.columns),
        rows: u64::from(range.rows),
        power_from_pct: range.power_from.percent(),
        power_to_pct: range.power_to.percent(),
        speed_from_mm_min: u64::from(range.speed_from),
        speed_to_mm_min: u64::from(range.speed_to),
        cell_mm: range.cell.as_mm(),
        gap_mm: range.gap.as_mm(),
    }
}

fn write_entry(entry: &HistoryEntry) -> EntryJson {
    let (event, set, grid, cell) = match &entry.event {
        HistoryEvent::Tested { grid, cell, set } => (
            "tested",
            Some(numbers_json(*set)),
            Some(grid_json(grid)),
            Some(CellJson {
                column: u64::from(cell.column),
                row: u64::from(cell.row),
            }),
        ),
        HistoryEvent::Edited { set } => ("edited", Some(numbers_json(*set)), None, None),
        HistoryEvent::Removed => ("removed", None, None, None),
    };
    EntryJson {
        id: entry.id.clone(),
        date: entry.date,
        event: event.to_owned(),
        kind: entry.kind.as_str().to_owned(),
        set,
        grid,
        cell,
    }
}

#[cfg(test)]
#[path = "material_file_tests.rs"]
mod tests;
