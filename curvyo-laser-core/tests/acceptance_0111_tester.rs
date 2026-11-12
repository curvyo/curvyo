//! Tester acceptance cases for `specs/0111-laser-operation-tools` in the laser model, written from
//! the specification and ADR 0056 before the implementation was read: the operation-only
//! settings with their ranges, defaults, kinds and damage (criteria 20, 25), Interval in
//! `op_settings.main` (19, 25), the last operation of a kind (Words, 3, 6) and the motorised Z
//! axis in the profile record format 4 and in the page copy (22).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::needless_pass_by_value,
    clippy::assert_is_empty,
    missing_docs
)]

use std::path::PathBuf;

use curvyo_document_core::{Document, DocumentSize, NewOperation, NewOutputPage, NodeId};
use curvyo_laser_core::operations::{OperationKind, last_of_kind};
use curvyo_laser_core::settings::{
    OperationOptions, OptionsDamage, SettingsRead, read_options, read_settings,
};
use curvyo_laser_core::{
    MachineProfile, ProfileCopy, RecordError, read_copy, read_record, write_copy, write_record,
};
use serde_json::{Value, json};

const HATCHING: [OperationKind; 2] = [OperationKind::Engrave, OperationKind::EngraveOutline];
const PLAIN: [OperationKind; 3] = [
    OperationKind::Cut,
    OperationKind::Score,
    OperationKind::CutOut,
];

// ---- criterion 20: op_options ---------------------------------------------------------------

#[test]
fn c20_absent_options_read_the_defaults() {
    let o = read_options(None, OperationKind::Engrave).unwrap();
    assert_eq!(o, OperationOptions::default());
    assert!(!o.crosshatch);
    assert!(o.bidirectional);
    assert_eq!(o.scan_angle.degrees(), 0.0);
    assert_eq!(o.z_offset.mm(), 0.0);
    // An empty map reads the defaults too (every key optional).
    assert_eq!(
        read_options(Some(&json!({})), OperationKind::Cut).unwrap(),
        OperationOptions::default()
    );
}

#[test]
fn c20_the_full_map_reads_on_both_hatching_kinds() {
    let v = json!({"crosshatch": true, "bidirectional": false, "scan_angle_deg": -90.0,
                   "overscan_pct": 50, "z_offset_mm": -20.0});
    for kind in HATCHING {
        let o = read_options(Some(&v), kind).unwrap();
        assert!(o.crosshatch);
        assert!(!o.bidirectional);
        assert_eq!(o.scan_angle.degrees(), -90.0);
        assert_eq!(o.z_offset.mm(), -20.0);
        assert_eq!(o.to_value(kind), v, "written back as read");
    }
}

#[test]
fn c20_z_offset_alone_reads_on_every_work_kind() {
    for kind in HATCHING.into_iter().chain(PLAIN) {
        let o = read_options(Some(&json!({"z_offset_mm": 20.0})), kind).unwrap();
        assert_eq!(o.z_offset.mm(), 20.0);
    }
}

#[test]
fn c25_damage_rules_of_op_options() {
    let e = OperationKind::Engrave;
    let cases: Vec<(Value, OperationKind)> = vec![
        (json!([1, 2]), e),
        (json!("on"), e),
        (Value::Null, e),
        (json!({"bogus": 1}), e),
        (json!({"crosshatch": "yes"}), e),
        (json!({"scan_angle_deg": 90.1}), e),
        (json!({"scan_angle_deg": -90.1}), e),
        (json!({"scan_angle_deg": 12.25}), e),
        (json!({"overscan_pct": 51}), e),
        (json!({"overscan_pct": -1}), e),
        (json!({"overscan_pct": 2.5}), e),
        (json!({"z_offset_mm": 20.1}), e),
        (json!({"z_offset_mm": 1.25}), e),
        (json!({"crosshatch": true}), OperationKind::Cut),
        (json!({"bidirectional": true}), OperationKind::Score),
        (json!({"scan_angle_deg": 0.0}), OperationKind::CutOut),
        (json!({"overscan_pct": 2}), OperationKind::Cut),
    ];
    for (v, kind) in cases {
        assert!(
            read_options(Some(&v), kind).is_err(),
            "{v} on {kind:?} must read as damaged"
        );
    }
    assert_eq!(
        read_options(Some(&json!(3)), e).unwrap_err(),
        OptionsDamage::NotAMap
    );
    assert_eq!(
        read_options(Some(&json!({"zz": 1})), e).unwrap_err(),
        OptionsDamage::UnknownKey
    );
}

