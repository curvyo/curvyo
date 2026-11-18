//! The compact text the two gradient registers store (ADR 0066 §1, amended 2026-11-18), free of
//! Loro.
//!
//! ```text
//!   fill_gradient       : "linear X1 Y1 X2 Y2" | "radial CX CY RX RY" | "radial CX CY RX RY RATIO"
//!   fill_gradient_stops : "#RRGGBB[AA] OFFSET;#RRGGBB[AA] OFFSET;..."      at least 2 stops
//!   number              : -?(0|[1-9][0-9]*)(.[0-9]+)?([eE][-+]?[0-9]+)?    finite
//! ```
//!
//! Fields are separated by exactly one space and stops by `;`, with nothing before or after.
//! A number is written as the shorter of its shortest plain and exponent forms, which both read
//! back to the same `f64`; a colour is written upper case, its alpha only when not 255, and read
//! in either case. A radial ratio of 1 (a circle) is not written.

use crate::gradient_model::{
    AxisRatio, BoxPoint, GradientGeometry, GradientStop, GradientStops, StopOffset,
};
use crate::path_model::Color;

const KIND_LINEAR: &str = "linear";
const KIND_RADIAL: &str = "radial";
const STOP_SEPARATOR: char = ';';

// ---------------------------------------------------------------------------
// Numbers
// ---------------------------------------------------------------------------

/// The shortest text that reads back as exactly `value` (finite).
fn number_text(value: f64) -> String {
    let plain = format!("{value}");
    let exponent = format!("{value:e}");
    if exponent.len() < plain.len() {
        exponent
    } else {
        plain
    }
}

/// Whether `text` is all ASCII digits and not empty.
fn digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

/// A number in the grammar of the module doc, finite; `None` for anything else.
fn parse_number(text: &str) -> Option<f64> {
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let (mantissa, exponent) = match unsigned.split_once(['e', 'E']) {
        Some((m, e)) => (m, Some(e)),
        None => (unsigned, None),
    };
    let (whole, fraction) = match mantissa.split_once('.') {
        Some((w, f)) => (w, Some(f)),
        None => (mantissa, None),
    };
    let whole_ok = digits(whole) && (whole == "0" || !whole.starts_with('0'));
    let fraction_ok = fraction.is_none_or(digits);
    let exponent_ok = exponent.is_none_or(|e| digits(e.strip_prefix(['-', '+']).unwrap_or(e)));
    if !(whole_ok && fraction_ok && exponent_ok) {
        return None;
    }
    text.parse::<f64>().ok().filter(|n| n.is_finite())
}

/// Exactly `N` numbers separated by single spaces.
fn parse_numbers<const N: usize>(fields: &[&str]) -> Option<[f64; N]> {
    let fields: &[&str; N] = fields.try_into().ok()?;
    let mut out = [0.0; N];
    for (slot, field) in out.iter_mut().zip(fields) {
        *slot = parse_number(field)?;
    }
    Some(out)
}

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

/// The stored text of `geometry`.
pub(crate) fn geometry_text(geometry: &GradientGeometry) -> String {
    let (kind, numbers) = match *geometry {
        GradientGeometry::Linear { start, end } => (
            KIND_LINEAR,
            vec![start.x.get(), start.y.get(), end.x.get(), end.y.get()],
        ),
        GradientGeometry::Radial {
            centre,
            radius,
            ratio,
        } => {
            let mut numbers = vec![
                centre.x.get(),
                centre.y.get(),
                radius.x.get(),
                radius.y.get(),
            ];
            if ratio != AxisRatio::CIRCLE {
                numbers.push(ratio.get());
            }
            (KIND_RADIAL, numbers)
        }
    };
    let mut text = kind.to_owned();
    for number in numbers {
        text.push(' ');
        text.push_str(&number_text(number));
    }
    text
}

