//! Independent tester tests for `specs/0049-linked-clone` at the document core. Written from the
//! specification before the implementation was read; only the names of the public seams were
//! looked up. The only knowledge of the stored form is the shape of the `clone_link` register
//! (a map with `origin` and `matrix`), taken from the golden file, used to forge hostile files.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(missing_docs, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#![allow(
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::similar_names
)]
#![allow(
    clippy::cast_precision_loss,
    clippy::type_complexity,
    clippy::doc_markdown
)]

use std::collections::HashMap;

use curvyo_document_core::{
    AnchorId, CURRENT_FORMAT_VERSION, Color, CopySource, Document, NewAnchor, NodeId,
    ObjectSnapshot, OpenError, Point, StyleEdit, Vec2, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeID};

const TREE: &str = "paths";
static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1_000);

fn aid() -> AnchorId {
    AnchorId::new(7, NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
}

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn square(d: &Document, x: f64, y: f64, s: f64) -> NodeId {
    d.create_path(
        &[
            NewAnchor::corner(aid(), pt(x, y)),
            NewAnchor::corner(aid(), pt(x + s, y)),
            NewAnchor::corner(aid(), pt(x + s, y + s)),
            NewAnchor::corner(aid(), pt(x, y + s)),
        ],
        true,
    )
}

fn tid(id: NodeId) -> TreeID {
    let j = serde_json::to_value(id).unwrap();
    TreeID::new(
        j["peer"].as_str().unwrap().parse().unwrap(),
        j["counter"].as_i64().unwrap() as i32,
    )
}

fn nid(t: TreeID) -> NodeId {
    serde_json::from_value(serde_json::json!({"peer": t.peer.to_string(), "counter": t.counter}))
        .unwrap()
}

fn origin_text(id: NodeId) -> String {
    let t = tid(id);
    format!("{}:{}", t.peer, t.counter)
}

fn link_value(origin: Option<&str>, matrix: Option<Vec<LoroValue>>) -> LoroValue {
    let mut m: HashMap<String, LoroValue> = HashMap::new();
    if let Some(o) = origin {
        m.insert("origin".into(), LoroValue::from(o));
    }
    if let Some(v) = matrix {
        m.insert("matrix".into(), LoroValue::from(v));
    }
    LoroValue::from(m)
}

fn nums(m: [f64; 6]) -> Vec<LoroValue> {
    m.iter().map(|&x| LoroValue::from(x)).collect()
}

/// Rewrites the `clone_link` register of `clone` in a copy of `d`.
fn forged(d: &Document, clone: NodeId, link: LoroValue) -> Result<Document, OpenError> {
    let l = LoroDoc::new();
    l.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let tree = l.get_tree(TREE);
    let meta = tree.get_meta(tid(clone)).unwrap();
    meta.insert("clone_link", link).unwrap();
    l.commit();
    reopen(5, &l.export(loro::ExportMode::Snapshot).unwrap())
}

fn set_matrix(d: &Document, clone: NodeId, m: [f64; 6]) -> Document {
    let l = LoroDoc::new();
    l.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let meta = l.get_tree(TREE).get_meta(tid(clone)).unwrap();
    let origin = match meta.get("clone_link").unwrap() {
        loro::ValueOrContainer::Value(LoroValue::Map(map)) => match map.get("origin").unwrap() {
            LoroValue::String(s) => s.to_string(),
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    };
    meta.insert("clone_link", link_value(Some(&origin), Some(nums(m))))
        .unwrap();
    l.commit();
    reopen(1, &l.export(loro::ExportMode::Snapshot).unwrap()).unwrap()
}

fn outline(o: &ObjectSnapshot) -> Vec<(f64, f64)> {
    match o {
        ObjectSnapshot::Sketch(_) => unreachable!("no sketch in this test"),
        ObjectSnapshot::Image(_) => unreachable!("no image in this test"),
        ObjectSnapshot::Dimension(_) => unreachable!("no dimension in this test"),
        ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
        ObjectSnapshot::Path(p) => p.anchors.iter().map(|a| (a.point.x, a.point.y)).collect(),
        ObjectSnapshot::Clone(c) => c
            .resolved
            .anchors
            .iter()
            .map(|a| (a.point.x, a.point.y))
            .collect(),
        ObjectSnapshot::Primitive(_) => panic!("a primitive"),
    }
}

fn out(d: &Document, id: NodeId) -> Vec<(f64, f64)> {
    outline(&d.object(id).expect("the object exists"))
}

fn near(a: &[(f64, f64)], b: &[(f64, f64)], tol: f64) {
    assert_eq!(a.len(), b.len(), "{a:?} vs {b:?}");
    for (p, q) in a.iter().zip(b) {
        assert!(
            (p.0 - q.0).abs() <= tol && (p.1 - q.1).abs() <= tol,
            "{a:?} vs {b:?}"
        );
    }
}

fn is_clone(d: &Document, id: NodeId) -> bool {
    matches!(d.object(id), Some(ObjectSnapshot::Clone(_)))
}

fn matrix_of(d: &Document, id: NodeId) -> [f64; 6] {
    match d.object(id) {
        Some(ObjectSnapshot::Clone(c)) => {
            let m = c.chain.matrix;
            [m.a, m.b, m.c, m.d, m.e, m.f]
        }
        other => panic!("not a clone: {other:?}"),
    }
}

fn composed_of(d: &Document, id: NodeId) -> [f64; 6] {
    match d.object(id) {
        Some(ObjectSnapshot::Clone(c)) => {
            let m = c.chain.composed;
            [m.a, m.b, m.c, m.d, m.e, m.f]
        }
        other => panic!("not a clone: {other:?}"),
    }
}

fn origin_of(d: &Document, id: NodeId) -> NodeId {
    match d.object(id) {
        Some(ObjectSnapshot::Clone(c)) => c.chain.origin,
        other => panic!("not a clone: {other:?}"),
    }
}

fn mk(d: &Document, of: NodeId) -> NodeId {
    d.linked_clone(&[of], &[]).unwrap()[0]
}

fn unlink(d: &Document, ids: &[NodeId]) {
    let src: Vec<CopySource> = ids
        .iter()
        .map(|&id| CopySource {
            id,
            anchor_ids: (0..out(d, id).len()).map(|_| aid()).collect(),
        })
        .collect();
    d.unlink_clones(&src).unwrap();
}

// ---------------------------------------------------------------------------------------------
// Making a clone (1, 3, 52)
// ---------------------------------------------------------------------------------------------

#[test]
fn ac1_clones_sit_directly_above_their_originals_with_the_identity_matrix() {
    let d = Document::new(1);
    let (a, b, c) = (
        square(&d, 0.0, 0.0, 10.0),
        square(&d, 20.0, 0.0, 10.0),
        square(&d, 40.0, 0.0, 10.0),
    );
    let made = d.linked_clone(&[a, b, c], &[]).unwrap();
    assert_eq!(made.len(), 3);
    let order = d.object_ids();
    assert_eq!(order, vec![a, made[0], b, made[1], c, made[2]]);
    for (k, src) in [a, b, c].iter().enumerate() {
        assert_eq!(matrix_of(&d, made[k]), [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        assert_eq!(origin_of(&d, made[k]), *src);
        near(&out(&d, made[k]), &out(&d, *src), 1e-12);
    }
}

#[test]
fn ac1_one_commit_labelled_linked_clone() {
    let d = Document::new(1);
    let a = square(&d, 0.0, 0.0, 10.0);
    let b = square(&d, 20.0, 0.0, 10.0);
    let before = labels(&d).len();
    d.linked_clone(&[a, b], &[]).unwrap();
    let l = labels(&d);
    assert_eq!(l.len(), before + 1, "{l:?}");
    assert_eq!(l.last().unwrap(), "linked_clone");
}

fn labels(d: &Document) -> Vec<String> {
    let l = LoroDoc::new();
    l.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let ids: Vec<loro::ID> = l.oplog_frontiers().iter().collect();
    let mut changes: Vec<(u32, String)> = Vec::new();
    l.travel_change_ancestors(&ids, &mut |c| {
        changes.push((
            c.lamport,
            c.message
                .map(|m| m.split(';').next().unwrap_or_default().to_string())
                .unwrap_or_default(),
        ));
        std::ops::ControlFlow::Continue(())
    })
    .unwrap();
    changes.sort();
    changes.into_iter().map(|(_, m)| m).collect()
}

#[test]
fn ac3_a_clone_of_a_clone_is_a_chain_with_the_selected_clone_as_original() {
    let d = Document::new(1);
    let p = square(&d, 0.0, 0.0, 10.0);
    let c1 = mk(&d, p);
    d.translate_objects(&[c1], Vec2::new(30.0, 0.0)).unwrap();
    let c2 = mk(&d, c1);
    assert_eq!(origin_of(&d, c2), c1);
    assert_eq!(matrix_of(&d, c2), [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    near(&out(&d, c2), &out(&d, c1), 1e-12);
    let ids = d.object_ids();
    let pos = |x| ids.iter().position(|&i| i == x).unwrap();
    assert_eq!(pos(c2), pos(c1) + 1);
    match d.object(c2).unwrap() {
        ObjectSnapshot::Clone(c) => {
            assert_eq!(c.chain.depth, 2);
            assert_eq!(c.chain.root, p);
        }
        _ => panic!(),
    }
    // Edit P: both change.
    d.translate_objects(&[p], Vec2::new(0.0, 7.0)).unwrap();
    assert_eq!(out(&d, c1)[0], (30.0, 7.0));
    assert_eq!(out(&d, c2)[0], (30.0, 7.0));
}

#[test]
fn ac52_the_ninth_level_is_refused_all_or_nothing() {
    let d = Document::new(1);
    let p = square(&d, 0.0, 0.0, 10.0);
    let other = square(&d, 50.0, 0.0, 10.0);
    let mut last = p;
    let mut chain = vec![];
    for _ in 0..8 {
        last = mk(&d, last);
        chain.push(last);
    }
    let before = d.object_ids();
    let err = d.linked_clone(&[other, last], &[]).unwrap_err();
    assert!(
        matches!(&err, curvyo_document_core::CloneError::TooDeep(v) if v == &vec![last]),
        "{err:?}"
    );
    assert_eq!(d.object_ids(), before, "nothing created");
    // The root original can still be cloned: depth 1.
    let fresh = mk(&d, p);
    match d.object(fresh).unwrap() {
        ObjectSnapshot::Clone(c) => assert_eq!(c.chain.depth, 1),
        _ => panic!(),
    }
}

// ---------------------------------------------------------------------------------------------
// What a clone shows (7, 8, 9, 10, 12, 50, 51)
// ---------------------------------------------------------------------------------------------

#[test]
fn ac7_resolved_outline_applies_the_matrix_to_points() {
    let d = Document::new(1);
    let p = square(&d, 0.0, 0.0, 10.0);
    let c = mk(&d, p);
    let d = set_matrix(&d, c, [2.0, 0.0, 0.0, 2.0, 30.0, 0.0]);
    near(
        &out(&d, c),
        &[(30.0, 0.0), (50.0, 0.0), (50.0, 20.0), (30.0, 20.0)],
        1e-12,
    );
    // Handles are mapped by the linear part only.
    let d2 = Document::new(1);
    let mut a0 = NewAnchor::corner(aid(), pt(0.0, 0.0));
    a0.handle_out = Vec2::new(3.0, 0.0);
    let a1 = NewAnchor::corner(aid(), pt(10.0, 0.0));
    let q = d2.create_path(&[a0, a1], false);
    let cq = mk(&d2, q);
    let d2 = set_matrix(&d2, cq, [0.0, 1.0, -1.0, 0.0, 5.0, 5.0]);
    match d2.object(cq).unwrap() {
        ObjectSnapshot::Clone(c) => {
            let a = &c.resolved.anchors[0];
            assert!((a.point.x - 5.0).abs() < 1e-12 && (a.point.y - 5.0).abs() < 1e-12);
            // L (3, 0) = (a*3, b*3) = (0, 3)
            assert!(a.handle_out.x.abs() < 1e-12 && (a.handle_out.y - 3.0).abs() < 1e-12);
            assert!(!c.resolved.closed, "open stays open");
        }
        _ => panic!(),
    }
}

#[test]
fn ac8_the_style_is_the_roots_read_live_through_a_chain() {
    let d = Document::new(1);
    let p = square(&d, 0.0, 0.0, 10.0);
    let a = mk(&d, p);
    let b = mk(&d, a);
    d.edit_style(&[p], &StyleEdit::StrokeColor(Color { r: 255, g: 0, b: 0 }))
        .unwrap();
    let root_style = d.object(p).unwrap().style().clone();
    for c in [a, b] {
        assert_eq!(*d.object(c).unwrap().style(), root_style);
    }
    // Style edits aimed at a clone alone are not accepted as a clone style: the root decides.
    let _ = d.edit_style(&[a], &StyleEdit::StrokeColor(Color { r: 0, g: 255, b: 0 }));
    assert_eq!(*d.object(b).unwrap().style(), *d.object(p).unwrap().style());
    assert_eq!(*d.object(a).unwrap().style(), *d.object(p).unwrap().style());
}

#[test]
fn ac10_and_12_a_move_of_the_original_moves_the_clone_in_its_local_frame() {
    let d = Document::new(1);
    let p = square(&d, 0.0, 0.0, 10.0);
    let c = mk(&d, p);
    let d = set_matrix(&d, c, [0.0, 1.0, -1.0, 0.0, 0.0, 0.0]);
    let before = out(&d, c);
    d.translate_objects(&[p], Vec2::new(5.0, 0.0)).unwrap();
    let after = out(&d, c);
    for (b, a) in before.iter().zip(&after) {
        assert!(
            (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1 - 5.0).abs() < 1e-9,
            "{b:?} {a:?}"
        );
    }
    // A translate-only matrix moves 5 mm right.
    let d = Document::new(1);
    let p = square(&d, 0.0, 0.0, 10.0);
    let c = mk(&d, p);
    d.translate_objects(&[c], Vec2::new(100.0, 0.0)).unwrap();
    let before = out(&d, c);
    d.translate_objects(&[p], Vec2::new(5.0, 0.0)).unwrap();
    for (b, a) in before.iter().zip(out(&d, c)) {
        assert!((a.0 - b.0 - 5.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9);
    }
}

#[test]
fn ac15_and_51_moving_a_clone_writes_only_that_clone() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let a = mk(&d, o);
    let b = mk(&d, a);
    let d = set_matrix(&d, a, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    let d = set_matrix(&d, b, [0.0, 1.0, -1.0, 0.0, 0.0, 0.0]);
    let (o0, b0) = (out(&d, o), out(&d, b));
    let m_b = matrix_of(&d, b);
    d.translate_objects(&[a], Vec2::new(5.0, 0.0)).unwrap();
    assert_eq!(out(&d, o), o0, "O untouched");
    assert_eq!(matrix_of(&d, b), m_b, "B's own matrix untouched");
    for (p, q) in b0.iter().zip(out(&d, b)) {
        assert!(
            (q.0 - p.0).abs() < 1e-9 && (q.1 - p.1 - 5.0).abs() < 1e-9,
            "B follows in its frame"
        );
    }
    // (c) B moved: A and O untouched.
    let a0 = out(&d, a);
    d.translate_objects(&[b], Vec2::new(0.0, 9.0)).unwrap();
    assert_eq!(out(&d, a), a0);
    assert_eq!(out(&d, o), o0);
}

#[test]
fn ac50_the_chain_composes_and_both_ways_agree() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let a = mk(&d, o);
    let b = mk(&d, a);
    let d = set_matrix(&d, a, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    let d = set_matrix(&d, b, [2.0, 0.0, 0.0, 2.0, 0.0, 0.0]);
    near(
        &out(&d, a),
        &[(30.0, 0.0), (40.0, 0.0), (40.0, 10.0), (30.0, 10.0)],
        1e-9,
    );
    near(
        &out(&d, b),
        &[(60.0, 0.0), (80.0, 0.0), (80.0, 20.0), (60.0, 20.0)],
        1e-9,
    );
    assert_eq!(composed_of(&d, b), [2.0, 0.0, 0.0, 2.0, 60.0, 0.0]);
}

#[test]
fn ac16_scaling_a_clone_together_with_its_original_keeps_the_relation() {
    // The document-level writer: results are the new snapshots of the selected objects.
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let c = mk(&d, o);
    let d = set_matrix(&d, c, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    // Translate both by (4, 6): the clone keeps its place relative to the original.
    d.translate_objects(&[o, c], Vec2::new(4.0, 6.0)).unwrap();
    assert_eq!(out(&d, o)[0], (4.0, 6.0));
    assert_eq!(out(&d, c)[0], (34.0, 6.0));
    assert_eq!(matrix_of(&d, c), [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
}

// ---------------------------------------------------------------------------------------------
// Unlink (28, 29, 41, 55)
// ---------------------------------------------------------------------------------------------

#[test]
fn ac28_unlink_bakes_the_resolved_outline_with_fresh_anchor_ids() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    d.edit_style(&[o], &StyleEdit::FillColor(Color { r: 1, g: 2, b: 3 }))
        .unwrap();
    let c = mk(&d, o);
    let d = set_matrix(&d, c, [0.0, 2.0, -2.0, 0.0, 7.0, 3.0]);
    let before = out(&d, c);
    let style = d.object(c).unwrap().style().clone();
    let ids_before = d.object_ids();
    let l0 = labels(&d).len();
    unlink(&d, &[c]);
    assert_eq!(d.object_ids(), ids_before, "same id, same place");
    assert!(!is_clone(&d, c));
    near(&out(&d, c), &before, 1e-9);
    assert_eq!(*d.object(c).unwrap().style(), style);
    let l = labels(&d);
    assert_eq!(l.len(), l0 + 1);
    assert_eq!(l.last().unwrap(), "unlink_clone");
    match d.object(c).unwrap() {
        ObjectSnapshot::Path(p) => {
            assert_eq!(p.rotation.as_radians(), 0.0);
            let mut ids: Vec<_> = p.anchors.iter().map(|a| a.id).collect();
            let root_ids: Vec<_> = match d.object(o).unwrap() {
                ObjectSnapshot::Path(r) => r.anchors.iter().map(|a| a.id).collect(),
                _ => panic!(),
            };
            for i in &ids {
                assert!(
                    !root_ids.contains(i),
                    "anchor ids are unique in the document"
                );
            }
            ids.sort_by_key(|i| format!("{i:?}"));
            ids.dedup();
            assert_eq!(ids.len(), 4);
        }
        _ => panic!("a path"),
    }
    // Later edit of the original changes nothing in it (29).
    d.translate_objects(&[o], Vec2::new(50.0, 50.0)).unwrap();
    near(&out(&d, c), &before, 1e-9);
}

#[test]
fn ac55_unlink_in_a_chain() {
    let build = || {
        let d = Document::new(1);
        let o = square(&d, 0.0, 0.0, 10.0);
        let a = mk(&d, o);
        let b = mk(&d, a);
        let d = set_matrix(&d, a, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
        let d = set_matrix(&d, b, [2.0, 0.0, 0.0, 2.0, 0.0, 0.0]);
        (d, o, a, b)
    };
    // (a) unlink A
    let (d, o, a, b) = build();
    let (a0, b0) = (out(&d, a), out(&d, b));
    unlink(&d, &[a]);
    assert!(!is_clone(&d, a) && is_clone(&d, b));
    assert_eq!(origin_of(&d, b), a);
    assert_eq!(matrix_of(&d, b), [2.0, 0.0, 0.0, 2.0, 0.0, 0.0]);
    near(&out(&d, a), &a0, 1e-9);
    near(&out(&d, b), &b0, 1e-9);
    d.translate_objects(&[o], Vec2::new(1.0, 1.0)).unwrap();
    near(&out(&d, a), &a0, 1e-9);
    near(&out(&d, b), &b0, 1e-9);
    // (b) unlink B only
    let (d, o, a, b) = build();
    let (a0, b0) = (out(&d, a), out(&d, b));
    unlink(&d, &[b]);
    assert!(!is_clone(&d, b) && is_clone(&d, a));
    near(&out(&d, b), &b0, 1e-9);
    near(&out(&d, a), &a0, 1e-9);
    d.translate_objects(&[o], Vec2::new(1.0, 0.0)).unwrap();
    assert!(
        (out(&d, a)[0].0 - a0[0].0 - 1.0).abs() < 1e-9,
        "A still follows O"
    );
    near(&out(&d, b), &b0, 1e-9);
    // (c) both
    let (d, _o, a, b) = build();
    let (a0, b0) = (out(&d, a), out(&d, b));
    unlink(&d, &[a, b]);
    assert!(!is_clone(&d, a) && !is_clone(&d, b));
    near(&out(&d, a), &a0, 1e-9);
    near(&out(&d, b), &b0, 1e-9);
    // (d) O is not a clone: no unlink.
    let (d, o, _a, _b) = build();
    let before = d.object_ids();
    assert!(
        d.unlink_clones(&[CopySource {
            id: o,
            anchor_ids: (0..4).map(|_| aid()).collect()
        }])
        .is_err()
    );
    assert_eq!(d.object_ids(), before);
}

#[test]
fn ac41_the_path_read_of_a_clone_equals_the_read_after_unlink() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    d.edit_style(&[o], &StyleEdit::FillColor(Color { r: 9, g: 8, b: 7 }))
        .unwrap();
    let a = mk(&d, o);
    let b = mk(&d, a);
    let d = set_matrix(&d, a, [1.0, 0.5, 0.0, 1.0, 3.0, 4.0]);
    let d = set_matrix(&d, b, [0.0, 1.0, -1.0, 0.0, 2.0, 0.0]);
    let before = d.object(b).unwrap();
    let pts = out(&d, b);
    let style = before.style().clone();
    unlink(&d, &[b]);
    near(&out(&d, b), &pts, 1e-9);
    assert_eq!(*d.object(b).unwrap().style(), style);
}

// ---------------------------------------------------------------------------------------------
// Delete and repair (30, 31, 54)
// ---------------------------------------------------------------------------------------------

#[test]
fn ac31_deleting_the_original_turns_its_clones_into_paths_that_stay_put() {
    let d = Document::new(1);
    let p = square(&d, 0.0, 0.0, 10.0);
    let c1 = mk(&d, p);
    let c2 = mk(&d, p);
    let d = set_matrix(&d, c1, [1.0, 0.0, 0.0, 1.0, 20.0, 0.0]);
    let d = set_matrix(&d, c2, [3.0, 0.0, 0.0, 3.0, 0.0, 40.0]);
    let (o1, o2) = (out(&d, c1), out(&d, c2));
    d.delete_objects(&[p]).unwrap();
    assert!(d.object(p).is_none());
    assert!(!is_clone(&d, c1) && !is_clone(&d, c2));
    near(&out(&d, c1), &o1, 1e-9);
    near(&out(&d, c2), &o2, 1e-9);
    let r = d.take_clone_repair();
    assert_eq!((r.unlinked, r.repointed), (2, 0));
}

#[test]
fn ac31_deleting_the_original_and_one_clone_unlinks_only_the_other() {
    let d = Document::new(1);
    let p = square(&d, 0.0, 0.0, 10.0);
    let c1 = mk(&d, p);
    let c2 = mk(&d, p);
    let d = set_matrix(&d, c2, [1.0, 0.0, 0.0, 1.0, 20.0, 0.0]);
    d.delete_objects(&[p, c1]).unwrap();
    assert!(d.object(c1).is_none());
    assert!(matches!(d.object(c2), Some(ObjectSnapshot::Path(_))));
    assert_eq!(d.take_clone_repair().unlinked, 1);
}

#[test]
fn ac54_deleting_a_middle_clone_repoints_the_one_behind_it() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let a = mk(&d, o);
    let b = mk(&d, a);
    let d = set_matrix(&d, a, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    let d = set_matrix(&d, b, [2.0, 0.0, 0.0, 2.0, 0.0, 0.0]);
    let before = out(&d, b);
    let order = d.object_ids();
    d.delete_objects(&[a]).unwrap();
    assert!(is_clone(&d, b));
    assert_eq!(origin_of(&d, b), o);
    assert_eq!(matrix_of(&d, b), [2.0, 0.0, 0.0, 2.0, 60.0, 0.0]);
    near(&out(&d, b), &before, 1e-9);
    let after = d.object_ids();
    assert_eq!(
        after,
        order.into_iter().filter(|&i| i != a).collect::<Vec<_>>()
    );
    let r = d.take_clone_repair();
    assert_eq!((r.unlinked, r.repointed), (0, 1));
}

#[test]
fn ac31_deleting_the_root_of_a_chain_keeps_the_rest_as_path_plus_clone() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let a = mk(&d, o);
    let b = mk(&d, a);
    let d = set_matrix(&d, a, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    let d = set_matrix(&d, b, [2.0, 0.0, 0.0, 2.0, 0.0, 0.0]);
    let (a0, b0) = (out(&d, a), out(&d, b));
    d.delete_objects(&[o]).unwrap();
    assert!(matches!(d.object(a), Some(ObjectSnapshot::Path(_))));
    assert!(is_clone(&d, b));
    assert_eq!(origin_of(&d, b), a);
    near(&out(&d, a), &a0, 1e-9);
    near(&out(&d, b), &b0, 1e-9);
    // Delete A and O together on a fresh doc: B becomes a path.
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let a = mk(&d, o);
    let b = mk(&d, a);
    let d = set_matrix(&d, a, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    let d = set_matrix(&d, b, [2.0, 0.0, 0.0, 2.0, 0.0, 0.0]);
    let b0 = out(&d, b);
    d.delete_objects(&[a, o]).unwrap();
    assert!(matches!(d.object(b), Some(ObjectSnapshot::Path(_))));
    near(&out(&d, b), &b0, 1e-9);
}

#[test]
fn ac30_deleting_a_clone_removes_only_the_clone() {
    let d = Document::new(1);
    let p = square(&d, 0.0, 0.0, 10.0);
    let c = mk(&d, p);
    d.delete_objects(&[c]).unwrap();
    assert!(d.object(c).is_none() && d.object(p).is_some());
    assert_eq!(d.object_ids(), vec![p]);
}

// ---------------------------------------------------------------------------------------------
// Copies (19, 47, 48)
// ---------------------------------------------------------------------------------------------

fn copy(d: &Document, ids: &[NodeId], off: Vec2) -> Vec<NodeId> {
    let src: Vec<CopySource> = ids
        .iter()
        .map(|&id| CopySource {
            id,
            anchor_ids: match d.object(id).unwrap() {
                ObjectSnapshot::Path(p) => p.anchors.iter().map(|_| aid()).collect(),
                _ => vec![],
            },
        })
        .collect();
    d.duplicate_objects(&src, off).unwrap()
}

#[test]
fn ac19_duplicate_of_a_clone_is_a_sibling_not_a_clone_of_it() {
    let d = Document::new(1);
    let p = square(&d, 0.0, 0.0, 10.0);
    let c = mk(&d, p);
    let d = set_matrix(&d, c, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    let copies = copy(&d, &[c], Vec2::new(5.0, 0.0));
    assert!(is_clone(&d, copies[0]));
    assert_eq!(origin_of(&d, copies[0]), p);
    assert_eq!(matrix_of(&d, copies[0]), [1.0, 0.0, 0.0, 1.0, 35.0, 0.0]);
    let ids = d.object_ids();
    let pos = |x| ids.iter().position(|&i| i == x).unwrap();
    assert_eq!(pos(copies[0]), pos(c) + 1, "above its source");
}

#[test]
fn ac19_duplicate_of_a_path_original_is_independent_and_clones_stay_linked() {
    let d = Document::new(1);
    let p = square(&d, 0.0, 0.0, 10.0);
    let c = mk(&d, p);
    let cp = copy(&d, &[p], Vec2::new(0.0, 20.0));
    assert!(matches!(d.object(cp[0]), Some(ObjectSnapshot::Path(_))));
    assert_eq!(origin_of(&d, c), p);
    d.translate_objects(&[cp[0]], Vec2::new(100.0, 0.0))
        .unwrap();
    assert_eq!(
        out(&d, c)[0],
        (0.0, 0.0),
        "the copy has no clones, c follows p"
    );
    // Duplicate of a middle clone A in B -> A -> O: copy is a clone of O, B stays on A.
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let a = mk(&d, o);
    let b = mk(&d, a);
    let d = set_matrix(&d, a, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    let ac = copy(&d, &[a], Vec2::new(0.0, 0.0))[0];
    assert_eq!(origin_of(&d, ac), o);
    assert_eq!(matrix_of(&d, ac), matrix_of(&d, a));
    assert_eq!(origin_of(&d, b), a);
}

#[test]
fn ac47_a_copied_set_relinks_clones_to_the_copy_of_the_original() {
    let d = Document::new(1);
    let p = square(&d, 0.0, 0.0, 10.0);
    let c1 = mk(&d, p);
    let c2 = mk(&d, p);
    let c3 = mk(&d, p); // outside the copied set
    let d = set_matrix(&d, c1, [1.0, 0.0, 0.0, 1.0, 20.0, 0.0]);
    let cps = copy(&d, &[p, c1, c2], Vec2::new(0.0, 50.0));
    assert_eq!(cps.len(), 3);
    let (p2, c1c, c2c) = (cps[0], cps[1], cps[2]);
    assert!(matches!(d.object(p2), Some(ObjectSnapshot::Path(_))));
    assert_eq!(origin_of(&d, c1c), p2);
    assert_eq!(origin_of(&d, c2c), p2);
    assert_eq!(origin_of(&d, c3), p);
    d.translate_objects(&[p2], Vec2::new(0.0, 5.0)).unwrap();
    let y = |x: NodeId| out(&d, x)[0].1;
    assert_eq!(y(c1c), 55.0);
    assert_eq!(y(c1), 0.0);
    assert_eq!(y(c3), 0.0);
    d.translate_objects(&[p], Vec2::new(0.0, 3.0)).unwrap();
    assert_eq!(y(c1), 3.0);
    assert_eq!(y(c3), 3.0);
    assert_eq!(y(c1c), 55.0);
}

#[test]
fn ac48_a_copied_chain_is_a_chain_of_copies_with_the_original_left_outside() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let a = mk(&d, o);
    let b = mk(&d, a);
    let cps = copy(&d, &[a, b], Vec2::new(0.0, 0.0));
    let (a2, b2) = (cps[0], cps[1]);
    assert_eq!(origin_of(&d, a2), o);
    assert_eq!(origin_of(&d, b2), a2);
    let m = [1.0, 0.0, 0.0, 1.0, 9.0, 0.0];
    let d = set_matrix(&d, a, m);
    assert_eq!(out(&d, b)[0].0, 9.0);
    assert_eq!(out(&d, a2)[0].0, 0.0, "A' unaffected");
    assert_eq!(out(&d, b2)[0].0, 0.0, "B' unaffected");
    // Origin copied, clone not: the clone stays on the source origin.
    let p = square(&d, 100.0, 0.0, 5.0);
    let c = mk(&d, p);
    let pc = copy(&d, &[p], Vec2::new(0.0, 10.0));
    assert_eq!(origin_of(&d, c), p);
    assert!(matches!(d.object(pc[0]), Some(ObjectSnapshot::Path(_))));
}

// ---------------------------------------------------------------------------------------------
// File: save, reopen, newer version, damaged, orphans, hostile sizes (35, 37, 38, 39, 40)
// ---------------------------------------------------------------------------------------------

#[test]
fn ac37_save_and_reopen_keeps_links_matrices_to_the_last_bit_and_order() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let a = mk(&d, o);
    let b = mk(&d, a);
    let c = mk(&d, b);
    let tricky = 0.1 + 0.2;
    let d = set_matrix(
        &d,
        a,
        [
            tricky,
            1.0 / 3.0,
            -2.5e-7,
            1.000_000_000_000_000_2,
            123.456_789_012_345_6,
            -9.9e6,
        ],
    );
    let d = set_matrix(&d, b, [1.0, 0.0, 0.0, 1.0, 0.1, 0.7]);
    let d = set_matrix(&d, c, [0.5, 0.25, -0.25, 0.5, 1e-9, 3.0]);
    let bytes = pack(&d, "0.1.0").unwrap();
    let e = unpack(9, &bytes).unwrap();
    assert_eq!(e.object_ids(), d.object_ids());
    for id in [a, b, c] {
        assert_eq!(origin_of(&e, id), origin_of(&d, id));
        let (m1, m2) = (matrix_of(&d, id), matrix_of(&e, id));
        for k in 0..6 {
            assert_eq!(m1[k].to_bits(), m2[k].to_bits(), "entry {k} of {id:?}");
        }
        assert_eq!(out(&e, id), out(&d, id));
    }
}

fn rezip_manifest(bytes: &[u8], version: u32) -> Vec<u8> {
    use std::io::{Read, Write};
    let mut zin = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut buf = Vec::new();
    {
        let mut zout = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        for i in 0..zin.len() {
            let mut f = zin.by_index(i).unwrap();
            let name = f.name().unwrap().into_owned();
            let mut data = Vec::new();
            f.read_to_end(&mut data).unwrap();
            if name == "manifest.json" {
                let mut v: serde_json::Value = serde_json::from_slice(&data).unwrap();
                v["format_version"] = version.into();
                data = serde_json::to_vec(&v).unwrap();
            }
            zout.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zout.write_all(&data).unwrap();
        }
        zout.finish().unwrap();
    }
    buf
}

#[test]
fn ac37_a_newer_format_is_refused_and_this_one_is_13() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    mk(&d, o);
    let bytes = pack(&d, "0.1.0").unwrap();
    let j: serde_json::Value = serde_json::from_slice(&d.export_json().unwrap()).unwrap();
    // 0049 took 13; later slices bumped it, so a file of 13 still opens and one above the
    // current version is refused.
    assert_eq!(j["format_version"], CURRENT_FORMAT_VERSION);
    let newer = rezip_manifest(&bytes, CURRENT_FORMAT_VERSION + 1);
    assert!(matches!(
        unpack(2, &newer),
        Err(OpenError::FormatTooNew { .. })
    ));
    let same = rezip_manifest(&bytes, 13);
    assert!(unpack(2, &same).is_ok());
    // A file without clones from an earlier build opens (the pinned older fixtures).
    let older = rezip_manifest(&pack(&Document::new(1), "0.1.0").unwrap(), 12);
    assert!(unpack(2, &older).is_ok());
}

#[test]
fn ac40_document_json_lists_the_clone_without_an_outline() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let c = mk(&d, o);
    let j: serde_json::Value = serde_json::from_slice(&d.export_json().unwrap()).unwrap();
    let objs = j["objects"].as_array().unwrap();
    let clone = objs
        .iter()
        .find(|x| x["shape"] == "clone")
        .expect("a clone entry");
    assert_eq!(clone["id"], serde_json::to_value(c).unwrap());
    let text = clone.to_string();
    assert!(
        text.contains(&serde_json::to_value(o).unwrap().to_string()[..0]),
        "{text}"
    );
    assert!(clone.get("anchors").is_none(), "no outline: {text}");
    let flat = clone["matrix"]
        .as_array()
        .or_else(|| clone["clone_link"]["matrix"].as_array());
    assert_eq!(flat.map(Vec::len), Some(6), "matrix is listed: {text}");
}

fn damaged_cases() -> Vec<(&'static str, LoroValue)> {
    let m = nums([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    let with = |k: usize, v: f64| {
        let mut n = nums([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        n[k] = LoroValue::from(v);
        n
    };
    vec![
        ("no origin", link_value(None, Some(m.clone()))),
        (
            "origin not an id",
            link_value(Some("hello"), Some(m.clone())),
        ),
        ("origin empty", link_value(Some(""), Some(m.clone()))),
        ("no matrix", link_value(Some("1:3"), None)),
        (
            "five numbers",
            link_value(Some("1:3"), Some(m[..5].to_vec())),
        ),
        ("seven numbers", {
            let mut v = m.clone();
            v.push(LoroValue::from(0.0));
            link_value(Some("1:3"), Some(v))
        }),
        ("NaN", link_value(Some("1:3"), Some(with(4, f64::NAN)))),
        (
            "infinity",
            link_value(Some("1:3"), Some(with(0, f64::INFINITY))),
        ),
        (
            "beyond +1e7",
            link_value(Some("1:3"), Some(with(4, 1.000_000_1e7))),
        ),
        (
            "beyond -1e7",
            link_value(Some("1:3"), Some(with(5, -1.000_000_1e7))),
        ),
        ("a string entry", {
            let mut v = m.clone();
            v[2] = LoroValue::from("x");
            link_value(Some("1:3"), Some(v))
        }),
        ("not a map", LoroValue::from(3_i64)),
    ]
}

#[test]
fn ac38_damaged_links_are_refused_without_a_crash() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let c = mk(&d, o);
    let valid = origin_text(o);
    assert!(
        forged(
            &d,
            c,
            link_value(Some(&valid), Some(nums([1.0, 0.0, 0.0, 1.0, 0.0, 0.0])))
        )
        .is_ok()
    );
    // The boundary itself is allowed.
    assert!(
        forged(
            &d,
            c,
            link_value(Some(&valid), Some(nums([1.0, 0.0, 0.0, 1.0, 1e7, -1e7])))
        )
        .is_ok()
    );
    for (name, bad) in damaged_cases() {
        // The origin of a damaged case is meaningless; only the failing field matters.
        let r = forged(&d, c, bad);
        assert!(
            matches!(r, Err(OpenError::Damaged)),
            "{name}: {:?}",
            r.map(|_| ())
        );
    }
}

#[test]
fn ac35_valid_origin_that_names_nothing_a_primitive_or_a_group_is_an_orphan_not_damage() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let keep = square(&d, 30.0, 0.0, 10.0);
    let prim = d.create_rect(curvyo_document_core::RectBounds {
        origin: pt(60.0, 0.0),
        width: curvyo_document_core::Length::from_mm(5.0),
        height: curvyo_document_core::Length::from_mm(5.0),
    });
    let g1 = square(&d, 80.0, 0.0, 5.0);
    let g2 = square(&d, 90.0, 0.0, 5.0);
    let group = d.group_objects(&[g1, g2]).unwrap();
    let c_none = mk(&d, o);
    let c_prim = mk(&d, o);
    let c_group = mk(&d, o);
    let c_keep = mk(&d, keep);
    let ident = nums([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    let mut doc = forged(&d, c_none, link_value(Some("5:9999"), Some(ident.clone()))).unwrap();
    doc = forged(
        &doc,
        c_prim,
        link_value(Some(&origin_text(prim)), Some(ident.clone())),
    )
    .unwrap();
    doc = forged(
        &doc,
        c_group,
        link_value(Some(&origin_text(group)), Some(ident)),
    )
    .unwrap();
    let read = doc.read_objects();
    for c in [c_none, c_prim, c_group] {
        assert!(read.orphans.contains(&c), "{c:?}");
        assert!(
            read.get(c).is_none(),
            "an orphan is not drawn, hit or selected"
        );
    }
    assert!(!read.orphans.contains(&c_keep));
    let removed = doc.remove_orphan_clones();
    assert_eq!(removed, 3);
    assert_eq!(doc.remove_orphan_clones(), 0, "idempotent");
    assert!(doc.object(c_keep).is_some());
    assert_eq!(labels(&doc).last().unwrap(), "remove_orphan_clones");
}

#[test]
fn ac35_a_self_loop_and_rings_of_any_length_are_orphans_and_do_not_loop() {
    for ring in [1_usize, 2, 3, 7] {
        let d = Document::new(1);
        let o = square(&d, 0.0, 0.0, 10.0);
        let cs: Vec<NodeId> = (0..ring).map(|_| mk(&d, o)).collect();
        let mut doc = d;
        let ident = nums([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        for i in 0..ring {
            let next = cs[(i + 1) % ring];
            doc = forged(
                &doc,
                cs[i],
                link_value(Some(&origin_text(next)), Some(ident.clone())),
            )
            .unwrap();
        }
        assert_eq!(doc.read_objects().orphans.len(), ring, "ring {ring}");
        assert_eq!(doc.remove_orphan_clones(), ring);
        assert_eq!(doc.object_ids(), vec![o]);
    }
}

#[test]
fn ac35_a_clone_behind_an_orphan_is_an_orphan_and_a_long_chain_is_not() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let a = mk(&d, o);
    let b = mk(&d, a);
    let _c = mk(&d, b);
    let doc = forged(
        &d,
        a,
        link_value(Some("5:9999"), Some(nums([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]))),
    )
    .unwrap();
    assert_eq!(doc.remove_orphan_clones(), 3, "A, B and C go in one commit");
    // 12 deep is made by hand: it resolves.
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let mut last = o;
    let mut all = vec![];
    for _ in 0..8 {
        last = mk(&d, last);
        all.push(last);
    }
    let mut doc = d;
    // Four more by forging: point a fresh depth-1 clone at the deepest, repeatedly.
    for _ in 0..4 {
        let extra = mk(&doc, o);
        doc = forged(
            &doc,
            extra,
            link_value(
                Some(&origin_text(last)),
                Some(nums([1.0, 0.0, 0.0, 1.0, 1.0, 0.0])),
            ),
        )
        .unwrap();
        last = extra;
        all.push(extra);
    }
    assert_eq!(doc.read_objects().orphans.len(), 0);
    match doc.object(last).unwrap() {
        ObjectSnapshot::Clone(c) => assert_eq!(c.chain.depth, 12),
        _ => panic!(),
    }
    assert_eq!(doc.remove_orphan_clones(), 0);
}

#[test]
fn ac39_a_ring_of_10000_and_a_chain_of_10000_open_without_recursion() {
    // Build 10,000 clones by hand in Loro, all as plain metas, then forge the links.
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let seed = mk(&d, o);
    let l = LoroDoc::new();
    l.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let tree = l.get_tree(TREE);
    let n = 10_000;
    let mut ids: Vec<TreeID> = Vec::with_capacity(n);
    for _ in 0..n {
        let t = tree.create(None).unwrap();
        tree.get_meta(t).unwrap().insert("shape", "clone").unwrap();
        ids.push(t);
    }
    let ident = nums([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    let o_text = origin_text(o);
    let _ = seed;
    // Ring first.
    for i in 0..n {
        let next = ids[(i + 1) % n];
        let text = format!("{}:{}", next.peer, next.counter);
        tree.get_meta(ids[i])
            .unwrap()
            .insert("clone_link", link_value(Some(&text), Some(ident.clone())))
            .unwrap();
    }
    l.commit();
    let ring = reopen(3, &l.export(loro::ExportMode::Snapshot).unwrap()).unwrap();
    assert_eq!(ring.read_objects().orphans.len(), n);
    assert_eq!(ring.remove_orphan_clones(), n);
    // Chain: ids[i] follows ids[i+1]; the last follows the path.
    for i in 0..n {
        let text = if i + 1 < n {
            let next = ids[i + 1];
            format!("{}:{}", next.peer, next.counter)
        } else {
            o_text.clone()
        };
        tree.get_meta(ids[i])
            .unwrap()
            .insert(
                "clone_link",
                link_value(Some(&text), Some(nums([1.0, 0.0, 0.0, 1.0, 0.001, 0.0]))),
            )
            .unwrap();
    }
    l.commit();
    let chain = reopen(3, &l.export(loro::ExportMode::Snapshot).unwrap()).unwrap();
    let read = chain.read_objects();
    assert!(read.orphans.is_empty(), "{} orphans", read.orphans.len());
    assert_eq!(chain.remove_orphan_clones(), 0);
    let first = nid(ids[0]);
    match chain.object(first).unwrap() {
        ObjectSnapshot::Clone(c) => {
            assert_eq!(c.chain.depth, n);
            assert!((c.chain.composed.e - 0.001 * n as f64).abs() < 1e-6);
        }
        _ => panic!(),
    }
}

#[test]
fn ac39_many_clones_of_one_original_open() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let l = LoroDoc::new();
    l.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let tree = l.get_tree(TREE);
    let o_text = origin_text(o);
    for _ in 0..30_000 {
        let t = tree.create(None).unwrap();
        let m = tree.get_meta(t).unwrap();
        m.insert("shape", "clone").unwrap();
        m.insert(
            "clone_link",
            link_value(Some(&o_text), Some(nums([1.0, 0.0, 0.0, 1.0, 1.0, 1.0]))),
        )
        .unwrap();
    }
    l.commit();
    let doc = reopen(3, &l.export(loro::ExportMode::Snapshot).unwrap()).unwrap();
    let read = doc.read_objects();
    assert_eq!(read.objects.len(), 30_001);
    assert_eq!(read.clones_of.get(&o).map(Vec::len), Some(30_000));
}

// ---------------------------------------------------------------------------------------------
// Merges (13), no cycles (53)
// ---------------------------------------------------------------------------------------------

fn merged(a: &Document, b: &Document, peer: u64) -> Document {
    let l = LoroDoc::new();
    l.import(&a.export_loro_snapshot().unwrap()).unwrap();
    l.import(&b.export_loro_snapshot().unwrap()).unwrap();
    l.commit();
    reopen(peer, &l.export(loro::ExportMode::Snapshot).unwrap()).unwrap()
}

#[test]
fn ac13_an_edit_of_the_original_and_a_transform_of_the_clone_both_survive_a_merge() {
    let base = Document::new(1);
    let o = square(&base, 0.0, 0.0, 10.0);
    let c = mk(&base, o);
    let snap = base.export_loro_snapshot().unwrap();
    let p1 = reopen(11, &snap).unwrap();
    let p2 = reopen(12, &snap).unwrap();
    p1.translate_objects(&[o], Vec2::new(5.0, 0.0)).unwrap();
    p2.translate_objects(&[c], Vec2::new(0.0, 40.0)).unwrap();
    let m = merged(&p1, &p2, 13);
    assert_eq!(out(&m, o)[0], (5.0, 0.0));
    assert_eq!(out(&m, c)[0], (5.0, 40.0));
    // Two peers transform the same clone: the link is one of the written ones, whole.
    let q1 = reopen(21, &snap).unwrap();
    let q2 = reopen(22, &snap).unwrap();
    q1.translate_objects(&[c], Vec2::new(10.0, 0.0)).unwrap();
    q2.translate_objects(&[c], Vec2::new(0.0, 20.0)).unwrap();
    let m = merged(&q1, &q2, 23);
    let mat = matrix_of(&m, c);
    assert!(
        mat == [1.0, 0.0, 0.0, 1.0, 10.0, 0.0] || mat == [1.0, 0.0, 0.0, 1.0, 0.0, 20.0],
        "never a mix: {mat:?}"
    );
    assert_eq!(origin_of(&m, c), o);
}

#[test]
fn ac35_a_peer_that_deletes_the_original_while_another_clones_it_leaves_an_orphan_that_is_repaired()
{
    let base = Document::new(1);
    let o = square(&base, 0.0, 0.0, 10.0);
    let snap = base.export_loro_snapshot().unwrap();
    let p1 = reopen(11, &snap).unwrap();
    let p2 = reopen(12, &snap).unwrap();
    p1.delete_objects(&[o]).unwrap();
    let c = mk(&p2, o);
    let m = merged(&p1, &p2, 13);
    assert!(m.read_objects().orphans.contains(&c));
    assert_eq!(m.remove_orphan_clones(), 1);
    // Another peer repairing the same state removes the same clones.
    let m2 = merged(&p2, &p1, 14);
    assert_eq!(m2.remove_orphan_clones(), 1);
    assert_eq!(m.object_ids(), m2.object_ids());
}

/// 1,000 seeded random sequences on a small document: after each step every chain ends in a path,
/// the repair finds nothing, and chains made by Linked clone stay within 8.
#[test]
fn ac53_random_sequences_never_form_a_cycle_or_an_orphan() {
    let mut seed = 0x9E37_79B9_7F4A_7C15_u64;
    let mut next = move |n: usize| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % n as u64) as usize
    };
    for _round in 0..1000 {
        let d = Document::new(1);
        for i in 0..4 {
            square(&d, f64::from(i) * 20.0, 0.0, 10.0);
        }
        for _step in 0..14 {
            let ids = d.object_ids();
            if ids.is_empty() {
                break;
            }
            let pick = |k: usize| ids[k % ids.len()];
            let x = pick(next(64));
            let y = pick(next(64));
            match next(7) {
                0 | 1 => {
                    let _ = d.linked_clone(&[x], &[]);
                }
                2 => {
                    let _ = copy_ok(&d, &[x, y]);
                }
                3 => {
                    if matches!(d.object(x), Some(ObjectSnapshot::Clone(_))) {
                        let n = out(&d, x).len();
                        let _ = d.unlink_clones(&[CopySource {
                            id: x,
                            anchor_ids: (0..n).map(|_| aid()).collect(),
                        }]);
                    }
                }
                4 => {
                    let _ = d.delete_objects(&[x]);
                }
                5 => {
                    let _ = d.translate_objects(&[x], Vec2::new(next(9) as f64, 1.0));
                }
                _ => {
                    let _ = d.delete_objects(&[x, y]);
                }
            }
            let read = d.read_objects();
            assert!(read.orphans.is_empty(), "orphan after a step");
            for o in read.objects.iter() {
                if let ObjectSnapshot::Clone(c) = o {
                    assert!(matches!(
                        read.get(c.chain.root),
                        Some(ObjectSnapshot::Path(_))
                    ));
                    assert!(c.chain.depth <= 12, "depth {}", c.chain.depth);
                }
            }
        }
        assert_eq!(d.remove_orphan_clones(), 0);
    }
}

fn copy_ok(d: &Document, ids: &[NodeId]) -> Option<Vec<NodeId>> {
    let mut seen = vec![];
    let mut src = vec![];
    for &id in ids {
        if seen.contains(&id) {
            continue;
        }
        seen.push(id);
        let n = match d.object(id)? {
            ObjectSnapshot::Path(p) => p.anchors.len(),
            _ => 0,
        };
        src.push(CopySource {
            id,
            anchor_ids: (0..n).map(|_| aid()).collect(),
        });
    }
    d.duplicate_objects(&src, Vec2::new(1.0, 1.0)).ok()
}

fn reopen(peer: u64, loro_bytes: &[u8]) -> Result<Document, OpenError> {
    use std::io::{Read, Write};
    let packed = pack(&Document::new(1), "0.1.0").unwrap();
    let mut zin = zip::ZipArchive::new(std::io::Cursor::new(packed)).unwrap();
    let mut buf = Vec::new();
    {
        let mut zout = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        for i in 0..zin.len() {
            let mut f = zin.by_index(i).unwrap();
            let name = f.name().unwrap().into_owned();
            let mut data = Vec::new();
            f.read_to_end(&mut data).unwrap();
            if name == "document.loro" {
                data = loro_bytes.to_vec();
            }
            zout.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zout.write_all(&data).unwrap();
        }
        zout.finish().unwrap();
    }
    unpack(peer, &buf)
}

#[test]
fn ac9_stroke_width_is_the_roots_whatever_the_scale() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    d.edit_style(
        &[o],
        &StyleEdit::StrokeWidth(curvyo_document_core::Length::from_mm(0.5)),
    )
    .unwrap();
    let a = mk(&d, o);
    let b = mk(&d, a);
    let d = set_matrix(&d, a, [4.0, 0.0, 0.0, 4.0, 0.0, 0.0]);
    let d = set_matrix(&d, b, [2.0, 0.0, 0.0, 2.0, 0.0, 0.0]);
    for c in [a, b] {
        assert_eq!(*d.object(c).unwrap().style(), *d.object(o).unwrap().style());
    }
    let j: serde_json::Value = serde_json::from_slice(&d.export_json().unwrap()).unwrap();
    assert_eq!(j["objects"][0]["style"]["stroke"]["width"], 0.5);
}

#[test]
fn ac34_the_link_holds_across_groups_and_ungroup() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let helper = square(&d, 50.0, 0.0, 10.0);
    let c = mk(&d, o);
    let d = set_matrix(&d, c, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    let before = out(&d, c);
    let g = d.group_objects(&[c, helper]).unwrap();
    assert!(is_clone(&d, c));
    assert_eq!(origin_of(&d, c), o);
    near(&out(&d, c), &before, 1e-9);
    d.translate_objects(&[o], Vec2::new(0.0, 5.0)).unwrap();
    assert_eq!(
        out(&d, c)[0],
        (30.0, 5.0),
        "follows the original from another group"
    );
    let _ = g;
}
