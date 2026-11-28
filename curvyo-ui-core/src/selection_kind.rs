//! The kind codes of the selected objects, by which the Select bar shows a kind group only while
//! every selected object is of its kind (`specs/0159-ui-consistency`, Frage 433 B).

use curvyo_document_core::{NodeId, ObjectSnapshot, PrimitiveSnapshot, Shape};

use crate::object_selection::{ObjectSelection, objects_with_ids};

/// The kind code of one object: `"rectangle"`, `"ellipse"`, `"polygon"`, `"star"`, `"path"`,
/// `"clone"`, `"sketch"`, `"text"`, `"image"`, `"dimension"` or `"mark"`.
#[must_use]
pub fn object_kind_code(object: &ObjectSnapshot) -> &'static str {
    match object {
        ObjectSnapshot::Primitive(PrimitiveSnapshot { shape, .. }) => match shape {
            Shape::Rect { .. } => "rectangle",
            Shape::Ellipse { .. } => "ellipse",
            Shape::Polygon { .. } => "polygon",
            Shape::Star { .. } => "star",
        },
        ObjectSnapshot::Path(_) => "path",
        ObjectSnapshot::Clone(_) => "clone",
        ObjectSnapshot::Sketch(_) => "sketch",
        ObjectSnapshot::Text(_) => "text",
        ObjectSnapshot::Image(_) => "image",
        ObjectSnapshot::Dimension(_) => "dimension",
        ObjectSnapshot::RegistrationMark(_) => "mark",
    }
}

/// The distinct kind codes of the selected objects, in selection order, `"generated"` for an
/// object `generated` says the generators made; ids the document no longer holds are dropped.
/// Empty while nothing is selected.
#[must_use]
pub fn selection_kind_codes(
    objects: &[ObjectSnapshot],
    selection: &ObjectSelection,
    generated: impl Fn(NodeId) -> bool,
) -> Vec<&'static str> {
    let mut codes: Vec<&'static str> = Vec::new();
    for object in objects_with_ids(objects, selection.ids()) {
        let code = if generated(object.id()) {
            "generated"
        } else {
            object_kind_code(object)
        };
        if !codes.contains(&code) {
            codes.push(code);
        }
    }
    codes
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{
        AnchorId, Angle, Document, Length, NewAnchor, Point, PointCount, RectBounds, StarFrame,
    };

    fn snapshots(document: &Document) -> Vec<ObjectSnapshot> {
        document
            .object_ids()
            .into_iter()
            .filter_map(|id| document.object(id))
            .collect()
    }

    #[test]
    fn the_codes_are_distinct_in_selection_order_and_empty_without_a_selection() {
        let document = Document::new(1);
        let rect = |x: f64| {
            document.create_rect(RectBounds {
                origin: Point::new(x, 0.0),
                width: Length::from_mm(5.0),
                height: Length::from_mm(5.0),
            })
        };
        let first = rect(0.0);
        let second = rect(10.0);
        let polygon = document.create_polygon(
            StarFrame {
                center: Point::new(0.0, 0.0),
                radius: Length::from_mm(10.0),
                angle: Angle::from_radians(0.0),
            },
            PointCount::new(5).unwrap(),
        );
        let path = document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
                NewAnchor::corner(AnchorId::new(1, 2), Point::new(5.0, 0.0)),
            ],
            false,
        );
        let objects = snapshots(&document);
        let mut selection = ObjectSelection::new();
        let none = |_: NodeId| false;
        assert_eq!(
            selection_kind_codes(&objects, &selection, none),
            Vec::<&str>::new()
        );
        for id in [path, first, polygon, second] {
            selection.toggle(id);
        }
        assert_eq!(
            selection_kind_codes(&objects, &selection, none),
            vec!["path", "rectangle", "polygon"]
        );
        assert_eq!(
            selection_kind_codes(&objects, &selection, |id| id == polygon),
            vec!["path", "rectangle", "generated"]
        );
        selection.set(&[first, second]);
        assert_eq!(
            selection_kind_codes(&objects, &selection, none),
            vec!["rectangle"]
        );
    }
}
