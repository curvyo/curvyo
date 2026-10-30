//! The trace values (`specs/0025-raster-trace` criteria 41, 59, 60; `specs/0089` adrs.md §7):
//! the choices, the presets, the values one trace takes and the geometry they map to.

use curvyo_document_core::Length;

use crate::schema::{
    DEFAULT_CORNERS, DEFAULT_SMOOTH, DEFAULT_SPECKS_MM, DEFAULT_TOLERANCE_PX, MAX_SPECKS_MM,
    MAX_TOLERANCE_PX, MIN_TOLERANCE_PX,
};

/// The median filter before the Threshold (criterion 41, `adrs.md` note 2026-10-28 point 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Denoise {
    /// No filter.
    #[default]
    Off,
    /// A 3 x 3 px median.
    Low,
    /// A 5 x 5 px median.
    High,
}

impl Denoise {
    /// The choices in schema order.
    pub const LABELS: &'static [&'static str] = &["Off", "Low", "High"];

    /// The choice at schema index `index` (`Off` for anything else).
    #[must_use]
    pub const fn from_index(index: u8) -> Self {
        match index {
            1 => Self::Low,
            2 => Self::High,
            _ => Self::Off,
        }
    }

    /// The schema index.
    #[must_use]
    pub const fn index(self) -> u8 {
        self as u8
    }

    /// The median window's radius in pixels (0 for none).
    pub(crate) const fn radius(self) -> usize {
        match self {
            Self::Off => 0,
            Self::Low => 1,
            Self::High => 2,
        }
    }
}

/// The 2x copy before tracing (criterion 41).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Upscale {
    /// Pictures under [`UPSCALE_AUTO_BELOW_PX`] are traced from a 2x copy.
    Auto,
    /// Never.
    #[default]
    Off,
}

/// Upscale Auto doubles pictures with fewer pixels than this (1 Mpx).
pub const UPSCALE_AUTO_BELOW_PX: u64 = 1_000_000;

impl Upscale {
    /// The choices in schema order.
    pub const LABELS: &'static [&'static str] = &["Auto", "Off"];

    /// The choice at schema index `index` (`Off` for anything else).
    #[must_use]
    pub const fn from_index(index: u8) -> Self {
        match index {
            0 => Self::Auto,
            _ => Self::Off,
        }
    }

    /// The schema index.
    #[must_use]
    pub const fn index(self) -> u8 {
        self as u8
    }

    /// Whether a picture of `width` x `height` is traced from a 2x copy.
    #[must_use]
    pub fn applies(self, width: u32, height: u32) -> bool {
        self == Self::Auto && u64::from(width) * u64::from(height) < UPSCALE_AUTO_BELOW_PX
    }
}

/// A named set of Trace settings for what the maker does with the paths (criteria 59 and 60):
/// a pick sets these values and nothing else, never the Threshold.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Preset {
    /// The stable key the app setting stores.
    pub key: &'static str,
    /// The name in the menu and on the button.
    pub label: &'static str,
    /// The menu's second line.
    pub detail: &'static str,
    /// Smooth, 0 to 100.
    pub smooth: u8,
    /// Tolerance, source px.
    pub tolerance_px: f64,
    /// Specks, mm.
    pub specks_mm: f64,
    /// Corners, 0 to 100.
    pub corners: u8,
    /// Upscale.
    pub upscale: Upscale,
    /// Denoise.
    pub denoise: Denoise,
}

impl Preset {
    /// `params` with this preset's values; Threshold, Invert and Ignore background stay.
    #[must_use]
    pub fn apply(&self, params: TraceParams) -> TraceParams {
        TraceParams {
            smooth: self.smooth,
            tolerance: PixelLength::from_px(self.tolerance_px),
            specks: Length::from_mm(self.specks_mm),
            corners: self.corners,
            upscale: self.upscale,
            denoise: self.denoise,
            ..params
        }
    }
}

