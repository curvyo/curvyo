//! The writers with linked clones (`specs/0049-linked-clone` criteria 10, 12, 15 to 17, 19, 26 to
//! 31, 33, 35, 41, 47, 48, 51, 54 and 55; `adrs.md` decisions 4, 5, 6 and 8).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names
)]

use std::collections::HashMap;

use curvyo_document_core::{
    Affine, AnchorId, AnchorKind, CloneError, CloneLinks, CloneRepair, CloneSnapshot, Color,
    CopySource, Document, NewAnchor, NodeId, ObjectEditError, ObjectSnapshot, PathEditError,
    PathSnapshot, Point, StyleEdit, UNLINK_CLONE_COMMIT_LABEL, Vec2, clone_matrices_after,
};

/// A square of side `size` with its top-left corner at `at`.
fn square(d: &Document, n: u64, at: Point, size: f64) -> NodeId {
    let corner = |k: u64, dx: f64, dy: f64| {
        NewAnchor::corner(
            AnchorId::new(1, n * 10 + k),
            Point::new(at.x + dx, at.y + dy),
        )
    };
    d.create_path(
        &[
            corner(0, 0.0, 0.0),
            corner(1, size, 0.0),
            corner(2, size, size),
            corner(3, 0.0, size),
        ],
        true,
    )
}

fn o(d: &Document) -> NodeId {
    square(d, 1, Point::new(0.0, 0.0), 10.0)
}

fn clone_of(d: &Document, id: NodeId) -> CloneSnapshot {
    match d.object(id) {
        Some(ObjectSnapshot::Clone(clone)) => clone,
        other => panic!("not a clone: {other:?}"),
    }
}

fn path_of(d: &Document, id: NodeId) -> PathSnapshot {
    match d.object(id) {
        Some(ObjectSnapshot::Path(path)) => path,
        other => panic!("not a path: {other:?}"),
    }
}

fn link(d: &Document, original: NodeId) -> NodeId {
    d.linked_clone(&[original], &[]).unwrap()[0]
}

/// The drawn outline of any object as a path.
fn drawn(d: &Document, id: NodeId) -> PathSnapshot {
    match d.object(id).unwrap() {
        ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
        ObjectSnapshot::Path(path) => path,
        ObjectSnapshot::Clone(clone) => clone.resolved,
        ObjectSnapshot::Primitive(_) => panic!("a primitive"),
    }
}

/// `(min x, max x, min y, max y)` of an outline.
fn span(path: &PathSnapshot) -> (f64, f64, f64, f64) {
    let xs: Vec<f64> = path.all_anchors().map(|a| a.point.x).collect();
    let ys: Vec<f64> = path.all_anchors().map(|a| a.point.y).collect();
    let min = |v: &[f64]| v.iter().copied().fold(f64::INFINITY, f64::min);
    let max = |v: &[f64]| v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (min(&xs), max(&xs), min(&ys), max(&ys))
}

fn near(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    (a.0 - b.0).abs() < 1e-6
        && (a.1 - b.1).abs() < 1e-6
        && (a.2 - b.2).abs() < 1e-6
        && (a.3 - b.3).abs() < 1e-6
}

fn same_points(a: &PathSnapshot, b: &PathSnapshot, tolerance: f64) {
    let (a, b): (Vec<_>, Vec<_>) = (a.all_anchors().collect(), b.all_anchors().collect());
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        assert!((x.point.x - y.point.x).abs() < tolerance, "{x:?} {y:?}");
        assert!((x.point.y - y.point.y).abs() < tolerance, "{x:?} {y:?}");
    }
}

fn fresh(start: u64, n: usize) -> Vec<AnchorId> {
    (0..n as u64).map(|k| AnchorId::new(9, start + k)).collect()
}

fn unlink(d: &Document, ids: &[NodeId], start: u64) {
    let unlinks: Vec<CopySource> = ids
        .iter()
        .enumerate()
        .map(|(k, &id)| CopySource {
            id,
            anchor_ids: fresh(
                start + 1000 * k as u64,
                clone_of(d, id).resolved.all_anchors().count(),
            ),
        })
        .collect();
    d.unlink_clones(&unlinks).unwrap();
}

