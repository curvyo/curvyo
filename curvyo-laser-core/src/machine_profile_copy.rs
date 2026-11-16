//! Reads and writes the machine profile copy of a manufacturing (ADR 0039 §1 to §3, §5, §6;
//! ADR 0056 §7): the `mfg_profile` map, every field read on its own with its default on damage,
//! how the page's copy differs from its record (ADR 0039 §5) and whether a page's set mirror
//! differs from its profile's sets (ADR 0059 §10).

use curvyo_document_core::{
    DocumentSize, FixtureRecord, FixtureSetRecord, PRESET_MATCH_TOLERANCE, SlotRecord,
};
use curvyo_extension_core::StoredParams;
use serde_json::Value;

use crate::machine_profile::{
    FeedRate, GENERIC_LASER_ID, MachineProfile, OriginCorner, PowerValue, ProfileValues,
    is_valid_name,
};

const ID: &str = "id";
const NAME: &str = "name";
const ORIGIN: &str = "origin";
const MAX_FEED: &str = "max_feed_mm_per_min";
const MAX_POWER: &str = "max_power";
const LASER_MODE: &str = "laser_mode";
const MOTORISED_Z: &str = "motorised_z";

/// The notice of a copy whose stored name was damaged (criterion 12).
pub const DAMAGED_NAME_NOTICE: &str = "Name was damaged, showing Generic laser";

/// The copy a manufacturing uses: the library id it came from and the copied values. The work
/// area is the output page's size, never part of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileCopy {
    /// The library record it came from; `None` for a missing or damaged id ("Not in your
    /// profiles").
    pub id: Option<String>,
    /// The copied values.
    pub values: ProfileValues,
}

impl ProfileCopy {
    /// The copy of `profile`.
    #[must_use]
    pub fn of(profile: &MachineProfile) -> Self {
        Self {
            id: Some(profile.id.clone()),
            values: profile.values.clone(),
        }
    }
}

/// A copy as read and whether its stored name was damaged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyRead {
    /// The copy.
    pub copy: ProfileCopy,
    /// The stored name was missing or not a valid name, so it reads as "Generic laser".
    pub name_damaged: bool,
}

/// The copy `stored` holds (`None`: a format-35 manufacturing, which reads as Generic laser with
/// `legacy_name` when that is a valid name). Nothing is rewritten on read (ADR 0033 §3).
#[must_use]
pub fn read_copy(stored: Option<&StoredParams>, legacy_name: Option<&str>) -> CopyRead {
    let Some(stored) = stored else {
        let mut values = ProfileValues::default();
        if let Some(name) = legacy_name.filter(|name| is_valid_name(name)) {
            name.clone_into(&mut values.name);
        }
        return CopyRead {
            copy: ProfileCopy {
                id: Some(GENERIC_LASER_ID.to_owned()),
                values,
            },
            name_damaged: false,
        };
    };
    let defaults = ProfileValues::default();
    let name = stored
        .get(NAME)
        .and_then(Value::as_str)
        .filter(|name| is_valid_name(name));
    let values = ProfileValues {
        name: name.map_or(defaults.name, str::to_owned),
        origin: stored
            .get(ORIGIN)
            .and_then(Value::as_str)
            .and_then(OriginCorner::from_name)
            .unwrap_or(defaults.origin),
        max_feed: whole(stored.get(MAX_FEED))
            .and_then(FeedRate::new)
            .unwrap_or(defaults.max_feed),
        max_power: whole(stored.get(MAX_POWER))
            .and_then(PowerValue::new)
            .unwrap_or(defaults.max_power),
        laser_mode: stored
            .get(LASER_MODE)
            .and_then(Value::as_bool)
            .unwrap_or(defaults.laser_mode),
        motorised_z: stored
            .get(MOTORISED_Z)
            .and_then(Value::as_bool)
            .unwrap_or(defaults.motorised_z),
    };
    CopyRead {
        copy: ProfileCopy {
            id: stored.get(ID).and_then(Value::as_str).map(str::to_owned),
            values,
        },
        name_damaged: name.is_none(),
    }
}

