//! What the Select bar's ellipse group and "Curve" field show
//! (`specs/0021-ellipse-arcs-and-shaping/` criteria 3, 26, 31, 44, 52, 65 and 66): always
//! complete while they apply, greyed with a reason where they would do nothing.

use curvyo_document_core::{
    ArcKind, Curve, NodeId, ObjectSnapshot, PrimitiveSnapshot, Shape, effective_curve,
};

use crate::object_selection::{ObjectSelection, objects_with_ids};
use crate::param_edit::{PARAM_EQUAL_EPSILON, ParamValue};
use crate::param_entry::shown_arc_angles;
use crate::select_bar::{BarPreview, BarValue, ObjectKind, ids_of_kind};
use crate::shape_arc_edit::{arc_capable, curve_capable};

/// Why a control of this group is greyed; the tooltip's second line names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GreyReason {
    /// Every selected ellipse has a curve other than 100 % (criterion 31).
    CurveNotFull,
    /// No selected ellipse has an arc: the kind and "Whole ellipse" have nothing to change.
    NoArc,
    /// Every selected ellipse, polygon or star that could take a curve is an ellipse with
    /// an arc (criterion 26).
    HasArc,
}

/// The ellipse group: shown while the selection holds an ellipse (criterion 3).
#[derive(Debug, Clone, PartialEq)]
pub struct EllipseBarState {
    /// The arc kind of the selected arcs; `None` while no selected ellipse has one.
    pub kind: Option<BarValue<ArcKind>>,
    /// Why the kind choice and "Whole ellipse" are greyed, `None` when they apply.
    pub kind_grey: Option<GreyReason>,
    /// "Start" in degrees, `[0, 360)`; a whole ellipse shows the resting 45°.
    pub start: Option<BarValue<f64>>,
    /// "End" in degrees.
    pub end: Option<BarValue<f64>>,
    /// Why "Start" and "End" are greyed, `None` when they apply.
    pub angles_grey: Option<GreyReason>,
}

/// The "Curve" field: shown while the selection holds only ellipses, polygons and stars
/// (criteria 26, 44 and 52).
#[derive(Debug, Clone, PartialEq)]
pub struct CurveBarState {
    /// The effective curve in percent of the objects it acts on, `None` while greyed.
    pub value: Option<BarValue<f64>>,
    /// For a uniform value: the stored percent and the limit it is held to, when a
    /// polygon's or star's stored value lies below its limit (the "limited" tag).
    pub limited: Option<(f64, f64)>,
    /// Why the field is greyed, `None` when it applies.
    pub grey: Option<GreyReason>,
}

fn shapes<'a>(objects: &'a [ObjectSnapshot], ids: &'a [NodeId]) -> Vec<&'a Shape> {
    objects_with_ids(objects, ids)
        .into_iter()
        .filter_map(|object| match object {
            ObjectSnapshot::Primitive(PrimitiveSnapshot { shape, .. }) => Some(shape),
            ObjectSnapshot::Path(_) | ObjectSnapshot::Clone(_) | ObjectSnapshot::Text(_) => None,
        })
        .collect()
}

fn uniform<T: Copy>(values: &[T], same: impl Fn(T, T) -> bool) -> Option<BarValue<T>> {
    let (first, rest) = values.split_first()?;
    Some(if rest.iter().all(|v| same(*first, *v)) {
        BarValue::Uniform(*first)
    } else {
        BarValue::Mixed
    })
}

const fn same_degrees(a: f64, b: f64) -> bool {
    (a - b).abs() <= PARAM_EQUAL_EPSILON
}

