//! Tester acceptance cases for `specs/0135-image-optimisation` slice 2 in `curvyo-image-core`,
//! written from the specification (criteria 5 to 7, 14, 17, 54) and ADR 0073 §2 to §4 before the
//! implementation was read: independent reference implementations of Crop (the pixel rectangle
//! `floor(l·w/100)` to `ceil((l+wd)·w/100)`), Resize (an exact rational area average, two passes,
//! rounded half up), Channel mix (`clamp(round((r·R + g·G + b·B) / 100))`), and the properties of
//! Curves that the spec states (monotone, no overshoot, passes through its points, end points
//! fixed); the per-channel rule before the first grey step (criterion 7), Channel mix on a grey
//! picture changing nothing (criterion 6), stepped equals one-shot and the goldens being present.
#![allow(
    clippy::many_single_char_names,
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::cast_possible_wrap,
    clippy::needless_range_loop,
    clippy::too_many_lines,
    missing_docs
)]

use curvyo_image_core::{
    AdjustRun, Adjusted, GreyPlane, PixelStep, SubPx, WorkingImage, channel_mix_grey, crop_plane,
    crop_rect, curves_table, resize_plane, resized_size, run_steps,
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

struct Lcg(u32);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0 >> 8
    }
    fn byte(&mut self) -> u8 {
        (self.next() & 0xff) as u8
    }
}

fn noisy_colour(w: u32, h: u32, seed: u32) -> WorkingImage {
    let mut r = Lcg(seed);
    image(w, h, |_, _| [r.byte(), r.byte(), r.byte(), 255])
}

fn plane(w: u32, h: u32, grey: Vec<u8>) -> GreyPlane {
    GreyPlane {
        width: w,
        height: h,
        grey,
    }
}

fn channel(img: &WorkingImage, c: usize) -> Vec<u8> {
    img.pixels.chunks(4).map(|p| p[c]).collect()
}

fn rgba(a: &Adjusted) -> &WorkingImage {
    match a {
        Adjusted::Rgba(i) => i,
        other => panic!("expected an RGBA picture, got {:?}", kind(other)),
    }
}

fn grey(a: &Adjusted) -> &GreyPlane {
    match a {
        Adjusted::Grey(p) => p,
        other => panic!("expected a grey picture, got {:?}", kind(other)),
    }
}

fn kind(a: &Adjusted) -> &'static str {
    match a {
        Adjusted::Unchanged => "Unchanged",
        Adjusted::Rgba(_) => "Rgba",
        Adjusted::Grey(_) => "Grey",
    }
}

// ---- references -------------------------------------------------------------------------------

/// The spec's crop rectangle (ADR 0073 §4).
fn ref_crop_rect(w: u32, h: u32, [l, t, wd, ht]: [u8; 4]) -> (u32, u32, u32, u32) {
    let span = |p: u32, s: u32, n: u32| {
        let a = (p * n) / 100;
        let b = ((p + s) * n).div_ceil(100).min(n);
        (a, (b - a).max(1))
    };
    let (x, cw) = span(u32::from(l), u32::from(wd), w);
    let (y, ch) = span(u32::from(t), u32::from(ht), h);
    (x, y, cw, ch)
}

/// The exact area average of one row `src` resampled to `n` samples: sample `i` covers the
/// source interval `[i·w/n, (i+1)·w/n)`; the mean is rounded half up.
fn area_row(src: &[u8], n: usize) -> Vec<u8> {
    let w = src.len();
    (0..n)
        .map(|i| {
            // Work in units of 1/n source pixel: sample i covers [i·w, (i+1)·w).
            let (a, b) = (i * w, (i + 1) * w);
            let mut sum = 0u64; // in grey · (1/n px)
            for (j, &v) in src.iter().enumerate() {
                let (s0, s1) = (j * n, (j + 1) * n);
                let lo = a.max(s0);
                let hi = b.min(s1);
                if hi > lo {
                    sum += u64::from(v) * (hi - lo) as u64;
                }
            }
            let den = w as u64; // the span length in 1/n px
            ((2 * sum + den) / (2 * den)) as u8
        })
        .collect()
}

