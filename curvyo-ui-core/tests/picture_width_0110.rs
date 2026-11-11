//! A linked clone of a group draws, measures and exports a profiled member by its variable-width
//! outline, as the member itself is (`specs/0110-pen-modes-variable-stroke` criterion 6, ADR 0053
//! "Width points are geometry", ADR 0045 §3).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::assert_is_empty)]

use curvyo_document_core::{
    AnchorId, Document, NewAnchor, ObjectSnapshot, PathSnapshot, Point, Vec2,
};
use curvyo_ui_core::{
    object_bounds_with_artwork, oriented_bounds_with_artwork, profiled_paths,
    verified_variable_stroke_artwork,
};

/// A horizontal line (10, 20)-(50, 20), 1 mm wide, four times wider in its middle; in a group.
fn grouped_profile(doc: &Document) -> (curvyo_document_core::NodeId, curvyo_document_core::NodeId) {
    let id = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(7, 1), Point::new(10.0, 20.0)),
            NewAnchor::corner(AnchorId::new(7, 2), Point::new(50.0, 20.0)),
        ],
        false,
    );
    doc.edit_style(
        &[id],
        &curvyo_document_core::StyleEdit::StrokeWidth(curvyo_document_core::Length::from_mm(1.0)),
    )
    .unwrap();
    doc.set_width_point(id, AnchorId::new(7, 3), 0.5, 4.0)
        .unwrap();
    let other = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(7, 4), Point::new(10.0, 30.0)),
            NewAnchor::corner(AnchorId::new(7, 5), Point::new(50.0, 30.0)),
        ],
        false,
    );
    let group = doc.group_objects(&[id, other]).unwrap();
    (group, id)
}

fn artwork(path: &PathSnapshot) -> Option<std::sync::Arc<(PathSnapshot, Option<PathSnapshot>)>> {
    verified_variable_stroke_artwork(path)
        .ok()
        .map(std::sync::Arc::new)
}

fn group_clone(doc: &Document) -> ObjectSnapshot {
    let (group, _) = grouped_profile(doc);
    let clone = doc
        .place_linked_clones(&[group], Vec2::new(0.0, 100.0))
        .unwrap()[0];
    doc.object(clone).unwrap()
}

#[test]
fn a_group_clone_names_its_profiled_member() {
    let doc = Document::new(7);
    let clone = group_clone(&doc);
    assert!(clone.picture().is_some(), "setup: a picture clone");
    let paths = profiled_paths(&clone);
    assert_eq!(paths.len(), 1, "the profiled member, not the plain one");
    assert!(!paths[0].width_points.is_empty());
}

#[test]
fn a_group_clone_is_measured_by_the_width_outline() {
    let doc = Document::new(7);
    let clone = group_clone(&doc);
    let (min, max) = object_bounds_with_artwork(&clone, &artwork).unwrap();
    // The middle is 4 mm wide: the outline reaches 2 mm above the line at y = 120.
    assert!(min.y < 118.5, "the widened middle counts ({min:?})");
    assert!(
        max.y >= 130.0 - 1e-9,
        "the plain member counts too ({max:?})"
    );
    let oriented = oriented_bounds_with_artwork(&clone, &artwork).unwrap();
    let corners = oriented.document_corners();
    let top = corners.iter().map(|c| c.y).fold(f64::INFINITY, f64::min);
    let left = corners.iter().map(|c| c.x).fold(f64::INFINITY, f64::min);
    assert!(top < 118.5, "the oriented box holds the outline ({top})");
    assert!(
        (left - 10.0).abs() < 1.0,
        "and starts at the members ({left})"
    );
}

#[test]
fn a_plain_object_has_no_profiled_paths() {
    let doc = Document::new(7);
    let id = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(7, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(7, 2), Point::new(5.0, 0.0)),
        ],
        false,
    );
    assert!(profiled_paths(&doc.object(id).unwrap()).is_empty());
}
