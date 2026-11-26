//! Tester acceptance cases for `specs/0135-image-optimisation` slice 1 in `curvyo-laser-core`,
//! written from the specification before the implementation was read: the seven preset tables
//! (criteria 31 to 38) recomputed by an independent reference from the spec's numbers and an
//! analysis, at Strength 0, 50, 100 and odd values, compared to `Preset::derive` within one field
//! step (criterion 54); the Dither each preset writes (27, 36, 37, 42, 43); the Interval rule
//! (40, 60) including Interval 0.01 and 2 mm; Expert (38); the stored-field ranges (5); the
//! registers' damage count (8, 23); the adjusted picture (19) and determinism (17).
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
    clippy::too_many_lines,
    clippy::float_cmp,
    missing_docs
)]

use curvyo_image_core::{Adjusted, Analysis, Histogram, WorkingImage};
use curvyo_laser_core::adjust::{
    AdjustRegisters, ManualStep, Preset, StepKind, StoredStep, adjust_picture,
};
use curvyo_laser_core::settings::{Dither, ImageEdit};

// ---- reference --------------------------------------------------------------------------------

/// v(S) = v0 + round-half-up((v100 - v0) x S / 100), criterion 31.
fn map(v0: i64, v100: i64, s: u8) -> i64 {
    let num = (v100 - v0) * i64::from(s);
    // round half up of num / 100
    v0 + (num * 2 + 100).div_euclid(200)
}

fn percentile(h: &Histogram, p_tenths: i64) -> i64 {
    let total: i64 = h.bins.iter().map(|&b| i64::from(b)).sum();
    let mut cum = 0;
    for g in 0..256 {
        cum += i64::from(h.bins[g]);
        if cum * 1000 >= p_tenths * total {
            return g as i64;
        }
    }
    255
}

fn median(h: &Histogram) -> i64 {
    percentile(h, 500)
}

fn otsu(h: &Histogram) -> i64 {
    let total: f64 = h.bins.iter().map(|&b| f64::from(b)).sum();
    let sum_all: f64 = (0..256).map(|g| g as f64 * f64::from(h.bins[g])).sum();
    let (mut best, mut best_t, mut w0, mut s0) = (-1.0, 0, 0.0, 0.0);
    for t in 0..256 {
        w0 += f64::from(h.bins[t]);
        s0 += t as f64 * f64::from(h.bins[t]);
        let w1 = total - w0;
        if w0 == 0.0 || w1 == 0.0 {
            continue;
        }
        let d = s0 / w0 - (sum_all - s0) / w1;
        let v = w0 * w1 * d * d;
        if v > best * (1.0 + 1e-12) {
            best = v;
            best_t = t as i64;
        }
    }
    best_t
}

fn pct_of_grey(g: i64) -> i64 {
    (g * 100 * 2 + 255).div_euclid(510)
}

/// Black and White from clip p (tenths of %), criterion 32 and ADR 0073 §5.
fn clip_points(h: &Histogram, clip_tenths: i64) -> (i64, i64) {
    let b = pct_of_grey(percentile(h, clip_tenths)).clamp(0, 98);
    let w = pct_of_grey(percentile(h, 1000 - clip_tenths))
        .clamp(2, 100)
        .max(b + 2);
    (b, w)
}

/// Gamma (centi) that lands grey `m` at target `t_pct` after the stretch B..W (`0123`
/// convention, above 1 lightens): ln(m') / ln(t).
fn gamma_for(m: i64, b_pct: i64, w_pct: i64, t_pct: i64) -> i64 {
    let b = (255 * b_pct + 50) / 100;
    let w = (255 * w_pct + 50) / 100;
    let x = (((m - b) * 255 * 2 + (w - b)) / (2 * (w - b))).clamp(0, 255);
    if x <= 1 || x >= 254 {
        return 100;
    }
    let mp = x as f64 / 255.0;
    let t = t_pct as f64 / 100.0;
    let g = (mp.ln() / t.ln() * 100.0).round() as i64;
    g.clamp(10, 400)
}

