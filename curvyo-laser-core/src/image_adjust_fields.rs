//! The stored fields of each adjustment step (ADR 0072 §1, ADR 0073 §2; `specs/0135` criterion
//! 5): kinds, integer keys with their units, ranges and defaults.

use curvyo_image_core::{PixelStep, SubPx};

use crate::steps::typed_steps;

/// The kind of a step of the slice-1 stack, in pipeline order (ADR 0073 §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StepKind {
    /// The bilateral filter.
    Denoise,
    /// Black point, White point and Gamma.
    Levels,
    /// CLAHE.
    LocalContrast,
    /// Dark lines from a difference of blurs.
    Contours,
    /// The unsharp mask.
    Sharpen,
}

impl StepKind {
    /// Every kind in pipeline order.
    pub const ALL: [Self; 5] = [
        Self::Denoise,
        Self::Levels,
        Self::LocalContrast,
        Self::Contours,
        Self::Sharpen,
    ];

    /// The stored kind.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Denoise => "denoise",
            Self::Levels => "levels",
            Self::LocalContrast => "local_contrast",
            Self::Contours => "contours",
            Self::Sharpen => "sharpen",
        }
    }

    /// The kind of a stored value, `None` for a kind this build does not know.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.key() == key)
    }

    /// The step's name in the UI.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Denoise => "Denoise",
            Self::Levels => "Levels",
            Self::LocalContrast => "Local contrast",
            Self::Contours => "Contours",
            Self::Sharpen => "Sharpen",
        }
    }

    /// The fields of the kind in their row order.
    #[must_use]
    pub const fn fields(self) -> &'static [FieldSpec] {
        match self {
            Self::Denoise => &[DENOISE_STRENGTH, DENOISE_DETAIL],
            Self::Levels => &[LEVELS_BLACK, LEVELS_WHITE, LEVELS_GAMMA],
            Self::LocalContrast => &[LOCAL_RADIUS, LOCAL_AMOUNT],
            Self::Contours => &[CONTOURS_WIDTH, CONTOURS_SENSITIVITY],
            Self::Sharpen => &[SHARPEN_RADIUS, SHARPEN_AMOUNT, SHARPEN_THRESHOLD],
        }
    }

    /// A new step of the kind (criterion 5's defaults); the Interval gives Sharpen's radius
    /// (1 x) and Contours' line width (3 x).
    #[must_use]
    pub fn default_step(self, interval_um: u32) -> ManualStep {
        let um = |spec: FieldSpec, value: u32| spec.clamped(i64::from(value));
        match self {
            Self::Denoise => ManualStep::Denoise {
                strength_pct: 30,
                detail_pct: 60,
            },
            Self::Levels => ManualStep::Levels {
                black_pct: 0,
                white_pct: 100,
                gamma_centi: 100,
            },
            Self::LocalContrast => ManualStep::LocalContrast {
                radius_um: 3_000,
                amount_pct: 50,
            },
            Self::Contours => ManualStep::Contours {
                width_um: to_u32(um(CONTOURS_WIDTH, 3 * interval_um)),
                sensitivity_pct: 50,
            },
            Self::Sharpen => ManualStep::Sharpen {
                radius_um: to_u32(um(SHARPEN_RADIUS, interval_um)),
                amount_pct: 100,
                threshold_pct: 0,
            },
        }
    }
}

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

