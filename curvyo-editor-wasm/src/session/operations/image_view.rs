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

/// The Image settings group (UX notes 2.2).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImageSettingsView {
    /// The picked mode's stored spelling.
    pub dither: &'static str,
    /// The seven modes in the pick's order.
    pub modes: Vec<DitherOptionView>,
    /// Row 2: Threshold with Threshold, Lowest power with Grey, `None` for the other five modes.
    pub mode_field: Option<ImageFieldView>,
    /// Rows 3 and 4: Brightness, Contrast, Gamma.
    pub fields: Vec<ImageFieldView>,
    /// The Invert toggle.
    pub invert: bool,
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

/// The group of the settings `image`.
pub(super) fn image_settings_view(image: &ImageSettings) -> ImageSettingsView {
    let mode_field = match image.dither {
        Dither::Threshold => Some(ImageField::Threshold),
        Dither::Grey => Some(ImageField::LowestPower),
        _ => None,
    };
    ImageSettingsView {
        dither: image.dither.key(),
        modes: dither_options(Dither::ALL),
        mode_field: mode_field.map(|field| field_view(field, image)),
        fields: [
            ImageField::Brightness,
            ImageField::Contrast,
            ImageField::Gamma,
        ]
        .into_iter()
        .map(|field| field_view(field, image))
        .collect(),
        invert: image.invert,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threshold_and_lowest_power_share_row_two_and_the_other_modes_leave_it_empty() {
        let view = |dither| {
            image_settings_view(&ImageSettings {
                dither,
                ..ImageSettings::DEFAULT
            })
        };
        assert_eq!(
            view(Dither::Threshold).mode_field.unwrap().field,
            "threshold"
        );
        let grey = view(Dither::Grey).mode_field.unwrap();
        assert_eq!(
            (grey.field, grey.label, grey.range),
            (
                "lowest_power",
                "Lowest power",
                "Lowest power is 0 to 100 %."
            )
        );
        assert!(view(Dither::FloydSteinberg).mode_field.is_none());
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
                "Grey"
            ]
        );
        let fields: Vec<&str> = view(Dither::Ordered)
            .fields
            .iter()
            .map(|f| f.field)
            .collect();
        assert_eq!(fields, ["brightness", "contrast", "gamma"]);
    }
}
