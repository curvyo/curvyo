//! Reads the page's output registers of ADR 0065 §1 (the fixture material pick, Start and the
//! user origin of `specs/0118` O1).

use curvyo_document_core::ManufacturingSnapshot;
use serde_json::{Map, Value};

use crate::job_input::PointUm;
use crate::manifest::PROCESS_ID;
use crate::record_envelope::is_record_id;

/// How the job starts (criterion 38; ADR 0065 §2; `specs/0118` O4): one closed list of four
/// values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Start {
    /// At the machine origin, in absolute moves (the default, also when the register is absent).
    #[default]
    MachineOrigin,
    /// Where the head stands, in relative moves.
    HeadPosition,
    /// The format's home command first, then exactly as Machine origin.
    HomeThenOrigin,
    /// In absolute moves with the job's anchor on the page's user origin (`specs/0118` 31).
    UserOrigin,
}

impl Start {
    /// The four values in the pick's order.
    pub const ALL: [Self; 4] = [
        Self::MachineOrigin,
        Self::HeadPosition,
        Self::HomeThenOrigin,
        Self::UserOrigin,
    ];

    /// The stored spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MachineOrigin => "machine_origin",
            Self::HeadPosition => "head_position",
            Self::HomeThenOrigin => "home_then_origin",
            Self::UserOrigin => "user_origin",
        }
    }

    /// The value a stored spelling names.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|start| start.as_str() == text)
    }

    /// Whether the moves are relative to where the head stands (Head position).
    #[must_use]
    pub const fn relative(self) -> bool {
        matches!(self, Self::HeadPosition)
    }

    /// Whether the file homes first (Home, then machine origin).
    #[must_use]
    pub const fn homes(self) -> bool {
        matches!(self, Self::HomeThenOrigin)
    }
}

/// The three page registers as read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OutputSettings {
    /// The fixture jobs' material id, `None` without a (readable) pick.
    pub fixture_material: Option<String>,
    /// The start the job runs with: Machine origin when the register is absent or damaged.
    pub start: Start,
    /// Whether the Start register holds a value this build does not read, or User origin with
    /// no readable point (block `StartDamaged`).
    pub start_damaged: bool,
    /// The user origin in µm of the job file's coordinates, `None` without a (readable) point.
    pub user_origin: Option<PointUm>,
    /// How many values are present but unreadable (each reads as its default); User origin
    /// without a point counts one, a damaged point with it counts one in all.
    pub damaged: usize,
}

/// The register key of `field` (`x.curvyo.laser.<field>`).
#[must_use]
pub fn register_key(field: &str) -> String {
    format!("x.{PROCESS_ID}.{field}")
}

/// The register holding the fixture jobs' material id.
pub const FIXTURE_MATERIAL_FIELD: &str = "fixture_material";
/// The register holding Start.
pub const START_FIELD: &str = "start";
/// The register holding the user origin (`specs/0118` O1).
pub const USER_ORIGIN_FIELD: &str = "user_origin_um";
/// The largest distance of a user origin from the machine origin on either axis, in µm.
pub const USER_ORIGIN_LIMIT_UM: i64 = 10_000_000;

/// The user origin `value` holds: two integers within the limit.
fn user_origin_of(value: &Value) -> Option<PointUm> {
    let [x, y] = value.as_array()?.as_slice() else {
        return None;
    };
    let axis = |v: &Value| {
        v.as_i64()
            .filter(|n| n.unsigned_abs() <= USER_ORIGIN_LIMIT_UM.unsigned_abs())
    };
    Some(PointUm::new(axis(x)?, axis(y)?))
}

/// The output registers of `manufacturing`; a value of the wrong type or spelling reads as its
/// default and counts one damaged value (ADR 0065 §1). Nothing is written.
#[must_use]
pub fn read_output_settings(manufacturing: &ManufacturingSnapshot) -> OutputSettings {
    read_values(&manufacturing.process_values)
}

