//! Property tests of the centre lines (ADR 0063 §12): a permuted ring start gives a bit-identical
//! result. The rotation and scale invariance properties are not in yet (see the story's plan).

#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::unwrap_used, missing_docs)]

use curvyo_document_core::{Point, Vec2};
use curvyo_geometry_core::{CentreLines, CentreOptions, CentreSkeleton, Outline};
use proptest::prelude::*;

type Triple = (Point, Vec2, Vec2);

/// A T of a 40 x 2 bar and a 2 x 20 stem, as one outline.
fn t_shape() -> Vec<Vec<(f64, f64)>> {
    vec![vec![
        (-20.0, -1.0),
        (20.0, -1.0),
        (20.0, 1.0),
        (1.0, 1.0),
        (1.0, 20.0),
        (-1.0, 20.0),
        (-1.0, 1.0),
        (-20.0, 1.0),
    ]]
}

fn place(rings: &[Vec<(f64, f64)>], angle: f64, scale: f64, dx: f64, dy: f64) -> Vec<Vec<Triple>> {
    let (s, c) = angle.sin_cos();
    rings
        .iter()
        .map(|ring| {
            ring.iter()
                .map(|&(x, y)| {
                    let p = Point::new(scale * (c * x - s * y) + dx, scale * (s * x + c * y) + dy);
                    (p, Vec2::ZERO, Vec2::ZERO)
                })
                .collect()
        })
        .collect()
}

fn run(rings: &[Vec<Triple>]) -> CentreLines {
    let outlines: Vec<Outline<'_>> = rings.iter().map(|r| Outline::new(r, true)).collect();
    CentreSkeleton::build(&outlines)
        .unwrap()
        .finish(&CentreOptions::default(), &[])
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    #[test]
    fn a_permuted_ring_start_gives_the_same_result(start in 0_usize..8) {
        let mut rings = place(&t_shape(), 0.3, 1.0, 5.0, 5.0);
        let a = format!("{:?}", run(&rings).paths);
        rings[0].rotate_left(start);
        prop_assert_eq!(a, format!("{:?}", run(&rings).paths));
    }
}
