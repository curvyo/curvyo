//! The geometry of one output leaf as it stands on its laser page (`specs/0029` criteria 13, 17;
//! ADR 0064 §1): its outlines or image frame mapped into page mm, Y down from the page's corner.

use curvyo_document_core::{Affine, ObjectSnapshot, PathSnapshot, Point, Vec2};
use curvyo_geometry_core::OutlineTriple;
use curvyo_laser_core::job::{LeafGeometry, LeafOutline};

/// The geometry of `object` mapped by `map` (document space to page mm); `None` for an object
/// with no outline (a dimension, a text not laid out yet, a clone of a picture).
pub(super) fn leaf_geometry(object: &ObjectSnapshot, map: Affine) -> Option<LeafGeometry> {
    let outlines = match object {
        ObjectSnapshot::Image(image) => {
            return Some(LeafGeometry::Image {
                frame: image.corners().map(|p| map.apply_point(p)),
            });
        }
        ObjectSnapshot::Primitive(primitive) => {
            let drawn = curvyo_document_core::drawing_outline_of_rotated(
                &primitive.shape,
                primitive.rotation,
            );
            vec![LeafOutline {
                anchors: drawn
                    .anchors
                    .iter()
                    .map(|a| mapped(map, (a.point, a.handle_in, a.handle_out)))
                    .collect(),
                closed: drawn.closed,
            }]
        }
        ObjectSnapshot::Dimension(_) => return None,
        _ => path_outlines(object.drawn_path()?, map),
    };
    Some(LeafGeometry::Outlines(outlines))
}

/// The outlines of `path` mapped by `map`.
fn path_outlines(path: &PathSnapshot, map: Affine) -> Vec<LeafOutline> {
    path.subpaths()
        .map(|sub| LeafOutline {
            anchors: sub
                .anchors
                .iter()
                .map(|a| mapped(map, (a.point, a.handle_in, a.handle_out)))
                .collect(),
            closed: sub.closed,
        })
        .collect()
}

/// One anchor mapped: the point by the whole map, the handles (relative to it) by its linear
/// part.
fn mapped(map: Affine, (point, handle_in, handle_out): OutlineTriple) -> OutlineTriple {
    (
        map.apply_point(point),
        map.apply_vector(handle_in),
        map.apply_vector(handle_out),
    )
}

/// The closed outline through `corners`, straight segments.
pub(super) fn polygon(corners: &[Point], closed: bool) -> LeafOutline {
    LeafOutline {
        anchors: corners
            .iter()
            .map(|&p| (p, Vec2::ZERO, Vec2::ZERO))
            .collect(),
        closed,
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{Document, RectBounds};

    use super::*;

    #[test]
    fn a_rectangle_is_one_closed_outline_moved_into_page_mm() {
        let d = Document::new(1);
        let id = d.create_rect(RectBounds::from_corners(
            Point::new(110.0, 20.0),
            Point::new(130.0, 40.0),
        ));
        let object = d.object(id).unwrap();
        let map = Affine::translation(Vec2::new(-100.0, 0.0));
        let Some(LeafGeometry::Outlines(outlines)) = leaf_geometry(&object, map) else {
            panic!("outlines")
        };
        assert_eq!(outlines.len(), 1);
        assert!(outlines[0].closed);
        let xs: Vec<f64> = outlines[0].anchors.iter().map(|a| a.0.x).collect();
        assert!(xs.iter().all(|x| (10.0..=30.0).contains(x)), "{xs:?}");
    }
}
