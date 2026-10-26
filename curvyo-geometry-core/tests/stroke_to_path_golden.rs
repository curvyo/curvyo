//! The golden results of the stroke outline kernel (`specs/0085-stroke-to-path` criteria 28 and
//! 29 (a), (d)): the shapes of criteria 4 to 10 and a 12-node Bézier path at every join and cap,
//! written as nodes and handles in integer nanometres (1e-6 mm), and a 5,000-node path with the
//! dash `2 1` as a summary (outlines, segments, a hash of the nodes). Compared with
//! `fixtures/stroke_to_path/cases.golden` natively and in Node on `wasm32-unknown-unknown`:
//!
//! ```text
//! cargo test --target wasm32-unknown-unknown -p curvyo-geometry-core --test stroke_to_path_golden
//! ```
//!
//! The budgets (50 ms per shape, 2 s for the long path) are asserted natively in a release build
//! (the `performance-budgets` CI job). Regenerate with `CURVYO_UPDATE_GOLDEN=1` (native) and
//! review the diff.

#![allow(
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    missing_docs
)]

mod common;

use std::fmt::Write as _;

use common::{Anchors, KERNEL_TOLERANCE, circle, polygon, rect, wavy};
use curvyo_document_core::{
    DashPattern, Length, LineCap, LineJoin, MarkerPlace, MarkerShape, Markers, Point, Vec2,
};
use curvyo_geometry_core::{
    OffsetOutlines, Outline, StrokeOutlineError, StrokeShape, stroke_outline,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const GOLDEN: &str = include_str!("fixtures/stroke_to_path/cases.golden");

/// `(name, outlines with their closed flag, stroke)`.
type Case = (String, Vec<(Anchors, bool)>, StrokeShape);

fn stroke(width: f64, join: LineJoin, cap: LineCap) -> StrokeShape {
    StrokeShape {
        width: Length::from_mm(width),
        join,
        cap,
        dash: DashPattern::solid(),
        markers: Markers::default(),
    }
}

fn dashed(mut shape: StrokeShape, pattern: &[f64]) -> StrokeShape {
    shape.dash = DashPattern::new(pattern.to_vec()).unwrap();
    shape
}

/// The 12-node path of criterion 11: lines and cubics, sharp and smooth nodes.
fn bezier_path() -> Anchors {
    let a = |x: f64, y: f64, hin: (f64, f64), hout: (f64, f64)| {
        (
            Point::new(x, y),
            Vec2::new(hin.0, hin.1),
            Vec2::new(hout.0, hout.1),
        )
    };
    vec![
        a(0.0, 0.0, (0.0, 0.0), (6.0, -4.0)),
        a(12.0, -2.0, (-3.0, -2.0), (3.0, 2.0)),
        a(20.0, 6.0, (0.0, 0.0), (0.0, 0.0)),
        a(30.0, 0.0, (0.0, 0.0), (4.0, 4.0)),
        a(36.0, 10.0, (0.0, -4.0), (0.0, 4.0)),
        a(30.0, 20.0, (4.0, 0.0), (0.0, 0.0)),
        a(20.0, 16.0, (0.0, 0.0), (0.0, 0.0)),
        a(12.0, 24.0, (4.0, -2.0), (-4.0, 2.0)),
        a(2.0, 22.0, (0.0, 0.0), (0.0, 0.0)),
        a(-4.0, 14.0, (0.0, 3.0), (0.0, -3.0)),
        a(4.0, 8.0, (-3.0, 0.0), (3.0, 0.0)),
        a(14.0, 10.0, (-2.0, -2.0), (0.0, 0.0)),
    ]
}

fn cases() -> Vec<Case> {
    use LineCap::{Butt, Round, Square};
    use LineJoin::{Bevel, Miter};
    let straight = || (polygon(&[(0.0, 0.0), (100.0, 0.0)]), false);
    let mut cases: Vec<Case> = vec![
        (
            "ac4 butt".into(),
            vec![straight()],
            stroke(2.0, Miter, Butt),
        ),
        (
            "ac4 square".into(),
            vec![straight()],
            stroke(2.0, Miter, Square),
        ),
        (
            "ac4 round".into(),
            vec![straight()],
            stroke(2.0, Miter, Round),
        ),
    ];
    for join in [Miter, LineJoin::Round, Bevel] {
        cases.push((
            format!("ac5 square {join:?}"),
            vec![(rect(0.0, 0.0, 20.0, 20.0), true)],
            stroke(2.0, join, Butt),
        ));
    }
    cases.push((
        "ac6 sharp corner".into(),
        vec![(polygon(&[(0.0, 0.0), (50.0, 0.0), (0.0, 5.0)]), false)],
        stroke(2.0, Miter, Butt),
    ));
    cases.push((
        "ac7 circle".into(),
        vec![(circle(0.0, 0.0, 10.0), true)],
        stroke(2.0, Miter, Butt),
    ));
    for cap in [Butt, Round, Square] {
        cases.push((
            format!("ac8 dash 5 5 {cap:?}"),
            vec![straight()],
            dashed(stroke(1.0, Miter, cap), &[5.0, 5.0]),
        ));
    }
    cases.push((
        "ac8 dash around a corner".into(),
        vec![(polygon(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]), false)],
        dashed(stroke(1.0, Miter, Butt), &[15.0, 5.0]),
    ));
    cases.push((
        "ac9 figure eight".into(),
        vec![(
            polygon(&[(0.0, 0.0), (20.0, 20.0), (20.0, 0.0), (0.0, 20.0)]),
            false,
        )],
        stroke(3.0, Miter, Butt),
    ));
    let mut arrows = stroke(1.0, Miter, Butt);
    arrows.markers = Markers {
        start: MarkerShape::Dot,
        end: MarkerShape::Arrow,
        mid: MarkerShape::Arrow,
        mid_place: MarkerPlace::AtNodes,
        ..Markers::default()
    };
    cases.push((
        "ac10 markers".into(),
        vec![(polygon(&[(0.0, 0.0), (30.0, 0.0), (30.0, 20.0)]), false)],
        arrows,
    ));
    for join in [Miter, LineJoin::Round, Bevel] {
        for cap in [Butt, Round, Square] {
            cases.push((
                format!("ac11 bezier {join:?} {cap:?}"),
                vec![(bezier_path(), false)],
                stroke(2.0, join, cap),
            ));
        }
    }
    cases
}

