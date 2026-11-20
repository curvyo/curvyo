//! The seven presets of a picture as data and their Strength mapping (ADR 0073 §5; `specs/0135`
//! criteria 25, 27, 31 to 38, 40, 41).

use curvyo_image_core::{Analysis, Histogram, levels_table};

use crate::image_adjust_fields::{ManualStep, StepKind};
use crate::operation_image_options::{Dither, ImageEdit};

/// The preset of a picture (criterion 25), in the strip's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Preset {
    /// Photo (detailed).
    Photo,
    /// Graphics (flat areas).
    Graphics,
    /// Black and white (hard edges).
    BlackWhite,
    /// Line art (outlines and handwriting).
    LineArt,
    /// Dot raster (round dots).
    DotRaster,
    /// Newspaper (retro screen).
    Newspaper,
    /// Expert: no preset, the manual stack.
    #[default]
    Expert,
}

impl Preset {
    /// The seven presets in the strip's order.
    pub const ALL: [Self; 7] = [
        Self::Photo,
        Self::Graphics,
        Self::BlackWhite,
        Self::LineArt,
        Self::DotRaster,
        Self::Newspaper,
        Self::Expert,
    ];

    /// The stored value (ADR 0072 §1).
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Photo => "photo",
            Self::Graphics => "graphics",
            Self::BlackWhite => "black_white",
            Self::LineArt => "line_art",
            Self::DotRaster => "dot_raster",
            Self::Newspaper => "newspaper",
            Self::Expert => "expert",
        }
    }

    /// The preset of a stored value, `None` for a value this build does not know.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.key() == key)
    }

    /// The full name (accessible name and tooltip).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Photo => "Photo (detailed)",
            Self::Graphics => "Graphics (flat areas)",
            Self::BlackWhite => "Black and white (hard edges)",
            Self::LineArt => "Line art (outlines and handwriting)",
            Self::DotRaster => "Dot raster (round dots)",
            Self::Newspaper => "Newspaper (retro screen)",
            Self::Expert => "Expert",
        }
    }

    /// The cell's short text (`ux-notes.md` §3.2).
    #[must_use]
    pub const fn cell(self) -> &'static str {
        match self {
            Self::Photo => "Photo",
            Self::Graphics => "Graphics",
            Self::BlackWhite => "Black and white",
            Self::LineArt => "Line art",
            Self::DotRaster => "Dots",
            Self::Newspaper => "Newspaper",
            Self::Expert => "Expert",
        }
    }

    /// The tooltip "`<full name> · Dither: <mode>`" (criterion 25).
    #[must_use]
    pub const fn tooltip(self) -> &'static str {
        match self {
            Self::Photo => "Photo (detailed) · Dither: Jarvis",
            Self::Graphics => "Graphics (flat areas) · Dither: Ordered",
            Self::BlackWhite => "Black and white (hard edges) · Dither: Threshold 50 %",
            Self::LineArt => "Line art (outlines and handwriting) · Dither: Threshold 50 %",
            Self::DotRaster => "Dot raster (round dots) · Dither: Halftone",
            Self::Newspaper => "Newspaper (retro screen) · Dither: Lines",
            Self::Expert => "Expert · Dither unchanged",
        }
    }

    /// The edits a pick folds into the picture's dither (criterion 27, ADR 0072 amendment A3): the
    /// Dither mode and that mode's fields, the screens' size as a multiple of `interval_um`
    /// (8 x for Dot raster, 12 x for Newspaper) clamped to 0.20 to 5.00 mm; none for Expert.
    #[must_use]
    pub fn dither_edits(self, interval_um: u32) -> Vec<ImageEdit> {
        let screen = |dither: Dither, k: u32| {
            let hundredths = (k * interval_um + 5) / 10;
            let hundredths = u16::try_from(hundredths.clamp(20, 500)).unwrap_or(500);
            vec![
                ImageEdit::Dither(dither),
                ImageEdit::ScreenSize(hundredths),
                ImageEdit::ScreenAngle(45),
            ]
        };
        match self {
            Self::Photo => vec![ImageEdit::Dither(Dither::Jarvis)],
            Self::Graphics => vec![ImageEdit::Dither(Dither::Ordered)],
            Self::BlackWhite | Self::LineArt => {
                vec![
                    ImageEdit::Dither(Dither::Threshold),
                    ImageEdit::Threshold(50),
                ]
            }
            Self::DotRaster => screen(Dither::Halftone, 8),
            Self::Newspaper => screen(Dither::LineScreen, 12),
            Self::Expert => Vec::new(),
        }
    }

    /// The derived steps of the preset at `strength` (0 to 100) for a picture with `analysis`
    /// evaluated at `interval_um`, in pipeline order and in stored units (so Make editable writes
    /// exactly what ran, criterion 29); empty for Expert (criterion 38).
    #[must_use]
    pub fn derive(self, strength: u8, analysis: &Analysis, interval_um: u32) -> Vec<ManualStep> {
        let s = i64::from(strength.min(100));
        let v = |v0: i64, v100: i64| v0 + round_half_up((v100 - v0) * s, 100);
        let k = |kind: StepKind, factor: u32, key: &str| {
            let spec = kind.fields().iter().find(|f| f.key == key).copied();
            spec.map_or(0, |spec| spec.clamped(i64::from(factor * interval_um)))
        };
        let denoise = |s0: i64, s100: i64, d0: i64, d100: i64| ManualStep::Denoise {
            strength_pct: to_u8(v(s0, s100)),
            detail_pct: to_u8(v(d0, d100)),
        };
        let levels = |clip0: i64, clip100: i64, target: Target| {
            levels_step(&analysis.histogram, v(clip0, clip100), target, &v)
        };
        let local = |factor: u32, a0: i64, a100: i64| ManualStep::LocalContrast {
            radius_um: to_u32(k(StepKind::LocalContrast, factor, "radius_um")),
            amount_pct: to_u16(v(a0, a100)),
        };
        let sharpen = |a0: i64, a100: i64, t0: i64, t100: i64| ManualStep::Sharpen {
            radius_um: to_u32(k(StepKind::Sharpen, 1, "radius_um")),
            amount_pct: to_u16(v(a0, a100)),
            threshold_pct: to_u8(v(t0, t100)),
        };
        match self {
            Self::Photo => {
                let mut steps = Vec::new();
                if analysis.noise_tenths > 30 {
                    steps.push(denoise(10, 40, 70, 50));
                }
                steps.extend([
                    levels(2, 20, Target::Median(50, 42)),
                    local(100, 10, 90),
                    sharpen(40, 200, 0, 0),
                ]);
                steps
            }
            Self::Graphics => vec![
                denoise(20, 60, 90, 70),
                levels(0, 10, Target::None),
                sharpen(0, 100, 5, 10),
            ],
            Self::BlackWhite => vec![
                denoise(0, 30, 80, 60),
                levels(5, 30, Target::Otsu),
                sharpen(50, 200, 0, 0),
            ],
            Self::LineArt => vec![
                denoise(20, 40, 80, 60),
                levels(5, 20, Target::None),
                local(300, 0, 30),
                ManualStep::Contours {
                    width_um: to_u32(k(StepKind::Contours, 5, "width_um")),
                    sensitivity_pct: to_u8(v(60, 100)),
                },
                sharpen(0, 100, 0, 0),
            ],
            Self::DotRaster => vec![
                denoise(20, 40, 60, 60),
                levels(5, 20, Target::Median(50, 45)),
                local(100, 0, 50),
                sharpen(0, 80, 0, 0),
            ],
            Self::Newspaper => vec![
                denoise(30, 60, 50, 50),
                levels(20, 60, Target::Median(45, 38)),
            ],
            Self::Expert => Vec::new(),
        }
    }
}