/// The presets in menu order (criteria 59 and 60). Logo / Line art holds the first-use defaults,
/// so slice 1's results do not change (criterion 65).
pub const PRESETS: &[Preset] = &[
    Preset {
        key: "logo",
        label: "Logo / Line art",
        detail: "Sharp corners, clean curves.",
        smooth: DEFAULT_SMOOTH,
        tolerance_px: DEFAULT_TOLERANCE_PX,
        specks_mm: DEFAULT_SPECKS_MM,
        corners: DEFAULT_CORNERS,
        upscale: Upscale::Off,
        denoise: Denoise::Off,
    },
    Preset {
        key: "photo",
        label: "Photo",
        detail: "Many shapes. Photos trace roughly.",
        smooth: 50,
        tolerance_px: 0.5,
        specks_mm: 0.1,
        corners: 30,
        upscale: Upscale::Off,
        denoise: Denoise::Off,
    },
    Preset {
        key: "silhouette",
        label: "Silhouette for cutting/carving",
        detail: "Closed, simple, few nodes.",
        smooth: 100,
        tolerance_px: 1.5,
        specks_mm: 2.0,
        corners: 70,
        upscale: Upscale::Off,
        denoise: Denoise::Low,
    },
    Preset {
        key: "engraving",
        label: "Engraving",
        detail: "Keeps fine lines.",
        smooth: 80,
        tolerance_px: 0.35,
        specks_mm: 0.1,
        corners: 70,
        upscale: Upscale::Auto,
        denoise: Denoise::Off,
    },
];

/// A distance in pixels of the traced image before Upscale (source pixels).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct PixelLength(f64);

impl PixelLength {
    /// `px` source pixels.
    #[must_use]
    pub const fn from_px(px: f64) -> Self {
        Self(px)
    }

    /// The distance in source pixels.
    #[must_use]
    pub const fn as_px(self) -> f64 {
        self.0
    }
}

/// The values one trace takes (black and white).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceParams {
    /// Pixels whose luma is below this are foreground (above or equal with `invert`).
    pub threshold: u8,
    /// 0 to 100: how strongly the pixel border is calmed before the fit.
    pub smooth: u8,
    /// The largest distance of an outline from the calmed pixel border, source px.
    pub tolerance: PixelLength,
    /// Regions and holes whose area is below the square of this side are removed.
    pub specks: Length,
    /// 0 to 100; higher means a bend becomes a sharp corner more readily.
    pub corners: u8,
    /// The 2x copy before tracing.
    pub upscale: Upscale,
    /// The median filter before the Threshold.
    pub denoise: Denoise,
    /// Trace the light parts: the mask is flipped.
    pub invert: bool,
    /// Drop the foreground regions that touch the picture's border.
    pub ignore_background: bool,
}

impl TraceParams {
    /// The values of a first use with `threshold`: the first preset's (Logo / Line art), Invert
    /// and Ignore background off.
    #[must_use]
    pub const fn with_threshold(threshold: u8) -> Self {
        Self {
            threshold,
            smooth: DEFAULT_SMOOTH,
            tolerance: PixelLength::from_px(DEFAULT_TOLERANCE_PX),
            specks: Length::from_mm(DEFAULT_SPECKS_MM),
            corners: DEFAULT_CORNERS,
            upscale: Upscale::Off,
            denoise: Denoise::Off,
            invert: false,
            ignore_background: false,
        }
    }

    /// Smooth, clamped to its range.
    #[must_use]
    pub fn smooth_clamped(self) -> u8 {
        self.smooth.min(100)
    }

    /// The Tolerance in source pixels, clamped to its range (a non-finite value is the default).
    #[must_use]
    pub fn tolerance_px(self) -> f64 {
        let px = self.tolerance.as_px();
        if px.is_finite() {
            px.clamp(MIN_TOLERANCE_PX, MAX_TOLERANCE_PX)
        } else {
            DEFAULT_TOLERANCE_PX
        }
    }

    /// The Specks side in millimetres, clamped to its range (a non-finite value is 0).
    #[must_use]
    pub fn specks_mm(self) -> f64 {
        let mm = self.specks.as_mm();
        if mm.is_finite() {
            mm.clamp(0.0, MAX_SPECKS_MM)
        } else {
            0.0
        }
    }
}

