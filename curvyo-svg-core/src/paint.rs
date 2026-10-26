//! Colours and paints (`specs/0024-svg-import-export` criteria 37 and 38): every CSS colour form
//! through `svgtypes`, `currentColor`, and a `url(#id)` to a gradient (its middle colour), a
//! pattern (mid grey) or nothing (the fallback, else none). Nothing outside the file is followed.

use std::collections::BTreeMap;

use roxmltree::Node;

use crate::drawing::Rgb;

/// A colour with alpha (0 to 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Rgba {
    pub(crate) rgb: Rgb,
    pub(crate) alpha: f64,
}

/// Black, the default of `color` and of a fill.
pub(crate) const BLACK: Rgba = Rgba {
    rgb: [0, 0, 0],
    alpha: 1.0,
};

/// The colour a pattern paint becomes.
const PATTERN_GREY: Rgba = Rgba {
    rgb: [0x80, 0x80, 0x80],
    alpha: 1.0,
};

/// How many gradients a chain of `href`s may pass to find the stops.
const MAX_GRADIENT_LINKS: usize = 16;

/// A paint as specified: off, or a colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum PaintValue {
    /// `none`.
    None,
    /// A colour (from a colour, `currentColor`, or a paint server's substitute).
    Colour(Rgba),
}

/// The elements of the file by `id` (the first element with an id wins).
pub(crate) type Ids<'a, 'input> = BTreeMap<&'a str, Node<'a, 'input>>;

/// A colour value; `None` for one that cannot be read (including `currentColor`, which the
/// caller resolves).
pub(crate) fn colour(text: &str) -> Option<Rgba> {
    let text = text.trim();
    // The 148th CSS colour name, which `svgtypes` lacks.
    if text.eq_ignore_ascii_case("rebeccapurple") {
        return Some(Rgba {
            rgb: [0x66, 0x33, 0x99],
            alpha: 1.0,
        });
    }
    let c: svgtypes::Color = text.parse().ok()?;
    Some(Rgba {
        rgb: [c.red, c.green, c.blue],
        alpha: functional_alpha(text).unwrap_or(f64::from(c.alpha) / 255.0),
    })
}

/// The alpha of `rgba()`, `hsla()` (and `rgb()`, `hsl()` with four arguments) as written, not
/// rounded to 8 bits: `rgba(255,0,0,0.5)` has the alpha 0.5 exactly.
fn functional_alpha(text: &str) -> Option<f64> {
    let (_, inner) = text.split_once('(')?;
    let inner = inner.strip_suffix(')')?;
    let parts: Vec<&str> = inner.split([',', '/']).map(str::trim).collect();
    let alpha = parts.get(3)?;
    let (number, scale) = alpha.strip_suffix('%').map_or((*alpha, 1.0), |n| (n, 0.01));
    let value = number.trim().parse::<f64>().ok()? * scale;
    value.is_finite().then(|| value.clamp(0.0, 1.0))
}

/// A `fill` or `stroke` value with `current` as `currentColor`; `None` when it cannot be read
/// (the property then keeps the value it would have without it). `inherit` is the caller's.
pub(crate) fn paint(text: &str, current: Rgba, ids: &Ids<'_, '_>) -> Option<PaintValue> {
    let text = text.trim();
    if text.eq_ignore_ascii_case("none") {
        return Some(PaintValue::None);
    }
    if text.eq_ignore_ascii_case("currentcolor") {
        return Some(PaintValue::Colour(current));
    }
    if let Some(rest) = text.strip_prefix("url(") {
        let (target, fallback) = rest.split_once(')')?;
        let target = target.trim().trim_matches(['"', '\'']);
        let fallback = fallback.trim();
        let server = target.strip_prefix('#').and_then(|id| ids.get(id));
        if let Some(server) = server
            && let Some(found) = server_colour(*server, ids)
        {
            return Some(found);
        }
        if fallback.is_empty() || target.strip_prefix('#').is_none() {
            return Some(PaintValue::None);
        }
        return paint(fallback, current, ids);
    }
    colour(text).map(PaintValue::Colour)
}

/// The substitute of a paint server: a gradient's middle colour, a pattern's grey; `None` for
/// any other element (the fallback applies).
fn server_colour(server: Node<'_, '_>, ids: &Ids<'_, '_>) -> Option<PaintValue> {
    match server.tag_name().name() {
        "linearGradient" | "radialGradient" => {
            Some(gradient_middle(server, ids).map_or(PaintValue::None, PaintValue::Colour))
        }
        "pattern" => Some(PaintValue::Colour(PATTERN_GREY)),
        _ => None,
    }
}

