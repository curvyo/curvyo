//! Tester acceptance cases for `specs/0135-image-optimisation` slice 1 in `curvyo-image-core`,
//! written from the specification and ADR 0073 before the implementation was read: independent
//! reference implementations of Levels (exact), the analysis (percentile, median, Otsu, noise;
//! exact or by the spec's thresholds), and floating-point references of the blur, Sharpen, Local
//! contrast (CLAHE), the bilateral Denoise and Contours (within stated tolerances, since the
//! production steps are integer approximations by design, criterion 17); determinism (stepped
//! run equals the one-shot run, two runs byte-identical) and edge pictures (1 x 1, all white, all
//! black, transparent, extreme fields).
#![allow(
    clippy::assert_is_empty,
    clippy::many_single_char_names,
    clippy::trivially_copy_pass_by_ref,
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::cast_possible_wrap,
    clippy::needless_range_loop,
    clippy::float_cmp,
    clippy::too_many_lines,
    missing_docs
)]

use curvyo_image_core::{
    AdjustRun, Adjusted, Analysis, GreyPlane, Histogram, PixelStep, SubPx, WorkingImage,
    gauss_blur, levels_table, noise_tenths, run_steps,
};

// ---- fixtures ---------------------------------------------------------------------------------

fn image(w: u32, h: u32, mut px: impl FnMut(u32, u32) -> [u8; 4]) -> WorkingImage {
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            pixels.extend_from_slice(&px(x, y));
        }
    }
    WorkingImage {
        width: w,
        height: h,
        pixels,
        original_size: (w, h),
        truncated: false,
        animated: false,
    }
}

fn grey_image(w: u32, h: u32, mut g: impl FnMut(u32, u32) -> u8) -> WorkingImage {
    image(w, h, |x, y| {
        let v = g(x, y);
        [v, v, v, 255]
    })
}

fn plane(w: u32, h: u32, mut g: impl FnMut(u32, u32) -> u8) -> GreyPlane {
    let mut grey = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            grey.push(g(x, y));
        }
    }
    GreyPlane {
        width: w,
        height: h,
        grey,
    }
}

struct Lcg(u32);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0 >> 8
    }
    /// -`spread` ..= `spread`.
    fn spread(&mut self, spread: i32) -> i32 {
        (self.next() % (2 * spread as u32 + 1)) as i32 - spread
    }
}

fn clamp8(v: i64) -> u8 {
    v.clamp(0, 255) as u8
}

/// A textured test picture: a soft disc, a gradient, a few hard edges and grain.
fn textured(w: u32, h: u32, seed: u32) -> GreyPlane {
    let mut r = Lcg(seed);
    plane(w, h, |x, y| {
        let base = 40 + (x * 160 / w.max(1)) as i64;
        let dx = x as i64 - i64::from(w) / 2;
        let dy = y as i64 - i64::from(h) / 2;
        let disc = if dx * dx + dy * dy < i64::from(w * w) / 16 {
            60
        } else {
            0
        };
        let bar = if (x / 5) % 4 == 0 { -30 } else { 0 };
        clamp8(base + disc + bar + i64::from(r.spread(6)))
    })
}

fn grey_of(a: &Adjusted, src: &WorkingImage) -> Vec<u8> {
    match a {
        Adjusted::Grey(p) => p.grey.clone(),
        Adjusted::Unchanged => GreyPlane::of_rgba(src).grey,
        Adjusted::Rgba(img) => GreyPlane::of_rgba(img).grey,
    }
}

fn to_image(p: &GreyPlane) -> WorkingImage {
    grey_image(p.width, p.height, |x, y| p.grey[(y * p.width + x) as usize])
}

fn stats(a: &[u8], b: &[u8]) -> (f64, u8) {
    assert_eq!(a.len(), b.len());
    let mut sum = 0u64;
    let mut max = 0u8;
    for (&x, &y) in a.iter().zip(b) {
        let d = x.abs_diff(y);
        sum += u64::from(d);
        max = max.max(d);
    }
    (sum as f64 / a.len() as f64, max)
}

// ---- float references (edge-clamped) ----------------------------------------------------------

fn at(p: &[f64], w: usize, h: usize, x: i64, y: i64) -> f64 {
    let x = x.clamp(0, w as i64 - 1) as usize;
    let y = y.clamp(0, h as i64 - 1) as usize;
    p[y * w + x]
}

