//! The fixture entries of a machine profile record (ADR 0050 §1 to §3, ADR 0052 §7, ADR 0060 §1):
//! their JSON form and the strict check that makes a damaged entry damage the whole record.

use curvyo_document_core::{
    Affine, FixtureParams, FixtureRecord, JobRecord, JobRole, JobTarget, SlotRecord,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::fixture_kind::{FixtureKind, KIND_PREFIX};
use crate::fixture_part::{FixturePart, job_parts};
use crate::fixture_stored::{params_damage, read_slot, translation_part};
use crate::machine_profile::is_valid_name;
use crate::operation_kind::OperationKind;

/// The most characters of a fixture, slot or set name in a record.
const MAX_ENTRY_NAME_CHARS: usize = curvyo_document_core::MAX_FIXTURE_NAME_CHARS;
/// How far from a rotation a stored fixture matrix may be (ADR 0049 §6).
const RIGID_TOLERANCE: f64 = 1e-6;
/// The slot keys besides its name, in the record's order.
const SLOT_KEYS: [&str; 6] = ["shape", "cx_mm", "cy_mm", "w_mm", "h_mm", "d_mm"];
/// The kind of both role jobs (`specs/0113-fixture-sets-and-pins` criterion 26).
const ROLE_JOB_KIND: &str = "score";
/// The roles every pin and screw entry holds one job of, in the order a missing one is added.
const ROLES: [JobRole; 2] = [JobRole::CentreMark, JobRole::Diameters];

/// One fixture entry as the record stores it: its id and Lock, no eye and no On the bed (ADR 0060
/// §1 amended 2026-11-16).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FixtureJson {
    id: String,
    kind: String,
    name: String,
    locked: bool,
    params: FixtureParams,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    matrix: Option<[f64; 6]>,
    slots: Vec<SlotJson>,
    jobs: Vec<JobJson>,
}

impl FixtureJson {
    /// The number of slots of the entry.
    pub(crate) fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// The number of jobs of the entry.
    pub(crate) fn job_count(&self) -> usize {
        self.jobs.len()
    }

    /// The number of part tokens of all jobs of the entry.
    pub(crate) fn target_count(&self) -> usize {
        self.jobs.iter().map(|job| job.targets.len()).sum()
    }

    /// The ids of the entry and of its jobs.
    pub(crate) fn ids(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.id.as_str()).chain(self.jobs.iter().map(|job| job.id.as_str()))
    }
}

/// One fixture job of an entry: its id, no Output (ADR 0052 §7); a pin's or a screw's role
/// (ADR 0060 §1).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JobJson {
    id: String,
    kind: String,
    name: String,
    show: bool,
    targets: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    role: Option<String>,
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

/// Whether `kind` owns the two role jobs (a pin or a screw).
fn has_roles(kind: FixtureKind) -> bool {
    matches!(kind, FixtureKind::Pin | FixtureKind::Screw)
}

/// The entry as written: the kind without its prefix, its jobs; `None` for a fixture of an
/// unknown kind (a record holds only known kinds). A job of an unknown kind or with an invalid
/// name is left out, so is a target that is no part of the fixture. A pin or a screw is written
/// with exactly one role job of each role, fixed kind and ticks, a missing one added with its
/// default name; a pin keeps no other job; a role on another kind is dropped. The matrix of a pin
/// or a screw is written as its translation part.
pub(crate) fn write_fixture(record: &FixtureRecord) -> Option<FixtureJson> {
    let kind_text = record.kind.strip_prefix(KIND_PREFIX)?;
    let kind = FixtureKind::from_stored(&record.kind);
    if kind == FixtureKind::Unknown {
        return None;
    }
    let slots = record.slots.len();
    let mut jobs = Vec::with_capacity(record.jobs.len() + ROLES.len());
    let mut written_roles = Vec::new();
    for job in &record.jobs {
        match job.role.filter(|_| has_roles(kind)) {
            Some(role) if !written_roles.contains(&role) && valid_name(&job.name) => {
                written_roles.push(role);
                jobs.push(role_job(kind, role, &job.id, &job.name, job.show));
            }
            Some(_) => {}
            None if kind == FixtureKind::Pin => {}
            None => jobs.extend(job_json(kind, slots, job)),
        }
    }
    if has_roles(kind) {
        for role in ROLES.into_iter().filter(|r| !written_roles.contains(r)) {
            jobs.push(role_job(kind, role, "", role_name(role), true));
        }
    }
    let matrix = record.matrix.map(|matrix| {
        if has_roles(kind) {
            translation_part(matrix).0.to_array()
        } else {
            matrix.to_array()
        }
    });
    Some(FixtureJson {
        id: record.id.clone(),
        kind: kind_text.to_owned(),
        name: record.name.clone(),
        locked: record.locked,
        params: record.params.clone(),
        matrix,
        slots: record.slots.iter().map(slot_json).collect(),
        jobs,
    })
}