/// The result of mapping every object of `ids` by `g` as a gesture gives it: paths mapped,
/// clones with the matrices of [`clone_matrices_after`].
fn mapped_results(d: &Document, ids: &[NodeId], g: Affine) -> Vec<ObjectSnapshot> {
    let links: CloneLinks = d
        .read_objects()
        .objects
        .iter()
        .filter_map(|object| match object {
            ObjectSnapshot::Clone(c) => Some((c.id(), (c.chain.origin, c.chain.matrix))),
            _ => None,
        })
        .collect();
    let matrices: HashMap<NodeId, Affine> =
        clone_matrices_after(&links, &d.own_placement_clones(), ids, g)
            .matrices
            .into_iter()
            .collect();
    ids.iter()
        .map(|&id| match d.object(id).unwrap() {
            ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
            ObjectSnapshot::Path(path) => ObjectSnapshot::Path(path.mapped(g)),
            ObjectSnapshot::Clone(clone) => {
                let mut moved = clone.mapped_by(g);
                moved.chain.matrix = matrices[&id];
                ObjectSnapshot::Clone(moved)
            }
            ObjectSnapshot::Primitive(_) => panic!("a primitive"),
        })
        .collect()
}

fn translate(x: f64, y: f64) -> Affine {
    Affine::translation(Vec2::new(x, y))
}

/// A clone of `original` moved so its matrix is `translate (x, y)` times what it was.
fn moved_clone(d: &Document, original: NodeId, x: f64, y: f64) -> NodeId {
    let clone = link(d, original);
    d.translate_objects(&[clone], Vec2::new(x, y)).unwrap();
    clone
}

#[test]
fn ac10_an_edit_of_the_original_reaches_every_clone_through_a_chain() {
    let d = Document::new(1);
    let p = o(&d);
    let a = moved_clone(&d, p, 100.0, 0.0);
    let b = moved_clone(&d, a, 0.0, 50.0);
    let top_right = path_of(&d, p).anchors[1].id;
    d.move_anchors(&[(p, top_right, Point::new(15.0, 5.0))])
        .unwrap();
    assert_eq!(
        clone_of(&d, a).resolved.anchors[1].point,
        Point::new(115.0, 5.0)
    );
    assert_eq!(
        clone_of(&d, b).resolved.anchors[1].point,
        Point::new(115.0, 55.0)
    );
}

#[test]
fn ac12_51_a_change_of_the_original_is_seen_in_the_clone_frame() {
    let d = Document::new(1);
    let p = o(&d);
    let turned = link(&d, p);
    let quarter = ObjectSnapshot::Clone(
        clone_of(&d, turned).mapped_by(Affine::new(0.0, 1.0, -1.0, 0.0, 0.0, 0.0)),
    );
    d.rotate_object(&quarter).unwrap();
    let plain = moved_clone(&d, p, 50.0, 0.0);
    let (before_turned, before_plain) = (span(&drawn(&d, turned)), span(&drawn(&d, plain)));
    d.translate_objects(&[p], Vec2::new(5.0, 0.0)).unwrap();
    let (t, q) = (span(&drawn(&d, turned)), span(&drawn(&d, plain)));
    assert!(
        near(
            t,
            (
                before_turned.0,
                before_turned.1,
                before_turned.2 + 5.0,
                before_turned.3 + 5.0
            )
        ),
        "5 mm down"
    );
    assert!(
        near(
            q,
            (
                before_plain.0 + 5.0,
                before_plain.1 + 5.0,
                before_plain.2,
                before_plain.3
            )
        ),
        "5 mm right"
    );
    // 51 (b): A moved by a gesture, B behind it moves in its own frame, O untouched.
    let b = link(&d, turned);
    let o_before = path_of(&d, p);
    let b_before = span(&drawn(&d, b));
    d.translate_objects(&[turned], Vec2::new(5.0, 0.0)).unwrap();
    assert_eq!(path_of(&d, p), o_before);
    assert!(near(
        span(&drawn(&d, b)),
        (b_before.0 + 5.0, b_before.1 + 5.0, b_before.2, b_before.3)
    ));
}