fn gauss_ref(p: &GreyPlane, sigma: f64) -> Vec<f64> {
    let w = p.width as usize;
    let h = p.height as usize;
    let src: Vec<f64> = p.grey.iter().map(|&g| f64::from(g)).collect();
    if sigma <= 0.0 {
        return src;
    }
    let r = (3.0 * sigma).ceil() as i64;
    let k: Vec<f64> = (-r..=r)
        .map(|i| (-(i * i) as f64 / (2.0 * sigma * sigma)).exp())
        .collect();
    let ks: f64 = k.iter().sum();
    let mut tmp = vec![0.0; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut s = 0.0;
            for i in -r..=r {
                s += k[(i + r) as usize] * at(&src, w, h, x as i64 + i, y as i64);
            }
            tmp[y * w + x] = s / ks;
        }
    }
    let mut out = vec![0.0; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut s = 0.0;
            for i in -r..=r {
                s += k[(i + r) as usize] * at(&tmp, w, h, x as i64, y as i64 + i);
            }
            out[y * w + x] = s / ks;
        }
    }
    out
}

/// Levels as the spec (criterion 5) and ADR 0073 §4 state it, the `0123` Gamma convention.
fn levels_ref(black_pct: u8, white_pct: u8, gamma_centi: u16) -> [u8; 256] {
    let b = (255 * i64::from(black_pct) + 50) / 100;
    let w = (255 * i64::from(white_pct) + 50) / 100;
    let gamma = f64::from(gamma_centi) / 100.0;
    let mut t = [0u8; 256];
    for g in 0..256i64 {
        let num = (g - b) * 255;
        let den = w - b;
        let x = if num <= 0 {
            0
        } else {
            ((num * 2 + den) / (2 * den)).clamp(0, 255)
        };
        let v = (255.0 * (x as f64 / 255.0).powf(1.0 / gamma)).round();
        t[g as usize] = v as u8;
    }
    t
}

fn sharpen_ref(p: &GreyPlane, sigma: f64, amount: f64, threshold_pct: f64) -> Vec<u8> {
    let b = gauss_ref(p, sigma);
    p.grey
        .iter()
        .zip(&b)
        .map(|(&g, &bl)| {
            let d = f64::from(g) - bl;
            if 100.0 * d.abs() < threshold_pct * 255.0 {
                g
            } else {
                clamp8((f64::from(g) + d * amount / 100.0).round() as i64)
            }
        })
        .collect()
}

fn bilateral_ref(p: &GreyPlane, strength: f64, detail: f64) -> Vec<u8> {
    let w = p.width as usize;
    let h = p.height as usize;
    let src: Vec<f64> = p.grey.iter().map(|&g| f64::from(g)).collect();
    let sr = 4.0 + (100.0 - detail) * 0.6;
    let mut out = Vec::with_capacity(w * h);
    for y in 0..h as i64 {
        for x in 0..w as i64 {
            let c = at(&src, w, h, x, y);
            let mut ws = 0.0;
            let mut vs = 0.0;
            for j in -2..=2i64 {
                for i in -2..=2i64 {
                    let v = at(&src, w, h, x + i, y + j);
                    let spatial = (-((i * i + j * j) as f64) / (2.0 * 1.5 * 1.5)).exp();
                    let d = v - c;
                    let range = (-(d * d) / (2.0 * sr * sr)).exp();
                    ws += spatial * range;
                    vs += spatial * range * v;
                }
            }
            let f = vs / ws;
            out.push(clamp8((c + (f - c) * strength / 100.0).round() as i64));
        }
    }
    out
}

