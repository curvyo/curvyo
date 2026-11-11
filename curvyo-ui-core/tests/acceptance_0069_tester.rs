//! Tester's acceptance tests for `specs/0069-tiling`, written from the specification before the
//! implementation was read: the grid and circle layouts (criteria 6 to 13), variance and the
//! seeded generator (criteria 15 to 21), the kept source (22), the counts (36), the Tile button
//! (3, 5, 26), the history names (37) and the export of a tiling (33, 42).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use curvyo_document_core::{
    Affine, Angle, CellKey, Document, Length, OffsetLines, Percent, Point, RecipeError, RectBounds,
    SpacingMode, TILING_MAX_COUNT, TILING_WARN_COUNT, TilingCell, TilingLayout, TilingRecipe,
    TilingResult, Variance, Vec2,
};
use curvyo_text_core::FaceStore;
use curvyo_ui_core::tiling::{
    CellBox, CountStatus, Pcg32, TileButton, cell_box, cell_variance, converted_spacing,
    count_status, default_recipe, tile_button, tiling_layout, tiling_request,
};
use curvyo_ui_core::{
    AnchorIdMinter, ExportScope, ImportFaces, SvgExportOptions, Units, operation_name, svg_export,
};

const MM: f64 = 1e-9;
const LIN: f64 = 1e-12;

// ---------------------------------------------------------------------------------------------
// An independent PCG32 (pcg_oneseq_64_xsh_rr_32), written from O'Neill's reference algorithm.
// ---------------------------------------------------------------------------------------------

const MULT: u64 = 6_364_136_223_846_793_005;
const INC: u64 = 1_442_695_040_888_963_407;

struct RefPcg(u64);

impl RefPcg {
    fn seeded(seed: u64) -> Self {
        let mut r = Self(0);
        r.step();
        r.0 = r.0.wrapping_add(seed);
        r.step();
        r
    }
    fn step(&mut self) {
        self.0 = self.0.wrapping_mul(MULT).wrapping_add(INC);
    }
    fn next(&mut self) -> u32 {
        let old = self.0;
        self.step();
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }
}

/// The eight unit draws of a cell per criterion 21.
fn ref_draws(seed: u32, row: u32, column: u32) -> [f64; 8] {
    let mut g = Pcg32::new(u64::from(seed));
    let index = (u64::from(row) << 32) | u64::from(column);
    g.advance(index.wrapping_mul(8));
    let mut out = [0.0; 8];
    for slot in &mut out {
        *slot = f64::from(g.next_u32()) / 4_294_967_296.0;
    }
    out
}

fn deg(d: f64) -> Angle {
    Angle::from_radians(d.to_radians())
}

fn bx() -> CellBox {
    CellBox {
        min: Point::new(10.0, 20.0),
        max: Point::new(30.0, 30.0),
    }
}

fn base(seed: u32) -> TilingRecipe {
    default_recipe(bx(), seed)
}

fn grid(columns: u32, rows: u32) -> TilingRecipe {
    let mut r = base(7);
    r.grid.columns = columns;
    r.grid.rows = rows;
    r.keep_source = false;
    r
}

fn circle(count: u32) -> TilingRecipe {
    let mut r = base(7);
    r.layout = TilingLayout::Circle;
    r.circle.count = count;
    r.keep_source = false;
    r
}

fn cell(cells: &[TilingCell], row: u32, column: u32) -> TilingCell {
    *cells
        .iter()
        .find(|c| c.key == CellKey { row, column })
        .unwrap_or_else(|| panic!("cell {row},{column} missing"))
}