const fn pct(
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

const DENOISE_STRENGTH: FieldSpec = pct(
    "strength_pct",
    "Strength",
    0,
    100,
    "Strength is 0 to 100 %.",
);
const DENOISE_DETAIL: FieldSpec = pct(
    "detail_pct",
    "Detail kept",
    0,
    100,
    "Detail kept is 0 to 100 %.",
);
const LEVELS_BLACK: FieldSpec = pct("black_pct", "Black", 0, 98, "Black point is 0 to 98 %.");
const LEVELS_WHITE: FieldSpec = pct("white_pct", "White", 2, 100, "White point is 2 to 100 %.");
const LEVELS_GAMMA: FieldSpec = FieldSpec {
    key: "gamma_centi",
    label: "Gamma",
    unit: "",
    step: 1,
    per_unit: 100,
    min: 10,
    max: 400,
    range_text: "Gamma is 0.10 to 4.00.",
};
const LOCAL_RADIUS: FieldSpec = FieldSpec {
    key: "radius_um",
    label: "Radius",
    unit: "mm",
    step: 100,
    per_unit: 10,
    min: 200,
    max: 20_000,
    range_text: "Radius is 0.2 to 20.0 mm.",
};
const LOCAL_AMOUNT: FieldSpec = pct("amount_pct", "Amount", 0, 200, "Amount is 0 to 200 %.");
const CONTOURS_WIDTH: FieldSpec = FieldSpec {
    key: "width_um",
    label: "Line width",
    unit: "mm",
    step: 10,
    per_unit: 100,
    min: 50,
    max: 2_000,
    range_text: "Line width is 0.05 to 2.00 mm.",
};
const CONTOURS_SENSITIVITY: FieldSpec = pct(
    "sensitivity_pct",
    "Sensitivity",
    0,
    100,
    "Sensitivity is 0 to 100 %.",
);
const SHARPEN_RADIUS: FieldSpec = FieldSpec {
    key: "radius_um",
    label: "Radius",
    unit: "mm",
    step: 10,
    per_unit: 100,
    min: 50,
    max: 5_000,
    range_text: "Radius is 0.05 to 5.00 mm.",
};
const SHARPEN_AMOUNT: FieldSpec = pct("amount_pct", "Amount", 0, 500, "Amount is 0 to 500 %.");
const SHARPEN_THRESHOLD: FieldSpec = pct(
    "threshold_pct",
    "Threshold",
    0,
    50,
    "Threshold is 0 to 50 %.",
);

/// One step of a picture's stack with its stored fields (ADR 0072 §1, ADR 0073 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ManualStep {
    /// Denoise.
    Denoise {
        /// Strength, 0 to 100 %.
        strength_pct: u8,
        /// Detail kept, 0 to 100 %.
        detail_pct: u8,
    },
    /// Levels.
    Levels {
        /// Black point, 0 to 98 %.
        black_pct: u8,
        /// White point, 2 to 100 %, at least Black + 2.
        white_pct: u8,
        /// Gamma in hundredths, 10 to 400.
        gamma_centi: u16,
    },
    /// Local contrast.
    LocalContrast {
        /// Radius in µm of the page, 200 to 20,000.
        radius_um: u32,
        /// Amount, 0 to 200 %.
        amount_pct: u16,
    },
    /// Contours.
    Contours {
        /// Line width in µm of the page, 50 to 2,000.
        width_um: u32,
        /// Sensitivity, 0 to 100 %.
        sensitivity_pct: u8,
    },
    /// Sharpen.
    Sharpen {
        /// Radius in µm of the page, 50 to 5,000.
        radius_um: u32,
        /// Amount, 0 to 500 %.
        amount_pct: u16,
        /// Threshold, 0 to 50 %.
        threshold_pct: u8,
    },
}

