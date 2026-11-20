//! Material record format 4 (`specs/0135-image-optimisation` criteria 51, 52; ADR 0072 §6): an
//! Image test's recipe keeps its Picture source and a Strength axis; the Image set stores nothing
//! new; a format-3 record holds neither and is written back as format 4.

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use curvyo_laser_core::grids::Param;
use curvyo_laser_core::grids::{
    PictureSource, Recipe, RecipeDefaults, Side, TestId, TestRecipe, derived_operations,
};
use curvyo_laser_core::library::{
    RECORD_FORMAT_VERSION, RecordFileError, read_record, write_record,
};
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

#[test]
fn the_format_is_4() {
    assert_eq!(RECORD_FORMAT_VERSION, 4);
}

/// Criterion 51: the picture test reads with its source and its Strength axis, and is written
/// back byte for byte; each cell's Strength comes from the axis, its numbers from the others.
#[test]
fn a_picture_test_keeps_its_source_and_strength_axis_byte_for_byte() {
    let text = fixture("format4_picture_test.json");
    let material = read(&text).unwrap();
    assert_eq!(write_record(&material), text);
    let picture_test = material.test(TestId::parse("7ST4R").unwrap()).unwrap();
    let recipe = &picture_test.recipe;
    assert_eq!(recipe.picture(), PictureSource::Selected);
    assert_eq!(recipe.dither(), Some(Dither::Jarvis));
    assert_eq!(recipe.axis(Side::Columns).param(), Param::Strength);
    let strengths: Vec<Option<u8>> = (0..5).map(|c| recipe.cell_strength(c, 0)).collect();
    assert_eq!(
        strengths,
        [Some(0), Some(25), Some(50), Some(75), Some(100)]
    );
    let numbers = recipe.cell_values(4, 2);
    assert_eq!(numbers.settings.power.tenths(), 800);
    assert_eq!(numbers.interval.unwrap().hundredths(), 10);
    let ramp = material.test(TestId::parse("9GR4D").unwrap()).unwrap();
    assert_eq!(ramp.recipe.picture(), PictureSource::Ramp);
    assert_eq!(ramp.recipe.cell_strength(0, 0), None);
    // Criterion 52: a cell writes only power, speed, passes and interval into the Image set.
    let ops = derived_operations(
        &Recipe {
            id: picture_test.id,
            test: recipe.clone(),
            material: None,
        },
        None,
    );
    assert_eq!(ops.len(), 1 + 15);
}

#[test]
fn a_format_3_record_reads_as_is_and_is_written_as_format_4() {
    let text = fixture("format3_image_test.json");
    let material = read(&text).unwrap();
    let written = write_record(&material);
    assert_eq!(
        written,
        text.replacen("\"format_version\": 3,", "\"format_version\": 4,", 1)
    );
    assert_eq!(read(&written).unwrap(), material);
}

/// ADR 0072 §6: a picture test in a format-3 record, a Strength axis on the Ramp, `selected` on
/// another kind and an unknown source are damage.
#[test]
fn a_picture_test_out_of_its_format_or_rules_is_damage() {
    let text = fixture("format4_picture_test.json");
    let damaged = |text: &str| {
        assert_eq!(
            read(text),
            Err(RecordFileError::Damaged(
                "The recipe of test 7ST4R is not valid.".to_owned()
            ))
        );
    };
    damaged(&text.replacen("\"format_version\": 4,", "\"format_version\": 3,", 1));
    damaged(&text.replacen("\"picture\": \"selected\"", "\"picture\": \"ramp\"", 1));
    damaged(&text.replacen("\"picture\": \"selected\"", "\"picture\": \"photo\"", 1));
    let strength_on_ramp = text.replacen(",\n        \"picture\": \"selected\"", "", 1);
    damaged(&strength_on_ramp);
    let over = text.replacen("\"max\": 100,", "\"max\": 101,", 1);
    damaged(&over);
}

/// Criterion 51: the Strength axis is offered only with the selected picture; back to the Ramp
/// it gives its axis to a free parameter; another kind is always the Ramp.
#[test]
fn the_strength_axis_follows_the_picture_source() {
    let defaults = RecipeDefaults {
        max_feed: curvyo_laser_core::FeedRate::new(6000).unwrap(),
        set: None,
    };
    let image = TestRecipe::new(&defaults).with_kind(SetKind::Image, &defaults);
    let refused = image.with_axis_param(Side::Columns, Param::Strength, &defaults);
    assert_eq!(refused, image, "no Strength axis on the Ramp");
    let picture = image.with_picture(PictureSource::Selected, &defaults);
    let axis = picture.with_axis_param(Side::Columns, Param::Strength, &defaults);
    assert_eq!(axis.axis(Side::Columns).param(), Param::Strength);
    assert_eq!(
        (
            axis.axis(Side::Columns).min().steps(),
            axis.axis(Side::Columns).max().steps(),
            axis.axis(Side::Columns).count()
        ),
        (0, 100, 5),
        "default 0, 100, 5"
    );
    assert!(axis.fixed(Param::Strength).is_none(), "never shared");
    let ramp = axis.with_picture(PictureSource::Ramp, &defaults);
    assert_ne!(ramp.axis(Side::Columns).param(), Param::Strength);
    assert_eq!(ramp.picture(), PictureSource::Ramp);
    let engrave = axis.with_kind(SetKind::Engrave, &defaults);
    assert_eq!(engrave.picture(), PictureSource::Ramp);
    assert_ne!(engrave.axis(Side::Columns).param(), Param::Strength);
    let cut = TestRecipe::new(&defaults).with_picture(PictureSource::Selected, &defaults);
    assert_eq!(cut.picture(), PictureSource::Ramp, "only an Image grid");
}