/// What a preset's Levels Gamma aims at.
#[derive(Debug, Clone, Copy)]
enum Target {
    /// Gamma 1.00.
    None,
    /// The median lands at this percentage, from Gentle to Strong.
    Median(i64, i64),
    /// The Otsu level lands at 50 %.
    Otsu,
}

/// The Black and White point of the Levels **Stretch** button (`specs/0135` criterion 5): the
/// 0.5 % and 99.5 % percentiles of `histogram`, as a preset clips them.
#[must_use]
pub fn stretch_points(histogram: &Histogram) -> (u8, u8) {
    clip_points(histogram, STRETCH_CLIP_TENTHS)
}

/// The share Stretch clips at each end, in tenths of a percent.
const STRETCH_CLIP_TENTHS: i64 = 5;

/// Black at percentile `clip_tenths` and White at `100 % − clip`, each rounded to whole percent,
/// in range and at least 2 apart.
fn clip_points(histogram: &Histogram, clip_tenths: i64) -> (u8, u8) {
    let clip = u16::try_from(clip_tenths.clamp(0, 500)).unwrap_or(0);
    let to_pct = |g: u8| round_half_up(100 * i64::from(g), 255);
    let black = to_pct(histogram.percentile(clip)).clamp(0, 98);
    let white = to_pct(histogram.percentile(1000 - clip)).clamp(black + 2, 100);
    let black = black.min(white - 2);
    (to_u8(black), to_u8(white))
}

