//! Independent acceptance tests for `specs/0118-machine-control` slice 2d (criteria 35 a and 36;
//! `adrs.md` P7 and P8 for the profile record): the focus pulse fields, their defaults, their caps
//! and the record format 8 round trip. Written from the specification before reading the
//! implementation.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::missing_panics_doc,
    missing_docs
)]

use std::path::PathBuf;

use curvyo_laser_core::{
    MachineProfile, clamp_pulse_length, clamp_pulse_power, read_record, record_file_name,
    write_record,
};

const ID: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";

fn golden(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/machine_profiles")
        .join(name);
    std::fs::read_to_string(path).unwrap()
}

fn read(text: &str) -> Result<MachineProfile, curvyo_laser_core::RecordError> {
    read_record(&record_file_name(ID), text)
}

// ---------------------------------------------------------------------------------------------
// Criterion 35 a: defaults off, 1 %, 300 ms; the generic laser has them too
// ---------------------------------------------------------------------------------------------

#[test]
fn a_new_profile_has_the_focus_pulse_off_at_one_percent_for_300_ms() {
    let profile = MachineProfile::generic_laser();
    let pulse = profile.values.focus_pulse;
    assert!(!pulse.on, "off by default (Frage 369 B)");
    assert_eq!(pulse.power.get(), 10, "1 % is 10 permille");
    assert_eq!(pulse.length.get(), 300, "300 ms");
}

#[test]
fn a_format_6_record_reads_the_focus_pulse_defaults() {
    let profile = read(&golden("format6_air_assist.json")).unwrap();
    assert!(!profile.values.focus_pulse.on);
    assert_eq!(profile.values.focus_pulse.power.get(), 10);
    assert_eq!(profile.values.focus_pulse.length.get(), 300);
}

// ---------------------------------------------------------------------------------------------
// Criterion 35 a: the clamp on commit: out of range is clamped, never refused
// ---------------------------------------------------------------------------------------------

#[test]
fn the_pulse_power_clamps_to_0_1_to_5_percent_and_rounds_to_a_tenth() {
    let low = clamp_pulse_power(0.0).expect("a clamp, not a refusal");
    assert_eq!(low.value.get(), 1, "0.1 % is the floor");
    assert!(low.notice.is_some(), "a notice for the clamp");
    let high = clamp_pulse_power(9.0).expect("a clamp, not a refusal");
    assert_eq!(high.value.get(), 50, "5 % is the cap");
    assert!(high.notice.is_some());
    let inside = clamp_pulse_power(2.5).expect("in range");
    assert_eq!(inside.value.get(), 25, "2.5 % is 25 permille");
    assert!(inside.notice.is_none(), "no notice inside the range");
    let rounded = clamp_pulse_power(1.0).expect("in range");
    assert_eq!(rounded.value.get(), 10);
}

#[test]
fn the_pulse_length_clamps_to_50_to_2000_ms_and_rounds_to_a_millisecond() {
    let low = clamp_pulse_length(10.0).expect("a clamp");
    assert_eq!(low.value.get(), 50);
    assert!(low.notice.is_some());
    let high = clamp_pulse_length(2500.0).expect("a clamp");
    assert_eq!(high.value.get(), 2000);
    assert!(high.notice.is_some());
    let inside = clamp_pulse_length(300.0).expect("in range");
    assert_eq!(inside.value.get(), 300);
    assert!(inside.notice.is_none());
}

// ---------------------------------------------------------------------------------------------
// Criterion 35 a and P7: record format 8 round trips byte for byte; the keys are written
// ---------------------------------------------------------------------------------------------

#[test]
fn a_format_8_record_with_the_focus_pulse_keys_reads_and_writes_back_unchanged() {
    let text = golden("format8_focus_pulse.json");
    let profile = read(&text).unwrap();
    assert_eq!(
        write_record(&profile),
        text,
        "format 8 round trip, byte for byte"
    );
}

#[test]
fn a_format_8_record_keeps_a_switched_on_pulse_with_its_power_and_length() {
    let text = golden("format8_focus_pulse.json")
        .replace("\"focus_pulse\": false", "\"focus_pulse\": true")
        .replace(
            "\"focus_pulse_power_permille\": 10",
            "\"focus_pulse_power_permille\": 25",
        )
        .replace(
            "\"focus_pulse_length_ms\": 300",
            "\"focus_pulse_length_ms\": 500",
        );
    let profile = read(&text).unwrap();
    let pulse = profile.values.focus_pulse;
    assert!(pulse.on);
    assert_eq!(pulse.power.get(), 25);
    assert_eq!(pulse.length.get(), 500);
    let written = write_record(&profile);
    assert!(written.contains("\"focus_pulse\": true"), "{written}");
    assert!(
        written.contains("\"focus_pulse_power_permille\": 25"),
        "{written}"
    );
    assert!(
        written.contains("\"focus_pulse_length_ms\": 500"),
        "{written}"
    );
    assert_eq!(
        read(&written).unwrap().values.focus_pulse,
        pulse,
        "a second round trip agrees"
    );
}

#[test]
fn a_format_6_record_with_a_focus_pulse_key_is_damaged_and_refused() {
    let text = golden("format6_focus_pulse_key.json");
    assert!(
        read(&text).is_err(),
        "a format 6 record that names the focus pulse keys is damaged (ADR 0065 section 3)"
    );
}

#[test]
fn a_record_from_a_newer_format_is_refused() {
    let text = golden("newer_format.json");
    assert!(read(&text).is_err(), "newer formats are refused");
}
