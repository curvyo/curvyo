//! A primitive as an element of the SVG export (`specs/0024-svg-import-export` criteria 55, 57,
//! 139 and 140): `<rect>`, `<ellipse>` or `<circle>`, Inkscape stars and arcs, or a `<path>` with
//! the primitive's numbers as shadow data.

use curvyo_document_core::{
    Angle, ArcKind, CornerRadii, Curve, EllipseArc, EllipseFrame, Length, Point, RectBounds, Shape,
    drawing_outline_of, drawing_outline_of_rotated,
};
use curvyo_svg_core::{ArcData, ExportKind, PathId, Shadow, ShadowKind, StarData};
use kurbo::Affine;

use crate::svg_export_leaf::{Node, outline_data, push_outline, six_decimals_exact};

/// The most passes [`stored_angle`] takes to settle (two in practice).
const ANGLE_SETTLE_PASSES: usize = 8;

/// The radians next to `radians` that the document's degree register reads back unchanged:
/// an arc stores its angles in degrees (`0021` decision 1), so writing this value makes the
/// second export byte-identical (criterion 63).
#[allow(clippy::float_cmp)] // a fixed point of the conversion is an exact question
pub(crate) fn stored_angle(radians: f64) -> f64 {
    let mut value = radians;
    for _ in 0..ANGLE_SETTLE_PASSES {
        let back = value.to_degrees().to_radians();
        if back == value {
            break;
        }
        value = back;
    }
    value
}

/// The element and shadow values of a primitive turned by `rotation` and mapped by the
/// translation `t` (criteria 55, 139, 140).
pub(crate) fn primitive_kind(shape: &Shape, rotation: Angle, t: Affine) -> (ExportKind, Shadow) {
    match *shape {
        Shape::Rect {
            bounds,
            corner_radii,
        } => rect_kind(shape, bounds, corner_radii, rotation, t),
        Shape::Ellipse {
            frame,
            arc: Some(arc),
            ..
        } => arc_kind(shape, frame, arc, rotation, t),
        Shape::Ellipse {
            frame,
            arc: None,
            curve,
        } => ellipse_kind(shape, frame, curve, rotation, t),
        Shape::Polygon { .. } | Shape::Star { .. } => ring_kind(shape, rotation, t),
    }
}

/// The outline of `shape` turned by `rotation` as a `<path>` with the id prefix `id`.
fn rotated_path(shape: &Shape, rotation: Angle, t: Affine, id: PathId) -> ExportKind {
    let (path, node_types) = outline_data(&drawing_outline_of_rotated(shape, rotation), t);
    ExportKind::Path {
        path,
        node_types,
        id,
    }
}

fn moved(p: Point, t: Affine) -> kurbo::Point {
    let shift = t.translation();
    kurbo::Point::new(p.x + shift.x, p.y + shift.y)
}

/// A rectangle: `<rect>` with one radius, else a path with its numbers (criteria 55, 57).
#[allow(clippy::float_cmp)] // equal radii are an exact question
fn rect_kind(
    shape: &Shape,
    bounds: RectBounds,
    corner_radii: CornerRadii,
    rotation: Angle,
    t: Affine,
) -> (ExportKind, Shadow) {
    let (w, h) = (bounds.width.as_mm(), bounds.height.as_mm());
    let radii = [
        corner_radii.tl,
        corner_radii.tr,
        corner_radii.br,
        corner_radii.bl,
    ]
    .map(Length::as_mm);
    let r = radii[0];
    let uniform = radii.iter().all(|c| *c == r);
    let origin = moved(bounds.origin, t);
    let turn = rotation.as_radians();
    let frame = vec![origin.x + w / 2.0, origin.y + h / 2.0, w, h, turn];
    if uniform && r <= w.min(h) / 2.0 {
        let rect = kurbo::Rect::new(origin.x, origin.y, origin.x + w, origin.y + h);
        // A turned rectangle keeps its exact numbers, and a radius `rx` cannot say exactly
        // keeps its exact radii (criteria 62, 63).
        let exact_radius = six_decimals_exact(r);
        let shadow = Shadow {
            frame: (turn != 0.0 || !exact_radius).then_some(frame),
            radii: (!exact_radius).then_some(radii),
            ..Shadow::default()
        };
        return (
            ExportKind::Rect {
                rect,
                radius: r,
                rotation_deg: turn.to_degrees(),
            },
            shadow,
        );
    }
    let shadow = Shadow {
        kind: Some(ShadowKind::Rectangle),
        frame: Some(frame),
        radii: Some(radii),
        ..Shadow::default()
    };
    (rotated_path(shape, rotation, t, PathId::Rect), shadow)
}