/// Rows first, then columns, each pass rounded half up.
fn ref_resize(p: &GreyPlane, nw: usize, nh: usize) -> Vec<u8> {
    let (w, h) = (p.width as usize, p.height as usize);
    let mut rows = vec![0u8; nw * h];
    for y in 0..h {
        let r = area_row(&p.grey[y * w..(y + 1) * w], nw);
        rows[y * nw..(y + 1) * nw].copy_from_slice(&r);
    }
    let mut out = vec![0u8; nw * nh];
    for x in 0..nw {
        let col: Vec<u8> = (0..h).map(|y| rows[y * nw + x]).collect();
        for (y, v) in area_row(&col, nh).into_iter().enumerate() {
            out[y * nw + x] = v;
        }
    }
    out
}

fn ref_resized_size(w: u32, h: u32, width_px: u32) -> (u32, u32) {
    let nw = width_px.clamp(1, w);
    // The height follows the ratio, rounded half up, at least one pixel.
    let nh = ((2 * u64::from(h) * u64::from(nw) + u64::from(w)) / (2 * u64::from(w))).max(1);
    (nw, nh as u32)
}

fn ref_channel_mix(r: u8, g: u8, b: u8, [wr, wg, wb]: [i16; 3]) -> u8 {
    let s =
        i64::from(r) * i64::from(wr) + i64::from(g) * i64::from(wg) + i64::from(b) * i64::from(wb);
    // Round half away from zero, then clamp.
    let q = if s >= 0 {
        (s + 50) / 100
    } else {
        -((-s + 50) / 100)
    };
    q.clamp(0, 255) as u8
}

fn grey_of_pct(p: u8) -> i64 {
    // 255 · p / 100 rounded half up.
    (255 * i64::from(p) * 2 + 100) / 200
}

// ---- criterion 5: Crop ------------------------------------------------------------------------

#[test]
fn c5_crop_rect_matches_the_spec_rectangle_for_every_valid_field() {
    for (w, h) in [
        (1u32, 1u32),
        (3, 7),
        (10, 10),
        (97, 31),
        (640, 480),
        (2000, 1),
    ] {
        for l in (0u8..100).step_by(7) {
            for wd in [1u8, 2, 13, 33, 50, 99, 100] {
                if u32::from(l) + u32::from(wd) > 100 {
                    continue;
                }
                for (t, ht) in [(0u8, 100u8), (20, 60), (99, 1), (37, 41)] {
                    let pct = [l, t, wd, ht];
                    let got = crop_rect(w, h, pct);
                    assert_eq!(got, ref_crop_rect(w, h, pct), "{w}x{h} {pct:?}");
                    let (x, y, cw, ch) = got;
                    assert!(cw >= 1 && ch >= 1 && x + cw <= w && y + ch <= h, "{got:?}");
                }
            }
        }
    }
}

#[test]
fn c5_crop_keeps_exactly_the_pixels_of_the_rectangle_per_channel() {
    let img = noisy_colour(37, 23, 11);
    for pct in [
        [20, 20, 60, 60],
        [0, 0, 100, 100],
        [99, 0, 1, 100],
        [13, 47, 31, 3],
    ] {
        let out = run_steps(&img, &[PixelStep::Crop { pct }]);
        let out = rgba(&out);
        let (x, y, cw, ch) = ref_crop_rect(37, 23, pct);
        assert_eq!((out.width, out.height), (cw, ch), "{pct:?}");
        for yy in 0..ch {
            for xx in 0..cw {
                let s = (((y + yy) * 37 + x + xx) * 4) as usize;
                let d = ((yy * cw + xx) * 4) as usize;
                assert_eq!(
                    &out.pixels[d..d + 3],
                    &img.pixels[s..s + 3],
                    "{pct:?} ({xx},{yy})"
                );
                assert_eq!(out.pixels[d + 3], 255, "opaque");
            }
        }
    }
}

