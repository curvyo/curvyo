//! Independent tester acceptance tests for `specs/0053-node-marquee-selection/` at the document
//! layer: the order guarantee of `PathSnapshot::all_anchors` (which `0055` relies on) and the
//! cross-path `delete_anchors` / `convert_anchor_kind` (criteria 9, 18, 20). Written from the
//! specification and the public signatures before the implementation diff was read.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(clippy::cast_precision_loss, clippy::too_many_lines, missing_docs)]
#![allow(clippy::many_single_char_names)]

use std::collections::HashSet;

use curvyo_document_core::{
    AnchorId, AnchorKind, Document, NewAnchor, NodeId, ObjectSnapshot, PathEditError, PathSnapshot,
    Point,
};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn a(peer: u64, n: u64, x: f64, y: f64) -> NewAnchor {
    NewAnchor::corner(AnchorId::new(peer, n), pt(x, y))
}

fn path_of(d: &Document, id: NodeId) -> PathSnapshot {
    match d.object(id).expect("object") {
        ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
        ObjectSnapshot::Path(p) => p,
        ObjectSnapshot::Primitive(_) | ObjectSnapshot::Clone(_) => panic!("path expected"),
    }
}

fn ids_of(p: &PathSnapshot) -> Vec<AnchorId> {
    p.all_anchors().map(|x| x.id).collect()
}

fn xs(p: &PathSnapshot) -> Vec<(f64, f64)> {
    p.all_anchors().map(|x| (x.point.x, x.point.y)).collect()
}

fn changes(d: &Document) -> usize {
    let l = loro::LoroDoc::new();
    l.import(&d.export_loro_snapshot().unwrap()).unwrap();
    l.len_changes()
}

/// A compound path with `outlines`; the first `create_path` source object is replaced.
fn compound(d: &Document, outlines: &[(Vec<NewAnchor>, bool)]) -> NodeId {
    let seed = d.create_path(&[a(900, 0, 0.0, 0.0), a(900, 1, 1.0, 0.0)], false);
    d.replace_with_path(&[seed], seed, outlines, "boolean_union")
        .unwrap()
}

#[test]
fn all_anchors_single_outline_is_storage_order_not_position_order() {
    let d = Document::new(1);
    // Ids and positions deliberately neither ascending nor sorted.
    let id = d.create_path(
        &[
            a(5, 9, 30.0, 5.0),
            a(5, 2, -4.0, 80.0),
            a(5, 7, 12.0, -3.0),
            a(5, 0, 12.0, -3.0),
        ],
        false,
    );
    let p = path_of(&d, id);
    assert_eq!(
        ids_of(&p),
        vec![
            AnchorId::new(5, 9),
            AnchorId::new(5, 2),
            AnchorId::new(5, 7),
            AnchorId::new(5, 0)
        ]
    );
    assert_eq!(p.all_anchors().count(), p.anchors.len());
}

#[test]
fn all_anchors_closed_outline_does_not_repeat_the_first_anchor() {
    let d = Document::new(1);
    let id = d.create_path(
        &[a(1, 0, 0.0, 0.0), a(1, 1, 10.0, 0.0), a(1, 2, 10.0, 10.0)],
        true,
    );
    let p = path_of(&d, id);
    assert!(p.closed);
    assert_eq!(p.all_anchors().count(), 3);
    assert_eq!(p.all_anchors().next().unwrap().id, AnchorId::new(1, 0));
    assert_eq!(p.all_anchors().last().unwrap().id, AnchorId::new(1, 2));
}

