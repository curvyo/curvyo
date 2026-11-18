//! The gradient paint of an exported element (`specs/0124-gradient-fill` criterion 13, ADR 0067
//! §6): the placed gradient as a map from the unit gradient into the exported coordinates, and
//! its stops.

use curvyo_document_core::{Fill, GradientBox, PlacedGradient};
use curvyo_svg_core::{ExportStyle, SvgGradient, SvgGradientShape, SvgStop};
use kurbo::{Affine, Point};

use crate::svg_export_leaf::paint;

fn kurbo(m: curvyo_document_core::Affine) -> Affine {
    Affine::new(m.to_array())
}

/// The stops as the writer takes them.
fn stops(placed: &PlacedGradient) -> Vec<SvgStop> {
    placed
        .stops()
        .as_slice()
        .iter()
        .map(|stop| SvgStop {
            offset: stop.offset.get(),
            rgb: [stop.colour.r, stop.colour.g, stop.colour.b],
            alpha: f64::from(stop.alpha) / 255.0,
        })
        .collect()
}

/// The gradient `fill` paints on an element whose gradient box is `frame`, from the unit
/// gradient into the coordinates `t` maps the document into (the writer re-expresses it in the
/// element's own space); `None` when it paints none or the gradient has no length.
pub(crate) fn export_gradient(
    fill: &Fill,
    frame: Option<GradientBox>,
    t: Affine,
) -> Option<SvgGradient> {
    let placed = PlacedGradient::new(fill.painted_gradient()?, &frame?);
    let out = (kurbo(placed.to_gradient()) * t.inverse()).inverse();
    let det = out.determinant();
    if !det.is_finite() || det.abs() < 1e-18 {
        return None;
    }
    let shape = if placed.is_radial() {
        SvgGradientShape::Radial {
            centre: Point::ORIGIN,
            radius: 1.0,
        }
    } else {
        SvgGradientShape::Linear {
            start: Point::ORIGIN,
            end: Point::new(1.0, 0.0),
        }
    };
    Some(SvgGradient {
        shape,
        transform: out,
        stops: stops(&placed),
    })
}

/// `style` with the gradient `fill` paints on an element with gradient box `frame`, mapped by
/// `t`: the gradient, and its first stop as the fill colour for the writer's other uses.
pub(crate) fn with_gradient(
    mut style: ExportStyle,
    fill: &Fill,
    frame: Option<GradientBox>,
    t: Affine,
) -> ExportStyle {
    if let Some(gradient) = export_gradient(fill, frame, t) {
        let first = gradient.stops[0];
        style.fill = Some(paint(
            curvyo_document_core::Color {
                r: first.rgb[0],
                g: first.rgb[1],
                b: first.rgb[2],
            },
            first.alpha,
        ));
        style.fill_gradient = Some(gradient);
    }
    style
}
