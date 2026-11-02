//! Tester check of `specs/0071-pattern-fill` criterion 17 and ADR 0029 §5: the pieces are the
//! same bits on every machine and in wasm. A digest of every coordinate's `f64` bits (not a
//! rounded print) per case is compared with a golden written by the native run; CI and the tester
//! run the same file on `wasm32-unknown-unknown` (`wasm-bindgen-test-runner`).

#![allow(clippy::unwrap_used, missing_docs)]

use std::fmt::Write as _;

use curvyo_document_core::{
    Angle, GeneratorKind, Length, PatternFill, PatternGenerator, PatternMode, PatternSeed, Point,
    Tolerance, Vec2,
};
use curvyo_geometry_core::{PatternOutline, pattern_pieces_of};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const GOLDEN: &str = include_str!("fixtures/pattern/acceptance_0071_bits.golden");

fn poly(points: &[(f64, f64)]) -> Vec<(Point, Vec2, Vec2)> {
    points
        .iter()
        .map(|&(x, y)| (Point::new(x, y), Vec2::ZERO, Vec2::ZERO))
        .collect()
}

fn circle(cx: f64, cy: f64, r: f64) -> Vec<(Point, Vec2, Vec2)> {
    let k = 0.552_284_749_830_793_6 * r;
    // Wound against the outer outline below, so it is a hole under the nonzero rule.
    vec![
        (
            Point::new(cx + r, cy),
            Vec2::new(0.0, k),
            Vec2::new(0.0, -k),
        ),
        (
            Point::new(cx, cy - r),
            Vec2::new(k, 0.0),
            Vec2::new(-k, 0.0),
        ),
        (
            Point::new(cx - r, cy),
            Vec2::new(0.0, -k),
            Vec2::new(0.0, k),
        ),
        (
            Point::new(cx, cy + r),
            Vec2::new(-k, 0.0),
            Vec2::new(k, 0.0),
        ),
    ]
}

fn region() -> Vec<PatternOutline> {
    vec![
        (
            poly(&[
                (3.3, 1.7),
                (91.1, 4.9),
                (88.0, 63.25),
                (47.5, 41.0),
                (2.0, 58.8),
            ]),
            true,
        ),
        (circle(30.0, 25.0, 7.5), true),
    ]
}

fn fnv(hash: &mut u64, v: f64) {
    for b in v.to_bits().to_le_bytes() {
        *hash ^= u64::from(b);
        *hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
}

fn digest() -> String {
    let mut out = String::new();
    for kind in GeneratorKind::ALL {
        for mode in [PatternMode::Lines, PatternMode::Shapes] {
            for (seed, rotation) in [(7_u32, 0.0), (4_000_000_000, 0.37)] {
                let fill = PatternFill {
                    generator: PatternGenerator::defaults(kind),
                    margin: Length::from_mm(1.25),
                    mode,
                    seed: PatternSeed::new(seed),
                    whole_pieces: false,
                };
                let pieces = pattern_pieces_of(
                    &region(),
                    Angle::from_radians(rotation),
                    &fill,
                    Length::from_mm(0.3),
                    Tolerance::from_mm(0.01),
                )
                .unwrap();
                let mut hash = 0xcbf2_9ce4_8422_2325_u64;
                let mut anchors = 0;
                for piece in &pieces {
                    for (outline, closed) in &piece.outlines {
                        fnv(&mut hash, if *closed { 1.0 } else { 0.0 });
                        for (p, a, b) in outline {
                            anchors += 1;
                            for v in [p.x, p.y, a.x, a.y, b.x, b.y] {
                                fnv(&mut hash, v);
                            }
                        }
                    }
                }
                writeln!(
                    out,
                    "{} {mode:?} seed {seed} rot {rotation}: {} pieces {anchors} anchors {hash:016x}",
                    kind.id(),
                    pieces.len()
                )
                .unwrap();
            }
        }
    }
    out
}

#[test]
fn c17_pieces_are_bit_identical_to_the_golden() {
    let text = digest();
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/pattern/acceptance_0071_bits.golden"
        );
        std::fs::write(path, &text).unwrap();
        return;
    }
    assert_eq!(text, GOLDEN, "pattern bits differ from the native golden");
}
