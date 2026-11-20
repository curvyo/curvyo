//! The Image settings of an Image operation (ADR 0069 §7, §13, ADR 0072 §6; `specs/0123` criteria 9
//! to 13, `specs/0135` criteria 42 to 44):
//! their ranges, defaults, typed edits and stored `op_options` keys.

use serde_json::{Map, Value, json};

use crate::operation_options::{ENTER_NUMBER, OptionsDamage};
use crate::steps::{exact_steps, typed_steps};

/// How the grey values of a raster item become dots (`specs/0123` criterion 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Dither {
    /// A dot burns when it is darker than the threshold.
    Threshold,
    /// The 8 x 8 Bayer pattern.
    Ordered,
    /// Error diffusion with the Floyd-Steinberg weights (the default, Proposal).
    #[default]
    FloydSteinberg,
    /// Error diffusion with the Jarvis, Judice and Ninke weights.
    Jarvis,
    /// Error diffusion with the Atkinson weights (a quarter of the error dropped).
    Atkinson,
    /// Error diffusion with the Stucki weights.
    Stucki,
    /// The dot's power follows its grey value (Frage 229).
    Grey,
    /// Round dots of changing size on a rotated cell grid (`specs/0135` criterion 42).
    Halftone,
    /// Lines of changing thickness on a rotated cell grid (`specs/0135` criterion 43).
    LineScreen,
}

impl Dither {
    /// The nine modes in the order of the pick (criterion 11; `specs/0135` criterion 44).
    pub const ALL: [Self; 9] = [
        Self::Threshold,
        Self::Ordered,
        Self::FloydSteinberg,
        Self::Jarvis,
        Self::Atkinson,
        Self::Stucki,
        Self::Grey,
        Self::Halftone,
        Self::LineScreen,
    ];

    /// The stored value.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Threshold => "threshold",
            Self::Ordered => "ordered",
            Self::FloydSteinberg => "floyd_steinberg",
            Self::Jarvis => "jarvis",
            Self::Atkinson => "atkinson",
            Self::Stucki => "stucki",
            Self::Grey => "grey",
            Self::Halftone => "halftone",
            Self::LineScreen => "line_screen",
        }
    }

    /// The mode of a stored value, `None` for a value this build does not know.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.key() == key)
    }

    /// Whether the mode burns every dot at one power (all but Grey).
    #[must_use]
    pub const fn is_binary(self) -> bool {
        !matches!(self, Self::Grey)
    }

    /// Whether the Image test grid offers the mode: the binary modes but the screens, whose
    /// cells are too large for a grid cell (`specs/0135` UX notes §10, P23).
    #[must_use]
    pub const fn in_test_grid(self) -> bool {
        self.is_binary() && !self.is_screen()
    }

    /// Whether the mode is a screen with a cell size and an angle (ADR 0074).
    #[must_use]
    pub const fn is_screen(self) -> bool {
        matches!(self, Self::Halftone | Self::LineScreen)
    }
}

/// The Image settings of one Engrave operation; operation-only, never a material's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageSettings {
    /// The dither mode.
    pub dither: Dither,
    /// Threshold mode's threshold, 0 to 100 %.
    pub threshold_pct: u8,
    /// Brightness, -100 to 100 %.
    pub brightness_pct: i8,
    /// Contrast, -100 to 100 %.
    pub contrast_pct: i8,
    /// Gamma in hundredths, 10 to 400 (0.10 to 4.00).
    pub gamma_hundredths: u16,
    /// Whether grey becomes 255 minus grey first.
    pub invert: bool,
    /// Grey mode's lowest power in tenths of a percent, 0 to 1000.
    pub lowest_power_tenths: u16,
    /// The screens' Cell size or Line pitch in hundredths of a mm, 20 to 500 (ADR 0072 §6).
    pub screen_size_hundredths: u16,
    /// The screens' Angle in whole degrees, 0 to 89.
    pub screen_angle_deg: u8,
}