/// The default name of a role job, as a pin or a screw is created with it (criterion 27).
const fn role_name(role: JobRole) -> &'static str {
    match role {
        JobRole::CentreMark => "Centre mark",
        JobRole::Diameters => "Diameters",
    }
}

fn role_job(kind: FixtureKind, role: JobRole, id: &str, name: &str, show: bool) -> JobJson {
    JobJson {
        id: id.to_owned(),
        kind: ROLE_JOB_KIND.to_owned(),
        name: name.to_owned(),
        show,
        targets: FixturePart::of_role(kind, role)
            .into_iter()
            .map(FixturePart::token)
            .collect(),
        role: Some(role.stored().to_owned()),
    }
}

fn job_json(kind: FixtureKind, slots: usize, job: &JobRecord) -> Option<JobJson> {
    OperationKind::parse(&job.kind)?;
    if !valid_name(&job.name) {
        return None;
    }
    let (parts, _) = job_parts(kind, slots, &job.targets);
    Some(JobJson {
        id: job.id.clone(),
        kind: job.kind.clone(),
        name: job.name.clone(),
        show: job.show,
        targets: parts.into_iter().map(FixturePart::token).collect(),
        role: None,
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

/// The fixture an entry holds; `None` when any part of it is damage: an unknown kind, a name over
/// 40 characters, a value that does not read, a matrix that is not finite or not rigid (for a pin
/// or a screw: not a translation), a slot that does not read, a job that does not read, or a role
/// rule broken (ADR 0050 §2, ADR 0052 §7, ADR 0060 §1).
pub(crate) fn read_fixture(fixture: FixtureJson) -> Option<FixtureRecord> {
    let stored = format!("{KIND_PREFIX}{}", fixture.kind);
    let kind = FixtureKind::from_stored(&stored);
    let matrix = match fixture.matrix {
        None => None,
        Some(entries) => {
            let matrix = Affine::from_array(entries);
            let placed = matrix.is_finite()
                && matrix.is_within_limit()
                && matrix.is_rigid_within(RIGID_TOLERANCE)
                && (!has_roles(kind) || translation_part(matrix).1 == 0);
            placed.then_some(matrix)?;
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
        .into_iter()
        .map(|job| read_job(kind, slots.len(), job))
        .collect::<Option<Vec<_>>>()?;
    roles_hold(kind, &jobs).then_some(())?;
    Some(FixtureRecord {
        id: fixture.id,
        kind: stored,
        name: fixture.name,
        locked: fixture.locked,
        params: fixture.params,
        matrix,
        slots,
        jobs,
    })
}

/// The role rule of ADR 0060 §1: a pin or a screw holds exactly one job of each role, of kind
/// score, with its fixed ticks, and a pin holds no other job; any other kind holds no role.
fn roles_hold(kind: FixtureKind, jobs: &[JobRecord]) -> bool {
    if !has_roles(kind) {
        return jobs.iter().all(|job| job.role.is_none());
    }
    let fixed = |role: JobRole| {
        let mut targets: Vec<JobTarget> = FixturePart::of_role(kind, role)
            .into_iter()
            .map(FixturePart::target)
            .collect();
        targets.sort_unstable();
        targets
    };
    let roles_ok = ROLES.into_iter().all(|role| {
        let mut of_role = jobs.iter().filter(|job| job.role == Some(role));
        matches!(
            (of_role.next(), of_role.next()),
            (Some(job), None) if job.kind == ROLE_JOB_KIND && job.targets == fixed(role)
        )
    });
    roles_ok && (kind == FixtureKind::Screw || jobs.iter().all(|job| job.role.is_some()))
}

/// A job of an entry of `kind` with `slots` slots; `None` for an unknown kind, an invalid name,
/// an unknown role, or a token that is no part of the entry or is given twice.
fn read_job(kind: FixtureKind, slots: usize, job: JobJson) -> Option<JobRecord> {
    OperationKind::parse(&job.kind)?;
    if !valid_name(&job.name) {
        return None;
    }
    let role = match &job.role {
        None => None,
        Some(text) => Some(JobRole::from_stored(text)?),
    };
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
        id: job.id,
        kind: job.kind,
        name: job.name,
        show: job.show,
        targets,
        role,
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

/// Whether `name` is a valid fixture, slot, job or set name of a record: 1 to 40 characters.
pub(crate) fn valid_name(name: &str) -> bool {
    is_valid_name(name) && name.chars().count() <= MAX_ENTRY_NAME_CHARS
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn params(value: Value) -> FixtureParams {
        serde_json::from_value(value).unwrap()
    }

    fn job(role: Option<JobRole>, kind: &str, parts: &[FixturePart]) -> JobRecord {
        let mut targets: Vec<JobTarget> = parts.iter().map(|p| p.target()).collect();
        targets.sort_unstable();
        JobRecord {
            id: String::new(),
            kind: kind.to_owned(),
            name: role.map_or("Cut 1", role_name).to_owned(),
            show: true,
            targets,
            role,
        }
    }

    fn pin(jobs: Vec<JobRecord>) -> FixtureRecord {
        FixtureRecord {
            kind: "curvyo.laser/pin".to_owned(),
            id: String::new(),
            name: "Pin 1".to_owned(),
            locked: false,
            params: params(json!({"diameter_mm": 6.0, "columns": 2, "rows": 2})),
            matrix: Some(Affine::new(1.0, 0.0, 0.0, 1.0, 40.0, 25.0)),
            slots: Vec::new(),
            jobs,
        }
    }

    fn round_trip(record: &FixtureRecord) -> Option<FixtureRecord> {
        let text = serde_json::to_string(&write_fixture(record)?).unwrap();
        read_fixture(serde_json::from_str(&text).unwrap())
    }

    #[test]
    fn a_pin_without_role_jobs_is_written_with_both_and_reads_back() {
        let written = round_trip(&pin(Vec::new())).unwrap();
        let roles: Vec<Option<JobRole>> = written.jobs.iter().map(|j| j.role).collect();
        assert_eq!(roles, [Some(JobRole::CentreMark), Some(JobRole::Diameters)]);
        assert_eq!(written.jobs[0].name, "Centre mark");
        assert_eq!(written.jobs[1].kind, "score");
        assert_eq!(round_trip(&written).unwrap(), written, "stable");
    }

    #[test]
    fn a_pin_keeps_no_other_job_and_a_second_role_job_is_dropped_on_write() {
        let marks = job(Some(JobRole::CentreMark), "score", &[FixturePart::PinMarks]);
        let record = pin(vec![
            job(None, "cut", &[FixturePart::PinCircle]),
            marks.clone(),
            marks,
        ]);
        let written = round_trip(&record).unwrap();
        assert_eq!(written.jobs.len(), 2);
        assert_eq!(written.jobs[0].role, Some(JobRole::CentreMark));
    }

    #[test]
    fn a_screw_keeps_its_0103_jobs_beside_its_role_jobs() {
        let screw = FixtureRecord {
            kind: "curvyo.laser/screw".to_owned(),
            params: params(json!({"thread_diameter_mm": 6.0, "head_diameter_mm": 10.0})),
            jobs: vec![job(
                None,
                "score",
                &[FixturePart::ScrewHead, FixturePart::ScrewThread],
            )],
            ..pin(Vec::new())
        };
        let written = round_trip(&screw).unwrap();
        let roles: Vec<Option<JobRole>> = written.jobs.iter().map(|j| j.role).collect();
        assert_eq!(
            roles,
            [None, Some(JobRole::CentreMark), Some(JobRole::Diameters)]
        );
        assert_eq!(written.jobs[2].targets.len(), 2, "head and thread");
    }

    #[test]
    fn a_turned_pin_is_written_by_its_translation_and_a_role_elsewhere_is_dropped() {
        let mut turned = pin(Vec::new());
        turned.matrix = Some(Affine::new(0.0, 1.0, -1.0, 0.0, 40.0, 25.0));
        let written = round_trip(&turned).unwrap();
        assert_eq!(
            written.matrix,
            Some(Affine::new(1.0, 0.0, 0.0, 1.0, 40.0, 25.0))
        );
        let clamp = FixtureRecord {
            kind: "curvyo.laser/clamp".to_owned(),
            params: params(json!({"width_mm": 40.0, "height_mm": 40.0})),
            jobs: vec![job(
                Some(JobRole::Diameters),
                "score",
                &[FixturePart::Clamp],
            )],
            ..pin(Vec::new())
        };
        let written = round_trip(&clamp).unwrap();
        assert_eq!(written.jobs[0].role, None);
    }

    #[test]
    fn the_role_rule_holds_for_pins_and_screws_only() {
        let marks = job(Some(JobRole::CentreMark), "score", &[FixturePart::PinMarks]);
        let circle = job(Some(JobRole::Diameters), "score", &[FixturePart::PinCircle]);
        assert!(roles_hold(
            FixtureKind::Pin,
            &[marks.clone(), circle.clone()]
        ));
        assert!(
            !roles_hold(FixtureKind::Pin, std::slice::from_ref(&marks)),
            "missing"
        );
        assert!(
            !roles_hold(
                FixtureKind::Pin,
                &[marks.clone(), circle.clone(), marks.clone()]
            ),
            "a second job of one role"
        );
        let mut cut = circle.clone();
        cut.kind = "cut".to_owned();
        assert!(
            !roles_hold(FixtureKind::Pin, &[marks.clone(), cut]),
            "not score"
        );
        let mut extra = circle.clone();
        extra.targets.push(FixturePart::PinMarks.target());
        extra.targets.sort_unstable();
        assert!(
            !roles_hold(FixtureKind::Pin, &[marks.clone(), extra]),
            "other ticks"
        );
        let other = job(None, "cut", &[FixturePart::PinCircle]);
        assert!(!roles_hold(
            FixtureKind::Pin,
            &[marks.clone(), circle, other]
        ));
        assert!(!roles_hold(FixtureKind::Holder, &[marks]));
        assert!(roles_hold(FixtureKind::Holder, &[]));
    }
}