#[test]
fn ac15_a_clone_alone_gets_g_times_m_and_the_chain_is_not_written() {
    let d = Document::new(1);
    let p = o(&d);
    let c = moved_clone(&d, p, 30.0, 0.0);
    let before = path_of(&d, p);
    let g = Affine::scale(2.0, 1.0);
    let results = mapped_results(&d, &[c], g);
    d.transform_objects(&results, false).unwrap();
    assert_eq!(path_of(&d, p), before, "nothing of the chain is written");
    assert_eq!(
        clone_of(&d, c).chain.matrix,
        g.compose(translate(30.0, 0.0))
    );
    assert!(near(span(&drawn(&d, c)), (60.0, 80.0, 0.0, 10.0)));
}

#[test]
fn ac16_a_b_c_chain_members_transformed_together() {
    // (a) The origin and its clone, scaled by (2, 1) about (0, 0).
    let d = Document::new(1);
    let p = o(&d);
    let c = moved_clone(&d, p, 30.0, 0.0);
    d.transform_objects(&mapped_results(&d, &[p, c], Affine::scale(2.0, 1.0)), false)
        .unwrap();
    assert!(near(span(&drawn(&d, p)), (0.0, 20.0, 0.0, 10.0)));
    assert!(near(span(&drawn(&d, c)), (60.0, 80.0, 0.0, 10.0)));
    // (b) A and B selected.
    let d = Document::new(1);
    let p = o(&d);
    let a = moved_clone(&d, p, 30.0, 0.0);
    let b = moved_clone(&d, a, 0.0, 30.0);
    d.transform_objects(&mapped_results(&d, &[a, b], Affine::scale(2.0, 1.0)), false)
        .unwrap();
    assert!(near(span(&drawn(&d, a)), (60.0, 80.0, 0.0, 10.0)));
    assert!(near(span(&drawn(&d, b)), (60.0, 80.0, 30.0, 40.0)));
    assert_eq!(clone_of(&d, b).chain.matrix, translate(0.0, 30.0));
    // (c) O and B selected, A not.
    let d = Document::new(1);
    let p = o(&d);
    let a = moved_clone(&d, p, 30.0, 0.0);
    let b = moved_clone(&d, a, 0.0, 30.0);
    d.transform_objects(&mapped_results(&d, &[p, b], Affine::scale(2.0, 1.0)), false)
        .unwrap();
    assert!(near(span(&drawn(&d, p)), (0.0, 20.0, 0.0, 10.0)));
    assert!(near(span(&drawn(&d, a)), (30.0, 50.0, 0.0, 10.0)));
    assert!(near(span(&drawn(&d, b)), (60.0, 80.0, 30.0, 40.0)));
}

#[test]
fn ac16_a_move_of_the_original_and_its_clone_keeps_them_together() {
    let d = Document::new(1);
    let p = o(&d);
    let c = moved_clone(&d, p, 30.0, 0.0);
    d.translate_objects(&[p, c], Vec2::new(5.0, 5.0)).unwrap();
    assert!(near(span(&drawn(&d, c)), (35.0, 45.0, 5.0, 15.0)));
    assert_eq!(clone_of(&d, c).chain.matrix, translate(30.0, 0.0));
}

#[test]
fn ac16_singular_case_unlinks_the_clone_and_maps_it_as_a_path() {
    let d = Document::new(1);
    let p = o(&d);
    let c = moved_clone(&d, p, 30.0, 0.0);
    let flat = Affine::scale(1.0, 0.0);
    let links: CloneLinks = HashMap::from([(c, (p, translate(30.0, 0.0)))]);
    let decided = clone_matrices_after(&links, &d.own_placement_clones(), &[p, c], flat);
    assert_eq!(decided.singular, vec![c]);
    // The gesture layer turns it into its unlinked path (fresh ids) mapped by G.
    let mut as_path = clone_of(&d, c).resolved.mapped(flat);
    for (anchor, id) in as_path.all_anchors_mut().zip(fresh(500, 4)) {
        anchor.id = id;
    }
    let results = vec![
        ObjectSnapshot::Path(path_of(&d, p).mapped(flat)),
        ObjectSnapshot::Path(as_path.clone()),
    ];
    d.transform_objects(&results, false).unwrap();
    assert_eq!(
        path_of(&d, c).anchors,
        as_path.anchors,
        "an independent path now"
    );
}

