//! The fixture set of a machine profile record, formats 2 and 3 (ADR 0050 §1 to §3, ADR 0052 §7):
//! its JSON form and the strict check that makes a damaged set damage the whole record.

use curvyo_document_core::{Affine, FixtureParams, FixtureRecord, JobRecord, SlotRecord};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::fixture_kind::{FixtureKind, KIND_PREFIX};
use crate::fixture_part::FixturePart;
use crate::fixture_stored::{params_damage, read_slot};
use crate::machine_profile::is_valid_name;
use crate::operation_kind::OperationKind;

/// The most fixtures a profile's set holds.
pub const MAX_SET_FIXTURES: usize = 32;
/// The most slots in a profile's whole set.
pub const MAX_SET_SLOTS: usize = 200;
/// The most fixture jobs in a profile's whole set (ADR 0052 §7).
pub const MAX_SET_JOBS: usize = 100;
/// The most part tokens of all jobs in a profile's whole set.
pub const MAX_SET_TARGETS: usize = 200;
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
    /// The fixture jobs: absent in format 2, required in format 3.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    jobs: Option<Vec<JobJson>>,
}

/// One fixture job of an entry: no id and no Output (ADR 0052 §7).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JobJson {
    kind: String,
    name: String,
    show: bool,
    targets: Vec<String>,
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

/// The set as written (format 3): the page's records in order, the kind without its prefix, each
/// with its `jobs`. A fixture of an unknown kind is left out (the set holds only known kinds), so
/// is a job of an unknown kind or with an invalid name, and a target that is no part of the
/// fixture.
pub(crate) fn write_set(set: &[FixtureRecord]) -> Vec<FixtureJson> {
    set.iter()
        .filter_map(|record| {
            let kind = record.kind.strip_prefix(KIND_PREFIX)?;
            let fixture_kind = FixtureKind::from_stored(&record.kind);
            (fixture_kind != FixtureKind::Unknown).then(|| FixtureJson {
                kind: kind.to_owned(),
                name: record.name.clone(),
                active: record.active,
                params: record.params.clone(),
                matrix: record.matrix.map(Affine::to_array),
                slots: record.slots.iter().map(slot_json).collect(),
                jobs: Some(
                    record
                        .jobs
                        .iter()
                        .filter_map(|job| job_json(fixture_kind, record.slots.len(), job))
                        .collect(),
                ),
            })
        })
        .collect()
}

fn job_json(kind: FixtureKind, slots: usize, job: &JobRecord) -> Option<JobJson> {
    OperationKind::parse(&job.kind)?;
    if !valid_name(&job.name) {
        return None;
    }
    let (parts, _) = crate::fixture_part::job_parts(kind, slots, &job.targets);
    Some(JobJson {
        kind: job.kind.clone(),
        name: job.name.clone(),
        show: job.show,
        targets: parts.into_iter().map(FixturePart::token).collect(),
    })
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
/// read, or more than 32 fixtures or 200 slots (ADR 0050 §2); with `jobs` (format 3) an entry
/// without `jobs`, a job that does not read, or more than 100 jobs or 200 targets, and without
/// (format 2) an entry with `jobs` (ADR 0052 §7).
pub(crate) fn read_set(set: Vec<FixtureJson>, jobs: bool) -> Option<Vec<FixtureRecord>> {
    let slots: usize = set.iter().map(|fixture| fixture.slots.len()).sum();
    let job_lists = set.iter().filter_map(|fixture| fixture.jobs.as_ref());
    let job_count: usize = job_lists.clone().map(Vec::len).sum();
    let targets: usize = job_lists.flatten().map(|job| job.targets.len()).sum();
    if set.len() > MAX_SET_FIXTURES
        || slots > MAX_SET_SLOTS
        || job_count > MAX_SET_JOBS
        || targets > MAX_SET_TARGETS
        || set.iter().any(|fixture| fixture.jobs.is_some() != jobs)
    {
        return None;
    }
    set.into_iter().map(read_fixture).collect()
}

/// A job of an entry of `kind` with `slots` slots; `None` for an unknown kind, an invalid name,
/// or a token that is no part of the entry or is given twice.
fn read_job(kind: FixtureKind, slots: usize, job: JobJson) -> Option<JobRecord> {
    OperationKind::parse(&job.kind)?;
    if !valid_name(&job.name) {
        return None;
    }
    let mut targets = Vec::with_capacity(job.targets.len());
    for token in &job.targets {
        let target = FixturePart::from_token(kind, slots, token)?.target();
        if targets.contains(&target) {
            return None;
        }
        targets.push(target);
    }
    targets.sort_unstable();
    Some(JobRecord {
        kind: job.kind,
        name: job.name,
        show: job.show,
        targets,
    })
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
    let jobs = fixture
        .jobs
        .unwrap_or_default()
        .into_iter()
        .map(|job| read_job(kind, slots.len(), job))
        .collect::<Option<Vec<_>>>()?;
    Some(FixtureRecord {
        kind: stored,
        name: fixture.name,
        active: fixture.active,
        params: fixture.params,
        matrix,
        slots,
        jobs,
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
