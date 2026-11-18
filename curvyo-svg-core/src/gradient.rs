//! Reads one gradient paint server through its `href` chain (`specs/0124-gradient-fill`
//! criterion 14, ADR 0067 §7): each attribute from the nearest link that has it, the stops from
//! the nearest link that has any, both `gradientUnits`, `gradientTransform`, and the stops
//! normalised as SVG says. Nothing outside the file is followed.

use kurbo::{Affine, Point, Rect};
use roxmltree::Node;

use crate::gradient_paint::{SvgGradient, SvgGradientShape, SvgStop};
use crate::paint::{BLACK, Ids, Rgba, colour};
use crate::units::{Axis, Units};

/// How many gradients a chain of `href`s may pass.
const MAX_GRADIENT_LINKS: usize = 16;

/// A focal point closer to the centre than this (in the gradient's own units) is the centre.
const SAME_FOCUS: f64 = 1e-9;

/// One gradient as read, before it is placed on an element.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReadGradient {
    /// The shape in gradient units and `gradientTransform` as [`SvgGradient::transform`].
    pub(crate) gradient: SvgGradient,
    /// `gradientUnits="objectBoundingBox"` (the default): the numbers are fractions of the box.
    pub(crate) bbox_units: bool,
    /// `spreadMethod` `reflect` or `repeat`, drawn as pad.
    pub(crate) repeats: bool,
    /// A focal point away from the centre, read as the centre.
    pub(crate) focal: bool,
}

/// Whether `node` is a linear or a radial gradient.
pub(crate) fn is_gradient(node: Node<'_, '_>) -> bool {
    matches!(node.tag_name().name(), "linearGradient" | "radialGradient")
}

/// Whether `node` is a mesh gradient (SVG 2 drafts and Inkscape write both spellings).
pub(crate) fn is_mesh(node: Node<'_, '_>) -> bool {
    matches!(node.tag_name().name(), "meshgradient" | "meshGradient")
}

/// `server` and the gradients its `href`s reach, nearest first: at most
/// [`MAX_GRADIENT_LINKS`] links, a cycle or a link to anything else ends the chain.
fn chain<'a, 'input>(server: Node<'a, 'input>, ids: &Ids<'a, 'input>) -> Vec<Node<'a, 'input>> {
    let mut out = vec![server];
    let mut current = server;
    for _ in 0..MAX_GRADIENT_LINKS {
        let Some(next) = crate::style::href(current)
            .and_then(|link| link.strip_prefix('#'))
            .and_then(|id| ids.get(id))
        else {
            break;
        };
        if !is_gradient(*next) || out.iter().any(|seen| seen.id() == next.id()) {
            break;
        }
        out.push(*next);
        current = *next;
    }
    out
}

/// The `stop` children of the nearest link that has any.
fn stop_nodes<'a, 'input>(links: &[Node<'a, 'input>]) -> Vec<Node<'a, 'input>> {
    links
        .iter()
        .map(|link| {
            link.children()
                .filter(|n| n.is_element() && n.tag_name().name() == "stop")
                .collect::<Vec<_>>()
        })
        .find(|stops| !stops.is_empty())
        .unwrap_or_default()
}

/// The first stop's colour and the colour at the middle of the line (interpolated in sRGB,
/// halves up) of the gradient `server`; `None` without a stop, so it paints nothing.
pub(crate) fn stand_ins(server: Node<'_, '_>, ids: &Ids<'_, '_>) -> Option<(Rgba, Rgba)> {
    let stops = stops(&chain(server, ids), 1.0);
    let first = stops.first()?;
    let after = stops.iter().position(|stop| stop.offset > 0.5)?;
    let (s0, s1) = (stops[after - 1], stops[after]);
    let t = if s1.offset > s0.offset {
        (0.5 - s0.offset) / (s1.offset - s0.offset)
    } else {
        1.0
    };
    let mix = |a: u8, b: u8| {
        let value = f64::from(a) + (f64::from(b) - f64::from(a)) * t;
        // invariant: between two channel values, so within 0..=255.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let rounded = (value + 0.5).floor().clamp(0.0, 255.0) as u8;
        rounded
    };
    let rgba = |stop: &SvgStop| Rgba {
        rgb: stop.rgb,
        alpha: stop.alpha,
    };
    Some((
        rgba(first),
        Rgba {
            rgb: [0, 1, 2].map(|i| mix(s0.rgb[i], s1.rgb[i])),
            alpha: s0.alpha + (s1.alpha - s0.alpha) * t,
        },
    ))
}

