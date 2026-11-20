//! The Image settings group of an unfolded Engrave row as the host reads it (`specs/0123`
//! criteria 11 to 13; UX notes 2): the Dither pick, the mode's own field in the row under it, then
//! Brightness, Contrast, Gamma and Invert, each field with its unit and range text.

use curvyo_laser_core::settings::{Dither, ImageField, ImageSettings};
use serde::Serialize;

/// The words of a Dither mode (criterion 11).
pub(in crate::session) const fn dither_label(mode: Dither) -> &'static str {
    match mode {
        Dither::Threshold => "Threshold",
        Dither::Ordered => "Ordered",
        Dither::FloydSteinberg => "Floyd-Steinberg",
        Dither::Jarvis => "Jarvis",
        Dither::Atkinson => "Atkinson",
        Dither::Stucki => "Stucki",
        Dither::Grey => "Grey",
        Dither::Halftone => "Halftone dots",
        Dither::LineScreen => "Line screen",
    }
}

/// One mode of a Dither pick or menu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DitherOptionView {
    /// Its stored spelling, sent back by the pick.
    pub value: &'static str,
    /// Its words.
    pub label: &'static str,
}

/// The modes of `modes` in their order, as options.
pub(in crate::session) fn dither_options(
    modes: impl IntoIterator<Item = Dither>,
) -> Vec<DitherOptionView> {
    modes
        .into_iter()
        .map(|mode| DitherOptionView {
            value: mode.key(),
            label: dither_label(mode),
        })
        .collect()
}

/// One number field of the group.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImageFieldView {
    /// The name `edit_operation_option` takes (`threshold`, `lowest_power`, `brightness`,
    /// `contrast`, `gamma`).
    pub field: &'static str,
    /// The label inside the field.
    pub label: &'static str,
    /// The value.
    pub value: f64,
    /// The unit inside the field ("%"; empty for Gamma).
    pub unit: &'static str,
    /// The chip text of a value out of range ("Gamma is 0.10 to 4.00.").
    pub range: &'static str,
}

/// A Dither strip with the mode's own fields: the picture's (`specs/0135` UX notes 3.10) or an
/// Image operation's for its gradient fills (UX notes 10).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DitherView {
    /// The picked mode's stored spelling.
    pub dither: &'static str,
    /// The nine modes in the strip's order.
    pub modes: Vec<DitherOptionView>,
    /// Threshold's field with Threshold, `None` for the other modes (Lowest power is the
    /// operation's, Frage 284 B).
    pub mode_field: Option<ImageFieldView>,
    /// The screen's row of Halftone dots (Cell size, Angle) and Line screen (Line pitch, Angle);
    /// empty for the other modes (`specs/0135` criteria 42 to 44).
    pub screen_fields: Vec<ImageFieldView>,
}

/// The Image settings group of an Image operation (UX notes 2.2; `specs/0135` UX notes 10).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImageSettingsView {
    /// Brightness, Contrast, Gamma.
    pub fields: Vec<ImageFieldView>,
    /// The Invert toggle.
    pub invert: bool,
    /// Lowest power, only while an item of the operation burns Grey (Frage 284 B).
    pub lowest_power: Option<ImageFieldView>,
    /// The operation's own Dither, "Dither for gradient fills", only while it holds one
    /// (ADR 0072 amendment A4); the stored keys stay while it is `None`.
    pub dither: Option<DitherView>,
}

/// What an Image operation's items need of its group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(in crate::session) struct RasterFacts {
    /// It holds a gradient-filled leaf (the operation's Dither applies to it).
    pub gradient: bool,
    /// One of its items burns Grey.
    pub grey: bool,
}

fn field_view(field: ImageField, image: &ImageSettings) -> ImageFieldView {
    let (name, label, value, unit) = match field {
        ImageField::Threshold => (
            "threshold",
            "Threshold",
            f64::from(image.threshold_pct),
            "%",
        ),
        ImageField::LowestPower => (
            "lowest_power",
            "Lowest power",
            f64::from(image.lowest_power_tenths) / 10.0,
            "%",
        ),
        ImageField::Brightness => (
            "brightness",
            "Brightness",
            f64::from(image.brightness_pct),
            "%",
        ),
        ImageField::Contrast => ("contrast", "Contrast", f64::from(image.contrast_pct), "%"),
        ImageField::ScreenSize => (
            "screen_size",
            "Cell size",
            f64::from(image.screen_size_hundredths) / 100.0,
            "mm",
        ),
        ImageField::ScreenAngle => (
            "screen_angle",
            "Angle",
            f64::from(image.screen_angle_deg),
            "°",
        ),
        ImageField::Gamma => (
            "gamma",
            "Gamma",
            f64::from(image.gamma_hundredths) / 100.0,
            "",
        ),
    };
    ImageFieldView {
        field: name,
        label,
        value,
        unit,
        range: field.range_text(),
    }
}

