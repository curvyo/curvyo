//! The contrast line of a code (`specs/0144` criterion 9): whether the code's fill is hard to tell
//! from the document background (WCAG contrast below 3 : 1), so "Low contrast, may not scan" shows.

use curvyo_document_core::{Color, Fill, FillPaint};

/// The WCAG ratio below which the line shows.
const LOW: f64 = 3.0;

/// The relative luminance of an sRGB colour (WCAG 2.x).
fn luminance(colour: Color) -> f64 {
    let channel = |value: u8| {
        let c = f64::from(value) / 255.0;
        if c <= 0.039_28 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(colour.r) + 0.7152 * channel(colour.g) + 0.0722 * channel(colour.b)
}

/// The contrast ratio of two colours, 1 to 21.
#[must_use]
pub fn contrast(a: Color, b: Color) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// Whether the solid `colour` (a code's Finder colour) is below 3 : 1 against `background`.
#[must_use]
pub fn colour_is_low(colour: Color, background: Color) -> bool {
    contrast(colour, background) < LOW
}

/// Whether the fill of a code is below 3 : 1 against `background`: for a gradient the worst stop,
/// `false` for a fill that is off.
#[must_use]
pub fn is_low(fill: &Fill, background: Color) -> bool {
    if !fill.paints() {
        return false;
    }
    match (fill.paint, &fill.gradient) {
        (FillPaint::Gradient, Some(gradient)) => gradient
            .stops
            .as_slice()
            .iter()
            .any(|stop| contrast(stop.colour, background) < LOW),
        _ => contrast(fill.color, background) < LOW,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLACK: Color = Color { r: 0, g: 0, b: 0 };

    #[test]
    fn black_on_white_is_21_and_light_grey_on_white_is_low() {
        assert!((contrast(BLACK, Color::WHITE) - 21.0).abs() < 1e-9);
        let grey = Color {
            r: 220,
            g: 220,
            b: 220,
        };
        assert!(contrast(grey, Color::WHITE) < LOW);
    }

    #[test]
    fn a_light_code_on_a_dark_background_passes_and_a_fill_that_is_off_never_warns() {
        let mut fill = Fill {
            color: Color::WHITE,
            enabled: true,
            ..Fill::default()
        };
        assert!(!is_low(&fill, BLACK));
        assert!(is_low(
            &fill,
            Color {
                r: 240,
                g: 240,
                b: 240
            }
        ));
        fill.enabled = false;
        assert!(!is_low(
            &fill,
            Color {
                r: 240,
                g: 240,
                b: 240
            }
        ));
    }
}