#[test]
fn c20_range_edges_read() {
    let e = OperationKind::Engrave;
    for v in [
        json!({"scan_angle_deg": 90.0}),
        json!({"scan_angle_deg": -90.0}),
        json!({"scan_angle_deg": 12.5}),
        json!({"overscan_pct": 0}),
        json!({"overscan_pct": 50}),
        json!({"z_offset_mm": -20.0}),
        json!({"z_offset_mm": 0.1}),
    ] {
        assert!(read_options(Some(&v), e).is_ok(), "{v}");
    }
}

// ---- criterion 19: Interval in op_settings.main --------------------------------------------

fn main_with(interval: Value) -> Value {
    json!({"main": {"power_pct": 40.0, "speed_mm_min": 3000, "passes": 1, "interval_mm": interval}})
}

#[test]
fn c19_interval_reads_on_engrave_and_defaults_when_absent() {
    let SettingsRead::Stored(s) =
        read_settings(Some(&main_with(json!(0.25))), OperationKind::Engrave)
    else {
        panic!("0.25 reads");
    };
    assert!(s.interval.is_some());
    let plain = json!({"main": {"power_pct": 40.0, "speed_mm_min": 3000, "passes": 1}});
    assert!(
        matches!(
            read_settings(Some(&plain), OperationKind::Engrave),
            SettingsRead::Stored(_)
        ),
        "absent interval is no damage (format 43)"
    );
    for edge in [0.01, 2.0] {
        assert!(
            matches!(
                read_settings(Some(&main_with(json!(edge))), OperationKind::Engrave),
                SettingsRead::Stored(_)
            ),
            "{edge}"
        );
    }
}

#[test]
fn c25_interval_damage_rules() {
    for bad in [
        json!(0.005),
        json!(2.01),
        json!(0.125),
        json!("0.1"),
        json!(0),
    ] {
        assert!(
            matches!(
                read_settings(Some(&main_with(bad.clone())), OperationKind::Engrave),
                SettingsRead::Damaged(_)
            ),
            "interval {bad}"
        );
    }
    for kind in PLAIN {
        assert!(
            matches!(
                read_settings(Some(&main_with(json!(0.1))), kind),
                SettingsRead::Damaged(_)
            ),
            "interval on {kind:?}"
        );
    }
}

// ---- Words, criteria 3 and 6: the last operation of a kind -----------------------------------

fn ops(kinds: &[(&str, &str)]) -> (Document, NodeId, Vec<NodeId>) {
    let d = Document::new(1);
    let made = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let m = made.kind.manufacturing().unwrap();
    let ids = kinds
        .iter()
        .enumerate()
        .map(|(i, (kind, name))| {
            d.add_operation(m, &NewOperation { kind, name }, i, &[])
                .unwrap()
        })
        .collect();
    (d, m, ids)
}

#[test]
fn c3_the_lowest_of_the_kind_in_list_order() {
    let (d, m, _) = ops(&[
        ("engrave", "Engrave 1"),
        ("cut", "Cut 1"),
        ("engrave", "Engrave 2"),
        ("score", "Score 1"),
    ]);
    let list = d.operations(m).unwrap();
    assert_eq!(last_of_kind(&list, OperationKind::Engrave), Some(2));
    assert_eq!(last_of_kind(&list, OperationKind::Cut), Some(1));
    assert_eq!(last_of_kind(&list, OperationKind::CutOut), None);
}

#[test]
fn c6_unknown_and_damaged_operations_are_skipped() {
    let (d, m, ids) = ops(&[
        ("engrave", "Engrave 1"),
        ("engrave", "Engrave 2"),
        ("engrave", "Engrave 3"),
        ("laser_weld", "Weld"),
    ]);
    d.write_operation_options(ids[2], json!({"overscan_pct": 99}).as_object().unwrap())
        .unwrap();
    d.write_operation_settings(
        curvyo_document_core::SettingsStep::EditSettings,
        &[curvyo_document_core::SettingsWrite {
            operation: ids[1],
            value: Some(json!({"main": {"bogus": 1}}).as_object().unwrap().clone()),
        }],
    )
    .unwrap();
    let list = d.operations(m).unwrap();
    assert_eq!(
        last_of_kind(&list, OperationKind::Engrave),
        Some(0),
        "Engrave 3 (damaged options) and Engrave 2 (damaged settings) are skipped"
    );
}

