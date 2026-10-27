//! An `<image>` (`specs/0024-svg-import-export` criteria 100 to 103, 105): the bytes of its
//! `data:` URI and its viewport; a linked image (a path or an address) is never read.

use roxmltree::Node;
use svgtypes::Align;

use crate::data_uri::decode_data_uri;
use crate::drawing::SvgNode;
use crate::drawing_text::{ImageAspect, SvgImage};
use crate::reader::{Ctx, Reader};
use crate::shapes::attr;
use crate::style::href;
use crate::units::{Axis, aspect};

/// An embedded image under `ctx`, or `None` when it is linked, damaged or has no area.
pub(crate) fn image(node: Node<'_, '_>, ctx: &Ctx) -> Option<SvgNode> {
    // The `data:` value is exempt from the attribute limit (criterion 105).
    let bytes = decode_data_uri(href(node)?)?;
    let length = |name: &str, axis| attr(node, name).and_then(|t| ctx.units.parse(t, axis));
    let extent = |name: &str, axis| -> Result<Option<f64>, ()> {
        match attr(node, name).map(str::trim) {
            None | Some("auto") => Ok(None),
            Some(_) => match length(name, axis) {
                Some(v) if v > 0.0 => Ok(Some(v)),
                _ => Err(()),
            },
        }
    };
    let size = (
        extent("width", Axis::X).ok()?,
        extent("height", Axis::Y).ok()?,
    );
    let rule = aspect(node.attribute("preserveAspectRatio"));
    let align = match rule.align {
        Align::None => None,
        Align::XMinYMin => Some((0.0, 0.0)),
        Align::XMidYMin => Some((0.5, 0.0)),
        Align::XMaxYMin => Some((1.0, 0.0)),
        Align::XMinYMid => Some((0.0, 0.5)),
        Align::XMidYMid => Some((0.5, 0.5)),
        Align::XMaxYMid => Some((1.0, 0.5)),
        Align::XMinYMax => Some((0.0, 1.0)),
        Align::XMidYMax => Some((0.5, 1.0)),
        Align::XMaxYMax => Some((1.0, 1.0)),
    };
    Some(SvgNode::Image {
        state: Reader::state(node, &ctx.style, true),
        image: Box::new(SvgImage {
            bytes,
            origin: (
                length("x", Axis::X).unwrap_or(0.0),
                length("y", Axis::Y).unwrap_or(0.0),
            ),
            size,
            aspect: ImageAspect {
                align,
                slice: rule.slice,
            },
            matrix: ctx.matrix,
        }),
    })
}