/// The Cell size or Line pitch and the Angle of a screen mode (criteria 42, 43); none for the
/// other modes.
fn screen_fields(image: &ImageSettings) -> Vec<ImageFieldView> {
    let pitch = match image.dither {
        Dither::Halftone => false,
        Dither::LineScreen => true,
        _ => return Vec::new(),
    };
    let mut size = field_view(ImageField::ScreenSize, image);
    if pitch {
        size.label = "Line pitch";
        size.range = "Line pitch is 0.20 to 5.00 mm.";
    }
    vec![size, field_view(ImageField::ScreenAngle, image)]
}

/// The Dither strip of `image`'s dither (a picture's dither goes through
/// [`ImageSettings::with_dither`]).
pub(in crate::session) fn dither_view(image: &ImageSettings) -> DitherView {
    DitherView {
        dither: image.dither.key(),
        modes: dither_options(Dither::ALL),
        mode_field: (image.dither == Dither::Threshold)
            .then(|| field_view(ImageField::Threshold, image)),
        screen_fields: screen_fields(image),
    }
}

/// The group of the settings `image` of an operation whose items are `facts`.
pub(super) fn image_settings_view(image: &ImageSettings, facts: RasterFacts) -> ImageSettingsView {
    ImageSettingsView {
        fields: [
            ImageField::Brightness,
            ImageField::Contrast,
            ImageField::Gamma,
        ]
        .into_iter()
        .map(|field| field_view(field, image))
        .collect(),
        invert: image.invert,
        lowest_power: facts
            .grey
            .then(|| field_view(ImageField::LowestPower, image)),
        dither: facts.gradient.then(|| dither_view(image)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Superseded by the Dither on the picture (2026-11-20): Threshold is the dither's field,
    /// Lowest power the operation's own field while an item burns Grey (Frage 284 B), and the
    /// operation's Dither shows only with a gradient fill.
    #[test]
    fn the_dither_view_has_threshold_and_the_screens_and_lowest_power_is_the_operations() {
        let view = |dither| {
            dither_view(&ImageSettings {
                dither,
                ..ImageSettings::DEFAULT
            })
        };
        assert_eq!(
            view(Dither::Threshold).mode_field.unwrap().field,
            "threshold"
        );
        assert!(
            view(Dither::Grey).mode_field.is_none(),
            "no Lowest power here"
        );
        assert!(view(Dither::FloydSteinberg).mode_field.is_none());
        assert_eq!(view(Dither::FloydSteinberg).screen_fields, []);
        let dots: Vec<_> = view(Dither::Halftone)
            .screen_fields
            .iter()
            .map(|f| (f.field, f.label, f.unit, f.value))
            .collect();
        assert_eq!(
            dots,
            [
                ("screen_size", "Cell size", "mm", 0.8),
                ("screen_angle", "Angle", "°", 45.0)
            ],
            "criterion 42"
        );
        let lines = view(Dither::LineScreen).screen_fields;
        assert_eq!(
            (lines[0].label, lines[0].range),
            ("Line pitch", "Line pitch is 0.20 to 5.00 mm."),
            "criterion 43"
        );
        let labels: Vec<&str> = view(Dither::Ordered)
            .modes
            .iter()
            .map(|m| m.label)
            .collect();
        assert_eq!(
            labels,
            [
                "Threshold",
                "Ordered",
                "Floyd-Steinberg",
                "Jarvis",
                "Atkinson",
                "Stucki",
                "Grey",
                "Halftone dots",
                "Line screen",
            ]
        );
        let image = ImageSettings::DEFAULT;
        let pictures_only = image_settings_view(&image, RasterFacts::default());
        assert_eq!(
            pictures_only
                .fields
                .iter()
                .map(|f| f.field)
                .collect::<Vec<_>>(),
            ["brightness", "contrast", "gamma"]
        );
        assert_eq!(
            (pictures_only.dither, pictures_only.lowest_power),
            (None, None)
        );
        let both = image_settings_view(
            &image,
            RasterFacts {
                gradient: true,
                grey: true,
            },
        );
        assert_eq!(both.dither.unwrap().dither, "floyd_steinberg");
        let lowest = both.lowest_power.unwrap();
        assert_eq!(
            (lowest.field, lowest.label, lowest.range),
            (
                "lowest_power",
                "Lowest power",
                "Lowest power is 0 to 100 %."
            )
        );
    }
}