/// A stop property from its attribute or its `style` (the later wins).
fn stop_property<'a>(stop: Node<'a, '_>, name: &str) -> Option<&'a str> {
    let mut value = stop.attribute(name);
    if let Some(style) = stop.attribute("style") {
        for declaration in simplecss::DeclarationTokenizer::from(style) {
            if declaration.name.eq_ignore_ascii_case(name) {
                value = Some(declaration.value);
            }
        }
    }
    value
}

/// A number or a percentage; `None` when it cannot be read.
fn number_or_percent(text: &str) -> Option<f64> {
    let text = text.trim();
    let (number, scale) = text.strip_suffix('%').map_or((text, 1.0), |n| (n, 0.01));
    let value = number.trim().parse::<f64>().ok()? * scale;
    value.is_finite().then_some(value)
}

/// The stops of `links`: offsets clamped to `[0, 1]` and never below the one before, the
/// colour's alpha times `stop-opacity` times `opacity`, a stop added at 0 and at 1 where the ends
/// are missing; one stop gives exactly two of its colour, at 0 and at 1. Empty without a stop.
fn stops(links: &[Node<'_, '_>], opacity: f64) -> Vec<SvgStop> {
    let mut out: Vec<SvgStop> = Vec::new();
    for stop in stop_nodes(links) {
        let offset = stop
            .attribute("offset")
            .and_then(number_or_percent)
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
        let offset = out
            .last()
            .map_or(offset, |before| offset.max(before.offset));
        let rgba: Rgba = stop_property(stop, "stop-color")
            .and_then(colour)
            .unwrap_or(BLACK);
        let stop_opacity = stop_property(stop, "stop-opacity")
            .and_then(number_or_percent)
            .map_or(1.0, |o| o.clamp(0.0, 1.0));
        out.push(SvgStop {
            offset,
            rgb: rgba.rgb,
            alpha: (rgba.alpha * stop_opacity * opacity).clamp(0.0, 1.0),
        });
    }
    if let [only] = out[..] {
        return vec![
            SvgStop {
                offset: 0.0,
                ..only
            },
            SvgStop {
                offset: 1.0,
                ..only
            },
        ];
    }
    if let Some(&first) = out.first()
        && first.offset > 0.0
    {
        out.insert(
            0,
            SvgStop {
                offset: 0.0,
                ..first
            },
        );
    }
    if let Some(&last) = out.last()
        && last.offset < 1.0
    {
        out.push(SvgStop {
            offset: 1.0,
            ..last
        });
    }
    out
}

/// The shape of a radial (`radial`) or linear gradient from its `length` attributes (name,
/// default, axis), and whether a radial one has a focal point off its centre (`has` says
/// whether an attribute is given).
fn read_shape(
    radial: bool,
    length: &dyn Fn(&str, &str, Axis) -> f64,
    has: &dyn Fn(&str) -> bool,
) -> (SvgGradientShape, bool) {
    if radial {
        let centre = Point::new(length("cx", "50%", Axis::X), length("cy", "50%", Axis::Y));
        let radius = length("r", "50%", Axis::Diagonal);
        let fx = has("fx").then(|| length("fx", "50%", Axis::X));
        let fy = has("fy").then(|| length("fy", "50%", Axis::Y));
        let focus = Point::new(fx.unwrap_or(centre.x), fy.unwrap_or(centre.y));
        let focal = (focus - centre).hypot() > SAME_FOCUS;
        (SvgGradientShape::Radial { centre, radius }, focal)
    } else {
        let start = Point::new(length("x1", "0%", Axis::X), length("y1", "0%", Axis::Y));
        let end = Point::new(length("x2", "100%", Axis::X), length("y2", "0%", Axis::Y));
        (SvgGradientShape::Linear { start, end }, false)
    }
}

/// Reads the gradient `server` for an element with `units`, its `fill-opacity` multiplied into
/// the stops; `None` without a stop.
pub(crate) fn read_gradient(
    server: Node<'_, '_>,
    ids: &Ids<'_, '_>,
    units: &Units,
    opacity: f64,
) -> Option<ReadGradient> {
    let links = chain(server, ids);
    let stops = stops(&links, opacity);
    if stops.is_empty() {
        return None;
    }
    // Every attribute from the nearest link that has it; the shape's numbers only from links of
    // the same kind.
    let kind = server.tag_name().name();
    let find = |name: &str, shape: bool| {
        links
            .iter()
            .filter(|link| !shape || link.tag_name().name() == kind)
            .find_map(|link| link.attribute(name))
    };
    let bbox_units = find("gradientUnits", false).map(str::trim) != Some("userSpaceOnUse");
    let transform = find("gradientTransform", false)
        .and_then(|t| t.parse::<svgtypes::Transform>().ok())
        .map_or(Affine::IDENTITY, |t| {
            Affine::new([t.a, t.b, t.c, t.d, t.e, t.f])
        });
    let repeats = matches!(
        find("spreadMethod", false).map(str::trim),
        Some("reflect" | "repeat")
    );
    let length = |name: &str, default: &str, axis: Axis| {
        let text = find(name, true).unwrap_or(default);
        let read = if bbox_units {
            number_or_percent(text)
        } else {
            units.parse(text, axis)
        };
        read.or_else(|| {
            if bbox_units {
                number_or_percent(default)
            } else {
                units.parse(default, axis)
            }
        })
        .unwrap_or(0.0)
    };
    let (shape, focal) = read_shape(kind == "radialGradient", &length, &|name: &str| {
        find(name, true).is_some()
    });
    Some(ReadGradient {
        gradient: SvgGradient {
            shape,
            transform,
            stops,
        },
        bbox_units,
        repeats,
        focal,
    })
}

