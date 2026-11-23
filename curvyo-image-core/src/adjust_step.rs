//! One adjustment step in pixel units, as it is worked on one plane (ADR 0073 §1, §3).

use crate::adjust_blur::SubPx;
use crate::adjust_clahe::ClaheRun;
use crate::adjust_denoise::DenoiseRun;
use crate::adjust_detail::DetailRun;
use crate::adjust_geometry::{crop_plane, resize_plane};
use crate::adjust_plane::GreyPlane;
use crate::adjust_tone::TableRun;

/// One adjustment step in pixel units, as it runs (ADR 0073 §1): radii in 1/16 pixel of the
/// step's input, greys and percentages as stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelStep {
    /// Keeps the pixel rectangle of left, top, width and height in percent
    /// ([`crate::crop_rect`]).
    Crop {
        /// Left, top, width and height, whole percent.
        pct: [u8; 4],
    },
    /// Reduces the picture to this width in pixels, the height following
    /// ([`crate::resized_size`]).
    Resize {
        /// The width in pixels.
        width_px: u32,
    },
    /// How colour becomes grey: red, green and blue weights in whole percent (-200 to 200).
    ChannelMix {
        /// Red, green and blue.
        weights: [i16; 3],
    },
    /// The bilateral filter: Strength and Detail kept in percent.
    Denoise {
        /// Strength, 0 to 100 %.
        strength_pct: u8,
        /// Detail kept, 0 to 100 %.
        detail_pct: u8,
    },
    /// Black point, White point in percent of the grey range, Gamma in hundredths.
    Levels {
        /// Black point, 0 to 98 %.
        black_pct: u8,
        /// White point, 2 to 100 %.
        white_pct: u8,
        /// Gamma in hundredths, 10 to 400.
        gamma_centi: u16,
    },
    /// The monotone curve through the first `count` of `points` (Input %, Output %).
    Curves {
        /// The points, Input strictly increasing; the ones past `count` unused.
        points: [(u8, u8); 8],
        /// How many points are used.
        count: u8,
    },
    /// CLAHE with tiles of 2 x `radius`.
    LocalContrast {
        /// The radius.
        radius: SubPx,
        /// Amount, 0 to 200 %.
        amount_pct: u16,
    },
    /// The dark lines of a difference of blurs at `sigma` (half the line width) and 1.6 x that.
    Contours {
        /// Half the line width.
        sigma: SubPx,
        /// Sensitivity, 0 to 100 %.
        sensitivity_pct: u8,
    },
    /// The unsharp mask.
    Sharpen {
        /// The blur's sigma.
        radius: SubPx,
        /// Amount, 0 to 500 %.
        amount_pct: u16,
        /// Threshold, 0 to 50 %.
        threshold_pct: u8,
    },
    /// The picture replaced by the stored output of the Upscale model (ADR 0077 §3).
    Upscale {
        /// The SHA-256 of the stored result's PNG.
        result: [u8; 32],
    },
    /// The picture blended with the stored output of the AI denoise model.
    AiDenoise {
        /// The SHA-256 of the stored result's PNG.
        result: [u8; 32],
        /// Strength, 0 to 100 %: the share of the model output.
        strength_pct: u8,
    },
    /// The stored mask of Remove background, mapped and applied last.
    Mask {
        /// The SHA-256 of the stored mask's PNG.
        result: [u8; 32],
        /// Threshold, 0 to 100 %.
        threshold_pct: u8,
        /// Edge softness, 0 to 20 %.
        softness_pct: u8,
    },
}

impl PixelStep {
    /// Whether the step runs on each colour channel while the picture is still in colour.
    pub(crate) const fn per_channel(self) -> bool {
        matches!(
            self,
            Self::Denoise { .. }
                | Self::Crop { .. }
                | Self::Resize { .. }
                | Self::Upscale { .. }
                | Self::AiDenoise { .. }
                | Self::Mask { .. }
        )
    }

    /// The stored result a step reads, `None` for a step that reads none.
    #[must_use]
    pub const fn result(self) -> Option<[u8; 32]> {
        match self {
            Self::Upscale { result }
            | Self::AiDenoise { result, .. }
            | Self::Mask { result, .. } => Some(result),
            _ => None,
        }
    }

    /// The step's run on `plane`, nothing worked yet.
    pub(crate) fn start(self, plane: &GreyPlane) -> StepRun {
        match self {
            Self::Crop { pct } => StepRun::Geometry(Geometry::Crop(pct)),
            Self::Resize { width_px } => StepRun::Geometry(Geometry::Resize(width_px)),
            // The grey conversion is the step; on a grey plane it changes nothing (criterion 6).
            Self::ChannelMix { .. } => StepRun::Table(TableRun::curves(&[])),
            Self::Curves { points, count } => {
                StepRun::Table(TableRun::curves(&points[..usize::from(count.min(8))]))
            }
            Self::Denoise {
                strength_pct,
                detail_pct,
            } => StepRun::Denoise(Box::new(DenoiseRun::new(plane, strength_pct, detail_pct))),
            Self::Levels {
                black_pct,
                white_pct,
                gamma_centi,
            } => StepRun::Table(TableRun::levels(black_pct, white_pct, gamma_centi)),
            Self::LocalContrast { radius, amount_pct } => {
                StepRun::Clahe(ClaheRun::new(plane, radius, amount_pct))
            }
            Self::Contours {
                sigma,
                sensitivity_pct,
            } => StepRun::Detail(DetailRun::contours(plane, sigma, sensitivity_pct)),
            Self::Sharpen {
                radius,
                amount_pct,
                threshold_pct,
            } => StepRun::Detail(DetailRun::sharpen(plane, radius, amount_pct, threshold_pct)),
            // Worked on every plane at once in `AdjustRun::advance`.
            Self::Upscale { .. } | Self::AiDenoise { .. } | Self::Mask { .. } => {
                StepRun::Geometry(Geometry::Resize(plane.width))
            }
        }
    }
}

/// One step being worked on one plane.
#[derive(Debug, Clone)]
pub(crate) enum StepRun {
    Geometry(Geometry),
    Denoise(Box<DenoiseRun>),
    Table(TableRun),
    Clahe(ClaheRun),
    Detail(DetailRun),
}

impl StepRun {
    pub(crate) fn work(&mut self, plane: &mut GreyPlane, work: &mut u64) -> bool {
        match self {
            Self::Geometry(step) => {
                *work = work.saturating_sub(plane.pixels());
                *plane = match *step {
                    Geometry::Crop(pct) => crop_plane(plane, pct),
                    Geometry::Resize(width_px) => resize_plane(plane, width_px),
                };
                true
            }
            Self::Denoise(run) => run.work(plane, work),
            Self::Table(run) => run.work(plane, work),
            Self::Clahe(run) => run.work(plane, work),
            Self::Detail(run) => run.work(plane, work),
        }
    }
}

/// A geometry step, worked on a plane at once (a crop is a copy, a resize one pass of each
/// axis).
#[derive(Debug, Clone, Copy)]
pub(crate) enum Geometry {
    Crop([u8; 4]),
    Resize(u32),
}