#[test]
fn ac17_a_change_that_pushes_a_clone_downstream_past_the_limit_is_refused() {
    let d = Document::new(1);
    let p = square(&d, 1, Point::new(0.0, 0.0), 1.0);
    let a = link(&d, p);
    d.transform_objects(
        &mapped_results(&d, &[a], Affine::scale(1000.0, 1000.0)),
        false,
    )
    .unwrap();
    let b = link(&d, a);
    d.transform_objects(
        &mapped_results(&d, &[b], Affine::scale(1000.0, 1000.0)),
        false,
    )
    .unwrap();
    assert!(near(span(&drawn(&d, b)), (0.0, 1e6, 0.0, 1e6)));
    let before = d.export_json().unwrap();
    assert_eq!(
        d.transform_objects(
            &mapped_results(&d, &[p], Affine::scale(100.0, 100.0)),
            false
        ),
        Err(ObjectEditError::CloneOutOfRange(b))
    );
    assert_eq!(
        d.translate_objects(&[b], Vec2::new(2e7, 0.0)),
        Err(ObjectEditError::CloneOutOfRange(b))
    );
    assert_eq!(d.export_json().unwrap(), before, "nothing written");
    // A node edit of the root is checked too.
    let far = path_of(&d, p).anchors[1].id;
    assert_eq!(
        d.move_anchors(&[(p, far, Point::new(100.0, 0.0))]),
        Err(PathEditError::CloneOutOfRange(b))
    );
    let corners: Vec<(AnchorId, Point, Vec2, Vec2)> = path_of(&d, p)
        .anchors
        .iter()
        .map(|a| {
            (
                a.id,
                Point::new(a.point.x * 50.0, a.point.y),
                a.handle_in,
                a.handle_out,
            )
        })
        .collect();
    assert_eq!(
        d.resize_path(p, &corners, None),
        Err(PathEditError::CloneOutOfRange(b))
    );
    assert_eq!(d.export_json().unwrap(), before, "nothing written");
    assert!(
        d.move_anchors(&[(p, far, Point::new(1.5, 0.0))]).is_ok(),
        "within the limit"
    );
}

#[test]
fn ac19_duplicate_of_a_clone_is_a_sibling_and_of_a_clone_original_follows_its_original() {
    let d = Document::new(1);
    let p = o(&d);
    let a = moved_clone(&d, p, 30.0, 0.0);
    let b = link(&d, a);
    let copies = d
        .duplicate_objects(
            &[CopySource {
                id: a,
                anchor_ids: Vec::new(),
            }],
            Vec2::new(0.0, 0.0),
        )
        .unwrap();
    let copy = clone_of(&d, copies[0]);
    assert_eq!(
        (copy.chain.origin, copy.chain.matrix, copy.chain.depth),
        (p, translate(30.0, 0.0), 1)
    );
    assert_eq!(clone_of(&d, b).chain.origin, a, "B stays on A");
    // A Ctrl-copy move of the clone by (5, 0).
    let moved = d
        .duplicate_objects(
            &[CopySource {
                id: a,
                anchor_ids: Vec::new(),
            }],
            Vec2::new(5.0, 0.0),
        )
        .unwrap();
    assert!(near(span(&drawn(&d, moved[0])), (35.0, 45.0, 0.0, 10.0)));
    // The copy of a path original is independent: no clone follows it.
    let path_copy = d
        .duplicate_objects(
            &[CopySource {
                id: p,
                anchor_ids: fresh(70, 4),
            }],
            Vec2::new(0.0, 0.0),
        )
        .unwrap()[0];
    assert!(!d.read_objects().clones_of.contains_key(&path_copy));
}

