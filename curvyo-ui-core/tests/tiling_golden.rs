//! The golden layouts of tiling (`specs/0069-tiling` criteria 6 to 17, 21 and 41): fixed recipes
//! and seeds with the expected cells in `tests/fixtures/tiling/layouts.json`, embedded with
//! `include_str!` so the same comparison runs natively and in Node on `wasm32-unknown-unknown`:
//!
//! ```text
//! cargo test --target wasm32-unknown-unknown -p curvyo-ui-core --test tiling_golden
//! ```
//!
//! Tolerance 1e-9 mm for translations and 1e-12 for the linear part (`adrs.md` decision 4): only
//! `sin` and `cos` may differ in the last bit between platforms. Rewrite the fixture after an
//! intended change with `TILING_WRITE_GOLDEN=1 cargo test -p curvyo-ui-core --test tiling_golden`.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use curvyo_document_core::{
    Angle, CellKey, Length, OffsetLines, Percent, Point, SpacingMode, TilingCell, TilingLayout,
    TilingRecipe,
};
use curvyo_ui_core::tiling::{CellBox, cell_variance, default_recipe, tiling_layout};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const FIXTURE: &str = include_str!("../../tests/fixtures/tiling/layouts.json");
const MM_TOLERANCE: f64 = 1e-9;
const LINEAR_TOLERANCE: f64 = 1e-12;

fn deg(d: f64) -> Angle {
    Angle::from_radians(d.to_radians())
}

fn cell_box() -> CellBox {
    CellBox {
        min: Point::new(10.0, 20.0),
        max: Point::new(30.0, 30.0),
    }
}

fn base() -> TilingRecipe {
    let mut r = default_recipe(cell_box(), 42);
    r.grid.columns = 3;
    r.grid.rows = 3;
    r.grid.spacing_x = Length::from_mm(0.0);
    r.grid.spacing_y = Length::from_mm(0.0);
    r.keep_source = false;
    r
}

fn all_channels(r: &mut TilingRecipe) {
    let v = &mut r.variance;
    v.rotation.random = deg(15.0);
    v.scale_x.random = Percent::new(20.0);
    v.offset_x.random = Length::from_mm(2.0);
    v.offset_y.random = Length::from_mm(1.5);
    v.opacity.random = Percent::new(30.0);
    v.hue.random = deg(40.0);
}

fn cases() -> Vec<(&'static str, TilingRecipe)> {
    let grid_gap_0 = base();
    let mut mirror = base();
    mirror.grid.mirror_rows = true;
    mirror.grid.mirror_columns = true;
    let mut offset_rows = base();
    offset_rows.grid.offset = OffsetLines::Rows;
    let mut pitch = base();
    pitch.grid.spacing_mode = SpacingMode::Pitch;
    pitch.grid.spacing_x = Length::from_mm(25.0);
    pitch.grid.spacing_y = Length::from_mm(12.0);
    let mut circle_default = base();
    circle_default.layout = TilingLayout::Circle;
    let mut circle_half = circle_default;
    circle_half.circle.count = 5;
    circle_half.circle.sweep = deg(180.0);
    circle_half.circle.face_centre = true;
    let mut random = base();
    all_channels(&mut random);
    let mut stepped = base();
    stepped.variance.rotation.per_row = deg(10.0);
    stepped.variance.rotation.per_column = deg(-5.0);
    stepped.variance.scale_x.per_column = Percent::new(10.0);
    stepped.variance.offset_y.per_row = Length::from_mm(1.0);
    let mut mixed = stepped;
    all_channels(&mut mixed);
    vec![
        ("grid_3x3_gap_0", grid_gap_0),
        ("grid_mirror_odd_rows_and_columns", mirror),
        ("grid_offset_rows", offset_rows),
        ("grid_pitch", pitch),
        ("circle_default", circle_default),
        ("circle_sweep_180_face_centre", circle_half),
        ("random_seed_42_all_channels", random),
        ("stepped_per_row_and_column", stepped),
        ("mixed_random_and_stepped", mixed),
    ]
}

fn cell_json(cell: &TilingCell) -> serde_json::Value {
    let m = cell.affine.to_array();
    serde_json::json!([
        cell.key.row,
        cell.key.column,
        m[0],
        m[1],
        m[2],
        m[3],
        m[4],
        m[5],
        cell.opacity.get(),
        cell.hue.as_radians()
    ])
}

#[test]
fn the_layouts_match_their_golden_cells() {
    let computed: Vec<(&str, Vec<TilingCell>)> = cases()
        .into_iter()
        .map(|(name, recipe)| (name, tiling_layout(cell_box(), &recipe)))
        .collect();
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var_os("TILING_WRITE_GOLDEN").is_some() {
        let mut text = String::from("{\n");
        for (index, (name, cells)) in computed.iter().enumerate() {
            text.push_str("  \"");
            text.push_str(name);
            text.push_str("\": [\n");
            let rows: Vec<String> = cells
                .iter()
                .map(|c| format!("    {}", cell_json(c)))
                .collect();
            text.push_str(&rows.join(",\n"));
            text.push_str(if index + 1 == computed.len() {
                "\n  ]\n"
            } else {
                "\n  ],\n"
            });
        }
        text.push_str("}\n");
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/fixtures/tiling/layouts.json"
        );
        std::fs::write(path, text).unwrap();
        return;
    }
    let golden: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    for (name, cells) in &computed {
        let expected = golden[*name].as_array().expect(name);
        assert_eq!(expected.len(), cells.len(), "{name}");
        for (want, cell) in expected.iter().zip(cells) {
            let got = cell_json(cell);
            let want: Vec<f64> = want
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect();
            let got: Vec<f64> = got
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect();
            for (i, (w, g)) in want.iter().zip(&got).enumerate() {
                let tolerance = if (2..6).contains(&i) {
                    LINEAR_TOLERANCE
                } else {
                    MM_TOLERANCE
                };
                assert!(
                    (w - g).abs() <= tolerance,
                    "{name} cell {got:?} entry {i}: {w} vs {g}"
                );
            }
        }
    }
}