/// "k x Interval" in µm, rounded to the field step and clamped.
fn k_interval(k: i64, interval_um: i64, step: i64, lo: i64, hi: i64) -> i64 {
    let v = k * interval_um;
    let v = (v * 2 + step).div_euclid(2 * step) * step;
    v.clamp(lo, hi)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    None,
    Median(i64, i64),
    Otsu,
}

/// The spec's tables (criteria 32 to 37) as an independent reference.
fn reference(preset: Preset, s: u8, a: &Analysis, interval_um: i64) -> Vec<ManualStep> {
    let h = &a.histogram;
    let mut out = Vec::new();
    let lc = |k: i64, a0: i64, a100: i64| ManualStep::LocalContrast {
        radius_um: k_interval(k, interval_um, 100, 200, 20_000) as u32,
        amount_pct: map(a0, a100, s) as u16,
    };
    let sharpen = |a0: i64, a100: i64, t0: i64, t100: i64| ManualStep::Sharpen {
        radius_um: k_interval(1, interval_um, 10, 50, 5_000) as u32,
        amount_pct: map(a0, a100, s) as u16,
        threshold_pct: map(t0, t100, s) as u8,
    };
    let denoise = |s0: i64, s100: i64, d0: i64, d100: i64| ManualStep::Denoise {
        strength_pct: map(s0, s100, s) as u8,
        detail_pct: map(d0, d100, s) as u8,
    };
    let levels = |c0: i64, c100: i64, target: Target| {
        let (b, w) = clip_points(h, map(c0, c100, s));
        let gamma = match target {
            Target::None => 100,
            Target::Median(t0, t100) => gamma_for(median(h), b, w, map(t0, t100, s)),
            Target::Otsu => gamma_for(otsu(h), b, w, 50),
        };
        ManualStep::Levels {
            black_pct: b as u8,
            white_pct: w as u8,
            gamma_centi: gamma as u16,
        }
    };
    match preset {
        Preset::Photo => {
            if a.noise_tenths > 30 {
                out.push(denoise(10, 40, 70, 50));
            }
            out.push(levels(2, 20, Target::Median(50, 42)));
            out.push(lc(100, 10, 90));
            out.push(sharpen(40, 200, 0, 0));
        }
        Preset::Graphics => {
            out.push(denoise(20, 60, 90, 70));
            out.push(levels(0, 10, Target::None));
            out.push(sharpen(0, 100, 5, 10));
        }
        Preset::BlackWhite => {
            out.push(denoise(0, 30, 80, 60));
            out.push(levels(5, 30, Target::Otsu));
            // Spec table amended 2026-11-20 (UX review U7): no Local contrast.
            out.push(sharpen(50, 200, 0, 0));
        }
        Preset::LineArt => {
            out.push(denoise(20, 40, 80, 60));
            out.push(levels(5, 20, Target::None));
            // Spec table amended 2026-11-20 (UX review U7).
            out.push(lc(300, 0, 30));
            out.push(ManualStep::Contours {
                width_um: k_interval(5, interval_um, 10, 50, 2_000) as u32,
                sensitivity_pct: map(60, 100, s) as u8,
            });
            out.push(sharpen(0, 100, 0, 0));
        }
        Preset::DotRaster => {
            out.push(denoise(20, 40, 60, 60));
            out.push(levels(5, 20, Target::Median(50, 45)));
            out.push(lc(100, 0, 50));
            out.push(sharpen(0, 80, 0, 0));
        }
        Preset::Newspaper => {
            out.push(denoise(30, 60, 50, 50));
            out.push(levels(20, 60, Target::Median(45, 38)));
        }
        Preset::Expert => {}
    }
    out
}

/// Field-by-field within one stored step (criterion 54).
fn close(a: &ManualStep, b: &ManualStep) -> Result<(), String> {
    let fa = a.fields();
    let fb = b.fields();
    if std::mem::discriminant(a) != std::mem::discriminant(b) || fa.len() != fb.len() {
        return Err(format!("kind {a:?} vs {b:?}"));
    }
    for ((ka, va), (kb, vb)) in fa.iter().zip(&fb) {
        // The radius fields' steps are 100 µm (Local contrast) and 10 µm (Sharpen, Contours).
        let step = if ka.ends_with("_um") {
            if matches!(a, ManualStep::LocalContrast { .. }) {
                100
            } else {
                10
            }
        } else {
            1
        };
        if ka != kb || (va - vb).abs() > step {
            return Err(format!("{ka}: got {va} want {vb} ({a:?} vs {b:?})"));
        }
    }
    Ok(())
}