/// The ellipse group's state, `None` when the selection holds no ellipse.
#[must_use]
pub fn ellipse_bar_state(
    objects: &[ObjectSnapshot],
    selection: &ObjectSelection,
) -> Option<EllipseBarState> {
    let all = ids_of_kind(objects, selection, ObjectKind::Primitive);
    if !shapes(objects, &all)
        .iter()
        .any(|shape| matches!(shape, Shape::Ellipse { .. }))
    {
        return None;
    }
    let acted = ids_of_kind(objects, selection, ObjectKind::ArcEllipse);
    let acted_shapes = shapes(objects, &acted);
    let angles: Vec<(f64, f64)> = acted_shapes
        .iter()
        .filter_map(|shape| shown_arc_angles(shape))
        .collect();
    let kinds: Vec<ArcKind> = acted_shapes
        .iter()
        .filter_map(|shape| arc_capable(shape).flatten().map(|arc| arc.kind))
        .collect();
    let angles_grey = acted.is_empty().then_some(GreyReason::CurveNotFull);
    let kind_grey = angles_grey.or_else(|| kinds.is_empty().then_some(GreyReason::NoArc));
    Some(EllipseBarState {
        kind: uniform(&kinds, |a, b| a == b),
        kind_grey,
        start: uniform(
            &angles.iter().map(|a| a.0).collect::<Vec<_>>(),
            same_degrees,
        ),
        end: uniform(
            &angles.iter().map(|a| a.1).collect::<Vec<_>>(),
            same_degrees,
        ),
        angles_grey,
    })
}

