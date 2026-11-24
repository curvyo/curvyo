//! `specs/0150-operation-always-has-numbers` in the laser extension: the default pair of the
//! profile copy and of record format 7 (criteria 1 to 3, `adrs.md` decisions 1 and 2), the read
//! of an operation without `op_settings` (criteria 4, 15, 17 to 19, 23) and its G-code
//! (criterion 26).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

mod job_fixtures;

use std::path::PathBuf;

use curvyo_laser_core::job::{Cause, GateFacts, PlanCache, build_job, write};
use curvyo_laser_core::operations::OperationKind;
use curvyo_laser_core::settings::{
    PowerPercent, SettingsDamage, SettingsRead, SettingsState, read_settings, resolve,
};
use curvyo_laser_core::{
    DefaultNumbers, FeedRate, MachineProfile, OutputFormat, ProfileCopy, RecordError, differs,
    read_copy, read_record, record_file_name, write_copy, write_record,
};
use job_fixtures::{Page, leaf, own, plain, rect};
use serde_json::{Value, json};

const ID: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";

fn golden(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/machine_profiles")
        .join(name);
    std::fs::read_to_string(path).unwrap()
}

fn read(name: &str) -> Result<MachineProfile, RecordError> {
    read_record(&record_file_name(ID), &golden(name))
}

fn pair(power: f64, speed: u64) -> DefaultNumbers {
    DefaultNumbers {
        power: PowerPercent::from_percent(power).unwrap(),
        speed: FeedRate::new(speed).unwrap(),
    }
}

fn copy_of(value: Value) -> curvyo_extension_core::StoredParams {
    serde_json::from_value(value).unwrap()
}

#[test]
fn criterion_3_the_copy_carries_the_pair_and_reads_each_field_on_its_own() {
    let mut profile = MachineProfile::generic_laser();
    profile.values.defaults = pair(12.5, 1500);
    let copy = ProfileCopy::of(&profile);
    let map = write_copy(&copy);
    assert_eq!(map["default_power_pct"], json!(12.5));
    assert_eq!(map["default_speed_mm_min"], json!(1500));
    assert_eq!(read_copy(Some(&map), None).copy, copy);
    // A copy made before this story: the built-in pair, no damage flag, nothing rewritten.
    let old = copy_of(json!({"id": "generic-laser", "name": "Generic laser"}));
    let read = read_copy(Some(&old), None);
    assert_eq!(read.copy.values.defaults, DefaultNumbers::BUILT_IN);
    assert_eq!(read.damaged_values(), 0);
    assert!(
        !old.contains_key("default_power_pct"),
        "never rewritten on read"
    );
    // W5: a copy whose max feed is under 1,000 reads the max feed.
    let slow = copy_of(json!({"name": "Slow", "max_feed_mm_per_min": 600}));
    assert_eq!(
        read_copy(Some(&slow), None).copy.values.defaults,
        pair(10.0, 600)
    );
    // Per field: a damaged power reads the built-in power and keeps the readable speed.
    let damaged = copy_of(json!({
        "name": "K40",
        "default_power_pct": 0,
        "default_speed_mm_min": 1200,
    }));
    assert_eq!(
        read_copy(Some(&damaged), None).copy.values.defaults,
        pair(10.0, 1200)
    );
    let two_decimals = copy_of(json!({"name": "K40", "default_power_pct": 12.55}));
    assert_eq!(
        read_copy(Some(&two_decimals), None)
            .copy
            .values
            .defaults
            .power,
        PowerPercent::TEN
    );
    // A damaged name leaves a readable pair in use (adrs.md decision 1).
    let bad_name = copy_of(json!({"name": "", "default_power_pct": 20.0}));
    let read = read_copy(Some(&bad_name), None);
    assert!(read.name_damaged);
    assert_eq!(read.copy.values.defaults, pair(20.0, 1000));
    // A format-35 manufacturing reads the built-in pair.
    assert_eq!(
        read_copy(None, Some("K40")).copy.values.defaults,
        DefaultNumbers::BUILT_IN
    );
}

#[test]
fn criterion_3_differs_from_profile_compares_the_pair() {
    let profile = MachineProfile::generic_laser();
    let mut copy = ProfileCopy::of(&profile);
    assert!(!differs(&copy, profile.work_area, &profile));
    copy.values.defaults.speed = FeedRate::new(900).unwrap();
    assert!(differs(&copy, profile.work_area, &profile));
}

