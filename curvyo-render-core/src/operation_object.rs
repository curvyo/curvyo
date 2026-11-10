//! The look of one drawn object under a laser operation (`specs/0101-laser-operations` criteria
//! 14, 18, 23): a copy of the object whose style is the look's, whose outline is the outer contour
//! for Cut-out, and whose image is replaced by its frame.

use std::sync::Arc;

use curvyo_document_core::{
    AnchorId, AnchorSnapshot, Angle, FontKind, ImageSnapshot, NewAnchor, ObjectSnapshot,
    PathSnapshot, Point, Style, SubpathSnapshot, ViewTransform,
};

use crate::operation_look::{OperationLook, look_style};

/// The look's copy of `object`, `None` for an object that is no drawn outline of its own (a sketch,
/// a dimension, a clone of a group, text or image): the caller keeps it as it is. An image becomes
/// its frame, a closed outline in the look, since its pixels never run (criterion 19).
#[must_use]
pub fn restyled(
    object: &ObjectSnapshot,
    look: OperationLook,
    view: ViewTransform,
) -> Option<ObjectSnapshot> {
    let style = look_style(look, view);
    let mut copy = object.clone();
    match &mut copy {
        ObjectSnapshot::Path(path) => restyle_path(path, look, style),
        ObjectSnapshot::Primitive(primitive) => primitive.style = style,
        ObjectSnapshot::Clone(clone) if !clone.is_picture() => {
            restyle_path(&mut clone.resolved, look, style);
        }
        ObjectSnapshot::Text(text) => {
            let mut glyph_style = style.clone();
            if text.font.kind == FontKind::SingleLine {
                glyph_style.fill.enabled = false;
            }
            text.style = style;
            if let Some(outline) = text.outline.as_mut() {
                restyle_path(&mut Arc::make_mut(outline).glyphs, look, glyph_style);
            }
        }
        ObjectSnapshot::Image(image) => {
            return Some(ObjectSnapshot::Path(frame_of(image, style)));
        }
        ObjectSnapshot::Clone(_) | ObjectSnapshot::Sketch(_) | ObjectSnapshot::Dimension(_) => {
            return None;
        }
    }
    Some(copy)
}

/// Sets the look's style on `path`; Cut-out keeps only its outer outlines first.
fn restyle_path(path: &mut PathSnapshot, look: OperationLook, style: Style) {
    if look == OperationLook::OuterLine {
        *path = outer_outlines(path);
    }
    path.style = style;
}

/// The frame of an image: its unit square placed by its placement, closed (criterion 19).
fn frame_of(image: &ImageSnapshot, style: Style) -> PathSnapshot {
    let anchors = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
        .into_iter()
        .enumerate()
        .map(|(i, (x, y))| {
            let at = image.placement.apply_point(Point::new(x, y));
            NewAnchor::corner(AnchorId::new(0, i as u64), at)
        })
        .collect();
    PathSnapshot {
        id: image.id,
        closed: true,
        style,
        anchors,
        extra_subpaths: Vec::new(),
        rotation: Angle::from_radians(0.0),
    }
}

/// The outer outlines of `path`: each outline whose first anchor lies inside an even number of the
/// other outlines (a hole lies inside one, an island inside a hole inside two). Holes and inner
/// detail drop out (criterion 23).
#[must_use]
pub fn outer_outlines(path: &PathSnapshot) -> PathSnapshot {
    if !path.is_compound() {
        return path.clone();
    }
    let outlines: Vec<(bool, Vec<AnchorSnapshot>)> = path
        .subpaths()
        .map(|sub| (sub.closed, sub.anchors.to_vec()))
        .collect();
    let kept: Vec<&(bool, Vec<AnchorSnapshot>)> = outlines
        .iter()
        .enumerate()
        .filter(|(i, (_, anchors))| {
            let Some(first) = anchors.first() else {
                return false;
            };
            let inside = outlines
                .iter()
                .enumerate()
                .filter(|(j, (_, other))| j != i && contains(other, first.point))
                .count();
            inside % 2 == 0
        })
        .map(|(_, outline)| outline)
        .collect();
    let Some(((closed, anchors), rest)) = kept.split_first() else {
        return path.clone();
    };
    let mut out = path.clone();
    out.closed = *closed;
    out.anchors.clone_from(anchors);
    out.extra_subpaths = rest
        .iter()
        .map(|(closed, anchors)| SubpathSnapshot {
            closed: *closed,
            anchors: anchors.clone(),
        })
        .collect();
    out
}