// ---- criterion 22: the motorised Z axis ------------------------------------------------------

fn profile_fixture(name: &str) -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/machine_profiles")
            .join(name),
    )
    .unwrap()
}

fn file_name_of(text: &str) -> String {
    let v: Value = serde_json::from_str(text).unwrap();
    format!("{}.json", v["id"].as_str().unwrap())
}

#[test]
fn c22_a_format_4_record_reads_the_capability_and_writes_back_byte_for_byte() {
    let text = profile_fixture("format4_motorised_z.json");
    let p = read_record(&file_name_of(&text), &text).unwrap();
    assert!(p.values.motorised_z);
    assert_eq!(write_record(&p), text);
}

#[test]
fn c22_an_older_record_reads_off_and_a_write_upgrades_to_4() {
    let text = profile_fixture("format4_motorised_z.json");
    let mut v: Value = serde_json::from_str(&text).unwrap();
    v.as_object_mut().unwrap().remove("motorised_z");
    v["format_version"] = json!(3);
    let old = serde_json::to_string_pretty(&v).unwrap();
    let p = read_record(&file_name_of(&old), &old).unwrap();
    assert!(!p.values.motorised_z, "a format 3 record reads off");
    let written: Value = serde_json::from_str(&write_record(&p)).unwrap();
    assert_eq!(written["format_version"], 4);
    assert_eq!(written["motorised_z"], false, "always written");

    let one = profile_fixture("format1_full_record.json");
    let p = read_record(&file_name_of(&one), &one).unwrap();
    assert!(!p.values.motorised_z, "a format 1 record reads off");
}

#[test]
fn c22_the_key_in_a_format_3_record_damages_it_and_format_5_is_newer() {
    let text = profile_fixture("format3_motorised_z_key.json");
    assert_eq!(
        read_record(&file_name_of(&text), &text).unwrap_err(),
        RecordError::Damaged
    );
    let text = profile_fixture("newer_format.json");
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["format_version"], 5);
    assert_eq!(
        read_record(&file_name_of(&text), &text).unwrap_err(),
        RecordError::Newer
    );
    // A non-boolean in a format 4 record is damage.
    let good = profile_fixture("format4_motorised_z.json");
    let mut v: Value = serde_json::from_str(&good).unwrap();
    v["motorised_z"] = json!("yes");
    let bad = serde_json::to_string_pretty(&v).unwrap();
    assert!(read_record(&file_name_of(&bad), &bad).is_err());
}

#[test]
fn c22_the_generic_laser_has_no_motorised_z() {
    assert!(!MachineProfile::generic_laser().values.motorised_z);
}

#[test]
fn c22_the_page_copy_carries_the_capability() {
    let text = profile_fixture("format4_motorised_z.json");
    let p = read_record(&file_name_of(&text), &text).unwrap();
    let copy = ProfileCopy::of(&p);
    let stored = write_copy(&copy);
    assert_eq!(stored.get("motorised_z"), Some(&json!(true)));
    let back = read_copy(Some(&stored), None);
    assert!(back.copy.values.motorised_z);
    // Absent reads false; a non-boolean reads false.
    let mut no_key = stored.clone();
    no_key.remove("motorised_z");
    assert!(!read_copy(Some(&no_key), None).copy.values.motorised_z);
    let mut bad = stored.clone();
    bad.insert("motorised_z".into(), json!(1));
    assert!(!read_copy(Some(&bad), None).copy.values.motorised_z);
}

// ---- criterion 27: no output, no I/O ---------------------------------------------------------

#[test]
fn c27_the_new_modules_reach_no_network_file_device_or_clock() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    for file in [
        "curvyo-laser-core/src/operation_options.rs",
        "curvyo-laser-core/src/operation_group.rs",
        "curvyo-laser-core/src/operation_naming.rs",
        "curvyo-editor-wasm/src/session/operations/kind_press.rs",
        "curvyo-editor-wasm/src/session/operations/tools.rs",
        "curvyo-editor-wasm/src/session/operations/options.rs",
    ] {
        let text = std::fs::read_to_string(root.join(file)).unwrap();
        for banned in [
            "std::net",
            "std::fs",
            "std::process",
            "reqwest",
            "http://",
            "https://",
            "SystemTime",
            "Instant::now",
            "serialport",
            "gcode",
            "G-code",
            "telemetry",
        ] {
            assert!(!text.contains(banned), "{file} mentions {banned}");
        }
    }
}
