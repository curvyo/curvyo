//! The golden files of `specs/0159-cutting-plotter` criteria 11, 13, 14 and 16: the Design Space
//! SVG and HPGL of fixed jobs (`tests/fixtures/cutter_*`). `CURVYO_UPDATE_GOLDEN=1` rewrites the
//! files natively; review the diff. The goldens are embedded with `include_str!`, so the same
//! comparison runs in Node on `wasm32-unknown-unknown` (`adrs.md` PL7):
//!
//! ```text
//! cargo test --target wasm32-unknown-unknown -p curvyo-cutter-core --test writer_goldens
//! ```

#![allow(clippy::unwrap_used, clippy::missing_panics_doc, missing_docs)]

use curvyo_cutter_core::{
    CRICUT_MAKER_3, CutterSettings, GENERIC_HPGL_CUTTER, ItemOutline, JobItem, JobOperation,
    MatSize, OpKind, Passes, build_job, write_hpgl, write_svg,
};
use curvyo_document_core::{DocumentSize, Point, Vec2};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn square(x: f64, y: f64, size: f64) -> ItemOutline {
    let p = |x, y| (Point::new(x, y), Vec2::ZERO, Vec2::ZERO);
    ItemOutline {
        anchors: vec![
            p(x, y),
            p(x + size, y),
            p(x + size, y + size),
            p(x, y + size),
        ],
        closed: true,
    }
}

/// A circle of radius `r` around `(cx, cy)` as four cubics.
fn circle(cx: f64, cy: f64, r: f64) -> ItemOutline {
    let k = 0.552_284_749_830_793_4 * r;
    ItemOutline {
        anchors: vec![
            (
                Point::new(cx + r, cy),
                Vec2::new(0.0, -k),
                Vec2::new(0.0, k),
            ),
            (
                Point::new(cx, cy + r),
                Vec2::new(k, 0.0),
                Vec2::new(-k, 0.0),
            ),
            (
                Point::new(cx - r, cy),
                Vec2::new(0.0, k),
                Vec2::new(0.0, -k),
            ),
            (
                Point::new(cx, cy - r),
                Vec2::new(-k, 0.0),
                Vec2::new(k, 0.0),
            ),
        ],
        closed: true,
    }
}

/// An open stroke, as a single-line font letter gives it.
fn stroke(points: &[(f64, f64)]) -> ItemOutline {
    ItemOutline {
        anchors: points
            .iter()
            .map(|&(x, y)| (Point::new(x, y), Vec2::ZERO, Vec2::ZERO))
            .collect(),
        closed: false,
    }
}

fn operation(name: &str, kind: OpKind, passes: u64, outlines: Vec<ItemOutline>) -> JobOperation {
    let mut settings = CutterSettings::defaults(&GENERIC_HPGL_CUTTER.profile().values);
    settings.passes = Passes::new(passes).unwrap();
    JobOperation {
        name: name.to_owned(),
        kind,
        settings,
        items: outlines
            .into_iter()
            .enumerate()
            .map(|(n, outline)| JobItem {
                outlines: vec![outline],
                stacking: u32::try_from(n).unwrap(),
            })
            .collect(),
    }
}

fn cut_draw() -> Vec<JobOperation> {
    vec![
        operation(
            "Draw 1",
            OpKind::Draw,
            1,
            vec![stroke(&[(40.0, 40.0), (60.0, 80.0), (80.0, 40.0)])],
        ),
        operation(
            "Cut 1",
            OpKind::Cut,
            1,
            vec![square(30.0, 30.0, 60.0), circle(200.0, 200.0, 25.0)],
        ),
    ]
}

fn check(name: &str, golden: &str, actual: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        std::fs::write(path, actual).unwrap();
        return;
    }
    assert_eq!(actual, golden, "{name} differs from its golden");
}

#[test]
fn design_space_svg_of_cut_and_draw_on_the_12x12_mat() {
    let mat = MatSize::In12x12;
    let svg = write_svg(&build_job(&cut_draw()), mat.field_size(), Some(mat));
    check(
        "cutter_mat12_cut_draw.svg",
        include_str!("fixtures/cutter_mat12_cut_draw.svg"),
        &svg,
    );
    assert!(svg.contains("width=\"12in\" height=\"12in\" viewBox=\"0 0 304.800 304.800\""));
}

