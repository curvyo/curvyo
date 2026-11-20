//! Reads and writes one material record file of format 4 (ADR 0058 §2, §6; ADR 0069 §8; ADR 0072
//! §6).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::machine_profile::FeedRate;
use crate::material::{
    Interval, MAX_HISTORY_ENTRIES, MAX_MATERIAL_NAME_CHARS, MachineTie, Passes, PowerPercent,
    SetKind, SetNumbers, Settings, Thickness,
};
use crate::material_record::{
    HistoryEntry, HistoryEvent, LaserSets, MAX_LASERS, MAX_TESTS, Material, MaterialRead, Set,
    SetSource, TestRecord,
};
use crate::material_test_file::{TestJson, read_tests, test_json};
use crate::record_envelope::is_machine_id;
use crate::record_envelope::{EnvelopeError, is_record_id, open, pretty, record_id};
use crate::test_id::TestId;

/// The record format this build writes and the newest it reads. Format 3 (`specs/0123`, Frage 237
/// A; ADR 0069 §13, §14) lets a test's recipe carry the kind `image` with `dither`, a laser's sets
/// an `image` set and a history entry the kind `image`; a format-2 record reads as is and is
/// written back as format 3 at its next save (no migration), one holding any of these is damaged.
/// Format 4 (`specs/0135` criterion 51, ADR 0072 §6) lets an Image test's recipe carry `picture`
/// `"selected"` and an axis on `strength`; a record of format 2 or 3 reads as is and is written
/// back as format 4 at its next save, one holding either is damaged.
pub const RECORD_FORMAT_VERSION: u64 = 4;
/// The oldest record format this build reads; format 1 is skipped as older.
const OLDEST_READ_FORMAT: u64 = 2;
/// The largest record text that is read or written (ADR 0058 §6).
pub const MAX_RECORD_TEXT: usize = 512 * 1024;
/// The only machine family a material is for.
const KIND_LASER: &str = "laser";