/// CLAHE as ADR 0073 §4 describes it: tiles of side 2 x radius (at least 8) from the top-left,
/// clip limit = mean bin x (1 + Amount / 25), excess spread evenly, bilinear blend of the four
/// nearest tile tables (centres), edge tiles clamp.
fn clahe_ref(p: &GreyPlane, radius_px: f64, amount: f64) -> Vec<u8> {
    let w = p.width as usize;
    let h = p.height as usize;
    let side = ((2.0 * radius_px).round() as usize).max(8);
    let tx = w.div_ceil(side);
    let ty = h.div_ceil(side);
    let mut tables = vec![[0.0f64; 256]; tx * ty];
    for j in 0..ty {
        for i in 0..tx {
            let mut hist = [0.0f64; 256];
            let mut n = 0.0;
            for y in j * side..((j + 1) * side).min(h) {
                for x in i * side..((i + 1) * side).min(w) {
                    hist[p.grey[y * w + x] as usize] += 1.0;
                    n += 1.0;
                }
            }
            // ADR 0073 §4 (amended 2026-11-20): the clip in real counts, the excess spread
            // evenly over all 256 bins in exact fractions.
            let limit = n / 256.0 * (1.0 + amount / 25.0);
            let mut excess = 0.0;
            for b in &mut hist {
                if *b > limit {
                    excess += *b - limit;
                    *b = limit;
                }
            }
            let each = excess / 256.0;
            let mut cum = 0.0;
            for g in 0..256 {
                cum += hist[g] + each;
                tables[j * tx + i][g] = cum * 255.0 / n;
            }
        }
    }
    let mut out = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let g = p.grey[y * w + x] as usize;
            let fx = (x as f64 + 0.5) / side as f64 - 0.5;
            let fy = (y as f64 + 0.5) / side as f64 - 0.5;
            let x0 = fx.floor();
            let y0 = fy.floor();
            let ax = fx - x0;
            let ay = fy - y0;
            let ci = |v: f64, n: usize| (v as i64).clamp(0, n as i64 - 1) as usize;
            let i0 = ci(x0, tx);
            let i1 = ci(x0 + 1.0, tx);
            let j0 = ci(y0, ty);
            let j1 = ci(y0 + 1.0, ty);
            let t = |i: usize, j: usize| tables[j * tx + i][g];
            let v = (1.0 - ay) * ((1.0 - ax) * t(i0, j0) + ax * t(i1, j0))
                + ay * ((1.0 - ax) * t(i0, j1) + ax * t(i1, j1));
            out.push(clamp8(v.round() as i64));
        }
    }
    out
}

fn contours_ref(p: &GreyPlane, sigma: f64, sensitivity: f64) -> Vec<u8> {
    let b1 = gauss_ref(p, sigma);
    let b2 = gauss_ref(p, 1.6 * sigma);
    b1.iter()
        .zip(&b2)
        .map(|(&a, &s)| {
            let d = s - a;
            if d > 0.0 {
                255 - clamp8((d * (100.0 + 15.0 * sensitivity) / 100.0).round() as i64)
            } else {
                255
            }
        })
        .collect()
}

fn hist_of(greys: &[u8]) -> Histogram {
    let mut bins = [0u32; 256];
    for &g in greys {
        bins[g as usize] += 1;
    }
    Histogram { bins }
}

fn percentile_ref(h: &Histogram, p_tenths: u16) -> u8 {
    let total: u64 = h.bins.iter().map(|&b| u64::from(b)).sum();
    let mut cum = 0u64;
    for g in 0..256 {
        cum += u64::from(h.bins[g]);
        // cum / total >= p / 1000
        if cum * 1000 >= u64::from(p_tenths) * total {
            return g as u8;
        }
    }
    255
}

/// Otsu with class 0 = greys <= t; the largest between-class variance, ties to the lower grey.
fn otsu_ref(h: &Histogram) -> u8 {
    let total: f64 = h.bins.iter().map(|&b| f64::from(b)).sum();
    let sum_all: f64 = (0..256).map(|g| g as f64 * f64::from(h.bins[g])).sum();
    let mut best = -1.0;
    let mut best_t = 0u8;
    let mut w0 = 0.0;
    let mut s0 = 0.0;
    for t in 0..256 {
        w0 += f64::from(h.bins[t]);
        s0 += t as f64 * f64::from(h.bins[t]);
        let w1 = total - w0;
        if w0 == 0.0 || w1 == 0.0 {
            continue;
        }
        let m0 = s0 / w0;
        let m1 = (sum_all - s0) / w1;
        let v = w0 * w1 * (m0 - m1) * (m0 - m1);
        if v > best * (1.0 + 1e-12) {
            best = v;
            best_t = t as u8;
        }
    }
    best_t
}

// ---- Levels (criterion 5): exact --------------------------------------------------------------

