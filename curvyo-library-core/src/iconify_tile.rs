//! An icon as a browser tile (ADR 0086 decisions 1 and 8): whether its body draws fixed colours,
//! and its SVG with the current colour set on the root for an `<img>`.

use crate::iconify::{IconBody, icon_svg};

/// A colour as three bytes; never page text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb8(pub u8, pub u8, pub u8);

/// The paint properties whose value decides whether an icon has its own colours.
const PAINTS: [&str; 4] = ["fill", "stroke", "stop-color", "color"];
/// The values that leave the colour to the object.
const FREE: [&str; 5] = ["currentcolor", "none", "inherit", "transparent", ""];

fn is_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' || byte == b':'
}

/// The value after `key` at `at` when `key` is a whole property name followed by `=` or `:`.
fn value_at<'a>(text: &'a str, at: usize, key: &str) -> Option<&'a str> {
    let bytes = text.as_bytes();
    if at > 0 && is_name_byte(bytes[at - 1]) {
        return None;
    }
    let rest = text[at + key.len()..].trim_start();
    let (sep, rest) = rest.split_at(rest.chars().next().map_or(0, char::len_utf8));
    match sep {
        "=" => {
            let rest = rest.trim_start();
            let quote = rest.chars().next()?;
            if quote == '"' || quote == '\'' {
                let inner = &rest[1..];
                Some(&inner[..inner.find(quote).unwrap_or(inner.len())])
            } else {
                Some(
                    rest.split(|c: char| c.is_whitespace() || c == '>' || c == '/')
                        .next()?,
                )
            }
        }
        ":" => Some(rest.split([';', '"', '\'', '}']).next().unwrap_or(rest)),
        _ => None,
    }
}

/// Whether `body` draws a colour of its own: a `fill`, `stroke`, `stop-color` or `color` (as an
/// attribute or in a style) other than `currentColor`, `none`, `inherit` or `transparent`, or an
/// `<image>`. Pure; the refinement of spec `0152` criterion 14.
#[must_use]
pub fn has_fixed_colour(body: &str) -> bool {
    let text = body.to_ascii_lowercase();
    if text.contains("<image") {
        return true;
    }
    PAINTS.iter().any(|key| {
        text.match_indices(key).any(|(at, _)| {
            value_at(&text, at, key).is_some_and(|value| !FREE.contains(&value.trim()))
        })
    })
}

/// [`icon_svg`] with `color="#rrggbb"` on the root, so `currentColor` draws in `colour` inside an
/// `<img>` (ADR 0086 decision 1).
#[must_use]
pub fn icon_tile_svg(icon: &IconBody, colour: Rgb8) -> String {
    let Rgb8(r, g, b) = colour;
    icon_svg(icon).replacen(
        "<svg ",
        &format!("<svg color=\"#{r:02x}{g:02x}{b:02x}\" "),
        1,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_colour_and_none_leave_the_colour_to_the_object() {
        assert!(!has_fixed_colour(
            "<path fill=\"currentColor\" fill-rule=\"evenodd\" d=\"M0 0\"/>"
        ));
        assert!(!has_fixed_colour(
            "<g fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\"><path d=\"M0 0\"/></g>"
        ));
        assert!(!has_fixed_colour("<path d=\"M0 0\"/>"));
        assert!(!has_fixed_colour(
            "<path style=\"fill: currentColor; opacity:.5\"/>"
        ));
    }

    #[test]
    fn a_fixed_paint_a_gradient_or_an_image_is_its_own_colour() {
        assert!(has_fixed_colour("<path fill=\"#f44336\" d=\"M0 0\"/>"));
        assert!(has_fixed_colour("<stop offset=\"0\" stop-color=\"#fff\"/>"));
        assert!(has_fixed_colour("<path fill=\"url(#SVGa)\"/>"));
        assert!(has_fixed_colour("<path style=\"stroke:red\"/>"));
        assert!(has_fixed_colour("<image href=\"data:x\"/>"));
        assert!(!has_fixed_colour("<path data-fill=\"red\" d=\"M0 0\"/>"));
    }

    #[test]
    fn the_tile_svg_carries_the_colour_on_its_root() {
        let icon = IconBody {
            name: "a".into(),
            body: "<path fill=\"currentColor\" d=\"M0 0\"/>".into(),
            left: 0.0,
            top: 0.0,
            width: 24.0,
            height: 24.0,
            rotate: 0,
            h_flip: false,
            v_flip: false,
        };
        let svg = icon_tile_svg(&icon, Rgb8(0x1f, 0x29, 0x37));
        assert!(svg.starts_with("<svg color=\"#1f2937\" xmlns="), "{svg}");
    }
}
