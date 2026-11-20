//! Material record format 3 (`specs/0123` criteria 41, 45; ADR 0069 §8, §13, §14, Frage 237 A,
//! Frage 270 B): an Image test's recipe keeps its Dither, so Place on page rebuilds it with its
//! dither; a laser holds an Image set; a format-2 record holds neither and is written back as
//! format 3.

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use curvyo_laser_core::grids::{Recipe, TestId, derived_operations};
use curvyo_laser_core::library::{
    RECORD_FORMAT_VERSION, RecordFileError, read_record, write_record,
};
use curvyo_laser_core::operations::OperationKind;
use curvyo_laser_core::settings::{Dither, SetKind};

const ID: &str = "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c";

fn fixture(name: &str) -> String {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests/fixtures/materials", name]
        .iter()
        .collect();
    std::fs::read_to_string(path).unwrap()
}

fn read(text: &str) -> Result<curvyo_laser_core::library::Material, RecordFileError> {
    read_record(&format!("{ID}.json"), text)
}

fn test_id(text: &str) -> TestId {
    TestId::parse(text).unwrap()
}

#[test]
fn the_format_is_3() {
    // Superseded by 0135 slice 1 (2026-11-19): format 4 (ADR 0072 §6).
    assert_eq!(RECORD_FORMAT_VERSION, 4);
}

#[test]
fn an_image_test_keeps_its_dither_and_sources_the_image_set_byte_for_byte() {
    let text = fixture("format3_image_test.json");
    let material = read(&text).unwrap();
    // Superseded by 0135 slice 1 (2026-11-19): written back as format 4.
    assert_eq!(
        write_record(&material),
        text.replacen("\"format_version\": 3,", "\"format_version\": 4,", 1)
    );
    let image = material.test(test_id("9GR4D")).unwrap();
    assert_eq!(image.recipe.kind(), SetKind::Image);
    assert_eq!(image.recipe.dither(), Some(Dither::Ordered));
    let engrave = material.test(test_id("5SL1D")).unwrap();
    assert_eq!(
        engrave.recipe.dither(),
        None,
        "an Engrave test has no dither"
    );
    let sets = &material.lasers[0].sets;
    assert_eq!(
        sets[&SetKind::Image].numbers.interval.unwrap().hundredths(),
        10
    );
    assert_eq!(
        sets[&SetKind::Engrave]
            .numbers
            .interval
            .unwrap()
            .hundredths(),
        15
    );
    assert_eq!(material.history[0].kind, SetKind::Image);
    // Place on page rebuilds the same grid: its cells are Image operations with the dither.
    let recipe = Recipe {
        id: image.id,
        test: image.recipe.clone(),
        material: None,
    };
    let ops = derived_operations(&recipe, None);
    assert!(
        ops[1..].iter().all(|op| op.kind == OperationKind::Image
            && op.options.unwrap().image.dither == Dither::Ordered)
    );
}

#[test]
fn a_format_2_record_reads_as_is_and_is_written_as_format_3() {
    let text = fixture("format2_reads_solid.json");
    let material = read(&text).unwrap();
    assert!(material.tests.iter().all(|t| t.recipe.dither().is_none()));
    let written = write_record(&material);
    // Superseded by 0135 slice 1 (2026-11-19): as format 4.
    assert_eq!(
        written,
        text.replacen("\"format_version\": 2,", "\"format_version\": 4,", 1)
    );
    assert_eq!(read(&written).unwrap(), material);
}

#[test]
fn an_image_test_in_a_format_2_record_or_a_grey_dither_is_damage() {
    let image = fixture("format3_image_test.json");
    let as_two = image.replacen("\"format_version\": 3,", "\"format_version\": 2,", 1);
    assert_eq!(
        read(&as_two),
        Err(RecordFileError::Damaged(
            "The recipe of test 9GR4D is not valid.".to_owned()
        ))
    );
    let grey = image.replacen("\"dither\": \"ordered\"", "\"dither\": \"grey\"", 1);
    assert_eq!(
        read(&grey),
        Err(RecordFileError::Damaged(
            "The recipe of test 9GR4D is not valid.".to_owned()
        ))
    );
    let fill = image.replacen(
        "\"dither\": \"ordered\"",
        "\"fill\": \"gradient\",\n        \"dither\": \"ordered\"",
        1,
    );
    assert!(read(&fill).is_err(), "`fill` left the recipe");
}

#[test]
fn an_image_set_needs_an_interval_and_a_format_2_record_holds_none() {
    let two = fixture("format2_image_set.json");
    assert_eq!(
        read(&two),
        Err(RecordFileError::Damaged(
            "A format-2 record holds an Image set.".to_owned()
        ))
    );
    let three = two.replacen("\"format_version\": 2,", "\"format_version\": 3,", 1);
    let material = read(&three).unwrap();
    // Superseded by 0135 slice 1 (2026-11-19): written back as format 4.
    assert_eq!(
        write_record(&material),
        three.replacen("\"format_version\": 3,", "\"format_version\": 4,", 1),
        "byte for byte"
    );
    let set = material.lasers[0]
        .sets
        .get(&curvyo_laser_core::settings::SetKind::Image)
        .unwrap();
    assert_eq!(set.numbers.interval.unwrap().hundredths(), 8);
    assert_eq!(
        read(&fixture("image_set_no_interval.json")),
        Err(RecordFileError::Damaged(
            "An Image set has no interval.".to_owned()
        ))
    );
}
