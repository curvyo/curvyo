//! The write of Weld (`specs/0068-laserable-lettering` criteria 17 and 20): the result takes the
//! place of a selected group and the style the caller gives, in one commit; a place that is not
//! made of operands is refused before anything is written.

#![allow(clippy::unwrap_used, clippy::cast_precision_loss, missing_docs)]

use curvyo_document_core::{
    AnchorId, Document, Length, NewAnchor, NodeId, ObjectEditError, Point, RectBounds, Style,
};

fn rect(document: &Document, x: f64) -> NodeId {
    document.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    })
}

fn square() -> Vec<(Vec<NewAnchor>, bool)> {
    let anchors = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        .into_iter()
        .enumerate()
        .map(|(k, (x, y))| NewAnchor::corner(AnchorId::new(9, k as u64), Point::new(x, y)))
        .collect();
    vec![(anchors, true)]
}

#[test]
fn ac17_the_result_takes_the_place_of_a_whole_group_and_the_given_style() {
    let document = Document::new(1);
    let below = rect(&document, -20.0);
    let a = rect(&document, 0.0);
    let b = rect(&document, 5.0);
    let group = document.group_objects(&[a, b]).unwrap();
    let above = rect(&document, 20.0);
    let mut style = Style::default();
    style.stroke.width = Length::from_mm(2.0);

    let not_operands = document.replace_with_path_at(&[a], &style, group, &square(), "weld");
    assert_eq!(not_operands, Err(ObjectEditError::BaseNotAnOperand));
    assert_eq!(
        document.children(None),
        vec![below, group, above],
        "nothing written"
    );

    let result = document
        .replace_with_path_at(&[a, b], &style, group, &square(), "weld")
        .unwrap();
    assert_eq!(document.children(None), vec![below, result, above]);
    assert!(!document.contains_node(group), "the emptied group is gone");
    assert_eq!(document.object(result).unwrap().style(), &style);
}
