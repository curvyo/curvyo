//! Tester acceptance cases for `specs/0121-print-and-cut` slice 1, the fit (criteria 11, 12, 15;
//! ADR 0081 §6), written from the specification before the implementation was read.
//!
//! The reference fit here is independent of `point_fit.rs`: it solves the 4 x 4 normal equations
//! of `x' = a·x − b·y + tx`, `y' = b·x + a·y + ty` (and the reflected form `x' = a·x + b·y + tx`,
//! `y' = b·x − a·y + ty`) by Gaussian elimination with partial pivoting, without removing the
//! centroids first.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    missing_docs
)]

use curvyo_document_core::{Affine, Length, Point};
use curvyo_geometry_core::{FitError, PointPair, fit_similarity};

const SPREAD: Length = Length::from_mm(1.0);

/// Solves `m · x = r` (4 x 4) by Gaussian elimination with partial pivoting.
fn solve4(mut m: [[f64; 4]; 4], mut r: [f64; 4]) -> [f64; 4] {
    for col in 0..4 {
        let pivot = (col..4)
            .max_by(|&i, &j| m[i][col].abs().partial_cmp(&m[j][col].abs()).unwrap())
            .unwrap();
        m.swap(col, pivot);
        r.swap(col, pivot);
        assert!(m[col][col].abs() > 1e-12, "singular reference system");
        for row in col + 1..4 {
            let f = m[row][col] / m[col][col];
            for k in col..4 {
                m[row][k] -= f * m[col][k];
            }
            r[row] -= f * r[col];
        }
    }
    let mut x = [0.0; 4];
    for row in (0..4).rev() {
        let mut s = r[row];
        for k in row + 1..4 {
            s -= m[row][k] * x[k];
        }
        x[row] = s / m[row][row];
    }
    x
}

/// The least-squares similarity (`mirror` false) or reflected similarity (`mirror` true) as an
/// `Affine`, from the normal equations of the residual rows.
fn reference(pairs: &[(Point, Point)], mirror: bool) -> Affine {
    // Each pair gives two rows: [ru; rv] · [a, b, tx, ty] = [qx; qy].
    let mut ata = [[0.0; 4]; 4];
    let mut atq = [0.0; 4];
    for &(p, q) in pairs {
        let (ru, rv) = if mirror {
            ([p.x, p.y, 1.0, 0.0], [-p.y, p.x, 0.0, 1.0])
        } else {
            ([p.x, -p.y, 1.0, 0.0], [p.y, p.x, 0.0, 1.0])
        };
        for i in 0..4 {
            for j in 0..4 {
                ata[i][j] += ru[i] * ru[j] + rv[i] * rv[j];
            }
            atq[i] += ru[i] * q.x + rv[i] * q.y;
        }
    }
    let [a, b, tx, ty] = solve4(ata, atq);
    if mirror {
        // x' = a x + b y, y' = b x - a y  ->  Affine(a, b, c=b, d=-a)
        Affine::new(a, b, b, -a, tx, ty)
    } else {
        Affine::new(a, b, -b, a, tx, ty)
    }
}

fn residuals(map: Affine, pairs: &[(Point, Point)]) -> Vec<f64> {
    pairs
        .iter()
        .map(|&(p, q)| {
            let m = map.apply_point(p);
            (m.x - q.x).hypot(m.y - q.y)
        })
        .collect()
}

fn rms(r: &[f64]) -> f64 {
    (r.iter().map(|v| v * v).sum::<f64>() / r.len() as f64).sqrt()
}

fn pairs_of(raw: &[(Point, Point)]) -> Vec<PointPair> {
    raw.iter()
        .map(|&(from, to)| PointPair { from, to })
        .collect()
}

/// A similarity: turn by `deg` (the `Affine` sense: b = sin), scale `s`, then move by `(tx, ty)`.
fn similarity(deg: f64, s: f64, tx: f64, ty: f64) -> Affine {
    let (sin, cos) = deg.to_radians().sin_cos();
    Affine::new(s * cos, s * sin, -s * sin, s * cos, tx, ty)
}

fn sheet_marks(n: usize) -> Vec<Point> {
    [
        Point::new(10.0, 10.0),
        Point::new(200.0, 10.0),
        Point::new(200.0, 287.0),
        Point::new(10.0, 287.0),
    ][..n]
        .to_vec()
}