/// The geometry one trace fits with (`specs/0089` adrs.md §2, §7: input smoothing, deviation
/// tolerance and corner protection kept apart), in source pixels; built only by [`geometry`],
/// so no caller hands in an unmapped value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Geometry {
    /// The standard deviation of the binomial smoothing along a chain, source px.
    pub smoothing_px: f64,
    /// The largest distance of an outline from the contour before simplification, source px.
    pub tolerance_px: f64,
    /// The cosine of the corner angle: a turn sharper than it is a structural corner.
    pub corner_cos: f64,
}

/// The smoothing per step of Smooth, source px (Smooth 50, Photo: 1.5 px; 80: 2.4 px; 100: 3 px).
const SMOOTHING_PER_STEP: f64 = 0.03;

/// The geometry of `smooth` (0 to 100: the standard deviation of the smoothing, 0.03 px per
/// step), the deviation tolerance `tolerance_px` and `corners` (0 to 100: the corner angle
/// 200° − 2 × Corners within 10° to 180°), each mapped to one concern only (`specs/0089`
/// adrs.md §7, Frage 85 B).
pub(crate) fn geometry(smooth: u8, tolerance_px: f64, corners: u8) -> Geometry {
    let smoothing_px = SMOOTHING_PER_STEP * f64::from(smooth.min(100));
    Geometry {
        smoothing_px,
        tolerance_px,
        corner_cos: corner_cos(corners),
    }
}

/// The cosine of the corner angle `200° − 2 × Corners` (within 10° to 180°), through a fixed
/// polynomial in basic operations, never `f64::cos` (ADR 0023 §4).
pub(crate) fn corner_cos(corners: u8) -> f64 {
    let degrees = (200 - 2 * i32::from(corners.min(100))).clamp(10, 180);
    // cos(x) = −cos(180° − x), so the series runs on 0 to 90° only.
    let (folded, sign) = if degrees > 90 {
        (180 - degrees, -1.0)
    } else {
        (degrees, 1.0)
    };
    let x = f64::from(folded) * (std::f64::consts::PI / 180.0);
    let x2 = x * x;
    // Taylor series to x^20 (error below 1e-15 on 0 to π/2), Horner in x².
    sign * TAYLOR_COS.iter().rev().fold(0.0, |sum, c| sum * x2 + c)
}

