//! Tester acceptance cases for `specs/0150-operation-always-has-numbers` in the laser extension,
//! written from the specification and its ADR notes: the default pair and its limits (criteria 1,
//! 2), the record format 7 and the reading of format 5 and 6 (ADR notes, decision 2), the reading
//! of an operation without `op_settings` as own numbers from the copy's pair (criteria 4, 6, 15,
//! Frage 338 A), a damaged register that stays damaged (criteria 17, 19), and the notices of the
//! limits (`specs/0150` section 2 of the UX notes).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    missing_docs
)]

use curvyo_laser_core::operations::OperationKind;
use curvyo_laser_core::settings::PowerPercent;
use curvyo_laser_core::settings::{SettingsRead, read_settings};
use curvyo_laser_core::{
    DefaultNumbers, FeedRate, MachineProfile, RecordError, clamp_default_power,
    clamp_default_speed, edit_max_feed, new_profile_id, read_record, record_file_name,
    write_record,
};
use serde_json::{Value, json};

fn read(text: &str) -> Result<MachineProfile, RecordError> {
    let v: Value = serde_json::from_str(text).unwrap();
    read_record(&record_file_name(v["id"].as_str().unwrap()), text)
}

fn stored_profile() -> MachineProfile {
    let mut p = MachineProfile::generic_laser();
    p.id = new_profile_id([9; 16]);
    p
}

fn generic_text() -> Value {
    serde_json::from_str(&write_record(&stored_profile())).unwrap()
}

fn with_format(mut v: Value, format: u64) -> String {
    v["format_version"] = json!(format);
    serde_json::to_string_pretty(&v).unwrap()
}

// ---- criterion 1: the built-in pair ----------------------------------------------------------

#[test]
fn c1_generic_laser_has_the_built_in_default_pair_of_10_percent_and_1000() {
    let g = MachineProfile::generic_laser();
    assert_eq!(g.values.defaults, DefaultNumbers::BUILT_IN);
    assert_eq!(DefaultNumbers::BUILT_IN.power, PowerPercent::TEN);
    assert_eq!(DefaultNumbers::BUILT_IN.speed.get(), 1_000);
}

#[test]
fn c1_a_typed_default_power_is_limited_to_1_to_100_with_one_decimal() {
    let over = clamp_default_power(150.0).unwrap();
    assert_eq!(over.value.percent(), 1000.0 / 10.0);
    assert_eq!(
        over.notice.as_deref(),
        Some("Default power limited to 100 %.")
    );
    let under = clamp_default_power(0.0).unwrap();
    assert_eq!(under.value.percent(), 10.0 / 10.0);
    assert_eq!(
        under.notice.as_deref(),
        Some("Default power limited to 1 %.")
    );
    let rounded = clamp_default_power(12.34).unwrap();
    assert_eq!(rounded.value.percent(), 123.0 / 10.0);
    assert_eq!(
        rounded.notice.as_deref(),
        Some("Default power rounded to 12.3 %.")
    );
    let exact = clamp_default_power(12.5).unwrap();
    assert_eq!(exact.notice, None, "a value in range is set without notice");
    assert!(clamp_default_power(f64::NAN).is_none());
}

#[test]
fn c1_a_typed_default_speed_is_limited_to_the_max_feed_with_a_notice() {
    let max = FeedRate::new(6_000).unwrap();
    let over = clamp_default_speed(7_000.0, max).unwrap();
    assert_eq!(over.value.get(), 6_000);
    assert_eq!(
        over.notice.as_deref(),
        Some("Default speed limited to 6,000 mm/min.")
    );
    let under = clamp_default_speed(0.0, max).unwrap();
    assert_eq!(under.value.get(), 1);
    assert_eq!(
        under.notice.as_deref(),
        Some("Default speed limited to 1 mm/min.")
    );
    let rounded = clamp_default_speed(1_250.4, max).unwrap();
    assert_eq!(rounded.value.get(), 1_250);
    assert_eq!(
        rounded.notice.as_deref(),
        Some("Default speed rounded to 1,250 mm/min.")
    );
    for text in [
        "Default power limited to 100 %.",
        "Default speed limited to 6,000 mm/min.",
    ] {
        assert!(
            text.chars().count() <= 70,
            "notice of at most 70 characters"
        );
    }
}