#[test]
fn ac47_a_copied_set_with_an_original_and_its_clones_relinks_to_the_copies() {
    let d = Document::new(1);
    let p = o(&d);
    let c1 = moved_clone(&d, p, 30.0, 0.0);
    let c2 = moved_clone(&d, p, 60.0, 0.0);
    let c3 = moved_clone(&d, p, 0.0, 30.0);
    let group = d.group_objects(&[p, c1, c2]).unwrap();
    let copies = d
        .duplicate_objects(
            &[CopySource {
                id: group,
                anchor_ids: fresh(80, 4),
            }],
            Vec2::new(0.0, 100.0),
        )
        .unwrap();
    let g2 = copies[0];
    let leaves = d.children(Some(g2));
    assert_eq!(leaves.len(), 3);
    let p2 = leaves[0];
    let with_offset = |x: f64| {
        *leaves
            .iter()
            .find(|&&id| matches!(d.object(id), Some(ObjectSnapshot::Clone(c)) if (c.chain.matrix.e - x).abs() < 1e-9))
            .unwrap()
    };
    let (c1b, c2b) = (with_offset(30.0), with_offset(60.0));
    assert_eq!(clone_of(&d, c1b).chain.origin, p2);
    assert_eq!(clone_of(&d, c2b).chain.origin, p2);
    assert_eq!(clone_of(&d, c3).chain.origin, p);
    assert!(
        near(span(&drawn(&d, c1b)), (30.0, 40.0, 100.0, 110.0)),
        "the arrangement moved"
    );
    let corner = path_of(&d, p2).anchors[0].id;
    d.move_anchors(&[(p2, corner, Point::new(-1.0, 99.0))])
        .unwrap();
    assert_eq!(
        clone_of(&d, c1b).resolved.anchors[0].point,
        Point::new(29.0, 99.0)
    );
    assert_eq!(
        clone_of(&d, c1).resolved.anchors[0].point,
        Point::new(30.0, 0.0),
        "the source clone does not follow the copy"
    );
    let _ = c2;
}

#[test]
fn ac48_a_copied_chain_without_its_root_is_a_chain_of_copies_on_the_root() {
    let d = Document::new(1);
    let p = o(&d);
    let other = square(&d, 2, Point::new(50.0, 0.0), 10.0);
    let a = link(&d, p);
    let b = link(&d, a);
    let group = d.group_objects(&[a, b, other]).unwrap();
    let copies = d
        .duplicate_objects(
            &[CopySource {
                id: group,
                anchor_ids: fresh(90, 4),
            }],
            Vec2::new(0.0, 0.0),
        )
        .unwrap();
    let leaves = d.children(Some(copies[0]));
    let (a2, b2) = (leaves[0], leaves[1]);
    assert_eq!(clone_of(&d, a2).chain.origin, p);
    assert_eq!(clone_of(&d, b2).chain.origin, a2);
    assert_eq!(clone_of(&d, b2).chain.depth, 2);
}

#[test]
fn ac28_29_55_unlink_keeps_the_picture_and_the_clones_behind_follow_the_path() {
    let d = Document::new(1);
    let p = o(&d);
    d.edit_style(&[p], &StyleEdit::StrokeColor(Color { r: 9, g: 8, b: 7 }))
        .unwrap();
    let a = moved_clone(&d, p, 30.0, 0.0);
    let b = moved_clone(&d, a, 0.0, 30.0);
    let (a_before, b_before) = (drawn(&d, a), drawn(&d, b));
    unlink(&d, &[a], 100);
    assert_eq!(
        d.own_steps().last().unwrap().label,
        UNLINK_CLONE_COMMIT_LABEL
    );
    let a_path = path_of(&d, a);
    same_points(&a_path, &a_before, 1e-9);
    assert_eq!(a_path.style, path_of(&d, p).style);
    assert_eq!(a_path.anchors[0].id, AnchorId::new(9, 100));
    assert_eq!(clone_of(&d, b).chain.origin, a, "B follows the path A");
    same_points(&drawn(&d, b), &b_before, 1e-9);
    // 29: an edit of O changes neither A nor B; an edit of A changes B.
    d.translate_objects(&[p], Vec2::new(3.0, 3.0)).unwrap();
    same_points(&drawn(&d, b), &b_before, 1e-9);
    d.translate_objects(&[a], Vec2::new(1.0, 0.0)).unwrap();
    assert!(near(
        span(&drawn(&d, b)),
        (
            span(&b_before).0 + 1.0,
            span(&b_before).1 + 1.0,
            span(&b_before).2,
            span(&b_before).3
        )
    ));
}

#[test]
fn ac28_unlink_refuses_wrong_ids_and_non_clones() {
    let d = Document::new(1);
    let p = o(&d);
    let c = link(&d, p);
    assert_eq!(
        d.unlink_clones(&[CopySource {
            id: c,
            anchor_ids: fresh(1, 3)
        }]),
        Err(CloneError::AnchorIds)
    );
    assert_eq!(
        d.unlink_clones(&[CopySource {
            id: p,
            anchor_ids: fresh(1, 4)
        }]),
        Err(CloneError::NotAClone)
    );
}

