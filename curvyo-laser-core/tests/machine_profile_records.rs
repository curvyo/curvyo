//! The machine profile record goldens of `tests/fixtures/machine_profiles/`: record format 6 with
//! air assist and the output format reads and writes back byte for byte, format 5 with named
//! fixture sets reads (air assist off, GRBL) and writes as format 6, every damage case is damaged,
//! a newer record is newer, and every record of formats 1 to 4 is skipped as older
//! (`specs/0113-fixture-sets-and-pins` criteria 10, 39 to 41; ADR 0060 §1 to §4, §9;
//! `specs/0029-laser-job-preview-and-output` criteria 25, 32, 39; ADR 0065 §3, §6).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::path::PathBuf;

use curvyo_document_core::JobRole;
use curvyo_laser_core::{
    MAX_RECORD_BYTES, MAX_RECORD_FIXTURES, MAX_RECORD_JOBS, MAX_RECORD_SETS, MAX_RECORD_SLOTS,
    MAX_RECORD_TARGETS, MachineProfiles, OutputFormat, RecordError, read_record, record_file_name,
    write_record,
};

const ID: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";

fn golden(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/machine_profiles")
        .join(name);
    std::fs::read_to_string(path).unwrap()
}

fn read(name: &str) -> Result<curvyo_laser_core::MachineProfile, RecordError> {
    read_record(&record_file_name(ID), &golden(name))
}

/// The format 6 text a format 5 golden writes: the version and the two keys of ADR 0065 §3 after
/// `motorised_z`, off and GRBL.
fn as_format_6(five: &str) -> String {
    let at = five.find("  \"connection\"").unwrap();
    let (head, tail) = five.split_at(at);
    format!(
        "{}  \"air_assist\": false,\n  \"output_format\": \"grbl\",\n{tail}",
        head.replacen("\"format_version\": 5", "\"format_version\": 6", 1)
    )
}

#[test]
fn format_6_air_assist_and_output_format_read_and_write_back_byte_for_byte() {
    let text = golden("format6_air_assist.json");
    let profile = read("format6_air_assist.json").unwrap();
    assert!(profile.values.air_assist);
    assert_eq!(profile.values.output_format, OutputFormat::Grbl);
    assert_eq!(write_record(&profile), text);
    let five = read("format5_fixture_sets.json").unwrap();
    assert!(!five.values.air_assist, "format 5 reads air assist off");
    assert_eq!(five.values.output_format, OutputFormat::Grbl);
    assert_eq!(
        write_record(&five),
        as_format_6(&golden("format5_fixture_sets.json")),
        "any write upgrades to 6"
    );
}

#[test]
fn either_new_key_in_format_5_and_an_unknown_output_format_damage_the_record() {
    for case in [
        "format5_air_assist_key.json",
        "format5_output_format_key.json",
        "format6_unknown_output_format.json",
    ] {
        assert_eq!(read(case), Err(RecordError::Damaged), "{case}");
    }
}

#[test]
fn format_5_sets_read_in_order_and_write_back_byte_for_byte() {
    let text = golden("format5_fixture_sets.json");
    let profile = read("format5_fixture_sets.json").unwrap();
    let names: Vec<&str> = profile
        .fixture_sets
        .iter()
        .map(|set| set.name.as_str())
        .collect();
    assert_eq!(names, ["Bed", "Coaster jig", "Empty"]);
    assert!(profile.fixture_sets[2].fixtures.is_empty(), "an empty set");
    let bed = &profile.fixture_sets[0].fixtures;
    assert_eq!(bed[1].slots.len(), 2);
    assert_eq!(bed[1].jobs[0].targets.len(), 3);
    let screw = &bed[2];
    assert_eq!(screw.params["columns"], 4);
    assert_eq!(screw.params["mark_mm"], 3.0);
    let roles: Vec<Option<JobRole>> = screw.jobs.iter().map(|job| job.role).collect();
    assert_eq!(
        roles,
        [None, Some(JobRole::CentreMark), Some(JobRole::Diameters)],
        "the 0103 job stays beside the role jobs"
    );
    let pin = &profile.fixture_sets[1].fixtures[0];
    assert_eq!(pin.kind, "curvyo.laser/pin");
    assert_eq!(pin.params["diameter_mm"], 6.0);
    assert_eq!(pin.jobs[1].role, Some(JobRole::Diameters));
    assert!(profile.values.motorised_z);
    assert_eq!(write_record(&profile), as_format_6(&text));
    assert!(!text.contains("on_page") && !text.contains("output") && !text.contains("active"));
    // 2026-11-16 (ADR 0060 §1 amended): ids everywhere, a locked set and a locked fixture.
    assert!(profile.fixture_sets[1].locked && !profile.fixture_sets[0].locked);
    assert!(bed[1].locked && !bed[0].locked);
    assert!(profile.fixture_sets.iter().all(|set| !set.id.is_empty()));
    assert!(
        profile
            .fixture_sets
            .iter()
            .flat_map(|set| &set.fixtures)
            .all(|f| !f.id.is_empty() && f.jobs.iter().all(|j| !j.id.is_empty()))
    );
}

