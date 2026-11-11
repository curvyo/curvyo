//! Independent acceptance tests for `specs/0103-fixture-jobs` at the profile-record level: record
//! format 3 with `jobs` in each fixture entry (ADR 0052 §7, criterion 23 and 24). Written from the
//! specification and ADR 0052 before the implementation was read. The goldens are the five of ADR
//! 0052 §7 and the older ones that must still read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::needless_pass_by_value,
    missing_docs
)]

use std::path::PathBuf;

use curvyo_laser_core::{read_record, record_file_name, write_record};
use serde_json::Value;

const ID: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";

fn golden(name: &str) -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/machine_profiles")
            .join(name),
    )
    .unwrap_or_else(|e| panic!("golden {name}: {e}"))
}

fn read(name: &str) -> Result<(), ()> {
    read_record(&record_file_name(ID), &golden(name))
        .map(|_| ())
        .map_err(|_| ())
}

#[test]
fn ac24_a_format_3_fixture_jobs_record_reads_and_writes_back_equal() {
    let text = golden("format3_fixture_jobs.json");
    let profile = read_record(&record_file_name(ID), &text).expect("format 3 reads");
    let written = write_record(&profile);
    let before: Value = serde_json::from_str(&text).unwrap();
    let after: Value = serde_json::from_str(&written).unwrap();
    assert_eq!(after, before, "reads, and the write is equal to the file");
    assert_eq!(after["format_version"], 3);
}

#[test]
fn ac24_the_maximum_set_reads_and_its_write_fits_128_kib() {
    let text = golden("format3_max_set.json");
    let profile = read_record(&record_file_name(ID), &text).expect("the maximum reads");
    let written = write_record(&profile);
    assert!(
        written.len() <= 128 * 1024,
        "the maximal set fits the 128 KiB record limit: {} bytes",
        written.len()
    );
    let back = read_record(&record_file_name(ID), &written).expect("the write reads again");
    assert_eq!(write_record(&back), written, "the write is stable");
}

#[test]
fn ac24_a_format_2_record_reads_and_any_write_is_format_3_with_empty_jobs() {
    let text = golden("format2_fixtures.json");
    let profile = read_record(&record_file_name(ID), &text).expect("format 2 reads");
    let written: Value = serde_json::from_str(&write_record(&profile)).unwrap();
    assert_eq!(written["format_version"], 3, "a write is format 3");
    let entries = written["fixtures"]
        .as_array()
        .expect("the fixtures are written");
    assert!(!entries.is_empty(), "the golden has fixtures");
    for entry in entries {
        assert_eq!(
            entry["jobs"],
            serde_json::json!([]),
            "jobs written, possibly empty"
        );
    }
}

#[test]
fn ac24_a_format_2_record_with_a_jobs_key_is_damaged() {
    assert!(
        read("format2_jobs_key.json").is_err(),
        "a jobs key in format 2 damages it"
    );
}

#[test]
fn ac24_a_format_3_record_with_a_grid_tapes_token_on_a_holder_is_damaged() {
    assert!(read("format3_damaged_token.json").is_err());
}

#[test]
fn ac23_a_format_3_entry_with_an_output_key_is_damaged() {
    assert!(
        read("format3_output_key.json").is_err(),
        "the profile never holds Output"
    );
}

#[test]
fn ac24_a_format_4_record_is_skipped_with_its_notice() {
    assert!(
        read("newer_format.json").is_err(),
        "a later format is skipped"
    );
}

#[test]
fn ac24_a_format_1_record_reads_with_no_fixtures() {
    let text = golden("format1_full_record.json");
    let profile = read_record(&record_file_name(ID), &text);
    assert!(profile.is_ok(), "format 1 reads (no fixtures, ADR 0050)");
}

// ---- criteria 13, 14 and 17: the run rule, the run order and the count (laser core) -----------

mod run_rule {
    use curvyo_laser_core::FixturePart;
    use curvyo_laser_core::operations::{FixtureJobRead, KindRead, OperationKind, fixture_block};

    fn job(kind: KindRead, output: bool, active: bool, ticked: usize) -> FixtureJobRead {
        FixtureJobRead {
            kind,
            output,
            fixture_active: active,
            parts: vec![FixturePart::Board; ticked],
        }
    }

    fn known(kind: OperationKind) -> KindRead {
        KindRead::Known(kind)
    }

    const UNKNOWN: KindRead = KindRead::Unknown;

    #[test]
    fn ac13_a_job_runs_only_when_every_condition_holds() {
        let cut = known(OperationKind::Cut);
        assert_eq!(fixture_block(&[job(cut, true, true, 1)], true), [0]);
        assert_eq!(
            fixture_block(&[job(cut, false, true, 1)], true),
            Vec::<usize>::new(),
            "Output off"
        );
        assert_eq!(
            fixture_block(&[job(cut, true, false, 1)], true),
            Vec::<usize>::new(),
            "not Active"
        );
        assert_eq!(
            fixture_block(&[job(UNKNOWN, true, true, 1)], true),
            Vec::<usize>::new(),
            "Unknown kind"
        );
        assert_eq!(
            fixture_block(&[job(cut, true, true, 0)], true),
            Vec::<usize>::new(),
            "no ticked part"
        );
    }

    #[test]
    fn ac14_with_cuts_last_on_marking_runs_before_cutting_and_off_is_list_order() {
        let jobs = [
            job(known(OperationKind::Cut), true, true, 1),
            job(known(OperationKind::Engrave), true, true, 1),
            job(known(OperationKind::CutOut), true, true, 2),
            job(known(OperationKind::EngraveOutline), true, true, 1),
        ];
        assert_eq!(
            fixture_block(&jobs, true),
            [1, 3, 0, 2],
            "Marking first in list order, then Cutting"
        );
        assert_eq!(
            fixture_block(&jobs, false),
            [0, 1, 2, 3],
            "list order when off"
        );
    }

    #[test]
    fn ac17_the_count_is_the_number_of_running_jobs_not_parts() {
        let jobs = [
            job(known(OperationKind::Cut), true, true, 3),
            job(known(OperationKind::Score), true, true, 2),
            job(UNKNOWN, true, true, 4),
            job(known(OperationKind::Engrave), false, true, 4),
        ];
        assert_eq!(
            fixture_block(&jobs, true).len(),
            2,
            "two jobs run; parts and the rest do not count"
        );
    }
}