fn analysis(greys: impl IntoIterator<Item = u8>, noise_tenths: u32) -> Analysis {
    let mut bins = [0u32; 256];
    for g in greys {
        bins[g as usize] += 1;
    }
    Analysis {
        histogram: Histogram { bins },
        noise_tenths,
    }
}

struct Lcg(u32);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0 >> 8
    }
}

/// Pictures of different shapes: a dark photo, a bright photo, a two-tone page, a full ramp,
/// a narrow band and a noisy photo.
fn analyses() -> Vec<(&'static str, Analysis)> {
    let mut r = Lcg(3);
    let dark: Vec<u8> = (0..20_000).map(|_| (20 + r.next() % 120) as u8).collect();
    let bright: Vec<u8> = (0..20_000)
        .map(|_| (120 + (r.next() % 120) * (r.next() % 2 + 1) / 2) as u8)
        .collect();
    let two: Vec<u8> = (0..10_000)
        .map(|i| if i % 5 == 0 { 30 } else { 220 })
        .collect();
    let ramp: Vec<u8> = (0..25_600).map(|i| (i % 256) as u8).collect();
    let narrow: Vec<u8> = (0..9_000).map(|_| (110 + r.next() % 30) as u8).collect();
    vec![
        ("dark", analysis(dark, 12)),
        ("bright", analysis(bright, 45)),
        ("two_tone", analysis(two, 5)),
        ("ramp", analysis(ramp, 0)),
        ("narrow", analysis(narrow.clone(), 31)),
        ("noise_at_three", analysis(narrow, 30)),
    ]
}

const PRESETS: [Preset; 6] = [
    Preset::Photo,
    Preset::Graphics,
    Preset::BlackWhite,
    Preset::LineArt,
    Preset::DotRaster,
    Preset::Newspaper,
];

// ---- criteria 31 to 38: the preset tables ------------------------------------------------------