/// The geometry `text` describes; `None` for anything the strict rule refuses.
pub(crate) fn parse_geometry(text: &str) -> Option<GradientGeometry> {
    let fields: Vec<&str> = text.split(' ').collect();
    let (kind, numbers) = fields.split_first()?;
    match *kind {
        KIND_LINEAR => {
            let [x1, y1, x2, y2] = parse_numbers::<4>(numbers)?;
            Some(GradientGeometry::Linear {
                start: BoxPoint::new(x1, y1).ok()?,
                end: BoxPoint::new(x2, y2).ok()?,
            })
        }
        KIND_RADIAL => {
            let (points, ratio) = match numbers.len() {
                4 => (numbers, AxisRatio::CIRCLE),
                5 => (
                    &numbers[..4],
                    AxisRatio::new(parse_number(numbers[4])?).ok()?,
                ),
                _ => return None,
            };
            let [cx, cy, rx, ry] = parse_numbers::<4>(points)?;
            Some(GradientGeometry::Radial {
                centre: BoxPoint::new(cx, cy).ok()?,
                radius: BoxPoint::new(rx, ry).ok()?,
                ratio,
            })
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Stops
// ---------------------------------------------------------------------------

/// The stored text of a stop colour: `#RRGGBB`, plus `AA` when not opaque, upper case.
pub(crate) fn hex_text(colour: Color, alpha: u8) -> String {
    if alpha == u8::MAX {
        format!("#{:02X}{:02X}{:02X}", colour.r, colour.g, colour.b)
    } else {
        format!(
            "#{:02X}{:02X}{:02X}{:02X}",
            colour.r, colour.g, colour.b, alpha
        )
    }
}

/// `#` and six or eight hex digits, either case, as a colour and its alpha (255 when absent).
fn parse_hex(text: &str) -> Option<(Color, u8)> {
    let digits = text.strip_prefix('#')?;
    if !matches!(digits.len(), 6 | 8) || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16).ok();
    let alpha = if digits.len() == 8 { byte(6)? } else { u8::MAX };
    Some((
        Color {
            r: byte(0)?,
            g: byte(2)?,
            b: byte(4)?,
        },
        alpha,
    ))
}

/// The stored text of `stops`.
pub(crate) fn stops_text(stops: &GradientStops) -> String {
    let mut text = String::with_capacity(stops.len() * 12);
    for (i, stop) in stops.as_slice().iter().enumerate() {
        if i > 0 {
            text.push(STOP_SEPARATOR);
        }
        text.push_str(&hex_text(stop.colour, stop.alpha));
        text.push(' ');
        text.push_str(&number_text(stop.offset.get()));
    }
    text
}

fn parse_stop(text: &str) -> Option<GradientStop> {
    let (hex, offset) = text.split_once(' ')?;
    let (colour, alpha) = parse_hex(hex)?;
    Some(GradientStop {
        colour,
        alpha,
        offset: StopOffset::new(parse_number(offset)?).ok()?,
    })
}

/// The stop list `text` describes; `None` for anything the strict rule refuses.
pub(crate) fn parse_stops(text: &str) -> Option<GradientStops> {
    let stops = text
        .split(STOP_SEPARATOR)
        .map(parse_stop)
        .collect::<Option<Vec<_>>>()?;
    GradientStops::new(stops).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stop(r: u8, alpha: u8, offset: f64) -> GradientStop {
        GradientStop {
            colour: Color { r, g: 0x80, b: 0 },
            alpha,
            offset: StopOffset::new(offset).unwrap(),
        }
    }

    #[test]
    fn numbers_are_written_short_and_read_back_exactly() {
        for (value, text) in [
            (0.0, "0"),
            (0.5, "0.5"),
            (1.0, "1"),
            (-0.25, "-0.25"),
            (12.5, "12.5"),
            (1.0 / 3.0, "0.3333333333333333"),
            (1e-7, "1e-7"),
            (2.5e20, "2.5e20"),
        ] {
            assert_eq!(number_text(value), text);
            assert_eq!(parse_number(text).unwrap().to_bits(), value.to_bits());
        }
        for value in [
            0.1 + 0.2,
            0.734_123_456_789_012_3,
            -7.0e-310,
            f64::MAX,
            1e300,
        ] {
            let text = number_text(value);
            assert_eq!(
                parse_number(&text).unwrap().to_bits(),
                value.to_bits(),
                "{text}"
            );
        }
    }

    #[test]
    fn numbers_outside_the_grammar_are_refused() {
        for bad in [
            "", "-", "+1", ".5", "5.", "01", "1..2", "1e", "1e+", "0x10", "inf", "NaN", "1 ", " 1",
            "1,5", "abc", "1e999",
        ] {
            assert_eq!(parse_number(bad), None, "{bad:?}");
        }
        assert_eq!(parse_number("1E+2"), Some(100.0));
        assert_eq!(parse_number("-0"), Some(-0.0));
    }

    #[test]
    fn geometry_round_trips_in_its_documented_text() {
        let ellipse = GradientGeometry::Radial {
            centre: BoxPoint::new(-0.25, 2.0).unwrap(),
            radius: BoxPoint::new(0.5, 0.125).unwrap(),
            ratio: AxisRatio::new(0.4).unwrap(),
        };
        for (geometry, text) in [
            (GradientGeometry::DEFAULT_LINEAR, "linear 0 0.5 1 0.5"),
            (GradientGeometry::DEFAULT_RADIAL, "radial 0.5 0.5 1 0.5"),
            (ellipse, "radial -0.25 2 0.5 0.125 0.4"),
        ] {
            assert_eq!(geometry_text(&geometry), text);
            assert_eq!(parse_geometry(text), Some(geometry));
        }
    }

    /// ADR 0066 §7: each damage reason of the geometry is refused.
    #[test]
    fn damaged_geometry_is_refused() {
        for bad in [
            "",
            "linear",
            "conic 0 0 1 0",
            "Linear 0 0.5 1 0.5",
            "linear 0 0.5 1",
            "linear 0 0.5 1 0.5 1",
            "linear abc 0.5 1 0.5",
            "linear 0  0.5 1 0.5",
            "linear 0 0.5 1 0.5 ",
            "radial 0.5 0.5 1 0.5 0",
            "radial 0.5 0.5 1 0.5 -1",
            "radial 0.5 0.5",
            "radial 0.5 0.5 1 0.5 1 2",
        ] {
            assert_eq!(parse_geometry(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn stops_round_trip_with_alpha_only_when_not_opaque() {
        let stops = GradientStops::new(vec![
            stop(0x01, 255, 0.0),
            stop(0xAB, 0xCD, 1.0 / 3.0),
            stop(0xAB, 0xCD, 1.0 / 3.0),
            stop(0xFF, 0, 1.0),
        ])
        .unwrap();
        let text =
            "#018000 0;#AB8000CD 0.3333333333333333;#AB8000CD 0.3333333333333333;#FF800000 1";
        assert_eq!(stops_text(&stops), text);
        assert_eq!(parse_stops(text), Some(stops.clone()));
        assert_eq!(
            parse_stops(&text.to_lowercase()),
            Some(stops),
            "hex is read in either case"
        );
        assert_eq!(
            parse_stops("#000000FF 0;#FFFFFF 1").unwrap().as_slice()[0].alpha,
            255
        );
    }

    #[test]
    fn many_stops_round_trip() {
        let count = 300_u32;
        let stops = GradientStops::new(
            (0..count)
                .map(|i| {
                    stop(
                        u8::try_from(i % 256).unwrap(),
                        255,
                        f64::from(i) / f64::from(count - 1),
                    )
                })
                .collect(),
        )
        .unwrap();
        assert_eq!(parse_stops(&stops_text(&stops)), Some(stops));
    }

    /// ADR 0066 §7: each damage reason of the stop list is refused.
    #[test]
    fn damaged_stops_are_refused() {
        for bad in [
            "",
            "#000000 0",
            "#00000 0;#FFFFFF 1",
            "#0000000 0;#FFFFFF 1",
            "000000 0;#FFFFFF 1",
            "#00000G 0;#FFFFFF 1",
            "#000000 abc;#FFFFFF 1",
            "#000000 0;#FFFFFF 1.5",
            "#000000 0.1;#FFFFFF 1",
            "#000000 0;#FFFFFF 0.9",
            "#000000 0;#808080 0.7;#808080 0.3;#FFFFFF 1",
            "0 #000000;#FFFFFF 1",
            "#000000 0;#FFFFFF 1;",
            "#000000 0; #FFFFFF 1",
            "#000000  0;#FFFFFF 1",
        ] {
            assert_eq!(parse_stops(bad), None, "{bad:?}");
        }
    }
}
