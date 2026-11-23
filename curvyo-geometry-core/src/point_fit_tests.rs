//! Tests of the similarity fit (ADR 0081 Verification; `specs/0121-print-and-cut` criteria 11,
//! 12, 15).

#![allow(clippy::many_single_char_names, clippy::needless_range_loop)]

use super::*;

const SPREAD: Length = Length::from_mm(1.0);
/// The noise-free residual bound of criterion 12.
const EXACT_MM: f64 = 1e-6;

/// The corners of a 200 x 150 mm sheet, 10 mm inside, clockwise from the top-left.
fn sheet_marks() -> [Point; 4] {
    [
        Point::new(10.0, 10.0),
        Point::new(190.0, 10.0),
        Point::new(190.0, 140.0),
        Point::new(10.0, 140.0),
    ]
}

fn similarity(degrees: f64, scale: f64, dx: f64, dy: f64) -> Affine {
    let (s, c) = degrees.to_radians().sin_cos();
    Affine::new(scale * c, scale * s, -scale * s, scale * c, dx, dy)
}

fn pairs_under(map: Affine, from: &[Point]) -> Vec<PointPair> {
    from.iter()
        .map(|&p| PointPair {
            from: p,
            to: map.apply_point(p),
        })
        .collect()
}

#[test]
fn two_pairs_fit_exactly_with_zero_residual() {
    let map = similarity(90.0, 1.0, 120.0, 30.0);
    let fit = fit_similarity(&pairs_under(map, &sheet_marks()[..2]), SPREAD).unwrap();
    assert!(fit.largest.as_mm() < EXACT_MM, "{fit:?}");
    assert!(fit.map.approx_eq(map, 1e-9));
    assert_eq!(fit.mirrored_rms, None);
}

#[test]
fn three_and_four_pairs_recover_a_known_map_without_noise() {
    let map = similarity(1.8, 1.001, 83.4, 41.2);
    for n in [3, 4] {
        let fit = fit_similarity(&pairs_under(map, &sheet_marks()[..n]), SPREAD).unwrap();
        assert!(fit.largest.as_mm() < EXACT_MM, "{n}: {fit:?}");
        assert!(fit.rms.as_mm() < EXACT_MM);
        assert!(fit.map.approx_eq(map, 1e-9));
        assert!((fit.scale - 1.001).abs() < 1e-12);
        assert!((fit.angle.as_radians() - 1.8_f64.to_radians()).abs() < 1e-12);
        assert_eq!(fit.residuals.len(), n);
    }
}

/// The least-squares similarity by the normal equations in (a, b, e, f), solved by Gaussian
/// elimination: an independent route to the residuals of criterion 12.
fn reference_residuals(pairs: &[PointPair]) -> Vec<f64> {
    let mut m = [[0.0_f64; 5]; 4];
    for p in pairs {
        let (x, y, u, v) = (p.from.x, p.from.y, p.to.x, p.to.y);
        // u = a x − b y + e, v = b x + a y + f
        for (row, target) in [([x, -y, 1.0, 0.0], u), ([y, x, 0.0, 1.0], v)] {
            for i in 0..4 {
                for j in 0..4 {
                    m[i][j] += row[i] * row[j];
                }
                m[i][4] += row[i] * target;
            }
        }
    }
    for col in 0..4 {
        let pivot = (col..4)
            .max_by(|&i, &j| m[i][col].abs().total_cmp(&m[j][col].abs()))
            .unwrap();
        m.swap(col, pivot);
        for row in 0..4 {
            if row != col {
                let k = m[row][col] / m[col][col];
                for j in 0..5 {
                    m[row][j] -= k * m[col][j];
                }
            }
        }
    }
    let s: Vec<f64> = (0..4).map(|i| m[i][4] / m[i][i]).collect();
    pairs
        .iter()
        .map(|p| {
            let u = s[0] * p.from.x - s[1] * p.from.y + s[2];
            let v = s[1] * p.from.x + s[0] * p.from.y + s[3];
            (u - p.to.x).hypot(v - p.to.y)
        })
        .collect()
}

#[test]
fn fixed_noise_gives_the_exact_residuals() {
    let map = similarity(-12.5, 0.997, 40.0, -15.0);
    let noise = [(0.2, 0.0), (0.0, -0.2), (-0.2, 0.0), (0.0, 0.2)];
    for n in [3, 4] {
        let mut pairs = pairs_under(map, &sheet_marks()[..n]);
        for (pair, (dx, dy)) in pairs.iter_mut().zip(noise) {
            pair.to = Point::new(pair.to.x + dx, pair.to.y + dy);
        }
        let fit = fit_similarity(&pairs, SPREAD).unwrap();
        let expected = reference_residuals(&pairs);
        for (got, want) in fit.residuals.iter().zip(&expected) {
            assert!((got.as_mm() - want).abs() < EXACT_MM, "{got:?} vs {want}");
        }
        let largest = expected.iter().copied().fold(0.0, f64::max);
        assert!((fit.largest.as_mm() - largest).abs() < EXACT_MM);
        #[allow(clippy::cast_precision_loss)]
        let rms = (expected.iter().map(|r| r * r).sum::<f64>() / n as f64).sqrt();
        assert!((fit.rms.as_mm() - rms).abs() < EXACT_MM);
        assert!(fit.largest.as_mm() > 0.01, "the noise shows");
    }
}

