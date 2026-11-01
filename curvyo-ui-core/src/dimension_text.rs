//! Formats a dimension's value as its canvas text ("12.5 mm", "R 25 mm", "⌀ 50 mm", "45°"), the
//! one formatter Sketch and Vector dimensions share (`specs/0070-measure-and-dimension` DIM-TEXT).

use curvyo_document_core::{DimensionKind, DimensionValue, DisplayUnit, Length};

/// The decimals a Vector dimension and the measure readout show (`0070` criterion 2).
pub const VECTOR_DECIMALS: usize = 3;

/// `value` with at most `decimals` decimals, trailing zeros trimmed and no "-0".
#[must_use]
pub fn trimmed(value: f64, decimals: usize) -> String {
    let text = format!("{value:.decimals$}");
    let text = if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.')
    } else {
        &text
    };
    if text == "-0" {
        "0".into()
    } else {
        text.into()
    }
}

/// A length in the display unit: "12.5 mm".
#[must_use]
pub fn length_text(length: Length, unit: DisplayUnit, decimals: usize) -> String {
    format!(
        "{} {}",
        trimmed(length.as_mm() / unit.mm_per_unit(), decimals),
        unit.symbol()
    )
}

/// The canvas text of a dimension of `kind` with `value` (a length, or an angle for the angle kind):
/// "R " for a radius, "⌀ " for a diameter, "↔ " / "↕ " for a horizontal or vertical distance
/// (shown as its size whatever side the geometry is on), degrees for an angle.
#[must_use]
pub fn dimension_text(
    kind: DimensionKind,
    value: DimensionValue,
    unit: DisplayUnit,
    decimals: usize,
) -> String {
    let value = value.raw();
    let prefix = match kind {
        DimensionKind::Angle => return format!("{}°", trimmed(value.to_degrees(), decimals)),
        DimensionKind::Radius => "R ",
        DimensionKind::Diameter => "⌀ ",
        DimensionKind::Horizontal => "↔ ",
        DimensionKind::Vertical => "↕ ",
        DimensionKind::Aligned => "",
    };
    let value = if prefix.is_empty() {
        value
    } else {
        value.abs()
    };
    format!(
        "{prefix}{}",
        length_text(Length::from_mm(value), unit, decimals)
    )
}

/// The spoken form of a dimension's value: "12.5 millimetres", "45 degrees".
#[must_use]
pub fn dimension_speech(kind: DimensionKind, value: DimensionValue, unit: DisplayUnit) -> String {
    let value = value.raw();
    if kind == DimensionKind::Angle {
        return format!("{} degrees", trimmed(value.to_degrees(), VECTOR_DECIMALS));
    }
    let word = match unit {
        DisplayUnit::Mm => "millimetres",
        DisplayUnit::Cm => "centimetres",
        DisplayUnit::In => "inches",
    };
    format!(
        "{} {word}",
        trimmed(value.abs() / unit.mm_per_unit(), VECTOR_DECIMALS)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_use_the_unit_the_prefixes_and_trimmed_decimals() {
        let text = |kind, value| {
            let value = if kind == DimensionKind::Angle {
                DimensionValue::Angle(curvyo_document_core::Angle::from_radians(value))
            } else {
                DimensionValue::Length(Length::from_mm(value))
            };
            dimension_text(kind, value, DisplayUnit::Mm, VECTOR_DECIMALS)
        };
        assert_eq!(text(DimensionKind::Aligned, 12.5), "12.5 mm");
        assert_eq!(text(DimensionKind::Radius, 25.0), "R 25 mm");
        assert_eq!(text(DimensionKind::Diameter, 50.0), "⌀ 50 mm");
        assert_eq!(text(DimensionKind::Horizontal, -8.25), "↔ 8.25 mm");
        assert_eq!(text(DimensionKind::Aligned, 21.1744), "21.174 mm");
        assert_eq!(
            text(DimensionKind::Angle, std::f64::consts::FRAC_PI_4),
            "45°"
        );
        assert_eq!(
            dimension_text(
                DimensionKind::Aligned,
                DimensionValue::Length(Length::from_mm(25.4)),
                DisplayUnit::In,
                3
            ),
            "1 in"
        );
        assert_eq!(trimmed(-0.0001, 3), "0");
    }

    #[test]
    fn speech_names_the_unit() {
        assert_eq!(
            dimension_speech(
                DimensionKind::Aligned,
                DimensionValue::Length(Length::from_mm(12.5)),
                DisplayUnit::Mm
            ),
            "12.5 millimetres"
        );
        assert_eq!(
            dimension_speech(
                DimensionKind::Angle,
                DimensionValue::Angle(curvyo_document_core::Angle::from_radians(
                    std::f64::consts::FRAC_PI_4
                )),
                DisplayUnit::Mm
            ),
            "45 degrees"
        );
    }
}