fn run(
    outlines: &[(Anchors, bool)],
    shape: &StrokeShape,
) -> Result<OffsetOutlines, StrokeOutlineError> {
    let borrowed: Vec<Outline<'_>> = outlines
        .iter()
        .map(|(anchors, closed)| Outline::new(anchors, *closed))
        .collect();
    stroke_outline(&borrowed, shape, KERNEL_TOLERANCE)
}

fn nm(v: f64) -> i64 {
    (v * 1.0e6).round() as i64
}

fn node_lines(result: &OffsetOutlines) -> Vec<String> {
    result
        .outlines()
        .iter()
        .map(|outline| {
            outline
                .iter()
                .map(|a| {
                    format!(
                        "{},{};{},{};{},{}{}",
                        nm(a.point.x),
                        nm(a.point.y),
                        nm(a.handle_in.x),
                        nm(a.handle_in.y),
                        nm(a.handle_out.x),
                        nm(a.handle_out.y),
                        if a.smooth { "s" } else { "" }
                    )
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

/// Runs `case` and checks `budget_ms` in a native release build.
fn timed(
    name: &str,
    budget_ms: u64,
    case: impl FnOnce() -> Result<OffsetOutlines, StrokeOutlineError>,
) -> Result<OffsetOutlines, StrokeOutlineError> {
    #[cfg(not(target_arch = "wasm32"))]
    let started = std::time::Instant::now();
    let result = case();
    #[cfg(not(target_arch = "wasm32"))]
    {
        let elapsed = started.elapsed();
        println!("{name}: {elapsed:?}");
        if !cfg!(debug_assertions) {
            assert!(
                elapsed < std::time::Duration::from_millis(budget_ms),
                "{name}: {elapsed:?}"
            );
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (name, budget_ms);
    result
}

fn render() -> String {
    let mut text = String::from(
        "# Stroke to path (specs/0085-stroke-to-path criteria 28, 29).\n# Regenerate with CURVYO_UPDATE_GOLDEN=1.\n",
    );
    for (name, outlines, shape) in cases() {
        writeln!(text, "# {name}").unwrap();
        match timed(&name, 50, || run(&outlines, &shape)) {
            Err(error) => writeln!(text, "error {error:?}").unwrap(),
            Ok(result) => {
                for line in node_lines(&result) {
                    writeln!(text, "{line}").unwrap();
                }
            }
        }
    }
    // Criterion 29 (d): 5,000 nodes, dash `2 1`.
    let long = vec![(wavy(5000, 0.0, 0.0), false)];
    let shape = dashed(stroke(0.5, LineJoin::Round, LineCap::Round), &[2.0, 1.0]);
    writeln!(text, "# ac29d 5000 nodes dash 2 1").unwrap();
    let result = timed("ac29d", 2000, || run(&long, &shape)).unwrap();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for line in node_lines(&result) {
        for byte in line.bytes() {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
        }
    }
    writeln!(
        text,
        "outlines {} segments {} fallback {} hash {hash:016x}",
        result.outlines().len(),
        result.segment_count(),
        result.fallback_outlines()
    )
    .unwrap();
    text
}

#[test]
fn ac28_ac29_every_case_matches_its_golden_result_within_budget() {
    let text = render();
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/stroke_to_path/cases.golden");
        std::fs::write(path, &text).unwrap();
        return;
    }
    assert_eq!(
        text,
        GOLDEN.replace("\r\n", "\n"),
        "the stroke outlines differ from the golden file"
    );
}
