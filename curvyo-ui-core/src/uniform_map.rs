//! The uniform map of a set of objects about one pivot (`specs/0154-library-into-slots`
//! criterion 6, ADR 0088 decision 3): a scale with the aspect kept, then a turn, applied kind by
//! kind as a group scale and a group rotate of `0010` do, strokes and corner radii scaled.

use curvyo_document_core::{Angle, Document, NodeId, ObjectSnapshot, Point};

use crate::group_transform::{GroupMap, commit_group, map_all_checked};
use crate::transform_drag::{CornerRadiusScaling, ScaleModes, StrokeScaling};

/// The commit label of the map (it is written to a fragment's private copy, never a history step).
const UNIFORM_MAP_COMMIT_LABEL: &str = "uniform_map";

/// A scale this close to 1 and a turn this close to 0 radians are no map.
const IDENTITY_EPSILON: f64 = 1e-12;

/// Scales every leaf of `ids` in `document` by `scale` about `pivot`, then turns it by `angle`
/// about `pivot`, writing each map as one commit. `true` when everything was written (or there
/// was nothing to do); `false` when a result was not sane or the document refused it.
pub fn map_uniform(
    document: &Document,
    ids: &[NodeId],
    pivot: Point,
    scale: f64,
    angle: Angle,
) -> bool {
    let mut ok = true;
    if (scale - 1.0).abs() > IDENTITY_EPSILON {
        let map = GroupMap::Scale {
            pivot,
            sx: scale,
            sy: scale,
        };
        ok &= write_map(document, ids, &map);
    }
    if ok && angle.as_radians().abs() > IDENTITY_EPSILON {
        let map = GroupMap::Rotate {
            pivot,
            delta: angle,
        };
        ok &= write_map(document, ids, &map);
    }
    ok
}

/// Applies `map` to every leaf of `ids` and commits the results.
fn write_map(document: &Document, ids: &[NodeId], map: &GroupMap) -> bool {
    let starts: Vec<ObjectSnapshot> = ids
        .iter()
        .flat_map(|&id| {
            if document.is_group(id) {
                document.leaves_of(id)
            } else {
                vec![id]
            }
        })
        .filter_map(|leaf| document.object(leaf))
        .collect();
    if starts.is_empty() {
        return true;
    }
    let modes = ScaleModes {
        stroke: StrokeScaling::Proportional,
        radius: CornerRadiusScaling::Proportional,
        proportional: false,
    };
    let converted = vec![None; starts.len()];
    let Some(results) = map_all_checked(&starts, &converted, map, modes) else {
        return false;
    };
    commit_group(
        document,
        &results,
        StrokeScaling::Proportional,
        UNIFORM_MAP_COMMIT_LABEL,
        Some(map),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{Length, RectBounds, Shape};

    fn rect(document: &Document, x: f64, y: f64, w: f64, h: f64) -> NodeId {
        document.create_rect(RectBounds {
            origin: Point::new(x, y),
            width: Length::from_mm(w),
            height: Length::from_mm(h),
        })
    }

    fn bounds(document: &Document, id: NodeId) -> (Point, Point) {
        crate::object_bounds::object_outline_bounds(&document.object(id).unwrap())
    }

    #[test]
    fn a_half_scale_about_the_centre_keeps_the_centre_and_the_aspect() {
        let document = Document::new(1);
        let id = rect(&document, 0.0, 0.0, 60.0, 40.0);
        assert!(map_uniform(
            &document,
            &[id],
            Point::new(30.0, 20.0),
            0.5,
            Angle::from_radians(0.0)
        ));
        let (min, max) = bounds(&document, id);
        assert!((min.x - 15.0).abs() < 1e-9 && (max.x - 45.0).abs() < 1e-9);
        assert!((min.y - 10.0).abs() < 1e-9 && (max.y - 30.0).abs() < 1e-9);
    }

    #[test]
    fn a_quarter_turn_swaps_the_box_and_keeps_the_kind() {
        let document = Document::new(1);
        let a = rect(&document, 0.0, 0.0, 40.0, 10.0);
        let b = rect(&document, 0.0, 20.0, 40.0, 10.0);
        let group = document.group_objects(&[a, b]).unwrap();
        assert!(map_uniform(
            &document,
            &[group],
            Point::new(20.0, 15.0),
            1.0,
            Angle::from_radians(std::f64::consts::FRAC_PI_2)
        ));
        let (min, max) = bounds(&document, a);
        assert!(((max.x - min.x) - 10.0).abs() < 1e-9, "{min:?} {max:?}");
        assert!(((max.y - min.y) - 40.0).abs() < 1e-9);
        assert!(matches!(
            document.object(a),
            Some(ObjectSnapshot::Primitive(p)) if matches!(p.shape, Shape::Rect { .. })
        ));
    }

    #[test]
    fn the_stroke_width_scales_with_the_map() {
        let document = Document::new(1);
        let id = rect(&document, 0.0, 0.0, 60.0, 40.0);
        let before = document.object(id).unwrap().style().stroke.width;
        assert!(map_uniform(
            &document,
            &[id],
            Point::new(0.0, 0.0),
            0.5,
            Angle::from_radians(0.0)
        ));
        let after = document.object(id).unwrap().style().stroke.width;
        assert!(
            (after.as_mm() - before.as_mm() * 0.5).abs() < 1e-6,
            "{before:?} {after:?}"
        );
    }
}