fn mapped(map: Affine, sheet: &[Point], noise: &[(f64, f64)]) -> Vec<(Point, Point)> {
    sheet
        .iter()
        .enumerate()
        .map(|(i, &p)| {
            let q = map.apply_point(p);
            let (nx, ny) = noise.get(i).copied().unwrap_or((0.0, 0.0));
            (p, Point::new(q.x + nx, q.y + ny))
        })
        .collect()
}

fn assert_map_close(got: Affine, want: Affine, tol: f64, what: &str) {
    assert!(
        got.approx_eq(want, tol),
        "{what}: got {got:?}, want {want:?}"
    );
}

// ---- criterion 11, 12: exact transforms with 2, 3 and 4 marks --------------------------------

#[test]
fn c11_c12_exact_similarity_with_2_3_and_4_marks_has_no_residual() {
    let truth = similarity(-17.3, 1.012, 83.4, 41.2);
    for n in 2..=4 {
        let raw = mapped(truth, &sheet_marks(n), &[]);
        let fit = fit_similarity(&pairs_of(&raw), SPREAD).unwrap();
        assert_map_close(fit.map, truth, 1e-9, &format!("{n} marks"));
        assert_eq!(fit.residuals.len(), n);
        for r in &fit.residuals {
            assert!(r.as_mm() < 1e-6, "{n} marks residual {}", r.as_mm());
        }
        assert!(fit.largest.as_mm() < 1e-6);
        assert!(fit.rms.as_mm() < 1e-6);
        assert!((fit.scale - 1.012).abs() < 1e-9, "scale {}", fit.scale);
        assert!(
            (fit.angle.as_radians() - (-17.3_f64).to_radians()).abs() < 1e-9,
            "angle {}",
            fit.angle.as_radians()
        );
    }
}

#[test]
fn c11_two_marks_fit_exactly_even_with_noise() {
    let truth = similarity(33.0, 0.97, -12.0, 300.0);
    let raw = mapped(truth, &sheet_marks(2), &[(0.3, -0.2), (-0.4, 0.15)]);
    let fit = fit_similarity(&pairs_of(&raw), SPREAD).unwrap();
    assert!(fit.largest.as_mm() < 1e-9, "{}", fit.largest.as_mm());
    assert!(fit.rms.as_mm() < 1e-9);
    for (p, q) in raw {
        let m = fit.map.apply_point(p);
        assert!((m.x - q.x).abs() < 1e-9 && (m.y - q.y).abs() < 1e-9);
    }
    assert!(fit.mirrored_rms.is_none(), "no mirror check with 2 marks");
}

#[test]
fn c12_fixed_noise_gives_the_reference_residuals_within_a_micrometre_of_a_micrometre() {
    let truth = similarity(1.8, 1.001, 83.4, 41.2);
    for n in 3..=4 {
        let noise = [(0.2, 0.0), (0.0, -0.2), (-0.2, 0.0), (0.0, 0.2)];
        // Asymmetric noise so the fit moves away from the truth.
        let noise = [noise[0], noise[1], (0.2, 0.2), noise[3]];
        let raw = mapped(truth, &sheet_marks(n), &noise[..n]);
        let want = reference(&raw, false);
        let fit = fit_similarity(&pairs_of(&raw), SPREAD).unwrap();
        assert_map_close(fit.map, want, 1e-9, &format!("{n} noisy marks"));
        let r = residuals(want, &raw);
        for (got, want) in fit.residuals.iter().zip(&r) {
            assert!(
                (got.as_mm() - want).abs() < 1e-6,
                "{} vs {want}",
                got.as_mm()
            );
        }
        let largest = r.iter().copied().fold(0.0, f64::max);
        assert!((fit.largest.as_mm() - largest).abs() < 1e-6);
        assert!((fit.rms.as_mm() - rms(&r)).abs() < 1e-6);
        assert!(largest > 0.01, "the noise shows: {largest}");
        let mirrored = residuals(reference(&raw, true), &raw);
        let got = fit.mirrored_rms.expect("3 or more pairs").as_mm();
        assert!(
            (got - rms(&mirrored)).abs() < 1e-6,
            "{got} vs {}",
            rms(&mirrored)
        );
    }
}

