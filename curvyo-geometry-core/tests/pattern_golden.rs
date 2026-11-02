//! The golden pieces of every pattern generator (`specs/0071-pattern-fill` criterion 17, ADR 0029
//! §5): a 100 mm square with a hole, a rotated circle and a wavy top over a straight bottom, Lines and Shapes, with and without a
//! margin, and with whole pieces only where that applies (criterion 29), written as a summary (pieces, outlines, nodes, a hash of the nodes in integer
//! nanometres) and the first piece in full. Compared with `fixtures/pattern/cases.golden`
//! natively and in Node on `wasm32-unknown-unknown`, which shows the pieces are the same bits on
//! both:
//!
//! ```text
//! cargo test --target wasm32-unknown-unknown -p curvyo-geometry-core --test pattern_golden
//! ```
//!
//! Regenerate with `CURVYO_UPDATE_GOLDEN=1` (native) and review the diff.

#![allow(
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    missing_docs
)]

use std::fmt::Write as _;

use curvyo_document_core::{
    Angle, GeneratorKind, Length, PatternFill, PatternGenerator, PatternMode, PatternSeed, Point,
    Tolerance, Vec2,
};
use curvyo_geometry_core::{PatternOutline, PatternPiece, pattern_pieces_of};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const GOLDEN: &str = include_str!("fixtures/pattern/cases.golden");
const TOLERANCE: Tolerance = Tolerance::from_mm(0.01);

fn polygon(points: &[(f64, f64)]) -> Vec<(Point, Vec2, Vec2)> {
    points
        .iter()
        .map(|&(x, y)| (Point::new(x, y), Vec2::ZERO, Vec2::ZERO))
        .collect()
}

fn circle(cx: f64, cy: f64, r: f64) -> Vec<(Point, Vec2, Vec2)> {
    let k = 0.552_284_749_830_793_6 * r;
    let a = |x: f64, y: f64, hin: (f64, f64), hout: (f64, f64)| {
        (
            Point::new(cx + x, cy + y),
            Vec2::new(hin.0, hin.1),
            Vec2::new(hout.0, hout.1),
        )
    };
    vec![
        a(r, 0.0, (0.0, -k), (0.0, k)),
        a(0.0, r, (k, 0.0), (-k, 0.0)),
        a(-r, 0.0, (0.0, k), (0.0, -k)),
        a(0.0, -r, (-k, 0.0), (k, 0.0)),
    ]
}

/// A wavy top edge over a straight bottom (Interpolation waves, customer 2026-11-02).
fn wavy_top() -> Vec<(Point, Vec2, Vec2)> {
    let a = |x: f64, y: f64, h: f64| (Point::new(x, y), Vec2::new(h, 0.0), Vec2::new(-h, 0.0));
    let corner = |x: f64, y: f64| (Point::new(x, y), Vec2::ZERO, Vec2::ZERO);
    vec![
        corner(0.0, 100.0),
        corner(100.0, 100.0),
        corner(100.0, 20.0),
        a(75.0, 10.0, 8.0),
        a(50.0, 20.0, 8.0),
        a(25.0, 30.0, 8.0),
        corner(0.0, 20.0),
    ]
}

/// `(name, outlines, rotation)`.
fn shapes() -> Vec<(&'static str, Vec<PatternOutline>, f64)> {
    vec![
        (
            "holed square",
            vec![
                (
                    polygon(&[(10.0, 10.0), (110.0, 10.0), (110.0, 110.0), (10.0, 110.0)]),
                    true,
                ),
                (
                    polygon(&[(50.0, 50.0), (50.0, 70.0), (70.0, 70.0), (70.0, 50.0)]),
                    true,
                ),
            ],
            0.0,
        ),
        (
            "rotated circle",
            vec![(circle(30.0, -20.0, 40.0), true)],
            0.3,
        ),
        ("wavy top", vec![(wavy_top(), true)], 0.0),
    ]
}

fn nm(v: f64) -> i64 {
    (v * 1.0e6).round() as i64
}

fn piece_line(piece: &PatternPiece) -> String {
    piece
        .outlines
        .iter()
        .map(|(anchors, closed)| {
            let nodes: Vec<String> = anchors
                .iter()
                .map(|(p, a, b)| {
                    format!(
                        "{},{};{},{};{},{}",
                        nm(p.x),
                        nm(p.y),
                        nm(a.x),
                        nm(a.y),
                        nm(b.x),
                        nm(b.y)
                    )
                })
                .collect();
            format!("{}[{}]", if *closed { "Z" } else { "L" }, nodes.join(" "))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The summary and first piece of one case.
fn render_case(text: &mut String, outlines: &[PatternOutline], rotation: f64, fill: &PatternFill) {
    let pieces = pattern_pieces_of(
        outlines,
        Angle::from_radians(rotation),
        fill,
        Length::from_mm(0.25),
        TOLERANCE,
    )
    .unwrap();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut nodes = 0;
    let mut outlines_n = 0;
    for piece in &pieces {
        outlines_n += piece.outlines.len();
        nodes += piece.outlines.iter().map(|o| o.0.len()).sum::<usize>();
        for byte in piece_line(piece).bytes() {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
        }
    }
    writeln!(
        text,
        "pieces {} outlines {outlines_n} nodes {nodes} hash {hash:016x}",
        pieces.len()
    )
    .unwrap();
    if let Some(first) = pieces.first() {
        writeln!(text, "first {}", piece_line(first)).unwrap();
    }
}

fn render() -> String {
    let mut text = String::from(
        "# Pattern pieces (specs/0071-pattern-fill criterion 17).\n# Regenerate with CURVYO_UPDATE_GOLDEN=1.\n",
    );
    for (shape, outlines, rotation) in shapes() {
        for kind in GeneratorKind::ALL {
            for (mode, margin) in [(PatternMode::Lines, 0.0), (PatternMode::Shapes, 2.0)] {
                for whole in [false, true] {
                    if whole && !kind.has_whole_pieces() {
                        continue;
                    }
                    let fill = PatternFill {
                        generator: PatternGenerator::defaults(kind),
                        margin: Length::from_mm(margin),
                        mode,
                        seed: PatternSeed::new(20_261_102),
                        whole_pieces: whole,
                    };
                    let suffix = if whole { " whole pieces" } else { "" };
                    writeln!(
                        text,
                        "# {shape} {} {mode:?} margin {margin}{suffix}",
                        kind.id()
                    )
                    .unwrap();
                    render_case(&mut text, &outlines, rotation, &fill);
                }
            }
        }
    }
    text
}

#[test]
fn ac17_every_generator_matches_its_golden_pieces() {
    let text = render();
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/pattern/cases.golden");
        std::fs::write(path, &text).unwrap();
        return;
    }
    assert_eq!(
        text,
        GOLDEN.replace("\r\n", "\n"),
        "the pattern pieces differ from the golden file"
    );
}
