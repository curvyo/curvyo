//! A tiling exports exactly as the same group of clones without the tiling (`specs/0069-tiling`
//! criteria 33 and 42, ADR 0030 §9): the SVG of a tiling equals the SVG of the plain group that
//! Release leaves, and it holds no tiling data.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{Angle, Document, Length, Percent, Point, RectBounds};
use curvyo_text_core::FaceStore;
use curvyo_ui_core::tiling::{TileButton, cell_box, default_recipe, tile_button, tiling_request};
use curvyo_ui_core::{
    AnchorIdMinter, ExportScope, ImportFaces, SvgExportOptions, Units, svg_export,
};

fn export(document: &Document) -> String {
    let store = FaceStore::new();
    let fonts = ImportFaces {
        faces: &[],
        store: &store,
        substitute: None,
    };
    let laid = document.read_objects();
    let outcome = svg_export(
        document,
        &laid,
        &fonts,
        &ExportScope {
            selection: &[],
            active_page: 0,
        },
        SvgExportOptions::default(),
    )
    .unwrap();
    curvyo_svg_core::write_svg(&outcome.svg)
}

#[test]
fn a_tiling_exports_as_the_released_group_of_clones() {
    let d = Document::new(1);
    let rect = d.create_rect(RectBounds {
        origin: Point::new(10.0, 10.0),
        width: Length::from_mm(12.0),
        height: Length::from_mm(6.0),
    });
    let TileButton::Tile(sources) = tile_button(&d, &Units::default(), &[rect]) else {
        panic!("offered")
    };
    let read = d.read_objects();
    let cell_box = cell_box(&d, &read, &sources).unwrap();
    let mut recipe = default_recipe(cell_box, 42);
    recipe.variance.rotation.random = Angle::from_radians(0.3);
    recipe.variance.hue.random = Angle::from_radians(1.0);
    recipe.variance.opacity.random = Percent::new(20.0);
    let mut minter = AnchorIdMinter::new(1);
    let request = tiling_request(&d, &read, &mut minter, (None, &sources), &recipe, cell_box);
    let mut mint = || minter.mint();
    let tiling = d.apply_tiling(&request, &mut mint).unwrap().tiling;
    let with_tiling = export(&d);
    assert!(
        !with_tiling.contains("tiling"),
        "no tiling data in the file"
    );
    d.release_tiling(tiling).unwrap();
    assert_eq!(export(&d), with_tiling);
}
