//! Re-check 2026-10-15 of `specs/0049-linked-clone` at the document core: criteria 35 and 38 as
//! amended (a clone without a link is an orphan, and the repair turns it into its stored outline
//! when that is a valid path). Written from the amended spec text.

#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
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
    AnchorId, Color, CopySource, Document, NewAnchor, NodeId, ObjectSnapshot, OpenError, Point,
    StyleEdit, Vec2, pack, unpack,
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

fn snapshot_of(d: &Document) -> LoroDoc {
    let l = LoroDoc::new();
    l.import(&d.export_loro_snapshot().unwrap()).unwrap();
    l
}

fn export(l: &LoroDoc) -> Vec<u8> {
    l.commit();
    l.export(loro::ExportMode::Snapshot).unwrap()
}

/// Removes the whole `clone_link` register of `node`: a clone tag with no link at all.
fn without_link(l: &LoroDoc, node: NodeId) {
    l.get_tree(TREE)
        .get_meta(tid(node))
        .unwrap()
        .delete("clone_link")
        .unwrap();
}

/// Makes a plain path node carry the clone tag and no link: merge residue with an outline.
fn tag_as_clone_without_link(l: &LoroDoc, node: NodeId) {
    let meta = l.get_tree(TREE).get_meta(tid(node)).unwrap();
    meta.insert("shape", "clone").unwrap();
    let _ = meta.delete("clone_link");
}

#[test]
fn ac38_a_clone_with_no_link_and_no_outline_is_an_orphan_the_repair_deletes() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let keep = square(&d, 30.0, 0.0, 10.0);
    let c = mk(&d, o);
    let l = snapshot_of(&d);
    without_link(&l, c);
    let doc = reopen(3, &export(&l)).expect("a missing link is not damage");
    let read = doc.read_objects();
    assert!(read.orphans.contains(&c));
    assert!(read.get(c).is_none(), "an orphan is not drawn");
    assert_eq!(doc.remove_orphan_clones(), 1);
    assert_eq!(doc.take_clone_repair().unlinked, 0, "nothing to convert");
    assert_eq!(doc.object_ids(), vec![o, keep]);
    assert_eq!(labels(&doc).last().unwrap(), "remove_orphan_clones");
    assert_eq!(doc.remove_orphan_clones(), 0, "idempotent");
}

#[test]
fn ac38_a_clone_with_no_link_but_a_valid_outline_becomes_that_path_in_place() {
    let d = Document::new(1);
    let a = square(&d, 0.0, 0.0, 10.0);
    let p = square(&d, 30.0, 0.0, 10.0);
    let z = square(&d, 60.0, 0.0, 10.0);
    d.edit_style(&[p], &StyleEdit::FillColor(Color { r: 9, g: 8, b: 7 }))
        .unwrap();
    let before = d.object(p).unwrap();
    let outline_before = out(&d, p);
    let behind = mk(&d, p);
    let l = snapshot_of(&d);
    tag_as_clone_without_link(&l, p);
    let doc = reopen(3, &export(&l)).expect("not damaged");
    let read = doc.read_objects();
    assert!(read.orphans.contains(&p), "an orphan until the repair");
    assert!(
        read.orphans.contains(&behind),
        "and so is the clone behind it"
    );
    assert!(read.get(p).is_none() && read.get(behind).is_none());
    assert_eq!(doc.remove_orphan_clones(), 0, "nothing is deleted");
    assert_eq!(
        doc.take_clone_repair().unlinked,
        1,
        "one conversion is counted"
    );
    assert_eq!(doc.read_objects().orphans, Vec::<NodeId>::new());
    assert_eq!(doc.object(p), Some(before), "same id, outline, style");
    assert_eq!(out(&doc, p), outline_before);
    assert!(!is_clone(&doc, p));
    assert!(
        is_clone(&doc, behind),
        "the clone behind it now follows the path"
    );
    assert_eq!(origin_of(&doc, behind), p);
    assert_eq!(
        doc.object_ids(),
        vec![a, p, behind, z],
        "same place in the stack"
    );
    assert_eq!(labels(&doc).last().unwrap(), "remove_orphan_clones");
    // The converted path is an ordinary path: editing it moves the clone behind it.
    doc.translate_objects(&[p], Vec2::new(5.0, 0.0)).unwrap();
    near(
        &out(&doc, behind),
        &outline_before
            .iter()
            .map(|&(x, y)| (x + 5.0, y))
            .collect::<Vec<_>>(),
        1e-9,
    );
}

#[test]
fn ac38_a_clone_with_no_link_and_an_unreadable_outline_is_deleted() {
    let d = Document::new(1);
    let _a = square(&d, 0.0, 0.0, 10.0);
    let p = square(&d, 30.0, 0.0, 10.0);
    let l = snapshot_of(&d);
    tag_as_clone_without_link(&l, p);
    l.get_tree(TREE)
        .get_meta(tid(p))
        .unwrap()
        .insert("anchors", "not a list")
        .unwrap();
    let doc = reopen(3, &export(&l)).expect("not damaged");
    assert_eq!(doc.remove_orphan_clones(), 1);
    assert!(doc.object(p).is_none());
}

#[test]
fn ac35_repair_of_the_same_state_gives_the_same_result_on_every_peer() {
    let d = Document::new(1);
    let _ = square(&d, 0.0, 0.0, 10.0);
    let p = square(&d, 30.0, 0.0, 10.0);
    let c = mk(&d, p);
    let bare = mk(&d, p);
    let l = snapshot_of(&d);
    tag_as_clone_without_link(&l, p);
    without_link(&l, bare);
    let bytes = export(&l);
    let results: Vec<(usize, usize, Vec<NodeId>)> = [3_u64, 4, 5]
        .iter()
        .map(|&peer| {
            let doc = reopen(peer, &bytes).unwrap();
            let removed = doc.remove_orphan_clones();
            (removed, doc.take_clone_repair().unlinked, doc.object_ids())
        })
        .collect();
    assert!(results.windows(2).all(|w| w[0] == w[1]), "{results:?}");
    assert_eq!(results[0].0, 1, "the bare tag goes");
    assert_eq!(results[0].1, 1, "the tag with an outline becomes a path");
    assert!(results[0].2.contains(&c));
}

#[test]
fn ac38_a_present_but_incomplete_link_is_still_damage() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let c = mk(&d, o);
    for (name, link) in [
        ("origin only", link_value(Some(&origin_text(o)), None)),
        (
            "matrix only",
            link_value(None, Some(nums([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]))),
        ),
    ] {
        let r = forged(&d, c, link);
        assert!(matches!(r, Err(OpenError::Damaged)), "{name}");
    }
}

#[test]
fn ac38_an_empty_link_map_has_no_origin_id_and_is_damage() {
    let d = Document::new(1);
    let o = square(&d, 0.0, 0.0, 10.0);
    let c = mk(&d, o);
    let r = forged(&d, c, link_value(None, None));
    assert!(matches!(r, Err(OpenError::Damaged)));
}