#[test]
fn a_3x3_grid_with_gap_0_puts_the_boxes_edge_to_edge() {
    let cells = tiling_layout(cell_box(), &base());
    assert_eq!(cells.len(), 9);
    let at = |row, column| {
        let cell = cells
            .iter()
            .find(|c| c.key == CellKey { row, column })
            .unwrap();
        (cell.affine.e, cell.affine.f)
    };
    assert_eq!(at(0, 0), (0.0, 0.0));
    assert_eq!(at(0, 2), (40.0, 0.0));
    assert_eq!(at(2, 1), (20.0, 20.0));
}

#[test]
fn mirror_flips_odd_rows_and_columns_about_their_own_centre() {
    let mut r = base();
    r.grid.mirror_rows = true;
    r.grid.mirror_columns = true;
    let cells = tiling_layout(cell_box(), &r);
    let odd = cells
        .iter()
        .find(|c| c.key == CellKey { row: 1, column: 1 })
        .unwrap();
    assert_eq!((odd.affine.a, odd.affine.d), (-1.0, -1.0));
    // The box centre (20, 25) lands on the centre of cell (1, 1), (40, 35).
    let centre = odd.affine.apply_point(Point::new(20.0, 25.0));
    assert!((centre.x - 40.0).abs() < MM_TOLERANCE && (centre.y - 35.0).abs() < MM_TOLERANCE);
}

#[test]
fn offset_rows_shift_odd_rows_by_half_the_pitch() {
    let mut r = base();
    r.grid.offset = OffsetLines::Rows;
    let cells = tiling_layout(cell_box(), &r);
    let first_of_row_1 = cells
        .iter()
        .find(|c| c.key == CellKey { row: 1, column: 0 })
        .unwrap();
    assert!((first_of_row_1.affine.e - 10.0).abs() < MM_TOLERANCE);
}

#[test]
fn the_default_circle_starts_at_the_source_and_runs_counterclockwise() {
    let mut r = base();
    r.layout = TilingLayout::Circle;
    r.keep_source = true;
    let cells = tiling_layout(cell_box(), &r);
    assert_eq!(cells.len(), 11);
    // Radius 20 (half of 20 plus 10), centre (0, 25); instance 3 sits at 90°, straight above.
    let third = cells.iter().find(|c| c.key.column == 3).unwrap();
    let centre = third.affine.apply_point(Point::new(20.0, 25.0));
    assert!((centre.x - 0.0).abs() < MM_TOLERANCE && (centre.y - 5.0).abs() < MM_TOLERANCE);
}

#[test]
fn a_half_circle_puts_first_and_last_on_the_ends_and_faces_the_centre() {
    let mut r = base();
    r.layout = TilingLayout::Circle;
    r.circle.count = 5;
    r.circle.sweep = deg(180.0);
    r.circle.face_centre = true;
    let cells = tiling_layout(cell_box(), &r);
    let last = cells.last().unwrap();
    let centre = last.affine.apply_point(Point::new(20.0, 25.0));
    assert!((centre.x + 20.0).abs() < MM_TOLERANCE && (centre.y - 25.0).abs() < MM_TOLERANCE);
    // Turned by 180° about its own centre.
    assert!((last.affine.a + 1.0).abs() < LINEAR_TOLERANCE);
}

#[test]
fn adding_a_column_or_changing_one_range_leaves_the_other_values_untouched() {
    let mut r = base();
    all_channels(&mut r);
    let before = tiling_layout(cell_box(), &r);
    let mut wider = r;
    wider.grid.columns = 4;
    let after = tiling_layout(cell_box(), &wider);
    for cell in &before {
        let same = after.iter().find(|c| c.key == cell.key).unwrap();
        assert_eq!(cell, same);
    }
    let mut other = r;
    other.variance.hue.random = deg(90.0);
    for key in [CellKey { row: 1, column: 2 }, CellKey { row: 2, column: 0 }] {
        let a = cell_variance(&r.variance, r.seed, key);
        let b = cell_variance(&other.variance, other.seed, key);
        assert_eq!(
            (a.rotation, a.scale_x, a.offset, a.opacity),
            (b.rotation, b.scale_x, b.offset, b.opacity)
        );
        assert_ne!(a.hue, b.hue);
    }
}

#[test]
fn the_generator_gives_the_reference_sequences_on_this_platform() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/tiling/pcg32.json")).unwrap();
    for sequence in fixture["sequences"].as_array().unwrap() {
        let mut rng = curvyo_ui_core::tiling::Pcg32::new(sequence["seed"].as_u64().unwrap());
        for expected in sequence["outputs"].as_array().unwrap() {
            assert_eq!(u64::from(rng.next_u32()), expected.as_u64().unwrap());
        }
    }
}
