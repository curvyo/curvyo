//! The stored adjustment steps with their integer fields (ADR 0072 §1, ADR 0073 §2; `specs/0135`
//! criterion 5), read from the stored shape and turned into pixel units.

use curvyo_image_core::{PixelStep, SubPx};

use crate::image_adjust_ai::ResultRef;

pub use crate::image_adjust_field_specs::FieldSpec;
use crate::image_adjust_field_specs::{CROP_HEIGHT, CROP_WIDTH, CURVES_FIELDS, LEVELS_WHITE};
pub use crate::image_adjust_kinds::{
    Band, CHANNEL_PRESETS, LUMINANCE_MIX, MAX_CURVE_POINTS, StepKind,
};

/// One step of a picture's stack with its stored fields (ADR 0072 §1, ADR 0073 §2) and, for an
/// AI step, its stored result (ADR 0077 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ManualStep {
    /// Crop.
    Crop {
        /// Left, top, width and height in whole %; left + width and top + height at most 100.
        pct: [u8; 4],
    },
    /// Resize.
    Resize {
        /// The width in pixels, 16 to 8192; the run never enlarges.
        width_px: u32,
    },
    /// Channel mix.
    ChannelMix {
        /// Red, green and blue, -200 to 200 %.
        weights: [i16; 3],
    },
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
    /// Curves.
    Curves {
        /// The points (Input %, Output %), Input strictly increasing from 0 to 100; those past
        /// `count` are (0, 0).
        points: [(u8, u8); MAX_CURVE_POINTS],
        /// How many points, 2 to 8.
        count: u8,
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
    /// Upscale (criterion 47).
    Upscale {
        /// 2 or 4.
        factor: u8,
        /// The stored result, `None` until Run.
        result: Option<ResultRef>,
    },
    /// AI denoise.
    AiDenoise {
        /// Strength, 0 to 100 %: the share of the model output.
        strength_pct: u8,
        /// The stored result, `None` until Run.
        result: Option<ResultRef>,
    },
    /// Remove background.
    RemoveBackground {
        /// Edge softness, 0 to 20 %.
        softness_pct: u8,
        /// Threshold, 0 to 100 %.
        threshold_pct: u8,
        /// The stored mask, `None` until Run.
        result: Option<ResultRef>,
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
    /// The straight curve: the two end points (0, 0) and (100, 100).
    #[must_use]
    pub const fn straight_curve() -> Self {
        let mut points = [(0, 0); MAX_CURVE_POINTS];
        points[1] = (100, 100);
        Self::Curves { points, count: 2 }
    }

    /// The curve through `points` when they are a stored curve: 2 to 8 points, Input strictly
    /// increasing from 0 to 100, every value at most 100.
    #[must_use]
    pub fn curve(points: &[(u8, u8)]) -> Option<Self> {
        let valid = (2..=MAX_CURVE_POINTS).contains(&points.len())
            && points.first()?.0 == 0
            && points.last()?.0 == 100
            && points.windows(2).all(|w| w[0].0 < w[1].0)
            && points.iter().all(|p| p.1 <= 100);
        valid.then(|| {
            let mut stored = [(0, 0); MAX_CURVE_POINTS];
            stored[..points.len()].copy_from_slice(points);
            Self::Curves {
                points: stored,
                // invariant: at most 8.
                count: u8::try_from(points.len()).unwrap_or(2),
            }
        })
    }

    /// The step's kind.
    #[must_use]
    pub const fn kind(&self) -> StepKind {
        match self {
            Self::Crop { .. } => StepKind::Crop,
            Self::Resize { .. } => StepKind::Resize,
            Self::ChannelMix { .. } => StepKind::ChannelMix,
            Self::Denoise { .. } => StepKind::Denoise,
            Self::Levels { .. } => StepKind::Levels,
            Self::Curves { .. } => StepKind::Curves,
            Self::LocalContrast { .. } => StepKind::LocalContrast,
            Self::Contours { .. } => StepKind::Contours,
            Self::Sharpen { .. } => StepKind::Sharpen,
            Self::Upscale { .. } => StepKind::Upscale,
            Self::AiDenoise { .. } => StepKind::AiDenoise,
            Self::RemoveBackground { .. } => StepKind::RemoveBackground,
        }
    }

    /// The stored fields, in the kind's row order (Curves: In and Out of the points it uses).
    #[must_use]
    pub fn fields(&self) -> Vec<(&'static str, i64)> {
        let values: Vec<i64> = match *self {
            Self::Crop { pct } => pct.map(i64::from).to_vec(),
            Self::Resize { width_px } => vec![width_px.into()],
            Self::ChannelMix { weights } => weights.map(i64::from).to_vec(),
            Self::Denoise {
                strength_pct,
                detail_pct,
            } => vec![strength_pct.into(), detail_pct.into()],
            Self::Levels {
                black_pct,
                white_pct,
                gamma_centi,
            } => vec![black_pct.into(), white_pct.into(), gamma_centi.into()],
            Self::Curves { points, count } => points[..usize::from(count)]
                .iter()
                .flat_map(|&(i, o)| [i64::from(i), i64::from(o)])
                .collect(),
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
            Self::Upscale { factor, .. } => vec![factor.into()],
            Self::AiDenoise { strength_pct, .. } => vec![strength_pct.into()],
            Self::RemoveBackground {
                softness_pct,
                threshold_pct,
                ..
            } => vec![softness_pct.into(), threshold_pct.into()],
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
    /// Levels' White below Black + 2, a crop past the picture's edge, a curve that is not one).
    pub fn read(kind: &str, field: impl Fn(&str) -> Option<i64>) -> Result<Self, StepDamage> {
        let kind = StepKind::from_key(kind).ok_or(StepDamage::UnknownKind)?;
        if kind == StepKind::Curves {
            return read_curve(field);
        }
        let mut values = [0_i64; 4];
        for (spec, slot) in kind.fields().iter().zip(values.iter_mut()) {
            let value = field(spec.key).ok_or(StepDamage::Missing(spec.key))?;
            if !(spec.min..=spec.max).contains(&value) {
                return Err(StepDamage::OutOfRange(spec.key));
            }
            *slot = value;
        }
        Self::of_values(kind, values)
    }

    /// The step of `kind` with its checked field values in row order; the cross-field rules
    /// (Levels' order, a crop inside the picture) are checked here.
    fn of_values(kind: StepKind, values: [i64; 4]) -> Result<Self, StepDamage> {
        let [a, b, c, d] = values;
        Ok(match kind {
            StepKind::Crop => {
                if a + c > 100 {
                    return Err(StepDamage::OutOfRange(CROP_WIDTH.key));
                }
                if b + d > 100 {
                    return Err(StepDamage::OutOfRange(CROP_HEIGHT.key));
                }
                Self::Crop {
                    pct: [a, b, c, d].map(to_u8),
                }
            }
            StepKind::Resize => Self::Resize {
                width_px: to_u32(a),
            },
            StepKind::ChannelMix => Self::ChannelMix {
                weights: [a, b, c].map(to_i16),
            },
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
            StepKind::Upscale => {
                if a != 2 && a != 4 {
                    return Err(StepDamage::OutOfRange("factor"));
                }
                Self::Upscale {
                    factor: to_u8(a),
                    result: None,
                }
            }
            StepKind::AiDenoise => Self::AiDenoise {
                strength_pct: to_u8(a),
                result: None,
            },
            StepKind::RemoveBackground => Self::RemoveBackground {
                softness_pct: to_u8(a),
                threshold_pct: to_u8(b),
                result: None,
            },
            StepKind::Curves => unreachable!("read above"),
        })
    }

    /// The step in pixel units for an input whose pixel is `pitch_mm` of the page: radii and
    /// widths `round(16 · µm / (1000 · pitch))` in 1/16 px (ADR 0073 §2), Contours' sigma half
    /// the line width. Resize's width is in the pixels of the full working image; the caller
    /// scales it for a reduced level. An AI step names its stored result (a step without one
    /// names no held result and changes nothing; the caller leaves it out).
    #[must_use]
    pub fn pixel_step(&self, pitch_mm: f64) -> PixelStep {
        let sub = |um: u32| sub_px(f64::from(um), pitch_mm);
        match *self {
            Self::Crop { pct } => PixelStep::Crop { pct },
            Self::Resize { width_px } => PixelStep::Resize { width_px },
            Self::ChannelMix { weights } => PixelStep::ChannelMix { weights },
            Self::Curves { points, count } => PixelStep::Curves { points, count },
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
            Self::Upscale { result, .. } => PixelStep::Upscale {
                result: result.map_or([0; 32], |r| r.asset),
            },
            Self::AiDenoise {
                strength_pct,
                result,
            } => PixelStep::AiDenoise {
                result: result.map_or([0; 32], |r| r.asset),
                strength_pct,
            },
            Self::RemoveBackground {
                softness_pct,
                threshold_pct,
                result,
            } => PixelStep::Mask {
                result: result.map_or([0; 32], |r| r.asset),
                threshold_pct,
                softness_pct,
            },
        }
    }
}

/// A stored curve: the pairs `in_k`, `out_k` that are present, in point order (one of a pair
/// alone is missing), checked as [`ManualStep::curve`].
fn read_curve(field: impl Fn(&str) -> Option<i64>) -> Result<ManualStep, StepDamage> {
    let mut points = Vec::new();
    for pair in CURVES_FIELDS.as_chunks::<2>().0 {
        match (field(pair[0].key), field(pair[1].key)) {
            (None, None) => {}
            (Some(i), Some(o)) => {
                for (value, spec) in [(i, pair[0]), (o, pair[1])] {
                    if !(spec.min..=spec.max).contains(&value) {
                        return Err(StepDamage::OutOfRange(spec.key));
                    }
                }
                points.push((to_u8(i), to_u8(o)));
            }
            (None, Some(_)) => return Err(StepDamage::Missing(pair[0].key)),
            (Some(_), None) => return Err(StepDamage::Missing(pair[1].key)),
        }
    }
    ManualStep::curve(&points).ok_or(StepDamage::OutOfRange(CURVES_FIELDS[0].key))
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

fn to_i16(v: i64) -> i16 {
    i16::try_from(v).unwrap_or(i16::MAX)
}

fn to_u16(v: i64) -> u16 {
    u16::try_from(v).unwrap_or(u16::MAX)
}

fn to_u32(v: i64) -> u32 {
    u32::try_from(v).unwrap_or(u32::MAX)
}

#[cfg(test)]
#[path = "image_adjust_fields_tests.rs"]
mod tests;
