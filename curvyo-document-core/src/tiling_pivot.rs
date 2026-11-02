//! The pivot a tiling instance holds a source change about (ADR 0030 §2, amended 2026-11-02):
//! the centre of the cell box Apply uses, the selection boxes of its sources' leaves before the
//! change.

use std::collections::HashMap;

use kurbo::ParamCurveExtrema;
use loro::{LoroTree, TreeID};

use crate::document::Document;
use crate::group_tree::group_parent;
use crate::path_model::PathSnapshot;
use crate::primitive_model::{ObjectSnapshot, shape_frame_bounds};
use crate::units::Point;

/// The pivots found so far, one per tiling group.
pub(crate) type Pivots = HashMap<TreeID, Option<Point>>;

impl Document {
    /// The pivot of the tiling instance `leaf` (a clone with a cell or member key), read once per
    /// tiling into `pivots`: `None` when no tiling group holds it or its sources draw nothing.
    pub(crate) fn tiling_pivot(
        &self,
        tree: &LoroTree,
        leaf: TreeID,
        pivots: &mut Pivots,
    ) -> Option<Point> {
        let mut node = group_parent(tree, leaf)?;
        let group = loop {
            let meta = tree.get_meta(node).ok()?;
            if crate::tiling_codec::has_recipe(&meta) {
                break node;
            }
            node = group_parent(tree, node)?;
        };
        *pivots.entry(group).or_insert_with(|| {
            let meta = tree.get_meta(group).ok()?;
            let sources = crate::tiling_codec::read_sources(&meta)?;
            self.tiling_leaves(&sources)
                .into_iter()
                .filter_map(|leaf| self.object(leaf).as_ref().and_then(box_of))
                .reduce(|(a_min, a_max), (b_min, b_max)| {
                    (
                        Point::new(a_min.x.min(b_min.x), a_min.y.min(b_min.y)),
                        Point::new(a_max.x.max(b_max.x), a_max.y.max(b_max.y)),
                    )
                })
                .map(|(min, max)| {
                    Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y))
                })
        })
    }
}

/// The selection box of `object` as `curvyo-ui-core`'s `object_bounds` draws it: a path's curve
/// extrema, a primitive's frame, an image's corners, a text's line box (its placement point
/// before it is laid out); `None` for a dimension, which a tiling skips.
fn box_of(object: &ObjectSnapshot) -> Option<(Point, Point)> {
    let corners = |points: &[Point]| points_box(points.iter().copied());
    match object {
        ObjectSnapshot::Path(path) => path_box(path),
        ObjectSnapshot::Sketch(sketch) => path_box(&sketch.resolved),
        ObjectSnapshot::Clone(clone) => path_box(&clone.resolved),
        ObjectSnapshot::Primitive(primitive) => Some(shape_frame_bounds(&primitive.shape)),
        ObjectSnapshot::Image(image) => corners(&image.corners()),
        ObjectSnapshot::Text(text) => match &text.outline {
            Some(outline) => corners(&outline.line_box),
            None => corners(&[Point::new(text.placement.e, text.placement.f)]),
        },
        ObjectSnapshot::Dimension(_) => None,
    }
}

/// The union of the curve extrema of every segment of `path`.
fn path_box(path: &PathSnapshot) -> Option<(Point, Point)> {
    let kurbo_point = |p: Point| kurbo::Point::new(p.x, p.y);
    let mut boxes = Vec::new();
    for subpath in path.subpaths() {
        let anchors = subpath.anchors;
        let count = anchors.len();
        let segments = if subpath.closed {
            count
        } else {
            count.saturating_sub(1)
        };
        boxes.extend(anchors.iter().map(|a| (a.point, a.point)).take(1));
        for i in 0..segments {
            let (start, end) = (&anchors[i], &anchors[(i + 1) % count]);
            let cubic = kurbo::CubicBez::new(
                kurbo_point(start.point),
                kurbo_point(start.point.translated(start.handle_out)),
                kurbo_point(end.point.translated(end.handle_in)),
                kurbo_point(end.point),
            );
            let bbox = cubic.bounding_box();
            boxes.push((Point::new(bbox.x0, bbox.y0), Point::new(bbox.x1, bbox.y1)));
        }
    }
    points_box(boxes.into_iter().flat_map(|(min, max)| [min, max]))
}

fn points_box(mut points: impl Iterator<Item = Point>) -> Option<(Point, Point)> {
    let first = points.next()?;
    Some(points.fold((first, first), |(min, max), p| {
        (
            Point::new(min.x.min(p.x), min.y.min(p.y)),
            Point::new(max.x.max(p.x), max.y.max(p.y)),
        )
    }))
}