impl ImageSettings {
    /// The largest threshold in percent.
    pub const MAX_THRESHOLD: u8 = 100;
    /// The range of Brightness and Contrast in percent.
    pub const ADJUST_RANGE: std::ops::RangeInclusive<i8> = -100..=100;
    /// The range of Gamma in hundredths.
    pub const GAMMA_RANGE: std::ops::RangeInclusive<u16> = 10..=400;
    /// The largest Lowest power in tenths of a percent.
    pub const MAX_LOWEST_TENTHS: u16 = 1000;
    /// The range of the screens' size in hundredths of a mm.
    pub const SCREEN_SIZE_RANGE: std::ops::RangeInclusive<u16> = 20..=500;
    /// The largest screen Angle in degrees.
    pub const MAX_SCREEN_ANGLE: u8 = 89;

    /// The settings of a new operation (criteria 9 to 11).
    pub const DEFAULT: Self = Self {
        dither: Dither::FloydSteinberg,
        threshold_pct: 50,
        brightness_pct: 0,
        contrast_pct: 0,
        gamma_hundredths: 100,
        invert: false,
        lowest_power_tenths: 0,
        screen_size_hundredths: 80,
        screen_angle_deg: 45,
    };
}

impl Default for ImageSettings {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A number field of the Image settings group (criterion 11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageField {
    /// Threshold, 0 to 100 %, whole.
    Threshold,
    /// Brightness, -100 to 100 %, whole.
    Brightness,
    /// Contrast, -100 to 100 %, whole.
    Contrast,
    /// Gamma, 0.10 to 4.00, two decimals.
    Gamma,
    /// Lowest power, 0 to 100 %, one decimal.
    LowestPower,
    /// Cell size or Line pitch, 0.20 to 5.00 mm, two decimals.
    ScreenSize,
    /// The screens' Angle, 0 to 89 degrees, whole.
    ScreenAngle,
}

impl ImageField {
    /// The chip text of a typed value out of range (UX notes 2.4).
    #[must_use]
    pub const fn range_text(self) -> &'static str {
        match self {
            Self::Threshold => "Threshold is 0 to 100 %.",
            Self::Brightness => "Brightness is -100 to 100 %.",
            Self::Contrast => "Contrast is -100 to 100 %.",
            Self::Gamma => "Gamma is 0.10 to 4.00.",
            Self::LowestPower => "Lowest power is 0 to 100 %.",
            Self::ScreenSize => "Cell size is 0.20 to 5.00 mm.",
            Self::ScreenAngle => "Angle is 0 to 89°.",
        }
    }

    /// Steps per unit and the range in steps.
    const fn steps(self) -> (f64, f64, f64) {
        match self {
            Self::Threshold => (1.0, 0.0, 100.0),
            Self::Brightness | Self::Contrast => (1.0, -100.0, 100.0),
            Self::Gamma => (100.0, 10.0, 400.0),
            Self::LowestPower => (10.0, 0.0, 1000.0),
            Self::ScreenSize => (100.0, 20.0, 500.0),
            Self::ScreenAngle => (1.0, 0.0, 89.0),
        }
    }

    /// The edit of `value` typed into this field, rounded to the field's step; a value out of
    /// range is refused before rounding (criterion 12).
    ///
    /// # Errors
    /// The chip text: [`ENTER_NUMBER`] for a value that is no number, else the range text.
    pub fn typed(self, value: f64) -> Result<ImageEdit, &'static str> {
        if !value.is_finite() {
            return Err(ENTER_NUMBER);
        }
        let (per_unit, min, max) = self.steps();
        let steps = typed_steps(value, per_unit, min, max).ok_or(self.range_text())?;
        let out_of_range = |_| self.range_text();
        Ok(match self {
            Self::Threshold => ImageEdit::Threshold(u8::try_from(steps).map_err(out_of_range)?),
            Self::Brightness => ImageEdit::Brightness(i8::try_from(steps).map_err(out_of_range)?),
            Self::Contrast => ImageEdit::Contrast(i8::try_from(steps).map_err(out_of_range)?),
            Self::Gamma => ImageEdit::Gamma(u16::try_from(steps).map_err(out_of_range)?),
            Self::LowestPower => {
                ImageEdit::LowestPower(u16::try_from(steps).map_err(out_of_range)?)
            }
            Self::ScreenSize => ImageEdit::ScreenSize(u16::try_from(steps).map_err(out_of_range)?),
            Self::ScreenAngle => ImageEdit::ScreenAngle(u8::try_from(steps).map_err(out_of_range)?),
        })
    }
}

