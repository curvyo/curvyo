//! Where the arc, curve and inner-radius knobs of an ellipse, polygon or star are drawn
//! (`specs/0021-ellipse-arcs-and-shaping/` criteria 1, 22 and 40, UX notes "Arc handle
//! layout" and "Knob displacement"): each at its true position, moved along its own travel
//! line off any glyph that is closer than 14 px.

use curvyo_document_core::{Point, Shape, Vec2, curve_handle_position, ellipse_point};

use crate::oriented_box::OrientedBox;
use crate::param_handles::{ParamHandle, star_inner_vertex};
use crate::shape_arc_edit::{ARC_SEAM, arc_capable, curve_capable};
use crate::transform_handle_layout::{
    ALL_EIGHT, EditHandle, HandleSpec, TransformHandleTolerances, transform_handles,
};

/// One knob before the displacement: its handle, its true position (box-local) and the
/// unit direction of its travel line there.
#[derive(Debug, Clone, Copy)]
struct Knob {
    handle: ParamHandle,
    at: Point,
    travel: Vec2,
}

fn unit(v: Vec2) -> Vec2 {
    let length = v.length();
    if length > f64::EPSILON {
        v.scaled(1.0 / length)
    } else {
        Vec2::ZERO
    }
}

/// The arc knobs of an ellipse with arc handles: on the true angles, or both resting at
/// the seam of 45°, the end knob half a pitch before it and the start knob half a pitch
/// after it along the outline.
fn arc_knobs(shape: &Shape, pitch_mm: f64) -> Vec<Knob> {
    let (Some(arc), Shape::Ellipse { frame, .. }) = (arc_capable(shape), shape) else {
        return Vec::new();
    };
    let tangent = |t: f64| Vec2::new(-frame.rx.as_mm() * t.sin(), frame.ry.as_mm() * t.cos());
    let (start, end) = arc.map_or_else(
        || {
            let seam = ARC_SEAM.as_radians();
            let step = pitch_mm / 2.0 / tangent(seam).length().max(f64::EPSILON);
            (seam + step, seam - step)
        },
        |arc| (arc.start().as_radians(), arc.end().as_radians()),
    );
    [(ParamHandle::ArcStart, start), (ParamHandle::ArcEnd, end)]
        .into_iter()
        .map(|(handle, t)| Knob {
            handle,
            at: ellipse_point(*frame, t),
            travel: unit(tangent(t)),
        })
        .collect()
}

/// The knobs `shape` has, in its box-local frame.
fn knobs(shape: &Shape, pitch_mm: f64) -> Vec<Knob> {
    let centre = curvyo_document_core::shape_center(shape);
    let mut knobs = Vec::new();
    if let Shape::Star {
        frame,
        point_count,
        inner_ratio,
        ..
    } = *shape
    {
        let at = star_inner_vertex(frame, point_count, inner_ratio);
        knobs.push(Knob {
            handle: ParamHandle::InnerRadius,
            at,
            travel: unit(centre.vector_to(at)),
        });
    }
    knobs.extend(arc_knobs(shape, pitch_mm));
    if curve_capable(shape)
        && let Some(at) = curve_handle_position(shape)
    {
        let middle = curve_side_middle(shape).unwrap_or(centre);
        knobs.push(Knob {
            handle: ParamHandle::Curve,
            at,
            travel: unit(centre.vector_to(middle)),
        });
    }
    knobs
}

/// The middle of the curve knob's straight side: its travel line is the bisector through
/// it and the centre.
fn curve_side_middle(shape: &Shape) -> Option<Point> {
    let straight = match *shape {
        Shape::Ellipse { frame, .. } => Shape::Ellipse {
            frame,
            arc: None,
            curve: curvyo_document_core::Curve::POLYGON_DEFAULT,
        },
        Shape::Polygon {
            frame, point_count, ..
        } => Shape::Polygon {
            frame,
            point_count,
            curve: curvyo_document_core::Curve::POLYGON_DEFAULT,
        },
        Shape::Star {
            frame,
            point_count,
            inner_ratio,
            ..
        } => Shape::Star {
            frame,
            point_count,
            inner_ratio,
            curve: curvyo_document_core::Curve::POLYGON_DEFAULT,
        },
        Shape::Rect { .. } => return None,
    };
    curve_handle_position(&straight)
}