/// The Levels step of a preset: Black at percentile `clip` (tenths of a percent) and White at
/// `100 % − clip`, each rounded to whole percent; Gamma `ln(m′) / ln(t)` for the grey `m′` the
/// target grey becomes after the stretch, rounded to hundredths and clamped to 0.10 to 4.00,
/// 1.00 when `m′` lies within one grey of black or white (ADR 0073 §5).
fn levels_step(
    histogram: &Histogram,
    clip_tenths: i64,
    target: Target,
    v: &dyn Fn(i64, i64) -> i64,
) -> ManualStep {
    let (black, white) = clip_points(histogram, clip_tenths);
    let aim = match target {
        Target::None => None,
        Target::Median(t0, t100) => Some((histogram.median(), v(t0, t100))),
        Target::Otsu => Some((histogram.otsu(), 50)),
    };
    let gamma_centi = aim.map_or(100, |(grey, target_pct)| {
        let stretched = levels_table(black, white, 100)[usize::from(grey)];
        gamma_for(stretched, target_pct)
    });
    ManualStep::Levels {
        black_pct: black,
        white_pct: white,
        gamma_centi,
    }
}

/// `round(100 · ln(m / 255) / ln(t / 100))` clamped to 10 to 400 (`libm::log`); 100 when `m`
/// lies within one grey of 0 or 255 or the target is not inside 0 to 100 %.
fn gamma_for(m: u8, target_pct: i64) -> u16 {
    if m <= 1 || m >= 254 || !(1..100).contains(&target_pct) {
        return 100;
    }
    #[allow(clippy::cast_precision_loss)]
    // invariant: a small whole number, exact in f64.
    let t = target_pct as f64 / 100.0;
    let gamma = libm::log(f64::from(m) / 255.0) / libm::log(t);
    let centi = (gamma * 100.0 + 0.5).floor();
    // invariant: clamped into 10..=400 before the cast.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let centi = centi.clamp(10.0, 400.0) as u16;
    centi
}

/// `n / d` rounded half up (towards +infinity at a half), `d > 0`.
const fn round_half_up(n: i64, d: i64) -> i64 {
    (2 * n + d).div_euclid(2 * d)
}

fn to_u8(v: i64) -> u8 {
    u8::try_from(v.clamp(0, 255)).unwrap_or(u8::MAX)
}

fn to_u16(v: i64) -> u16 {
    u16::try_from(v.clamp(0, i64::from(u16::MAX))).unwrap_or(u16::MAX)
}

