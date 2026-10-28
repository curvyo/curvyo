//! The trace settings as data (`specs/0025-raster-trace` criteria 4 to 6, 41, 45, 59, 60): the
//! schema the bar's controls are generated from, the presets, the values one trace takes and the
//! tracer settings they map to.

use curvyo_document_core::Length;

/// What kind of value a setting holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    /// A whole number.
    Integer,
    /// A length in millimetres.
    Length,
    /// One of the listed choices; the value is the choice's index.
    Choice(&'static [&'static str]),
    /// On or off; the value is 0 or 1.
    Toggle,
}

/// The unit a setting is shown in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamUnit {
    /// No unit: a grey level or a share.
    None,
    /// Millimetres.
    Millimetre,
}

/// One setting of the Trace tool: everything a bar control needs to be drawn and stepped.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParamSpec {
    /// The setting's key, as the host stores it.
    pub name: &'static str,
    /// The label inside the field.
    pub label: &'static str,
    /// What kind of value it holds.
    pub kind: ParamKind,
    /// The unit it is shown in.
    pub unit: ParamUnit,
    /// The smallest value.
    pub min: f64,
    /// The largest value.
    pub max: f64,
    /// The value of a first use; the Threshold's is replaced by the image's automatic value.
    pub default: f64,
    /// Up and Down change the value by this.
    pub step: f64,
    /// Shift+Up and Shift+Down change it by this.
    pub big_step: f64,
    /// Ctrl+Up and Ctrl+Down change it by this (the same as `step` where there is no finer one).
    pub fine_step: f64,
    /// The group of the bar it sits in.
    pub group: &'static str,
    /// Whether it sits in the Advanced block (criterion 41).
    pub advanced: bool,
}

/// The default Smooth (criterion 4). Step 0: criterion 21 holds from Smooth 30 up (mean 0.33 px
/// at 20, 0.27 px at 30, 0.25 px at 80); 80 gives fewer nodes, less wobble and a lower mean
/// deviation on the quality set than 30, so it stays (`specs/0025-raster-trace/plan.md`).
pub const DEFAULT_SMOOTH: u8 = 80;

/// The default Specks size in millimetres (criterion 4).
pub const DEFAULT_SPECKS_MM: f64 = 0.5;

/// The Threshold of a picture of one grey level, and of a first use before the image is read
/// (criterion 9).
pub const DEFAULT_THRESHOLD: u8 = 128;

/// The largest Specks size in millimetres.
pub const MAX_SPECKS_MM: f64 = 10.0;

/// The settings of the Trace tool in bar order (criteria 4, 5 and 6).
pub const PARAMS: &[ParamSpec] = &[
    ParamSpec {
        name: "threshold",
        label: "Threshold",
        kind: ParamKind::Integer,
        unit: ParamUnit::None,
        min: 0.0,
        max: 255.0,
        default: DEFAULT_THRESHOLD as f64,
        step: 1.0,
        big_step: 10.0,
        fine_step: 1.0,
        group: "threshold",
        advanced: false,
    },
    ParamSpec {
        name: "smooth",
        label: "Smooth",
        kind: ParamKind::Integer,
        unit: ParamUnit::None,
        min: 0.0,
        max: 100.0,
        default: DEFAULT_SMOOTH as f64,
        step: 1.0,
        big_step: 10.0,
        fine_step: 1.0,
        group: "smooth",
        advanced: false,
    },
    ParamSpec {
        name: "specks",
        label: "Ignore below",
        kind: ParamKind::Length,
        unit: ParamUnit::Millimetre,
        min: 0.0,
        max: MAX_SPECKS_MM,
        default: DEFAULT_SPECKS_MM,
        step: 0.1,
        big_step: 1.0,
        fine_step: 0.01,
        group: "specks",
        advanced: false,
    },
];

