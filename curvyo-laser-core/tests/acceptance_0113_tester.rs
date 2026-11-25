//! Independent acceptance tests for `specs/0113-fixture-sets-and-pins` in laser-core: the pin and
//! grid limits (criteria 18, 19, 25), the grid centres (19), the profile record format 5 with its
//! named sets (10), records of formats 1 to 4 skipped as older with the counted notice (39), a
//! damaged set skipping the record (40) and a newer record skipped (41). Written from the
//! specification and ADR 0060 before the implementation was read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    missing_docs
)]

use std::path::PathBuf;

use curvyo_document_core::{Length, Point};
use curvyo_laser_core::{
    FixtureKind, GridCounts, MachineProfiles, RecordError, centres, clamp_count, mark_range,
    pin_diameter_range, read_record, record_file_name, spacing_range, write_record,
};

const ID: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";

fn golden(name: &str) -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/machine_profiles")
            .join(name),
    )
    .unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn mm(v: f64) -> Length {
    Length::from_mm(v)
}

// ---- criteria 18, 19, 25: ranges and defaults ----------------------------------------------------

#[test]
fn ac18_pin_kind_and_diameter_range() {
    assert_eq!(FixtureKind::from_stored("curvyo.laser/pin").word(), "Pin");
    let r = pin_diameter_range();
    assert_eq!(r.clamp(mm(0.1)), (mm(0.5), true));
    assert_eq!(r.clamp(mm(2000.0)), (mm(1000.0), true));
    assert_eq!(r.clamp(mm(6.0)), (mm(6.0), false));
}

#[test]
fn ac19_25_spacing_and_mark_ranges() {
    assert_eq!(spacing_range().clamp(mm(0.5)), (mm(1.0), true));
    assert_eq!(spacing_range().clamp(mm(1001.0)), (mm(1000.0), true));
    assert_eq!(mark_range().clamp(mm(0.1)), (mm(0.5), true));
    assert_eq!(mark_range().clamp(mm(51.0)), (mm(50.0), true));
}

#[test]
fn ac19_counts_are_1_to_50_and_the_product_at_most_400() {
    assert!(GridCounts::new(1, 1).is_some());
    assert!(GridCounts::new(50, 8).is_some());
    assert!(GridCounts::new(20, 20).is_some());
    assert!(GridCounts::new(50, 9).is_none(), "450 centres");
    assert!(GridCounts::new(51, 1).is_none());
    assert!(GridCounts::new(0, 1).is_none());
    assert_eq!(clamp_count(50, 9), (8, true), "the largest allowed count");
    assert_eq!(clamp_count(1, 70), (50, true));
    assert_eq!(clamp_count(4, 5), (5, false));
}

#[test]
fn ac19_centres_run_right_by_columns_and_down_by_rows_from_the_first() {
    let c = centres(
        Point::new(10.0, 20.0),
        GridCounts::new(3, 2).unwrap(),
        mm(5.0),
        mm(7.0),
    );
    assert_eq!(c.len(), 6);
    let mut xs: Vec<(f64, f64)> = c.iter().map(|p| (p.x, p.y)).collect();
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(
        xs,
        [
            (10.0, 20.0),
            (10.0, 27.0),
            (15.0, 20.0),
            (15.0, 27.0),
            (20.0, 20.0),
            (20.0, 27.0)
        ]
    );
    assert_eq!(
        centres(
            Point::new(0.0, 0.0),
            GridCounts::new(50, 8).unwrap(),
            mm(1.0),
            mm(1.0)
        )
        .len(),
        400
    );
}

// ---- criteria 10, 39 to 41: the profile record ---------------------------------------------------

#[test]
fn ac10_a_format_5_record_reads_its_named_sets_and_writes_back_equal() {
    let text = golden("format5_fixture_sets.json");
    let p = read_record(&record_file_name(ID), &text).expect("reads");
    assert_ne!(p.fixture_sets.len(), 0, "named sets are read");
    assert_eq!(p.fixture_sets[0].name, "Bed");
    let again = read_record(&record_file_name(ID), &write_record(&p)).unwrap();
    assert_eq!(again, p, "round trip");
    let v: serde_json::Value = serde_json::from_str(&write_record(&p)).unwrap();
    // Any write is record format 7 since `0150`.
    // Any write is record format 8 since `0118` 2d (adrs P7).
    assert_eq!(v["format_version"], 8);
    assert!(v.get("fixtures").is_none());
    assert!(
        !write_record(&p).contains("on_page"),
        "no On this page in a record"
    );
}

#[test]
fn ac39_records_of_formats_1_to_4_are_skipped_as_older() {
    for name in [
        "format1_full_record.json",
        "format2_fixtures.json",
        "format3_fixture_jobs.json",
        "format4_motorised_z.json",
    ] {
        assert_eq!(
            read_record(&record_file_name(ID), &golden(name)),
            Err(RecordError::Older),
            "{name}"
        );
    }
}

#[test]
fn ac39_the_library_counts_older_records_in_one_notice_and_keeps_them() {
    let one = MachineProfiles::load(&[(record_file_name(ID), golden("format4_motorised_z.json"))]);
    assert!(one.find(ID).is_none(), "not read");
    assert!(one.is_skipped(ID), "its id is left alone");
    assert_eq!(
        one.load_notices(),
        ["1 profile has an older format. It was left as it is."]
    );
    let two = MachineProfiles::load(&[
        (record_file_name(ID), golden("format4_motorised_z.json")),
        (
            record_file_name("7f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f"),
            golden("format3_fixture_jobs.json").replace(ID, "7f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f"),
        ),
    ]);
    assert_eq!(two.load_notices().len(), 1, "{:?}", two.load_notices());
    assert!(two.load_notices()[0].starts_with("2 profiles have an older format"));
}

#[test]
fn ac40_a_damaged_set_value_skips_the_whole_record() {
    for name in [
        "format5_damaged_pin.json",
        "format5_damaged_role.json",
        "format5_fixtures_key.json",
    ] {
        assert_eq!(
            read_record(&record_file_name(ID), &golden(name)),
            Err(RecordError::Damaged),
            "{name}"
        );
    }
}

#[test]
fn ac40_a_set_without_a_name_or_a_pin_without_a_diameter_is_damaged() {
    let text = golden("format5_fixture_sets.json");
    let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
    v["fixture_sets"][0].as_object_mut().unwrap().remove("name");
    assert_eq!(
        read_record(&record_file_name(ID), &v.to_string()),
        Err(RecordError::Damaged),
        "a missing set name"
    );
    let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
    let mut found = false;
    for set in v["fixture_sets"].as_array_mut().unwrap() {
        for f in set["fixtures"].as_array_mut().unwrap() {
            if f["kind"] == "pin" {
                f["params"]["columns"] = serde_json::json!(0);
                found = true;
            }
        }
    }
    if found {
        assert_eq!(
            read_record(&record_file_name(ID), &v.to_string()),
            Err(RecordError::Damaged),
            "a count of 0"
        );
    }
}

#[test]
fn ac41_a_newer_record_is_skipped() {
    assert_eq!(
        read_record(&record_file_name(ID), &golden("newer_format.json")),
        Err(RecordError::Newer)
    );
}