#[test]
fn c31_to_c37_every_preset_matches_the_spec_table_at_strength_0_50_100_and_odd_values() {
    let mut failures = Vec::new();
    for preset in PRESETS {
        for (name, a) in analyses() {
            for s in [0u8, 1, 33, 50, 67, 99, 100] {
                for interval in [100i64, 200] {
                    let got = preset.derive(s, &a, interval as u32);
                    let want = reference(preset, s, &a, interval);
                    if got.len() != want.len() {
                        failures.push(format!(
                            "{preset:?} {name} S{s} I{interval}: {} steps, want {}\n got {got:?}\nwant {want:?}",
                            got.len(),
                            want.len()
                        ));
                        continue;
                    }
                    for (g, w) in got.iter().zip(&want) {
                        if let Err(e) = close(g, w) {
                            failures.push(format!("{preset:?} {name} S{s} I{interval}: {e}"));
                        }
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures[..failures.len().min(12)].join("\n")
    );
}

#[test]
fn c38_expert_derives_nothing_and_writes_no_dither() {
    for (_, a) in analyses() {
        assert!(Preset::Expert.derive(70, &a, 100).is_empty());
    }
    assert!(Preset::Expert.dither_edits(100).is_empty());
}

#[test]
fn c32_photo_denoise_only_above_noise_three() {
    let (_, quiet) = analyses().remove(4);
    let mut at_three = quiet.clone();
    at_three.noise_tenths = 30;
    let mut above = quiet;
    above.noise_tenths = 31;
    let kinds = |a: &Analysis| {
        Preset::Photo
            .derive(50, a, 100)
            .iter()
            .filter(|s| matches!(s, ManualStep::Denoise { .. }))
            .count()
    };
    assert_eq!(kinds(&at_three), 0, "3.0 is not above 3.0");
    assert_eq!(kinds(&above), 1);
}

#[test]
fn c27_c32_to_c37_each_preset_writes_its_dither_and_fields() {
    let mode = |p: Preset| {
        p.dither_edits(100)
            .into_iter()
            .find_map(|e| match e {
                ImageEdit::Dither(d) => Some(d),
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(mode(Preset::Photo), Dither::Jarvis);
    assert_eq!(mode(Preset::Graphics), Dither::Ordered);
    assert_eq!(mode(Preset::BlackWhite), Dither::Threshold);
    assert_eq!(mode(Preset::LineArt), Dither::Threshold);
    assert_eq!(mode(Preset::DotRaster), Dither::Halftone);
    assert_eq!(mode(Preset::Newspaper), Dither::LineScreen);
    for p in [Preset::BlackWhite, Preset::LineArt] {
        assert!(
            p.dither_edits(100).contains(&ImageEdit::Threshold(50)),
            "{p:?}: Threshold 50 %"
        );
    }
    // Cell 8 x Interval, Line pitch 12 x Interval, Angle 45, in hundredths of mm.
    let dr = Preset::DotRaster.dither_edits(100);
    assert!(dr.contains(&ImageEdit::ScreenSize(80)), "{dr:?}");
    assert!(dr.contains(&ImageEdit::ScreenAngle(45)), "{dr:?}");
    let np = Preset::Newspaper.dither_edits(100);
    assert!(np.contains(&ImageEdit::ScreenSize(120)), "{np:?}");
    assert!(np.contains(&ImageEdit::ScreenAngle(45)), "{np:?}");
    // Clamped to 0.20 to 5.00 mm at extreme Intervals.
    assert!(
        Preset::DotRaster
            .dither_edits(10)
            .contains(&ImageEdit::ScreenSize(20))
    );
    assert!(
        Preset::Newspaper
            .dither_edits(2_000)
            .contains(&ImageEdit::ScreenSize(500))
    );
    // Criterion 27 / 41: a pick never writes Invert, Brightness, Contrast, Gamma.
    for p in PRESETS {
        for e in p.dither_edits(100) {
            assert!(
                !matches!(
                    e,
                    ImageEdit::Invert(_)
                        | ImageEdit::Brightness(_)
                        | ImageEdit::Contrast(_)
                        | ImageEdit::Gamma(_)
                        | ImageEdit::LowestPower(_)
                ),
                "{p:?} writes {e:?}"
            );
        }
    }
}

#[test]
fn c40_interval_0_1_and_0_2_differ_exactly_in_the_mm_fields() {
    for preset in PRESETS {
        for (name, a) in analyses() {
            let x = preset.derive(50, &a, 100);
            let y = preset.derive(50, &a, 200);
            assert_eq!(x.len(), y.len());
            for (p, q) in x.iter().zip(&y) {
                for ((k, v1), (_, v2)) in p.fields().iter().zip(q.fields().iter()) {
                    if k.ends_with("_um") {
                        let clamped = *v1 == 20_000 || *v2 == 20_000 || *v2 == 2_000;
                        if !clamped {
                            assert_eq!(*v2, 2 * v1, "{preset:?} {name} {k}");
                        }
                    } else {
                        assert_eq!(v1, v2, "{preset:?} {name} {k}: non-mm field changed");
                    }
                }
            }
        }
    }
}

#[test]
fn edge_interval_0_01_and_2_mm_clamp_the_mm_fields_to_their_ranges() {
    for preset in PRESETS {
        for (_, a) in analyses() {
            for interval in [10u32, 2_000] {
                for step in preset.derive(100, &a, interval) {
                    match step {
                        ManualStep::LocalContrast { radius_um, .. } => {
                            assert!(
                                (200..=20_000).contains(&radius_um),
                                "{preset:?} {radius_um}"
                            );
                        }
                        ManualStep::Sharpen { radius_um, .. } => {
                            assert!((50..=5_000).contains(&radius_um), "{preset:?} {radius_um}");
                        }
                        ManualStep::Contours { width_um, .. } => {
                            assert!((50..=2_000).contains(&width_um), "{preset:?} {width_um}");
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

#[test]
fn edge_extreme_strength_values_stay_in_range() {
    for preset in PRESETS {
        for (_, a) in analyses() {
            for s in [0u8, 100, 101, 200, 255] {
                for step in preset.derive(s, &a, 100) {
                    let kind = StepKind::from_key(kind_key(&step)).unwrap();
                    let back = ManualStep::read(kind_key(&step), |k| {
                        step.fields()
                            .iter()
                            .find(|(key, _)| *key == k)
                            .map(|(_, v)| *v)
                    });
                    assert_eq!(
                        back,
                        Ok(step),
                        "{preset:?} S{s}: {step:?} reads back ({kind:?})"
                    );
                }
            }
        }
    }
}

fn kind_key(s: &ManualStep) -> &'static str {
    match s {
        ManualStep::Denoise { .. } => "denoise",
        ManualStep::Levels { .. } => "levels",
        ManualStep::LocalContrast { .. } => "local_contrast",
        ManualStep::Contours { .. } => "contours",
        ManualStep::Sharpen { .. } => "sharpen",
        // Slice 2 kinds; no preset derives them.
        ManualStep::Crop { .. } => "crop",
        ManualStep::Resize { .. } => "resize",
        ManualStep::ChannelMix { .. } => "channel_mix",
        ManualStep::Curves { .. } => "curves",
        // Slice 3 kinds; no preset derives them.
        ManualStep::Upscale { .. } => "upscale",
        ManualStep::AiDenoise { .. } => "ai_denoise",
        ManualStep::RemoveBackground { .. } => "remove_background",
    }
}

#[test]
fn c3_derived_steps_are_in_pipeline_order() {
    let order = |s: &ManualStep| match s {
        ManualStep::Denoise { .. } => 0,
        ManualStep::Levels { .. } => 1,
        ManualStep::LocalContrast { .. } => 2,
        ManualStep::Contours { .. } => 3,
        ManualStep::Sharpen { .. } => 4,
        // Slice 2 kinds; no preset derives them.
        ManualStep::Crop { .. }
        | ManualStep::Resize { .. }
        | ManualStep::ChannelMix { .. }
        | ManualStep::Curves { .. }
        // Slice 3 kinds; no preset derives them.
        | ManualStep::Upscale { .. }
        | ManualStep::AiDenoise { .. }
        | ManualStep::RemoveBackground { .. } => 5,
    };
    for preset in PRESETS {
        for (_, a) in analyses() {
            let steps = preset.derive(50, &a, 100);
            assert!(
                steps.windows(2).all(|w| order(&w[0]) < order(&w[1])),
                "{preset:?}"
            );
        }
    }
}

#[test]
fn c25_preset_keys_read_and_unknown_reads_none() {
    for (k, p) in [
        ("photo", Preset::Photo),
        ("graphics", Preset::Graphics),
        ("black_white", Preset::BlackWhite),
        ("line_art", Preset::LineArt),
        ("dot_raster", Preset::DotRaster),
        ("newspaper", Preset::Newspaper),
        ("expert", Preset::Expert),
    ] {
        assert_eq!(Preset::from_key(k), Some(p));
    }
    assert_eq!(Preset::from_key("halftone"), None);
    assert_eq!(Preset::from_key(""), None);
}

// ---- criterion 5: stored ranges ---------------------------------------------------------------

#[test]
fn c5_out_of_range_fields_are_damage_not_clamped() {
    let read = |kind: &str, fields: &[(&str, i64)]| {
        ManualStep::read(kind, |k| {
            fields.iter().find(|(f, _)| *f == k).map(|(_, v)| *v)
        })
    };
    assert!(
        read(
            "levels",
            &[("black_pct", 0), ("white_pct", 100), ("gamma_centi", 100)]
        )
        .is_ok()
    );
    assert!(
        read(
            "levels",
            &[("black_pct", 99), ("white_pct", 100), ("gamma_centi", 100)]
        )
        .is_err()
    );
    assert!(
        read(
            "levels",
            &[("black_pct", 50), ("white_pct", 51), ("gamma_centi", 100)]
        )
        .is_err(),
        "black + 2"
    );
    assert!(
        read(
            "levels",
            &[("black_pct", 0), ("white_pct", 100), ("gamma_centi", 9)]
        )
        .is_err()
    );
    assert!(
        read(
            "levels",
            &[("black_pct", 0), ("white_pct", 100), ("gamma_centi", 401)]
        )
        .is_err()
    );
    assert!(read("local_contrast", &[("radius_um", 199), ("amount_pct", 50)]).is_err());
    assert!(
        read(
            "local_contrast",
            &[("radius_um", 20_001), ("amount_pct", 50)]
        )
        .is_err()
    );
    assert!(
        read(
            "local_contrast",
            &[("radius_um", 3_000), ("amount_pct", 201)]
        )
        .is_err()
    );
    assert!(
        read(
            "sharpen",
            &[("radius_um", 49), ("amount_pct", 100), ("threshold_pct", 0)]
        )
        .is_err()
    );
    assert!(
        read(
            "sharpen",
            &[
                ("radius_um", 100),
                ("amount_pct", 501),
                ("threshold_pct", 0)
            ]
        )
        .is_err()
    );
    assert!(
        read(
            "sharpen",
            &[
                ("radius_um", 100),
                ("amount_pct", 100),
                ("threshold_pct", 51)
            ]
        )
        .is_err()
    );
    assert!(read("denoise", &[("strength_pct", 101), ("detail_pct", 60)]).is_err());
    assert!(
        read("denoise", &[("strength_pct", 30)]).is_err(),
        "missing field"
    );
    assert!(read("contours", &[("width_um", 2_001), ("sensitivity_pct", 50)]).is_err());
    assert!(read("contours", &[("width_um", 300), ("sensitivity_pct", -1)]).is_err());
    assert!(read("curves_x", &[]).is_err());
}

#[test]
fn c5_c45_defaults_are_the_spec_defaults() {
    let d = |k: StepKind| k.default_step(100);
    assert_eq!(
        d(StepKind::Sharpen),
        ManualStep::Sharpen {
            radius_um: 100,
            amount_pct: 100,
            threshold_pct: 0
        },
        "default radius = the Interval"
    );
    assert_eq!(
        d(StepKind::LocalContrast),
        ManualStep::LocalContrast {
            radius_um: 3_000,
            amount_pct: 50
        }
    );
    assert_eq!(
        d(StepKind::Denoise),
        ManualStep::Denoise {
            strength_pct: 30,
            detail_pct: 60
        }
    );
    assert_eq!(
        d(StepKind::Contours),
        ManualStep::Contours {
            width_um: 300,
            sensitivity_pct: 50
        },
        "3 x Interval"
    );
}

// ---- registers: Expert, preset in use, damage -------------------------------------------------

fn ramp_image() -> WorkingImage {
    let mut pixels = Vec::new();
    for _y in 0..32u32 {
        for x in 0..32u32 {
            let g = (x * 8) as u8;
            pixels.extend_from_slice(&[g, g, g, 255]);
        }
    }
    WorkingImage {
        width: 32,
        height: 32,
        pixels,
        original_size: (32, 32),
        truncated: false,
        animated: false,
    }
}

fn frame() -> [curvyo_document_core::Point; 4] {
    use curvyo_document_core::Point;
    [
        Point::new(0.0, 0.0),
        Point::new(3.2, 0.0),
        Point::new(3.2, 3.2),
        Point::new(0.0, 3.2),
    ]
}

#[test]
fn c1_c54_empty_registers_are_unchanged_byte_for_byte() {
    let (steps, adjusted) =
        adjust_picture(&ramp_image(), &frame(), &AdjustRegisters::default(), 100);
    assert!(steps.is_empty());
    assert_eq!(adjusted, Adjusted::Unchanged);
}

#[test]
fn c2_a_switched_off_step_is_as_if_absent() {
    let lv = ManualStep::Levels {
        black_pct: 20,
        white_pct: 80,
        gamma_centi: 150,
    };
    let regs = AdjustRegisters {
        steps: vec![StoredStep {
            enabled: false,
            step: Some(lv),
        }],
        ..AdjustRegisters::default()
    };
    let (steps, adjusted) = adjust_picture(&ramp_image(), &frame(), &regs, 100);
    assert!(steps.is_empty());
    assert_eq!(adjusted, Adjusted::Unchanged);
    assert!(
        regs.is_adjusted(),
        "Restore original stays enabled with an off step (criterion 59)"
    );
}

#[test]
fn c30_a_preset_replaces_the_manual_tone_steps() {
    let lv = ManualStep::Levels {
        black_pct: 40,
        white_pct: 60,
        gamma_centi: 300,
    };
    let regs = AdjustRegisters {
        preset: Preset::Graphics,
        strength: 50,
        steps: vec![StoredStep {
            enabled: true,
            step: Some(lv),
        }],
        ..AdjustRegisters::default()
    };
    let img = ramp_image();
    let (steps, _) = adjust_picture(&img, &frame(), &regs, 100);
    let only = Preset::Graphics.derive(50, &Analysis::of(&img), 100);
    assert_eq!(
        steps, only,
        "the manual Levels does not run beside the preset"
    );
}

#[test]
fn c17_c29_the_derived_steps_run_as_their_stored_form() {
    // Make editable writes what ran: the same stack as Expert gives the same picture.
    let img = ramp_image();
    for preset in PRESETS {
        let regs = AdjustRegisters {
            preset,
            strength: 63,
            ..AdjustRegisters::default()
        };
        let (steps, adjusted) = adjust_picture(&img, &frame(), &regs, 100);
        let manual = AdjustRegisters {
            steps: steps
                .iter()
                .map(|s| StoredStep {
                    enabled: true,
                    step: Some(*s),
                })
                .collect(),
            ..AdjustRegisters::default()
        };
        let (_, again) = adjust_picture(&img, &frame(), &manual, 100);
        assert_eq!(adjusted, again, "{preset:?}");
        let (_, twice) = adjust_picture(&img, &frame(), &regs, 100);
        assert_eq!(adjusted, twice, "{preset:?} deterministic");
    }
}

#[test]
fn c23_unreadable_steps_and_registers_count() {
    let regs = AdjustRegisters {
        steps: vec![
            StoredStep {
                enabled: true,
                step: None,
            },
            StoredStep {
                enabled: false,
                step: None,
            },
            StoredStep {
                enabled: true,
                step: Some(StepKind::Sharpen.default_step(100)),
            },
        ],
        damaged_registers: 1,
        ..AdjustRegisters::default()
    };
    assert_eq!(regs.broken_steps(), 3);
}

// ---- criterion 51, 52: material record format 4 -----------------------------------------------

#[test]
fn c51_c52_the_format_4_picture_record_round_trips_and_its_image_set_holds_no_adjustment() {
    use curvyo_laser_core::grids::{Param, PictureSource, Side, TestId};
    use curvyo_laser_core::library::{RECORD_FORMAT_VERSION, read_record, write_record};
    let path: std::path::PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "tests/fixtures/materials/format4_picture_test.json",
    ]
    .iter()
    .collect();
    let text = std::fs::read_to_string(path).unwrap();
    assert_eq!(RECORD_FORMAT_VERSION, 5);
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["format_version"], 4);
    let m = read_record("0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c.json", &text).unwrap();
    // Superseded by 0146 (ADR 0090): written back as format 5, nothing else differs.
    assert_eq!(
        write_record(&m),
        text.replacen("\"format_version\": 4,", "\"format_version\": 5,", 1),
        "byte for byte"
    );
    let t = m.test(TestId::parse("7ST4R").unwrap()).unwrap();
    assert_eq!(t.recipe.picture(), PictureSource::Selected);
    assert_eq!(t.recipe.axis(Side::Columns).param(), Param::Strength);
    // Criterion 52: nothing of a stack, preset or strength in any set of the record.
    let s = text.to_lowercase();
    for word in [
        "image_adjustments",
        "image_preset",
        "\"preset\"",
        "\"stack\"",
        "adjustments",
    ] {
        assert!(!s.contains(word), "the record stores no {word}");
    }
    // A record whose Strength axis runs past 100 is damage, never clamped.
    let bad = text.replacen("\"param\": \"strength\"", "\"param\": \"strength_x\"", 1);
    assert!(read_record("0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c.json", &bad).is_err());
}