#[test]
fn ac41_the_path_read_of_a_clone_equals_the_path_read_after_unlink() {
    let d = Document::new(1);
    let p = o(&d);
    let a = moved_clone(&d, p, 30.0, 0.0);
    let b = moved_clone(&d, a, 0.0, 30.0);
    let before = drawn(&d, b);
    unlink(&d, &[b], 300);
    let after = path_of(&d, b);
    same_points(&before, &after, 1e-9);
    assert_eq!(before.style, after.style);
}

#[test]
fn ac31_delete_of_the_original_unlinks_its_clones_and_keeps_them_in_place() {
    let d = Document::new(1);
    let p = o(&d);
    let c1 = moved_clone(&d, p, 30.0, 0.0);
    let c2 = moved_clone(&d, p, 60.0, 0.0);
    let (b1, b2) = (drawn(&d, c1), drawn(&d, c2));
    d.delete_objects(&[p]).unwrap();
    same_points(&path_of(&d, c1), &b1, 1e-9);
    same_points(&path_of(&d, c2), &b2, 1e-9);
    assert_eq!(
        d.take_clone_repair(),
        CloneRepair {
            repointed: 0,
            unlinked: 2
        }
    );
    assert_eq!(d.take_clone_repair(), CloneRepair::default(), "taken once");
    let ids: Vec<AnchorId> = path_of(&d, c1)
        .all_anchors()
        .chain(path_of(&d, c2).all_anchors())
        .map(|a| a.id)
        .collect();
    let unique: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(unique.len(), ids.len(), "every anchor id unique");
    // Delete P and C1 together: only C2 becomes a path.
    let d = Document::new(1);
    let p = o(&d);
    let c1 = link(&d, p);
    let c2 = link(&d, p);
    d.delete_objects(&[p, c1]).unwrap();
    assert!(matches!(d.object(c2), Some(ObjectSnapshot::Path(_))));
    assert_eq!(d.object_ids(), vec![c2]);
}

#[test]
fn ac31_54_delete_in_a_chain_repoints_or_unlinks() {
    // Delete A: B follows O with M_B·M_A and draws as before.
    let d = Document::new(1);
    let p = o(&d);
    let a = moved_clone(&d, p, 30.0, 0.0);
    let b = link(&d, a);
    d.transform_objects(&mapped_results(&d, &[b], Affine::scale(2.0, 2.0)), false)
        .unwrap();
    let before = drawn(&d, b);
    assert!(near(span(&before), (60.0, 80.0, 0.0, 20.0)));
    d.delete_objects(&[a]).unwrap();
    let after = clone_of(&d, b);
    assert_eq!(after.chain.origin, p);
    assert_eq!(
        after.chain.matrix,
        Affine::new(2.0, 0.0, 0.0, 2.0, 60.0, 0.0)
    );
    same_points(&after.resolved, &before, 1e-9);
    assert_eq!(
        d.take_clone_repair(),
        CloneRepair {
            repointed: 1,
            unlinked: 0
        }
    );
    // Delete O: A becomes a path, B stays a clone of A.
    let d = Document::new(1);
    let p = o(&d);
    let a = link(&d, p);
    let b = link(&d, a);
    d.delete_objects(&[p]).unwrap();
    assert!(matches!(d.object(a), Some(ObjectSnapshot::Path(_))));
    assert_eq!(clone_of(&d, b).chain.origin, a);
    // Delete A and O together: B becomes a path.
    let d = Document::new(1);
    let p = o(&d);
    let a = link(&d, p);
    let b = link(&d, a);
    d.delete_objects(&[a, p]).unwrap();
    assert!(matches!(d.object(b), Some(ObjectSnapshot::Path(_))));
}

#[test]
fn ac31_a_boolean_that_consumes_the_original_repairs_its_clones() {
    let d = Document::new(1);
    let p = o(&d);
    let q = square(&d, 2, Point::new(5.0, 5.0), 10.0);
    let c = moved_clone(&d, p, 50.0, 0.0);
    let before = drawn(&d, c);
    let outline: Vec<NewAnchor> = path_of(&d, q)
        .anchors
        .iter()
        .enumerate()
        .map(|(k, a)| NewAnchor {
            id: AnchorId::new(9, 900 + k as u64),
            ..*a
        })
        .collect();
    d.replace_with_path(&[p, q], p, &[(outline, true)], "boolean_union")
        .unwrap();
    same_points(&path_of(&d, c), &before, 1e-9);
}

