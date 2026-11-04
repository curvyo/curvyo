//! Reads and writes the machine profile copy of a manufacturing (ADR 0039 §1 to §3, §5, §6): the
//! `mfg_profile` map, every field read on its own with its default on damage.

use curvyo_document_core::{DocumentSize, PRESET_MATCH_TOLERANCE};
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
    }
}