fn read_values(values: &Map<String, Value>) -> OutputSettings {
    let get = |field: &str| values.get(&register_key(field));
    let fixture = get(FIXTURE_MATERIAL_FIELD).map(|value| {
        value
            .as_str()
            .filter(|id| is_record_id(id))
            .map(str::to_owned)
    });
    let start = get(START_FIELD).map(|value| value.as_str().and_then(Start::parse));
    let origin = get(USER_ORIGIN_FIELD).map(user_origin_of);
    let fixture_damaged = matches!(fixture, Some(None));
    let origin_damaged = matches!(origin, Some(None));
    let user_origin = origin.flatten();
    let spelling_damaged = matches!(start, Some(None));
    let pointless = start.flatten() == Some(Start::UserOrigin) && user_origin.is_none();
    let start_damaged = spelling_damaged || pointless;
    OutputSettings {
        fixture_material: fixture.flatten(),
        start: start
            .flatten()
            .filter(|_| !start_damaged)
            .unwrap_or_default(),
        start_damaged,
        user_origin,
        damaged: usize::from(fixture_damaged)
            + usize::from(origin_damaged)
            + usize::from(spelling_damaged || (pointless && !origin_damaged)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn read(values: &[(&str, Value)]) -> OutputSettings {
        let mut map = Map::new();
        for (field, value) in values {
            map.insert(register_key(field), value.clone());
        }
        read_values(&map)
    }

    const ID: &str = "0b3c7f1e-8f0e-4c4a-9a51-2f6c1c1e9d10";

    #[test]
    fn absent_registers_read_their_defaults() {
        let settings = read(&[]);
        assert_eq!(settings, OutputSettings::default());
        assert_eq!(settings.start, Start::MachineOrigin);
        assert!(!settings.start_damaged);
    }

    #[test]
    fn each_start_spelling_reads_its_value() {
        for start in Start::ALL {
            let settings = read(&[
                (START_FIELD, json!(start.as_str())),
                (USER_ORIGIN_FIELD, json!([1_000, 2_000])),
            ]);
            assert_eq!(settings.start, start);
            assert_eq!(settings.damaged, 0);
        }
        let settings = read(&[(FIXTURE_MATERIAL_FIELD, json!(ID))]);
        assert_eq!(settings.fixture_material.as_deref(), Some(ID));
    }

    #[test]
    fn only_head_position_is_relative_and_only_home_then_origin_homes() {
        assert!(Start::HeadPosition.relative());
        assert!(!Start::MachineOrigin.relative() && !Start::HomeThenOrigin.relative());
        assert!(!Start::UserOrigin.relative() && !Start::UserOrigin.homes());
        assert!(Start::HomeThenOrigin.homes());
        assert!(!Start::MachineOrigin.homes() && !Start::HeadPosition.homes());
    }

    #[test]
    fn the_user_origin_is_two_integers_within_ten_metres() {
        let settings = read(&[
            (START_FIELD, json!("user_origin")),
            (USER_ORIGIN_FIELD, json!([123_456, -78_900])),
        ]);
        assert_eq!(settings.start, Start::UserOrigin);
        assert_eq!(settings.user_origin, Some(PointUm::new(123_456, -78_900)));
        assert_eq!(settings.damaged, 0);
        for value in [
            json!([1.5, 2]),
            json!([1, 2, 3]),
            json!("12,3"),
            json!([10_000_001, 0]),
        ] {
            let settings = read(&[(USER_ORIGIN_FIELD, value.clone())]);
            assert_eq!(settings.user_origin, None, "{value}");
            assert_eq!(settings.damaged, 1, "{value}");
            assert!(!settings.start_damaged, "only with User origin picked");
        }
        let settings = read(&[(START_FIELD, json!("user_origin"))]);
        assert!(settings.start_damaged);
        assert_eq!(settings.start, Start::MachineOrigin);
        assert_eq!(settings.damaged, 1);
    }

    #[test]
    fn a_damaged_start_reads_machine_origin_and_counts_damaged() {
        for value in [json!("middle"), json!(true), json!(3)] {
            let settings = read(&[(START_FIELD, value)]);
            assert_eq!(settings.start, Start::MachineOrigin);
            assert!(settings.start_damaged);
            assert_eq!(settings.damaged, 1);
        }
        let settings = read(&[(FIXTURE_MATERIAL_FIELD, json!(7))]);
        assert_eq!(settings.fixture_material, None, "a damaged pick is no pick");
        assert!(!settings.start_damaged);
        assert_eq!(settings.damaged, 1);
    }

    #[test]
    fn the_first_builds_registers_are_not_read() {
        let settings = read(&[
            ("start_from", json!("head_position")),
            ("home_first", json!(true)),
        ]);
        assert_eq!(settings, OutputSettings::default());
    }
}