fn to_u32(v: i64) -> u32 {
    u32::try_from(v.clamp(0, i64::from(u32::MAX))).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_image_core::GreyPlane;

    fn analysis(noise_tenths: u32) -> Analysis {
        let ramp = GreyPlane {
            width: 256,
            height: 4,
            grey: (0..1024_u32)
                .map(|i| u8::try_from(i % 256).unwrap())
                .collect(),
        };
        Analysis {
            noise_tenths,
            ..Analysis::of_plane(&ramp)
        }
    }

    #[test]
    fn keys_round_trip_and_expert_derives_nothing() {
        for p in Preset::ALL {
            assert_eq!(Preset::from_key(p.key()), Some(p));
            assert!(p.tooltip().starts_with(p.name()));
        }
        assert_eq!(Preset::Expert.derive(50, &analysis(0), 100), Vec::new());
        assert_eq!(Preset::Expert.dither_edits(100), Vec::new());
    }

    #[test]
    fn strength_moves_each_field_from_gentle_to_strong() {
        let a = analysis(0);
        let sharpen = |s| {
            Preset::Photo
                .derive(s, &a, 100)
                .into_iter()
                .find(|st| st.kind() == StepKind::Sharpen)
                .unwrap()
        };
        let amount = |st: ManualStep| match st {
            ManualStep::Sharpen { amount_pct, .. } => amount_pct,
            _ => unreachable!(),
        };
        assert_eq!(amount(sharpen(0)), 40);
        assert_eq!(amount(sharpen(50)), 120);
        assert_eq!(amount(sharpen(100)), 200);
        assert_eq!(amount(sharpen(33)), 93, "40 + round(52.8)");
    }

    #[test]
    fn photo_denoises_only_a_noisy_picture() {
        let has = |noise| {
            Preset::Photo
                .derive(50, &analysis(noise), 100)
                .iter()
                .any(|s| s.kind() == StepKind::Denoise)
        };
        assert!(!has(30));
        assert!(has(31));
    }

    #[test]
    fn the_mm_fields_follow_the_interval_and_nothing_else() {
        let a = analysis(50);
        for p in Preset::ALL {
            let (fine, coarse) = (p.derive(60, &a, 100), p.derive(60, &a, 200));
            assert_eq!(fine.len(), coarse.len());
            for (f, c) in fine.iter().zip(&coarse) {
                for ((key, x), (_, y)) in f.fields().iter().zip(c.fields()) {
                    if key.ends_with("_um") {
                        assert!(y >= *x, "{p:?} {key}");
                    } else {
                        assert_eq!(*x, y, "{p:?} {key}");
                    }
                }
            }
        }
        let radius = |i| match Preset::Photo.derive(50, &a, i)[2] {
            ManualStep::LocalContrast { radius_um, .. } => radius_um,
            _ => unreachable!(),
        };
        assert_eq!((radius(100), radius(200)), (10_000, 20_000));
        assert_eq!(radius(300), 20_000, "clamped to 20 mm");
    }

    #[test]
    fn stretch_takes_the_half_percent_ends_and_keeps_two_apart() {
        // A ramp of 1,024 pixels: 0.5 % is grey 1 (0 %), 99.5 % is grey 254 (100 %).
        assert_eq!(stretch_points(&analysis(0).histogram), (0, 100));
        let flat = GreyPlane {
            width: 4,
            height: 4,
            grey: vec![128; 16],
        };
        assert_eq!(
            stretch_points(&Analysis::of_plane(&flat).histogram),
            (50, 52),
            "a flat picture: Black 50 %, White 2 above"
        );
    }

    #[test]
    fn levels_clip_the_percentiles_and_aim_the_median() {
        // A ramp: percentile 2 % is grey 5, 98 % is grey 250; the median 127 stretches to 127.
        let a = analysis(0);
        let ManualStep::Levels {
            black_pct,
            white_pct,
            gamma_centi,
        } = Preset::Photo.derive(100, &a, 100)[0]
        else {
            panic!("Levels first");
        };
        assert_eq!((black_pct, white_pct), (2, 98));
        // ln(127/255) / ln(0.42) = 0.80.
        assert!((79..=81).contains(&gamma_centi), "{gamma_centi}");
        assert_eq!(gamma_for(0, 50), 100);
        assert_eq!(gamma_for(128, 50), 99);
    }

    #[test]
    fn a_pick_writes_the_dither_and_its_fields() {
        assert_eq!(
            Preset::DotRaster.dither_edits(100),
            vec![
                ImageEdit::Dither(Dither::Halftone),
                ImageEdit::ScreenSize(80),
                ImageEdit::ScreenAngle(45)
            ]
        );
        assert_eq!(
            Preset::Newspaper.dither_edits(50)[1],
            ImageEdit::ScreenSize(60)
        );
        assert_eq!(
            Preset::BlackWhite.dither_edits(100),
            vec![
                ImageEdit::Dither(Dither::Threshold),
                ImageEdit::Threshold(50)
            ]
        );
    }
}