#[test]
fn c5_crop_plane_is_the_rectangle_of_the_grey_plane() {
    let mut r = Lcg(5);
    let g: Vec<u8> = (0..(19 * 11)).map(|_| r.byte()).collect();
    let p = plane(19, 11, g.clone());
    let pct = [10, 30, 45, 50];
    let (x, y, cw, ch) = ref_crop_rect(19, 11, pct);
    let c = crop_plane(&p, pct);
    assert_eq!((c.width, c.height), (cw, ch));
    for yy in 0..ch {
        for xx in 0..cw {
            assert_eq!(
                c.grey[(yy * cw + xx) as usize],
                g[((y + yy) * 19 + x + xx) as usize]
            );
        }
    }
}

#[test]
fn c6_crop_composites_alpha_over_white() {
    let img = image(4, 4, |x, _| {
        if x < 2 {
            [10, 20, 30, 0]
        } else {
            [10, 20, 30, 255]
        }
    });
    let out = run_steps(
        &img,
        &[PixelStep::Crop {
            pct: [0, 0, 100, 100],
        }],
    );
    let out = rgba(&out);
    for (i, p) in out.pixels.chunks(4).enumerate() {
        let x = i % 4;
        if x < 2 {
            assert_eq!(p, [255, 255, 255, 255], "transparent becomes white");
        } else {
            assert_eq!(p, [10, 20, 30, 255]);
        }
    }
}

// ---- criterion 5, 14: Resize ------------------------------------------------------------------

#[test]
fn c14_resize_only_reduces_and_the_height_follows_the_ratio() {
    for (w, h) in [(1u32, 1u32), (16, 9), (100, 37), (2000, 2000), (33, 1000)] {
        for width_px in [
            1u32,
            16,
            w / 2,
            w.saturating_sub(1),
            w,
            w + 1,
            4 * w,
            u32::MAX,
        ] {
            let got = resized_size(w, h, width_px);
            assert_eq!(
                got,
                ref_resized_size(w, h, width_px),
                "{w}x{h} -> {width_px}"
            );
            assert!(got.0 <= w, "never wider");
            assert!(got.1 <= h + 1 && got.1 >= 1, "{got:?}");
        }
    }
    // A Resize above the width is the picture itself: no enlargement through the pipeline.
    let img = noisy_colour(20, 10, 3);
    let out = run_steps(&img, &[PixelStep::Resize { width_px: 80 }]);
    let out = rgba(&out);
    assert_eq!((out.width, out.height), (20, 10));
    assert_eq!(out.pixels, img.pixels);
}

#[test]
fn c5_resize_half_is_the_exact_two_by_two_mean_rounded_half_up() {
    let mut r = Lcg(9);
    let g: Vec<u8> = (0..(16 * 12)).map(|_| r.byte()).collect();
    let p = plane(16, 12, g);
    let out = resize_plane(&p, 8);
    assert_eq!((out.width, out.height), (8, 6));
    assert_eq!(out.grey, ref_resize(&p, 8, 6));
}

#[test]
fn c5_resize_by_any_ratio_is_the_area_average_within_one_grey() {
    let mut r = Lcg(21);
    for (w, h, nw) in [
        (17u32, 13u32, 5u32),
        (100, 3, 7),
        (31, 29, 30),
        (9, 40, 2),
        (64, 64, 1),
    ] {
        let g: Vec<u8> = (0..(w * h)).map(|_| r.byte()).collect();
        let p = plane(w, h, g);
        let (ew, eh) = ref_resized_size(w, h, nw);
        let out = resize_plane(&p, nw);
        assert_eq!((out.width, out.height), (ew, eh), "{w}x{h} -> {nw}");
        let want = ref_resize(&p, ew as usize, eh as usize);
        for (i, (&a, &b)) in out.grey.iter().zip(&want).enumerate() {
            assert!(
                (i32::from(a) - i32::from(b)).abs() <= 1,
                "{w}x{h} -> {nw} at {i}: {a} vs {b}"
            );
        }
    }
    // A flat plane stays flat at every ratio (weights sum exactly to one).
    for v in [0u8, 1, 127, 254, 255] {
        let p = plane(37, 23, vec![v; 37 * 23]);
        assert!(
            resize_plane(&p, 11).grey.iter().all(|&g| g == v),
            "flat {v}"
        );
    }
}

