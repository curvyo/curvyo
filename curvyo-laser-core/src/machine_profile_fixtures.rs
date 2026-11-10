//! The fixture set of a machine profile record, format 2 (ADR 0050 §1 to §3): its JSON form and
//! the strict check that makes a damaged set damage the whole record.

use curvyo_document_core::{Affine, FixtureParams, FixtureRecord, SlotRecord};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::fixture_kind::{FixtureKind, KIND_PREFIX};
use crate::fixture_stored::{params_damage, read_slot};
use crate::machine_profile::is_valid_name;

/// The most fixtures a profile's set holds.
pub const MAX_SET_FIXTURES: usize = 32;
/// The most slots in a profile's whole set.
pub const MAX_SET_SLOTS: usize = 200;
/// The most characters of a fixture or slot name in the set.
const MAX_SET_NAME_CHARS: usize = curvyo_document_core::MAX_FIXTURE_NAME_CHARS;
/// How far from a rotation a stored fixture matrix may be (ADR 0049 §6).
const RIGID_TOLERANCE: f64 = 1e-6;
/// The slot keys besides its name, in the record's order.
const SLOT_KEYS: [&str; 6] = ["shape", "cx_mm", "cy_mm", "w_mm", "h_mm", "d_mm"];

/// One fixture of the set as the record stores it: no id, no eye.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FixtureJson {
    kind: String,
    name: String,
    active: bool,
    params: FixtureParams,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    matrix: Option<[f64; 6]>,
    slots: Vec<SlotJson>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SlotJson {
    name: String,
    shape: String,
    cx_mm: f64,
    cy_mm: f64,
    w_mm: f64,
    h_mm: f64,
    d_mm: f64,
}

/// The set as written: the page's records in order, the kind without its prefix. A fixture of an
/// unknown kind is left out (the set holds only known kinds).
pub(crate) fn write_set(set: &[FixtureRecord]) -> Vec<FixtureJson> {
    set.iter()
        .filter_map(|record| {
            let kind = record.kind.strip_prefix(KIND_PREFIX)?;
            (FixtureKind::from_stored(&record.kind) != FixtureKind::Unknown).then(|| FixtureJson {
                kind: kind.to_owned(),
                name: record.name.clone(),
                active: record.active,
                params: record.params.clone(),
                matrix: record.matrix.map(Affine::to_array),
                slots: record.slots.iter().map(slot_json).collect(),
            })
        })
        .collect()
}

fn slot_json(slot: &SlotRecord) -> SlotJson {
    let number = |key: &str| slot.params.get(key).and_then(Value::as_f64).unwrap_or(0.0);
    SlotJson {
        name: slot.name.clone(),
        shape: slot
            .params
            .get("shape")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        cx_mm: number("cx_mm"),
        cy_mm: number("cy_mm"),
        w_mm: number("w_mm"),
        h_mm: number("h_mm"),
        d_mm: number("d_mm"),
    }
}

/// The set a record holds; `None` when any part of it is damage: an unknown kind, a name over 40
/// characters, a value that does not read, a matrix that is not finite or not rigid, a slot that does not
/// read, or more than 32 fixtures or 200 slots (ADR 0050 §2).
pub(crate) fn read_set(set: Vec<FixtureJson>) -> Option<Vec<FixtureRecord>> {
    let slots: usize = set.iter().map(|fixture| fixture.slots.len()).sum();
    if set.len() > MAX_SET_FIXTURES || slots > MAX_SET_SLOTS {
        return None;
    }
    set.into_iter().map(read_fixture).collect()
}

fn read_fixture(fixture: FixtureJson) -> Option<FixtureRecord> {
    let stored = format!("{KIND_PREFIX}{}", fixture.kind);
    let kind = FixtureKind::from_stored(&stored);
    let matrix = match fixture.matrix {
        None => None,
        Some(entries) => {
            let matrix = Affine::from_array(entries);
            (matrix.is_finite()
                && matrix.is_within_limit()
                && matrix.is_rigid_within(RIGID_TOLERANCE))
            .then_some(matrix)?;
            Some(matrix)
        }
    };
    let valid = kind != FixtureKind::Unknown
        && valid_name(&fixture.name)
        && params_damage(kind, &fixture.params) == 0
        && (kind == FixtureKind::Holder || fixture.slots.is_empty());
    if !valid {
        return None;
    }
    let slots = fixture
        .slots
        .into_iter()
        .map(read_slot_json)
        .collect::<Option<Vec<_>>>()?;
    Some(FixtureRecord {
        kind: stored,
        name: fixture.name,
        active: fixture.active,
        params: fixture.params,
        matrix,
        slots,
    })
}

fn read_slot_json(slot: SlotJson) -> Option<SlotRecord> {
    let values = [
        Value::from(slot.shape),
        Value::from(slot.cx_mm),
        Value::from(slot.cy_mm),
        Value::from(slot.w_mm),
        Value::from(slot.h_mm),
        Value::from(slot.d_mm),
    ];
    let params: FixtureParams = SLOT_KEYS
        .iter()
        .map(|key| (*key).to_owned())
        .zip(values)
        .collect();
    (valid_name(&slot.name) && read_slot(&params).is_some()).then_some(SlotRecord {
        name: slot.name,
        params,
    })
}

fn valid_name(name: &str) -> bool {
    is_valid_name(name) && name.chars().count() <= MAX_SET_NAME_CHARS
}