#[test]
fn ac33_a_result_based_on_a_clone_takes_the_root_style() {
    let d = Document::new(1);
    let p = o(&d);
    d.edit_style(&[p], &StyleEdit::StrokeColor(Color { r: 200, g: 0, b: 0 }))
        .unwrap();
    let c = moved_clone(&d, p, 20.0, 0.0);
    let q = square(&d, 2, Point::new(25.0, 0.0), 10.0);
    let outline: Vec<NewAnchor> = drawn(&d, c)
        .anchors
        .iter()
        .enumerate()
        .map(|(k, a)| NewAnchor {
            id: AnchorId::new(9, 800 + k as u64),
            ..*a
        })
        .collect();
    let result = d
        .replace_with_path(&[c, q], c, &[(outline, true)], "boolean_union")
        .unwrap();
    assert_eq!(path_of(&d, result).style, path_of(&d, p).style);
}

#[test]
fn ac35_a_merge_orphan_and_its_followers_are_removed_in_one_commit() {
    let d = Document::new(1);
    let p = o(&d);
    let keep = square(&d, 2, Point::new(50.0, 0.0), 10.0);
    let kept = link(&d, keep);
    assert_eq!(d.remove_orphan_clones(), 0, "nothing to remove, no commit");
    let peer =
        curvyo_document_core::unpack(2, &curvyo_document_core::pack(&d, "t").unwrap()).unwrap();
    let start = d.version();
    // One peer clones P (and clones the clone) while the other deletes P.
    let c = link(&d, p);
    let _ = link(&d, c);
    peer.delete_objects(&[p]).unwrap();
    d.import_updates(&peer.export_updates(&start).unwrap())
        .unwrap();
    assert_eq!(d.read_objects().orphans.len(), 2);
    let steps = d.own_step_count();
    assert_eq!(d.remove_orphan_clones(), 2);
    assert_eq!(d.own_step_count(), steps + 1, "one commit");
    assert_eq!(
        d.own_steps().last().unwrap().label,
        curvyo_document_core::REMOVE_ORPHAN_CLONES_COMMIT_LABEL
    );
    assert_eq!(d.object_ids(), vec![keep, kept]);
}

#[test]
fn ac26_node_edits_refuse_a_clone() {
    let d = Document::new(1);
    let p = o(&d);
    let c = link(&d, p);
    let anchor = path_of(&d, p).anchors[0].id;
    assert_eq!(
        d.move_anchors(&[(c, anchor, Point::new(1.0, 1.0))]),
        Err(PathEditError::NoSuchPath)
    );
    // `delete_anchors` takes pairs and skips a pair whose path is no editable path
    // (`specs/0053-node-marquee-selection/adrs.md` decision 4, note of 2026-10-14): a clone's
    // pair writes nothing and adds no step; Convert still refuses it.
    let steps = d.own_step_count();
    d.delete_anchors(&[(c, anchor)]);
    assert_eq!(d.own_step_count(), steps);
    assert_eq!(
        d.convert_anchor_kind(&[(c, anchor)], AnchorKind::Symmetric),
        Err(PathEditError::NoSuchPath)
    );
    assert!(matches!(d.object(c), Some(ObjectSnapshot::Clone(_))));
}

#[test]
fn ac34_a_document_resize_moves_clones_with_everything() {
    let d = Document::new(1);
    let p = o(&d);
    let c = moved_clone(&d, p, 30.0, 0.0);
    let before = span(&drawn(&d, c));
    let p_before = span(&drawn(&d, p));
    let size = d.size();
    d.resize(curvyo_document_core::DocumentSize::new(
        curvyo_document_core::Length::from_mm(size.width.as_mm() + 20.0),
        size.height,
    ))
    .unwrap();
    let shift = span(&drawn(&d, p)).0 - p_before.0;
    let after = span(&drawn(&d, c));
    assert!(near(
        after,
        (before.0 + shift, before.1 + shift, before.2, before.3)
    ));
}

