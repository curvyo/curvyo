//! Fracture and Flatten on the fixtures of `specs/0037-fracture-and-flatten` criterion 17 and a
//! seeded check that the results tile the union (criterion 7).
//!
//! (a) and (b) come from a generator, their expected counts are golden; (c) reuses the fixture
//! files of `0016` criterion 40. The 2 s budget is asserted in release builds only; CI runs this
//! file in release in the `performance-budgets` job:
//! `cargo test --release -p curvyo-geometry-core --test fracture_flatten`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    missing_docs
)]

mod common;

use std::path::Path;
use std::time::{Duration, Instant};

use common::golden::{input_part, normalise, parse_operands};
use common::{KERNEL_TOLERANCE, Operand, circle, rect};
use curvyo_document_core::{Point, Vec2};
use curvyo_geometry_core::{
    BooleanOp, Fracture, Outline, StackError, Visible, boolean, fracture_cells, signed_area_mm2,
    visible_regions,
};

const LIMIT: usize = 5_000;
const BUDGET: Duration = Duration::from_secs(2);

fn with_operands<T>(operands: &[Operand], run: impl FnOnce(&[&[Outline<'_>]]) -> T) -> T {
    let outlines: Vec<Vec<Outline<'_>>> = operands
        .iter()
        .map(|operand| {
            operand
                .iter()
                .map(|anchors| Outline::new(anchors, true))
                .collect()
        })
        .collect();
    let borrowed: Vec<&[Outline<'_>]> = outlines.iter().map(Vec::as_slice).collect();
    run(&borrowed)
}

fn fracture(operands: &[Operand]) -> Result<Fracture, StackError> {
    with_operands(operands, |o| fracture_cells(o, KERNEL_TOLERANCE, LIMIT))
}

fn flatten(operands: &[Operand]) -> Result<Vec<Visible>, StackError> {
    with_operands(operands, |o| visible_regions(o, KERNEL_TOLERANCE))
}

fn area(outlines: &[Vec<Point>]) -> f64 {
    outlines
        .iter()
        .map(|outline| signed_area_mm2(outline))
        .sum()
}

/// The area of an operand of straight outlines.
fn operand_area(operand: &Operand) -> f64 {
    let points: Vec<Vec<Point>> = operand
        .iter()
        .map(|anchors| anchors.iter().map(|anchor| anchor.0).collect())
        .collect();
    area(&points)
}

/// A result fed back in as an operand.
fn as_operand(outlines: &[Vec<Point>]) -> Operand {
    outlines
        .iter()
        .map(|outline| {
            outline
                .iter()
                .map(|&p| (p, Vec2::ZERO, Vec2::ZERO))
                .collect()
        })
        .collect()
}

/// `count` squares of side 10 mm at x = 5 i, the highest index on top.
fn chain(count: usize) -> Vec<Operand> {
    (0..count)
        .map(|i| {
            let x = 5.0 * i as f64;
            vec![rect(x, 0.0, x + 10.0, 10.0)]
        })
        .collect()
}

fn timed<T>(name: &str, run: impl FnOnce() -> T) -> T {
    let started = Instant::now();
    let result = run();
    let elapsed = started.elapsed();
    println!("{name}: {elapsed:?}");
    if !cfg!(debug_assertions) {
        assert!(elapsed < BUDGET, "{name} took {elapsed:?}");
    }
    result
}

/// Criterion 17 (a): 21 columns of 5 x 10 mm; Flatten trims 19 squares and leaves the top one.
#[test]
fn twenty_squares_in_a_chain() {
    let operands = chain(20);
    let pieces = fracture(&operands).unwrap().pieces;
    assert_eq!(pieces.len(), 21);
    assert!(
        pieces
            .iter()
            .all(|(_, outlines)| (area(outlines) - 50.0).abs() < 1e-6)
    );
    assert_eq!(
        pieces[0].0, 0,
        "the left-most column has one covering square"
    );
    assert!(
        pieces[1..]
            .iter()
            .zip(1..)
            .all(|(piece, owner)| piece.0 == owner.min(19))
    );
    let visible = flatten(&operands).unwrap();
    assert_eq!(visible[19], Visible::Untouched);
    for entry in &visible[..19] {
        let Visible::Trimmed(outlines) = entry else {
            panic!("trimmed expected: {entry:?}");
        };
        assert!((area(outlines) - 50.0).abs() < 1e-6);
    }
}

/// Criterion 17 (b): 2,000 squares in a chain, within 2 s each.
#[test]
fn two_thousand_squares_in_a_chain() {
    let operands = chain(2_000);
    let pieces = timed("fracture (b)", || fracture(&operands))
        .unwrap()
        .pieces;
    assert_eq!(pieces.len(), 2_001);
    let visible = timed("flatten (b)", || flatten(&operands)).unwrap();
    assert_eq!(visible.len(), 2_000);
}

/// Nested shapes (a target of 500 rings, the largest at the bottom) stay linear: every box meets
/// every other, so a pairwise fold would be quadratic (review of 2026-10-19).
#[test]
fn five_hundred_concentric_circles() {
    let operands: Vec<Operand> = (0..500)
        .map(|i| vec![circle(0.0, 0.0, 1.0 + 0.2 * f64::from(500 - i))])
        .collect();
    let pieces = timed("fracture (nested)", || fracture(&operands))
        .unwrap()
        .pieces;
    assert_eq!(pieces.len(), 500, "499 rings and the inner disc");
    let visible = timed("flatten (nested)", || flatten(&operands)).unwrap();
    assert!(
        visible[..499]
            .iter()
            .all(|entry| matches!(entry, Visible::Trimmed(_)))
    );
    assert_eq!(visible[499], Visible::Untouched);
    let mut reversed = operands;
    reversed.reverse();
    let pieces = timed("fracture (nested, largest on top)", || fracture(&reversed))
        .unwrap()
        .pieces;
    assert_eq!(pieces.len(), 500);
    let visible = timed("flatten (nested, largest on top)", || flatten(&reversed)).unwrap();
    assert!(visible[..499].iter().all(|entry| *entry == Visible::Hidden));
}

/// Criterion 17 (c): the fixtures of `0016` criterion 40 give a result or a refusal, in time.
#[test]
fn the_boolean_fixtures_give_a_result_or_a_refusal() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/boolean");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "fixture"))
        .collect();
    files.sort();
    assert!(files.len() >= 8, "fixtures are missing");
    for file in files {
        let text = normalise(&std::fs::read_to_string(&file).unwrap());
        let operands = parse_operands(input_part(&text));
        let name = file.file_name().unwrap().to_string_lossy().to_string();
        match timed(&name, || fracture(&operands)) {
            Ok(_) | Err(StackError::Operands(_) | StackError::TooManyPieces) => {}
        }
        let _ = timed(&name, || flatten(&operands));
    }
}

/// A small deterministic generator (a linear congruential one), so the check needs no crate.
struct Seeded(u64);

impl Seeded {
    fn below(&mut self, n: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) % n
    }
}

/// Criterion 7: the pieces add up to the union, and no two pieces share area; the visible
/// regions of Flatten add up to the union too.
#[test]
fn pieces_and_visible_regions_tile_the_union() {
    let mut random = Seeded(37);
    for _ in 0..12 {
        let operands: Vec<Operand> = (0..5)
            .map(|_| {
                let (x, y) = (random.below(30) as f64, random.below(30) as f64);
                let side = 5.0 + random.below(20) as f64;
                vec![rect(x, y, x + side, y + side)]
            })
            .collect();
        let union = with_operands(&operands, |o| {
            area(
                boolean(BooleanOp::Union, o, KERNEL_TOLERANCE)
                    .unwrap()
                    .outlines(),
            )
        });
        let Ok(result) = fracture(&operands) else {
            panic!("fracture refused");
        };
        let total: f64 = result
            .untouched
            .iter()
            .map(|&i| operand_area(&operands[i]))
            .chain(result.pieces.iter().map(|(_, outlines)| area(outlines)))
            .sum();
        assert!(
            (total - union).abs() < 1e-3,
            "pieces {total} vs union {union}"
        );
        for (i, a) in result.pieces.iter().enumerate() {
            for b in &result.pieces[i + 1..] {
                let pair = [as_operand(&a.1), as_operand(&b.1)];
                let shared = with_operands(&pair, |o| {
                    boolean(BooleanOp::Intersection, o, KERNEL_TOLERANCE)
                });
                assert!(shared.is_err(), "two pieces share area");
            }
        }
        let visible: f64 = flatten(&operands)
            .unwrap()
            .iter()
            .zip(&operands)
            .map(|(entry, operand)| match entry {
                Visible::Untouched => operand_area(operand),
                Visible::Trimmed(outlines) => area(outlines),
                Visible::Hidden => 0.0,
            })
            .sum();
        assert!(
            (visible - union).abs() < 1e-3,
            "visible {visible} vs union {union}"
        );
    }
}