#[test]
fn fewer_than_two_pairs_is_refused() {
    let one = pairs_under(
        Affine::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
        &sheet_marks()[..1],
    );
    assert_eq!(fit_similarity(&one, SPREAD), Err(FitError::TooFewPairs));
    assert_eq!(fit_similarity(&[], SPREAD), Err(FitError::TooFewPairs));
}

#[test]
fn marks_or_measures_within_the_spread_are_too_close() {
    let close_marks = [
        PointPair {
            from: Point::new(10.0, 10.0),
            to: Point::new(0.0, 0.0),
        },
        PointPair {
            from: Point::new(10.5, 10.5),
            to: Point::new(100.0, 0.0),
        },
    ];
    assert_eq!(
        fit_similarity(&close_marks, SPREAD),
        Err(FitError::TooClose)
    );
    let close_measures = [
        PointPair {
            from: Point::new(10.0, 10.0),
            to: Point::new(50.0, 50.0),
        },
        PointPair {
            from: Point::new(190.0, 10.0),
            to: Point::new(50.3, 50.4),
        },
        PointPair {
            from: Point::new(190.0, 140.0),
            to: Point::new(50.1, 49.8),
        },
    ];
    assert_eq!(
        fit_similarity(&close_measures, SPREAD),
        Err(FitError::TooClose)
    );
}

#[test]
fn a_repeated_measure_of_one_mark_still_fits() {
    let map = similarity(5.0, 1.0, 10.0, 20.0);
    let marks = sheet_marks();
    let mut pairs = pairs_under(map, &[marks[0], marks[1], marks[1]]);
    pairs[2].to = Point::new(pairs[2].to.x + 0.1, pairs[2].to.y);
    let fit = fit_similarity(&pairs, SPREAD).unwrap();
    assert!(fit.largest.as_mm() < 0.1);
    assert!(fit.largest.as_mm() > 0.0);
}

#[test]
fn reversed_order_on_four_marks_is_mirrored() {
    let map = similarity(3.0, 1.0, 60.0, 30.0);
    let marks = sheet_marks();
    let pairs: Vec<PointPair> = (0..4)
        .map(|i| PointPair {
            from: marks[i],
            to: map.apply_point(marks[3 - i]),
        })
        .collect();
    let fit = fit_similarity(&pairs, SPREAD).unwrap();
    let mirrored = fit.mirrored_rms.unwrap();
    assert!(mirrored.as_mm() < EXACT_MM, "{fit:?}");
    assert!(fit.rms.as_mm() > 1.0);
}

#[test]
fn collinear_marks_fit_and_do_not_look_mirrored() {
    let map = similarity(20.0, 1.0, 5.0, 5.0);
    let line = [
        Point::new(10.0, 10.0),
        Point::new(100.0, 10.0),
        Point::new(190.0, 10.0),
    ];
    let mut pairs = pairs_under(map, &line);
    pairs[1].to = Point::new(pairs[1].to.x, pairs[1].to.y + 0.1);
    let fit = fit_similarity(&pairs, SPREAD).unwrap();
    let mirrored = fit.mirrored_rms.unwrap();
    assert!((mirrored.as_mm() - fit.rms.as_mm()).abs() < 1e-9, "{fit:?}");
}

#[test]
fn the_matrix_bits_are_pinned() {
    let marks = sheet_marks();
    let measured = [
        Point::new(83.4, 41.2),
        Point::new(263.3, 46.9),
        Point::new(259.1, 176.8),
        Point::new(79.3, 171.0),
    ];
    let pairs: Vec<PointPair> = marks
        .iter()
        .zip(measured)
        .map(|(&from, to)| PointPair { from, to })
        .collect();
    let bits = fit_similarity(&pairs, SPREAD)
        .unwrap()
        .map
        .to_array()
        .map(f64::to_bits);
    assert_eq!(bits, PINNED_BITS, "{bits:#x?}");
}

/// The matrix of [`the_matrix_bits_are_pinned`], native and wasm32 alike.
const PINNED_BITS: [u64; 6] = [
    0x3fef_f845_f4be_02de,
    0x3fa0_5a0f_e911_ad0b,
    0xbfa0_5a0f_e911_ad0b,
    0x3fef_f845_f4be_02de,
    0x4052_70ef_478e_2d13,
    0x403e_da1e_8876_cf14,
];

#[cfg(not(target_arch = "wasm32"))]
mod property {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn exact_similarities_are_recovered(
            degrees in -180.0..180.0_f64,
            scale in 0.5..2.0_f64,
            dx in -500.0..500.0_f64,
            dy in -500.0..500.0_f64,
            n in 2..=4_usize,
            jitter in proptest::collection::vec((-20.0..20.0_f64, -20.0..20.0_f64), 4),
        ) {
            let map = similarity(degrees, scale, dx, dy);
            let from: Vec<Point> = sheet_marks()
                .iter()
                .zip(&jitter)
                .take(n)
                .map(|(p, (jx, jy))| Point::new(p.x + jx, p.y + jy))
                .collect();
            let fit = fit_similarity(&pairs_under(map, &from), SPREAD).unwrap();
            prop_assert!(fit.largest.as_mm() < EXACT_MM);
            prop_assert!(fit.map.approx_eq(map, 1e-8));
            prop_assert!((fit.scale - scale).abs() < 1e-9);
            if let Some(mirrored) = fit.mirrored_rms {
                prop_assert!(mirrored.as_mm() > fit.rms.as_mm());
            }
        }
    }
}