/// A whole ellipse: `<ellipse>` or `<circle>` at the default Curve, else a path (criterion 55).
#[allow(clippy::float_cmp)] // the stored default curve is an exact question
fn ellipse_kind(
    shape: &Shape,
    frame: EllipseFrame,
    curve: Curve,
    rotation: Angle,
    t: Affine,
) -> (ExportKind, Shadow) {
    let c = moved(frame.center, t);
    let (rx, ry) = (frame.rx.as_mm(), frame.ry.as_mm());
    let turn = rotation.as_radians();
    if curve.get() == 1.0 {
        let shadow = Shadow {
            frame: (turn != 0.0).then(|| vec![c.x, c.y, rx, ry, turn, 1.0]),
            ..Shadow::default()
        };
        return (
            ExportKind::Ellipse {
                center: c,
                rx,
                ry,
                rotation_deg: turn.to_degrees(),
            },
            shadow,
        );
    }
    let shadow = Shadow {
        kind: Some(ShadowKind::Ellipse),
        frame: Some(vec![c.x, c.y, rx, ry, turn, curve.get()]),
        ..Shadow::default()
    };
    (rotated_path(shape, rotation, t, PathId::Ellipse), shadow)
}

/// An ellipse arc: Inkscape's arc with its drawn `d`, a pie from its centre (criterion 139).
fn arc_kind(
    shape: &Shape,
    frame: EllipseFrame,
    arc: EllipseArc,
    rotation: Angle,
    t: Affine,
) -> (ExportKind, Shadow) {
    let c = moved(frame.center, t);
    let (rx, ry) = (frame.rx.as_mm(), frame.ry.as_mm());
    let start = stored_angle(arc.start().as_radians());
    let end = stored_angle(arc.end().as_radians());
    let written_end = if end <= start {
        end + std::f64::consts::TAU
    } else {
        end
    };
    let (arc_type, open) = match arc.kind {
        ArcKind::Pie => ("slice", false),
        ArcKind::Arc => ("arc", true),
        ArcKind::Chord => ("chord", false),
    };
    let outline = drawing_outline_of(shape);
    let mut nodes: Vec<Node> = outline
        .anchors
        .iter()
        .map(|a| (a.point, a.handle_in, a.handle_out, a.kind))
        .collect();
    if arc.kind == ArcKind::Pie {
        // The outline ends at the centre; the file starts there (criterion 139).
        nodes.rotate_right(1);
    }
    let mut path = kurbo::BezPath::new();
    let mut letters = String::new();
    push_outline(&nodes, outline.closed, t, &mut path, &mut letters);
    let data = ArcData {
        center: c,
        rx,
        ry,
        start: Some(start),
        end: Some(written_end),
        arc_type: Some(arc_type.to_owned()),
        open,
    };
    let turn = rotation.as_radians();
    let shadow = Shadow {
        frame: Some(vec![c.x, c.y, rx, ry, turn, start, end]),
        ..Shadow::default()
    };
    (
        ExportKind::Arc {
            arc: data,
            path,
            rotation_deg: turn.to_degrees(),
        },
        shadow,
    )
}

/// A polygon or star: Inkscape's star at Curve 0, else a path with its numbers (criteria 55,
/// 140).
#[allow(clippy::many_single_char_names)] // the centre and count of the specification
fn ring_kind(shape: &Shape, rotation: Angle, t: Affine) -> (ExportKind, Shadow) {
    let (frame, point_count, curve, ratio) = match *shape {
        Shape::Polygon {
            frame,
            point_count,
            curve,
        } => (frame, point_count, curve, None),
        Shape::Star {
            frame,
            point_count,
            inner_ratio,
            curve,
        } => (frame, point_count, curve, Some(inner_ratio.get())),
        Shape::Rect { .. } | Shape::Ellipse { .. } => {
            return primitive_kind(shape, rotation, t);
        }
    };
    let c = moved(frame.center, t);
    let radius = frame.radius.as_mm();
    let angle = frame.angle.as_radians();
    let n = point_count.get();
    let turn = rotation.as_radians();
    let mut numbers = vec![c.x, c.y, radius, f64::from(n), angle, turn, curve.get()];
    numbers.extend(ratio);
    let id = if ratio.is_some() {
        PathId::Star
    } else {
        PathId::Polygon
    };
    if curve.get() != 0.0 {
        let shadow = Shadow {
            kind: Some(if ratio.is_some() {
                ShadowKind::Star
            } else {
                ShadowKind::Polygon
            }),
            frame: Some(numbers),
            ..Shadow::default()
        };
        return (rotated_path(shape, rotation, t, id), shadow);
    }
    let arg1 = angle + turn;
    let star = StarData {
        sides: n,
        center: c,
        r1: radius,
        r2: Some(radius * ratio.unwrap_or(1.0)),
        arg1,
        arg2: Some(arg1 + std::f64::consts::PI / f64::from(n)),
        flat: ratio.is_none(),
    };
    let (path, node_types) = outline_data(&drawing_outline_of_rotated(shape, rotation), t);
    (
        ExportKind::Star {
            star,
            path,
            node_types,
        },
        Shadow {
            frame: Some(numbers),
            ..Shadow::default()
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac63_a_stored_angle_reads_back_through_degrees() {
        for radians in [2.598_162_104_591_138_7, 0.1, 1.0, 6.2, 1e-3] {
            let value = stored_angle(radians);
            assert!((value - radians).abs() < 1e-12);
            assert_eq!(value.to_degrees().to_radians().to_bits(), value.to_bits());
        }
    }
}
