//! The ranges and words of each stored adjustment field (ADR 0073 §2; `specs/0135` criterion
//! 5).

use crate::steps::typed_steps;

/// One stored field: its key, its words, and its range in stored units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldSpec {
    /// The stored key; the unit is in its suffix (`_pct`, `_centi`, `_um`).
    pub key: &'static str,
    /// The label inside the field.
    pub label: &'static str,
    /// The unit shown inside the field.
    pub unit: &'static str,
    /// Stored units per field step.
    pub step: i64,
    /// Field steps per shown unit (100 for two decimals of mm when the step is 10 µm).
    pub per_unit: i64,
    /// The smallest stored value.
    pub min: i64,
    /// The largest stored value.
    pub max: i64,
    /// The message of a typed value out of range (criterion 5, as `0123` criterion 12).
    pub range_text: &'static str,
}

impl FieldSpec {
    /// `value` (stored units) rounded half up to the field's step and clamped to its range.
    #[must_use]
    pub const fn clamped(self, value: i64) -> i64 {
        let stepped = (2 * value + self.step).div_euclid(2 * self.step) * self.step;
        if stepped < self.min {
            self.min
        } else if stepped > self.max {
            self.max
        } else {
            stepped
        }
    }

    /// The stored value of `value` typed in the shown unit, rounded to the field's step; a
    /// value out of range is refused before rounding.
    ///
    /// # Errors
    /// The range text, also for a value that is no number.
    pub fn typed(self, value: f64) -> Result<i64, &'static str> {
        #[allow(clippy::cast_precision_loss)]
        // invariant: every range here is a few thousand steps, exact in f64.
        let (per, min, max) = (
            self.per_unit as f64,
            (self.min / self.step) as f64,
            (self.max / self.step) as f64,
        );
        typed_steps(value, per, min, max)
            .map(|steps| steps * self.step)
            .ok_or(self.range_text)
    }

    /// The stored `value` in the shown unit.
    #[must_use]
    pub fn shown(self, value: i64) -> f64 {
        #[allow(clippy::cast_precision_loss)]
        // invariant: stored values are a few thousand steps, exact in f64.
        let shown = (value / self.step) as f64 / self.per_unit as f64;
        shown
    }
}

pub(crate) const fn pct(
    key: &'static str,
    label: &'static str,
    min: i64,
    max: i64,
    range_text: &'static str,
) -> FieldSpec {
    FieldSpec {
        key,
        label,
        unit: "%",
        step: 1,
        per_unit: 1,
        min,
        max,
        range_text,
    }
}

pub(crate) const DENOISE_STRENGTH: FieldSpec = pct(
    "strength_pct",
    "Strength",
    0,
    100,
    "Strength is 0 to 100 %.",
);
pub(crate) const DENOISE_DETAIL: FieldSpec = pct(
    "detail_pct",
    "Detail kept",
    0,
    100,
    "Detail kept is 0 to 100 %.",
);
pub(crate) const LEVELS_BLACK: FieldSpec =
    pct("black_pct", "Black", 0, 98, "Black point is 0 to 98 %.");
pub(crate) const LEVELS_WHITE: FieldSpec =
    pct("white_pct", "White", 2, 100, "White point is 2 to 100 %.");
pub(crate) const LEVELS_GAMMA: FieldSpec = FieldSpec {
    key: "gamma_centi",
    label: "Gamma",
    unit: "",
    step: 1,
    per_unit: 100,
    min: 10,
    max: 400,
    range_text: "Gamma is 0.10 to 4.00.",
};
pub(crate) const LOCAL_RADIUS: FieldSpec = FieldSpec {
    key: "radius_um",
    label: "Radius",
    unit: "mm",
    step: 100,
    per_unit: 10,
    min: 200,
    max: 20_000,
    range_text: "Radius is 0.2 to 20.0 mm.",
};
pub(crate) const LOCAL_AMOUNT: FieldSpec =
    pct("amount_pct", "Amount", 0, 200, "Amount is 0 to 200 %.");
pub(crate) const CONTOURS_WIDTH: FieldSpec = FieldSpec {
    key: "width_um",
    label: "Line width",
    unit: "mm",
    step: 10,
    per_unit: 100,
    min: 50,
    max: 2_000,
    range_text: "Line width is 0.05 to 2.00 mm.",
};
pub(crate) const CONTOURS_SENSITIVITY: FieldSpec = pct(
    "sensitivity_pct",
    "Sensitivity",
    0,
    100,
    "Sensitivity is 0 to 100 %.",
);
pub(crate) const SHARPEN_RADIUS: FieldSpec = FieldSpec {
    key: "radius_um",
    label: "Radius",
    unit: "mm",
    step: 10,
    per_unit: 100,
    min: 50,
    max: 5_000,
    range_text: "Radius is 0.05 to 5.00 mm.",
};
pub(crate) const SHARPEN_AMOUNT: FieldSpec =
    pct("amount_pct", "Amount", 0, 500, "Amount is 0 to 500 %.");
pub(crate) const SHARPEN_THRESHOLD: FieldSpec = pct(
    "threshold_pct",
    "Threshold",
    0,
    50,
    "Threshold is 0 to 50 %.",
);

pub(crate) const CROP_LEFT: FieldSpec = pct("left_pct", "Left", 0, 99, "Left is 0 to 99 %.");
pub(crate) const CROP_TOP: FieldSpec = pct("top_pct", "Top", 0, 99, "Top is 0 to 99 %.");
pub(crate) const CROP_WIDTH: FieldSpec = pct("width_pct", "Width", 1, 100, "Width is 1 to 100 %.");
pub(crate) const CROP_HEIGHT: FieldSpec =
    pct("height_pct", "Height", 1, 100, "Height is 1 to 100 %.");
pub(crate) const RESIZE_WIDTH: FieldSpec = FieldSpec {
    key: "width_px",
    label: "Width",
    unit: "px",
    step: 1,
    per_unit: 1,
    min: 16,
    max: 8_192,
    range_text: "Width is 16 to 8192 px.",
};
pub(crate) const MIX_RED: FieldSpec = pct("red_pct", "Red", -200, 200, "Red is -200 to 200 %.");
pub(crate) const MIX_GREEN: FieldSpec =
    pct("green_pct", "Green", -200, 200, "Green is -200 to 200 %.");
pub(crate) const MIX_BLUE: FieldSpec = pct("blue_pct", "Blue", -200, 200, "Blue is -200 to 200 %.");

const fn curve(key: &'static str, label: &'static str) -> FieldSpec {
    pct(key, label, 0, 100, "Points are 0 to 100 %.")
}

/// The Input and Output fields of the eight Curves points, in point order.
pub(crate) const CURVES_FIELDS: [FieldSpec; 16] = [
    curve("in_0", "In"),
    curve("out_0", "Out"),
    curve("in_1", "In"),
    curve("out_1", "Out"),
    curve("in_2", "In"),
    curve("out_2", "Out"),
    curve("in_3", "In"),
    curve("out_3", "Out"),
    curve("in_4", "In"),
    curve("out_4", "Out"),
    curve("in_5", "In"),
    curve("out_5", "Out"),
    curve("in_6", "In"),
    curve("out_6", "Out"),
    curve("in_7", "In"),
    curve("out_7", "Out"),
];