#[test]
fn all_anchors_several_outlines_first_then_extra_in_storage_order() {
    let d = Document::new(1);
    let outlines = vec![
        (
            vec![
                a(10, 3, 50.0, 50.0),
                a(10, 1, 0.0, 0.0),
                a(10, 2, 20.0, 0.0),
            ],
            true,
        ),
        (vec![a(20, 8, -5.0, 9.0), a(20, 4, 100.0, 100.0)], false),
        (
            vec![
                a(30, 5, 7.0, 7.0),
                a(30, 6, 1.0, 1.0),
                a(30, 0, 4.0, 4.0),
                a(30, 9, 2.0, 2.0),
            ],
            true,
        ),
        (vec![a(40, 1, 0.0, 0.0)], false),
    ];
    let id = compound(&d, &outlines);
    let p = path_of(&d, id);
    assert!(p.is_compound());
    assert_eq!(p.extra_subpaths.len(), 3);
    let expected: Vec<AnchorId> = outlines
        .iter()
        .flat_map(|(v, _)| v.iter().map(|n| n.id))
        .collect();
    assert_eq!(ids_of(&p), expected, "outline by outline, anchor by anchor");
    // No repeats, the count is the sum of the outline lengths.
    let set: HashSet<_> = ids_of(&p).into_iter().collect();
    assert_eq!(set.len(), 10);
    // Closed/open flags do not change the walk.
    let flags: Vec<bool> = p.subpaths().map(|s| s.closed).collect();
    assert_eq!(flags, vec![true, false, true, false]);
    // Same order as flattening subpaths().
    let via_subpaths: Vec<AnchorId> = p
        .subpaths()
        .flat_map(|s| s.anchors.iter().map(|x| x.id))
        .collect();
    assert_eq!(ids_of(&p), via_subpaths);
}

#[test]
fn all_anchors_order_survives_save_and_reopen_and_edits_elsewhere() {
    let d = Document::new(1);
    let outlines = vec![
        (
            vec![a(1, 5, 9.0, 9.0), a(1, 4, 1.0, 1.0), a(1, 6, 5.0, 5.0)],
            true,
        ),
        (vec![a(2, 2, 3.0, 3.0), a(2, 1, 8.0, 8.0)], true),
    ];
    let id = compound(&d, &outlines);
    let other = d.create_path(&[a(3, 0, 0.0, 0.0), a(3, 1, 1.0, 1.0)], false);
    let before = ids_of(&path_of(&d, id));
    d.delete_anchors(&[(other, AnchorId::new(3, 1))]);
    let bytes = curvyo_document_core::pack(&d, "t").unwrap();
    let re = curvyo_document_core::unpack(7, &bytes).unwrap();
    assert_eq!(ids_of(&path_of(&re, id)), before);
}

#[test]
fn all_anchors_pseudo_random_outline_shapes_never_repeat_and_keep_storage_order() {
    let mut seed = 0x2545_F491_4F6C_DD1D_u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for case in 0..40_u64 {
        let d = Document::new(1);
        let n_out = 1 + (next() % 5) as usize;
        let mut outlines = Vec::new();
        let mut expected = Vec::new();
        let mut counter = 0_u64;
        for o in 0..n_out {
            let len = 1 + (next() % 6) as usize;
            let mut v = Vec::new();
            for _ in 0..len {
                // Ids descend in places, positions random.
                counter += 1 + next() % 3;
                let n = if next() % 2 == 0 {
                    counter
                } else {
                    1_000 - counter
                };
                let x = (next() % 200) as f64 - 100.0;
                let y = (next() % 200) as f64 - 100.0;
                expected.push(AnchorId::new(100 + o as u64, n));
                v.push(a(100 + o as u64, n, x, y));
            }
            outlines.push((v, next() % 2 == 0));
        }
        let id = if n_out == 1 && outlines[0].0.len() >= 2 {
            d.create_path(&outlines[0].0, outlines[0].1)
        } else if n_out == 1 {
            // A one-anchor single outline is not a path; skip.
            continue;
        } else {
            compound(&d, &outlines)
        };
        let p = path_of(&d, id);
        assert_eq!(ids_of(&p), expected, "case {case}");
        let set: HashSet<_> = ids_of(&p).into_iter().collect();
        assert_eq!(set.len(), expected.len(), "case {case}: no repeats");
    }
}

// ---- criterion 18: Delete across paths, one commit --------------------------------------