#[test]
fn c5_levels_table_equals_an_independent_reference_on_a_grid_of_fields() {
    let mut mismatches = Vec::new();
    for &(b, w) in &[
        (0u8, 100u8),
        (10, 90),
        (0, 2),
        (98, 100),
        (25, 27),
        (40, 60),
        (5, 100),
    ] {
        for &gm in &[10u16, 50, 83, 100, 132, 200, 400] {
            let got = levels_table(b, w, gm);
            let want = levels_ref(b, w, gm);
            for g in 0..256 {
                if got[g] != want[g] {
                    mismatches.push((b, w, gm, g, got[g], want[g]));
                }
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} mismatches, first: {:?}",
        mismatches.len(),
        &mismatches[..mismatches.len().min(8)]
    );
}

#[test]
fn c5_levels_identity_is_identity_and_gamma_above_one_lightens() {
    let id = levels_table(0, 100, 100);
    for g in 0..256 {
        assert_eq!(id[g], g as u8);
    }
    let light = levels_table(0, 100, 200);
    let dark = levels_table(0, 100, 50);
    assert!(light[128] > 128 && dark[128] < 128, "0123 convention");
    for g in 1..256 {
        assert!(
            light[g] >= light[g - 1] && dark[g] >= dark[g - 1],
            "monotone"
        );
    }
}

#[test]
fn c5_levels_runs_in_the_pipeline_as_the_table() {
    let src = textured(23, 17, 3);
    let img = to_image(&src);
    let out = run_steps(
        &img,
        &[PixelStep::Levels {
            black_pct: 10,
            white_pct: 85,
            gamma_centi: 140,
        }],
    );
    let t = levels_ref(10, 85, 140);
    let want: Vec<u8> = src.grey.iter().map(|&g| t[g as usize]).collect();
    assert_eq!(grey_of(&out, &img), want);
}

// ---- analysis (criterion 39) ------------------------------------------------------------------

#[test]
fn c39_percentile_median_and_otsu_equal_independent_references() {
    let mut r = Lcg(77);
    for case in 0..40 {
        let n = 50 + (case * 37) % 400;
        let greys: Vec<u8> = (0..n)
            .map(|_| {
                if case % 3 == 0 {
                    (r.next() % 256) as u8
                } else if case % 3 == 1 {
                    clamp8(
                        60 + i64::from(r.spread(20))
                            + if r.next().is_multiple_of(2) { 120 } else { 0 },
                    )
                } else {
                    clamp8(128 + i64::from(r.spread(3)))
                }
            })
            .collect();
        let h = hist_of(&greys);
        for &p in &[0u16, 2, 5, 20, 100, 500, 900, 980, 995, 998, 1000] {
            assert_eq!(h.percentile(p), percentile_ref(&h, p), "case {case} p {p}");
        }
        assert_eq!(h.median(), percentile_ref(&h, 500), "median case {case}");
        let o = h.otsu();
        let o_ref = otsu_ref(&h);
        assert_eq!(o, o_ref, "otsu case {case}");
    }
}

#[test]
fn c39_two_greys_give_the_otsu_level_between_them() {
    for &(a, b) in &[(10u8, 200u8), (0, 255), (100, 101), (30, 31)] {
        let greys: Vec<u8> = (0..1000).map(|i| if i % 3 == 0 { a } else { b }).collect();
        let o = hist_of(&greys).otsu();
        assert!(o >= a && o < b, "{a} {b} -> {o}");
    }
}

#[test]
fn c39_noise_estimate_ramp_below_one_and_noisy_ramp_above_three() {
    let ramp = plane(256, 64, |x, _| x as u8);
    let n = noise_tenths(&ramp);
    assert!(n < 10, "noise-free ramp {n} tenths");
    let mut r = Lcg(5);
    let noisy = plane(256, 64, |x, _| {
        clamp8(i64::from(x as u8) + i64::from(r.spread(8)))
    });
    let n2 = noise_tenths(&noisy);
    assert!(n2 > 30, "ramp +-8: {n2} tenths");
    // The Immerkær estimate of uniform +-8 noise is sigma = 8 / sqrt(3) ~ 4.9: a sane band.
    assert!((25..=80).contains(&n2), "{n2}");
    // Analysis reads the same estimate.
    let a = Analysis::of_plane(&noisy);
    assert_eq!(a.noise_tenths, n2);
    assert_eq!(a.histogram, Histogram::of(&noisy));
}

#[test]
fn c39_analysis_reads_at_most_one_megapixel_box_mip() {
    // 2,048 x 1,024 = 2 Mpx: the analysis reads its 1,024 x 512 halving.
    let big = grey_image(2048, 1024, |x, y| ((x + y) % 256) as u8);
    let a = Analysis::of(&big);
    assert!(a.histogram.total() <= 1_000_000, "{}", a.histogram.total());
    assert_eq!(a.histogram.total(), 1024 * 512);
}

#[test]
fn c39_c6_alpha_is_blended_over_white_before_the_analysis() {
    let clear = image(16, 16, |_, _| [0, 0, 0, 0]);
    let a = Analysis::of(&clear);
    assert_eq!(a.histogram.bins[255], 256, "transparent reads white");
}

// ---- blur, Sharpen, Local contrast, Denoise, Contours (criteria 5, 17, 45) -------------------

#[test]
fn c17_three_box_blur_approximates_the_gaussian() {
    let p = textured(48, 40, 9);
    for &s16 in &[16u32, 24, 40, 64] {
        let got = gauss_blur(&p, SubPx(s16));
        let want: Vec<u8> = gauss_ref(&p, f64::from(s16) / 16.0)
            .iter()
            .map(|v| clamp8(v.round() as i64))
            .collect();
        let (mean, max) = stats(&got.grey, &want);
        assert!(
            mean <= 1.5 && max <= 8,
            "sigma {s16}/16: mean {mean} max {max}"
        );
    }
    let flat = plane(20, 20, |_, _| 77);
    assert_eq!(
        gauss_blur(&flat, SubPx(80)).grey,
        flat.grey,
        "flat stays flat"
    );
}

#[test]
fn c5_sharpen_is_an_unsharp_mask_near_the_reference() {
    let p = textured(40, 32, 21);
    let img = to_image(&p);
    for &(s16, amount, thr) in &[
        (16u32, 100u16, 0u8),
        (24, 200, 0),
        (32, 50, 5),
        (16, 500, 0),
    ] {
        let got = grey_of(
            &run_steps(
                &img,
                &[PixelStep::Sharpen {
                    radius: SubPx(s16),
                    amount_pct: amount,
                    threshold_pct: thr,
                }],
            ),
            &img,
        );
        let want = sharpen_ref(&p, f64::from(s16) / 16.0, f64::from(amount), f64::from(thr));
        let (mean, max) = stats(&got, &want);
        let tol = 1.5 * f64::from(amount.max(100)) / 100.0;
        assert!(mean <= tol, "{s16} {amount} {thr}: mean {mean} max {max}");
    }
    // Amount 0 changes nothing; a flat picture is unchanged by any amount.
    let none = run_steps(
        &img,
        &[PixelStep::Sharpen {
            radius: SubPx(16),
            amount_pct: 0,
            threshold_pct: 0,
        }],
    );
    assert_eq!(grey_of(&none, &img), p.grey);
    let flat = grey_image(12, 12, |_, _| 140);
    let f = run_steps(
        &flat,
        &[PixelStep::Sharpen {
            radius: SubPx(40),
            amount_pct: 500,
            threshold_pct: 0,
        }],
    );
    assert!(grey_of(&f, &flat).iter().all(|&g| g == 140));
}

#[test]
fn c5_sharpen_threshold_keeps_flat_noise_flat() {
    // Grain of +-2 on flat grey: with threshold 10 % (25 greys) nothing changes.
    let mut r = Lcg(1);
    let p = plane(32, 32, |_, _| clamp8(128 + i64::from(r.spread(2))));
    let img = to_image(&p);
    let out = run_steps(
        &img,
        &[PixelStep::Sharpen {
            radius: SubPx(16),
            amount_pct: 300,
            threshold_pct: 10,
        }],
    );
    assert_eq!(grey_of(&out, &img), p.grey);
}

#[test]
fn c5_local_contrast_clahe_near_the_reference_and_widens_a_dull_picture() {
    let mut r = Lcg(4);
    // A dull picture: greys 100..140 in soft blobs.
    let p = plane(64, 48, |x, y| {
        clamp8(120 + ((x as i64 % 16) - 8) + ((y as i64 % 12) - 6) + i64::from(r.spread(3)))
    });
    let img = to_image(&p);
    for &(rad16, amount) in &[
        (8 * 16u32, 50u16),
        (12 * 16, 100),
        (6 * 16, 200),
        (20 * 16, 10),
    ] {
        let got = grey_of(
            &run_steps(
                &img,
                &[PixelStep::LocalContrast {
                    radius: SubPx(rad16),
                    amount_pct: amount,
                }],
            ),
            &img,
        );
        let want = clahe_ref(&p, f64::from(rad16) / 16.0, f64::from(amount));
        let (mean, max) = stats(&got, &want);
        // Partial edge tiles are centred on their own length (white-box), the reference on a
        // full side: the edge rows differ, the mean stays close.
        assert!(
            mean <= 4.0,
            "radius {} px amount {amount}: mean {mean} max {max}",
            rad16 / 16
        );
        let span =
            |v: &[u8]| i32::from(*v.iter().max().unwrap()) - i32::from(*v.iter().min().unwrap());
        assert!(
            span(&got) > span(&p.grey),
            "Local contrast widens a dull picture"
        );
    }
}

#[test]
fn c5_local_contrast_amount_zero_is_no_step() {
    let p = textured(30, 30, 2);
    let img = to_image(&p);
    let out = run_steps(
        &img,
        &[PixelStep::LocalContrast {
            radius: SubPx(160),
            amount_pct: 0,
        }],
    );
    assert_eq!(grey_of(&out, &img), p.grey);
}

#[test]
fn c5_denoise_is_a_bilateral_filter_near_the_reference() {
    let p = textured(24, 20, 13);
    let img = to_image(&p);
    for &(s, d) in &[(30u8, 60u8), (100, 0), (100, 100), (50, 50), (10, 90)] {
        let got = grey_of(
            &run_steps(
                &img,
                &[PixelStep::Denoise {
                    strength_pct: s,
                    detail_pct: d,
                }],
            ),
            &img,
        );
        let want = bilateral_ref(&p, f64::from(s), f64::from(d));
        let (mean, max) = stats(&got, &want);
        assert!(
            mean <= 0.6 && max <= 3,
            "strength {s} detail {d}: mean {mean} max {max}"
        );
    }
    let zero = run_steps(
        &img,
        &[PixelStep::Denoise {
            strength_pct: 0,
            detail_pct: 60,
        }],
    );
    assert_eq!(grey_of(&zero, &img), p.grey, "Strength 0 changes nothing");
}

#[test]
fn c5_c7_denoise_alone_runs_per_channel_and_keeps_colour() {
    let img = image(10, 10, |x, y| [(x * 25) as u8, (y * 25) as u8, 90, 255]);
    let out = run_steps(
        &img,
        &[PixelStep::Denoise {
            strength_pct: 50,
            detail_pct: 50,
        }],
    );
    match out {
        Adjusted::Rgba(i) => {
            assert_eq!((i.width, i.height), (10, 10));
            assert!(i.pixels.chunks(4).any(|c| c[0] != c[1]), "still colour");
            assert!(i.pixels.chunks(4).all(|c| c[3] == 255), "opaque");
        }
        other => panic!("Denoise alone before a grey step must stay colour, got {other:?}"),
    }
}

#[test]
fn c45_contours_turn_a_dark_line_into_a_dark_line_and_white_stays_white() {
    let white = grey_image(32, 32, |_, _| 255);
    let w = run_steps(
        &white,
        &[PixelStep::Contours {
            sigma: SubPx(24),
            sensitivity_pct: 80,
        }],
    );
    assert!(
        grey_of(&w, &white).iter().all(|&g| g == 255),
        "white stays white"
    );
    // A 2 px dark line on white at x = 15, 16.
    let p = plane(32, 32, |x, _| if x == 15 || x == 16 { 40 } else { 230 });
    let img = to_image(&p);
    let got = grey_of(
        &run_steps(
            &img,
            &[PixelStep::Contours {
                sigma: SubPx(16),
                sensitivity_pct: 50,
            }],
        ),
        &img,
    );
    let row = |x: usize| got[16 * 32 + x];
    assert!(
        row(15) < 128 && row(16) < 128,
        "line dark: {} {}",
        row(15),
        row(16)
    );
    assert_eq!(row(2), 255, "far from the line is white");
    let want = contours_ref(&p, 1.0, 50.0);
    let (mean, max) = stats(&got, &want);
    assert!(mean <= 4.0, "near the DoG reference: mean {mean} max {max}");
}

#[test]
fn c45_contours_on_textured_picture_near_the_reference() {
    let p = textured(40, 40, 31);
    let img = to_image(&p);
    for &(s16, sens) in &[(16u32, 30u8), (24, 80), (48, 0), (32, 100)] {
        let got = grey_of(
            &run_steps(
                &img,
                &[PixelStep::Contours {
                    sigma: SubPx(s16),
                    sensitivity_pct: sens,
                }],
            ),
            &img,
        );
        let want = contours_ref(&p, f64::from(s16) / 16.0, f64::from(sens));
        let (mean, max) = stats(&got, &want);
        // The gain is 1x to 16x (plan deviation), so the blur approximation is amplified.
        let tol = 2.0 + 0.3 * (100.0 + 15.0 * f64::from(sens)) / 100.0;
        assert!(
            mean <= tol,
            "sigma {s16}/16 sens {sens}: mean {mean} max {max}"
        );
    }
}

// ---- determinism (criteria 13, 17, 18) --------------------------------------------------------

fn photo_stack() -> Vec<PixelStep> {
    vec![
        PixelStep::Denoise {
            strength_pct: 25,
            detail_pct: 60,
        },
        PixelStep::Levels {
            black_pct: 3,
            white_pct: 97,
            gamma_centi: 115,
        },
        PixelStep::LocalContrast {
            radius: SubPx(10 * 16),
            amount_pct: 50,
        },
        PixelStep::Contours {
            sigma: SubPx(20),
            sensitivity_pct: 40,
        },
        PixelStep::Sharpen {
            radius: SubPx(16),
            amount_pct: 120,
            threshold_pct: 0,
        },
    ]
}

#[test]
fn c13_c17_a_stepped_run_equals_the_one_shot_run_for_every_work_size() {
    let img = image(53, 41, |x, y| {
        [
            (x * 4 + y) as u8,
            (y * 5) as u8,
            ((x * y) % 256) as u8,
            if x % 7 == 0 { 128 } else { 255 },
        ]
    });
    let steps = photo_stack();
    let once = run_steps(&img, &steps);
    assert_eq!(once, run_steps(&img, &steps), "two runs byte-identical");
    for &work in &[1u64, 7, 100, 1_000, 25_000, u64::MAX / 4] {
        let mut run = AdjustRun::new(&img, &steps);
        let mut guard = 0;
        while !run.advance(work) {
            guard += 1;
            assert!(guard < 10_000_000, "advance must finish");
        }
        assert_eq!(run.finish(), once, "work {work}");
    }
    // Finishing a run that was never advanced gives the same picture.
    assert_eq!(AdjustRun::new(&img, &steps).finish(), once);
}

#[test]
fn c1_an_empty_stack_is_the_working_image_unchanged() {
    let img = image(9, 7, |x, y| [x as u8, y as u8, 3, 100]);
    assert_eq!(run_steps(&img, &[]), Adjusted::Unchanged);
}

#[test]
fn c17_a_neutral_levels_step_reproduces_the_0123_grey() {
    let img = image(16, 16, |x, y| {
        [(x * 16) as u8, (y * 16) as u8, 200, (x * y) as u8]
    });
    let out = run_steps(
        &img,
        &[PixelStep::Levels {
            black_pct: 0,
            white_pct: 100,
            gamma_centi: 100,
        }],
    );
    assert_eq!(grey_of(&out, &img), GreyPlane::of_rgba(&img).grey);
}

// ---- edge pictures ----------------------------------------------------------------------------

#[test]
fn edge_one_pixel_runs_every_step() {
    for v in [0u8, 1, 128, 254, 255] {
        let img = grey_image(1, 1, |_, _| v);
        let out = run_steps(&img, &photo_stack());
        let g = grey_of(&out, &img);
        assert_eq!(g.len(), 1);
        let _ = Analysis::of(&img);
    }
}

#[test]
fn edge_all_white_and_all_black_never_crash_and_white_stays_white_under_sharpen_and_denoise() {
    let white = grey_image(40, 30, |_, _| 255);
    let black = grey_image(40, 30, |_, _| 0);
    for img in [&white, &black] {
        let _ = run_steps(img, &photo_stack());
        let a = Analysis::of(img);
        let _ = (
            a.histogram.median(),
            a.histogram.otsu(),
            a.histogram.percentile(5),
        );
        assert_eq!(a.noise_tenths, 0);
    }
    let detail = [
        PixelStep::Denoise {
            strength_pct: 100,
            detail_pct: 0,
        },
        PixelStep::Sharpen {
            radius: SubPx(80),
            amount_pct: 500,
            threshold_pct: 0,
        },
    ];
    assert!(
        grey_of(&run_steps(&white, &detail), &white)
            .iter()
            .all(|&g| g == 255)
    );
    assert!(
        grey_of(&run_steps(&black, &detail), &black)
            .iter()
            .all(|&g| g == 0)
    );
}

#[test]
fn edge_a_transparent_picture_reads_white_through_every_step() {
    let clear = image(20, 20, |x, _| [0, 0, 0, if x < 10 { 0 } else { 255 }]);
    let out = run_steps(
        &clear,
        &[PixelStep::Levels {
            black_pct: 0,
            white_pct: 100,
            gamma_centi: 100,
        }],
    );
    let g = grey_of(&out, &clear);
    assert_eq!(g[0], 255, "transparent is white");
    assert_eq!(g[15], 0);
}

#[test]
fn edge_extreme_radii_and_fields_do_not_crash() {
    let img = to_image(&textured(17, 13, 8));
    let steps = [
        PixelStep::LocalContrast {
            radius: SubPx(20_000 * 16),
            amount_pct: 200,
        },
        PixelStep::Sharpen {
            radius: SubPx(5_000 * 16),
            amount_pct: 500,
            threshold_pct: 50,
        },
        PixelStep::Contours {
            sigma: SubPx(1),
            sensitivity_pct: 100,
        },
        PixelStep::Sharpen {
            radius: SubPx(0),
            amount_pct: 500,
            threshold_pct: 0,
        },
        PixelStep::Levels {
            black_pct: 98,
            white_pct: 100,
            gamma_centi: 10,
        },
        PixelStep::Levels {
            black_pct: 0,
            white_pct: 2,
            gamma_centi: 400,
        },
    ];
    let out = run_steps(&img, &steps);
    assert_eq!(grey_of(&out, &img).len(), 17 * 13);
}

/// Finding (white-box, after reading `adjust_clahe.rs`): the remainder of the excess goes to the
/// lowest bins, so on a small tile (excess below 256 counts) the whole excess lands on the
/// darkest greys and the tile's table is lifted by up to excess / total x 255: a flat-ish
/// picture gets lighter instead of keeping its mean ("spread evenly", criterion 5 / ADR 0073
/// §4). Bounded here so the bias is visible: on realistic tiles (>= 4,096 px) the lift stays
/// within 2 greys of a mean-preserving spread.
#[test]
fn c5_local_contrast_keeps_the_mean_on_realistic_tiles() {
    let mut r = Lcg(4);
    let p = plane(256, 256, |x, y| {
        clamp8(120 + ((x as i64 % 16) - 8) + ((y as i64 % 12) - 6) + i64::from(r.spread(3)))
    });
    let img = to_image(&p);
    let got = grey_of(
        &run_steps(
            &img,
            &[PixelStep::LocalContrast {
                radius: SubPx(32 * 16),
                amount_pct: 50,
            }],
        ),
        &img,
    );
    let mean = |v: &[u8]| v.iter().map(|&g| f64::from(g)).sum::<f64>() / v.len() as f64;
    let (a, b) = (mean(&p.grey), mean(&got));
    assert!((a - b).abs() <= 8.0, "mean {a} -> {b}");
}

/// Local contrast "brings out detail inside small areas without changing the picture as a
/// whole" (criterion 5): a flat picture must stay near its grey at every radius the field
/// allows. Small tiles (side 8 to about 16 px, i.e. a radius of 0.2 mm on a fine picture) put
/// the whole excess on the lowest bins and turn a flat mid grey white.
#[test]
fn c5_local_contrast_keeps_a_flat_picture_near_its_grey_at_every_radius() {
    let mut bad = Vec::new();
    for grey in [40u8, 128, 200] {
        let img = grey_image(64, 64, |_, _| grey);
        for radius_px in [4u32, 6, 8, 12, 16, 32] {
            for amount in [10u16, 50, 200] {
                let out = grey_of(
                    &run_steps(
                        &img,
                        &[PixelStep::LocalContrast {
                            radius: SubPx(radius_px * 16),
                            amount_pct: amount,
                        }],
                    ),
                    &img,
                );
                let worst = out.iter().map(|&g| g.abs_diff(grey)).max().unwrap();
                if worst > 16 {
                    bad.push(format!("grey {grey} radius {radius_px} px amount {amount}: off by {worst} (e.g. {})", out[0]));
                }
            }
        }
    }
    assert!(bad.is_empty(), "{} cases:\n{}", bad.len(), bad.join("\n"));
}