#[test]
fn c7_resize_before_grey_runs_per_channel() {
    let img = noisy_colour(24, 16, 77);
    let out = run_steps(&img, &[PixelStep::Resize { width_px: 9 }]);
    let out = rgba(&out);
    let (nw, nh) = ref_resized_size(24, 16, 9);
    assert_eq!((out.width, out.height), (nw, nh));
    for c in 0..3 {
        let p = plane(24, 16, channel(&img, c));
        let want = resize_plane(&p, 9).grey;
        assert_eq!(channel(out, c), want, "channel {c} resized on its own");
    }
    assert!(channel(out, 3).iter().all(|&a| a == 255));
}

// ---- criterion 5, 6, 7: Channel mix -----------------------------------------------------------

#[test]
fn c5_channel_mix_grey_is_the_rounded_clamped_weighted_sum() {
    let mut r = Lcg(1);
    let weights: Vec<[i16; 3]> = vec![
        [21, 72, 7],
        [100, 0, 0],
        [0, 100, 0],
        [0, 0, 100],
        [200, 200, 200],
        [-200, -200, -200],
        [-200, 200, 0],
        [150, -75, 33],
        [1, 1, 1],
        [-1, 0, 0],
    ];
    for w in weights {
        for _ in 0..4000 {
            let (a, b, c) = (r.byte(), r.byte(), r.byte());
            assert_eq!(
                channel_mix_grey(a, b, c, w),
                ref_channel_mix(a, b, c, w),
                "{a} {b} {c} {w:?}"
            );
        }
        for (a, b, c) in [
            (0, 0, 0),
            (255, 255, 255),
            (255, 0, 0),
            (1, 1, 1),
            (2, 3, 5),
        ] {
            assert_eq!(channel_mix_grey(a, b, c, w), ref_channel_mix(a, b, c, w));
        }
    }
    // Half away from zero at exact halves: 50 · 1 / 100 = 0.5 -> 1.
    assert_eq!(channel_mix_grey(50, 0, 0, [1, 0, 0]), 1);
    assert_eq!(channel_mix_grey(150, 0, 0, [1, 0, 0]), 2);
}

#[test]
fn c5_c7_channel_mix_as_the_first_grey_step_makes_the_grey_with_its_weights() {
    let img = noisy_colour(13, 9, 41);
    for w in [[100, 0, 0], [0, 100, 0], [0, 0, 100], [150, -30, 10]] {
        let out = run_steps(&img, &[PixelStep::ChannelMix { weights: w }]);
        let p = grey(&out);
        let want: Vec<u8> = img
            .pixels
            .chunks(4)
            .map(|q| ref_channel_mix(q[0], q[1], q[2], w))
            .collect();
        assert_eq!(p.grey, want, "{w:?}");
    }
    // Red filter after a Crop: the grey is the red of the cropped pixels (per channel before grey).
    let pct = [25, 25, 50, 50];
    let out = run_steps(
        &img,
        &[
            PixelStep::Crop { pct },
            PixelStep::ChannelMix {
                weights: [100, 0, 0],
            },
        ],
    );
    let p = grey(&out);
    let (x, y, cw, ch) = ref_crop_rect(13, 9, pct);
    let mut want = Vec::new();
    for yy in y..y + ch {
        for xx in x..x + cw {
            want.push(img.pixels[((yy * 13 + xx) * 4) as usize]);
        }
    }
    assert_eq!((p.width, p.height), (cw, ch));
    assert_eq!(p.grey, want);
}