fn centre_of(c: &TilingCell) -> Point {
    c.affine.apply_point(bx().centre())
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

fn assert_point(p: Point, x: f64, y: f64, what: &str) {
    assert!(
        close(p.x, x, MM) && close(p.y, y, MM),
        "{what}: got ({}, {}), want ({x}, {y})",
        p.x,
        p.y
    );
}

/// The linear part of `a` as (a, b, c, d).
fn lin(a: Affine) -> [f64; 4] {
    [a.a, a.b, a.c, a.d]
}

fn assert_lin(a: Affine, want: [f64; 4], what: &str) {
    let got = lin(a);
    for i in 0..4 {
        assert!(
            close(got[i], want[i], LIN),
            "{what}: got {got:?}, want {want:?}"
        );
    }
}

/// A rotation counterclockwise on screen (y down) by `t` radians, as (a, b, c, d).
fn screen_ccw(t: f64) -> [f64; 4] {
    [t.cos(), -t.sin(), t.sin(), t.cos()]
}

/// The screen-counterclockwise angle of the linear part (the image of the x axis).
fn angle_of(a: Affine) -> f64 {
    (-a.b).atan2(a.a)
}

// ---------------------------------------------------------------------------------------------
// B. Grid layout (criteria 6 to 10)
// ---------------------------------------------------------------------------------------------

#[test]
fn c06_defaults_are_5_by_4_with_5mm_gaps_and_cell_00_at_the_source() {
    let r = base(1);
    assert_eq!(r.layout, TilingLayout::Grid);
    assert_eq!((r.grid.columns, r.grid.rows), (5, 4));
    assert_eq!(r.grid.spacing_x.as_mm(), 5.0);
    assert_eq!(r.grid.spacing_y.as_mm(), 5.0);
    assert_eq!(r.grid.spacing_mode, SpacingMode::Gap);
    assert!(!r.grid.mirror_rows && !r.grid.mirror_columns);
    assert_eq!(r.grid.offset, OffsetLines::None);
    assert_eq!(r.grid.shift.get(), 50.0);
    assert_eq!(r.variance, Variance::default());
    assert!(!r.variance.varies_style());
    assert_eq!(
        r.result,
        TilingResult::Clones,
        "Result as defaults to Clones"
    );
    assert!(r.keep_source, "Keep source in place defaults to on");
    assert_eq!(r.seed, 1);
    // 20 instances, the kept source standing for (0, 0): 19 generated.
    let cells = tiling_layout(bx(), &r);
    assert_eq!(cells.len(), 19);
    assert!(cells.iter().all(|c| c.key != CellKey::ORIGIN));
    // Columns grow to the right, rows downward on screen; box 20 x 10, gap 5.
    assert_point(centre_of(&cell(&cells, 0, 1)), 45.0, 25.0, "(0,1)");
    assert_point(centre_of(&cell(&cells, 1, 0)), 20.0, 40.0, "(1,0)");
    assert_point(centre_of(&cell(&cells, 3, 4)), 120.0, 70.0, "(3,4)");
    for c in &cells {
        assert_lin(
            c.affine,
            [1.0, 0.0, 0.0, 1.0],
            "no variance: pure translation",
        );
    }
}

#[test]
fn c06_without_kept_source_cell_00_is_generated_at_the_source() {
    let cells = tiling_layout(bx(), &grid(5, 4));
    assert_eq!(cells.len(), 20);
    let origin = cell(&cells, 0, 0);
    assert_lin(origin.affine, [1.0, 0.0, 0.0, 1.0], "origin");
    assert_point(centre_of(&origin), 20.0, 25.0, "origin at source");
}

#[test]
fn c06_degenerate_one_by_one_grid() {
    let mut r = grid(1, 1);
    assert_eq!(tiling_layout(bx(), &r).len(), 1);
    r.keep_source = true;
    assert!(tiling_layout(bx(), &r).is_empty(), "the source alone");
    assert!(r.check().is_ok());
}

#[test]
fn c06_zero_rows_or_columns_are_refused() {
    let mut r = grid(0, 3);
    assert_eq!(r.check(), Err(RecipeError::CountZero));
    r.grid.columns = 3;
    r.grid.rows = 0;
    assert_eq!(r.check(), Err(RecipeError::CountZero));
    let mut c = circle(0);
    c.layout = TilingLayout::Circle;
    assert_eq!(c.check(), Err(RecipeError::CountZero));
}

#[test]
fn c07_gap_zero_makes_boxes_touch_and_negative_gap_overlaps() {
    let mut r = grid(3, 3);
    r.grid.spacing_x = Length::from_mm(0.0);
    r.grid.spacing_y = Length::from_mm(0.0);
    let cells = tiling_layout(bx(), &r);
    // Right edge of (0,0) = 30 equals the left edge of (0,1).
    let left_of_next = cell(&cells, 0, 1).affine.apply_point(bx().min).x;
    assert!(close(left_of_next, 30.0, MM));
    let top_of_next = cell(&cells, 1, 0).affine.apply_point(bx().min).y;
    assert!(close(top_of_next, 30.0, MM));
    r.grid.spacing_x = Length::from_mm(-4.0);
    assert!(r.check().is_ok(), "negative gaps are allowed");
    let cells = tiling_layout(bx(), &r);
    let left = cell(&cells, 0, 1).affine.apply_point(bx().min).x;
    assert!(close(left, 26.0, MM), "overlap by 4: {left}");
}

#[test]
fn c07_gap_is_edge_to_edge_and_pitch_is_centre_to_centre() {
    let mut r = grid(3, 2);
    r.grid.spacing_x = Length::from_mm(3.0);
    r.grid.spacing_y = Length::from_mm(7.0);
    let cells = tiling_layout(bx(), &r);
    let next = cell(&cells, 0, 1).affine.apply_point(bx().min).x;
    assert!(close(next - 30.0, 3.0, MM), "X gap");
    let below = cell(&cells, 1, 0).affine.apply_point(bx().min).y;
    assert!(close(below - 30.0, 7.0, MM), "Y gap");

    r.grid.spacing_mode = SpacingMode::Pitch;
    r.grid.spacing_x = Length::from_mm(33.0);
    r.grid.spacing_y = Length::from_mm(12.5);
    let cells = tiling_layout(bx(), &r);
    let a = centre_of(&cell(&cells, 0, 0));
    let b = centre_of(&cell(&cells, 0, 2));
    let c = centre_of(&cell(&cells, 1, 0));
    assert!(close(b.x - a.x, 66.0, MM));
    assert!(close(c.y - a.y, 12.5, MM));
}

#[test]
fn c07_switching_gap_and_pitch_converts_so_the_layout_does_not_move() {
    let mut r = grid(4, 3);
    r.grid.spacing_x = Length::from_mm(2.5);
    r.grid.spacing_y = Length::from_mm(-1.0);
    let before = tiling_layout(bx(), &r);
    let (x, y) = converted_spacing(bx(), &r.grid, SpacingMode::Pitch);
    assert!(close(x.as_mm(), 22.5, MM) && close(y.as_mm(), 9.0, MM));
    let mut p = r;
    p.grid.spacing_mode = SpacingMode::Pitch;
    p.grid.spacing_x = x;
    p.grid.spacing_y = y;
    let after = tiling_layout(bx(), &p);
    for (a, b) in before.iter().zip(&after) {
        assert_eq!(a.key, b.key);
        assert!(close(a.affine.e, b.affine.e, MM) && close(a.affine.f, b.affine.f, MM));
    }
    let (gx, gy) = converted_spacing(bx(), &p.grid, SpacingMode::Gap);
    assert!(
        close(gx.as_mm(), 2.5, MM) && close(gy.as_mm(), -1.0, MM),
        "round trip"
    );
    // Same mode: unchanged.
    assert_eq!(
        converted_spacing(bx(), &r.grid, SpacingMode::Gap),
        (r.grid.spacing_x, r.grid.spacing_y)
    );
}

#[test]
fn c07_pitch_must_be_more_than_zero() {
    let mut r = grid(2, 2);
    r.grid.spacing_mode = SpacingMode::Pitch;
    for bad in [0.0, -5.0] {
        r.grid.spacing_x = Length::from_mm(bad);
        r.grid.spacing_y = Length::from_mm(10.0);
        assert_eq!(r.check(), Err(RecipeError::OutOfRange), "pitch x {bad}");
        r.grid.spacing_x = Length::from_mm(10.0);
        r.grid.spacing_y = Length::from_mm(bad);
        assert_eq!(r.check(), Err(RecipeError::OutOfRange), "pitch y {bad}");
    }
}

#[test]
fn c08_mirror_rows_flips_odd_rows_about_their_horizontal_centre_line() {
    let mut r = grid(3, 4);
    r.grid.mirror_rows = true;
    let cells = tiling_layout(bx(), &r);
    for c in &cells {
        let want = if c.key.row % 2 == 1 {
            [1.0, 0.0, 0.0, -1.0]
        } else {
            [1.0, 0.0, 0.0, 1.0]
        };
        assert_lin(c.affine, want, "mirror rows");
        // About its own centre: the centre lands on the unmirrored cell centre.
        let p = centre_of(c);
        assert_point(
            p,
            20.0 + 25.0 * f64::from(c.key.column),
            25.0 + 15.0 * f64::from(c.key.row),
            "mirrored centre",
        );
    }
}

#[test]
fn c08_mirror_columns_and_both() {
    let mut r = grid(4, 4);
    r.grid.mirror_columns = true;
    for c in tiling_layout(bx(), &r) {
        let sx = if c.key.column % 2 == 1 { -1.0 } else { 1.0 };
        assert_lin(c.affine, [sx, 0.0, 0.0, 1.0], "mirror columns");
    }
    r.grid.mirror_rows = true;
    for c in tiling_layout(bx(), &r) {
        let sx = if c.key.column % 2 == 1 { -1.0 } else { 1.0 };
        let sy = if c.key.row % 2 == 1 { -1.0 } else { 1.0 };
        assert_lin(c.affine, [sx, 0.0, 0.0, sy], "both");
        assert_point(
            centre_of(&c),
            20.0 + 25.0 * f64::from(c.key.column),
            25.0 + 15.0 * f64::from(c.key.row),
            "both: centre",
        );
    }
}

#[test]
fn c09_offset_rows_shifts_odd_rows_right_and_offset_columns_odd_columns_down() {
    let mut r = grid(3, 4);
    r.grid.offset = OffsetLines::Rows;
    r.grid.shift = Percent::new(40.0);
    let cells = tiling_layout(bx(), &r);
    // Pitch x = 25: 40 % is 10.
    for c in &cells {
        let dx = if c.key.row % 2 == 1 { 10.0 } else { 0.0 };
        assert_point(
            centre_of(c),
            20.0 + 25.0 * f64::from(c.key.column) + dx,
            25.0 + 15.0 * f64::from(c.key.row),
            "offset rows",
        );
    }
    r.grid.offset = OffsetLines::Columns;
    let cells = tiling_layout(bx(), &r);
    // Pitch y = 15: 40 % is 6.
    for c in &cells {
        let dy = if c.key.column % 2 == 1 { 6.0 } else { 0.0 };
        assert_point(
            centre_of(c),
            20.0 + 25.0 * f64::from(c.key.column),
            25.0 + 15.0 * f64::from(c.key.row) + dy,
            "offset columns",
        );
    }
    assert_point(
        centre_of(&cell(&cells, 0, 0)),
        20.0,
        25.0,
        "row/col 0 at source",
    );
}

#[test]
fn c09_brick_pattern_with_gap_zero_and_shift_50() {
    let mut r = grid(4, 2);
    r.grid.spacing_x = Length::from_mm(0.0);
    r.grid.spacing_y = Length::from_mm(0.0);
    r.grid.offset = OffsetLines::Rows;
    let cells = tiling_layout(bx(), &r);
    // Row 1 starts half a brick (10 mm) right of row 0 and sits directly below (y + 10).
    let p = cell(&cells, 1, 0).affine.apply_point(bx().min);
    assert_point(p, 20.0, 30.0, "brick");
}

#[test]
fn c09_shift_is_0_to_100() {
    let mut r = grid(2, 2);
    r.grid.offset = OffsetLines::Rows;
    for ok in [0.0, 100.0] {
        r.grid.shift = Percent::new(ok);
        assert!(r.check().is_ok(), "{ok}");
    }
    for bad in [-0.1, 100.1] {
        r.grid.shift = Percent::new(bad);
        assert_eq!(r.check(), Err(RecipeError::OutOfRange), "{bad}");
    }
}

#[test]
fn c10_mirror_flips_about_the_offset_centre() {
    let mut r = grid(2, 2);
    r.grid.offset = OffsetLines::Rows;
    r.grid.mirror_rows = true;
    r.grid.mirror_columns = true;
    let cells = tiling_layout(bx(), &r);
    let c = cell(&cells, 1, 1);
    assert_lin(c.affine, [-1.0, 0.0, 0.0, -1.0], "both flips");
    // Centre: column 1 (+25), row 1 (+15), offset half of 25.
    assert_point(centre_of(&c), 20.0 + 25.0 + 12.5, 40.0, "offset centre");
}

// ---------------------------------------------------------------------------------------------
// C. Circle layout (criteria 11 to 13)
// ---------------------------------------------------------------------------------------------

#[test]
fn c11_circle_defaults_and_the_source_is_the_instance_at_the_start_angle() {
    let r = base(1);
    let c = r.circle;
    assert_eq!(c.count, 12);
    // Half the larger side (20 / 2) plus 10.
    assert!(close(c.radius.as_mm(), 20.0, MM));
    assert_eq!(c.start.as_radians(), 0.0);
    assert!(close(c.sweep.as_radians(), TAU, LIN));
    assert!(!c.face_centre);
    let mut all = r;
    all.layout = TilingLayout::Circle;
    all.keep_source = false;
    let cells = tiling_layout(bx(), &all);
    assert_eq!(cells.len(), 12);
    assert_point(
        centre_of(&cell(&cells, 0, 0)),
        20.0,
        25.0,
        "index 0 is the source",
    );
    // Default centre: box centre (20,25) shifted left by 20 = (0, 25).
    for k in 0..12 {
        let p = centre_of(&cell(&cells, 0, k));
        let d = ((p.x - 0.0).powi(2) + (p.y - 25.0).powi(2)).sqrt();
        assert!(close(d, 20.0, 1e-9), "on the circle: {k}");
    }
}

#[test]
fn c11_angles_run_counterclockwise_on_screen_and_negative_sweep_clockwise() {
    let cells = tiling_layout(bx(), &circle(4));
    // 90 degrees points up on screen (y smaller).
    assert_point(centre_of(&cell(&cells, 0, 1)), 0.0, 5.0, "90 deg");
    assert_point(centre_of(&cell(&cells, 0, 2)), -20.0, 25.0, "180 deg");
    assert_point(centre_of(&cell(&cells, 0, 3)), 0.0, 45.0, "270 deg");
    let mut cw = circle(4);
    cw.circle.sweep = Angle::from_radians(-TAU);
    let cells = tiling_layout(bx(), &cw);
    assert_point(centre_of(&cell(&cells, 0, 1)), 0.0, 45.0, "clockwise");
}

#[test]
fn c11_start_angle_and_centre_relative_to_the_cell_box() {
    let mut r = circle(1);
    r.circle.start = deg(90.0);
    r.circle.centre = Vec2::new(5.0, -3.0);
    r.circle.radius = Length::from_mm(10.0);
    let cells = tiling_layout(bx(), &r);
    assert_point(centre_of(&cells[0]), 25.0, 12.0, "start 90 around (25,22)");
}

#[test]
fn c11_ranges_radius_and_sweep() {
    let mut r = circle(5);
    r.circle.radius = Length::from_mm(0.0);
    assert_eq!(r.check(), Err(RecipeError::OutOfRange), "radius 0");
    r.circle.radius = Length::from_mm(-1.0);
    assert_eq!(r.check(), Err(RecipeError::OutOfRange), "radius < 0");
    r.circle.radius = Length::from_mm(1.0);
    for ok in [-360.0, 360.0, 0.5, -0.5] {
        r.circle.sweep = deg(ok);
        assert!(r.check().is_ok(), "sweep {ok}");
    }
    for bad in [360.5, -361.0] {
        r.circle.sweep = deg(bad);
        assert_eq!(r.check(), Err(RecipeError::OutOfRange), "sweep {bad}");
    }
}

#[test]
fn c12_full_turn_divides_by_count_and_an_arc_by_count_minus_one() {
    let mut r = circle(6);
    r.circle.radius = Length::from_mm(10.0);
    r.circle.centre = Vec2::new(-10.0, 0.0);
    r.circle.face_centre = true;
    let cells = tiling_layout(bx(), &r);
    for k in 0..6 {
        let t = f64::from(k) * TAU / 6.0;
        let a = angle_of(cell(&cells, 0, k).affine);
        let diff = (a - t).rem_euclid(TAU);
        assert!(
            diff < 1e-12 || TAU - diff < 1e-12,
            "full turn {k}: {a} vs {t}"
        );
    }
    // The 360 case puts no instance on the start twice.
    let last = centre_of(&cell(&cells, 0, 5));
    assert!(!close(last.y, 25.0, 1e-6) || !close(last.x, 20.0, 1e-6));

    r.circle.sweep = deg(180.0);
    let cells = tiling_layout(bx(), &r);
    // Box centre (20,25), centre (10,25): first at 0, last at 180 degrees.
    assert_point(centre_of(&cell(&cells, 0, 0)), 20.0, 25.0, "arc start");
    assert_point(centre_of(&cell(&cells, 0, 5)), 0.0, 25.0, "arc end");
    let mid = angle_of(cell(&cells, 0, 1).affine);
    assert!(close(mid, PI / 5.0, 1e-12), "k * 180 / 5: {mid}");

    r.circle.sweep = deg(-90.0);
    let cells = tiling_layout(bx(), &r);
    assert_point(
        centre_of(&cell(&cells, 0, 5)),
        10.0,
        35.0,
        "clockwise arc end",
    );
}

#[test]
fn c12_count_one_places_one_instance_at_the_start_for_any_sweep() {
    for sweep in [360.0, 180.0, -45.0] {
        let mut r = circle(1);
        r.circle.radius = Length::from_mm(10.0);
        r.circle.centre = Vec2::new(-10.0, 0.0);
        r.circle.sweep = deg(sweep);
        r.circle.start = deg(30.0);
        let cells = tiling_layout(bx(), &r);
        assert_eq!(cells.len(), 1);
        let p = centre_of(&cells[0]);
        let t = 30_f64.to_radians();
        assert_point(p, 10.0 + 10.0 * t.cos(), 25.0 - 10.0 * t.sin(), "count 1");
        assert!(p.x.is_finite() && p.y.is_finite());
    }
}

#[test]
fn c13_face_centre_rotates_each_instance_by_its_angle_and_off_only_moves() {
    let mut r = circle(8);
    for c in tiling_layout(bx(), &r) {
        assert_lin(
            c.affine,
            [1.0, 0.0, 0.0, 1.0],
            "face centre off: translation only",
        );
    }
    r.circle.face_centre = true;
    for c in tiling_layout(bx(), &r) {
        let t = f64::from(c.key.column) * TAU / 8.0;
        assert_lin(c.affine, screen_ccw(t), "face centre on");
        // Rotated about its own box centre: the centre lands on the circle point.
        let p = centre_of(&c);
        assert_point(p, 20.0 * t.cos(), 25.0 - 20.0 * t.sin(), "about own centre");
    }
}

#[test]
fn c13_face_centre_with_a_start_angle_keeps_orientation_relative_to_the_radius() {
    // Relative to the radius: instance k is turned by k * step relative to instance 0.
    let mut r = circle(4);
    r.circle.face_centre = true;
    r.circle.start = deg(30.0);
    let cells = tiling_layout(bx(), &r);
    let a0 = angle_of(cell(&cells, 0, 0).affine);
    let a1 = angle_of(cell(&cells, 0, 1).affine);
    assert!(close((a1 - a0).rem_euclid(TAU), FRAC_PI_2, 1e-12));
}

// ---------------------------------------------------------------------------------------------
// D. Variance and seed (criteria 15 to 21)
// ---------------------------------------------------------------------------------------------

#[test]
fn c20_pcg32_matches_an_independent_reference_for_the_four_seeds() {
    for seed in [0_u64, 1, 42, 4_294_967_295] {
        let mut a = Pcg32::new(seed);
        let mut b = RefPcg::seeded(seed);
        for i in 0..16 {
            assert_eq!(a.next_u32(), b.next(), "seed {seed} output {i}");
        }
    }
}

#[test]
fn c20_pcg32_golden_file_holds_the_first_16_outputs_of_the_reference() {
    let text = include_str!("../../tests/fixtures/tiling/pcg32.json");
    let json: serde_json::Value = serde_json::from_str(text).unwrap();
    let seqs = json["sequences"].as_array().unwrap();
    let seeds: Vec<u64> = seqs.iter().map(|s| s["seed"].as_u64().unwrap()).collect();
    assert_eq!(seeds, vec![0, 1, 42, 4_294_967_295]);
    for s in seqs {
        let mut r = RefPcg::seeded(s["seed"].as_u64().unwrap());
        let outs = s["outputs"].as_array().unwrap();
        assert_eq!(outs.len(), 16);
        for o in outs {
            assert_eq!(o.as_u64().unwrap(), u64::from(r.next()));
        }
    }
}

#[test]
fn c20_advance_equals_single_steps_and_composes() {
    for n in [0_u64, 1, 2, 7, 8, 63, 64, 1000, 4097] {
        let mut a = Pcg32::new(42);
        let mut b = Pcg32::new(42);
        a.advance(n);
        for _ in 0..n {
            b.next_u32();
        }
        assert_eq!(a.next_u32(), b.next_u32(), "advance({n})");
    }
    // Jumps of the cell stretches: (2^32 + 5) * 8 in one, or in two parts.
    let big = ((1_u64 << 32) + 5) * 8;
    let mut one = Pcg32::new(9);
    one.advance(big);
    let mut two = Pcg32::new(9);
    two.advance(big / 2);
    two.advance(big - big / 2);
    assert_eq!(one, two);
    // advance(2^64 - 1) then one step returns to the start state (the LCG has period 2^64).
    let mut full = Pcg32::new(3);
    let start = full;
    full.advance(u64::MAX);
    full.next_u32();
    assert_eq!(full, start);
}

#[test]
fn c20_a_draw_becomes_a_unit_value_exactly() {
    let mut a = Pcg32::new(5);
    let mut b = Pcg32::new(5);
    for _ in 0..100 {
        let u = a.next_unit();
        let x = b.next_u32();
        assert_eq!(u, f64::from(x) / 4_294_967_296.0);
        assert!((0.0..1.0).contains(&u));
    }
}

fn all_random(r: &mut TilingRecipe) {
    let v = &mut r.variance;
    v.rotation.random = deg(20.0);
    v.scale_x.random = Percent::new(30.0);
    v.scale_y.random = Percent::new(30.0);
    v.offset_x.random = Length::from_mm(4.0);
    v.offset_y.random = Length::from_mm(2.0);
    v.opacity.random = Percent::new(25.0);
    v.hue.random = deg(90.0);
}

#[test]
fn c16_c21_random_values_follow_the_pinned_draw_order() {
    let mut r = grid(3, 3);
    all_random(&mut r);
    r.variance.uniform_scale = true;
    r.seed = 42;
    for row in 0..3 {
        for column in 0..3 {
            let key = CellKey { row, column };
            let v = cell_variance(&r.variance, 42, key);
            let u = ref_draws(42, row, column);
            let pm = |range: f64, u: f64| range * 2.0f64.mul_add(u, -1.0);
            assert!(close(
                v.rotation.as_radians(),
                pm(20_f64.to_radians(), u[0]),
                1e-12
            ));
            assert!(
                close(v.scale_x, 1.0 + pm(30.0, u[1]) / 100.0, 1e-12),
                "scale x"
            );
            assert!(
                close(v.scale_y, v.scale_x, 0.0),
                "uniform scale uses the X draw"
            );
            assert!(close(v.offset.x, pm(4.0, u[3]), 1e-12), "offset x");
            assert!(close(v.offset.y, pm(2.0, u[4]), 1e-12), "offset y");
            assert!(close(v.opacity.get(), pm(25.0, u[5]), 1e-12), "opacity");
            assert!(
                close(v.hue.as_radians(), pm(90_f64.to_radians(), u[6]), 1e-12),
                "hue"
            );
        }
    }
}

#[test]
fn c16_stepped_variance_is_row_times_s_plus_column_times_t_and_draws_nothing() {
    let mut r = grid(4, 3);
    r.variance.rotation.per_row = deg(10.0);
    r.variance.rotation.per_column = deg(-3.0);
    r.variance.offset_x.per_row = Length::from_mm(1.5);
    r.variance.offset_y.per_column = Length::from_mm(-2.0);
    r.variance.scale_x.per_column = Percent::new(10.0);
    r.variance.uniform_scale = true;
    r.variance.opacity.per_row = Percent::new(-20.0);
    r.variance.hue.per_column = deg(15.0);
    let a = tiling_layout(bx(), &r);
    r.seed = 999_999;
    let b = tiling_layout(bx(), &r);
    assert_eq!(a, b, "stepped variance does not depend on the seed");
    for c in &a {
        let (i, j) = (f64::from(c.key.row), f64::from(c.key.column));
        let rot = (i * 10.0 - j * 3.0).to_radians();
        let k = 1.0 + j * 10.0 / 100.0;
        let want = screen_ccw(rot).map(|x| x * k);
        assert_lin(c.affine, want, "rotation and scale");
        assert_point(
            centre_of(c),
            20.0 + 25.0 * j + 1.5 * i,
            25.0 + 15.0 * i - 2.0 * j,
            "offset",
        );
        assert!(close(c.opacity.get(), -20.0 * i, 1e-12));
        assert!(close(c.hue.as_radians(), (15.0 * j).to_radians(), 1e-12));
    }
}

#[test]
fn c16_mixed_random_and_stepped_add_up() {
    let mut r = grid(3, 3);
    r.variance.rotation.random = deg(5.0);
    r.variance.rotation.per_row = deg(10.0);
    r.variance.rotation.per_column = deg(1.0);
    for c in tiling_layout(bx(), &r) {
        let u = ref_draws(r.seed, c.key.row, c.key.column);
        let want = f64::from(c.key.row).mul_add(10.0, f64::from(c.key.column))
            + 5.0 * 2.0f64.mul_add(u[0], -1.0);
        let v = cell_variance(&r.variance, r.seed, c.key);
        assert!(close(v.rotation.as_radians(), want.to_radians(), 1e-12));
    }
}

#[test]
fn c16_random_ranges_must_be_zero_or_more_and_negative_steps_are_allowed() {
    let mut r = grid(2, 2);
    r.variance.rotation.per_row = deg(-5.0);
    r.variance.scale_x.per_column = Percent::new(-10.0);
    r.variance.opacity.per_row = Percent::new(-50.0);
    assert!(r.check().is_ok());
    let mut bad = r;
    bad.variance.rotation.random = deg(-1.0);
    assert_eq!(bad.check(), Err(RecipeError::OutOfRange));
    let mut bad = r;
    bad.variance.offset_y.random = Length::from_mm(-0.1);
    assert_eq!(bad.check(), Err(RecipeError::OutOfRange));
    let mut bad = r;
    bad.variance.hue.random = deg(-0.1);
    assert_eq!(bad.check(), Err(RecipeError::OutOfRange));
    let mut bad = r;
    bad.variance.opacity.random = Percent::new(-1.0);
    assert_eq!(bad.check(), Err(RecipeError::OutOfRange));
}

#[test]
fn c17_instance_affine_is_translate_rotate_scale_about_the_box_centre() {
    let mut r = grid(3, 3);
    all_random(&mut r);
    r.variance.uniform_scale = true;
    for c in tiling_layout(bx(), &r) {
        let v = cell_variance(&r.variance, r.seed, c.key);
        let k = v.scale_x;
        let want = screen_ccw(v.rotation.as_radians()).map(|x| x * k);
        assert_lin(c.affine, want, "R * S");
        let cell_pos = Vec2::new(25.0 * f64::from(c.key.column), 15.0 * f64::from(c.key.row));
        assert_point(
            centre_of(&c),
            20.0 + cell_pos.x + v.offset.x,
            25.0 + cell_pos.y + v.offset.y,
            "translate(cell + offset)",
        );
        assert!(
            (0.7 - 1e-12..=1.3 + 1e-12).contains(&k),
            "factor in 1 +/- 0.3: {k}"
        );
    }
}

#[test]
fn c17_c22_the_kept_source_is_never_varied() {
    let mut r = base(5);
    all_random(&mut r);
    r.variance.rotation.per_row = deg(5.0);
    assert!(
        tiling_layout(bx(), &r)
            .iter()
            .all(|c| c.key != CellKey::ORIGIN)
    );
    r.keep_source = false;
    let origin = cell(&tiling_layout(bx(), &r), 0, 0);
    // Not kept: (0, 0) is generated and carries its random variance.
    let v = cell_variance(&r.variance, 5, CellKey::ORIGIN);
    assert_ne!(v.rotation.as_radians(), 0.0);
    assert_ne!(origin.affine, Affine::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0));
}

