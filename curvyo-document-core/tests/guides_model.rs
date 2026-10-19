//! The guide flag in the model (`specs/0046-document-shapes` criteria 2, 9, 10, 26, 28 and 34,
//! ADR 0019 §2): which nodes can carry it, the one commit that sets it, its undo, its merge, the
//! clone rule, the output rule and the operations that make a new path from inputs.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::many_single_char_names,
    clippy::similar_names
)]

use curvyo_document_core::{
    AnchorId, CopySource, Document, EllipseFrame, GuideMarking, NewAnchor, NodeId, NodeRead,
    ObjectEditError, PieceFill, Point, RectBounds, SET_GUIDE_COMMIT_LABEL, Vec2, pack, unpack,
};

fn rect(document: &Document, x: f64) -> NodeId {
    document.create_rect(RectBounds::from_corners(
        Point::new(x, 0.0),
        Point::new(x + 10.0, 10.0),
    ))
}

fn line(document: &Document, n: u64, y: f64) -> NodeId {
    document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, n * 10), Point::new(0.0, y)),
            NewAnchor::corner(AnchorId::new(1, n * 10 + 1), Point::new(10.0, y)),
            NewAnchor::corner(AnchorId::new(1, n * 10 + 2), Point::new(20.0, y)),
        ],
        false,
    )
}

fn read_of(document: &Document, id: NodeId) -> NodeRead {
    document
        .read_nodes()
        .into_iter()
        .find(|read| read.id == id)
        .unwrap()
}

fn is_guide(document: &Document, id: NodeId) -> bool {
    read_of(document, id).guide
}

fn stored_guide(document: &Document, id: NodeId) -> bool {
    document.node_registers(id).unwrap().guide
}

fn last_label(document: &Document) -> String {
    document.own_steps().last().unwrap().label.clone()
}

fn replica(document: &Document, peer: u64) -> Document {
    unpack(peer, &pack(document, "test").unwrap()).unwrap()
}

fn sync(a: &Document, b: &Document) {
    let to_b = a.export_updates(&b.version()).unwrap();
    let to_a = b.export_updates(&a.version()).unwrap();
    b.import_updates(&to_b).unwrap();
    a.import_updates(&to_a).unwrap();
}

#[test]
fn c2_paths_and_every_shape_can_be_marked_and_unmarking_removes_the_key() {
    let d = Document::new(1);
    let path = line(&d, 1, 0.0);
    let r = rect(&d, 0.0);
    let e = d.create_ellipse(EllipseFrame::from_corners(
        Point::new(40.0, 40.0),
        Point::new(60.0, 50.0),
    ));
    let written = d.set_guide(&[path, r, e], true).unwrap();
    assert_eq!(written, vec![path, r, e]);
    assert_eq!(last_label(&d), SET_GUIDE_COMMIT_LABEL);
    for id in [path, r, e] {
        assert!(stored_guide(&d, id) && is_guide(&d, id));
    }
    let json: serde_json::Value = serde_json::from_slice(&d.export_json().unwrap()).unwrap();
    assert_eq!(json["objects"][0]["guide"], true);
    d.set_guide(&[path, r, e], false).unwrap();
    for id in [path, r, e] {
        assert!(!stored_guide(&d, id));
    }
    let json: serde_json::Value = serde_json::from_slice(&d.export_json().unwrap()).unwrap();
    assert!(
        json["objects"]
            .as_array()
            .unwrap()
            .iter()
            .all(|o| o.get("guide").is_none())
    );
}

#[test]
fn c9_a_group_gives_its_paths_and_shapes_and_a_clone_is_skipped() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let b = rect(&d, 20.0);
    let group = d.group_objects(&[a, b]).unwrap();
    let origin = line(&d, 5, 40.0);
    let clone = d.linked_clone(&[origin], &[]).unwrap()[0];
    assert_eq!(
        d.guide_marking(&[group, clone]),
        GuideMarking {
            markable: 2,
            guides: 0
        }
    );
    assert_eq!(d.set_guide(&[group, clone], true).unwrap(), vec![a, b]);
    assert!(!stored_guide(&d, group), "a group carries no flag");
    assert!(!stored_guide(&d, clone));
    assert!(!read_of(&d, group).guide);
    assert!(
        matches!(
            d.set_guide(&[clone], true),
            Err(ObjectEditError::NoSuchObject)
        ),
        "only a clone: nothing markable"
    );
    assert!(!d.guide_marking(&[clone]).enabled());
    assert!(!d.guide_marking(&[]).enabled());
}

#[test]
fn c10_the_marking_tells_pressed_and_mixed() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let b = rect(&d, 20.0);
    d.set_guide(&[a], true).unwrap();
    let mixed = d.guide_marking(&[a, b]);
    assert!(mixed.enabled() && !mixed.all_guides());
    d.set_guide(&[a, b], true).unwrap();
    assert!(d.guide_marking(&[a, b]).all_guides());
}

#[test]
fn c10_unmarking_keeps_geometry_and_style_exactly() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let before = d.object(a).unwrap();
    d.set_guide(&[a], true).unwrap();
    d.set_guide(&[a], false).unwrap();
    assert_eq!(d.object(a).unwrap(), before);
}

#[test]
fn c28_one_commit_and_one_undo_step_and_redo() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let b = rect(&d, 20.0);
    let steps = d.own_steps().len();
    d.set_guide(&[a, b], true).unwrap();
    assert_eq!(d.own_steps().len(), steps + 1);
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
    assert!(!stored_guide(&d, a) && !stored_guide(&d, b));
}