/// Why a stored step does not run (ADR 0072 §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepDamage {
    /// A kind this build does not know.
    UnknownKind,
    /// A field is missing or not an integer.
    Missing(&'static str),
    /// A field is out of its range.
    OutOfRange(&'static str),
}

impl ManualStep {
    /// The step's kind.
    #[must_use]
    pub const fn kind(&self) -> StepKind {
        match self {
            Self::Denoise { .. } => StepKind::Denoise,
            Self::Levels { .. } => StepKind::Levels,
            Self::LocalContrast { .. } => StepKind::LocalContrast,
            Self::Contours { .. } => StepKind::Contours,
            Self::Sharpen { .. } => StepKind::Sharpen,
        }
    }

    /// The stored fields, in the kind's row order.
    #[must_use]
    pub fn fields(&self) -> Vec<(&'static str, i64)> {
        let values: Vec<i64> = match *self {
            Self::Denoise {
                strength_pct,
                detail_pct,
            } => vec![strength_pct.into(), detail_pct.into()],
            Self::Levels {
                black_pct,
                white_pct,
                gamma_centi,
            } => vec![black_pct.into(), white_pct.into(), gamma_centi.into()],
            Self::LocalContrast {
                radius_um,
                amount_pct,
            } => vec![radius_um.into(), amount_pct.into()],
            Self::Contours {
                width_um,
                sensitivity_pct,
            } => vec![width_um.into(), sensitivity_pct.into()],
            Self::Sharpen {
                radius_um,
                amount_pct,
                threshold_pct,
            } => vec![radius_um.into(), amount_pct.into(), threshold_pct.into()],
        };
        self.kind()
            .fields()
            .iter()
            .map(|spec| spec.key)
            .zip(values)
            .collect()
    }

    /// The step of stored `kind` whose field `key` reads `field(key)`.
    ///
    /// # Errors
    /// [`StepDamage`]: an unknown kind, a missing field, or a field out of its range (also
    /// Levels' White below Black + 2).
    pub fn read(kind: &str, field: impl Fn(&str) -> Option<i64>) -> Result<Self, StepDamage> {
        let kind = StepKind::from_key(kind).ok_or(StepDamage::UnknownKind)?;
        let mut values = [0_i64; 3];
        for (spec, slot) in kind.fields().iter().zip(values.iter_mut()) {
            let value = field(spec.key).ok_or(StepDamage::Missing(spec.key))?;
            if !(spec.min..=spec.max).contains(&value) {
                return Err(StepDamage::OutOfRange(spec.key));
            }
            *slot = value;
        }
        let [a, b, c] = values;
        Ok(match kind {
            StepKind::Denoise => Self::Denoise {
                strength_pct: to_u8(a),
                detail_pct: to_u8(b),
            },
            StepKind::Levels => {
                if b < a + 2 {
                    return Err(StepDamage::OutOfRange(LEVELS_WHITE.key));
                }
                Self::Levels {
                    black_pct: to_u8(a),
                    white_pct: to_u8(b),
                    gamma_centi: to_u16(c),
                }
            }
            StepKind::LocalContrast => Self::LocalContrast {
                radius_um: to_u32(a),
                amount_pct: to_u16(b),
            },
            StepKind::Contours => Self::Contours {
                width_um: to_u32(a),
                sensitivity_pct: to_u8(b),
            },
            StepKind::Sharpen => Self::Sharpen {
                radius_um: to_u32(a),
                amount_pct: to_u16(b),
                threshold_pct: to_u8(c),
            },
        })
    }

    /// The step in pixel units for an input whose pixel is `pitch_mm` of the page: radii and
    /// widths `round(16 · µm / (1000 · pitch))` in 1/16 px (ADR 0073 §2), Contours' sigma half
    /// the line width.
    #[must_use]
    pub fn pixel_step(&self, pitch_mm: f64) -> PixelStep {
        let sub = |um: u32| sub_px(f64::from(um), pitch_mm);
        match *self {
            Self::Denoise {
                strength_pct,
                detail_pct,
            } => PixelStep::Denoise {
                strength_pct,
                detail_pct,
            },
            Self::Levels {
                black_pct,
                white_pct,
                gamma_centi,
            } => PixelStep::Levels {
                black_pct,
                white_pct,
                gamma_centi,
            },
            Self::LocalContrast {
                radius_um,
                amount_pct,
            } => PixelStep::LocalContrast {
                radius: sub(radius_um),
                amount_pct,
            },
            Self::Contours {
                width_um,
                sensitivity_pct,
            } => PixelStep::Contours {
                sigma: sub_px(f64::from(width_um) / 2.0, pitch_mm),
                sensitivity_pct,
            },
            Self::Sharpen {
                radius_um,
                amount_pct,
                threshold_pct,
            } => PixelStep::Sharpen {
                radius: sub(radius_um),
                amount_pct,
                threshold_pct,
            },
        }
    }
}

/// `um` of the page in 1/16 pixel of pitch `pitch_mm`, rounded, saturating; 0 for a pitch that
/// is not positive.
fn sub_px(um: f64, pitch_mm: f64) -> SubPx {
    if !(pitch_mm.is_finite() && pitch_mm > 0.0) {
        return SubPx(0);
    }
    let sixteenths = (16.0 * um / (1000.0 * pitch_mm)).round();
    // invariant: `as` saturates; a radius of more than 2^32 sixteenths is far over any picture.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let sixteenths = sixteenths.clamp(0.0, f64::from(u32::MAX)) as u32;
    SubPx(sixteenths)
}

fn to_u8(v: i64) -> u8 {
    u8::try_from(v).unwrap_or(u8::MAX)
}

fn to_u16(v: i64) -> u16 {
    u16::try_from(v).unwrap_or(u16::MAX)
}

fn to_u32(v: i64) -> u32 {
    u32::try_from(v).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_reads_back_its_default_fields() {
        for kind in StepKind::ALL {
            let step = kind.default_step(100);
            let fields = step.fields();
            let read = ManualStep::read(kind.key(), |key| {
                fields.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
            });
            assert_eq!(read, Ok(step), "{kind:?}");
            assert_eq!(StepKind::from_key(kind.key()), Some(kind));
        }
    }

    #[test]
    fn damage_is_named() {
        assert_eq!(
            ManualStep::read("vignette", |_| Some(1)),
            Err(StepDamage::UnknownKind)
        );
        assert_eq!(
            ManualStep::read("levels", |k| (k != "gamma_centi").then_some(5)),
            Err(StepDamage::Missing("gamma_centi"))
        );
        assert_eq!(
            ManualStep::read("sharpen", |k| Some(if k == "amount_pct" {
                501
            } else {
                100
            })),
            Err(StepDamage::OutOfRange("amount_pct")),
        );
        let levels = |b, w| {
            ManualStep::read("levels", move |k| {
                Some(match k {
                    "black_pct" => b,
                    "white_pct" => w,
                    _ => 100,
                })
            })
        };
        assert_eq!(levels(50, 51), Err(StepDamage::OutOfRange("white_pct")));
        assert!(levels(50, 52).is_ok());
    }

    #[test]
    fn typed_values_round_to_the_step_and_refuse_out_of_range() {
        assert_eq!(SHARPEN_RADIUS.typed(0.104), Ok(100));
        assert_eq!(LOCAL_RADIUS.typed(3.04), Ok(3_000));
        assert_eq!(LOCAL_RADIUS.typed(20.1), Err("Radius is 0.2 to 20.0 mm."));
        assert_eq!(LEVELS_GAMMA.typed(1.234), Ok(123));
        assert_eq!(LEVELS_GAMMA.typed(f64::NAN), Err("Gamma is 0.10 to 4.00."));
        assert!((LOCAL_RADIUS.shown(3_000) - 3.0).abs() < 1e-9);
    }

    #[test]
    fn radii_become_sixteenths_of_the_pixel_pitch() {
        let s = ManualStep::Sharpen {
            radius_um: 100,
            amount_pct: 100,
            threshold_pct: 0,
        };
        assert_eq!(
            s.pixel_step(0.05),
            PixelStep::Sharpen {
                radius: SubPx(32),
                amount_pct: 100,
                threshold_pct: 0
            }
        );
        let c = ManualStep::Contours {
            width_um: 300,
            sensitivity_pct: 50,
        };
        assert_eq!(
            c.pixel_step(0.1),
            PixelStep::Contours {
                sigma: SubPx(24),
                sensitivity_pct: 50
            }
        );
    }
}