/// One edit of the Image settings group: one step "Edit settings" (criterion 12). The values
/// are those of [`ImageSettings`]; build number edits with [`ImageField::typed`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageEdit {
    /// The dither mode.
    Dither(Dither),
    /// Threshold in percent.
    Threshold(u8),
    /// Brightness in percent.
    Brightness(i8),
    /// Contrast in percent.
    Contrast(i8),
    /// Gamma in hundredths.
    Gamma(u16),
    /// Invert on or off.
    Invert(bool),
    /// Lowest power in tenths of a percent.
    LowestPower(u16),
    /// The screens' size in hundredths of a mm.
    ScreenSize(u16),
    /// The screens' Angle in degrees.
    ScreenAngle(u8),
}

impl ImageSettings {
    /// The settings with `edit` applied.
    #[must_use]
    pub const fn with(self, edit: ImageEdit) -> Self {
        let mut next = self;
        match edit {
            ImageEdit::Dither(dither) => next.dither = dither,
            ImageEdit::Threshold(pct) => next.threshold_pct = pct,
            ImageEdit::Brightness(pct) => next.brightness_pct = pct,
            ImageEdit::Contrast(pct) => next.contrast_pct = pct,
            ImageEdit::Gamma(hundredths) => next.gamma_hundredths = hundredths,
            ImageEdit::Invert(on) => next.invert = on,
            ImageEdit::LowestPower(tenths) => next.lowest_power_tenths = tenths,
            ImageEdit::ScreenSize(hundredths) => next.screen_size_hundredths = hundredths,
            ImageEdit::ScreenAngle(degrees) => next.screen_angle_deg = degrees,
        }
        next
    }
}

const KEY_DITHER: &str = "dither";
const KEY_THRESHOLD: &str = "threshold_pct";
const KEY_BRIGHTNESS: &str = "brightness_pct";
const KEY_CONTRAST: &str = "contrast_pct";
const KEY_GAMMA: &str = "gamma";
const KEY_INVERT: &str = "invert";
const KEY_LOWEST_POWER: &str = "lowest_power_pct";
const KEY_SCREEN_SIZE: &str = "screen_size_mm";
const KEY_SCREEN_ANGLE: &str = "screen_angle_deg";

/// The nine stored keys, allowed on the Image kind only (ADR 0069 §13, ADR 0072 §6).
pub(crate) const IMAGE_KEYS: [&str; 9] = [
    KEY_DITHER,
    KEY_THRESHOLD,
    KEY_BRIGHTNESS,
    KEY_CONTRAST,
    KEY_GAMMA,
    KEY_INVERT,
    KEY_LOWEST_POWER,
    KEY_SCREEN_SIZE,
    KEY_SCREEN_ANGLE,
];