/// The "Curve" field's state, `None` when the selection is empty or holds anything but
/// ellipses, polygons and stars. A pending bar edit `preview` shows as committed.
#[must_use]
pub fn curve_bar_state(
    objects: &[ObjectSnapshot],
    selection: &ObjectSelection,
    preview: Option<&BarPreview>,
) -> Option<CurveBarState> {
    let held = objects_with_ids(objects, selection.ids());
    let shaped = |object: &&ObjectSnapshot| {
        matches!(
            object,
            ObjectSnapshot::Primitive(PrimitiveSnapshot {
                shape: Shape::Ellipse { .. } | Shape::Polygon { .. } | Shape::Star { .. },
                ..
            })
        )
    };
    if held.is_empty() || !held.iter().all(shaped) {
        return None;
    }
    let acted = ids_of_kind(objects, selection, ObjectKind::Curved);
    let acted_shapes: Vec<&Shape> = shapes(objects, &acted)
        .into_iter()
        .filter(|shape| curve_capable(shape))
        .collect();
    if acted_shapes.is_empty() {
        return Some(CurveBarState {
            value: None,
            limited: None,
            grey: Some(GreyReason::HasArc),
        });
    }
    let effective: Vec<f64> = acted_shapes
        .iter()
        .filter_map(|shape| effective_curve(shape).map(Curve::percent))
        .collect();
    let mut value = uniform(&effective, same_degrees);
    if let (Some(preview), Some(_)) = (preview, value)
        && let ParamValue::Curve(fraction) = preview.value
    {
        value = Some(BarValue::Uniform(fraction * 100.0));
    }
    let limited = match value {
        Some(BarValue::Uniform(_)) => acted_shapes.iter().find_map(|shape| {
            let stored = match shape {
                Shape::Ellipse { curve, .. }
                | Shape::Polygon { curve, .. }
                | Shape::Star { curve, .. } => *curve,
                Shape::Rect { .. } => return None,
            };
            let limit = effective_curve(shape)?;
            ((stored.get() - limit.get()).abs() > PARAM_EQUAL_EPSILON)
                .then(|| (stored.percent(), limit.percent()))
        }),
        _ => None,
    };
    Some(CurveBarState {
        value,
        limited,
        grey: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{
        Angle, Document, EllipseArc, EllipseFrame, InnerRatio, Length, Point, PointCount,
        RectBounds, StarFrame,
    };

    fn circle(d: &Document) -> NodeId {
        d.create_ellipse(EllipseFrame {
            center: Point::new(0.0, 0.0),
            rx: Length::from_mm(20.0),
            ry: Length::from_mm(20.0),
        })
    }

    fn select(d: &Document, ids: &[NodeId]) -> (Vec<ObjectSnapshot>, ObjectSelection) {
        let mut selection = ObjectSelection::default();
        for id in ids {
            selection.add(*id);
        }
        (d.read_objects().objects.to_vec(), selection)
    }

    fn deg(v: f64) -> Angle {
        Angle::from_radians(v.to_radians())
    }

    /// Criterion 3: a whole ellipse shows the resting angles and a greyed kind; an arc
    /// shows its own.
    #[test]
    fn the_ellipse_group_on_whole_and_arc() {
        let d = Document::new(1);
        let a = circle(&d);
        let (objects, selection) = select(&d, &[a]);
        let state = ellipse_bar_state(&objects, &selection).unwrap();
        assert_eq!(state.kind, None);
        assert_eq!(state.kind_grey, Some(GreyReason::NoArc));
        assert_eq!(state.start, Some(BarValue::Uniform(45.0)));
        assert_eq!(state.angles_grey, None);
        d.set_ellipse_arcs(&[(a, EllipseArc::new(deg(30.0), deg(150.0), ArcKind::Pie))])
            .unwrap();
        let (objects, selection) = select(&d, &[a]);
        let state = ellipse_bar_state(&objects, &selection).unwrap();
        assert_eq!(state.kind, Some(BarValue::Uniform(ArcKind::Pie)));
        assert_eq!(state.kind_grey, None);
        let BarValue::Uniform(end) = state.end.unwrap() else {
            panic!("uniform")
        };
        assert!((end - 150.0).abs() < 1e-9);
    }

    /// Criteria 26, 31 and 52: the curve field greys for an arc, the arc controls for a
    /// curve, and a rectangle hides the curve field.
    #[test]
    fn the_greyed_rules() {
        let d = Document::new(1);
        let a = circle(&d);
        d.set_curves(&[(a, Curve::new(0.0).unwrap())]).unwrap();
        let (objects, selection) = select(&d, &[a]);
        let state = ellipse_bar_state(&objects, &selection).unwrap();
        assert_eq!(state.angles_grey, Some(GreyReason::CurveNotFull));
        assert_eq!(state.kind_grey, Some(GreyReason::CurveNotFull));
        let curve = curve_bar_state(&objects, &selection, None).unwrap();
        assert_eq!(curve.value, Some(BarValue::Uniform(0.0)));
        let b = circle(&d);
        d.set_ellipse_arcs(&[(b, EllipseArc::new(deg(0.0), deg(90.0), ArcKind::Arc))])
            .unwrap();
        let (objects, selection) = select(&d, &[b]);
        let curve = curve_bar_state(&objects, &selection, None).unwrap();
        assert_eq!(curve.grey, Some(GreyReason::HasArc));
        let r = d.create_rect(RectBounds {
            origin: Point::new(0.0, 0.0),
            width: Length::from_mm(5.0),
            height: Length::from_mm(5.0),
        });
        let (objects, selection) = select(&d, &[a, r]);
        assert!(curve_bar_state(&objects, &selection, None).is_none());
    }

    /// Criteria 38 and 44: the effective value, Mixed, and the "limited" tag.
    #[test]
    fn the_curve_field_shows_the_effective_value() {
        let d = Document::new(1);
        let star = d.create_star(
            StarFrame {
                center: Point::new(0.0, 0.0),
                radius: Length::from_mm(20.0),
                angle: Angle::from_radians(0.0),
            },
            PointCount::new(5).unwrap(),
            InnerRatio::new(0.2).unwrap(),
        );
        d.set_curves(&[(star, Curve::new(-0.8).unwrap())]).unwrap();
        let (objects, selection) = select(&d, &[star]);
        let curve = curve_bar_state(&objects, &selection, None).unwrap();
        let Some(BarValue::Uniform(shown)) = curve.value else {
            panic!("uniform")
        };
        assert!((shown + 44.4).abs() < 0.1, "{shown}");
        let (stored, limit) = curve.limited.unwrap();
        assert!((stored + 80.0).abs() < 1e-9 && (limit - shown).abs() < 1e-9);
        let e = circle(&d);
        let (objects, selection) = select(&d, &[star, e]);
        assert_eq!(
            curve_bar_state(&objects, &selection, None).unwrap().value,
            Some(BarValue::Mixed)
        );
    }

    /// Decided 2026-10-19 (±400 %): a stored value beyond a shape's outward limit shows the
    /// limit with the "limited" tag, as one beyond the inward limit does.
    #[test]
    fn the_limited_tag_shows_above_the_outward_limit() {
        let d = Document::new(1);
        let triangle = d.create_polygon(
            StarFrame {
                center: Point::new(0.0, 0.0),
                radius: Length::from_mm(20.0),
                angle: Angle::from_radians(0.0),
            },
            PointCount::new(3).unwrap(),
        );
        d.set_curves(&[(triangle, Curve::new(4.0).unwrap())])
            .unwrap();
        let (objects, selection) = select(&d, &[triangle]);
        let curve = curve_bar_state(&objects, &selection, None).unwrap();
        assert_eq!(curve.value, Some(BarValue::Uniform(250.0)));
        assert_eq!(curve.limited, Some((400.0, 250.0)));
    }
}