#[test]
fn ac18_delete_anchors_across_two_paths_is_one_commit_and_each_path_follows_the_rule() {
    let d = Document::new(1);
    let p1 = d.create_path(
        &[
            a(1, 0, 0.0, 0.0),
            a(1, 1, 10.0, 0.0),
            a(1, 2, 20.0, 0.0),
            a(1, 3, 30.0, 0.0),
            a(1, 4, 40.0, 0.0),
        ],
        false,
    );
    let p2 = d.create_path(
        &[
            a(2, 0, 0.0, 50.0),
            a(2, 1, 10.0, 50.0),
            a(2, 2, 20.0, 50.0),
            a(2, 3, 30.0, 50.0),
        ],
        true,
    );
    let before = changes(&d);
    d.delete_anchors(&[
        (p1, AnchorId::new(1, 1)),
        (p1, AnchorId::new(1, 2)),
        (p1, AnchorId::new(1, 3)),
        (p2, AnchorId::new(2, 1)),
    ]);
    assert_eq!(changes(&d), before + 1, "one commit for both paths");
    let r1 = path_of(&d, p1);
    assert_eq!(xs(&r1), vec![(0.0, 0.0), (40.0, 0.0)]);
    let r2 = path_of(&d, p2);
    assert_eq!(xs(&r2), vec![(0.0, 50.0), (20.0, 50.0), (30.0, 50.0)]);
    assert!(r2.closed);
}

#[test]
fn ac18_a_path_left_with_fewer_than_two_nodes_is_removed_in_the_same_commit() {
    let d = Document::new(1);
    let p1 = d.create_path(
        &[a(1, 0, 0.0, 0.0), a(1, 1, 10.0, 0.0), a(1, 2, 20.0, 0.0)],
        false,
    );
    let p2 = d.create_path(
        &[a(2, 0, 0.0, 9.0), a(2, 1, 10.0, 9.0), a(2, 2, 20.0, 9.0)],
        false,
    );
    let before = changes(&d);
    d.delete_anchors(&[
        (p1, AnchorId::new(1, 0)),
        (p1, AnchorId::new(1, 1)),
        (p1, AnchorId::new(1, 2)),
        (p2, AnchorId::new(2, 1)),
    ]);
    assert_eq!(changes(&d), before + 1);
    assert!(d.object(p1).is_none(), "p1 removed");
    assert_eq!(xs(&path_of(&d, p2)), vec![(0.0, 9.0), (20.0, 9.0)]);
    // One node left (2 of 3 removed): also removed.
    let p3 = d.create_path(
        &[a(3, 0, 0.0, 0.0), a(3, 1, 1.0, 0.0), a(3, 2, 2.0, 0.0)],
        false,
    );
    d.delete_anchors(&[(p3, AnchorId::new(3, 0)), (p3, AnchorId::new(3, 2))]);
    assert!(d.object(p3).is_none(), "one node left: removed");
}

#[test]
fn ac18_deleting_the_last_members_of_a_group_removes_the_group_in_the_same_commit() {
    let d = Document::new(1);
    let p1 = d.create_path(&[a(1, 0, 0.0, 0.0), a(1, 1, 10.0, 0.0)], false);
    let p2 = d.create_path(&[a(2, 0, 0.0, 9.0), a(2, 1, 10.0, 9.0)], false);
    let keep = d.create_path(&[a(3, 0, 0.0, 99.0), a(3, 1, 10.0, 99.0)], false);
    let g = d.group_objects(&[p1, p2]).unwrap();
    let before = changes(&d);
    d.delete_anchors(&[(p1, AnchorId::new(1, 0)), (p2, AnchorId::new(2, 0))]);
    assert_eq!(changes(&d), before + 1);
    assert!(d.object(p1).is_none() && d.object(p2).is_none());
    assert_eq!(d.object_ids(), vec![keep]);
    let _ = g;
}

#[test]
fn ac18_duplicates_stale_and_empty_input_are_harmless() {
    let d = Document::new(1);
    let p1 = d.create_path(
        &[
            a(1, 0, 0.0, 0.0),
            a(1, 1, 10.0, 0.0),
            a(1, 2, 20.0, 0.0),
            a(1, 3, 30.0, 0.0),
        ],
        false,
    );
    let before = changes(&d);
    d.delete_anchors(&[]);
    assert_eq!(changes(&d), before, "empty input writes nothing");
    d.delete_anchors(&[
        (p1, AnchorId::new(1, 1)),
        (p1, AnchorId::new(1, 1)),
        (p1, AnchorId::new(77, 77)),
    ]);
    assert_eq!(
        xs(&path_of(&d, p1)),
        vec![(0.0, 0.0), (20.0, 0.0), (30.0, 0.0)]
    );
    // A path that no longer exists does not panic.
    d.delete_anchors(&[(p1, AnchorId::new(1, 0)), (p1, AnchorId::new(1, 2))]);
    assert!(d.object(p1).is_none());
    d.delete_anchors(&[(p1, AnchorId::new(1, 3))]);
}