/// The controls of the Advanced block in its order, the two toggles first (criteria 41 and 45): the same schema as
/// [`PARAMS`], kept apart so the bar's main rows stay the three of slice 1.
pub const ADVANCED_PARAMS: &[ParamSpec] = &[
    ParamSpec {
        name: "invert",
        label: "Invert",
        kind: ParamKind::Toggle,
        unit: ParamUnit::None,
        min: 0.0,
        max: 1.0,
        default: 0.0,
        step: 1.0,
        big_step: 1.0,
        fine_step: 1.0,
        group: "advanced",
        advanced: true,
    },
    ParamSpec {
        name: "ignore_background",
        label: "Ignore background",
        kind: ParamKind::Toggle,
        unit: ParamUnit::None,
        min: 0.0,
        max: 1.0,
        default: 0.0,
        step: 1.0,
        big_step: 1.0,
        fine_step: 1.0,
        group: "advanced",
        advanced: true,
    },
    ParamSpec {
        name: "denoise",
        label: "Denoise",
        kind: ParamKind::Choice(Denoise::LABELS),
        unit: ParamUnit::None,
        min: 0.0,
        max: 2.0,
        default: 0.0,
        step: 1.0,
        big_step: 1.0,
        fine_step: 1.0,
        group: "advanced",
        advanced: true,
    },
    ParamSpec {
        name: "upscale",
        label: "Upscale",
        kind: ParamKind::Choice(Upscale::LABELS),
        unit: ParamUnit::None,
        min: 0.0,
        max: 1.0,
        default: 1.0,
        step: 1.0,
        big_step: 1.0,
        fine_step: 1.0,
        group: "advanced",
        advanced: true,
    },
    ParamSpec {
        name: "corners",
        label: "Corners",
        kind: ParamKind::Integer,
        unit: ParamUnit::None,
        min: 0.0,
        max: 100.0,
        default: DEFAULT_CORNERS as f64,
        step: 1.0,
        big_step: 10.0,
        fine_step: 1.0,
        group: "advanced",
        advanced: true,
    },
];

/// The default Corners: a bend sharper than 60° stays a corner, slice 1's fixed angle.
pub const DEFAULT_CORNERS: u8 = 70;

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
        specks_mm: 0.1,
        corners: 70,
        upscale: Upscale::Auto,
        denoise: Denoise::Off,
    },
];

/// The values one trace takes (black and white).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceParams {
    /// Pixels whose luma is below this are foreground (above or equal with `invert`).
    pub threshold: u8,
    /// 0 to 100; higher means fewer nodes and a larger deviation.
    pub smooth: u8,
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

/// The settings of visioncortex's spline mode one trace runs with (ADR 0023 §3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CurveSettings {
    /// A bend sharper than this, in radians, stays a corner.
    pub corner_threshold: f64,
    /// The length of a smoothing segment in pixels.
    pub segment_length: f64,
    /// Smoothing passes at most.
    pub max_iterations: usize,
    /// A turn larger than this, in radians, splits the curve fit.
    pub splice_threshold: f64,
}

/// The shortest smoothing segment in pixels (Smooth 0).
const SEGMENT_MIN_PX: f64 = 3.5;

/// The longest smoothing segment in pixels (Smooth 100).
const SEGMENT_MAX_PX: f64 = 10.0;

/// The splice angle, degrees (fixed).
const SPLICE_DEGREES: f64 = 45.0;

/// The smoothing passes (fixed).
const MAX_ITERATIONS: usize = 10;

/// Smooth 0 to 100 mapped monotonically to the segment length 3.5 to 10 px, Corners 0 to 100 to
/// the corner angle 200° − 2 × Corners within 10° to 180° (Corners 70 is slice 1's 60°); the
/// splice angle and the passes are fixed (ADR 0023 §3).
pub(crate) fn curve_settings(smooth: u8, corners: u8) -> CurveSettings {
    let share = f64::from(smooth.min(100)) / 100.0;
    let corner_degrees = (200.0 - 2.0 * f64::from(corners.min(100))).clamp(10.0, 180.0);
    CurveSettings {
        corner_threshold: corner_degrees.to_radians(),
        segment_length: share.mul_add(SEGMENT_MAX_PX - SEGMENT_MIN_PX, SEGMENT_MIN_PX),
        max_iterations: MAX_ITERATIONS,
        splice_threshold: SPLICE_DEGREES.to_radians(),
    }
}

/// The node reduction's tolerance in mm for Smooth (`adrs.md` 2026-10-28, second note, point 1):
/// none up to 80, `(Smooth − 80) × 0.01 mm` above, so 0.2 mm at 100.
pub(crate) fn reduce_tolerance_mm(smooth: u8) -> f64 {
    f64::from(smooth.min(100).saturating_sub(80)) * 0.01
}