#[test]
fn design_space_svg_of_two_cuts_on_the_12x24_mat() {
    let mat = MatSize::In12x24;
    let ops = vec![
        operation("Cut", OpKind::Cut, 1, vec![square(10.0, 400.0, 100.0)]),
        operation("Cut", OpKind::Cut, 3, vec![circle(150.0, 150.0, 40.0)]),
    ];
    let svg = write_svg(&build_job(&ops), mat.field_size(), Some(mat));
    check(
        "cutter_mat24_cut.svg",
        include_str!("fixtures/cutter_mat24_cut.svg"),
        &svg,
    );
    assert!(svg.contains("id=\"Cut-2\""), "unique group ids");
    assert_eq!(
        svg.matches("<path ").count(),
        2,
        "passes are not repeated in SVG"
    );
}

fn hpgl_page() -> DocumentSize {
    DocumentSize::from_mm(600.0, 1000.0)
}

#[test]
fn hpgl_of_a_50_mm_square_at_the_origin() {
    let values = GENERIC_HPGL_CUTTER.profile().values;
    let ops = vec![operation(
        "Cut 1",
        OpKind::Cut,
        1,
        vec![square(0.0, 950.0, 50.0)],
    )];
    let plt = write_hpgl(&build_job(&ops), hpgl_page(), values.origin, values.force);
    check(
        "cutter_hpgl_square.plt",
        include_str!("fixtures/cutter_hpgl_square.plt"),
        &plt,
    );
    assert!(plt.starts_with("IN;\nVS20;\nFS32;\n"), "{plt}");
    assert!(plt.contains("2000,2000"));
    assert!(plt.ends_with("PU;\nSP0;\n"));
}

#[test]
fn hpgl_passes_two_doubles_the_pen_down_runs() {
    let values = GENERIC_HPGL_CUTTER.profile().values;
    let ops = |passes| {
        vec![operation(
            "Cut 1",
            OpKind::Cut,
            passes,
            vec![square(0.0, 950.0, 50.0)],
        )]
    };
    let one = write_hpgl(
        &build_job(&ops(1)),
        hpgl_page(),
        values.origin,
        values.force,
    );
    let two = write_hpgl(
        &build_job(&ops(2)),
        hpgl_page(),
        values.origin,
        values.force,
    );
    let pd = |text: &str| text.lines().filter(|l| l.starts_with("PD")).count();
    assert_eq!(pd(&two), 2 * pd(&one), "criterion 14");
    let runs: Vec<&str> = two.lines().filter(|l| l.starts_with("PD")).collect();
    assert_eq!(runs[0], runs[1], "each pass on the same points");
}

#[test]
fn hpgl_of_cut_and_draw() {
    let values = GENERIC_HPGL_CUTTER.profile().values;
    let plt = write_hpgl(
        &build_job(&cut_draw()),
        hpgl_page(),
        values.origin,
        values.force,
    );
    check(
        "cutter_hpgl_cut_draw.plt",
        include_str!("fixtures/cutter_hpgl_cut_draw.plt"),
        &plt,
    );
}

#[test]
fn two_exports_of_an_unchanged_job_are_byte_identical() {
    let mat = MatSize::In12x12;
    let a = write_svg(&build_job(&cut_draw()), mat.field_size(), Some(mat));
    let b = write_svg(&build_job(&cut_draw()), mat.field_size(), Some(mat));
    assert_eq!(a, b, "criterion 16");
    assert_eq!(
        CRICUT_MAKER_3.profile().values.mats.first(),
        Some(&MatSize::In12x12)
    );
}

#[test]
fn consecutive_equal_points_after_rounding_are_dropped() {
    let values = GENERIC_HPGL_CUTTER.profile().values;
    // 10 µm steps round onto the same plotter unit (25 µm).
    let ops = vec![operation(
        "Draw 1",
        OpKind::Draw,
        1,
        vec![stroke(&[(0.0, 0.0), (0.01, 0.0), (0.02, 0.0), (10.0, 0.0)])],
    )];
    let plt = write_hpgl(&build_job(&ops), hpgl_page(), values.origin, values.force);
    let pd: Vec<&str> = plt.lines().filter(|l| l.starts_with("PD")).collect();
    assert_eq!(pd, ["PD1,40000,400,40000;"], "{plt}");
}
