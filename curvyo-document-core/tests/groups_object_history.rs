//! The object history (`specs/0041-object-history`) next to groups (`specs/0023-groups`): the
//! Group step is a Replace step for its members, the object undo of a member inside a group, and
//! the clone lineage of a duplicated group (0041 criterion 27).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::many_single_char_names
)]

use curvyo_document_core::{CopySource, Document, Length, NodeId, Point, RectBounds, Vec2};

fn rect(document: &Document, x: f64) -> NodeId {
    document.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    })
}

fn json(document: &Document) -> Vec<u8> {
    document.export_json().unwrap()
}

/// Criterion 6 and 18 of `0041`: Group is a Replace step, so an object undo of a member whose
/// latest step is the grouping takes the whole step back: the group goes, every member returns.
#[test]
fn an_object_undo_of_a_member_takes_the_whole_group_step_back() {
    let d = Document::new(1);
    let (a, b, c) = (rect(&d, 0.0), rect(&d, 20.0), rect(&d, 40.0));
    let before = json(&d);
    let group = d.group_objects(&[a, b]).unwrap();
    let restored = d.undo_object_step(a).unwrap().expect("a step to take back");
    assert!(
        restored.whole_step,
        "grouping replaces objects: the whole step"
    );
    assert_eq!(json(&d), before);
    assert!(!d.contains_node(group));
    assert_eq!(d.children(None), vec![a, b, c]);
    // Object redo brings the grouping back.
    d.redo_object_step(a)
        .unwrap()
        .expect("a step to apply again");
    assert_eq!(d.children(None).len(), 2);
    assert_eq!(d.parent_group(a), d.parent_group(b));
    assert!(d.parent_group(a).is_some());
}

/// A change of one member (a move) inside a group is taken back for that member only, and the
/// member stays in its group.
#[test]
fn an_object_undo_of_a_move_inside_a_group_keeps_the_member_in_the_group() {
    let d = Document::new(1);
    let (a, b) = (rect(&d, 0.0), rect(&d, 20.0));
    let group = d.group_objects(&[a, b]).unwrap();
    d.translate_objects(&[a, b], Vec2::new(5.0, 0.0)).unwrap();
    let restored = d.undo_object_step(a).unwrap().unwrap();
    assert!(!restored.whole_step);
    assert_eq!(d.parent_group(a), Some(group));
    let x = |id| match d.object(id).unwrap() {
        curvyo_document_core::ObjectSnapshot::Image(_) => unreachable!("no image in this test"),
        curvyo_document_core::ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
        curvyo_document_core::ObjectSnapshot::Clone(_) => panic!("no clone here"),
        curvyo_document_core::ObjectSnapshot::Primitive(p) => match p.shape {
            curvyo_document_core::Shape::Rect { bounds, .. } => bounds.origin.x,
            _ => panic!("a rectangle"),
        },
        curvyo_document_core::ObjectSnapshot::Path(_) => panic!("a rectangle"),
    };
    assert!((x(a) - 0.0).abs() < 1e-9, "a is back");
    assert!((x(b) - 25.0).abs() < 1e-9, "b stays moved");
}

/// The object undo of a member's deletion brings it back inside its group, at its place.
#[test]
fn the_undo_of_deleting_a_member_brings_it_back_into_its_group() {
    let d = Document::new(1);
    let (a, b, c) = (rect(&d, 0.0), rect(&d, 20.0), rect(&d, 40.0));
    let group = d.group_objects(&[a, b, c]).unwrap();
    d.delete_objects(&[b]).unwrap();
    // A deleted object cannot be selected, so this is the maker's Ctrl+Z: the revival goes back
    // under the member's group at its place.
    assert_eq!(d.children(Some(group)), vec![a, c]);
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
    assert_eq!(d.children(Some(group)), vec![a, b, c]);
}

/// Criterion 27 of `0041`: a duplicate of a group makes every descendant a clone with its own
/// inherited rows, and the new group one too.
#[test]
fn a_duplicate_of_a_group_makes_clones_that_inherit_the_rows_of_their_sources() {
    let d = Document::new(1);
    let (a, b) = (rect(&d, 0.0), rect(&d, 20.0));
    let group = d.group_objects(&[a, b]).unwrap();
    d.translate_objects(&[a, b], Vec2::new(1.0, 0.0)).unwrap();
    let copy = d
        .duplicate_objects(
            &[CopySource {
                id: group,
                anchor_ids: vec![],
            }],
            Vec2::new(50.0, 0.0),
        )
        .unwrap()[0];
    assert!(d.object_inherits(copy), "the new group has inherited rows");
    for leaf in d.leaves_of(copy) {
        assert!(d.object_inherits(leaf), "every copied member inherits");
        assert!(d.object_history_len(leaf) >= 2);
    }
    // The originals do not inherit anything.
    assert!(!d.object_inherits(group));
    assert!(!d.object_inherits(a));
}