/// The cosine of the corner angle `200° − 2 × Corners` (within 10° to 180°) that `curve_settings`
/// gives visioncortex, through a fixed polynomial in basic operations, never `f64::cos`
/// (ADR 0023 §4).
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

    /// Criterion 6: the schema carries the defaults and ranges of criteria 4 and 5.
    #[test]
    fn the_schema_matches_criteria_4_and_5() {
        let names: Vec<_> = PARAMS.iter().map(|p| p.name).collect();
        assert_eq!(names, ["threshold", "smooth", "specks"]);
        let threshold = &PARAMS[0];
        assert_eq!((threshold.min, threshold.max), (0.0, 255.0));
        assert_eq!((threshold.step, threshold.big_step), (1.0, 10.0));
        let smooth = &PARAMS[1];
        assert_eq!((smooth.min, smooth.max, smooth.default), (0.0, 100.0, 80.0));
        assert_eq!((smooth.step, smooth.big_step), (1.0, 10.0));
        let specks = &PARAMS[2];
        assert_eq!(specks.label, "Ignore below");
        assert_eq!(specks.unit, ParamUnit::Millimetre);
        assert_eq!((specks.min, specks.max, specks.default), (0.0, 10.0, 0.5));
        assert_eq!(
            (specks.step, specks.big_step, specks.fine_step),
            (0.1, 1.0, 0.01)
        );
        assert!(PARAMS.iter().all(|p| !p.advanced));
    }

    /// Criterion 41: the Advanced block's controls, in order, with their ranges and defaults.
    #[test]
    fn the_advanced_controls_are_in_the_schema() {
        assert!(ADVANCED_PARAMS.iter().all(|p| p.advanced));
        let names: Vec<_> = ADVANCED_PARAMS.iter().map(|p| p.name).collect();
        assert_eq!(
            names,
            [
                "invert",
                "ignore_background",
                "denoise",
                "upscale",
                "corners"
            ]
        );
        let advanced = &ADVANCED_PARAMS[2..];
        assert_eq!(ADVANCED_PARAMS[0].kind, ParamKind::Toggle);
        assert_eq!(ADVANCED_PARAMS[1].label, "Ignore background");
        assert_eq!(advanced[0].kind, ParamKind::Choice(&["Off", "Low", "High"]));
        assert_eq!(advanced[1].kind, ParamKind::Choice(&["Auto", "Off"]));
        assert_eq!((advanced[2].min, advanced[2].max), (0.0, 100.0));
        // The schema's defaults are the first use: Logo / Line art, switches off.
        let first = TraceParams::with_threshold(DEFAULT_THRESHOLD);
        assert_eq!(advanced[0].default, f64::from(first.denoise.index()));
        assert_eq!(advanced[1].default, f64::from(first.upscale.index()));
        assert_eq!(advanced[2].default, f64::from(first.corners));
    }

    /// Criterion 43 (removed, Frage 74 A): neither the schema nor the values know a Node limit
    /// or a Full-size preview.
    #[test]
    fn there_is_no_node_limit_and_no_full_size_preview() {
        for p in PARAMS.iter().chain(ADVANCED_PARAMS) {
            let name = p.name.to_lowercase();
            let label = p.label.to_lowercase();
            assert!(
                !name.contains("node") && !label.contains("node"),
                "{}",
                p.name
            );
            assert!(
                !name.contains("full") && !label.contains("full"),
                "{}",
                p.name
            );
        }
        let fields = format!("{:?}", TraceParams::with_threshold(DEFAULT_THRESHOLD));
        assert!(!fields.contains("node_limit"), "{fields}");
        assert!(!fields.contains("full_size"), "{fields}");
    }

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
    fn corners_map_monotonically_to_the_corner_angle() {
        assert!((curve_settings(80, 70).corner_threshold - 60_f64.to_radians()).abs() < 1e-15);
        assert!((curve_settings(80, 0).corner_threshold - 180_f64.to_radians()).abs() < 1e-12);
        assert!((curve_settings(80, 100).corner_threshold - 10_f64.to_radians()).abs() < 1e-12);
        for c in 0..100 {
            assert!(
                curve_settings(80, c).corner_threshold
                    >= curve_settings(80, c + 1).corner_threshold
            );
        }
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
    fn smooth_maps_monotonically_to_the_segment_length() {
        assert!((curve_settings(0, 70).segment_length - 3.5).abs() < 1e-12);
        assert!((curve_settings(100, 70).segment_length - 10.0).abs() < 1e-12);
        assert!((curve_settings(200, 70).segment_length - 10.0).abs() < 1e-12);
        for s in 0..100 {
            assert!(
                curve_settings(s, 70).segment_length < curve_settings(s + 1, 70).segment_length
            );
        }
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

    #[test]
    fn the_reduction_tolerance_starts_above_smooth_80() {
        assert_eq!(reduce_tolerance_mm(0), 0.0);
        assert_eq!(reduce_tolerance_mm(80), 0.0);
        assert!((reduce_tolerance_mm(81) - 0.01).abs() < 1e-15);
        assert!((reduce_tolerance_mm(100) - 0.2).abs() < 1e-15);
        assert!((reduce_tolerance_mm(200) - 0.2).abs() < 1e-15);
    }
}