#[test]
fn the_maximal_sets_read_and_fit_the_record_limit() {
    let text = golden("format5_max_sets.json");
    assert!(text.len() > 128 * 1024 && text.len() <= MAX_RECORD_BYTES);
    let profile = read("format5_max_sets.json").unwrap();
    let fixtures = || profile.fixture_sets.iter().flat_map(|set| &set.fixtures);
    let slots: usize = fixtures().map(|f| f.slots.len()).sum();
    let jobs: usize = fixtures().map(|f| f.jobs.len()).sum();
    let targets: usize = fixtures()
        .flat_map(|f| &f.jobs)
        .map(|j| j.targets.len())
        .sum();
    assert_eq!(
        (
            profile.fixture_sets.len(),
            fixtures().count(),
            slots,
            jobs,
            targets
        ),
        (
            MAX_RECORD_SETS,
            MAX_RECORD_FIXTURES,
            MAX_RECORD_SLOTS,
            MAX_RECORD_JOBS,
            MAX_RECORD_TARGETS
        )
    );
    assert_eq!(write_record(&profile), as_format_6(&text));
    let one_more = text.replacen(
        "\"targets\": [\n",
        "\"targets\": [\n            \"clamp\",\n",
        1,
    );
    assert_ne!(one_more, text);
    assert_eq!(
        read_record(&record_file_name(ID), &one_more),
        Err(RecordError::Damaged),
        "401 targets"
    );
}

#[test]
fn a_damaged_role_pin_id_or_a_fixtures_or_active_key_damages_the_record() {
    for case in [
        "format5_damaged_role.json",
        "format5_damaged_pin.json",
        "format5_fixtures_key.json",
        // 2026-11-16 (ADR 0060 §1, §9 amended): a repeated id, a missing id, an `active` key.
        "format5_duplicate_id.json",
        "format5_missing_id.json",
        "format5_active_key.json",
    ] {
        assert_eq!(read(case), Err(RecordError::Damaged), "{case}");
    }
    let text = golden("format5_fixture_sets.json");
    for (from, to, why) in [
        (
            "\"pin.circle\"",
            "\"pin.marks\"",
            "a role job with other ticks",
        ),
        (
            "\"role\": \"diameters\"",
            "\"role\": \"edges\"",
            "an unknown role",
        ),
        ("\"name\": \"Empty\"", "\"name\": \"\"", "an empty set name"),
        ("\"columns\": 2", "\"columns\": 0", "a count of 0"),
        ("\"columns\": 4", "\"columns\": 401", "a count above 50"),
        ("\"slot:1\"", "\"slot:2\"", "a slot out of range"),
    ] {
        let damaged = text.replacen(from, to, 1);
        assert_ne!(damaged, text, "{why}");
        assert_eq!(
            read_record(&record_file_name(ID), &damaged),
            Err(RecordError::Damaged),
            "{why}"
        );
    }
    let turned = text.replacen(
        "            1.0,\n            0.0,\n            0.0,\n            1.0,\n            200.0,",
        "            0.0,\n            1.0,\n            -1.0,\n            0.0,\n            200.0,",
        1,
    );
    assert_ne!(turned, text);
    assert_eq!(
        read_record(&record_file_name(ID), &turned),
        Err(RecordError::Damaged),
        "a pin that turns"
    );
}

#[test]
fn every_record_of_formats_1_to_4_is_skipped_as_older() {
    for case in [
        "format1_full_record.json",
        "format2_fixtures.json",
        "format2_damaged_fixture.json",
        "format2_damaged_screw.json",
        "format2_damaged_matrix.json",
        "format2_unknown_kind.json",
        "format2_over_limit.json",
        "format2_jobs_key.json",
        "format3_fixture_jobs.json",
        "format3_max_set.json",
        "format3_damaged_token.json",
        "format3_output_key.json",
        "format3_motorised_z_key.json",
        "format4_motorised_z.json",
    ] {
        assert_eq!(read(case), Err(RecordError::Older), "{case}");
    }
    let library =
        MachineProfiles::load(&[(record_file_name(ID), golden("format4_motorised_z.json"))]);
    assert_eq!(
        library.load_notices(),
        ["1 profile has an older format. It was left as it is."]
    );
    assert!(library.is_skipped(ID) && library.find(ID).is_none());
}

#[test]
fn every_damage_case_is_damaged_and_a_newer_record_is_newer() {
    for case in [
        "damaged_not_json.json",
        "damaged_wrong_type.json",
        "damaged_out_of_range.json",
        "damaged_unknown_key.json",
        "damaged_kind.json",
        "damaged_id_mismatch.json",
    ] {
        assert_eq!(read(case), Err(RecordError::Damaged), "{case}");
    }
    let big = format!(
        "{}{}",
        golden("generic_laser_copy.json"),
        " ".repeat(MAX_RECORD_BYTES)
    );
    assert_eq!(
        read_record(&record_file_name(ID), &big),
        Err(RecordError::Damaged)
    );
    assert_eq!(read("newer_format.json"), Err(RecordError::Newer));
    let without = golden("generic_laser_copy.json").replace("  \"motorised_z\": false,\n", "");
    let name = "0f0e0d0c-0b0a-4908-8706-050403020100.json";
    assert_eq!(read_record(name, &without), Err(RecordError::Damaged));
    let no_sets = golden("generic_laser_copy.json").replace(",\n  \"fixture_sets\": []", "");
    assert_eq!(read_record(name, &no_sets), Err(RecordError::Damaged));
}

#[test]
fn a_duplicated_built_in_reads_as_a_user_profile() {
    let text = golden("generic_laser_copy.json");
    let name = "0f0e0d0c-0b0a-4908-8706-050403020100.json";
    let profile = read_record(name, &text).unwrap();
    assert_eq!(profile.values.name, "Generic laser copy");
    assert!(!profile.is_builtin());
    assert_eq!(write_record(&profile), as_format_6(&text));
}