/// The colour at offset 0.5 of a gradient's stops, interpolated in sRGB, each channel rounded
/// to the nearest whole value with halves up. The stops may come through `href`s to other
/// gradients. `None` without stops.
fn gradient_middle(gradient: Node<'_, '_>, ids: &Ids<'_, '_>) -> Option<Rgba> {
    let mut current = gradient;
    let mut stops = Vec::new();
    for _ in 0..MAX_GRADIENT_LINKS {
        stops = read_stops(current);
        if !stops.is_empty() {
            break;
        }
        let link = crate::style::href(current)?.strip_prefix('#')?;
        current = *ids.get(link)?;
    }
    let first = stops.first()?;
    let last = stops.last()?;
    if 0.5 <= first.0 {
        return Some(first.1);
    }
    if 0.5 >= last.0 {
        return Some(last.1);
    }
    let after = stops.iter().position(|(offset, _)| *offset > 0.5)?;
    let (o0, c0) = stops[after - 1];
    let (o1, c1) = stops[after];
    let t = if o1 > o0 { (0.5 - o0) / (o1 - o0) } else { 1.0 };
    let mix = |a: u8, b: u8| {
        let value = f64::from(a) + (f64::from(b) - f64::from(a)) * t;
        // invariant: between two channel values, so within 0..=255.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let rounded = (value + 0.5).floor().clamp(0.0, 255.0) as u8;
        rounded
    };
    Some(Rgba {
        rgb: [
            mix(c0.rgb[0], c1.rgb[0]),
            mix(c0.rgb[1], c1.rgb[1]),
            mix(c0.rgb[2], c1.rgb[2]),
        ],
        alpha: c0.alpha + (c1.alpha - c0.alpha) * t,
    })
}

/// The `stop` children of a gradient: offsets clamped to 0..=1 and never below the one before,
/// colours from `stop-color` (attribute or `style`), black by default.
fn read_stops(gradient: Node<'_, '_>) -> Vec<(f64, Rgba)> {
    let mut stops: Vec<(f64, Rgba)> = Vec::new();
    for stop in gradient
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "stop")
    {
        let offset = stop.attribute("offset").map_or(0.0, |text| {
            let text = text.trim();
            let (number, scale) = text.strip_suffix('%').map_or((text, 1.0), |n| (n, 0.01));
            number.trim().parse::<f64>().map_or(0.0, |n| n * scale)
        });
        let offset = if offset.is_finite() {
            offset.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let offset = stops
            .last()
            .map_or(offset, |(before, _)| offset.max(*before));
        let mut colour_text = stop.attribute("stop-color");
        if let Some(style) = stop.attribute("style") {
            for declaration in simplecss::DeclarationTokenizer::from(style) {
                if declaration.name.eq_ignore_ascii_case("stop-color") {
                    colour_text = Some(declaration.value);
                }
            }
        }
        let rgba = colour_text.and_then(colour).unwrap_or(BLACK);
        stops.push((offset, rgba));
    }
    stops
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn ids<'a, 'input>(doc: &'a roxmltree::Document<'input>) -> Ids<'a, 'input> {
        doc.descendants()
            .filter_map(|n| Some((n.attribute("id")?, n)))
            .collect()
    }

    #[test]
    fn colour_forms() {
        let red = Rgba {
            rgb: [255, 0, 0],
            alpha: 1.0,
        };
        for text in [
            "#f00",
            "#ff0000",
            "rgb(255,0,0)",
            "rgb(100%, 0%, 0%)",
            "red",
            "RED",
            "hsl(0, 100%, 50%)",
        ] {
            assert_eq!(colour(text), Some(red), "{text}");
        }
        assert_eq!(colour("rgba(255,0,0,0.5)").unwrap().rgb, [255, 0, 0]);
        assert!((colour("#ff000080").unwrap().alpha - 128.0 / 255.0).abs() < 1e-12);
        assert_eq!(colour("transparent").unwrap().alpha, 0.0);
        assert_eq!(colour("bogus"), None);
    }

    #[test]
    fn paint_servers_become_colours() {
        let text = r##"<svg xmlns="http://www.w3.org/2000/svg">
            <linearGradient id="g"><stop offset="0" stop-color="#000"/><stop offset="1" style="stop-color:#fff"/></linearGradient>
            <linearGradient id="h" href="#g"/>
            <pattern id="p"/>
        </svg>"##;
        let doc = roxmltree::Document::parse(text).unwrap();
        let ids = ids(&doc);
        let grey = PaintValue::Colour(Rgba {
            rgb: [0x80; 3],
            alpha: 1.0,
        });
        assert_eq!(paint("url(#g)", BLACK, &ids), Some(grey));
        assert_eq!(paint("url(#h)", BLACK, &ids), Some(grey));
        assert_eq!(paint("url(#p)", BLACK, &ids), Some(grey));
        assert_eq!(paint("url(#missing)", BLACK, &ids), Some(PaintValue::None));
        assert_eq!(
            paint("url(#missing) red", BLACK, &ids),
            Some(PaintValue::Colour(Rgba {
                rgb: [255, 0, 0],
                alpha: 1.0
            }))
        );
        assert_eq!(
            paint("url(file.svg#g) red", BLACK, &ids),
            Some(PaintValue::None)
        );
        assert_eq!(
            paint("currentColor", BLACK, &ids),
            Some(PaintValue::Colour(BLACK))
        );
    }
}