/// Reads one stored image key into `settings`: `Ok(false)` when `key` is not an image key.
///
/// # Errors
/// The damage of a value that does not read.
pub(crate) fn read_image_key(
    settings: &mut ImageSettings,
    key: &str,
    value: &Value,
) -> Result<bool, OptionsDamage> {
    let adjust = |value: &Value| {
        let n = value.as_i64().ok_or(OptionsDamage::WrongType)?;
        i8::try_from(n)
            .ok()
            .filter(|n| ImageSettings::ADJUST_RANGE.contains(n))
            .ok_or(OptionsDamage::BadValue)
    };
    match key {
        KEY_DITHER => {
            let text = value.as_str().ok_or(OptionsDamage::WrongType)?;
            settings.dither = Dither::from_key(text).ok_or(OptionsDamage::BadValue)?;
        }
        KEY_THRESHOLD => {
            let n = value.as_u64().ok_or(OptionsDamage::WrongType)?;
            settings.threshold_pct = u8::try_from(n)
                .ok()
                .filter(|n| *n <= ImageSettings::MAX_THRESHOLD)
                .ok_or(OptionsDamage::BadValue)?;
        }
        KEY_BRIGHTNESS => settings.brightness_pct = adjust(value)?,
        KEY_CONTRAST => settings.contrast_pct = adjust(value)?,
        KEY_GAMMA => {
            let gamma = value.as_f64().ok_or(OptionsDamage::WrongType)?;
            settings.gamma_hundredths = exact_steps(gamma, 100.0)
                .and_then(|n| u16::try_from(n).ok())
                .filter(|n| ImageSettings::GAMMA_RANGE.contains(n))
                .ok_or(OptionsDamage::BadValue)?;
        }
        KEY_INVERT => settings.invert = value.as_bool().ok_or(OptionsDamage::WrongType)?,
        KEY_LOWEST_POWER => {
            let percent = value.as_f64().ok_or(OptionsDamage::WrongType)?;
            settings.lowest_power_tenths = exact_steps(percent, 10.0)
                .and_then(|n| u16::try_from(n).ok())
                .filter(|n| *n <= ImageSettings::MAX_LOWEST_TENTHS)
                .ok_or(OptionsDamage::BadValue)?;
        }
        KEY_SCREEN_SIZE => {
            let mm = value.as_f64().ok_or(OptionsDamage::WrongType)?;
            settings.screen_size_hundredths = exact_steps(mm, 100.0)
                .and_then(|n| u16::try_from(n).ok())
                .filter(|n| ImageSettings::SCREEN_SIZE_RANGE.contains(n))
                .ok_or(OptionsDamage::BadValue)?;
        }
        KEY_SCREEN_ANGLE => {
            let n = value.as_u64().ok_or(OptionsDamage::WrongType)?;
            settings.screen_angle_deg = u8::try_from(n)
                .ok()
                .filter(|n| *n <= ImageSettings::MAX_SCREEN_ANGLE)
                .ok_or(OptionsDamage::BadValue)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// Writes `settings` into `map`: the seven keys of `0123`, or none while every value is its
/// default (a missing key reads as its default, so an Engrave that was never given an Image
/// setting keeps the map of `0111`); the two screen keys only when they differ from their
/// defaults (ADR 0072 §6: absent = 0.80 mm and 45 degrees), so a `0123` map stays as it was.
pub(crate) fn write_image_keys(settings: &ImageSettings, map: &mut Map<String, Value>) {
    if *settings == ImageSettings::DEFAULT {
        return;
    }
    map.insert(KEY_DITHER.to_owned(), json!(settings.dither.key()));
    map.insert(KEY_THRESHOLD.to_owned(), json!(settings.threshold_pct));
    map.insert(KEY_BRIGHTNESS.to_owned(), json!(settings.brightness_pct));
    map.insert(KEY_CONTRAST.to_owned(), json!(settings.contrast_pct));
    map.insert(
        KEY_GAMMA.to_owned(),
        json!(f64::from(settings.gamma_hundredths) / 100.0),
    );
    map.insert(KEY_INVERT.to_owned(), json!(settings.invert));
    map.insert(
        KEY_LOWEST_POWER.to_owned(),
        json!(f64::from(settings.lowest_power_tenths) / 10.0),
    );
    if settings.screen_size_hundredths != ImageSettings::DEFAULT.screen_size_hundredths {
        map.insert(
            KEY_SCREEN_SIZE.to_owned(),
            json!(f64::from(settings.screen_size_hundredths) / 100.0),
        );
    }
    if settings.screen_angle_deg != ImageSettings::DEFAULT.screen_angle_deg {
        map.insert(
            KEY_SCREEN_ANGLE.to_owned(),
            json!(settings.screen_angle_deg),
        );
    }
}

#[cfg(test)]
#[path = "operation_image_options_tests.rs"]
mod tests;
