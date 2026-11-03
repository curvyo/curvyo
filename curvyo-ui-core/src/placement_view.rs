//! How a placement shows its drawing page on an output page (ADR 0032 §6): which objects are
//! the page's members and the map from the drawing plane to the output page.

use std::collections::HashSet;

use curvyo_document_core::{Affine, NodeId, ObjectSnapshot, Page, Point, Vec2};

use crate::object_bounds;

/// The map from the drawing plane to the output page's local mm: the source page's top-left
/// moves to the origin, then the placement's matrix applies.
#[must_use]
pub fn placement_map(source: &Page, matrix: Affine) -> Affine {
    matrix.compose(Affine::translation(Vec2::new(
        -source.origin.x,
        -source.origin.y,
    )))
}

/// The indexes of the objects a placement of `source` draws: members of the page under ADR 0024
/// §1 (their bounds touch or overlap the frame), guides left out. Each is drawn in full.
#[must_use]
pub fn placement_members<S: std::hash::BuildHasher>(
    objects: &[ObjectSnapshot],
    source: &Page,
    guides: &HashSet<NodeId, S>,
) -> Vec<usize> {
    objects
        .iter()
        .enumerate()
        .filter(|(_, object)| !guides.contains(&object.id()))
        .filter(|(_, object)| {
            let (min, max) = object_bounds(object);
            source.overlaps(min, max)
        })
        .map(|(index, _)| index)
        .collect()
}

/// The union of the members' bounds on the output page (the corners mapped by `map`), `None` for
/// no member: what the overhang notice measures (criterion 8).
#[must_use]
pub fn placed_bounds(
    objects: &[ObjectSnapshot],
    members: &[usize],
    map: Affine,
) -> Option<(Point, Point)> {
    let mut union: Option<(Point, Point)> = None;
    for &index in members {
        let (min, max) = object_bounds(&objects[index]);
        for corner in [min, Point::new(max.x, min.y), max, Point::new(min.x, max.y)] {
            let p = map.apply_point(corner);
            union = Some(union.map_or((p, p), |(lo, hi)| {
                (
                    Point::new(lo.x.min(p.x), lo.y.min(p.y)),
                    Point::new(hi.x.max(p.x), hi.y.max(p.y)),
                )
            }));
        }
    }
    union
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{DocumentSize, PageGrid, RectBounds};

    use super::*;

    fn page(x: f64) -> Page {
        Page {
            id: "p".into(),
            name: "P".into(),
            origin: Point::new(x, 0.0),
            size: DocumentSize::from_mm(100.0, 100.0),
            grid: PageGrid::DEFAULT,
        }
    }

    fn rect(document: &curvyo_document_core::Document, x: f64, w: f64) -> NodeId {
        document.create_rect(RectBounds::from_corners(
            Point::new(x, 10.0),
            Point::new(x + w, 20.0),
        ))
    }

    #[test]
    fn members_overlap_the_page_and_guides_stay_out() {
        let document = curvyo_document_core::Document::new(1);
        let inside = rect(&document, 210.0, 10.0);
        let across = rect(&document, 290.0, 30.0);
        let _outside = rect(&document, 10.0, 10.0);
        let guide = rect(&document, 220.0, 10.0);
        let objects = document.read_objects().objects;
        let guides = HashSet::from([guide]);
        let members = placement_members(&objects, &page(200.0), &guides);
        let ids: Vec<NodeId> = members.iter().map(|&i| objects[i].id()).collect();
        assert_eq!(ids, vec![inside, across]);
        let map = placement_map(&page(200.0), Affine::IDENTITY);
        let (min, max) = placed_bounds(&objects, &members, map).unwrap();
        assert!((min.x - 10.0).abs() < 1.0, "{min:?}");
        assert!(
            (max.x - 120.0).abs() < 1.0,
            "past the right edge, drawn in full: {max:?}"
        );
    }
}