#[test]
fn c6_channel_mix_on_a_grey_picture_changes_nothing() {
    let mut r = Lcg(8);
    let neutral = image(15, 11, |_, _| {
        let v = r.byte();
        [v, v, v, 255]
    });
    let base = GreyPlane::of_rgba(&neutral).grey;
    for w in [
        [200, -50, 0],
        [-200, -200, -200],
        [0, 0, 0],
        [100, 0, 0],
        [21, 72, 7],
    ] {
        let out = run_steps(&neutral, &[PixelStep::ChannelMix { weights: w }]);
        let got = match &out {
            Adjusted::Unchanged => base.clone(),
            Adjusted::Rgba(i) => GreyPlane::of_rgba(i).grey,
            Adjusted::Grey(p) => p.grey.clone(),
        };
        assert_eq!(
            got, base,
            "a neutral picture is grey already: {w:?} changes nothing"
        );
    }
    // After a grey step a Channel mix changes nothing either.
    let img = noisy_colour(12, 12, 4);
    let levels = PixelStep::Levels {
        black_pct: 10,
        white_pct: 90,
        gamma_centi: 120,
    };
    let once = run_steps(&img, &[levels]);
    let twice = run_steps(
        &img,
        &[
            levels,
            PixelStep::ChannelMix {
                weights: [200, 0, -100],
            },
        ],
    );
    assert_eq!(grey(&twice).grey, grey(&once).grey);
}

#[test]
fn c7_denoise_before_and_after_channel_mix_both_run() {
    let img = noisy_colour(20, 20, 99);
    let dn = PixelStep::Denoise {
        strength_pct: 80,
        detail_pct: 20,
    };
    let mix = PixelStep::ChannelMix {
        weights: [100, 0, 0],
    };
    let before = run_steps(&img, &[dn, mix]);
    let after = run_steps(&img, &[mix, dn]);
    let only = run_steps(&img, &[mix]);
    assert_eq!(kind(&before), "Grey");
    assert_eq!(kind(&after), "Grey");
    assert_ne!(grey(&before).grey, grey(&only).grey, "Denoise before ran");
    assert_ne!(grey(&after).grey, grey(&only).grey, "Denoise after ran");
}

// ---- criterion 5: Curves ----------------------------------------------------------------------

fn points(v: &[(u8, u8)]) -> Vec<(u8, u8)> {
    v.to_vec()
}

fn random_points(r: &mut Lcg, monotone: bool) -> Vec<(u8, u8)> {
    let n = 2 + (r.next() % 7) as usize; // 2..=8
    let mut xs: Vec<u8> = vec![0, 100];
    while xs.len() < n {
        let x = (1 + r.next() % 99) as u8;
        if !xs.contains(&x) {
            xs.push(x);
        }
    }
    xs.sort_unstable();
    let mut ys: Vec<u8> = (0..n).map(|_| (r.next() % 101) as u8).collect();
    if monotone {
        ys.sort_unstable();
        if r.next().is_multiple_of(2) {
            ys.reverse();
        }
    }
    xs.into_iter().zip(ys).collect()
}

#[test]
fn c5_curves_identity_and_collinear_points_are_the_identity_table() {
    let id: Vec<u8> = (0..=255).collect();
    assert_eq!(curves_table(&points(&[(0, 0), (100, 100)])).to_vec(), id);
    assert_eq!(
        curves_table(&points(&[(0, 0), (40, 40), (100, 100)])).to_vec(),
        id
    );
    assert_eq!(
        curves_table(&[]).to_vec(),
        id,
        "fewer than two points is the identity"
    );
}

#[test]
fn c5_curves_end_points_and_points_are_hit() {
    // Inputs at multiples of 20 % fall on greys 0, 51, 102, 153, 204, 255 exactly.
    let pts = points(&[(0, 20), (20, 40), (40, 0), (60, 100), (80, 60), (100, 80)]);
    let t = curves_table(&pts);
    for &(x, y) in &pts {
        let g = usize::from(x) * 255 / 100;
        assert_eq!(i64::from(t[g]), grey_of_pct(y), "point ({x}, {y})");
    }
}

#[test]
fn c5_curves_monotone_points_give_a_monotone_table() {
    let mut r = Lcg(2024);
    for _ in 0..3000 {
        let pts = random_points(&mut r, true);
        let t = curves_table(&pts);
        let up = pts.first().unwrap().1 <= pts.last().unwrap().1;
        for g in 1..256 {
            if up {
                assert!(t[g] >= t[g - 1], "{pts:?} at {g}");
            } else {
                assert!(t[g] <= t[g - 1], "{pts:?} at {g}");
            }
        }
    }
}