/// A stored whole number: an integer, or an integral finite double (ADR 0039 §1).
fn whole(value: Option<&Value>) -> Option<u64> {
    let value = value?;
    value.as_u64().or_else(|| {
        let n = value
            .as_f64()
            .filter(|n| n.is_finite() && n.fract() == 0.0 && *n >= 0.0)?;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // integral, >= 0
        Some(n as u64)
    })
}

/// The stored map of `copy`: the id (when known) and the copied values, never a connection field
/// (ADR 0039 §2: the copy type has none).
#[must_use]
pub fn write_copy(copy: &ProfileCopy) -> StoredParams {
    let mut map = StoredParams::new();
    if let Some(id) = &copy.id {
        map.insert(ID.to_owned(), Value::from(id.as_str()));
    }
    map.insert(NAME.to_owned(), Value::from(copy.values.name.as_str()));
    map.insert(ORIGIN.to_owned(), Value::from(copy.values.origin.name()));
    map.insert(MAX_FEED.to_owned(), Value::from(copy.values.max_feed.get()));
    map.insert(
        MAX_POWER.to_owned(),
        Value::from(copy.values.max_power.get()),
    );
    map.insert(LASER_MODE.to_owned(), Value::from(copy.values.laser_mode));
    map.insert(MOTORISED_Z.to_owned(), Value::from(copy.values.motorised_z));
    map
}

/// "Differs from profile" (ADR 0039 §5): any copied value differs from the record's, or the
/// page size differs from its work area by more than 0.01 mm. Derived, never stored.
#[must_use]
pub fn differs(copy: &ProfileCopy, page: DocumentSize, record: &MachineProfile) -> bool {
    let tolerance = PRESET_MATCH_TOLERANCE.as_mm();
    copy.values != record.values
        || (page.width.as_mm() - record.work_area.width.as_mm()).abs() > tolerance
        || (page.height.as_mm() - record.work_area.height.as_mm()).abs() > tolerance
}

/// Whether the page's set mirror `page` differs in content from the record's sets `record`: the
/// alignment's test (ADR 0059 §10; "Differs from profile" no longer compares sets, ADR 0060 §7
/// amended 2026-11-16). Compared in order: set ids, names and locks, and their fixtures (id,
/// kind, name, Lock, parameters, matrix, slots, jobs with their id, Show, role and ticks),
/// lengths within [`PRESET_MATCH_TOLERANCE`]. The page's choices (On this page, On the bed, the
/// eye, Output) and its job fixtures are no part of either side.
#[must_use]
pub fn sets_differ(page: &[FixtureSetRecord], record: &[FixtureSetRecord]) -> bool {
    page.len() != record.len()
        || page.iter().zip(record).any(|(page, record)| {
            page.id != record.id
                || page.name != record.name
                || page.locked != record.locked
                || page.fixtures.len() != record.fixtures.len()
                || page
                    .fixtures
                    .iter()
                    .zip(&record.fixtures)
                    .any(|(a, b)| !fixtures_match(a, b))
        })
}

/// Whether a page's set mirror `page`, last aligned to or written with the record time
/// `page_modified`, is to be aligned with `record` (ADR 0059 §10 a): the record is newer and its
/// sets differ in content. A record that is not newer never overwrites the mirror.
#[must_use]
pub fn align_due(page: &[FixtureSetRecord], page_modified: i64, record: &MachineProfile) -> bool {
    record.modified > page_modified && sets_differ(page, &record.fixture_sets)
}

fn fixtures_match(a: &FixtureRecord, b: &FixtureRecord) -> bool {
    let tolerance = PRESET_MATCH_TOLERANCE.as_mm();
    let matrix = match (a.matrix, b.matrix) {
        (None, None) => true,
        (Some(a), Some(b)) => a.approx_eq(b, tolerance),
        _ => false,
    };
    a.id == b.id
        && a.kind == b.kind
        && a.name == b.name
        && a.locked == b.locked
        && matrix
        && maps_match(&a.params, &b.params)
        && a.slots.len() == b.slots.len()
        && a.slots.iter().zip(&b.slots).all(slots_match)
        && a.jobs == b.jobs
}