/// `knob` moved by `distance` along its travel line (an arc knob along the outline, by
/// the same arc length).
fn moved(shape: &Shape, knob: Knob, distance: f64) -> Point {
    if let (ParamHandle::ArcStart | ParamHandle::ArcEnd, Shape::Ellipse { frame, .. }) =
        (knob.handle, shape)
    {
        let offset = frame.center.vector_to(knob.at);
        let (rx, ry) = (frame.rx.as_mm(), frame.ry.as_mm());
        if rx > f64::EPSILON && ry > f64::EPSILON {
            let t = (offset.y / ry).atan2(offset.x / rx);
            let speed = Vec2::new(-rx * t.sin(), ry * t.cos()).length();
            if speed > f64::EPSILON {
                return ellipse_point(*frame, t + distance / speed);
            }
        }
    }
    knob.at.translated(knob.travel.scaled(distance))
}

/// The finest step of the search along a travel line, as a fraction of the pitch.
const SEARCH_STEPS: u32 = 28;

/// The drawn position of every knob of `shape` (box-local), in order: each stays at its
/// true position unless a glyph or an already placed knob is closer than the clearance; it
/// is then moved along its travel line by the shortest distance (at most one pitch, either
/// way) that clears them, and towards the centre if neither way does.
fn displaced(shape: &Shape, knobs: &[Knob], glyphs: &[Point], pitch: f64) -> Vec<Point> {
    let centre = curvyo_document_core::shape_center(shape);
    // A transform glyph is wider than a knob: the knob radius, the 12 px rotate glyph's
    // radius (the widest) and the 4 px gap give 15 px, 15/14 of the pitch.
    let glyph_clearance = pitch * 15.0 / 14.0;
    let mut placed: Vec<Point> = Vec::with_capacity(knobs.len());
    for knob in knobs {
        // The inner-radius knob keeps the place it always had; the others clear it.
        if knob.handle == ParamHandle::InnerRadius {
            placed.push(knob.at);
            continue;
        }
        let clear = |p: Point| {
            glyphs
                .iter()
                .all(|g| g.vector_to(p).length() >= glyph_clearance - 1e-9)
                && placed
                    .iter()
                    .all(|k| k.vector_to(p).length() >= pitch - 1e-9)
        };
        // Up to two pitches either way: a knob exactly on a glyph needs more than one.
        let found = (0..=2 * SEARCH_STEPS)
            .flat_map(|step| {
                let distance = pitch * f64::from(step) / f64::from(SEARCH_STEPS);
                [distance, -distance]
            })
            .map(|distance| moved(shape, *knob, distance))
            .find(|p| clear(*p));
        let inward = unit(knob.at.vector_to(centre));
        let at = found
            .or_else(|| {
                (1..=2 * SEARCH_STEPS)
                    .map(|step| {
                        knob.at.translated(
                            inward.scaled(pitch * f64::from(step) / f64::from(SEARCH_STEPS)),
                        )
                    })
                    .find(|p| clear(*p))
            })
            .unwrap_or(knob.at);
        placed.push(at);
    }
    placed
}