/// A closed outline of four anchors with ids from `start`, the square (x, y)–(x+s, y+s).
fn outline(start: u64, x: f64, y: f64, s: f64) -> curvyo_document_core::PieceOutlines {
    let ids = fresh(start, 4);
    let points = [(x, y), (x + s, y), (x + s, y + s), (x, y + s)];
    vec![(
        ids.iter()
            .zip(points)
            .map(|(&id, (px, py))| NewAnchor::corner(id, Point::new(px, py)))
            .collect(),
        true,
    )]
}

/// Criterion 33 with `0038`: an Offset that adds a copy next to a clone reads the clone as a path
/// and gives the copy the root's style, the clone stays.
#[test]
fn ac33_an_offset_copy_of_a_clone_takes_the_roots_style() {
    use curvyo_document_core::{OffsetGeometry, OffsetWrite};
    let d = Document::new(1);
    let p = o(&d);
    d.edit_style(&[p], &StyleEdit::StrokeColor(Color { r: 9, g: 8, b: 7 }))
        .unwrap();
    let c = moved_clone(&d, p, 30.0, 0.0);
    let ids = d
        .write_offset(
            &[OffsetWrite::Add {
                source: c,
                below: true,
                geometry: OffsetGeometry::Path(outline(500, 29.0, -1.0, 12.0)),
            }],
            "offset",
        )
        .unwrap();
    assert_eq!(path_of(&d, ids[0]).style, path_of(&d, p).style);
    assert_eq!(clone_of(&d, c).chain.origin, p);
}

/// Criterion 33 with `0038`: Change original on a clone consumes it like any operand. It keeps
/// its id and place and becomes a path with the offset outline and the root's style; a clone
/// behind it stays linked and follows the new path.
#[test]
fn ac33_an_offset_change_of_a_clone_makes_it_a_path_its_clones_follow() {
    use curvyo_document_core::{OffsetGeometry, OffsetWrite};
    let d = Document::new(1);
    let p = o(&d);
    d.edit_style(&[p], &StyleEdit::StrokeColor(Color { r: 9, g: 8, b: 7 }))
        .unwrap();
    let a = moved_clone(&d, p, 30.0, 0.0);
    let b = moved_clone(&d, a, 0.0, 30.0);
    let order = d.object_ids();
    d.write_offset(
        &[OffsetWrite::Change {
            target: a,
            geometry: OffsetGeometry::Path(outline(600, 29.0, -1.0, 12.0)),
        }],
        "offset",
    )
    .unwrap();
    assert_eq!(d.object_ids(), order, "same id and place");
    let changed = path_of(&d, a);
    assert_eq!(changed.style, path_of(&d, p).style);
    assert_eq!(changed.anchors[0].point, Point::new(29.0, -1.0));
    let behind = clone_of(&d, b);
    assert_eq!(behind.chain.origin, a);
    assert_eq!(behind.chain.root, a);
    assert_eq!(behind.resolved.anchors[0].point, Point::new(29.0, 29.0));
}

/// Criterion 17 with `0038`: Change original of an Offset on a root is refused, whole, when a
/// clone that follows it would pass the limit, and the refusal names that clone, as for a node
/// edit.
#[test]
fn ac17_an_offset_change_of_a_root_that_pushes_a_clone_past_the_limit_is_refused() {
    use curvyo_document_core::{OffsetGeometry, OffsetWrite};
    let d = Document::new(1);
    let p = o(&d);
    let c = moved_clone(&d, p, 9_999_000.0, 0.0);
    let before = d.version();
    let result = d.write_offset(
        &[OffsetWrite::Change {
            target: p,
            geometry: OffsetGeometry::Path(outline(700, -1.0, -1.0, 2_000.0)),
        }],
        "offset",
    );
    assert_eq!(result, Err(ObjectEditError::CloneOutOfRange(c)));
    assert_eq!(d.take_clone_refusal(), Some(c));
    assert_eq!(d.version(), before, "nothing was changed");
    // Within the limit the same change goes through.
    d.write_offset(
        &[OffsetWrite::Change {
            target: p,
            geometry: OffsetGeometry::Path(outline(800, -1.0, -1.0, 12.0)),
        }],
        "offset",
    )
    .unwrap();
}
