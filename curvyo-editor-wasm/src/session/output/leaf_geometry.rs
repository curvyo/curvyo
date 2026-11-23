//! The geometry of one output leaf as it stands on its laser page (`specs/0029` criteria 13, 17;
//! `specs/0123` criteria 3, 4, 23, 26; ADR 0064 §1, ADR 0068 §1): its outlines, image frame, or
//! outlines with their placed gradient, mapped into page mm, Y down from the page's corner.

use curvyo_document_core::{
    Affine, GradientBox, ObjectSnapshot, PathSnapshot, PlacedGradient, Point, Vec2,
};
use curvyo_geometry_core::OutlineTriple;
use curvyo_laser_core::adjust::AdjustRegisters;
use curvyo_laser_core::job::{LeafGeometry, LeafOutline};

/// The geometry of `object` mapped by `map` (document space to page mm); an image's working
/// image is left out (the job input adds it for an Engrave, `input.rs`). `None` for an object
/// with no outline (a dimension, a text not laid out yet).
pub(super) fn leaf_geometry(object: &ObjectSnapshot, map: Affine) -> Option<LeafGeometry> {
    let outlines = match object {
        ObjectSnapshot::Image(image) => {
            return Some(LeafGeometry::Image {
                frame: image.corners().map(|p| map.apply_point(p)),
                asset: image.asset.clone(),
                pixels: None,
                adjust: AdjustRegisters::read(&image.adjustments),
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
        // Neither a dimension nor a registration mark ever burns (ADR 0027, ADR 0081 §3).
        ObjectSnapshot::Dimension(_) | ObjectSnapshot::RegistrationMark(_) => return None,
        _ => path_outlines(object.drawn_path()?, map),
    };
    Some(match placed_gradient(object, map) {
        Some(gradient) => LeafGeometry::Gradient { outlines, gradient },
        None => LeafGeometry::Outlines(outlines),
    })
}

/// The gradient `object`'s fill paints, placed in its box mapped by `map` (`specs/0123`
/// criterion 26: the gradient moves with the shape); `None` without a gradient fill.
fn placed_gradient(object: &ObjectSnapshot, map: Affine) -> Option<PlacedGradient> {
    let fill = object.style().fill.painted_gradient()?;
    let frame = object.gradient_box()?;
    Some(PlacedGradient::new(
        fill,
        &GradientBox {
            min: frame.min,
            max: frame.max,
            to_document: map.compose(frame.to_document),
        },
    ))
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
