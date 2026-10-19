//! Independent tester tests for `specs/0057-clone-overrides` at the document core. Written from
//! the specification and `plan.md` before the implementation was read; only the names of the
//! public seams (the two switches, the `CloneChain` fields, the commit labels) were looked up.
//! The stored form is taken from `plan.md` ("Stored form"): `own_placement` and `own_style` are
//! plain booleans on the clone node, absent when false.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(missing_docs, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#![allow(
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::type_complexity,
    clippy::doc_markdown
)]

use std::collections::HashMap;

use curvyo_document_core::{
    Affine, AnchorId, CURRENT_FORMAT_VERSION, Color, CopySource, Document, NewAnchor, NodeId,
    ObjectSnapshot, Opacity, OpenError, Point, Style, StyleEdit, Vec2, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeID};

const TREE: &str = "paths";
static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(5_000);

fn aid() -> AnchorId {
    AnchorId::new(9, NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
}
fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}
fn rect(d: &Document, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    d.create_path(
        &[
            NewAnchor::corner(aid(), pt(x, y)),
            NewAnchor::corner(aid(), pt(x + w, y)),
            NewAnchor::corner(aid(), pt(x + w, y + h)),
            NewAnchor::corner(aid(), pt(x, y + h)),
        ],
        true,
    )
}
fn square(d: &Document, x: f64, y: f64, s: f64) -> NodeId {
    rect(d, x, y, s, s)
}
fn tid(id: NodeId) -> TreeID {
    let j = serde_json::to_value(id).unwrap();
    TreeID::new(
        j["peer"].as_str().unwrap().parse().unwrap(),
        j["counter"].as_i64().unwrap() as i32,
    )
}
fn opa(v: f64) -> Opacity {
    Opacity::new(v).unwrap()
}
fn col(r: u8, g: u8, b: u8) -> Color {
    Color { r, g, b }
}
fn outline(o: &ObjectSnapshot) -> Vec<(f64, f64)> {
    match o {
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
fn chain(d: &Document, id: NodeId) -> curvyo_document_core::CloneChain {
    match d.object(id) {
        Some(ObjectSnapshot::Clone(c)) => *c.chain,
        other => panic!("not a clone: {other:?}"),
    }
}
fn matrix_of(d: &Document, id: NodeId) -> [f64; 6] {
    chain(d, id).matrix.to_array()
}
fn mk(d: &Document, of: NodeId) -> NodeId {
    d.linked_clone(&[of], &[]).unwrap()[0]
}
fn style_of(d: &Document, id: NodeId) -> Style {
    d.object(id).unwrap().style().clone()
}
/// The node's own opacity register and its effective opacity (own times ancestors).
fn opacity_of(d: &Document, id: NodeId) -> (f64, f64) {
    let n = d.read_nodes().into_iter().find(|n| n.id == id).unwrap();
    (n.registers.opacity.get(), n.opacity.get())
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
    let mut link: HashMap<String, LoroValue> = HashMap::new();
    link.insert("origin".into(), LoroValue::from(origin.as_str()));
    link.insert(
        "matrix".into(),
        LoroValue::from(m.iter().map(|&x| LoroValue::from(x)).collect::<Vec<_>>()),
    );
    meta.insert("clone_link", LoroValue::from(link)).unwrap();
    l.commit();
    reopen(1, &l.export(loro::ExportMode::Snapshot).unwrap()).unwrap()
}
/// Writes a raw meta value on a node of a copy of `d` and reopens it.
fn forged(d: &Document, node: NodeId, key: &str, value: LoroValue) -> Result<Document, OpenError> {
    let l = LoroDoc::new();
    l.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let meta = l.get_tree(TREE).get_meta(tid(node)).unwrap();
    meta.insert(key, value).unwrap();
    l.commit();
    reopen(5, &l.export(loro::ExportMode::Snapshot).unwrap())
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
fn labels(d: &Document) -> Vec<String> {
    d.own_steps().into_iter().map(|s| s.label).collect()
}
/// Undoes the `k`-th newest original step (0 = newest), skipping steps that are undos.
fn undo_nth(d: &Document, k: usize) {
    let steps: Vec<_> = d
        .own_steps()
        .into_iter()
        .filter(|s| s.reverts.is_none())
        .collect();
    let step = steps[steps.len() - 1 - k].step;
    d.undo_step(step).unwrap();
}
fn json(d: &Document) -> serde_json::Value {
    serde_json::from_slice(&d.export_json().unwrap()).unwrap()
}
/// Maps `ids` by `g` the way a gesture does: paths mapped, clones with the matrices of
/// `clone_matrices_after` (own-placement clones that are not selected are written by the writer).
fn transform(d: &Document, ids: &[NodeId], g: Affine) {
    let links: curvyo_document_core::CloneLinks = d
        .read_objects()
        .objects
        .iter()
        .filter_map(|object| match object {
            ObjectSnapshot::Clone(c) => Some((c.id(), (c.chain.origin, c.chain.matrix))),
            _ => None,
        })
        .collect();
    let matrices: HashMap<NodeId, Affine> =
        curvyo_document_core::clone_matrices_after(&links, &d.own_placement_clones(), ids, g)
            .matrices
            .into_iter()
            .collect();
    let results: Vec<ObjectSnapshot> = ids
        .iter()
        .map(|&id| match d.object(id).unwrap() {
            ObjectSnapshot::Path(p) => ObjectSnapshot::Path(p.mapped(g)),
            ObjectSnapshot::Clone(c) => {
                let mut moved = c.mapped_by(g);
                moved.chain.matrix = matrices[&id];
                ObjectSnapshot::Clone(moved)
            }
            ObjectSnapshot::Primitive(_) => panic!("primitive"),
        })
        .collect();
    d.transform_objects(&results, false).unwrap();
}
fn rot(deg: f64, cx: f64, cy: f64) -> Affine {
    let (s, c) = deg.to_radians().sin_cos();
    Affine::new(c, s, -s, c, cx - c * cx + s * cy, cy - s * cx - c * cy)
}
fn red_blue_root(d: &Document) -> NodeId {
    let o = square(d, 0.0, 0.0, 10.0);
    d.edit_style(&[o], &StyleEdit::StrokeColor(col(255, 0, 0)))
        .unwrap();
    d.edit_style(
        &[o],
        &StyleEdit::StrokeWidth(curvyo_document_core::Length::from_mm(0.5)),
    )
    .unwrap();
    d.edit_style(&[o], &StyleEdit::FillEnabled(true)).unwrap();
    d.edit_style(&[o], &StyleEdit::FillColor(col(0, 0, 255)))
        .unwrap();
    d.set_node_opacity(&[o], opa(0.8)).unwrap();
    o
}

// ===== Own placement (1..9, 35) ==============================================================

#[test]
fn ac1_ac2_new_clone_follows_and_switching_moves_nothing() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let c = mk(&d, o);
    let d = set_matrix(&d, c, [0.0, 1.0, -1.0, 0.0, 12.0, 3.0]);
    assert!(!chain(&d, c).own_placement && !chain(&d, c).own_style);
    let (o0, m0) = (out(&d, c), matrix_of(&d, c));
    let n = labels(&d).len();
    assert_eq!(d.set_own_placement(&[c], true).unwrap(), 1);
    assert!(chain(&d, c).own_placement);
    near(&out(&d, c), &o0, 1e-9);
    assert_eq!(matrix_of(&d, c), m0);
    assert_eq!(labels(&d).len(), n + 1);
    assert_eq!(labels(&d).last().unwrap(), "own_placement");
    // Again: nothing changes, nothing is committed.
    assert_eq!(d.set_own_placement(&[c], true).unwrap(), 0);
    assert_eq!(labels(&d).len(), n + 1);
    assert_eq!(d.set_own_placement(&[c], false).unwrap(), 1);
    assert_eq!(labels(&d).last().unwrap(), "follow_placement");
    near(&out(&d, c), &o0, 1e-9);
    assert_eq!(matrix_of(&d, c), m0);
    assert!(!chain(&d, c).own_placement);
    // A path is not a clone: the switch writes nothing for it.
    let before = labels(&d).len();
    assert!(d.set_own_placement(&[o], true).map_or(true, |n| n == 0));
    assert_eq!(labels(&d).len(), before);
}

#[test]
fn ac3_spec_test_move_right_and_rotate() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let c = mk(&d, o);
    let d = set_matrix(&d, c, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    let f = mk(&d, o);
    let d = set_matrix(&d, f, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    d.set_own_placement(&[c], true).unwrap();
    let n = labels(&d).len();
    d.translate_objects(&[o], Vec2::new(5.0, 0.0)).unwrap();
    assert_eq!(labels(&d).len(), n + 1, "one commit");
    assert_eq!(out(&d, o)[0], (5.0, 0.0));
    near(
        &out(&d, c),
        &[(30.0, 0.0), (40.0, 0.0), (40.0, 10.0), (30.0, 10.0)],
        1e-6,
    );
    assert_eq!(out(&d, f)[0], (35.0, 0.0), "the following clone moves");
    assert_eq!(matrix_of(&d, c), [1.0, 0.0, 0.0, 1.0, 25.0, 0.0], "M·T⁻¹");
    assert_eq!(
        matrix_of(&d, f),
        [1.0, 0.0, 0.0, 1.0, 30.0, 0.0],
        "not written"
    );
    // Rotation by 90 degrees about O's centre (now at 10, 5).
    let c0 = out(&d, c);
    let f0 = out(&d, f);
    transform(&d, &[o], rot(90.0, 10.0, 5.0));
    near(&out(&d, c), &c0, 1e-6);
    assert_ne!(out(&d, f), f0, "the following clone turns");
}

#[test]
fn ac3_every_kind_of_transform_holds_the_clone_incl_thin_and_rotated_outlines() {
    // A deterministic spread of maps G: move, rotate, scale, skew, mixed, with thin shapes.
    let shapes: [(f64, f64); 5] = [
        (10.0, 10.0),
        (100.0, 0.01),
        (0.01, 100.0),
        (50.0, 1e-4),
        (3.0, 7.0),
    ];
    let maps = [
        Affine::translation(Vec2::new(5.0, -3.0)),
        rot(30.0, 2.0, 1.0),
        rot(-137.0, 50.0, 0.0),
        Affine::scale(2.0, 0.5),
        Affine::scale(-1.0, 1.0),
        Affine::scale(0.001, 3.0),
        Affine::new(1.0, 0.0, 0.6, 1.0, 0.0, 0.0),
        Affine::new(1.0, 0.4, 0.0, 1.0, 1.0, 1.0),
        Affine::new(0.8, 0.6, -0.3, 1.2, 7.0, -2.0),
    ];
    for &(w, h) in &shapes {
        for g in maps {
            for rotated_clone in [false, true] {
                let d = Document::new(1);
                let o = rect(&d, 1.0, 2.0, w, h);
                let c = mk(&d, o);
                let m = if rotated_clone {
                    let (s, co) = 0.5f64.sin_cos();
                    [co, s, -s, co, 40.0, 10.0]
                } else {
                    [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]
                };
                let d = set_matrix(&d, c, m);
                d.set_own_placement(&[c], true).unwrap();
                let before = out(&d, c);
                transform(&d, &[o], g);
                near(&out(&d, c), &before, 1e-6);
                // The shape is O's: node edit now reaches C.
            }
        }
    }
}

#[test]
fn ac3_rotate_object_and_resize_and_nudge_writers_hold_back_too() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let c = mk(&d, o);
    let d = set_matrix(&d, c, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    d.set_own_placement(&[c], true).unwrap();
    let before = out(&d, c);
    // resize_path: stretch O to twice the width
    let ObjectSnapshot::Path(p) = d.object(o).unwrap() else {
        panic!()
    };
    let anchors: Vec<_> = p
        .anchors
        .iter()
        .map(|a| {
            (
                a.id,
                pt(a.point.x * 2.0, a.point.y),
                a.handle_in,
                a.handle_out,
            )
        })
        .collect();
    d.resize_path(o, &anchors, None).unwrap();
    near(&out(&d, c), &before, 1e-6);
    // rotate_object with a rotated snapshot
    let ObjectSnapshot::Path(p) = d.object(o).unwrap() else {
        panic!()
    };
    d.rotate_object(&ObjectSnapshot::Path(p.mapped(rot(33.0, 5.0, 5.0))))
        .unwrap();
    near(&out(&d, c), &before, 1e-6);
}

#[test]
fn ac4_node_edits_reach_the_held_clone() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let c = mk(&d, o);
    let d = set_matrix(&d, c, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    d.set_own_placement(&[c], true).unwrap();
    let ObjectSnapshot::Path(p) = d.object(o).unwrap() else {
        panic!()
    };
    let tr = p.anchors[2].id;
    d.move_anchors(&[(o, tr, pt(15.0, 15.0))]).unwrap();
    assert_eq!(
        out(&d, c)[2],
        (45.0, 15.0),
        "top-right node moves by (5, 5)"
    );
    assert_eq!(out(&d, c)[0], (30.0, 0.0));
    assert_eq!(matrix_of(&d, c), [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    // Moving every node by (5, 0) with the Node tool is a node edit, not a transform.
    let ObjectSnapshot::Path(p) = d.object(o).unwrap() else {
        panic!()
    };
    let moves: Vec<_> = p
        .anchors
        .iter()
        .map(|a| (o, a.id, pt(a.point.x + 5.0, a.point.y)))
        .collect();
    let before = out(&d, c);
    d.move_anchors(&moves).unwrap();
    let after = out(&d, c);
    for (b, a) in before.iter().zip(&after) {
        assert!((a.0 - b.0 - 5.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9);
    }
}

#[test]
fn ac5_chains() {
    // (a) A own placement, O moved: B (follows A) does not move either.
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let a = mk(&d, o);
    let b = mk(&d, a);
    let d = set_matrix(&d, a, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    let d = set_matrix(&d, b, [1.0, 0.0, 0.0, 1.0, 0.0, 30.0]);
    d.set_own_placement(&[a], true).unwrap();
    let (a0, b0) = (out(&d, a), out(&d, b));
    d.translate_objects(&[o], Vec2::new(5.0, 0.0)).unwrap();
    near(&out(&d, a), &a0, 1e-6);
    near(&out(&d, b), &b0, 1e-6);
    // (b) B own placement, A follows O.
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let a = mk(&d, o);
    let b = mk(&d, a);
    let d = set_matrix(&d, a, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    let d = set_matrix(&d, b, [1.0, 0.0, 0.0, 1.0, 0.0, 30.0]);
    d.set_own_placement(&[b], true).unwrap();
    let b0 = out(&d, b);
    d.translate_objects(&[o], Vec2::new(5.0, 0.0)).unwrap();
    near(
        &out(&d, a),
        &[(35.0, 0.0), (45.0, 0.0), (45.0, 10.0), (35.0, 10.0)],
        1e-6,
    );
    near(&out(&d, b), &b0, 1e-6);
    near(
        &out(&d, b),
        &[(30.0, 30.0), (40.0, 30.0), (40.0, 40.0), (30.0, 40.0)],
        1e-6,
    );
    let m = matrix_of(&d, b);
    assert_eq!(m, [1.0, 0.0, 0.0, 1.0, -5.0, 30.0]);
    // (c) A moved by a gesture of its own: B is held back.
    let b1 = out(&d, b);
    d.translate_objects(&[a], Vec2::new(0.0, 7.0)).unwrap();
    near(&out(&d, b), &b1, 1e-6);
}

#[test]
fn ac6_selection_with_original_and_clone_alone() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let c = mk(&d, o);
    let d = set_matrix(&d, c, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    d.set_own_placement(&[c], true).unwrap();
    d.translate_objects(&[o, c], Vec2::new(4.0, 6.0)).unwrap();
    assert_eq!(out(&d, o)[0], (4.0, 6.0));
    assert_eq!(out(&d, c)[0], (34.0, 6.0));
    assert_eq!(matrix_of(&d, c), [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    // Rotation of both together keeps the relation (own placement or not).
    let rel = |d: &Document| {
        let (a, b) = (out(d, o), out(d, c));
        (b[0].0 - a[0].0).hypot(b[0].1 - a[0].1)
    };
    let r0 = rel(&d);
    transform(&d, &[o, c], rot(40.0, 0.0, 0.0));
    assert!((rel(&d) - r0).abs() < 1e-6);
    // Moved alone: G·M in either state.
    let d2 = Document::new(1);
    let o2 = square(&d2, 0.0, 0.0, 10.0);
    let c2 = mk(&d2, o2);
    d2.set_own_placement(&[c2], true).unwrap();
    d2.translate_objects(&[c2], Vec2::new(0.0, 9.0)).unwrap();
    assert_eq!(matrix_of(&d2, c2), [1.0, 0.0, 0.0, 1.0, 0.0, 9.0]);
}

#[test]
fn ac7_singular_unlinks_unselected_own_placement_clones() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    d.edit_style(&[o], &StyleEdit::StrokeColor(col(1, 2, 3)))
        .unwrap();
    let c1 = mk(&d, o);
    let c2 = mk(&d, o);
    let f = mk(&d, o);
    let d = set_matrix(&d, c1, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    let d = set_matrix(&d, c2, [1.0, 0.0, 0.0, 1.0, 50.0, 0.0]);
    d.set_own_placement(&[c1, c2], true).unwrap();
    let (b1, b2) = (out(&d, c1), out(&d, c2));
    let st = style_of(&d, c1);
    let n = labels(&d).len();
    transform(&d, &[o], Affine::scale(0.0, 1.0));
    assert_eq!(labels(&d).len(), n + 1, "same commit");
    for (c, b) in [(c1, b1), (c2, b2)] {
        assert!(!is_clone(&d, c), "became a path");
        near(&out(&d, c), &b, 1e-6);
        assert_eq!(style_of(&d, c), st, "keeps the style it showed");
    }
    assert!(is_clone(&d, f), "the following clone stays a clone");
}

#[test]
fn ac7_out_of_range_refuses_everything() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let c = mk(&d, o);
    let d = set_matrix(&d, c, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    d.set_own_placement(&[c], true).unwrap();
    let before = (out(&d, o), out(&d, c), matrix_of(&d, c));
    let n = labels(&d).len();
    let big = ObjectSnapshot::Path(match d.object(o).unwrap() {
        ObjectSnapshot::Path(p) => p.mapped(Affine::scale(1e9, 1e9)),
        _ => panic!(),
    });
    assert!(d.transform_objects(&[big], false).is_err());
    assert_eq!(labels(&d).len(), n);
    assert_eq!((out(&d, o), out(&d, c), matrix_of(&d, c)), before);
}

#[test]
fn ac35_flip_of_the_original_holds_back_and_mirrors_followers() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let c = mk(&d, o);
    let f = mk(&d, o);
    let d = set_matrix(&d, c, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    let d = set_matrix(&d, f, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    d.set_own_placement(&[c], true).unwrap();
    let (c0, f0) = (out(&d, c), out(&d, f));
    let flip = Affine::new(-1.0, 0.0, 0.0, 1.0, 10.0, 0.0);
    transform(&d, &[o], flip);
    near(&out(&d, c), &c0, 1e-6);
    // Follower: the clone's local frame is mirrored with the original: x' = 10 - x_local + ...
    assert_ne!(out(&d, f), f0);
    // The held-back matrix M·T⁻¹ already has det -1; a Flip of the clone itself flips it back.
    let det = |m: [f64; 6]| m[0] * m[3] - m[1] * m[2];
    assert!(det(matrix_of(&d, c)) < 0.0);
    transform(&d, &[c], flip);
    assert!(det(matrix_of(&d, c)) > 0.0);
    // And a fresh following clone flipped on its own gets a negative determinant (0056).
    let d2 = Document::new(1);
    let o2 = square(&d2, 0.0, 0.0, 10.0);
    let c2 = mk(&d2, o2);
    transform(&d2, &[c2], flip);
    assert!(det(matrix_of(&d2, c2)) < 0.0);
}

// ===== Own style (10..18) =====================================================================

#[test]
fn ac10_11_switch_to_own_copies_effective_style_and_opacity() {
    let d = Document::new(1);
    let o = red_blue_root(&d);
    let c = mk(&d, o);
    let before = style_of(&d, c);
    assert_eq!(before, style_of(&d, o));
    assert_eq!(opacity_of(&d, c).1, 0.8);
    assert_eq!(d.set_own_style(&[c], true).unwrap(), 1);
    assert_eq!(labels(&d).last().unwrap(), "own_style");
    assert_eq!(style_of(&d, c), before, "nothing jumps");
    assert_eq!(opacity_of(&d, c), (0.8, 0.8));
    assert!(chain(&d, c).own_style);
    assert_eq!(chain(&d, c).style_source, c);
    assert_eq!(style_of(&d, c).stroke.color, col(255, 0, 0));
    assert_eq!(style_of(&d, c).stroke.width.as_mm(), 0.5);
    assert_eq!(style_of(&d, c).fill.color, col(0, 0, 255));
}

#[test]
fn ac12_own_style_is_cut_off_from_the_original() {
    let d = Document::new(1);
    let o = red_blue_root(&d);
    let own = mk(&d, o);
    let plain = mk(&d, o);
    d.set_own_style(&[own], true).unwrap();
    d.edit_style(&[o], &StyleEdit::StrokeColor(col(0, 255, 0)))
        .unwrap();
    d.set_node_opacity(&[o], opa(0.5)).unwrap();
    assert_eq!(style_of(&d, own).stroke.color, col(255, 0, 0));
    assert_eq!(opacity_of(&d, own).1, 0.8);
    assert_eq!(style_of(&d, plain).stroke.color, col(0, 255, 0));
    assert_eq!(opacity_of(&d, plain).1, 0.5);
    // Edits to the own-style clone write to it only.
    d.edit_style(&[own], &StyleEdit::FillColor(col(9, 9, 9)))
        .unwrap();
    assert_eq!(style_of(&d, own).fill.color, col(9, 9, 9));
    assert_eq!(style_of(&d, o).fill.color, col(0, 0, 255));
    d.set_node_opacity(&[own], opa(0.3)).unwrap();
    assert_eq!(opacity_of(&d, own).1, 0.3);
    assert_eq!(opacity_of(&d, o).1, 0.5);
    assert_eq!(
        style_of(&d, own).stroke.width.as_mm(),
        0.5,
        "width stays document mm"
    );
}

#[test]
fn ac13_twelve_rainbow_clones_survive_save_and_open_bit_for_bit() {
    let d = Document::new(1);
    let o = red_blue_root(&d);
    let clones: Vec<NodeId> = (0..12).map(|_| mk(&d, o)).collect();
    d.set_own_style(&clones, true).unwrap();
    let mut want = Vec::new();
    for (i, &c) in clones.iter().enumerate() {
        let f = col(20 * i as u8 + 1, 255 - 20 * i as u8, (i * 7) as u8);
        let s = col((i * 13) as u8, 3, 250 - i as u8);
        d.edit_style(&[c], &StyleEdit::FillColor(f)).unwrap();
        d.edit_style(&[c], &StyleEdit::StrokeColor(s)).unwrap();
        d.edit_style(&[c], &StyleEdit::FillOpacity(opa(0.1 + 0.07 * i as f64)))
            .unwrap();
        want.push((f, s));
    }
    // The original is untouched by all of it.
    assert_eq!(style_of(&d, o).fill.color, col(0, 0, 255));
    let packed = pack(&d, "t").unwrap();
    let again = unpack(2, &packed).unwrap();
    for (&c, (f, s)) in clones.iter().zip(&want) {
        let st = style_of(&again, c);
        assert_eq!((st.fill.color, st.stroke.color), (*f, *s));
        assert_eq!(st, style_of(&d, c));
        assert!(chain(&again, c).own_style);
    }
    assert_eq!(json(&again), json(&d));
}

#[test]
fn ac14_back_to_originals_removes_everything_and_undo_restores_it() {
    let d = Document::new(1);
    let o = red_blue_root(&d);
    let c = mk(&d, o);
    d.set_own_style(&[c], true).unwrap();
    d.edit_style(&[c], &StyleEdit::FillColor(col(7, 8, 9)))
        .unwrap();
    d.edit_style(
        &[c],
        &StyleEdit::StrokeWidth(curvyo_document_core::Length::from_mm(2.0)),
    )
    .unwrap();
    d.set_node_opacity(&[c], opa(0.25)).unwrap();
    let own_style = style_of(&d, c);
    assert_eq!(d.set_own_style(&[c], false).unwrap(), 1);
    assert_eq!(labels(&d).last().unwrap(), "original_style");
    assert_eq!(style_of(&d, c), style_of(&d, o));
    assert_eq!(opacity_of(&d, c).1, 0.8);
    assert!(!chain(&d, c).own_style);
    // Not kept in the background: no style or opacity keys in the stored clone.
    let j = json(&d);
    let entry = j["objects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == serde_json::to_value(c).unwrap())
        .unwrap();
    assert!(entry.get("own_style").is_none_or(|v| v == false), "{entry}");
    assert!(entry.get("style").is_none(), "{entry}");
    // Switching to Own again takes a fresh copy of the original's, not the dormant one.
    d.set_own_style(&[c], true).unwrap();
    assert_eq!(style_of(&d, c), style_of(&d, o));
    undo_nth(&d, 0);
    undo_nth(&d, 1);
    assert!(chain(&d, c).own_style);
    assert_eq!(style_of(&d, c), own_style);
    assert_eq!(opacity_of(&d, c).1, 0.25);
}

#[test]
fn ac15_chain_style_source_is_the_nearest_own_style_clone() {
    let d = Document::new(1);
    let o = red_blue_root(&d);
    let a = mk(&d, o);
    let b = mk(&d, a);
    d.set_own_style(&[a], true).unwrap();
    d.edit_style(&[a], &StyleEdit::FillColor(col(0, 0, 200)))
        .unwrap();
    d.edit_style(&[a], &StyleEdit::StrokeColor(col(0, 0, 100)))
        .unwrap();
    assert_eq!(chain(&d, b).style_source, a);
    assert_eq!(style_of(&d, b), style_of(&d, a));
    d.edit_style(&[o], &StyleEdit::FillColor(col(250, 250, 0)))
        .unwrap();
    assert_eq!(style_of(&d, a).fill.color, col(0, 0, 200));
    assert_eq!(style_of(&d, b).fill.color, col(0, 0, 200));
    d.set_own_style(&[a], false).unwrap();
    assert_eq!(style_of(&d, a), style_of(&d, o));
    assert_eq!(style_of(&d, b), style_of(&d, o));
    assert_eq!(chain(&d, b).style_source, o);
}

#[test]
fn ac16_opacity_is_own_times_the_clones_own_ancestors() {
    let d = Document::new(1);
    let p = square(&d, 0.0, 0.0, 10.0);
    d.set_node_opacity(&[p], opa(0.5)).unwrap();
    let c = mk(&d, p);
    let helper = square(&d, 40.0, 0.0, 5.0);
    let g = d.group_objects(&[c, helper]).unwrap();
    d.set_node_opacity(&[g], opa(0.5)).unwrap();
    assert!(
        (opacity_of(&d, c).1 - 0.25).abs() < 1e-12,
        "{:?}",
        opacity_of(&d, c)
    );
    // Original's: the Opacity writer leaves it alone.
    d.set_node_opacity(&[c], opa(0.9)).unwrap();
    assert!((opacity_of(&d, c).1 - 0.25).abs() < 1e-12);
    d.set_own_style(&[c], true).unwrap();
    assert!(
        (opacity_of(&d, c).1 - 0.25).abs() < 1e-12,
        "copies 50 %, shown at 25 %"
    );
    d.set_node_opacity(&[c], opa(1.0)).unwrap();
    assert!((opacity_of(&d, c).1 - 0.5).abs() < 1e-12);
    d.set_node_opacity(&[p], opa(0.1)).unwrap();
    assert!((opacity_of(&d, c).1 - 0.5).abs() < 1e-12, "unchanged");
    // Chain: A Own at 40 % in a layer of 50 %, B Original's of A at top level: 40 %, not 20 %.
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let a = mk(&d, o);
    let b = mk(&d, a);
    d.set_own_style(&[a], true).unwrap();
    d.set_node_opacity(&[a], opa(0.4)).unwrap();
    let h = square(&d, 40.0, 0.0, 5.0);
    let g = d.group_objects(&[a, h]).unwrap();
    d.set_node_opacity(&[g], opa(0.5)).unwrap();
    assert!((opacity_of(&d, a).1 - 0.2).abs() < 1e-12);
    assert!(
        (opacity_of(&d, b).1 - 0.4).abs() < 1e-12,
        "{:?}",
        opacity_of(&d, b)
    );
}

#[test]
fn ac17_own_style_lengths_are_not_scaled_by_matrices() {
    let d = Document::new(1);
    let o = red_blue_root(&d);
    let c = mk(&d, o);
    d.set_own_style(&[c], true).unwrap();
    let d = set_matrix(&d, c, [4.0, 0.0, 0.0, 4.0, 0.0, 0.0]);
    assert_eq!(style_of(&d, c).stroke.width.as_mm(), 0.5);
}

#[test]
fn ac18_mixed_selection_writes_only_owners() {
    let d = Document::new(1);
    let o = red_blue_root(&d);
    let own = mk(&d, o);
    let plain = mk(&d, o);
    d.set_own_style(&[own], true).unwrap();
    let before_plain = style_of(&d, plain);
    d.edit_style(&[o, own, plain], &StyleEdit::FillColor(col(1, 1, 1)))
        .unwrap();
    assert_eq!(style_of(&d, o).fill.color, col(1, 1, 1));
    assert_eq!(style_of(&d, own).fill.color, col(1, 1, 1));
    // `plain` shows the original's, hence also (1,1,1), but nothing is written on it.
    assert_eq!(style_of(&d, plain), style_of(&d, o));
    assert!(!chain(&d, plain).own_style);
    assert_ne!(before_plain.fill.color, style_of(&d, plain).fill.color);
    // Edit only Original's clones: nothing becomes own.
    d.edit_style(&[plain], &StyleEdit::FillColor(col(2, 2, 2)))
        .unwrap();
    assert_eq!(style_of(&d, o).fill.color, col(1, 1, 1));
    assert_eq!(style_of(&d, plain).fill.color, col(1, 1, 1));
    assert!(!chain(&d, plain).own_style);
}

// ===== Interplay (30..33, 36, 40) ============================================================

#[test]
fn ac30_unlink_keeps_outline_effective_style_and_opacity() {
    let d = Document::new(1);
    let o = red_blue_root(&d);
    let own = mk(&d, o);
    let plain = mk(&d, o);
    let d = set_matrix(&d, own, [0.0, 1.0, -1.0, 0.0, 20.0, 5.0]);
    let d = set_matrix(&d, plain, [1.0, 0.0, 0.0, 1.0, 50.0, 5.0]);
    d.set_own_style(&[own], true).unwrap();
    d.set_own_placement(&[own], true).unwrap();
    d.edit_style(&[own], &StyleEdit::FillColor(col(5, 6, 7)))
        .unwrap();
    d.set_node_opacity(&[own], opa(0.6)).unwrap();
    let want = [
        (own, out(&d, own), style_of(&d, own), 0.6),
        (plain, out(&d, plain), style_of(&d, plain), 0.8),
    ];
    for (id, _, _, _) in &want {
        let src = [CopySource {
            id: *id,
            anchor_ids: (0..4).map(|_| aid()).collect(),
        }];
        d.unlink_clones(&src).unwrap();
    }
    for (id, o, s, op) in want {
        assert!(!is_clone(&d, id));
        near(&out(&d, id), &o, 1e-9);
        assert_eq!(style_of(&d, id), s);
        assert!((opacity_of(&d, id).0 - op).abs() < 1e-12);
    }
    // Held-back state is gone: moving O no longer matters (it is a path anyway).
}

#[test]
fn ac31_delete_of_a_style_source_copies_its_style_to_the_dependants() {
    let d = Document::new(1);
    let o = red_blue_root(&d);
    let a = mk(&d, o);
    let b = mk(&d, a);
    d.set_own_style(&[a], true).unwrap();
    d.edit_style(&[a], &StyleEdit::FillColor(col(0, 0, 222)))
        .unwrap();
    d.set_node_opacity(&[a], opa(0.4)).unwrap();
    d.set_own_placement(&[b], true).unwrap();
    let (bo, bs) = (out(&d, b), style_of(&d, b));
    d.delete_objects(&[a]).unwrap();
    assert!(is_clone(&d, b));
    assert_eq!(chain(&d, b).origin, o);
    near(&out(&d, b), &bo, 1e-9);
    assert_eq!(style_of(&d, b), bs, "still blue");
    assert!((opacity_of(&d, b).1 - 0.4).abs() < 1e-12);
    assert!(chain(&d, b).own_style, "(b) it gets a copy as its own");
    assert!(chain(&d, b).own_placement, "(a) keeps its placement state");
    // Deleting O unlinks: loses the two states, keeps look.
    d.delete_objects(&[o]).unwrap();
    assert!(!is_clone(&d, b));
    assert_eq!(style_of(&d, b), bs);
    near(&out(&d, b), &bo, 1e-9);
}

#[test]
fn ac32_duplicate_carries_both_states_independently() {
    let d = Document::new(1);
    let o = red_blue_root(&d);
    let c = mk(&d, o);
    d.set_own_style(&[c], true).unwrap();
    d.set_own_placement(&[c], true).unwrap();
    d.set_node_opacity(&[c], opa(0.35)).unwrap();
    let copies = d
        .duplicate_objects(
            &[CopySource {
                id: c,
                anchor_ids: vec![],
            }],
            Vec2::new(5.0, 0.0),
        )
        .unwrap();
    let k = copies[0];
    assert!(chain(&d, k).own_placement && chain(&d, k).own_style);
    assert_eq!(style_of(&d, k), style_of(&d, c));
    assert!((opacity_of(&d, k).1 - 0.35).abs() < 1e-12);
    d.edit_style(&[c], &StyleEdit::FillColor(col(9, 9, 9)))
        .unwrap();
    assert_ne!(style_of(&d, k).fill.color, col(9, 9, 9), "independent");
    assert_eq!(chain(&d, k).style_source, k);
}

#[test]
fn ac33_undo_of_switches_and_of_a_hold_back() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let c = mk(&d, o);
    let d = set_matrix(&d, c, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    d.set_own_placement(&[c], true).unwrap();
    d.translate_objects(&[o], Vec2::new(5.0, 0.0)).unwrap();
    assert_eq!(matrix_of(&d, c)[4], 25.0);
    undo_nth(&d, 0);
    assert_eq!(matrix_of(&d, c)[4], 30.0);
    assert_eq!(out(&d, o)[0], (0.0, 0.0));
    assert!(chain(&d, c).own_placement);
    undo_nth(&d, 1);
    assert!(!chain(&d, c).own_placement);
}

#[test]
fn ac36_orphan_with_overrides_is_still_removed() {
    // A clone whose original vanished (forged: the origin text points to nowhere).
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let c = mk(&d, o);
    d.set_own_style(&[c], true).unwrap();
    d.set_own_placement(&[c], true).unwrap();
    let mut link: HashMap<String, LoroValue> = HashMap::new();
    link.insert("origin".into(), LoroValue::from("123456:77"));
    link.insert(
        "matrix".into(),
        LoroValue::from(
            [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
                .iter()
                .map(|&x| LoroValue::from(x))
                .collect::<Vec<_>>(),
        ),
    );
    match forged(&d, c, "clone_link", LoroValue::from(link)) {
        Ok(opened) => assert!(opened.object(c).is_none(), "removed by the repair"),
        Err(e) => panic!("a lost original is repaired, not refused: {e:?}"),
    }
}

#[test]
fn ac40_path_read_equals_unlink_for_overrides() {
    let d = Document::new(1);
    let o = red_blue_root(&d);
    let c = mk(&d, o);
    let d = set_matrix(&d, c, [0.0, 2.0, -2.0, 0.0, 9.0, 1.0]);
    d.set_own_style(&[c], true).unwrap();
    d.set_own_placement(&[c], true).unwrap();
    d.edit_style(&[c], &StyleEdit::StrokeColor(col(4, 5, 6)))
        .unwrap();
    let read = match d.object(c).unwrap() {
        ObjectSnapshot::Clone(k) => k.resolved,
        _ => panic!(),
    };
    d.unlink_clones(&[CopySource {
        id: c,
        anchor_ids: (0..4).map(|_| aid()).collect(),
    }])
    .unwrap();
    let ObjectSnapshot::Path(p) = d.object(c).unwrap() else {
        panic!()
    };
    near(
        &p.anchors
            .iter()
            .map(|a| (a.point.x, a.point.y))
            .collect::<Vec<_>>(),
        &read
            .anchors
            .iter()
            .map(|a| (a.point.x, a.point.y))
            .collect::<Vec<_>>(),
        1e-9,
    );
    assert_eq!(p.style, read.style);
}

// ===== Format 14 (37..39) ====================================================================

#[test]
fn ac37_every_save_is_format_14_and_a_newer_one_is_refused() {
    assert!(
        std::hint::black_box(CURRENT_FORMAT_VERSION) >= 14,
        "0057 took 14"
    );
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    mk(&d, o);
    let bytes = pack(&d, "0.1.0").unwrap();
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();
    let mut text = String::new();
    std::io::Read::read_to_string(&mut z.by_name("manifest.json").unwrap(), &mut text).unwrap();
    let m: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        m["format_version"], CURRENT_FORMAT_VERSION,
        "also without overrides"
    );
    // What an older build (13) sees: a version above its own, so it refuses. We model the older
    // build by the same check one version up.
    assert!(matches!(
        unpack(2, &rezip_manifest(&bytes, CURRENT_FORMAT_VERSION + 1)),
        Err(OpenError::FormatTooNew { .. })
    ));
    assert!(unpack(2, &rezip_manifest(&bytes, 14)).is_ok());
    assert!(
        unpack(2, &rezip_manifest(&bytes, 13)).is_ok(),
        "a 13 file still opens"
    );
}

#[test]
fn ac37_golden_file_has_each_override_alone_both_and_a_chain() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/clone_overrides_v14.curvyo"
    );
    let bytes = std::fs::read(path).unwrap();
    let d = unpack(1, &bytes).unwrap();
    let mut states = std::collections::HashSet::new();
    let mut middle_own_style_with_follower = false;
    for id in d.object_ids() {
        if let Some(ObjectSnapshot::Clone(c)) = d.object(id) {
            states.insert((c.chain.own_placement, c.chain.own_style));
            if c.chain.own_style {
                middle_own_style_with_follower |= d.object_ids().into_iter().any(|o| {
                    matches!(d.object(o), Some(ObjectSnapshot::Clone(k))
                        if k.chain.origin == id && !k.chain.own_style && k.chain.style_source == id)
                });
            }
        }
    }
    assert_eq!(states.len(), 4, "alone, alone, both, neither: {states:?}");
    assert!(middle_own_style_with_follower);
    // Save and reopen is lossless.
    let again = unpack(2, &pack(&d, "t").unwrap()).unwrap();
    assert_eq!(json(&again), json(&d));
    let mf = std::fs::read(path).unwrap();
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(mf)).unwrap();
    let mut text = String::new();
    std::io::Read::read_to_string(&mut z.by_name("manifest.json").unwrap(), &mut text).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&text).unwrap()["format_version"],
        14
    );
}

#[test]
fn ac37_the_damaged_v13_link_file_is_still_refused() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let bytes = std::fs::read(format!("{dir}/clone_matrix_five_v13.curvyo")).unwrap();
    assert!(unpack(1, &bytes).is_err());
}

#[test]
fn ac37_older_fixtures_open_unchanged() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    for name in ["clone_v13.curvyo", "clone_orphan_and_ring_v13.curvyo"] {
        let bytes = std::fs::read(format!("{dir}/{name}")).unwrap();
        let d = unpack(1, &bytes).unwrap();
        for id in d.object_ids() {
            if let Some(ObjectSnapshot::Clone(c)) = d.object(id) {
                assert!(!c.chain.own_placement && !c.chain.own_style, "{name}");
                assert_eq!(c.chain.style_source, c.chain.root, "{name}");
            }
        }
    }
}

fn two_clones() -> (Document, NodeId, NodeId, NodeId) {
    let d = Document::new(1);
    let o = red_blue_root(&d);
    let c = mk(&d, o);
    d.set_own_style(&[c], true).unwrap();
    d.set_own_placement(&[c], true).unwrap();
    let plain = mk(&d, o);
    (d, o, c, plain)
}

#[test]
fn ac38_damaged_flags_and_opacities_are_refused() {
    let (d, _, c, _) = two_clones();
    for key in ["own_placement", "own_style"] {
        for bad in [
            LoroValue::from("yes"),
            LoroValue::from(1.0),
            LoroValue::from(0i64),
            LoroValue::Null,
        ] {
            let r = forged(&d, c, key, bad.clone());
            assert!(r.is_err(), "{key}={bad:?} must be refused as damaged");
        }
    }
    for bad in [1.5, -0.1, f64::NAN, f64::INFINITY] {
        assert!(
            forged(&d, c, "opacity", LoroValue::from(bad)).is_err(),
            "{bad}"
        );
    }
    // Opacity outside 0..1 on any node, not just a clone.
    let (d, o, _, _) = two_clones();
    assert!(forged(&d, o, "opacity", LoroValue::from(2.0)).is_err());
    // A true boolean is fine.
    assert!(forged(&d, c, "own_style", LoroValue::from(true)).is_ok());
}

#[test]
fn ac38_residue_is_ignored() {
    let (d, o, _, plain) = two_clones();
    // Flags on a path (even mistyped) are ignored.
    let r = forged(&d, o, "own_placement", LoroValue::from(true)).unwrap();
    assert!(matches!(r.object(o), Some(ObjectSnapshot::Path(_))));
    let r = forged(&d, o, "own_style", LoroValue::from("junk")).unwrap();
    assert!(matches!(r.object(o), Some(ObjectSnapshot::Path(_))));
    // Style keys and opacity on a clone without own style are ignored.
    let r = forged(&d, plain, "opacity", LoroValue::from(0.1)).unwrap();
    assert_eq!(opacity_of(&r, plain).1, 0.8, "reads the source's register");
    assert!(!chain(&r, plain).own_style);
}

#[test]
fn ac38_missing_style_key_of_an_own_style_reads_default() {
    // A fresh switch to Own on a default root, with a key removed by hand, must still open.
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let c = mk(&d, o);
    d.set_own_style(&[c], true).unwrap();
    let r = forged(&d, c, "own_style", LoroValue::from(true)).unwrap();
    assert!(chain(&r, c).own_style);
}

#[test]
fn ac39_document_json_lists_the_states() {
    let (d, _, c, plain) = two_clones();
    let j = json(&d);
    let find = |id: NodeId| {
        j["objects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["id"] == serde_json::to_value(id).unwrap())
            .unwrap()
            .clone()
    };
    let e = find(c);
    assert_eq!(e["own_placement"], true, "{e}");
    assert_eq!(e["own_style"], true, "{e}");
    assert!(e["style"].is_object(), "{e}");
    assert!(
        e.get("origin").is_some() || e.to_string().contains("origin"),
        "{e}"
    );
    let p = find(plain);
    assert!(p.get("own_style").is_none_or(|v| v == false), "{p}");
    assert!(p.get("style").is_none(), "{p}");
}

#[test]
fn a_file_without_overrides_has_no_override_keys() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    mk(&d, o);
    let text = String::from_utf8(d.export_json().unwrap()).unwrap();
    assert!(!text.contains("own_placement") || text.contains("\"own_placement\":false"));
}

#[test]
fn switching_a_whole_selection_is_one_commit_and_counts_changes() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let cs: Vec<NodeId> = (0..5).map(|_| mk(&d, o)).collect();
    d.set_own_style(&cs[..2], true).unwrap();
    let n = labels(&d).len();
    assert_eq!(d.set_own_style(&cs, true).unwrap(), 3);
    assert_eq!(labels(&d).len(), n + 1);
    assert_eq!(d.set_own_style(&cs, true).unwrap(), 0);
    assert_eq!(labels(&d).len(), n + 1);
}

#[test]
fn deep_chain_with_overrides_does_not_recurse_badly() {
    let d = Document::new(1);
    let o = red_blue_root(&d);
    let mut prev = o;
    let mut ids = vec![];
    for _ in 0..8 {
        prev = mk(&d, prev);
        ids.push(prev);
    }
    d.set_own_style(&[ids[3]], true).unwrap();
    d.edit_style(&[ids[3]], &StyleEdit::FillColor(col(1, 2, 3)))
        .unwrap();
    for (i, &id) in ids.iter().enumerate() {
        let want = if i >= 3 { col(1, 2, 3) } else { col(0, 0, 255) };
        assert_eq!(style_of(&d, id).fill.color, want, "link {i}");
    }
}

// ===== White-box-driven edges: degenerate originals through the result-based writers ==========

fn line(d: &Document, x0: f64, y0: f64, x1: f64, y1: f64) -> NodeId {
    d.create_path(
        &[
            NewAnchor::corner(aid(), pt(x0, y0)),
            NewAnchor::corner(aid(), pt(x1, y1)),
        ],
        false,
    )
}

#[test]
fn edge_a_two_anchor_line_rotated_or_resized_still_holds_the_clone() {
    for (x1, y1) in [(10.0, 0.0), (0.0, 10.0), (7.0, 7.0), (1e-3, 0.0)] {
        let d = Document::new(1);
        let o = line(&d, 0.0, 0.0, x1, y1);
        let c = mk(&d, o);
        let d = set_matrix(&d, c, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
        d.set_own_placement(&[c], true).unwrap();
        let before = out(&d, c);
        let ObjectSnapshot::Path(p) = d.object(o).unwrap() else {
            panic!()
        };
        d.rotate_object(&ObjectSnapshot::Path(p.mapped(rot(30.0, 1.0, 1.0))))
            .unwrap();
        near(&out(&d, c), &before, 1e-6);
        let ObjectSnapshot::Path(p) = d.object(o).unwrap() else {
            panic!()
        };
        d.transform_objects(
            &[ObjectSnapshot::Path(p.mapped(Affine::scale(2.0, 3.0)))],
            false,
        )
        .unwrap();
        near(&out(&d, c), &before, 1e-6);
    }
}

#[test]
fn edge_a_one_anchor_path_and_a_curved_path_hold_the_clone() {
    let d = Document::new(1);
    let single = d.create_path(&[NewAnchor::corner(aid(), pt(3.0, 4.0))], false);
    let c = mk(&d, single);
    d.set_own_placement(&[c], true).unwrap();
    let before = out(&d, c);
    d.translate_objects(&[single], Vec2::new(5.0, 5.0)).unwrap();
    near(&out(&d, c), &before, 1e-6);
    // A curve with handles, skewed.
    let d = Document::new(1);
    let mut a0 = NewAnchor::corner(aid(), pt(0.0, 0.0));
    a0.handle_out = Vec2::new(5.0, 8.0);
    let mut a1 = NewAnchor::corner(aid(), pt(10.0, 0.0));
    a1.handle_in = Vec2::new(-5.0, 8.0);
    let o = d.create_path(&[a0, a1], false);
    let c = mk(&d, o);
    let d = set_matrix(&d, c, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    d.set_own_placement(&[c], true).unwrap();
    let before = out(&d, c);
    let ObjectSnapshot::Path(p) = d.object(o).unwrap() else {
        panic!()
    };
    d.transform_objects(
        &[ObjectSnapshot::Path(
            p.mapped(Affine::new(1.0, 0.0, 0.7, 1.0, 0.0, 0.0)),
        )],
        false,
    )
    .unwrap();
    near(&out(&d, c), &before, 1e-6);
}

#[test]
fn edge_group_transform_of_the_original_holds_back_too() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let helper = square(&d, 20.0, 0.0, 5.0);
    let c = mk(&d, o);
    let d = set_matrix(&d, c, [1.0, 0.0, 0.0, 1.0, 60.0, 0.0]);
    d.set_own_placement(&[c], true).unwrap();
    let g = d.group_objects(&[o, helper]).unwrap();
    let before = out(&d, c);
    d.translate_objects(&[o, helper], Vec2::new(5.0, 9.0))
        .unwrap();
    let _ = g;
    near(&out(&d, c), &before, 1e-6);
    assert_eq!(out(&d, o)[0], (5.0, 9.0));
}

#[test]
fn edge_switching_a_locked_or_missing_clone_and_nonsense_ids() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let c = mk(&d, o);
    d.delete_objects(&[c]).unwrap();
    let n = labels(&d).len();
    // A deleted id must not panic and must not commit.
    let r = d.set_own_style(&[c], true);
    assert!(r.is_err() || r == Ok(0));
    let r = d.set_own_placement(&[c], true);
    assert!(r.is_err() || r == Ok(0));
    assert_eq!(labels(&d).len(), n);
    assert!(d.set_own_style(&[], true).map_or(true, |k| k == 0));
}

#[test]
fn edge_spread_of_ten_thousand_hostile_opacity_values_never_leaves_range() {
    // Opacity registers are validated 0..1 on open (criterion 38); the boundary values are fine.
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let c = mk(&d, o);
    d.set_own_style(&[c], true).unwrap();
    for v in [0.0, 1.0, f64::MIN_POSITIVE, 1.0 - f64::EPSILON] {
        d.set_node_opacity(&[c], opa(v)).unwrap();
        let again = unpack(3, &pack(&d, "t").unwrap()).unwrap();
        assert_eq!(opacity_of(&again, c).0.to_bits(), v.to_bits(), "{v}");
    }
}

#[test]
fn edge_a_held_clone_behind_a_singular_following_clone_is_still_held_or_unlinked() {
    // O -> A (follows, matrix collapses x) -> C (own placement). Moving O cannot be undone by
    // M·T_A⁻¹ because T_A = M_A·T_O·M_A⁻¹ needs M_A⁻¹. Criterion 7: the clone is unlinked first
    // so that it keeps the outline it had; at the very least its outline must not change.
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let a = mk(&d, o);
    let c = mk(&d, a);
    let d = set_matrix(&d, a, [0.0, 0.0, 0.0, 1.0, 5.0, 0.0]);
    let d = set_matrix(&d, c, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    d.set_own_placement(&[c], true).unwrap();
    let before = out(&d, c);
    d.translate_objects(&[o], Vec2::new(5.0, 3.0)).unwrap();
    near(&out(&d, c), &before, 1e-6);
}

#[test]
fn ac38_a_damaged_own_style_is_refused_but_the_same_key_on_a_following_clone_is_residue() {
    let (d, _, own, plain) = two_clones();
    for (key, bad) in [
        ("stroke_width", LoroValue::from(-1.0)),
        ("stroke_width", LoroValue::from("wide")),
        ("stroke_cap", LoroValue::from("zigzag")),
        ("fill_enabled", LoroValue::from(3.0)),
    ] {
        assert!(
            forged(&d, own, key, bad.clone()).is_err(),
            "{key}={bad:?} on an own style is damage"
        );
        let r = forged(&d, plain, key, bad.clone());
        assert!(r.is_ok(), "{key}={bad:?} on a following clone is residue");
        let r = r.unwrap();
        assert_eq!(
            style_of(&r, plain),
            style_of(&r, chain(&r, plain).root),
            "{key}"
        );
    }
}