/// Why a material record was skipped (ADR 0058 §6).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RecordFileError {
    /// Not a valid record: skipped and named with this reason, never rewritten by a read.
    #[error("{0}")]
    Damaged(String),
    /// Written by a newer build: skipped and never written.
    #[error("Needs a newer Curvyo")]
    Newer,
    /// Format 1: skipped and never written until its conversion is built (plan Part F).
    #[error("Material has an older format.")]
    Older,
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
    lasers: Vec<LaserJson>,
    tests: Vec<TestJson>,
    history: Vec<EntryJson>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MachineJson {
    id: String,
    name: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LaserJson {
    machine: MachineJson,
    sets: SetsJson,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SetsJson {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cut: Option<SetJson>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    score: Option<SetJson>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    engrave: Option<SetJson>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    image: Option<SetJson>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SetJson {
    power_pct: f64,
    speed_mm_min: u64,
    passes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    interval_mm: Option<f64>,
    source: Value,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NumbersJson {
    power_pct: f64,
    speed_mm_min: u64,
    passes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    interval_mm: Option<f64>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryJson {
    id: String,
    date: u64,
    event: String,
    machine: String,
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    set: Option<NumbersJson>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source: Option<Value>,
}

pub(crate) fn damaged(reason: impl Into<String>) -> RecordFileError {
    RecordFileError::Damaged(reason.into())
}

/// The record file `file_name` as loaded: [`MaterialRead::Loaded`], or the reason it is skipped.
#[must_use]
pub fn load_record(file_name: &str, text: &str) -> MaterialRead {
    let id = record_id(file_name).unwrap_or(file_name).to_owned();
    match read_record(file_name, text) {
        Ok(material) => MaterialRead::Loaded(material),
        Err(RecordFileError::Damaged(reason)) => MaterialRead::Damaged { id, reason },
        Err(RecordFileError::Newer) => MaterialRead::Newer { id },
        Err(RecordFileError::Older) => MaterialRead::Older { id },
    }
}

/// The material the record file `file_name` holds.
///
/// # Errors
/// [`RecordFileError::Newer`] above format 4, [`RecordFileError::Older`] for format 1, and
/// [`RecordFileError::Damaged`] with one reason for every damage of ADR 0055 §3 and ADR 0058 §6.
/// Nothing is written on read.
pub fn read_record(file_name: &str, text: &str) -> Result<Material, RecordFileError> {
    let (value, version) =
        open(text, MAX_RECORD_TEXT, 1, RECORD_FORMAT_VERSION).map_err(|error| match error {
            EnvelopeError::Newer => RecordFileError::Newer,
            EnvelopeError::TooLarge => damaged("The file is over 512 KiB."),
            EnvelopeError::NotJson | EnvelopeError::BadVersion => {
                damaged("The file is not a material record.")
            }
        })?;
    if version < OLDEST_READ_FORMAT {
        return Err(RecordFileError::Older);
    }
    let record: RecordJson = serde_json::from_value(value).map_err(|e| shape_error(&e))?;
    let thickness = check_fields(file_name, &record)?;
    let tests = read_tests(record.tests, version)?;
    let lasers = read_lasers(record.lasers, &tests, version)?;
    let history = record
        .history
        .into_iter()
        .map(|entry| read_entry(entry, version))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Material {
        id: record.id,
        modified: record.modified,
        name: record.name,
        thickness,
        lasers,
        tests,
        history,
    })
}

/// The thickness of `record` once its id, kind, name and list lengths are valid.
fn check_fields(file_name: &str, record: &RecordJson) -> Result<Thickness, RecordFileError> {
    if record_id(file_name) != Some(record.id.as_str()) {
        return Err(damaged("The id differs from the file name."));
    }
    if record.kind != KIND_LASER {
        return Err(damaged(format!("Kind {} is not laser.", record.kind)));
    }
    let chars = record.name.chars().count();
    if record.name.trim().is_empty() || chars > MAX_MATERIAL_NAME_CHARS {
        return Err(damaged(format!(
            "The name has {chars} characters, not 1 to 60."
        )));
    }
    let lengths = [
        (record.lasers.len(), MAX_LASERS, "lasers"),
        (record.tests.len(), MAX_TESTS, "tests"),
        (record.history.len(), MAX_HISTORY_ENTRIES, "history entries"),
    ];
    for (len, max, what) in lengths {
        if len > max {
            return Err(damaged(format!(
                "The record has {len} {what}, more than {max}."
            )));
        }
    }
    Thickness::from_mm(record.thickness_mm).ok_or_else(|| {
        damaged(format!(
            "Thickness {} is outside 0.1 to 100 mm.",
            record.thickness_mm
        ))
    })
}

/// The reason of a JSON shape error: the unknown key by name, else one text.
fn shape_error(error: &serde_json::Error) -> RecordFileError {
    let message = error.to_string();
    if let Some(rest) = message.strip_prefix("unknown field `")
        && let Some(key) = rest.split('`').next()
    {
        return damaged(format!("Unknown key {key}."));
    }
    damaged("The file is not a material record.")
}

pub(crate) fn read_machine(machine: MachineJson) -> Result<MachineTie, RecordFileError> {
    if !is_machine_id(&machine.id) {
        return Err(damaged("A machine id is not valid."));
    }
    if machine.name.chars().count() > MAX_MATERIAL_NAME_CHARS {
        return Err(damaged("A machine name has more than 60 characters."));
    }
    Ok(MachineTie {
        id: machine.id,
        name: machine.name,
    })
}

fn read_numbers(kind: SetKind, set: &NumbersJson) -> Result<SetNumbers, RecordFileError> {
    let (power, speed, passes) = (set.power_pct, set.speed_mm_min, set.passes);
    let settings = Settings {
        power: PowerPercent::from_percent(power)
            .ok_or_else(|| damaged(format!("Power {power} is outside 1 to 100.")))?,
        speed: FeedRate::new(speed)
            .ok_or_else(|| damaged(format!("Speed {speed} is outside 1 to 100,000 mm/min.")))?,
        passes: Passes::new(passes)
            .ok_or_else(|| damaged(format!("Passes {passes} are outside 1 to 20.")))?,
    };
    let interval = match (kind.has_interval(), set.interval_mm) {
        (true, Some(mm)) => Some(
            Interval::from_mm(mm)
                .ok_or_else(|| damaged(format!("Interval {mm} is outside 0.01 to 2.00 mm.")))?,
        ),
        (true, None) => {
            return Err(damaged(format!(
                "{} {} set has no interval.",
                article(kind),
                kind.label()
            )));
        }
        (false, Some(_)) => {
            return Err(damaged(format!("A {} set has an interval.", kind.label())));
        }
        (false, None) => None,
    };
    Ok(SetNumbers { settings, interval })
}

/// "A" or "An" before the label of `kind`.
const fn article(kind: SetKind) -> &'static str {
    match kind {
        SetKind::Engrave | SetKind::Image => "An",
        SetKind::Cut | SetKind::Score => "A",
    }
}

/// The damage of an Image set, entry or test in a record of format 2, which never had them
/// (ADR 0069 §14).
fn image_in_format_2(kind: SetKind, version: u64) -> Result<(), RecordFileError> {
    if kind == SetKind::Image && version < 3 {
        return Err(damaged("A format-2 record holds an Image set."));
    }
    Ok(())
}

fn read_source(value: &Value) -> Result<SetSource, RecordFileError> {
    match value {
        Value::String(text) if text == "typed" => Ok(SetSource::Typed),
        Value::Object(map) if map.len() == 1 => map
            .get("test")
            .and_then(Value::as_str)
            .and_then(TestId::parse)
            .map(SetSource::Test)
            .ok_or_else(|| damaged("A set's source is not valid.")),
        _ => Err(damaged("A set's source is not valid.")),
    }
}

fn read_lasers(
    lasers: Vec<LaserJson>,
    tests: &[TestRecord],
    version: u64,
) -> Result<Vec<LaserSets>, RecordFileError> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::with_capacity(lasers.len());
    for laser in lasers {
        let machine = read_machine(laser.machine)?;
        if !seen.insert(machine.id.clone()) {
            return Err(damaged(format!(
                "The laser {} is listed twice.",
                machine.name
            )));
        }
        let mut sets = BTreeMap::new();
        for (kind, set) in [
            (SetKind::Cut, laser.sets.cut),
            (SetKind::Score, laser.sets.score),
            (SetKind::Engrave, laser.sets.engrave),
            (SetKind::Image, laser.sets.image),
        ] {
            let Some(set) = set else { continue };
            image_in_format_2(kind, version)?;
            let numbers = read_numbers(
                kind,
                &NumbersJson {
                    power_pct: set.power_pct,
                    speed_mm_min: set.speed_mm_min,
                    passes: set.passes,
                    interval_mm: set.interval_mm,
                },
            )?;
            let source = read_source(&set.source)?;
            if let SetSource::Test(id) = source {
                let fits = tests
                    .iter()
                    .any(|t| t.id == id && t.machine.id == machine.id && t.recipe.kind() == kind);
                if !fits {
                    return Err(damaged(format!(
                        "The {} set of {} names no fitting test.",
                        kind.label(),
                        machine.name
                    )));
                }
            }
            sets.insert(kind, Set { numbers, source });
        }
        out.push(LaserSets { machine, sets });
    }
    Ok(out)
}

fn read_entry(entry: EntryJson, version: u64) -> Result<HistoryEntry, RecordFileError> {
    if !is_record_id(&entry.id) {
        return Err(damaged("A history entry has no valid id."));
    }
    if !is_machine_id(&entry.machine) {
        return Err(damaged("A history entry has no valid machine."));
    }
    let kind = SetKind::parse(&entry.kind)
        .ok_or_else(|| damaged(format!("Unknown set kind {}.", entry.kind)))?;
    image_in_format_2(kind, version)?;
    let event = match (entry.event.as_str(), entry.set, entry.source) {
        ("set", Some(set), Some(source)) => HistoryEvent::Set {
            numbers: read_numbers(kind, &set)?,
            source: read_source(&source)?,
        },
        ("removed", None, None) => HistoryEvent::Removed,
        ("set" | "removed", _, _) => {
            return Err(damaged(format!(
                "A {} entry has keys it may not have.",
                entry.event
            )));
        }
        (other, _, _) => return Err(damaged(format!("Unknown history event {other}."))),
    };
    Ok(HistoryEntry {
        id: entry.id,
        date: entry.date,
        machine: entry.machine,
        kind,
        event,
    })
}

pub(crate) fn machine_json(machine: &MachineTie) -> MachineJson {
    MachineJson {
        id: machine.id.clone(),
        name: machine.name.clone(),
    }
}

fn numbers_json(numbers: &SetNumbers) -> NumbersJson {
    NumbersJson {
        power_pct: numbers.settings.power.percent(),
        speed_mm_min: u64::from(numbers.settings.speed),
        passes: u64::from(numbers.settings.passes.get()),
        interval_mm: numbers.interval.map(Interval::mm),
    }
}

fn source_json(source: &SetSource) -> Value {
    match source {
        SetSource::Test(id) => json!({ "test": id.as_str() }),
        SetSource::Typed => json!("typed"),
        SetSource::History(entry) => json!({ "history": entry }),
    }
}

/// The canonical text of `material` in record format 3 (key order of ADR 0058 §2, two-space
/// indent, trailing newline), so read, write, read is the identity.
#[must_use]
pub fn write_record(material: &Material) -> String {
    let set = |laser: &LaserSets, kind| {
        laser.sets.get(&kind).map(|set: &Set| {
            let n = numbers_json(&set.numbers);
            SetJson {
                power_pct: n.power_pct,
                speed_mm_min: n.speed_mm_min,
                passes: n.passes,
                interval_mm: n.interval_mm,
                source: source_json(&set.source),
            }
        })
    };
    let record = RecordJson {
        format_version: RECORD_FORMAT_VERSION,
        id: material.id.clone(),
        kind: KIND_LASER.to_owned(),
        modified: material.modified,
        name: material.name.clone(),
        thickness_mm: material.thickness.mm(),
        lasers: material
            .lasers
            .iter()
            .map(|laser| LaserJson {
                machine: machine_json(&laser.machine),
                sets: SetsJson {
                    cut: set(laser, SetKind::Cut),
                    score: set(laser, SetKind::Score),
                    engrave: set(laser, SetKind::Engrave),
                    image: set(laser, SetKind::Image),
                },
            })
            .collect(),
        tests: material.tests.iter().map(test_json).collect(),
        history: material
            .history
            .iter()
            .map(|entry| {
                let (event, set, source) = match &entry.event {
                    HistoryEvent::Set { numbers, source } => (
                        "set",
                        Some(numbers_json(numbers)),
                        Some(source_json(source)),
                    ),
                    HistoryEvent::Removed => ("removed", None, None),
                };
                EntryJson {
                    id: entry.id.clone(),
                    date: entry.date,
                    event: event.to_owned(),
                    machine: entry.machine.clone(),
                    kind: entry.kind.as_str().to_owned(),
                    set,
                    source,
                }
            })
            .collect(),
    };
    pretty(&record)
}
