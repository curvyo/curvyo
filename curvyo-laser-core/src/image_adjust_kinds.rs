//! The kinds of stored adjustment steps in pipeline order (ADR 0073 §3; `specs/0135` criteria 3,
//! 5): keys, names, bands, field lists and defaults.

use crate::image_adjust_field_specs::{
    CONTOURS_SENSITIVITY, CONTOURS_WIDTH, CROP_HEIGHT, CROP_LEFT, CROP_TOP, CROP_WIDTH,
    CURVES_FIELDS, DENOISE_DETAIL, DENOISE_STRENGTH, FieldSpec, LEVELS_BLACK, LEVELS_GAMMA,
    LEVELS_WHITE, LOCAL_AMOUNT, LOCAL_RADIUS, MIX_BLUE, MIX_GREEN, MIX_RED, RESIZE_WIDTH,
    SHARPEN_AMOUNT, SHARPEN_RADIUS, SHARPEN_THRESHOLD,
};
use crate::image_adjust_fields::ManualStep;

/// The band of a step (ADR 0073 §3, `ux-notes.md` §3.7): the geometry steps run first in their
/// fixed order, the tone and detail steps in the maker's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Band {
    /// Crop, Resize (and in slice 3 Upscale, AI denoise).
    Geometry,
    /// Channel mix to Sharpen.
    Tone,
}

/// The kind of a stored step, in pipeline order (ADR 0073 §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StepKind {
    /// Keeps a rectangle of the picture.
    Crop,
    /// Fewer pixels, the ratio kept.
    Resize,
    /// How colour becomes grey.
    ChannelMix,
    /// The bilateral filter.
    Denoise,
    /// Black point, White point and Gamma.
    Levels,
    /// A monotone tone curve.
    Curves,
    /// CLAHE.
    LocalContrast,
    /// Dark lines from a difference of blurs.
    Contours,
    /// The unsharp mask.
    Sharpen,
}

/// The largest number of Curves points.
pub const MAX_CURVE_POINTS: usize = 8;

impl StepKind {
    /// Every kind in pipeline order.
    pub const ALL: [Self; 9] = [
        Self::Crop,
        Self::Resize,
        Self::ChannelMix,
        Self::Denoise,
        Self::Levels,
        Self::Curves,
        Self::LocalContrast,
        Self::Contours,
        Self::Sharpen,
    ];

    /// The stored kind.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Crop => "crop",
            Self::Resize => "resize",
            Self::ChannelMix => "channel_mix",
            Self::Denoise => "denoise",
            Self::Levels => "levels",
            Self::Curves => "curves",
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
            Self::Crop => "Crop",
            Self::Resize => "Resize",
            Self::ChannelMix => "Channel mix",
            Self::Denoise => "Denoise",
            Self::Levels => "Levels",
            Self::Curves => "Curves",
            Self::LocalContrast => "Local contrast",
            Self::Contours => "Contours",
            Self::Sharpen => "Sharpen",
        }
    }

    /// The step's band.
    #[must_use]
    pub const fn band(self) -> Band {
        match self {
            Self::Crop | Self::Resize => Band::Geometry,
            _ => Band::Tone,
        }
    }

    /// The fields of the kind in their row order (Curves: In and Out of each of its eight
    /// points; a stored curve holds the points it uses).
    #[must_use]
    pub const fn fields(self) -> &'static [FieldSpec] {
        match self {
            Self::Crop => &[CROP_LEFT, CROP_TOP, CROP_WIDTH, CROP_HEIGHT],
            Self::Resize => &[RESIZE_WIDTH],
            Self::ChannelMix => &[MIX_RED, MIX_GREEN, MIX_BLUE],
            Self::Denoise => &[DENOISE_STRENGTH, DENOISE_DETAIL],
            Self::Levels => &[LEVELS_BLACK, LEVELS_WHITE, LEVELS_GAMMA],
            Self::Curves => &CURVES_FIELDS,
            Self::LocalContrast => &[LOCAL_RADIUS, LOCAL_AMOUNT],
            Self::Contours => &[CONTOURS_WIDTH, CONTOURS_SENSITIVITY],
            Self::Sharpen => &[SHARPEN_RADIUS, SHARPEN_AMOUNT, SHARPEN_THRESHOLD],
        }
    }

    /// A new step of the kind (criterion 5's defaults); the Interval gives Sharpen's radius
    /// (1 x) and Contours' line width (3 x). A new Resize keeps the whole width up to the
    /// field's largest value; the caller sets the picture's width.
    #[must_use]
    pub fn default_step(self, interval_um: u32) -> ManualStep {
        let um = |spec: FieldSpec, value: u32| spec.clamped(i64::from(value));
        match self {
            Self::Crop => ManualStep::Crop {
                pct: [0, 0, 100, 100],
            },
            Self::Resize => ManualStep::Resize {
                width_px: u32::try_from(RESIZE_WIDTH.max).unwrap_or(u32::MAX),
            },
            Self::ChannelMix => ManualStep::ChannelMix {
                weights: LUMINANCE_MIX,
            },
            Self::Denoise => ManualStep::Denoise {
                strength_pct: 30,
                detail_pct: 60,
            },
            Self::Levels => ManualStep::Levels {
                black_pct: 0,
                white_pct: 100,
                gamma_centi: 100,
            },
            Self::Curves => ManualStep::straight_curve(),
            Self::LocalContrast => ManualStep::LocalContrast {
                radius_um: 3_000,
                amount_pct: 50,
            },
            Self::Contours => ManualStep::Contours {
                width_um: u32::try_from(um(CONTOURS_WIDTH, 3 * interval_um)).unwrap_or(u32::MAX),
                sensitivity_pct: 50,
            },
            Self::Sharpen => ManualStep::Sharpen {
                radius_um: u32::try_from(um(SHARPEN_RADIUS, interval_um)).unwrap_or(u32::MAX),
                amount_pct: 100,
                threshold_pct: 0,
            },
        }
    }
}

/// The Channel mix of the `0123` luminance, 21.26, 71.52 and 7.22 % as whole percent
/// (criterion 5).
pub const LUMINANCE_MIX: [i16; 3] = [21, 72, 7];

/// The Channel preset cells (`ux-notes.md` §3.8; Proposal): name and weights.
pub const CHANNEL_PRESETS: [(&str, [i16; 3]); 4] = [
    ("Luminance", LUMINANCE_MIX),
    ("Red filter", [100, 0, 0]),
    ("Green filter", [0, 100, 0]),
    ("Blue filter", [0, 0, 100]),
];