#[test]
fn c17_layout_is_repeatable() {
    let mut r = grid(10, 10);
    all_random(&mut r);
    assert_eq!(tiling_layout(bx(), &r), tiling_layout(bx(), &r));
}

#[test]
fn c19_seed_extremes_work_and_different_seeds_differ() {
    let mut r = grid(4, 4);
    all_random(&mut r);
    r.seed = u32::MAX;
    let a = tiling_layout(bx(), &r);
    assert!(
        a.iter()
            .all(|c| c.affine.to_array().iter().all(|x| x.is_finite()))
    );
    assert_eq!(a, tiling_layout(bx(), &r));
    let u = ref_draws(u32::MAX, 3, 3);
    let v = cell_variance(&r.variance, u32::MAX, CellKey { row: 3, column: 3 });
    assert!(close(
        v.rotation.as_radians(),
        20_f64.to_radians() * 2.0f64.mul_add(u[0], -1.0),
        1e-12
    ));
    r.seed = 0;
    let b = tiling_layout(bx(), &r);
    assert_ne!(a, b);
}

#[test]
fn c21_one_channel_never_changes_another() {
    let mut r = grid(5, 5);
    all_random(&mut r);
    let before: Vec<_> = cell_keys_of(&r)
        .into_iter()
        .map(|k| cell_variance(&r.variance, r.seed, k))
        .collect();
    let mut changed = r;
    changed.variance.rotation.random = deg(0.0);
    changed.variance.opacity.random = Percent::new(80.0);
    changed.variance.offset_x.per_row = Length::from_mm(3.0);
    for (k, b) in cell_keys_of(&r).into_iter().zip(&before) {
        let a = cell_variance(&changed.variance, r.seed, k);
        assert_eq!(a.scale_x, b.scale_x);
        assert_eq!(a.scale_y, b.scale_y);
        assert_eq!(a.offset.y, b.offset.y);
        assert_eq!(a.hue, b.hue);
        assert_eq!(a.rotation.as_radians(), 0.0);
    }
    // Turning a channel on from zero keeps the others too (a zero channel still draws).
    let mut zero = r;
    zero.variance.hue.random = deg(0.0);
    for (k, b) in cell_keys_of(&r).into_iter().zip(&before) {
        let a = cell_variance(&zero.variance, r.seed, k);
        assert_eq!(a.rotation, b.rotation);
        assert_eq!(a.opacity, b.opacity);
    }
}