// ---- criterion 2: lowering Max feed clamps Default speed in the same step --------------------

#[test]
fn c2_lowering_max_feed_below_default_speed_lowers_the_default_in_the_same_edit() {
    let mut values = MachineProfile::generic_laser().values;
    values.defaults.speed = FeedRate::new(5_000).unwrap();
    let notice = edit_max_feed(&mut values, 4_000.0);
    assert!(notice.is_some(), "the edit is accepted");
    assert_eq!(values.max_feed.get(), 4_000);
    assert_eq!(
        values.defaults.speed.get(),
        4_000,
        "never above the max feed"
    );
    assert_eq!(
        notice.unwrap().as_deref(),
        Some("Default speed limited to 4,000 mm/min.")
    );
}

#[test]
fn c2_raising_max_feed_leaves_the_default_speed_alone() {
    let mut values = MachineProfile::generic_laser().values;
    values.defaults.speed = FeedRate::new(5_000).unwrap();
    let _ = edit_max_feed(&mut values, 12_000.0);
    assert_eq!(values.defaults.speed.get(), 5_000);
}

// ---- criterion 3, ADR notes decision 2: record format 7 and formats 5 and 6 ----------------

#[test]
fn c3_the_record_is_written_as_format_7_with_the_pair() {
    let text = write_record(&MachineProfile::generic_laser());
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["format_version"], 7);
    assert_eq!(v["default_power_pct"], json!(10.0));
    assert_eq!(v["default_speed_mm_min"], json!(1_000));
}

#[test]
fn c3_a_format_7_record_round_trips_its_pair() {
    let mut p = stored_profile();
    p.values.defaults = DefaultNumbers {
        power: PowerPercent::from_tenths(125).unwrap(),
        speed: FeedRate::new(1_500).unwrap(),
    };
    let back = read(&write_record(&p)).unwrap();
    assert_eq!(back.values.defaults, p.values.defaults);
    assert_eq!(back, p, "the whole record reads back as written");
}

#[test]
fn c3_a_format_6_record_without_the_keys_reads_the_built_in_pair() {
    let text = with_format(generic_text_without_defaults(), 6);
    let read = read(&text).expect("a format 6 record is readable");
    assert_eq!(read.values.defaults, DefaultNumbers::BUILT_IN);
}

#[test]
fn c3_a_format_5_record_reads_the_built_in_pair_too() {
    let mut v = generic_text_without_defaults();
    v.as_object_mut().unwrap().remove("air_assist");
    v.as_object_mut().unwrap().remove("output_format");
    let text = with_format(v, 5);
    let read = read(&text).expect("a format 5 record is readable");
    assert_eq!(read.values.defaults, DefaultNumbers::BUILT_IN);
}

#[test]
fn c3_a_format_6_record_with_the_pair_keys_is_damaged() {
    let mut v = generic_text();
    v["default_power_pct"] = json!(12.0);
    let text = with_format(v, 6);
    assert!(read(&text).is_err(), "a format 6 record holds no pair");
}

#[test]
fn c3_a_format_7_record_without_both_keys_is_damaged() {
    let mut v = generic_text();
    v.as_object_mut().unwrap().remove("default_speed_mm_min");
    assert!(read(&v.to_string()).is_err());
    let mut w = generic_text();
    w.as_object_mut().unwrap().remove("default_power_pct");
    assert!(read(&w.to_string()).is_err());
}