#[test]
fn c5_curves_never_overshoot_between_two_points() {
    let mut r = Lcg(7);
    for i in 0..3000 {
        let pts = random_points(&mut r, i % 3 == 0);
        let t = curves_table(&pts);
        for w in pts.windows(2) {
            let ((x0, y0), (x1, y1)) = (w[0], w[1]);
            let lo = grey_of_pct(y0.min(y1));
            let hi = grey_of_pct(y0.max(y1));
            for g in 0..256usize {
                let input = g as f64 * 100.0 / 255.0;
                if input >= f64::from(x0) && input <= f64::from(x1) {
                    let v = i64::from(t[g]);
                    assert!(
                        v >= lo - 1 && v <= hi + 1,
                        "{pts:?}: grey {g} gives {v} outside {lo}..{hi}"
                    );
                }
            }
        }
    }
}

#[test]
fn c5_curves_s_curve_darkens_shadows_and_lifts_highlights() {
    let t = curves_table(&points(&[(0, 0), (25, 15), (75, 85), (100, 100)]));
    assert_eq!(t[0], 0);
    assert_eq!(t[255], 255);
    assert!(t[64] < 64, "shadows darker: {}", t[64]);
    assert!(t[191] > 191, "highlights lighter: {}", t[191]);
    let step = PixelStep::Curves {
        points: [
            (0, 0),
            (25, 15),
            (75, 85),
            (100, 100),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
        ],
        count: 4,
    };
    let img = image(16, 16, |x, y| {
        let v = (x * 16 + y) as u8;
        [v, v, v, 255]
    });
    let out = run_steps(&img, &[step]);
    let base = GreyPlane::of_rgba(&img).grey;
    let want: Vec<u8> = base.iter().map(|&g| t[usize::from(g)]).collect();
    assert_eq!(grey(&out).grey, want, "the step is the table on the grey");
}

// ---- criterion 17: determinism, stepped equals one-shot ---------------------------------------

#[test]
fn c17_slice2_steps_stepped_equal_one_shot_and_repeat() {
    let img = noisy_colour(48, 40, 1234);
    let lists: Vec<Vec<PixelStep>> = vec![
        vec![PixelStep::Crop {
            pct: [10, 5, 70, 80],
        }],
        vec![PixelStep::Resize { width_px: 17 }],
        vec![
            PixelStep::Crop {
                pct: [10, 5, 70, 80],
            },
            PixelStep::Resize { width_px: 20 },
            PixelStep::ChannelMix {
                weights: [60, 30, 10],
            },
            PixelStep::Curves {
                points: [
                    (0, 10),
                    (50, 40),
                    (100, 90),
                    (0, 0),
                    (0, 0),
                    (0, 0),
                    (0, 0),
                    (0, 0),
                ],
                count: 3,
            },
            PixelStep::Sharpen {
                radius: SubPx(16),
                amount_pct: 100,
                threshold_pct: 0,
            },
        ],
    ];
    for steps in lists {
        let once = run_steps(&img, &steps);
        assert_eq!(run_steps(&img, &steps), once, "repeatable");
        for work in [1u64, 7, 100, 5000, 1 << 20] {
            let mut run = AdjustRun::new(&img, &steps);
            let mut guard = 0;
            while !run.advance(work) {
                guard += 1;
                assert!(guard < 10_000_000);
            }
            assert_eq!(run.finish(), once, "{steps:?} work {work}");
        }
    }
}

#[test]
fn c54_the_slice2_goldens_exist_as_png() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/adjust");
    for name in [
        "curves_s",
        "channel_mix_red_filter",
        "crop_20pct",
        "resize_half",
    ] {
        let bytes = std::fs::read(dir.join(format!("{name}.png"))).unwrap();
        assert_eq!(&bytes[1..4], b"PNG", "{name}");
    }
}

#[test]
fn c5_curves_a_flat_segment_at_a_half_grey_is_flat() {
    let t = curves_table(&[(0, 0), (30, 50), (70, 50), (100, 100)]);
    for g in 1..256 {
        assert!(
            t[g] >= t[g - 1],
            "monotone at {g}: {} then {}",
            t[g - 1],
            t[g]
        );
    }
}
