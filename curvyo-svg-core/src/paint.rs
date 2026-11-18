//! Colours and paints (`specs/0024-svg-import-export` criteria 37 and 38, `specs/0124-gradient-fill`
//! criterion 14): every CSS colour form through `svgtypes`, `currentColor`, and a `url(#id)` to a
//! gradient (kept as a gradient), a mesh gradient (no paint, counted), a pattern (mid grey) or
//! nothing (the fallback, else none). Nothing outside the file is followed.

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

/// A paint as specified: off, a colour or a gradient.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum PaintValue {
    /// `none`.
    None,
    /// A colour (from a colour, `currentColor`, or a pattern's substitute).
    Colour(Rgba),
    /// A linear or radial gradient with at least one stop.
    Gradient(GradientPaint),
    /// A mesh gradient: paints nothing, and the import notice counts it.
    Mesh(roxmltree::NodeId),
}

/// A gradient paint as the cascade carries it: the server and the colours that stand in where
/// a colour is needed (the Colour register of the fill, a stroke).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GradientPaint {
    /// The gradient element.
    pub(crate) server: roxmltree::NodeId,
    /// Its first stop's colour.
    pub(crate) first: Rgba,
    /// Its colour at the middle of the line, which a stroke takes (a stroke has no gradient).
    pub(crate) middle: Rgba,
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

/// The paint of a paint server: a gradient as itself (none without a stop), a mesh as counted
/// nothing, a pattern's grey; `None` for any other element (the fallback applies).
fn server_colour(server: Node<'_, '_>, ids: &Ids<'_, '_>) -> Option<PaintValue> {
    if crate::gradient::is_gradient(server) {
        return Some(crate::gradient::stand_ins(server, ids).map_or(
            PaintValue::None,
            |(first, middle)| {
                PaintValue::Gradient(GradientPaint {
                    server: server.id(),
                    first,
                    middle,
                })
            },
        ));
    }
    if crate::gradient::is_mesh(server) {
        return Some(PaintValue::Mesh(server.id()));
    }
    match server.tag_name().name() {
        "pattern" => Some(PaintValue::Colour(PATTERN_GREY)),
        _ => None,
    }
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
    fn paint_servers_become_paints() {
        let text = r##"<svg xmlns="http://www.w3.org/2000/svg">
            <linearGradient id="g"><stop offset="0" stop-color="#000"/><stop offset="1" style="stop-color:#fff"/></linearGradient>
            <linearGradient id="h" href="#g"/>
            <linearGradient id="empty"/>
            <meshgradient id="m"/>
            <pattern id="p"/>
        </svg>"##;
        let doc = roxmltree::Document::parse(text).unwrap();
        let ids = ids(&doc);
        let grey = PaintValue::Colour(Rgba {
            rgb: [0x80; 3],
            alpha: 1.0,
        });
        let Some(PaintValue::Gradient(g)) = paint("url(#h)", BLACK, &ids) else {
            panic!("a gradient")
        };
        assert_eq!(g.server, ids["h"].id());
        assert_eq!((g.first.rgb, g.middle.rgb), ([0; 3], [0x80; 3]));
        assert_eq!(paint("url(#empty)", BLACK, &ids), Some(PaintValue::None));
        assert_eq!(
            paint("url(#m)", BLACK, &ids),
            Some(PaintValue::Mesh(ids["m"].id()))
        );
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