#[test]
fn c3_a_format_7_record_with_an_out_of_range_pair_is_damaged() {
    let mut speed_over = generic_text();
    speed_over["default_speed_mm_min"] = json!(6_001);
    assert!(
        read(&speed_over.to_string()).is_err(),
        "a default speed above the record's max feed is damaged"
    );
    let mut power_two_decimals = generic_text();
    power_two_decimals["default_power_pct"] = json!(12.34);
    assert!(read(&power_two_decimals.to_string()).is_err());
    let mut power_zero = generic_text();
    power_zero["default_power_pct"] = json!(0.0);
    assert!(read(&power_zero.to_string()).is_err());
}

fn generic_text_without_defaults() -> Value {
    let mut v = generic_text();
    v.as_object_mut().unwrap().remove("default_power_pct");
    v.as_object_mut().unwrap().remove("default_speed_mm_min");
    v
}

// ---- criteria 4, 6, 15: an absent register reads as own numbers from the pair ----------------

fn own_value(kind: OperationKind, defaults: DefaultNumbers) -> Value {
    match read_settings(None, kind, defaults) {
        SettingsRead::Stored(stored) => stored.to_value(),
        SettingsRead::Damaged(_) => panic!("an absent register is not damaged"),
    }
}

#[test]
fn c15_an_absent_register_reads_own_numbers_from_the_copy_pair() {
    let pair = DefaultNumbers {
        power: PowerPercent::from_tenths(125).unwrap(),
        speed: FeedRate::new(1_500).unwrap(),
    };
    let v = own_value(OperationKind::Cut, pair);
    assert_eq!(v["main"]["power_pct"], json!(12.5));
    assert_eq!(v["main"]["speed_mm_min"], json!(1_500));
    assert_eq!(v["main"]["passes"], json!(1));
    assert!(v.get("material").is_none(), "no link");
    assert!(
        v.get("overrides").is_none() || v["overrides"].is_null(),
        "no typed override"
    );
}

#[test]
fn c15_the_built_in_pair_reads_as_10_percent_and_1000_for_a_cut() {
    let v = own_value(OperationKind::Cut, DefaultNumbers::BUILT_IN);
    assert_eq!(v["main"]["power_pct"], json!(10.0));
    assert_eq!(v["main"]["speed_mm_min"], json!(1_000));
}

#[test]
fn c15_engrave_and_outline_reads_the_outline_equal_to_main() {
    let v = own_value(OperationKind::EngraveOutline, DefaultNumbers::BUILT_IN);
    assert_eq!(
        v["main"], v["outline"],
        "Engrave + Outline outline equals main"
    );
}

#[test]
fn c15_engrave_reads_the_kind_default_interval() {
    let v = own_value(OperationKind::Engrave, DefaultNumbers::BUILT_IN);
    assert!(
        v["main"].get("interval_mm").is_none() || v["main"]["interval_mm"] == json!(0.1),
        "the interval keeps its default, no override"
    );
}

#[test]
fn c17_a_damaged_register_is_kept_as_damaged_and_never_filled_from_the_pair() {
    let damaged = read_settings(
        Some(&json!("60 %")),
        OperationKind::Cut,
        DefaultNumbers::BUILT_IN,
    );
    assert!(
        matches!(damaged, SettingsRead::Damaged(_)),
        "a register that is no map stays Damaged, never own numbers"
    );
    let zero = read_settings(
        Some(&json!({"main": {"power_pct": 0.0, "speed_mm_min": 1000, "passes": 1}})),
        OperationKind::Cut,
        DefaultNumbers::BUILT_IN,
    );
    assert!(
        matches!(zero, SettingsRead::Damaged(_)),
        "power 0 is damaged"
    );
}

#[test]
fn c19_a_material_link_without_readable_numbers_is_damaged_not_filled() {
    let linked = read_settings(
        Some(
            &json!({"material": {"id": "22222222-2222-4222-8222-222222222222", "name": "Birch", "modified": 5}}),
        ),
        OperationKind::Cut,
        DefaultNumbers::BUILT_IN,
    );
    assert!(matches!(linked, SettingsRead::Damaged(_)));
}
