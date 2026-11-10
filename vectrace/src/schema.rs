//! The settings schema of the Trace tool as data (`specs/0025-raster-trace` criteria 4 to 6, 41,
//! 45; `specs/0089` Frage 85 B): the fields the bar's controls are generated from, with their
//! defaults and steps.

use crate::params::{Denoise, Selection, Upscale};
use crate::select::DEFAULT_PICK_TOLERANCE;

/// What kind of value a setting holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    /// A whole number.
    Integer,
    /// A length, in the setting's unit.
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
    /// Pixels of the traced image (before Upscale).
    Pixel,
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

/// The default deviation tolerance in source pixels (`specs/0089` Frage 85 B): the largest
/// distance of an outline from the smoothed pixel border.
pub const DEFAULT_TOLERANCE_PX: f64 = 0.5;

/// The smallest deviation tolerance in source pixels.
pub const MIN_TOLERANCE_PX: f64 = 0.1;

/// The largest deviation tolerance in source pixels.
pub const MAX_TOLERANCE_PX: f64 = 3.0;

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
        name: "tolerance",
        label: "Tolerance",
        kind: ParamKind::Length,
        unit: ParamUnit::Pixel,
        min: MIN_TOLERANCE_PX,
        max: MAX_TOLERANCE_PX,
        default: DEFAULT_TOLERANCE_PX,
        step: 0.05,
        big_step: 0.5,
        fine_step: 0.01,
        group: "tolerance",
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

/// The selection method and its colour tolerance (`specs/0089` criterion 14): the Method menu
/// and the field that takes the Threshold's place in the colour methods.
pub const METHOD_PARAMS: &[ParamSpec] = &[
    ParamSpec {
        name: "method",
        label: "Method",
        kind: ParamKind::Choice(Selection::LABELS),
        unit: ParamUnit::None,
        min: 0.0,
        max: 2.0,
        default: 0.0,
        step: 1.0,
        big_step: 1.0,
        fine_step: 1.0,
        group: "method",
        advanced: false,
    },
    ParamSpec {
        name: "colour_tolerance",
        label: "Colour tolerance",
        kind: ParamKind::Integer,
        unit: ParamUnit::None,
        min: 0.0,
        max: 100.0,
        default: DEFAULT_PICK_TOLERANCE as f64,
        step: 1.0,
        big_step: 10.0,
        fine_step: 1.0,
        group: "method",
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
        label: "Drop border shapes",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::TraceParams;

    /// Criterion 6: the schema carries the defaults and ranges of criteria 4 and 5, plus the
    /// Tolerance of `specs/0089` Frage 85 B after Smooth.
    #[test]
    fn the_schema_matches_criteria_4_and_5() {
        let names: Vec<_> = PARAMS.iter().map(|p| p.name).collect();
        assert_eq!(names, ["threshold", "smooth", "tolerance", "specks"]);
        let [method, colour] = METHOD_PARAMS else {
            panic!("two method settings")
        };
        assert_eq!(method.name, "method");
        assert_eq!(
            method.kind,
            ParamKind::Choice(&["Brightness", "Remove background", "Pick colours"])
        );
        assert_eq!(colour.label, "Colour tolerance");
        assert_eq!((colour.min, colour.max, colour.default), (0.0, 100.0, 19.0));
        let tolerance = &PARAMS[2];
        assert_eq!(tolerance.unit, ParamUnit::Pixel);
        assert_eq!(
            (tolerance.min, tolerance.max, tolerance.default),
            (0.1, 3.0, 0.5)
        );
        let threshold = &PARAMS[0];
        assert_eq!((threshold.min, threshold.max), (0.0, 255.0));
        assert_eq!((threshold.step, threshold.big_step), (1.0, 10.0));
        let smooth = &PARAMS[1];
        assert_eq!((smooth.min, smooth.max, smooth.default), (0.0, 100.0, 80.0));
        assert_eq!((smooth.step, smooth.big_step), (1.0, 10.0));
        let specks = &PARAMS[3];
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
        // Frage 88 A: the toggle's label; its stored key stays.
        assert_eq!(ADVANCED_PARAMS[1].label, "Drop border shapes");
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
        for p in PARAMS.iter().chain(ADVANCED_PARAMS).chain(METHOD_PARAMS) {
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
}