#[test]
fn ac18_delete_keeps_the_neighbours_own_handles() {
    let d = Document::new(1);
    let mut n0 = a(1, 0, 0.0, 0.0);
    n0.handle_out = curvyo_document_core::Vec2::new(3.0, 4.0);
    let mut n2 = a(1, 2, 20.0, 0.0);
    n2.handle_in = curvyo_document_core::Vec2::new(-2.0, 5.0);
    let p = d.create_path(&[n0, a(1, 1, 10.0, 0.0), n2], false);
    d.delete_anchors(&[(p, AnchorId::new(1, 1))]);
    let r = path_of(&d, p);
    assert_eq!(r.anchors.len(), 2);
    assert_eq!(
        r.anchors[0].handle_out,
        curvyo_document_core::Vec2::new(3.0, 4.0)
    );
    assert_eq!(
        r.anchors[1].handle_in,
        curvyo_document_core::Vec2::new(-2.0, 5.0)
    );
}

// ---- criterion 20: Make corner / symmetric / asymmetric across paths ----------------------

#[test]
fn ac20_convert_anchor_kind_acts_on_nodes_of_several_paths_in_one_commit() {
    let d = Document::new(1);
    let mk = |peer: u64, y: f64| {
        let mut n = a(peer, 1, 10.0, y);
        n.kind = AnchorKind::Symmetric;
        n.handle_in = curvyo_document_core::Vec2::new(-3.0, 0.0);
        n.handle_out = curvyo_document_core::Vec2::new(3.0, 0.0);
        n
    };
    let p1 = d.create_path(&[a(1, 0, 0.0, 0.0), mk(1, 0.0), a(1, 2, 20.0, 0.0)], false);
    let p2 = d.create_path(&[a(2, 0, 0.0, 9.0), mk(2, 9.0), a(2, 2, 20.0, 9.0)], false);
    let before = changes(&d);
    d.convert_anchor_kind(
        &[(p1, AnchorId::new(1, 1)), (p2, AnchorId::new(2, 1))],
        AnchorKind::Corner,
    )
    .unwrap();
    assert_eq!(changes(&d), before + 1, "one commit");
    for p in [p1, p2] {
        let r = path_of(&d, p);
        assert_eq!(r.anchors[1].kind, AnchorKind::Corner);
        assert_eq!(r.anchors[0].kind, AnchorKind::Corner);
    }
    d.convert_anchor_kind(
        &[(p1, AnchorId::new(1, 1)), (p2, AnchorId::new(2, 1))],
        AnchorKind::Symmetric,
    )
    .unwrap();
    for p in [p1, p2] {
        assert_eq!(path_of(&d, p).anchors[1].kind, AnchorKind::Symmetric);
    }
}

#[test]
fn ac20_a_stale_id_refuses_the_whole_conversion_before_any_write() {
    let d = Document::new(1);
    let p1 = d.create_path(
        &[a(1, 0, 0.0, 0.0), a(1, 1, 10.0, 0.0), a(1, 2, 20.0, 0.0)],
        false,
    );
    let p2 = d.create_path(
        &[a(2, 0, 0.0, 9.0), a(2, 1, 10.0, 9.0), a(2, 2, 20.0, 9.0)],
        false,
    );
    let snap = (path_of(&d, p1), path_of(&d, p2));
    let before = changes(&d);
    let err = d
        .convert_anchor_kind(
            &[(p1, AnchorId::new(1, 1)), (p2, AnchorId::new(2, 99))],
            AnchorKind::Symmetric,
        )
        .unwrap_err();
    assert!(matches!(err, PathEditError::NoSuchAnchor), "{err:?}");
    assert_eq!(changes(&d), before, "nothing written");
    assert_eq!((path_of(&d, p1), path_of(&d, p2)), snap);
}