fn cell_keys_of(r: &TilingRecipe) -> Vec<CellKey> {
    tiling_layout(bx(), r).iter().map(|c| c.key).collect()
}

#[test]
fn c21_adding_rows_columns_or_instances_never_changes_existing_cells() {
    let mut r = grid(4, 3);
    all_random(&mut r);
    r.variance.rotation.per_row = deg(2.0);
    let before = tiling_layout(bx(), &r);
    for (columns, rows) in [(5, 3), (4, 4), (9, 9), (100, 1000)] {
        let mut more = r;
        more.grid.columns = columns;
        more.grid.rows = rows;
        let after = tiling_layout(bx(), &more);
        for b in &before {
            let a = cell(&after, b.key.row, b.key.column);
            assert_eq!(
                a, *b,
                "cell {:?} moved after growing to {columns}x{rows}",
                b.key
            );
        }
    }
    // Circle: adding instances keeps the variance of the existing indices (positions move).
    let mut c = circle(5);
    all_random(&mut c);
    let k3 = cell_variance(&c.variance, c.seed, CellKey { row: 0, column: 3 });
    c.circle.count = 50;
    assert_eq!(
        cell_variance(&c.variance, c.seed, CellKey { row: 0, column: 3 }),
        k3
    );
}

#[test]
fn c15_circle_per_step_adds_per_instance_index() {
    let mut r = circle(6);
    r.variance.rotation.per_column = deg(7.0);
    r.variance.offset_x.per_column = Length::from_mm(1.0);
    for c in tiling_layout(bx(), &r) {
        let k = f64::from(c.key.column);
        assert_lin(
            c.affine,
            screen_ccw((7.0 * k).to_radians()),
            "per step rotation",
        );
        let t = k * TAU / 6.0;
        assert_point(
            centre_of(&c),
            20.0 * t.cos() + k,
            25.0 - 20.0 * t.sin(),
            "per step offset",
        );
    }
}