#[test]
fn record_format_7_round_trips_byte_for_byte() {
    let text = golden("format7_defaults.json");
    let profile = read("format7_defaults.json").unwrap();
    assert_eq!(profile.values.defaults, pair(12.5, 1500));
    assert!(profile.values.air_assist);
    assert_eq!(profile.values.output_format, OutputFormat::Grbl);
    assert_eq!(write_record(&profile), text);
    let generic = read_record(
        "0f0e0d0c-0b0a-4908-8706-050403020100.json",
        &golden("generic_laser_copy.json"),
    )
    .unwrap();
    assert_eq!(generic.values.defaults, DefaultNumbers::BUILT_IN);
}

#[test]
fn record_formats_5_and_6_read_the_built_in_pair_and_are_not_rewritten() {
    for name in ["format5_fixture_sets.json", "format6_air_assist.json"] {
        let before = golden(name);
        let profile = read(name).unwrap();
        assert_eq!(profile.values.defaults, DefaultNumbers::BUILT_IN, "{name}");
        assert_eq!(golden(name), before, "{name}: nothing written on read");
        assert!(write_record(&profile).starts_with("{\n  \"format_version\": 7,"));
    }
}

#[test]
fn record_damage_of_the_pair() {
    for case in [
        "format6_default_power_key.json",
        "format7_missing_default_speed.json",
        "format7_default_speed_over_max_feed.json",
        "format7_default_power_two_decimals.json",
    ] {
        assert_eq!(read(case), Err(RecordError::Damaged), "{case}");
    }
    let text = golden("format7_defaults.json");
    for (from, to) in [
        ("\"default_power_pct\": 12.5", "\"default_power_pct\": 0.5"),
        (
            "\"default_power_pct\": 12.5",
            "\"default_power_pct\": 100.1",
        ),
        (
            "\"default_speed_mm_min\": 1500",
            "\"default_speed_mm_min\": 0",
        ),
    ] {
        let damaged = text.replacen(from, to, 1);
        assert_eq!(
            read_record(&record_file_name(ID), &damaged),
            Err(RecordError::Damaged),
            "{to}"
        );
    }
    assert_eq!(read("newer_format.json"), Err(RecordError::Newer));
}

#[test]
fn criteria_4_15_an_operation_without_settings_runs_own_numbers_from_the_pair() {
    let defaults = pair(12.5, 1500);
    for kind in OperationKind::ALL {
        let read = read_settings(None, kind, defaults);
        let SettingsState::Own(run) = resolve(&read, kind, &[], "p") else {
            panic!("{kind:?}: own numbers");
        };
        assert_eq!(run.main, defaults.settings());
        assert_eq!(run.main.passes.get(), 1);
        assert_eq!(
            run.outline,
            (kind == OperationKind::EngraveOutline).then_some(defaults.settings())
        );
        assert_eq!(
            run.interval.is_some(),
            kind.has_interval(),
            "{kind:?}: the kind's default interval (criterion 5)"
        );
    }
}

#[test]
fn criteria_17_19_damage_stays_damage() {
    let defaults = DefaultNumbers::BUILT_IN;
    let zero = json!({"main": {"power_pct": 0.0, "speed_mm_min": 300, "passes": 1}});
    assert_eq!(
        read_settings(Some(&zero), OperationKind::Cut, defaults),
        SettingsRead::Damaged(SettingsDamage::BadSet)
    );
    let link_only = json!({"material": {"id": "m", "name": "Birch", "modified": 1}});
    assert_eq!(
        read_settings(Some(&link_only), OperationKind::Cut, defaults),
        SettingsRead::Damaged(SettingsDamage::BadSet),
        "a link without readable numbers in main is never filled"
    );
}

#[test]
fn criteria_23_26_derived_numbers_never_block_and_write_the_typed_g_code() {
    let job_text = |numbers: SettingsRead| {
        let mut page = Page::new();
        page.operation(
            OperationKind::Cut,
            "Cut 1",
            numbers,
            plain(),
            vec![leaf(vec![rect(10.0, 10.0, 20.0, 10.0)])],
        );
        let built = build_job(&page.input, &[], &mut PlanCache::default());
        let timeline = curvyo_laser_core::job::Timeline::of(&built.job);
        let gate =
            curvyo_laser_core::job::gate(&page.input, &built, &timeline, &GateFacts::default());
        assert!(gate.export_allowed());
        assert!(
            !gate
                .issues
                .iter()
                .any(|issue| matches!(issue.cause, Cause::DamagedSettings)),
        );
        write(OutputFormat::Grbl, &built.job)
    };
    let derived = job_text(read_settings(
        None,
        OperationKind::Cut,
        DefaultNumbers::BUILT_IN,
    ));
    let typed = job_text(own(OperationKind::Cut, 10.0, 1000, 1));
    assert!(derived.len() > 10, "a G-code text");
    assert_eq!(derived, typed, "no marker, no warning");
}