/// The Taylor coefficients of cos in x²: (−1)^k / (2k)!.
const TAYLOR_COS: [f64; 11] = [
    1.0,
    -1.0 / 2.0,
    1.0 / 24.0,
    -1.0 / 720.0,
    1.0 / 40_320.0,
    -1.0 / 3_628_800.0,
    1.0 / 479_001_600.0,
    -1.0 / 87_178_291_200.0,
    1.0 / 20_922_789_888_000.0,
    -1.0 / 6_402_373_705_728_000.0,
    1.0 / 2_432_902_008_176_640_000.0,
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{ADVANCED_PARAMS, PARAMS};

    /// Criteria 45, 59 and 60: the preset table in menu order with the customer's texts, its
    /// values within the schema's ranges, Logo / Line art equal to the first use.
    #[test]
    fn the_presets_match_criteria_59_and_60() {
        let menu: Vec<_> = PRESETS.iter().map(|p| (p.key, p.label, p.detail)).collect();
        assert_eq!(
            menu,
            [
                ("logo", "Logo / Line art", "Sharp corners, clean curves."),
                ("photo", "Photo", "Many shapes. Photos trace roughly."),
                (
                    "silhouette",
                    "Silhouette for cutting/carving",
                    "Closed, simple, few nodes."
                ),
                ("engraving", "Engraving", "Keeps fine lines."),
            ]
        );
        let spec = |name: &str| {
            PARAMS
                .iter()
                .chain(ADVANCED_PARAMS)
                .find(|p| p.name == name)
                .unwrap()
        };
        for preset in PRESETS {
            for (name, value) in [
                ("smooth", f64::from(preset.smooth)),
                ("specks", preset.specks_mm),
                ("corners", f64::from(preset.corners)),
                ("upscale", f64::from(preset.upscale.index())),
                ("denoise", f64::from(preset.denoise.index())),
            ] {
                let p = spec(name);
                assert!((p.min..=p.max).contains(&value), "{} {name}", preset.key);
            }
        }
        let first = TraceParams::with_threshold(140);
        assert_eq!(PRESETS[0].apply(first), first);
        // A pick never sets the Threshold or the switches.
        let mine = TraceParams {
            invert: true,
            ignore_background: true,
            ..first
        };
        for preset in PRESETS {
            let picked = preset.apply(mine);
            assert_eq!(picked.threshold, 140);
            assert!(picked.invert && picked.ignore_background);
        }
        // The values of criterion 60 (step 0 of slice 2, `adrs.md` 2026-10-28 second note).
        let values: Vec<_> = PRESETS
            .iter()
            .map(|p| (p.smooth, p.specks_mm, p.corners, p.upscale, p.denoise))
            .collect();
        assert_eq!(
            values,
            [
                (80, 0.5, 70, Upscale::Off, Denoise::Off),
                (50, 0.1, 30, Upscale::Off, Denoise::Off),
                (100, 2.0, 70, Upscale::Off, Denoise::Low),
                (80, 0.1, 70, Upscale::Auto, Denoise::Off),
            ]
        );
    }

    #[test]
    fn upscale_auto_applies_under_one_megapixel() {
        assert!(Upscale::Auto.applies(999, 1000));
        assert!(!Upscale::Auto.applies(1000, 1000));
        assert!(!Upscale::Off.applies(10, 10));
        assert_eq!(Upscale::from_index(0), Upscale::Auto);
        assert_eq!(Denoise::from_index(2), Denoise::High);
        assert_eq!(Denoise::from_index(9), Denoise::Off);
    }

    #[test]
    fn smooth_tolerance_and_corners_each_map_to_one_concern() {
        assert_eq!(geometry(0, 0.5, 70).smoothing_px, 0.0);
        assert!((geometry(50, 0.5, 70).smoothing_px - 1.5).abs() < 1e-12);
        assert_eq!(geometry(200, 0.5, 70), geometry(100, 0.5, 70));
        for s in 0..100 {
            assert!(geometry(s, 0.5, 70).smoothing_px < geometry(s + 1, 0.5, 70).smoothing_px);
        }
        let g = geometry(80, 0.7, 20);
        assert_eq!(g.smoothing_px, geometry(80, 0.2, 90).smoothing_px);
        assert_eq!(g.tolerance_px, 0.7);
        assert_eq!(g.corner_cos, corner_cos(20));
    }

    #[test]
    fn the_tolerance_is_clamped() {
        let mut params = TraceParams::with_threshold(128);
        assert_eq!(params.tolerance_px(), DEFAULT_TOLERANCE_PX);
        params.tolerance = PixelLength::from_px(f64::NAN);
        assert_eq!(params.tolerance_px(), DEFAULT_TOLERANCE_PX);
        params.tolerance = PixelLength::from_px(99.0);
        assert_eq!(params.tolerance_px(), MAX_TOLERANCE_PX);
        params.tolerance = PixelLength::from_px(0.0);
        assert_eq!(params.tolerance_px(), MIN_TOLERANCE_PX);
    }

    #[test]
    fn specks_are_clamped() {
        let mut params = TraceParams::with_threshold(128);
        params.specks = Length::from_mm(f64::NAN);
        assert!(params.specks_mm().abs() < f64::EPSILON);
        params.specks = Length::from_mm(99.0);
        assert!((params.specks_mm() - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn the_corner_cosine_is_the_cosine_of_the_corner_angle() {
        for corners in 0..=100_u8 {
            let degrees = (200.0 - 2.0 * f64::from(corners)).clamp(10.0, 180.0);
            let exact = degrees.to_radians().cos();
            assert!((corner_cos(corners) - exact).abs() < 1e-14, "{corners}");
        }
        assert!((corner_cos(70) - 0.5).abs() < 1e-15);
    }
}