#[test]
fn c15_scale_is_uniform() {
    let mut r = grid(4, 4);
    r.variance.scale_x.random = Percent::new(40.0);
    r.variance.uniform_scale = true;
    for c in tiling_layout(bx(), &r) {
        assert!(
            close(c.affine.a, c.affine.d, 1e-12),
            "uniform: {:?}",
            c.affine
        );
        assert!(c.affine.a > 0.0);
    }
}

#[test]
fn c15_scale_of_minus_100_percent_or_less_does_not_produce_a_degenerate_matrix() {
    // Factor 1 + v / 100 would be 0 or negative; the result must stay a usable affine.
    let mut r = grid(3, 1);
    r.variance.scale_x.per_column = Percent::new(-100.0);
    r.variance.uniform_scale = true;
    for c in tiling_layout(bx(), &r) {
        assert!(c.affine.determinant() > 0.0, "{:?}", c.key);
        assert!(c.affine.to_array().iter().all(|x| x.is_finite()));
    }
}

#[test]
fn c20_nan_and_infinite_inputs_are_refused() {
    let mut r = grid(2, 2);
    r.grid.spacing_x = Length::from_mm(f64::NAN);
    assert_eq!(r.check(), Err(RecipeError::NotFinite));
    let mut r = circle(3);
    r.circle.radius = Length::from_mm(f64::INFINITY);
    assert_eq!(r.check(), Err(RecipeError::NotFinite));
    let mut r = grid(2, 2);
    r.variance.hue.per_row = Angle::from_radians(f64::NAN);
    assert_eq!(r.check(), Err(RecipeError::NotFinite));
    let mut r = circle(3);
    r.circle.centre = Vec2::new(f64::NEG_INFINITY, 0.0);
    assert_eq!(r.check(), Err(RecipeError::NotFinite));
}