#[test]
fn c26_mark_and_move_merge_both_ways() {
    for mark_first in [true, false] {
        let d = Document::new(1);
        let a = rect(&d, 0.0);
        let other = replica(&d, 2);
        let (marker, mover) = if mark_first {
            (&d, &other)
        } else {
            (&other, &d)
        };
        marker.set_guide(&[a], true).unwrap();
        mover.translate_objects(&[a], Vec2::new(5.0, 7.0)).unwrap();
        sync(&d, &other);
        for replica in [&d, &other] {
            assert!(stored_guide(replica, a));
        }
        assert_eq!(d.object(a), other.object(a));
        let moved = d.object(a).unwrap();
        let expected = {
            let e = Document::new(1);
            let id = rect(&e, 0.0);
            e.translate_objects(&[id], Vec2::new(5.0, 7.0)).unwrap();
            e.object(id).unwrap()
        };
        assert_eq!(moved, expected);
    }
}

#[test]
fn c26_mark_and_unmark_converge_on_both_replicas() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    d.set_guide(&[a], true).unwrap();
    let other = replica(&d, 2);
    d.set_guide(&[a], false).unwrap();
    other.set_guide(&[a], true).unwrap();
    sync(&d, &other);
    assert_eq!(stored_guide(&d, a), stored_guide(&other, a));
}

#[test]
fn c34_a_clone_of_a_guide_reads_as_a_guide_and_is_not_output() {
    let d = Document::new(1);
    let origin = line(&d, 5, 0.0);
    let clone = d.linked_clone(&[origin], &[]).unwrap()[0];
    assert!(read_of(&d, clone).is_output_node());
    d.set_guide(&[origin], true).unwrap();
    assert!(is_guide(&d, clone));
    assert!(!read_of(&d, clone).is_output_node());
    assert!(!read_of(&d, origin).is_output_node());
    d.set_guide(&[origin], false).unwrap();
    assert!(!is_guide(&d, clone));
}

#[test]
fn c41_the_output_rule_leaves_out_guides_hidden_nodes_and_containers() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let b = rect(&d, 20.0);
    let c = rect(&d, 40.0);
    let e = rect(&d, 60.0);
    let group = d.group_objects(&[c, e]).unwrap();
    d.set_guide(&[a], true).unwrap();
    d.set_node_visible(&[group], false).unwrap();
    let output: Vec<NodeId> = d
        .read_nodes()
        .into_iter()
        .filter(NodeRead::is_output_node)
        .map(|read| read.id)
        .collect();
    assert_eq!(output, vec![b]);
}

#[test]
fn c34_duplicate_keeps_the_flag() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    d.set_guide(&[a], true).unwrap();
    let copy = d
        .duplicate_objects(
            &[CopySource {
                id: a,
                anchor_ids: Vec::new(),
            }],
            Vec2::new(5.0, 0.0),
        )
        .unwrap()[0];
    assert!(stored_guide(&d, copy));
}

fn outline(n: u64) -> Vec<(Vec<NewAnchor>, bool)> {
    vec![(
        vec![
            NewAnchor::corner(AnchorId::new(1, 900 + n * 10), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 901 + n * 10), Point::new(5.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 902 + n * 10), Point::new(5.0, 5.0)),
        ],
        true,
    )]
}

#[test]
fn c34_a_boolean_result_is_a_guide_only_when_every_input_was() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let b = rect(&d, 20.0);
    d.set_guide(&[a], true).unwrap();
    let mixed = d
        .replace_with_path(&[a, b], a, &outline(1), "union")
        .unwrap();
    assert!(!stored_guide(&d, mixed), "one input was ordinary");

    let c = rect(&d, 40.0);
    let e = rect(&d, 60.0);
    d.set_guide(&[c, e], true).unwrap();
    let both = d
        .replace_with_path(&[c, e], e, &outline(2), "union")
        .unwrap();
    assert!(stored_guide(&d, both));
}

#[test]
fn c34_a_piece_of_a_guide_is_a_guide() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let b = rect(&d, 20.0);
    d.set_guide(&[a], true).unwrap();
    let pieces = d
        .replace_with_pieces(
            &[a, b],
            &[(a, outline(1)), (b, outline(2))],
            PieceFill::Keep,
            "break_apart",
        )
        .unwrap();
    assert!(stored_guide(&d, pieces[0]));
    assert!(!stored_guide(&d, pieces[1]));
}

#[test]
fn c34_a_join_is_a_guide_only_when_both_paths_were() {
    for (a_guide, b_guide) in [(true, false), (false, true), (true, true)] {
        let d = Document::new(1);
        let a = line(&d, 1, 0.0);
        let b = line(&d, 2, 30.0);
        if a_guide {
            d.set_guide(&[a], true).unwrap();
        }
        if b_guide {
            d.set_guide(&[b], true).unwrap();
        }
        let (joined, _) = d
            .join_endpoints(a, AnchorId::new(1, 12), b, AnchorId::new(1, 20))
            .unwrap();
        assert_eq!(
            stored_guide(&d, joined),
            a_guide && b_guide,
            "{a_guide} {b_guide}"
        );
    }
}

#[test]
fn c34_a_split_keeps_the_flag_on_both_parts() {
    let d = Document::new(1);
    let a = line(&d, 1, 0.0);
    d.set_guide(&[a], true).unwrap();
    let (first, second) = d
        .split_at_anchor(a, AnchorId::new(1, 11), AnchorId::new(1, 99))
        .unwrap();
    assert!(stored_guide(&d, first.0));
    assert!(stored_guide(&d, second.0));
}