/// A deterministic random walk over many similarities and noisy marks (no proptest on wasm).
#[test]
fn c12_random_fits_match_the_reference() {
    let mut seed: u64 = 0x0121_0121;
    let mut next = || {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((seed >> 11) as f64) / ((1_u64 << 53) as f64)
    };
    for _ in 0..300 {
        let truth = similarity(
            next() * 360.0 - 180.0,
            0.6 + next(),
            next() * 800.0 - 400.0,
            next() * 800.0 - 400.0,
        );
        let n = 2 + (next() * 3.0) as usize;
        let sheet: Vec<Point> = (0..n)
            .map(|_| Point::new(next() * 300.0, next() * 300.0))
            .collect();
        let noise: Vec<(f64, f64)> = (0..n)
            .map(|_| (next() * 0.6 - 0.3, next() * 0.6 - 0.3))
            .collect();
        let raw = mapped(truth, &sheet, &noise);
        let Ok(fit) = fit_similarity(&pairs_of(&raw), SPREAD) else {
            continue;
        };
        let want = reference(&raw, false);
        assert_map_close(fit.map, want, 1e-7, "random");
        let r = residuals(want, &raw);
        assert!((fit.rms.as_mm() - rms(&r)).abs() < 1e-6);
    }
}

// ---- residual vs tolerance (the fit's part: largest and RMS are what is compared) ---------

#[test]
fn c13_largest_is_the_worst_mark_and_rms_the_mean_square() {
    let truth = similarity(0.0, 1.0, 50.0, 50.0);
    // Mark 3 measured 2 mm off: the largest residual is mark 3's.
    let raw = mapped(
        truth,
        &sheet_marks(4),
        &[(0.0, 0.0), (0.0, 0.0), (2.0, 0.0)],
    );
    let fit = fit_similarity(&pairs_of(&raw), SPREAD).unwrap();
    let worst = fit
        .residuals
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.as_mm().partial_cmp(&b.1.as_mm()).unwrap())
        .unwrap()
        .0;
    assert_eq!(worst, 2, "mark 3");
    assert_eq!(fit.largest.as_mm(), fit.residuals[worst].as_mm());
    assert!(fit.largest.as_mm() > 0.5, "above the 0.5 mm tolerance");
    let r: Vec<f64> = fit.residuals.iter().map(|l| l.as_mm()).collect();
    assert!((fit.rms.as_mm() - rms(&r)).abs() < 1e-12);
}

// ---- mirrored, collinear ------------------------------------------------------------------

#[test]
fn c14_reversed_order_of_four_marks_fits_better_mirrored() {
    let truth = similarity(5.0, 1.0, 100.0, 20.0);
    let sheet = sheet_marks(4);
    // Measured in reverse order: mark 1 where 4 is, 2 where 3 is, and so on (a mirror image).
    let raw: Vec<(Point, Point)> = vec![
        (sheet[0], truth.apply_point(sheet[3])),
        (sheet[1], truth.apply_point(sheet[2])),
        (sheet[2], truth.apply_point(sheet[1])),
        (sheet[3], truth.apply_point(sheet[0])),
    ];
    let fit = fit_similarity(&pairs_of(&raw), SPREAD).unwrap();
    let mirrored = fit.mirrored_rms.unwrap().as_mm();
    assert!(mirrored < 1e-6, "a reflection fits exactly: {mirrored}");
    assert!(
        fit.rms.as_mm() > 1.0,
        "the similarity does not: {}",
        fit.rms.as_mm()
    );
    let want = reference(&raw, false);
    assert_map_close(fit.map, want, 1e-9, "still the best similarity");
}

#[test]
fn c11_c15_collinear_marks_are_a_valid_fit_and_not_mirrored() {
    let truth = similarity(-30.0, 1.0, 10.0, 200.0);
    let sheet = [
        Point::new(10.0, 50.0),
        Point::new(100.0, 50.0),
        Point::new(190.0, 50.0),
    ];
    let raw = mapped(truth, &sheet, &[(0.0, 0.1), (0.0, -0.2), (0.05, 0.1)]);
    let fit = fit_similarity(&pairs_of(&raw), SPREAD).unwrap();
    assert_map_close(fit.map, reference(&raw, false), 1e-9, "collinear");
    let mirrored = fit.mirrored_rms.unwrap().as_mm();
    // A reflection about the line through the marks fits as well as the similarity, never
    // better by more than the 0.01 mm margin of ADR 0081 §6.
    assert!(
        mirrored + 0.01 >= fit.rms.as_mm(),
        "mirrored {mirrored} vs rms {}",
        fit.rms.as_mm()
    );
}