// ---------------------------------------------------------------------------------------------
// G. Counts (criterion 36)
// ---------------------------------------------------------------------------------------------

#[test]
fn c36_count_limits_warn_above_10000_and_refuse_above_100000() {
    assert_eq!(TILING_WARN_COUNT, 10_000);
    assert_eq!(TILING_MAX_COUNT, 100_000);
    assert_eq!(count_status(&grid(100, 100)), CountStatus::Fine(10_000));
    assert_eq!(
        count_status(&grid(10_001, 1)),
        CountStatus::Simplified(10_001)
    );
    assert_eq!(
        count_status(&grid(1000, 100)),
        CountStatus::Simplified(100_000)
    );
    assert_eq!(
        count_status(&grid(100_001, 1)),
        CountStatus::Refused(100_001)
    );
    assert_eq!(
        count_status(&grid(100_000, 100_000)),
        CountStatus::Refused(10_000_000_000)
    );
    assert!(grid(100_000, 1).check().is_ok());
    assert!(grid(1, 100_000).check().is_ok());
    assert_eq!(grid(100_001, 1).check(), Err(RecipeError::TooMany));
    assert_eq!(grid(400, 251).check(), Err(RecipeError::TooMany));
    assert_eq!(grid(u32::MAX, u32::MAX).check(), Err(RecipeError::TooMany));
    assert_eq!(
        count_status(&circle(100_000)),
        CountStatus::Simplified(100_000)
    );
    assert_eq!(circle(100_001).check(), Err(RecipeError::TooMany));
    // The kept source counts as an instance.
    let mut kept = grid(5, 4);
    kept.keep_source = true;
    assert_eq!(count_status(&kept), CountStatus::Fine(20));
}