/// Whether `p` lies inside the polygon of `anchors` (even-odd ray casting over the anchor points).
fn contains(anchors: &[AnchorSnapshot], p: Point) -> bool {
    let mut inside = false;
    for (i, a) in anchors.iter().enumerate() {
        let b = &anchors[(i + 1) % anchors.len()];
        let (a, b) = (a.point, b.point);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
    }
    inside
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{
        AnchorId, Document, NewAnchor, ObjectSnapshot, Point, SubpathSnapshot, ViewTransform,
    };

    use super::*;
    use crate::operation_look::OperationLook;

    fn square(from: f64, to: f64) -> Vec<NewAnchor> {
        [(from, from), (to, from), (to, to), (from, to)]
            .into_iter()
            .enumerate()
            .map(|(i, (x, y))| NewAnchor::corner(AnchorId::new(9, i as u64), Point::new(x, y)))
            .collect()
    }

    /// A closed square path of the document, as read back, with extra outlines added by the test.
    fn path_with(extra: Vec<SubpathSnapshot>) -> ObjectSnapshot {
        let d = Document::new(1);
        let id = d.create_path(&square(0.0, 10.0), true);
        let ObjectSnapshot::Path(mut path) = d.object(id).unwrap() else {
            panic!("an anchor path is a path");
        };
        path.extra_subpaths = extra;
        ObjectSnapshot::Path(path)
    }

    fn outline_count(object: &ObjectSnapshot) -> usize {
        match object {
            ObjectSnapshot::Path(path) => path.subpaths().count(),
            _ => panic!("expected a path"),
        }
    }

    #[test]
    fn cut_out_keeps_only_the_outer_outline_of_a_ring() {
        let hole = SubpathSnapshot {
            closed: true,
            anchors: square(3.0, 7.0),
        };
        let ring = path_with(vec![hole]);
        let style = restyled(&ring, OperationLook::OuterLine, ViewTransform::identity()).unwrap();
        assert_eq!(outline_count(&style), 1);
    }

    #[test]
    fn cut_out_keeps_an_island_inside_a_hole() {
        let hole = SubpathSnapshot {
            closed: true,
            anchors: square(2.0, 8.0),
        };
        let island = SubpathSnapshot {
            closed: true,
            anchors: square(4.0, 6.0),
        };
        let object = path_with(vec![hole, island]);
        let out = restyled(&object, OperationLook::OuterLine, ViewTransform::identity()).unwrap();
        assert_eq!(outline_count(&out), 2, "the outer square and the island");
    }

    #[test]
    fn the_other_looks_keep_every_outline() {
        let hole = SubpathSnapshot {
            closed: true,
            anchors: square(3.0, 7.0),
        };
        let object = path_with(vec![hole]);
        let out = restyled(&object, OperationLook::SolidLine, ViewTransform::identity()).unwrap();
        assert_eq!(outline_count(&out), 2);
    }

    #[test]
    fn a_style_only_look_changes_the_style_and_not_the_outline() {
        let object = path_with(vec![]);
        let out = restyled(&object, OperationLook::AreaFill, ViewTransform::identity()).unwrap();
        let ObjectSnapshot::Path(path) = out else {
            panic!()
        };
        assert!(
            !path.style.fill.enabled && !path.style.stroke.enabled,
            "the hatch is drawn apart"
        );
        assert_eq!(path.anchors.len(), 4);
    }

    #[test]
    fn a_text_takes_the_look_in_its_style() {
        use curvyo_document_core::{Affine, FontKind, FontRef, Length, TextLayoutValues, Vec2};
        let d = Document::new(1);
        let id = d
            .create_text(
                "A",
                &FontRef {
                    family: "Source Sans 3".into(),
                    style: "Regular".into(),
                    face_hash: "c".repeat(64),
                    face_index: 0,
                    kind: FontKind::Outline,
                },
                Length::from_mm(10.0),
                Affine::translation(Vec2::new(0.0, 0.0)),
                &TextLayoutValues::DEFAULT,
            )
            .unwrap();
        let object = d.object(id).unwrap();
        let out = restyled(&object, OperationLook::AreaFill, ViewTransform::identity()).unwrap();
        let ObjectSnapshot::Text(text) = out else {
            panic!("a text stays a text")
        };
        assert!(
            !text.style.fill.enabled,
            "a hatched look has no fill of its own"
        );
    }
}
