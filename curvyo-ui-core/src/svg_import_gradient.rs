//! An imported gradient fill (`specs/0124-gradient-fill` criteria 14 and 27; ADR 0067 §7): the
//! reader's gradient, placed in millimetres, as the model's fractions of the new object's box
//! (the exact image, so a skew or an unequal scale gives the ellipse), and the notice lines of
//! what the file lost.

use curvyo_document_core::{
    Affine, AnchorId, AxisRatio, BoxPoint, Color, DrawingOutline, FillPaint, GradientBox,
    GradientFill, GradientGeometry, GradientStop, GradientStops, NewAnchor, Point, StopOffset,
    Style, outlines_gradient_box,
};
use curvyo_svg_core::{GradientLosses, SvgGradient, SvgGradientShape};

/// The box of an imported path with `outlines` (no rotation).
pub(crate) fn outlines_box(outlines: &[DrawingOutline]) -> Option<GradientBox> {
    let mut k = 0;
    let anchors: Vec<(Vec<NewAnchor>, bool)> = outlines
        .iter()
        .map(|outline| {
            let anchors = outline
                .anchors
                .iter()
                .map(|a| {
                    k += 1;
                    NewAnchor {
                        id: AnchorId::new(0, k),
                        point: a.point,
                        handle_in: a.handle_in,
                        handle_out: a.handle_out,
                        kind: a.kind,
                    }
                })
                .collect();
            (anchors, outline.closed)
        })
        .collect();
    outlines_gradient_box(&anchors)
}

/// The stops of the model; `None` when the reader's list breaks the model's rules.
fn model_stops(gradient: &SvgGradient) -> Option<GradientStops> {
    let stops = gradient
        .stops
        .iter()
        .map(|stop| {
            // invariant: an alpha in [0, 1] times 255, rounded, lies in [0, 255].
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let alpha = (stop.alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
            Some(GradientStop {
                colour: Color {
                    r: stop.rgb[0],
                    g: stop.rgb[1],
                    b: stop.rgb[2],
                },
                alpha,
                offset: StopOffset::new(stop.offset).ok()?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    GradientStops::new(stops).ok()
}

/// The model gradient of `gradient` (gradient space into the file's millimetres) after the
/// placement `placement`, as fractions of `frame`.
pub(crate) fn model_gradient(
    gradient: &SvgGradient,
    placement: kurbo::Affine,
    frame: &GradientBox,
) -> Option<GradientFill> {
    let point = |p: kurbo::Point| BoxPoint::new(p.x, p.y).ok();
    // Gradient space as a unit box, so its coordinates are the fractions.
    let geometry = match gradient.shape {
        SvgGradientShape::Linear { start, end } => GradientGeometry::Linear {
            start: point(start)?,
            end: point(end)?,
        },
        SvgGradientShape::Radial { centre, radius } => GradientGeometry::Radial {
            centre: point(centre)?,
            radius: point(centre + kurbo::Vec2::new(radius, 0.0))?,
            ratio: AxisRatio::CIRCLE,
        },
    };
    let source = GradientFill {
        geometry,
        stops: model_stops(gradient)?,
    };
    let unit = GradientBox::axis_aligned(Point::new(0.0, 0.0), Point::new(1.0, 1.0));
    let map = Affine::from_array((placement * gradient.transform).as_coeffs());
    Some(source.mapped(&unit, map, frame))
}

/// `style` with the imported gradient as its fill paint, when the leaf has one and a box.
pub(crate) fn apply_gradient(
    style: &mut Style,
    gradient: Option<&SvgGradient>,
    placement: kurbo::Affine,
    frame: Option<GradientBox>,
) {
    let (Some(gradient), Some(frame)) = (gradient, frame) else {
        return;
    };
    if let Some(fill) = model_gradient(gradient, placement, &frame) {
        style.fill.paint = FillPaint::Gradient;
        style.fill.gradient = Some(fill);
    }
}

/// The import notice's lines for what the file lost, in the order of criterion 27; empty when
/// nothing was lost.
#[must_use]
pub fn gradient_loss_lines(losses: GradientLosses) -> Vec<String> {
    let line = |n: usize, one: &str, many: &str| match n {
        0 => None,
        1 => Some(one.to_owned()),
        n => Some(many.replace("<n>", &n.to_string())),
    };
    [
        line(
            losses.repeats,
            "1 gradient repeat was shown as a plain shade.",
            "<n> gradient repeats were shown as plain shades.",
        ),
        line(
            losses.focal_points,
            "1 focal point was read as the centre.",
            "<n> focal points were read as the centre.",
        ),
        line(
            losses.meshes,
            "1 mesh gradient was left out.",
            "<n> mesh gradients were left out.",
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_notice_names_only_what_was_lost() {
        let lines = gradient_loss_lines(GradientLosses {
            repeats: 1,
            focal_points: 2,
            meshes: 0,
        });
        assert_eq!(
            lines,
            [
                "1 gradient repeat was shown as a plain shade.",
                "2 focal points were read as the centre."
            ]
        );
        assert_eq!(
            gradient_loss_lines(GradientLosses::default()),
            Vec::<String>::new()
        );
    }
}
