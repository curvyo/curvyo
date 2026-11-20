//! The laser extension reads the format 50 goldens of `curvyo-document-core/tests/fixtures/`
//! (`specs/0123-raster-engraving` criteria 13, 41, 44; ADR 0069 §7, §8, §13, §14): every Image
//! setting of the Image operation and its link to the material's Image set, the Engrave beside
//! it, the Image grid's Dither and cells, and one damage per reason.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use curvyo_document_core::{Document, OperationSnapshot, unpack};
use curvyo_laser_core::grids::{Recipe, cell_style};
use curvyo_laser_core::operations::{KindRead, OperationKind};
use curvyo_laser_core::settings::{
    Dither, ImageSettings, OptionsDamage, SettingsRead, read_options, read_settings,
};

fn golden(name: &str) -> Document {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-document-core/tests/fixtures")
        .join(name);
    unpack(5, &std::fs::read(path).unwrap()).unwrap()
}

fn operations(d: &Document) -> Vec<OperationSnapshot> {
    d.operations(d.manufacturings()[0].id).unwrap()
}

fn kind(op: &OperationSnapshot) -> OperationKind {
    KindRead::of(op.kind.as_deref()).known().unwrap()
}

#[test]
fn the_golden_reads_every_image_setting() {
    let d = golden("raster_engraving_v50.curvyo");
    let ops = operations(&d);
    let engrave = &ops[0];
    assert_eq!(kind(engrave), OperationKind::Image);
    let options = read_options(engrave.options.as_ref(), kind(engrave)).unwrap();
    assert_eq!(options.scan_angle.tenths(), 300);
    assert_eq!(
        options.image,
        ImageSettings {
            dither: Dither::Grey,
            threshold_pct: 60,
            brightness_pct: 10,
            contrast_pct: -20,
            gamma_hundredths: 180,
            invert: true,
            lowest_power_tenths: 55,
            ..ImageSettings::DEFAULT
        }
    );
    assert_eq!(
        &options.to_value(kind(engrave)),
        engrave.options.as_ref().unwrap(),
        "written back as stored"
    );
    let SettingsRead::Stored(stored) = read_settings(engrave.settings.as_ref(), kind(engrave))
    else {
        panic!("stored settings")
    };
    assert_eq!(stored.material.as_ref().unwrap().name, "Slate tile");
    assert_eq!(
        stored.interval.unwrap().hundredths(),
        8,
        "the Image set's interval"
    );
    let hatch = &ops[1];
    assert_eq!(kind(hatch), OperationKind::Engrave);
    let options = read_options(hatch.options.as_ref(), kind(hatch)).unwrap();
    assert_eq!(
        options.image,
        ImageSettings::DEFAULT,
        "no Image key on an Engrave"
    );
}

#[test]
fn the_image_grid_reads_its_dither_and_cells() {
    let d = golden("raster_engraving_v50.curvyo");
    let ops = operations(&d);
    let group = ops
        .iter()
        .find_map(|op| d.test_grid_of_derived_row(op.id))
        .unwrap();
    let grid = d.test_grid(group).unwrap();
    let recipe = Recipe::from_value(&grid.recipe).unwrap();
    assert_eq!(recipe.test.dither(), Some(Dither::Ordered));
    let style = cell_style(&recipe.test);
    for cell in &grid.cells {
        assert_eq!(
            d.object(cell.id).unwrap().style().fill.painted_gradient(),
            style.fill.painted_gradient(),
            "every cell is the Gradient rectangle the recipe makes"
        );
    }
    for op in ops.iter().filter(|op| grid.derived[1..].contains(&op.id)) {
        assert_eq!(kind(op), OperationKind::Image, "{}", op.name);
        let options = read_options(op.options.as_ref(), kind(op)).unwrap();
        assert_eq!(options.image.dither, Dither::Ordered, "{}", op.name);
    }
}

#[test]
fn each_damage_reads_as_damaged_settings() {
    let d = golden("raster_engraving_damaged_v50.curvyo");
    let damage: Vec<_> = operations(&d)
        .iter()
        .map(|op| read_options(op.options.as_ref(), kind(op)).unwrap_err())
        .collect();
    assert_eq!(
        damage,
        [
            OptionsDamage::BadValue,
            OptionsDamage::BadValue,
            OptionsDamage::KeyOnKind,
            OptionsDamage::KeyOnKind,
            OptionsDamage::KeyOnKind
        ]
    );
}