impl ReadGradient {
    /// The gradient placed on an element with the user-space box `bbox` under `matrix` (user
    /// units to millimetres): its transform then maps gradient space into millimetres. `None`
    /// for `objectBoundingBox` on an element whose box has no width or no height (SVG then
    /// paints nothing).
    pub(crate) fn placed(&self, matrix: Affine, bbox: Rect) -> Option<SvgGradient> {
        let units = if self.bbox_units {
            if bbox.width() <= 0.0 || bbox.height() <= 0.0 {
                return None;
            }
            Affine::new([bbox.width(), 0.0, 0.0, bbox.height(), bbox.x0, bbox.y0])
        } else {
            Affine::IDENTITY
        };
        Some(SvgGradient {
            transform: matrix * units * self.gradient.transform,
            ..self.gradient.clone()
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn read(text: &str, id: &str) -> Option<ReadGradient> {
        let doc = roxmltree::Document::parse(text).unwrap();
        let ids: Ids<'_, '_> = doc
            .descendants()
            .filter_map(|n| Some((n.attribute("id")?, n)))
            .collect();
        let units = Units {
            px_mm: 1.0,
            viewport: (200.0, 100.0),
        };
        read_gradient(*ids.get(id).unwrap(), &ids, &units, 1.0)
    }

    #[test]
    fn a_chain_lends_stops_and_attributes_from_the_nearest_link() {
        let text = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
            <linearGradient id="v"><stop offset="0.2" style="stop-color:#ff0000;stop-opacity:0.5"/><stop offset="0.1" stop-color="#00f"/></linearGradient>
            <linearGradient id="u" xlink:href="#v" gradientUnits="userSpaceOnUse" x1="10" x2="50%" spreadMethod="reflect"/>
        </svg>"##;
        let g = read(text, "u").unwrap();
        assert!(!g.bbox_units && g.repeats && !g.focal);
        assert_eq!(
            g.gradient.shape,
            SvgGradientShape::Linear {
                start: Point::new(10.0, 0.0),
                end: Point::new(100.0, 0.0)
            }
        );
        let offsets: Vec<f64> = g.gradient.stops.iter().map(|s| s.offset).collect();
        assert_eq!(
            offsets,
            [0.0, 0.2, 0.2, 1.0],
            "completed, clamped, monotonic"
        );
        assert!((g.gradient.stops[0].alpha - 0.5).abs() < 1e-12);
        assert_eq!(g.gradient.stops[3].rgb, [0, 0, 255]);
    }

    #[test]
    fn a_cycle_ends_the_chain_and_no_stop_is_no_gradient() {
        let text = r##"<svg xmlns="http://www.w3.org/2000/svg">
            <linearGradient id="a" href="#b"/><linearGradient id="b" href="#a"/>
        </svg>"##;
        assert_eq!(read(text, "a"), None);
    }

    #[test]
    fn a_focal_point_is_noted_and_one_stop_gives_exactly_two_at_the_ends() {
        let text = r##"<svg xmlns="http://www.w3.org/2000/svg">
            <radialGradient id="r" fx="0.25"><stop offset="0.5" stop-color="#0f0"/></radialGradient>
        </svg>"##;
        let g = read(text, "r").unwrap();
        assert!(g.bbox_units && g.focal);
        assert_eq!(
            g.gradient.shape,
            SvgGradientShape::Radial {
                centre: Point::new(0.5, 0.5),
                radius: 0.5
            }
        );
        let offsets: Vec<f64> = g.gradient.stops.iter().map(|s| s.offset).collect();
        assert_eq!(offsets, [0.0, 1.0], "exactly two stops");
        assert_eq!(
            g.gradient.stops[0],
            SvgStop {
                offset: 0.0,
                ..g.gradient.stops[1]
            }
        );
    }
}
