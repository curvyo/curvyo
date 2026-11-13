//! Goldens of material record format 2 (ADR 0058 §6, §10; `specs/0112` criteria 19, 22, 30).

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use curvyo_laser_core::grids::TestId;
use curvyo_laser_core::library::{
    MAX_RECORD_TEXT, MaterialRead, PhotoKind, RecordFileError, SetSource, load_record, read_record,
    write_record,
};
use curvyo_laser_core::settings::{CellIndex, SetKind};

const ID: &str = "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c";
const DIODE: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";

fn fixture(name: &str) -> String {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests/fixtures/materials", name]
        .iter()
        .collect();
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn file_name() -> String {
    format!("{ID}.json")
}

fn read(name: &str) -> Result<curvyo_laser_core::library::Material, RecordFileError> {
    read_record(&file_name(), &fixture(name))
}

fn test_id(text: &str) -> TestId {
    TestId::parse(text).unwrap()
}

#[test]
fn the_full_record_reads_and_writes_back_byte_for_byte() {
    let text = fixture("format2_full.json");
    let material = read("format2_full.json").unwrap();
    assert_eq!(write_record(&material), text);
    assert_eq!(material.lasers.len(), 2);
    let cut = material.set_for(DIODE, SetKind::Cut).unwrap();
    assert_eq!(cut.source, SetSource::Test(test_id("7K2QX")));
    let engrave = material.set_for(DIODE, SetKind::Engrave).unwrap();
    assert_eq!(engrave.numbers.interval.unwrap().hundredths(), 8);
    assert_eq!(engrave.source, SetSource::Typed);
    assert_eq!(material.set_for("generic-laser", SetKind::Cut), None);
    assert_eq!(material.set_count(), 3);
    let lasered = material.test(test_id("7K2QX")).unwrap();
    assert_eq!(lasered.best, Some(CellIndex { column: 2, row: 1 }));
    assert_eq!(lasered.photo, Some(PhotoKind::Jpg));
    assert!(lasered.lasered.is_some());
    assert_eq!(lasered.note, "Clean edge at 60 %.");
    let engrave_test = material.test(test_id("4H8ZP")).unwrap();
    assert!(engrave_test.recipe.crosshatch());
    assert_eq!(engrave_test.lasered, None);
    assert_eq!(material.history.len(), 3);
}

#[test]
fn the_one_laser_and_the_maximal_record_read_and_write_back() {
    for name in ["format2_one_laser.json", "format2_max.json"] {
        let text = fixture(name);
        let material = read(name).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(write_record(&material), text, "{name}");
    }
    let max = fixture("format2_max.json");
    assert!(max.len() <= MAX_RECORD_TEXT, "{} bytes", max.len());
    let material = read("format2_max.json").unwrap();
    assert_eq!(
        (
            material.lasers.len(),
            material.tests.len(),
            material.history.len()
        ),
        (16, 200, 500)
    );
}

#[test]
fn every_damage_of_adr_0058_section_6_skips_the_record_with_its_reason() {
    let cases = [
        (
            "duplicate_laser.json",
            "The laser Diode 20 W is listed twice.",
        ),
        ("set_wrong_kind.json", "Unknown key cut_out."),
        ("interval_missing.json", "An Engrave set has no interval."),
        ("interval_on_cut.json", "A Cut set has an interval."),
        (
            "source_no_test.json",
            "The Score set of Generic laser names no fitting test.",
        ),
        (
            "test_id_bad.json",
            "Test id 8pl4d is not 5 characters of 0-9A-Z.",
        ),
        ("test_id_twice.json", "Test 7K2QX is listed twice."),
        (
            "best_outside.json",
            "The best cell of test 8PL4D is outside its grid.",
        ),
        (
            "note_501.json",
            "The note of test 8PL4D has more than 500 characters.",
        ),
        (
            "photo_gif.json",
            "The photo of test 7K2QX is not jpg or png.",
        ),
        ("tests_201.json", "The record has 201 tests, more than 200."),
    ];
    for (name, reason) in cases {
        assert_eq!(
            read(name),
            Err(RecordFileError::Damaged(reason.to_owned())),
            "{name}"
        );
        assert!(reason.chars().count() <= 70, "{reason}");
        assert_eq!(
            load_record(&file_name(), &fixture(name)),
            MaterialRead::Damaged {
                id: ID.to_owned(),
                reason: reason.to_owned()
            }
        );
    }
}

#[test]
fn a_newer_record_is_newer_and_a_format_1_record_is_skipped_as_older() {
    assert_eq!(read("newer_format.json"), Err(RecordFileError::Newer));
    assert_eq!(read("full.json"), Err(RecordFileError::Older));
    assert_eq!(
        load_record(&file_name(), &fixture("full.json")),
        MaterialRead::Older { id: ID.to_owned() }
    );
}

#[test]
fn a_record_over_512_kib_or_with_another_id_is_damaged() {
    let mut big = fixture("format2_one_laser.json");
    big.push_str(&" ".repeat(MAX_RECORD_TEXT));
    assert_eq!(
        read_record(&file_name(), &big),
        Err(RecordFileError::Damaged(
            "The file is over 512 KiB.".to_owned()
        ))
    );
    assert_eq!(
        read_record(
            "11111111-1111-4111-8111-111111111111.json",
            &fixture("format2_full.json")
        ),
        Err(RecordFileError::Damaged(
            "The id differs from the file name.".to_owned()
        ))
    );
}
