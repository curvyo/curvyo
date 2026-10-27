//! The trace settings as data (`specs/0025-raster-trace` criteria 4 to 6): the schema the bar's
//! controls are generated from, the values one trace takes and the tracer settings Smooth maps to.

use curvyo_document_core::Length;

/// What kind of value a setting holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    /// A whole number.
    Integer,
    /// A length in millimetres.
    Length,
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
    /// Whether it sits in the Advanced block (none in slice 1).
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

/// The values one trace takes (black and white, slice 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceParams {
    /// Pixels whose luma is below this are foreground.
    pub threshold: u8,
    /// 0 to 100; higher means fewer nodes and a larger deviation.
    pub smooth: u8,
    /// Regions and holes whose area is below the square of this side are removed.
    pub specks: Length,
}

impl TraceParams {
    /// The values of a first use with `threshold`.
    #[must_use]
    pub const fn with_threshold(threshold: u8) -> Self {
        Self {
            threshold,
            smooth: DEFAULT_SMOOTH,
            specks: Length::from_mm(DEFAULT_SPECKS_MM),
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

/// The corner angle, degrees (fixed in slice 1).
const CORNER_DEGREES: f64 = 60.0;

/// The splice angle, degrees (fixed in slice 1).
const SPLICE_DEGREES: f64 = 45.0;

/// The smoothing passes (fixed in slice 1).
const MAX_ITERATIONS: usize = 10;

/// Smooth 0 to 100 mapped monotonically to the segment length 3.5 to 10 px; the angles and
/// passes are fixed in slice 1 (ADR 0023 §3).
pub(crate) fn curve_settings(smooth: u8) -> CurveSettings {
    let share = f64::from(smooth.min(100)) / 100.0;
    CurveSettings {
        corner_threshold: CORNER_DEGREES.to_radians(),
        segment_length: share.mul_add(SEGMENT_MAX_PX - SEGMENT_MIN_PX, SEGMENT_MIN_PX),
        max_iterations: MAX_ITERATIONS,
        splice_threshold: SPLICE_DEGREES.to_radians(),
    }
}

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

    #[test]
    fn smooth_maps_monotonically_to_the_segment_length() {
        assert!((curve_settings(0).segment_length - 3.5).abs() < 1e-12);
        assert!((curve_settings(100).segment_length - 10.0).abs() < 1e-12);
        assert!((curve_settings(200).segment_length - 10.0).abs() < 1e-12);
        for s in 0..100 {
            assert!(curve_settings(s).segment_length < curve_settings(s + 1).segment_length);
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
}