/// The arc, curve and inner-radius knobs of `shape` in document space, drawn off the
/// transform glyphs of `box_` (resize, rotate and skew handles as the Select tool draws
/// them) and off each other.
pub(crate) fn shape_knobs(
    shape: &Shape,
    box_: &OrientedBox,
    tolerances: &TransformHandleTolerances,
) -> Vec<(ParamHandle, Point)> {
    let pitch = tolerances.param_pitch_mm;
    let knobs = knobs(shape, pitch);
    let spec = HandleSpec {
        resize_directions: &ALL_EIGHT,
        side_rotate: false,
    };
    let glyphs: Vec<Point> = transform_handles(box_, spec, tolerances)
        .into_iter()
        .filter(|(handle, _)| *handle != EditHandle::Move)
        .map(|(_, at)| box_.to_local(at))
        .collect();
    displaced(shape, &knobs, &glyphs, pitch)
        .into_iter()
        .zip(&knobs)
        .map(|(at, knob)| (knob.handle, box_.to_document(at)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oriented_box::oriented_bounds;
    use curvyo_document_core::{
        Angle, ArcKind, Curve, Document, EllipseArc, EllipseFrame, Length, ObjectSnapshot,
        PrimitiveSnapshot, Style,
    };

    fn object(arc: Option<EllipseArc>, curve: f64) -> ObjectSnapshot {
        ObjectSnapshot::Primitive(PrimitiveSnapshot {
            id: Document::new(1).create_ellipse(EllipseFrame {
                center: Point::new(0.0, 0.0),
                rx: Length::from_mm(1.0),
                ry: Length::from_mm(1.0),
            }),
            shape: Shape::Ellipse {
                frame: EllipseFrame {
                    center: Point::new(0.0, 0.0),
                    rx: Length::from_mm(300.0),
                    ry: Length::from_mm(200.0),
                },
                arc,
                curve: Curve::new(curve).unwrap(),
            },
            style: Style::default(),
            rotation: Angle::from_radians(0.3),
        })
    }

    fn knobs_of(object: &ObjectSnapshot) -> Vec<(ParamHandle, Point)> {
        let ObjectSnapshot::Primitive(p) = object else {
            unreachable!()
        };
        let t = TransformHandleTolerances::at_scale(1.0);
        shape_knobs(&p.shape, &oriented_bounds(object), &t)
    }

    /// Criterion 1 and the UX notes: on a whole ellipse both arc knobs rest 14 px apart
    /// about the seam of 45°, the end knob first; the curve knob sits upper right.
    #[test]
    fn whole_ellipse_knobs_rest_at_the_seam() {
        let o = object(None, 1.0);
        let k = knobs_of(&o);
        assert_eq!(k[0].0, ParamHandle::ArcStart);
        assert_eq!(k[1].0, ParamHandle::ArcEnd);
        assert_eq!(k[2].0, ParamHandle::Curve);
        let gap = k[0].1.vector_to(k[1].1).length();
        assert!(
            (14.0..15.0).contains(&gap),
            "14 px along the outline: {gap}"
        );
        let ObjectSnapshot::Primitive(p) = &o else {
            unreachable!()
        };
        let box_ = oriented_bounds(&o);
        let local = |at: Point| box_.to_local(at);
        let seam = ellipse_point(
            match p.shape {
                Shape::Ellipse { frame, .. } => frame,
                _ => unreachable!(),
            },
            std::f64::consts::FRAC_PI_4,
        );
        assert!(local(k[0].1).vector_to(seam).length() < 7.5);
        let curve = local(k[2].1);
        assert!(curve.x > 0.0 && curve.y < 0.0, "upper right: {curve:?}");
    }

    /// Criteria 22 and 31: an arc has no curve knob, a curved ellipse no arc knobs, and
    /// arc knobs sit on their true angles when nothing is near.
    #[test]
    fn arc_and_curve_exclude_each_other() {
        let arc = EllipseArc::new(
            Angle::from_radians(0.5),
            Angle::from_radians(2.0),
            ArcKind::Pie,
        );
        let k = knobs_of(&object(arc, 1.0));
        assert_eq!(
            k.iter().map(|(h, _)| *h).collect::<Vec<_>>(),
            [ParamHandle::ArcStart, ParamHandle::ArcEnd]
        );
        let k = knobs_of(&object(None, 0.3));
        assert_eq!(
            k.iter().map(|(h, _)| *h).collect::<Vec<_>>(),
            [ParamHandle::Curve]
        );
    }

    /// The knob displacement rule: an arc knob that would sit on a resize corner is drawn
    /// off it, at least 14.66 px from its centre.
    #[test]
    fn a_knob_is_drawn_off_a_resize_glyph() {
        // An arc ending at 0°: its end lies on the box's right edge midpoint.
        let arc = EllipseArc::new(
            Angle::from_radians(-1.0),
            Angle::from_radians(0.0),
            ArcKind::Arc,
        );
        let o = object(arc, 1.0);
        let box_ = oriented_bounds(&o);
        let k = knobs_of(&o);
        for direction in ALL_EIGHT {
            let glyph = box_.to_document(
                crate::transform_handle_layout::resize_handle_local_position(&box_, direction),
            );
            for (_, at) in &k {
                assert!(glyph.vector_to(*at).length() >= 14.66, "{direction:?}");
            }
        }
    }
}
