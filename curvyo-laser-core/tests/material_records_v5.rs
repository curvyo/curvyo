//! Material record format 5 (`specs/0146-material-test-picks` criteria 10, 11, 23; ADR 0093
//! decisions 2, 4, 5): marks on tests, pick sets per laser, the damage rules and the 640 KiB limit.

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use curvyo_laser_core::grids::TestId;
use curvyo_laser_core::library::{
    MAX_RECORD_TEXT, MaterialRead, RECORD_FORMAT_VERSION, RecordFileError, SetSource, load_record,
    read_record, write_record,
};
use curvyo_laser_core::settings::{CellIndex, PickKind, SetKind};

const ID: &str = "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c";
const DIODE: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";

fn fixture(name: &str) -> String {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests/fixtures/materials", name]
        .iter()
        .collect();
    std::fs::read_to_string(path).unwrap()
}

fn read(name: &str) -> Result<curvyo_laser_core::library::Material, RecordFileError> {
    read_record(&format!("{ID}.json"), &fixture(name))
}

fn cell(column: u8, row: u8) -> CellIndex {
    CellIndex { column, row }
}

#[test]
fn the_format_is_5_and_the_limit_640_kib() {
    assert_eq!(RECORD_FORMAT_VERSION, 5);
    assert_eq!(MAX_RECORD_TEXT, 640 * 1024);
}

/// Criteria 10, 11: four picks on three cells (one cell with two), an Engrave pick with its
/// interval and a pick whose test is gone read, and write back byte for byte.
#[test]
fn a_record_with_picks_round_trips_byte_for_byte() {
    let text = fixture("format5_picks.json");
    let material = read("format5_picks.json").unwrap();
    assert_eq!(write_record(&material), text);
    let judged = material.test(TestId::parse("7K2QX").unwrap()).unwrap();
    assert_eq!(judged.marks.best, Some(cell(2, 1)));
    assert_eq!(judged.marks.second_best, Some(cell(1, 1)));
    assert_eq!(judged.marks.fastest, Some(cell(3, 3)));
    assert_eq!(judged.marks.deepest, Some(cell(3, 3)));
    assert_eq!(
        judged.marks.kinds_of(cell(3, 3)),
        [PickKind::Fastest, PickKind::Deepest]
    );
    let fastest = material
        .pick_set(DIODE, SetKind::Cut, PickKind::Fastest)
        .unwrap();
    assert_eq!(fastest.numbers.settings.power.tenths(), 800);
    assert_eq!(fastest.source, SetSource::Test(judged.id));
    let best = material
        .pick_set(DIODE, SetKind::Cut, PickKind::Best)
        .unwrap();
    assert_eq!(best.numbers.settings.power.tenths(), 600);
    let second = material
        .pick_set(DIODE, SetKind::Engrave, PickKind::SecondBest)
        .unwrap();
    assert_eq!(second.numbers.interval.unwrap().hundredths(), 5);
    assert!(
        material
            .pick_set(DIODE, SetKind::Engrave, PickKind::Fastest)
            .is_none()
    );
}

/// Criterion 10: a pick whose test was deleted reads "Test <id>" and the record stays valid.
#[test]
fn a_pick_of_a_deleted_test_is_valid() {
    let material = read("format5_picks.json").unwrap();
    let pick = material
        .pick_set("generic-laser", SetKind::Score, PickKind::Deepest)
        .unwrap();
    assert_eq!(
        pick.source,
        SetSource::Test(TestId::parse("ZZZZZ").unwrap())
    );
    assert!(material.test(TestId::parse("ZZZZZ").unwrap()).is_none());
}

/// Criterion 11: a record of format 4 or lower opens unchanged and has Best only; reads write
/// nothing (the written text is the reader's input untouched until a save).
#[test]
fn an_older_record_has_best_only() {
    let material = read("format2_full.json").unwrap();
    let old = material.test(TestId::parse("7K2QX").unwrap()).unwrap();
    assert_eq!(old.marks.best, Some(cell(2, 1)));
    assert!(old.marks.second_best.is_none() && old.marks.fastest.is_none());
    assert!(material.lasers.iter().all(|l| l.picks.is_empty()));
}

/// Criterion 23: each damage of ADR 0093 decision 5 names its reason.
#[test]
fn every_pick_damage_skips_the_record_with_its_reason() {
    let cases = [
        (
            "format5_mark_outside.json",
            "The fastest cell of test 7K2QX is outside its grid.",
        ),
        ("format5_pick_best_key.json", "Unknown key best."),
        ("format5_pick_kind_unknown.json", "Unknown key sepia."),
        (
            "format5_pick_wrong_test.json",
            "The Cut Fastest pick of Diode 20 W names no fitting test.",
        ),
        (
            "format5_pick_no_interval.json",
            "An Engrave set has no interval.",
        ),
        (
            "format5_pick_interval_on_cut.json",
            "A Cut set has an interval.",
        ),
        (
            "format5_pick_source_typed.json",
            "The Cut Second best pick of Diode 20 W has no test as its source.",
        ),
        (
            "format5_picks_empty.json",
            "The picks of Diode 20 W are empty.",
        ),
        (
            "format5_picks_kind_empty.json",
            "The Cut picks of Diode 20 W are empty.",
        ),
        (
            "format4_with_picks.json",
            "A format-4 record holds pick sets.",
        ),
        (
            "format4_with_mark.json",
            "A format-4 record marks a second best cell.",
        ),
    ];
    for (name, reason) in cases {
        assert_eq!(
            load_record(&format!("{ID}.json"), &fixture(name)),
            MaterialRead::Damaged {
                id: ID.to_owned(),
                reason: reason.to_owned()
            },
            "{name}"
        );
    }
}

/// Criterion 11: a record above format 5 is skipped as newer.
#[test]
fn a_format_6_record_is_newer() {
    assert_eq!(read("newer_format.json"), Err(RecordFileError::Newer));
}

/// Criterion 11 and ADR 0093 decision 6: every mark and every pick set of the old maximum fits
/// the 640 KiB limit and reads back byte for byte.
#[test]
fn the_maximal_record_with_every_pick_fits_640_kib() {
    let text = fixture("format5_max.json");
    assert!(text.len() <= MAX_RECORD_TEXT, "{} bytes", text.len());
    let material = read("format5_max.json").unwrap();
    assert_eq!(write_record(&material), text);
}