// ---- too few, too close, duplicates, repeated measures -------------------------------------

#[test]
fn c15_fewer_than_two_pairs_is_an_error() {
    assert_eq!(
        fit_similarity(&[], SPREAD).err(),
        Some(FitError::TooFewPairs)
    );
    let one = [PointPair {
        from: Point::new(0.0, 0.0),
        to: Point::new(5.0, 5.0),
    }];
    assert_eq!(
        fit_similarity(&one, SPREAD).err(),
        Some(FitError::TooFewPairs)
    );
}

#[test]
fn c15_all_marks_within_1_mm_are_too_close() {
    let raw = [
        (Point::new(10.0, 10.0), Point::new(50.0, 50.0)),
        (Point::new(10.6, 10.0), Point::new(150.0, 50.0)),
        (Point::new(10.0, 10.7), Point::new(150.0, 150.0)),
    ];
    assert_eq!(
        fit_similarity(&pairs_of(&raw), SPREAD).err(),
        Some(FitError::TooClose),
        "all sheet marks within 1 mm"
    );
}

#[test]
fn c15_all_measures_within_1_mm_are_too_close() {
    let raw = [
        (Point::new(10.0, 10.0), Point::new(50.0, 50.0)),
        (Point::new(200.0, 10.0), Point::new(50.4, 50.0)),
        (Point::new(200.0, 287.0), Point::new(50.0, 50.6)),
        (Point::new(10.0, 287.0), Point::new(50.3, 50.3)),
    ];
    assert_eq!(
        fit_similarity(&pairs_of(&raw), SPREAD).err(),
        Some(FitError::TooClose),
        "all measures within 1 mm"
    );
    // Two exactly coincident pairs: also too close.
    let same = [raw[0], raw[0]];
    assert_eq!(
        fit_similarity(&pairs_of(&same), SPREAD).err(),
        Some(FitError::TooClose)
    );
}

#[test]
fn c15_a_repeated_measure_of_one_mark_is_still_a_fit() {
    // Marks 1 and 2 measured at the same place (the maker forgot to move): the fit exists and
    // shows a large error; the "same place" warning is the laser side's job.
    let truth = similarity(0.0, 1.0, 20.0, 20.0);
    let sheet = sheet_marks(4);
    let raw = vec![
        (sheet[0], truth.apply_point(sheet[0])),
        (sheet[1], truth.apply_point(sheet[0])),
        (sheet[2], truth.apply_point(sheet[2])),
        (sheet[3], truth.apply_point(sheet[3])),
    ];
    let fit = fit_similarity(&pairs_of(&raw), SPREAD).unwrap();
    assert_map_close(fit.map, reference(&raw, false), 1e-9, "repeated measure");
    assert!(fit.largest.as_mm() > 10.0);
}

#[test]
fn c15_a_duplicated_pair_weights_but_does_not_break_the_fit() {
    let truth = similarity(12.0, 1.0, -5.0, 7.0);
    let sheet = sheet_marks(3);
    let mut raw = mapped(truth, &sheet, &[(0.1, 0.0), (0.0, 0.1), (-0.1, -0.1)]);
    raw.push(raw[0]);
    let fit = fit_similarity(&pairs_of(&raw), SPREAD).unwrap();
    assert_map_close(fit.map, reference(&raw, false), 1e-9, "duplicate");
    assert_eq!(fit.residuals.len(), 4);
    assert!((fit.residuals[0].as_mm() - fit.residuals[3].as_mm()).abs() < 1e-12);
}

#[test]
fn c15_the_fit_is_deterministic_bit_for_bit() {
    let truth = similarity(1.8, 1.001, 83.4, 41.2);
    let raw = mapped(
        truth,
        &sheet_marks(4),
        &[(0.2, 0.0), (0.0, -0.2), (0.2, 0.2)],
    );
    let a = fit_similarity(&pairs_of(&raw), SPREAD).unwrap();
    let b = fit_similarity(&pairs_of(&raw), SPREAD).unwrap();
    assert_eq!(
        a.map.to_array().map(f64::to_bits),
        b.map.to_array().map(f64::to_bits)
    );
}