#[test]
fn c36_a_tiling_at_the_limit_lays_out_every_cell_with_distinct_keys() {
    let mut r = grid(1000, 100);
    all_random(&mut r);
    r.keep_source = true;
    let cells = tiling_layout(bx(), &r);
    assert_eq!(cells.len(), 99_999);
    let mut keys: Vec<CellKey> = cells.iter().map(|c| c.key).collect();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), 99_999);
    let mut c = circle(100_000);
    c.circle.face_centre = true;
    assert_eq!(tiling_layout(bx(), &c).len(), 100_000);
}

// ---------------------------------------------------------------------------------------------
// Golden layouts (criterion 41)
// ---------------------------------------------------------------------------------------------

#[test]
fn c41_layout_goldens_hold_every_named_case() {
    let text = include_str!("../../tests/fixtures/tiling/layouts.json");
    let json: serde_json::Value = serde_json::from_str(text).unwrap();
    for case in [
        "grid_3x3_gap_0",
        "grid_mirror_odd_rows_and_columns",
        "grid_offset_rows",
        "grid_pitch",
        "circle_default",
        "circle_sweep_180_face_centre",
        "random_seed_42_all_channels",
        "stepped_per_row_and_column",
        "mixed_random_and_stepped",
    ] {
        let rows = json[case]
            .as_array()
            .unwrap_or_else(|| panic!("{case} missing"));
        assert!(!rows.is_empty(), "{case}");
        for row in rows {
            let v = row.as_array().unwrap();
            assert_eq!(v.len(), 10, "{case}: key, six affine entries, opacity, hue");
            assert!(v.iter().all(|x| x.as_f64().is_some_and(f64::is_finite)));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// A. Tile button and cell box (criteria 3, 5 and 26), on a document
// ---------------------------------------------------------------------------------------------

fn rect(d: &Document, x: f64, y: f64, w: f64, h: f64) -> curvyo_document_core::NodeId {
    d.create_rect(RectBounds {
        origin: Point::new(x, y),
        width: Length::from_mm(w),
        height: Length::from_mm(h),
    })
}

fn tile_doc(
    d: &Document,
    sources: &[curvyo_document_core::NodeId],
    edit: impl Fn(&mut TilingRecipe),
) -> curvyo_document_core::NodeId {
    let TileButton::Tile(sources) = tile_button(d, &Units::build(d, None), sources) else {
        panic!("Tile offered")
    };
    let read = d.read_objects();
    let cb = cell_box(d, &read, &sources).unwrap();
    let mut recipe = default_recipe(cb, 42);
    edit(&mut recipe);
    let mut minter = AnchorIdMinter::new(3);
    let request = tiling_request(d, &read, &mut minter, (None, &sources), &recipe, cb);
    let mut mint = || minter.mint();
    d.apply_tiling(&request, &mut mint).unwrap().tiling
}

#[test]
fn c03_tile_is_disabled_without_selection_and_for_a_tiling_with_others() {
    let d = Document::new(1);
    let a = rect(&d, 0.0, 0.0, 10.0, 10.0);
    let b = rect(&d, 50.0, 0.0, 10.0, 10.0);
    assert_eq!(
        tile_button(&d, &Units::build(&d, None), &[]),
        TileButton::NothingSelected
    );
    let tiling = tile_doc(&d, &[a], |_| {});
    let units = Units::build(&d, None);
    let mut leaves = d.tiling_leaves(&[tiling]);
    assert_eq!(tile_button(&d, &units, &leaves), TileButton::Edit(tiling));
    leaves.push(b);
    assert_eq!(
        tile_button(&d, &units, &leaves),
        TileButton::TilingWithOthers
    );
    assert!(matches!(tile_button(&d, &units, &[b]), TileButton::Tile(_)));
}

#[test]
fn c05_cell_box_is_the_union_box_and_a_group_counts_as_its_leaves() {
    let d = Document::new(1);
    let a = rect(&d, 0.0, 0.0, 10.0, 10.0);
    let b = rect(&d, 30.0, 5.0, 10.0, 20.0);
    let read = d.read_objects();
    let cb = cell_box(&d, &read, &[a, b]).unwrap();
    assert_point(cb.min, 0.0, 0.0, "min");
    assert_point(cb.max, 40.0, 25.0, "max");
    let g = d.group_objects(&[a, b]).unwrap();
    let read = d.read_objects();
    let leaves = d.tiling_leaves(&[g]);
    assert_eq!(leaves.len(), 2, "a group counts as its leaves");
    let cg = cell_box(&d, &read, &[g]).unwrap();
    assert_eq!(cg, cb);
}

#[test]
fn c05_nested_groups_are_flattened_inside_each_instance() {
    let d = Document::new(1);
    let a = rect(&d, 0.0, 0.0, 10.0, 10.0);
    let b = rect(&d, 20.0, 0.0, 10.0, 10.0);
    let c = rect(&d, 40.0, 0.0, 10.0, 10.0);
    let inner = d.group_objects(&[a, b]).unwrap();
    let outer = d.group_objects(&[inner, c]).unwrap();
    assert_eq!(d.tiling_leaves(&[outer]).len(), 3);
    let tiling = tile_doc(&d, &[outer], |r| {
        r.grid.columns = 2;
        r.grid.rows = 1;
    });
    // Kept source (the group) + one instance = a group of three members, no nested groups.
    let children = d.children(Some(tiling));
    assert_eq!(children.len(), 2);
    let instance = children.into_iter().find(|id| *id != outer).unwrap();
    let members = d.children(Some(instance));
    assert_eq!(members.len(), 3);
    assert!(members.iter().all(|m| d.children(Some(*m)).is_empty()));
}

// ---------------------------------------------------------------------------------------------
// H. History names (criterion 37)
// ---------------------------------------------------------------------------------------------

#[test]
fn c37_history_names() {
    assert_eq!(
        operation_name(curvyo_document_core::TILE_COMMIT_LABEL),
        "Tile"
    );
    assert_eq!(
        operation_name(curvyo_document_core::EDIT_TILING_COMMIT_LABEL),
        "Edit tiling"
    );
    assert_eq!(
        operation_name(curvyo_document_core::RELEASE_TILING_COMMIT_LABEL),
        "Release tiling"
    );
}

// ---------------------------------------------------------------------------------------------
// F. Export (criteria 33 and 42)
// ---------------------------------------------------------------------------------------------

fn export(d: &Document) -> String {
    let store = FaceStore::new();
    let fonts = ImportFaces {
        faces: &[],
        store: &store,
        substitute: None,
    };
    let laid = d.read_objects();
    let outcome = svg_export(
        d,
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
fn c33_svg_of_a_multi_source_circle_tiling_equals_the_released_group() {
    let d = Document::new(1);
    let a = rect(&d, 10.0, 10.0, 12.0, 6.0);
    let b = rect(&d, 30.0, 10.0, 4.0, 4.0);
    let tiling = tile_doc(&d, &[a, b], |r| {
        r.layout = TilingLayout::Circle;
        r.circle.face_centre = true;
        r.keep_source = false;
        r.variance.rotation.random = deg(15.0);
        r.variance.scale_x.random = Percent::new(10.0);
        r.variance.opacity.per_column = Percent::new(-5.0);
        r.grid.mirror_rows = true;
    });
    let with = export(&d);
    for word in ["tiling", "tiling_cell", "tiling_member", "tiling_sources"] {
        assert!(!with.contains(word), "{word} leaked into the SVG");
    }
    d.release_tiling(tiling).unwrap();
    assert_eq!(export(&d), with);
}

#[test]
fn c33_exported_and_on_screen_geometry_match_the_layout_within_a_micrometre() {
    let d = Document::new(1);
    let a = rect(&d, 10.0, 20.0, 20.0, 10.0);
    let tiling = tile_doc(&d, &[a], |r| {
        all_random(r);
        r.grid.columns = 3;
        r.grid.rows = 2;
        r.variance.uniform_scale = true;
    });
    let read = d.read_objects();
    let recipe = d.tiling(tiling).unwrap().recipe;
    let cells = tiling_layout(bx(), &recipe);
    let source = read.get(a).unwrap().drawn_path().cloned().unwrap();
    let mut matched = 0;
    for child in d.children(Some(tiling)) {
        if child == a {
            continue;
        }
        let Some(curvyo_document_core::ObjectSnapshot::Clone(clone)) = read.get(child) else {
            panic!("a clone instance")
        };
        let affine = clone.chain.matrix;
        let cell = cells
            .iter()
            .find(|c| {
                c.affine
                    .to_array()
                    .iter()
                    .zip(affine.to_array())
                    .all(|(x, y)| close(*x, y, 1e-9))
            })
            .expect("the clone's matrix is one cell's affine");
        for (s, r) in source.anchors.iter().zip(&clone.resolved.anchors) {
            let want = cell.affine.apply_point(s.point);
            assert!(close(want.x, r.point.x, 1e-3) && close(want.y, r.point.y, 1e-3));
        }
        matched += 1;
    }
    assert_eq!(matched, 5);
}