/// Two slots are the same as the record stores them (`machine_profile_fixtures`): the name, the
/// shape and the five lengths, an absent length as 0, within [`PRESET_MATCH_TOLERANCE`].
fn slots_match((a, b): (&SlotRecord, &SlotRecord)) -> bool {
    let number =
        |slot: &SlotRecord, key: &str| slot.params.get(key).and_then(Value::as_f64).unwrap_or(0.0);
    a.name == b.name
        && a.params.get("shape") == b.params.get("shape")
        && SLOT_LENGTHS
            .iter()
            .all(|key| (number(a, key) - number(b, key)).abs() <= PRESET_MATCH_TOLERANCE.as_mm())
}

/// The lengths a record keeps of a slot (`machine_profile_fixtures`' slot entry).
const SLOT_LENGTHS: [&str; 5] = ["cx_mm", "cy_mm", "w_mm", "h_mm", "d_mm"];

fn maps_match(a: &StoredParams, b: &StoredParams) -> bool {
    a.len() == b.len()
        && a.iter()
            .all(|(key, value)| b.get(key).is_some_and(|other| values_match(value, other)))
}

/// Two stored values are the same, numbers within [`PRESET_MATCH_TOLERANCE`].
fn values_match(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(_), Value::Number(_)) => match (a.as_f64(), b.as_f64()) {
            (Some(x), Some(y)) => (x - y).abs() <= PRESET_MATCH_TOLERANCE.as_mm(),
            _ => a == b,
        },
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| values_match(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, value)| b.get(key).is_some_and(|other| values_match(value, other)))
        }
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn stored(value: Value) -> StoredParams {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn a_copy_round_trips_without_any_connection_field() {
        let mut profile = MachineProfile::generic_laser();
        profile.id = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f".into();
        profile.values.name = "K40".into();
        profile.connection.controller_port = "/dev/ttyUSB0".into();
        let copy = ProfileCopy::of(&profile);
        let map = write_copy(&copy);
        let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "id",
                "laser_mode",
                "max_feed_mm_per_min",
                "max_power",
                "motorised_z",
                "name",
                "origin"
            ]
        );
        assert!(!serde_json::to_string(&map).unwrap().contains("ttyUSB0"));
        assert_eq!(read_copy(Some(&map), None).copy, copy);
    }

    #[test]
    fn a_damaged_field_reads_as_its_default_and_a_damaged_name_is_flagged() {
        let map = stored(json!({
            "id": "x",
            "name": "n".repeat(61),
            "origin": "middle",
            "max_feed_mm_per_min": "fast",
            "max_power": 0,
            "laser_mode": 1,
            "motorised_z": "yes",
        }));
        let read = read_copy(Some(&map), None);
        assert!(read.name_damaged);
        assert_eq!(read.copy.values, ProfileValues::default());
        assert_eq!(read.copy.id.as_deref(), Some("x"));
        let integral = stored(json!({"name": "K40", "max_feed_mm_per_min": 1200.0, "id": 3}));
        let read = read_copy(Some(&integral), None);
        assert!(!read.name_damaged);
        assert_eq!(read.copy.values.max_feed.get(), 1_200);
        assert_eq!(read.copy.id, None, "a non-string id is no library record");
    }

    #[test]
    fn a_format_35_manufacturing_reads_as_generic_laser_with_its_name() {
        let read = read_copy(None, Some("Old laser"));
        assert_eq!(read.copy.id.as_deref(), Some(GENERIC_LASER_ID));
        assert_eq!(read.copy.values.name, "Old laser");
        assert_eq!(read_copy(None, Some("")).copy.values.name, "Generic laser");
    }

    #[test]
    fn differs_compares_every_copied_value_and_the_page_size() {
        let record = MachineProfile::generic_laser();
        let copy = ProfileCopy::of(&record);
        let area = record.work_area;
        assert!(!differs(&copy, area, &record));
        assert!(!differs(
            &copy,
            DocumentSize::from_mm(400.005, 300.0),
            &record
        ));
        assert!(differs(&copy, DocumentSize::from_mm(400.1, 300.0), &record));
        let mut renamed = copy.clone();
        renamed.values.name = "Other".into();
        assert!(differs(&renamed, area, &record));
        let mut motorised = copy.clone();
        motorised.values.motorised_z = true;
        assert!(differs(&motorised, area, &record));
    }

    fn set(name: &str, width: f64) -> FixtureSetRecord {
        FixtureSetRecord {
            id: format!("{name}-id"),
            name: name.to_owned(),
            locked: false,
            fixtures: vec![FixtureRecord {
                id: format!("{name}-clamp"),
                kind: "curvyo.laser/clamp".to_owned(),
                name: "Clamp 1".to_owned(),
                locked: false,
                params: stored(json!({"width_mm": width, "height_mm": 40.0})),
                matrix: Some(curvyo_document_core::Affine::new(
                    1.0, 0.0, 0.0, 1.0, 10.0, 10.0,
                )),
                slots: Vec::new(),
                jobs: Vec::new(),
            }],
        }
    }

    #[test]
    fn sets_differ_in_order_by_id_name_lock_and_fixtures_with_lengths_in_tolerance() {
        let record = [set("Bed", 40.0), set("Jig", 40.0)];
        assert!(!sets_differ(&record, &record));
        assert!(!sets_differ(
            &[set("Bed", 40.005), set("Jig", 40.0)],
            &record
        ));
        assert!(sets_differ(&[set("Bed", 40.1), set("Jig", 40.0)], &record));
        assert!(
            sets_differ(&[set("Jig", 40.0), set("Bed", 40.0)], &record),
            "order"
        );
        assert!(sets_differ(&[set("Bed", 40.0)], &record));
        let mut locked = set("Bed", 40.0);
        locked.fixtures[0].locked = true;
        assert!(sets_differ(&[locked, set("Jig", 40.0)], &record), "a Lock");
        let mut set_lock = set("Bed", 40.0);
        set_lock.locked = true;
        assert!(
            sets_differ(&[set_lock, set("Jig", 40.0)], &record),
            "a set Lock"
        );
        let mut other = set("Bed", 40.0);
        other.id = "x".to_owned();
        assert!(sets_differ(&[other, set("Jig", 40.0)], &record), "an id");
        let mut moved = set("Bed", 40.0);
        moved.fixtures[0].matrix = Some(curvyo_document_core::Affine::new(
            1.0, 0.0, 0.0, 1.0, 10.004, 10.0,
        ));
        assert!(!sets_differ(&[moved, set("Jig", 40.0)], &record));
        assert!(!sets_differ(&[], &[]));
    }

    #[test]
    fn a_slot_reads_as_the_record_stores_it_absent_numbers_as_zero() {
        // A rectangle slot made on the page has no `d_mm`; its record, read back, holds 0.
        let with_slot = |params: Value| {
            let mut bed = set("Bed", 40.0);
            bed.fixtures[0].slots.push(SlotRecord {
                name: "Slot 1".to_owned(),
                params: stored(params),
            });
            [bed]
        };
        let page = with_slot(
            json!({"shape": "rect", "cx_mm": 5.0, "cy_mm": 5.0, "w_mm": 4.0, "h_mm": 4.0}),
        );
        let record = with_slot(
            json!({"shape": "rect", "cx_mm": 5.0, "cy_mm": 5.0, "w_mm": 4.0, "h_mm": 4.0, "d_mm": 0.0}),
        );
        assert!(!sets_differ(&page, &record));
        let wider = with_slot(
            json!({"shape": "rect", "cx_mm": 5.0, "cy_mm": 5.0, "w_mm": 4.5, "h_mm": 4.0, "d_mm": 0.0}),
        );
        assert!(sets_differ(&page, &wider));
        let round = with_slot(
            json!({"shape": "circle", "cx_mm": 5.0, "cy_mm": 5.0, "w_mm": 4.0, "h_mm": 4.0, "d_mm": 0.0}),
        );
        assert!(sets_differ(&page, &round), "the shape");
    }

    #[test]
    fn the_motorised_z_axis_is_copied_and_absent_reads_off() {
        let mut profile = MachineProfile::generic_laser();
        assert!(!profile.values.motorised_z, "Generic laser has none");
        profile.values.motorised_z = true;
        let map = write_copy(&ProfileCopy::of(&profile));
        assert_eq!(map.get("motorised_z"), Some(&json!(true)));
        assert!(read_copy(Some(&map), None).copy.values.motorised_z);
        let older = stored(json!({"name": "K40", "laser_mode": false}));
        assert!(!read_copy(Some(&older), None).copy.values.motorised_z);
    }
}
