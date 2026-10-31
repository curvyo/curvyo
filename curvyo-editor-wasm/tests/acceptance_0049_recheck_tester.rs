//! Re-check 2026-10-15 of `specs/0049-linked-clone` at the editor session: the amended criteria
//! 35/38, the Linked clone button of the Layers toolbar, Duplicate with clones, Offset (0038) with
//! clones, and criteria 11, 14, 16 b/c, 20, 26 that the first pass left to the developer tests.

#![allow(
    dead_code,
    unused_imports,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp
)]
#![allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless
)]
#![allow(missing_docs, clippy::doc_markdown, clippy::needless_pass_by_value)]
#![allow(clippy::assert_is_empty, clippy::bool_assert_comparison)]
#![allow(
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::items_after_statements,
    clippy::type_complexity,
    clippy::redundant_closure_for_method_calls
)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use curvyo_document_core::{
    AnchorId, Color, Document, Length, NewAnchor, NodeId, ObjectSnapshot, Opacity, Point,
    RectBounds, StyleEdit, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, RowPress, Session, Tool};
use curvyo_ui_core::{MoveEntryMode, StyleField};
use loro::{LoroDoc, LoroValue, TreeID};
use serde_json::Value;

static TIME: AtomicU64 = AtomicU64::new(1_000);
static AID: AtomicU64 = AtomicU64::new(50_000);

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn rect_at(d: &Document, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    d.create_rect(RectBounds {
        origin: pt(x, y),
        width: Length::from_mm(w),
        height: Length::from_mm(h),
    })
}

fn path_square(d: &Document, x: f64, y: f64, s: f64) -> NodeId {
    let n = |px, py| {
        NewAnchor::corner(
            AnchorId::new(9, AID.fetch_add(1, Ordering::SeqCst)),
            pt(px, py),
        )
    };
    let id = d.create_path(&[n(x, y), n(x + s, y), n(x + s, y + s), n(x, y + s)], true);
    red_fill(d, &[id]);
    id
}

/// A 30 x 30 path square, top-left at x = 20 + 40 i, y = 20.
fn square(d: &Document, i: usize) -> NodeId {
    path_square(d, 20.0 + 40.0 * i as f64, 20.0, 30.0)
}

fn mid(i: usize) -> Point {
    pt(35.0 + 40.0 * i as f64, 35.0)
}

fn red_fill(d: &Document, ids: &[NodeId]) {
    d.edit_style(ids, &StyleEdit::FillColor(Color { r: 255, g: 0, b: 0 }))
        .unwrap();
    d.edit_style(ids, &StyleEdit::FillEnabled(true)).unwrap();
}

fn open(d: &Document) -> Session {
    let mut s = Session::open(2, &pack(d, "0.1.0").unwrap()).unwrap();
    s.set_tool(Tool::Select);
    s.resize_viewport(1600.0, 1000.0);
    s
}

fn doc_of(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).expect("the session output reopens")
}

fn rid(id: NodeId) -> String {
    serde_json::to_string(&id).unwrap()
}

fn tid(id: NodeId) -> TreeID {
    let j = serde_json::to_value(id).unwrap();
    TreeID::new(
        j["peer"].as_str().unwrap().parse().unwrap(),
        j["counter"].as_i64().unwrap() as i32,
    )
}

fn mk(d: &Document, of: NodeId) -> NodeId {
    d.linked_clone(&[of], &[]).unwrap()[0]
}

/// Rezips `d` with a Loro snapshot in which the matrix of each listed clone is replaced.
fn forge(d: &Document, edits: &[(NodeId, [f64; 6])]) -> Document {
    use std::io::{Read, Write};
    let l = LoroDoc::new();
    l.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let tree = l.get_tree("paths");
    for (clone, m) in edits {
        let meta = tree.get_meta(tid(*clone)).unwrap();
        let origin = match meta.get("clone_link").unwrap() {
            loro::ValueOrContainer::Value(LoroValue::Map(map)) => {
                match map.get("origin").unwrap() {
                    LoroValue::String(s) => s.to_string(),
                    other => panic!("{other:?}"),
                }
            }
            other => panic!("{other:?}"),
        };
        let mut link: HashMap<String, LoroValue> = HashMap::new();
        link.insert("origin".into(), LoroValue::from(origin.as_str()));
        link.insert(
            "matrix".into(),
            LoroValue::from(m.iter().map(|&x| LoroValue::from(x)).collect::<Vec<_>>()),
        );
        meta.insert("clone_link", LoroValue::from(link)).unwrap();
    }
    l.commit();
    let bytes = l.export(loro::ExportMode::Snapshot).unwrap();
    let packed = pack(d, "0.1.0").unwrap();
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
                data.clone_from(&bytes);
            }
            zout.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zout.write_all(&data).unwrap();
        }
        zout.finish().unwrap();
    }
    unpack(1, &buf).unwrap()
}

fn outline_of(o: &ObjectSnapshot) -> Vec<(f64, f64)> {
    match o {
        ObjectSnapshot::Sketch(_) => unreachable!("no sketch in this test"),
        ObjectSnapshot::Image(_) => unreachable!("no image in this test"),
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

fn out(s: &Session, id: NodeId) -> Vec<(f64, f64)> {
    outline_of(&doc_of(s).object(id).expect("exists"))
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

fn is_clone(s: &Session, id: NodeId) -> bool {
    matches!(doc_of(s).object(id), Some(ObjectSnapshot::Clone(_)))
}

fn origin_of(s: &Session, id: NodeId) -> NodeId {
    match doc_of(s).object(id) {
        Some(ObjectSnapshot::Clone(c)) => c.chain.origin,
        other => panic!("not a clone: {other:?}"),
    }
}

fn matrix_of(s: &Session, id: NodeId) -> [f64; 6] {
    match doc_of(s).object(id) {
        Some(ObjectSnapshot::Clone(c)) => {
            let m = c.chain.matrix;
            [m.a, m.b, m.c, m.d, m.e, m.f]
        }
        other => panic!("not a clone: {other:?}"),
    }
}

/// The selected leaves, read from the Layers rows (the Style scope leaves clones out).
fn selected(s: &mut Session) -> Vec<NodeId> {
    layers(s)
        .into_iter()
        .filter(|r| r["mark"] == "selected")
        .map(|r| serde_json::from_value(r["id"].clone()).unwrap())
        .collect()
}

fn hold(s: &mut Session, at: Point, shift: bool, ctrl: bool, alt: bool) {
    s.modifiers_changed(shift, ctrl, alt);
    s.pointer_hover(at, shift, ctrl);
}

fn click(s: &mut Session, p: Point) {
    hold(s, p, false, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}

fn drag(s: &mut Session, a: Point, b: Point) {
    hold(s, a, false, false, false);
    s.pointer_down(a, false);
    hold(
        s,
        pt(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y)),
        false,
        false,
        false,
    );
    hold(s, b, false, false, false);
    s.pointer_up(b, false, false);
}

fn key_with(s: &mut Session, k: &str, ctrl: bool, shift: bool, alt: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        ctrl,
        shift,
        alt,
        time_ms: TIME.fetch_add(1_000, Ordering::SeqCst) as f64,
        ..KeyInput::default()
    })
}

fn ctrl_a(s: &mut Session) {
    assert_eq!(
        key_with(s, "a", true, false, false),
        KeyOutcome::SelectedAll
    );
}

fn undo(s: &mut Session) {
    match key_with(s, "z", true, false, false) {
        KeyOutcome::UndoBusy => {
            let _ = s.undo();
        }
        KeyOutcome::Undo => {}
        other => panic!("Ctrl+Z gave {other:?}"),
    }
}

fn labels(s: &Session) -> Vec<String> {
    let l = LoroDoc::new();
    l.import(&doc_of(s).export_loro_snapshot().unwrap())
        .unwrap();
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

fn colour_at(s: &Session, p: Point) -> Option<[u8; 4]> {
    let list = s.draw_list();
    let mut found = None;
    for t in list.triangles.chunks(3) {
        let [a, b, c] = [t[0].position, t[1].position, t[2].position];
        let d1 = (p.x - b.x) * (a.y - b.y) - (a.x - b.x) * (p.y - b.y);
        let d2 = (p.x - c.x) * (b.y - c.y) - (b.x - c.x) * (p.y - c.y);
        let d3 = (p.x - a.x) * (c.y - a.y) - (c.x - a.x) * (p.y - a.y);
        let neg = d1 < -1e-9 || d2 < -1e-9 || d3 < -1e-9;
        let pos = d1 > 1e-9 || d2 > 1e-9 || d3 > 1e-9;
        if !(neg && pos) {
            found = Some([t[0].color.r, t[0].color.g, t[0].color.b, t[0].color.a]);
        }
    }
    found
}

const EMPTY: Point = Point { x: 900.0, y: 700.0 };

fn drawn(s: &Session, p: Point) -> bool {
    colour_at(s, p) != colour_at(s, EMPTY)
}

fn layers(s: &mut Session) -> Vec<Value> {
    let v: Value = serde_json::from_str(&s.layers_view_json(0, 500)).unwrap();
    v["rows"].as_array().unwrap().clone()
}

fn row(s: &mut Session, id: NodeId) -> Value {
    let want = serde_json::to_value(id).unwrap();
    layers(s)
        .into_iter()
        .find(|r| r["id"] == want)
        .unwrap_or_else(|| panic!("no row for {id:?}"))
}

fn clone_count(s: &Session) -> usize {
    let d = doc_of(s);
    d.object_ids()
        .into_iter()
        .filter(|&i| matches!(d.object(i), Some(ObjectSnapshot::Clone(_))))
        .count()
}

// ---------------------------------------------------------------------------------------------
// Making a clone (1 to 6)
fn one_clone(m: [f64; 6]) -> (Document, NodeId, NodeId) {
    let d = Document::new(1);
    let p = path_square(&d, 10.0, 10.0, 10.0);
    let c = mk(&d, p);
    (forge(&d, &[(c, m)]), p, c)
}

fn chain() -> (Document, NodeId, NodeId, NodeId) {
    let d = Document::new(1);
    let o = path_square(&d, 10.0, 10.0, 30.0);
    let a = mk(&d, o);
    let b = mk(&d, a);
    let d = forge(
        &d,
        &[
            (a, [1.0, 0.0, 0.0, 1.0, 100.0, 0.0]),
            (b, [1.0, 0.0, 0.0, 1.0, 0.0, 100.0]),
        ],
    );
    (d, o, a, b)
}

use curvyo_document_core::{MovePlan, MoveTarget, Vec2};
use curvyo_ui_core::{OffsetDirection, OffsetResultChoice};

fn repack(d: &Document, l: &LoroDoc) -> Vec<u8> {
    use std::io::{Read, Write};
    l.commit();
    let bytes = l.export(loro::ExportMode::Snapshot).unwrap();
    let mut zin = zip::ZipArchive::new(std::io::Cursor::new(pack(d, "0.1.0").unwrap())).unwrap();
    let mut buf = Vec::new();
    {
        let mut zout = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        for i in 0..zin.len() {
            let mut f = zin.by_index(i).unwrap();
            let name = f.name().unwrap().into_owned();
            let mut data = Vec::new();
            f.read_to_end(&mut data).unwrap();
            if name == "document.loro" {
                data.clone_from(&bytes);
            }
            zout.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zout.write_all(&data).unwrap();
        }
        zout.finish().unwrap();
    }
    buf
}

fn snapshot_of(d: &Document) -> LoroDoc {
    let l = LoroDoc::new();
    l.import(&d.export_loro_snapshot().unwrap()).unwrap();
    l
}

fn bounds(v: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    let f = |g: fn(&(f64, f64)) -> f64, pick: fn(f64, f64) -> f64, init: f64| {
        v.iter().map(g).fold(init, pick)
    };
    (
        f(|p| p.0, f64::min, f64::MAX),
        f(|p| p.1, f64::min, f64::MAX),
        f(|p| p.0, f64::max, f64::MIN),
        f(|p| p.1, f64::max, f64::MIN),
    )
}

fn assert_box(v: &[(f64, f64)], want: (f64, f64, f64, f64), what: &str) {
    let b = bounds(v);
    let ok = (b.0 - want.0).abs() < 1e-6
        && (b.1 - want.1).abs() < 1e-6
        && (b.2 - want.2).abs() < 1e-6
        && (b.3 - want.3).abs() < 1e-6;
    assert!(ok, "{what}: got {b:?}, want {want:?}");
}

fn toolbar(s: &mut Session) -> Value {
    let v: Value = serde_json::from_str(&s.layers_view_json(0, 500)).unwrap();
    v["toolbar"].clone()
}

fn layer_names(s: &mut Session) -> Vec<String> {
    layers(s)
        .iter()
        .filter(|r| r["kind"] == "layer")
        .map(|r| r["name"].as_str().unwrap_or_default().to_owned())
        .collect()
}

// ---------------------------------------------------------------------------------------------
// 35 / 38 amended: a clone with no link is an orphan
// ---------------------------------------------------------------------------------------------

#[test]
fn ac35_38_open_says_both_sentences_for_deleted_and_converted_orphans() {
    let d = Document::new(1);
    let p = path_square(&d, 10.0, 10.0, 10.0);
    let q = path_square(&d, 50.0, 10.0, 10.0);
    let bare = mk(&d, p);
    let bare2 = mk(&d, p);
    let fill = match d.object(q).unwrap() {
        ObjectSnapshot::Path(x) => x.style.fill.color,
        other => panic!("{other:?}"),
    };
    let l = snapshot_of(&d);
    for b in [bare, bare2] {
        l.get_tree("paths")
            .get_meta(tid(b))
            .unwrap()
            .delete("clone_link")
            .unwrap();
    }
    let meta = l.get_tree("paths").get_meta(tid(q)).unwrap();
    meta.insert("shape", "clone").unwrap();
    let mut s = Session::open(2, &repack(&d, &l)).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    assert_eq!(
        s.take_orphan_notice(),
        "Removed 2 clones whose original was not found. 1 clone became a path."
    );
    let after = doc_of(&s);
    assert_eq!(after.object_ids(), vec![p, q]);
    match after.object(q).unwrap() {
        ObjectSnapshot::Path(x) => assert_eq!(x.style.fill.color, fill),
        other => panic!("{other:?}"),
    }
    assert!(s.is_dirty());
    assert_eq!(labels(&s).last().unwrap(), "remove_orphan_clones");
}

#[test]
fn ac35_38_conversion_only_says_only_the_conversion_sentence() {
    let d = Document::new(1);
    let p = path_square(&d, 10.0, 10.0, 10.0);
    let l = snapshot_of(&d);
    l.get_tree("paths")
        .get_meta(tid(p))
        .unwrap()
        .insert("shape", "clone")
        .unwrap();
    let mut s = Session::open(2, &repack(&d, &l)).unwrap();
    assert_eq!(s.take_orphan_notice(), "1 clone became a path.");
    assert!(
        drawn(
            &{
                let mut s2 = Session::open(2, &repack(&d, &l)).unwrap();
                s2.resize_viewport(1600.0, 1000.0);
                s2
            },
            pt(15.0, 15.0)
        ),
        "the converted path is drawn"
    );
}

// ---------------------------------------------------------------------------------------------
// The Linked clone button of the Layers toolbar, Duplicate and the Initial Layer
// ---------------------------------------------------------------------------------------------

#[test]
fn toolbar_linked_clone_applies_with_the_select_bars_rule_and_never_hides() {
    let d = Document::new(1);
    let a = square(&d, 0);
    let b = square(&d, 1);
    let g = d.group_objects(&[a, b]).unwrap();
    let c = square(&d, 3);
    let mut s = open(&d);
    assert_eq!(toolbar(&mut s)["linkedClone"], false, "nothing selected");
    click(&mut s, mid(3));
    assert_eq!(toolbar(&mut s)["linkedClone"], true, "a path");
    click(&mut s, mid(0));
    // A click on a grouped leaf selects the group: not offered for a group (criterion 4).
    let t = toolbar(&mut s);
    assert_eq!(t["linkedClone"], s.linked_clone_shown());
    s.press_layer_row(&rid(c), RowPress::Plain);
    s.set_tool(Tool::Rectangle);
    assert_eq!(toolbar(&mut s)["linkedClone"], false, "another tool");
    s.set_tool(Tool::Select);
    s.press_layer_row(&rid(c), RowPress::Plain);
    assert_eq!(toolbar(&mut s)["linkedClone"], true);
    // A clone selected: still offered (a clone of a clone).
    let r = s.linked_clone();
    assert_eq!(r.code(), "done");
    assert_eq!(toolbar(&mut s)["linkedClone"], true);
    let _ = g;
}

#[test]
fn toolbar_linked_clone_never_creates_an_initial_layer() {
    let d = Document::new(1);
    let _p = square(&d, 0);
    let mut s = open(&d);
    assert!(layer_names(&mut s).is_empty(), "the file has no layer");
    click(&mut s, mid(0));
    let n = s.linked_clone();
    assert_eq!(n.code(), "done");
    assert!(
        layer_names(&mut s).is_empty(),
        "Linked clone made a layer: {:?}",
        layer_names(&mut s)
    );
    assert_eq!(doc_of(&s).object_ids().len(), 2);
    // Duplicate in the Layers toolbar does not create one either.
    let _ = s.duplicate_layer_target();
    assert!(layer_names(&mut s).is_empty(), "{:?}", layer_names(&mut s));
    assert_eq!(doc_of(&s).object_ids().len(), 3);
    // Linked clone from a drawing tool is not offered.
    s.set_tool(Tool::Pen);
    assert!(!s.linked_clone_shown());
    assert_eq!(s.linked_clone().code(), "ignored");
    assert!(layer_names(&mut s).is_empty());
}

#[test]
fn toolbar_linked_clone_inside_a_layer_stays_in_that_layer_above_the_original() {
    let d = Document::new(1);
    let layer = d.new_layer(None);
    let p = square(&d, 0);
    d.move_nodes(&MovePlan {
        nodes: vec![p],
        target: MoveTarget::FrontOf(Some(layer)),
    })
    .ok();
    let mut s = open(&d);
    s.press_layer_row(&rid(p), RowPress::Plain);
    assert_eq!(s.linked_clone().code(), "done");
    assert_eq!(layer_names(&mut s).len(), 1, "no second layer");
    let rows = layers(&mut s);
    let kinds: Vec<_> = rows
        .iter()
        .map(|r| {
            (
                r["kind"].as_str().unwrap_or("").to_owned(),
                r["depth"].clone(),
            )
        })
        .collect();
    assert!(
        rows.iter()
            .filter(|r| r["kind"] != "layer")
            .all(|r| r["depth"] != 0),
        "both leaves are inside the layer: {kinds:?}"
    );
}

#[test]
fn duplicate_layer_repoints_the_clones_inside_it_to_the_copied_originals() {
    let d = Document::new(1);
    let layer = d.new_layer(None);
    let o = path_square(&d, 10.0, 10.0, 10.0);
    let c = mk(&d, o);
    let d = forge(&d, &[(c, [1.0, 0.0, 0.0, 1.0, 40.0, 0.0])]);
    for n in [o, c] {
        d.move_nodes(&MovePlan {
            nodes: vec![n],
            target: MoveTarget::FrontOf(Some(layer)),
        })
        .unwrap();
    }
    let mut s = open(&d);
    let before = doc_of(&s).object_ids();
    s.press_layer_row(&rid(layer), RowPress::Plain);
    let _ = s.duplicate_layer_target();
    let doc = doc_of(&s);
    let added: Vec<NodeId> = doc
        .object_ids()
        .into_iter()
        .filter(|i| !before.contains(i))
        .collect();
    assert_eq!(added.len(), 2, "{added:?}");
    let copy_clone = *added
        .iter()
        .find(|&&i| matches!(doc.object(i), Some(ObjectSnapshot::Clone(_))))
        .expect("the copy of the clone is a clone");
    let copy_origin = *added.iter().find(|&&i| i != copy_clone).unwrap();
    assert!(matches!(
        doc.object(copy_origin),
        Some(ObjectSnapshot::Path(_))
    ));
    match doc.object(copy_clone).unwrap() {
        ObjectSnapshot::Clone(k) => {
            assert_eq!(
                k.chain.origin, copy_origin,
                "follows the copy, not the source"
            );
            let m = k.chain.matrix;
            assert_eq!(
                [m.a, m.b, m.c, m.d, m.e, m.f],
                [1.0, 0.0, 0.0, 1.0, 40.0, 0.0]
            );
        }
        other => panic!("{other:?}"),
    }
    // The source clone still follows the source original.
    assert_eq!(origin_of(&s, c), o);
    assert_eq!(labels(&s).last().unwrap(), "duplicate_objects");
    undo(&mut s);
    assert_eq!(doc_of(&s).object_ids(), before);
}

#[test]
fn duplicate_layer_holding_only_a_clone_follows_the_same_original() {
    let d = Document::new(1);
    let o = path_square(&d, 10.0, 10.0, 10.0);
    let layer = d.new_layer(None);
    let c = mk(&d, o);
    d.move_nodes(&MovePlan {
        nodes: vec![c],
        target: MoveTarget::FrontOf(Some(layer)),
    })
    .unwrap();
    let mut s = open(&d);
    s.press_layer_row(&rid(layer), RowPress::Plain);
    let _ = s.duplicate_layer_target();
    let doc = doc_of(&s);
    let ids = doc.object_ids();
    assert_eq!(ids.len(), 3);
    let copy = *ids.iter().find(|&&i| i != o && i != c).unwrap();
    assert_eq!(
        origin_of(&s, copy),
        o,
        "same original, not a clone of the clone"
    );
}

#[test]
fn duplicate_of_selected_clone_rows_gives_siblings_above_with_the_same_matrix() {
    let (d, o, a, b) = chain();
    let mut s = open(&d);
    s.press_layer_row(&rid(b), RowPress::Plain);
    let _ = s.duplicate_layer_target();
    let doc = doc_of(&s);
    let ids = doc.object_ids();
    assert_eq!(ids, vec![o, a, b, ids[3]]);
    let copy = ids[3];
    assert_eq!(origin_of(&s, copy), a, "a sibling of b: same original");
    assert_eq!(matrix_of(&s, copy), matrix_of(&s, b));
    assert_eq!(selected(&mut s), vec![copy], "the copy is selected");
    near(&out(&s, copy), &out(&s, b), 1e-9);
    undo(&mut s);
    assert_eq!(doc_of(&s).object_ids(), vec![o, a, b]);
}

// ---------------------------------------------------------------------------------------------
// Offset (0038) with clones
// ---------------------------------------------------------------------------------------------

fn offset_clone_session() -> (Session, NodeId, NodeId) {
    let d = Document::new(1);
    let p = path_square(&d, 10.0, 10.0, 20.0);
    let c = mk(&d, p);
    let d = forge(&d, &[(c, [1.0, 0.0, 0.0, 1.0, 60.0, 0.0])]);
    let mut s = open(&d);
    s.press_layer_row(&rid(c), RowPress::Plain);
    s.set_tool(Tool::Offset);
    (s, p, c)
}

#[test]
fn offset_new_object_next_to_a_clone_takes_the_roots_style() {
    let (mut s, p, c) = offset_clone_session();
    s.set_offset_result(OffsetResultChoice::NewObject);
    s.set_offset_direction(OffsetDirection::Out);
    assert!(s.set_offset_distance(3.0));
    assert!(s.open_offset_chip(), "a clone is an operand");
    let root_style = match doc_of(&s).object(p).unwrap() {
        ObjectSnapshot::Path(x) => x.style.clone(),
        other => panic!("{other:?}"),
    };
    let before = doc_of(&s).object_ids();
    let r = s.apply_offset();
    assert!(
        matches!(
            r,
            curvyo_editor_wasm::OffsetOutcome::Applied {
                added: 1,
                changed: 0,
                ..
            }
        ),
        "{r:?}"
    );
    let doc = doc_of(&s);
    let new: Vec<NodeId> = doc
        .object_ids()
        .into_iter()
        .filter(|i| !before.contains(i))
        .collect();
    assert_eq!(new.len(), 1);
    match doc.object(new[0]).unwrap() {
        ObjectSnapshot::Path(x) => assert_eq!(x.style, root_style, "the root's style"),
        other => panic!("the copy is a path, got {other:?}"),
    }
    assert_box(
        &out(&s, new[0]),
        (67.0, 7.0, 93.0, 33.0),
        "outset by 3 around the clone",
    );
    assert!(is_clone(&s, c), "the clone itself is untouched");
    assert_box(&out(&s, c), (70.0, 10.0, 90.0, 30.0), "clone");
    assert_eq!(origin_of(&s, c), p);
}

#[test]
fn offset_change_original_on_a_clone_turns_it_into_a_path_and_the_root_stays() {
    let d = Document::new(1);
    let p = path_square(&d, 10.0, 10.0, 20.0);
    let c = mk(&d, p);
    let k = mk(&d, c);
    let d = forge(&d, &[(c, [1.0, 0.0, 0.0, 1.0, 60.0, 0.0])]);
    let mut s = open(&d);
    s.press_layer_row(&rid(c), RowPress::Plain);
    s.set_tool(Tool::Offset);
    s.set_offset_result(OffsetResultChoice::ChangeOriginal);
    assert!(s.set_offset_distance(3.0));
    assert!(s.open_offset_chip());
    let r = s.apply_offset();
    assert!(
        matches!(
            r,
            curvyo_editor_wasm::OffsetOutcome::Applied {
                added: 0,
                changed: 1,
                ..
            }
        ),
        "{r:?}"
    );
    assert!(!is_clone(&s, c), "the clone became a path");
    assert_box(&out(&s, c), (67.0, 7.0, 93.0, 33.0), "grown clone");
    assert_box(&out(&s, p), (10.0, 10.0, 30.0, 30.0), "root unchanged");
    assert!(!is_clone(&s, p));
    assert!(is_clone(&s, k), "the clone behind still is one");
    assert_eq!(
        origin_of(&s, k),
        c,
        "and follows the former clone, now a path"
    );
    assert_box(&out(&s, k), (67.0, 7.0, 93.0, 33.0), "the follower");
    let (cs, ps) = match (doc_of(&s).object(c).unwrap(), doc_of(&s).object(p).unwrap()) {
        (ObjectSnapshot::Path(a), ObjectSnapshot::Path(b)) => (a.style, b.style),
        other => panic!("{other:?}"),
    };
    assert_eq!(cs, ps, "the root's style");
    undo(&mut s);
    assert!(is_clone(&s, c), "one step: undo restores the clone");
    assert_box(&out(&s, c), (70.0, 10.0, 90.0, 30.0), "after undo");
}

#[test]
fn offset_that_pushes_a_clone_past_the_limit_is_refused_as_a_whole() {
    let d = Document::new(1);
    let p = path_square(&d, 0.0, 0.0, 10.0);
    let c = mk(&d, p);
    // Clone's right edge at 9_999_995 mm; an outset of 10 mm moves the root edge by 10.
    let d = forge(&d, &[(c, [1.0, 0.0, 0.0, 1.0, 9_999_985.0, 0.0])]);
    let mut s = open(&d);
    s.press_layer_row(&rid(p), RowPress::Plain);
    s.set_tool(Tool::Offset);
    s.set_offset_result(OffsetResultChoice::ChangeOriginal);
    s.set_offset_direction(OffsetDirection::Out);
    assert!(s.set_offset_distance(10.0));
    assert!(s.open_offset_chip());
    let n = labels(&s).len();
    let before = out(&s, p);
    let r = s.apply_offset();
    assert!(
        !matches!(r, curvyo_editor_wasm::OffsetOutcome::Applied { .. }),
        "{r:?}"
    );
    assert_eq!(labels(&s).len(), n, "nothing written");
    assert_eq!(out(&s, p), before);
    let note = s.take_clone_limit_notice();
    assert!(
        note.starts_with("Nothing was changed: Clone of ") && note.ends_with("limit of 10 km."),
        "{note}"
    );
    // A smaller offset stays within the limit and is written.
    assert!(s.set_offset_distance(2.0));
    assert!(matches!(
        s.apply_offset(),
        curvyo_editor_wasm::OffsetOutcome::Applied { .. }
    ));
}

// ---------------------------------------------------------------------------------------------
// 14, 16 b/c, 20, 26, 11
// ---------------------------------------------------------------------------------------------

#[test]
fn ac14_typed_move_rotate_size_and_skew_on_a_clone_keep_it_a_clone() {
    let mk_session = || {
        let (d, p, c) = one_clone([1.0, 0.0, 0.0, 1.0, 40.0, 0.0]);
        let mut s = open(&d);
        s.press_layer_row(&rid(c), RowPress::Plain);
        (s, p, c)
    };
    // Move (M): 12 right, 7 down.
    let (mut s, p, c) = mk_session();
    let before = out(&s, c);
    let root = out(&s, p);
    assert_eq!(
        key_with(&mut s, "m", false, false, false),
        KeyOutcome::EntryOpened
    );
    assert_eq!(
        s.commit_move_entry(
            "12",
            "7",
            curvyo_ui_core::MoveEntryMode {
                absolute: false,
                copy: false
            }
        ),
        curvyo_ui_core::EntryOutcome::Committed
    );
    assert!(is_clone(&s, c));
    assert_eq!(out(&s, p), root, "nothing of the chain is written");
    near(
        &out(&s, c),
        &before
            .iter()
            .map(|&(x, y)| (x + 12.0, y + 7.0))
            .collect::<Vec<_>>(),
        1e-6,
    );
    // Rotate (R) by 90 degrees about the box centre: the box stays a square on the same centre.
    let (mut s, p, c) = mk_session();
    let root = out(&s, p);
    assert_eq!(
        key_with(&mut s, "r", false, false, false),
        KeyOutcome::EntryOpened
    );
    assert_eq!(
        s.commit_transform_entry("90", "", 0),
        curvyo_ui_core::EntryOutcome::Committed
    );
    assert!(is_clone(&s, c));
    assert_eq!(out(&s, p), root);
    assert_box(
        &out(&s, c),
        (50.0, 10.0, 60.0, 20.0),
        "square rotated about its centre",
    );
    let m = matrix_of(&s, c);
    assert!(
        m[1].abs() > 0.99 && m[0].abs() < 1e-9,
        "a quarter turn in the matrix: {m:?}"
    );
    // Size (S): double the width.
    let (mut s, p, c) = mk_session();
    let root = out(&s, p);
    let _ = key_with(&mut s, "s", false, false, false);
    let view = s.transform_entry().expect("size entry");
    let w: f64 = view.fields[0].prefill.parse().unwrap();
    let h: f64 = view.fields[1].prefill.parse().unwrap();
    assert!((w - 10.0).abs() < 1e-6);
    let _ = s.commit_transform_entry(&format!("{}", 2.0 * w), &format!("{h}"), 0);
    assert!(is_clone(&s, c));
    assert_eq!(out(&s, p), root);
    let b = bounds(&out(&s, c));
    assert!(
        (b.2 - b.0 - 20.0).abs() < 1e-6 && (b.3 - b.1 - 10.0).abs() < 1e-6,
        "{b:?}"
    );
    // Skew (K): never converts a clone.
    let (mut s, p, c) = mk_session();
    let root = out(&s, p);
    let r = key_with(&mut s, "k", false, false, false);
    assert_eq!(r, KeyOutcome::EntryOpened, "Skew is offered for a clone");
    let skew_before = out(&s, c);
    let _ = s.commit_transform_entry("20", "", 0);
    assert!(is_clone(&s, c), "no conversion");
    assert_eq!(out(&s, p), root);
    assert_ne!(out(&s, c), skew_before, "the clone is skewed");
}

#[test]
fn ac16b_and_c_chain_selected_with_and_without_the_middle_clone() {
    // (b) O at 10..20; A = clone with translate (30, 0), B = clone of A with translate (0, 30).
    let make = || {
        let d = Document::new(1);
        let o = path_square(&d, 0.0, 0.0, 10.0);
        let a = mk(&d, o);
        let b = mk(&d, a);
        let d = forge(
            &d,
            &[
                (a, [1.0, 0.0, 0.0, 1.0, 30.0, 0.0]),
                (b, [1.0, 0.0, 0.0, 1.0, 0.0, 30.0]),
            ],
        );
        (open(&d), o, a, b)
    };
    let double_width = |s: &mut Session| {
        let _ = key_with(s, "s", false, false, false);
        let v = s.transform_entry().expect("size entry");
        let w: f64 = v.fields[0].prefill.parse().unwrap();
        let h: f64 = v.fields[1].prefill.parse().unwrap();
        let _ = s.commit_transform_entry(&format!("{}", 2.0 * w), &format!("{h}"), 0);
    };
    let (mut s, o, a, b) = make();
    let (a0, b0) = (out(&s, a), out(&s, b));
    s.press_layer_row(&rid(a), RowPress::Plain);
    s.press_layer_row(&rid(b), RowPress::Toggle);
    double_width(&mut s);
    // The group box is x 30..40 (centre 35): x' = 2x - 35.
    let g = |v: &[(f64, f64)]| {
        v.iter()
            .map(|&(x, y)| (2.0 * x - 35.0, y))
            .collect::<Vec<_>>()
    };
    assert_box(&out(&s, o), (0.0, 0.0, 10.0, 10.0), "(b) O is not selected");
    near(&out(&s, a), &g(&a0), 1e-6);
    near(&out(&s, b), &g(&b0), 1e-6);
    assert!(is_clone(&s, a) && is_clone(&s, b));
    assert_eq!(
        matrix_of(&s, b),
        [1.0, 0.0, 0.0, 1.0, 0.0, 30.0],
        "G M G^-1 of a translation along y"
    );
    // (c) O and B selected, A not.
    let (mut s, o, a, b) = make();
    let (o0, b0) = (out(&s, o), out(&s, b));
    s.press_layer_row(&rid(o), RowPress::Plain);
    s.press_layer_row(&rid(b), RowPress::Toggle);
    double_width(&mut s);
    // Group box x 0..10 and 30..40: x 0..40, centre 20: x' = 2x - 20.
    let g = |v: &[(f64, f64)]| {
        v.iter()
            .map(|&(x, y)| (2.0 * x - 20.0, y))
            .collect::<Vec<_>>()
    };
    near(&out(&s, o), &g(&o0), 1e-6);
    near(&out(&s, b), &g(&b0), 1e-6);
    // A is not selected and follows O in its local frame: O' translated by (30, 0).
    let o1 = out(&s, o);
    near(
        &out(&s, a),
        &o1.iter().map(|&(x, y)| (x + 30.0, y)).collect::<Vec<_>>(),
        1e-6,
    );
    assert!(is_clone(&s, a) && is_clone(&s, b));
}

#[test]
fn ac20_click_alt_click_cycle_and_lasso_hit_a_clone_by_its_resolved_outline() {
    let d = Document::new(1);
    let p = path_square(&d, 10.0, 10.0, 30.0);
    let c = mk(&d, p);
    let d = forge(&d, &[(c, [1.0, 0.0, 0.0, 1.0, 100.0, 0.0])]);
    let mut s = open(&d);
    // The interior of the clone, far from its original, hits the clone.
    click(&mut s, pt(125.0, 25.0));
    assert_eq!(selected(&mut s), vec![c]);
    click(&mut s, pt(25.0, 25.0));
    assert_eq!(selected(&mut s), vec![p]);
    click(&mut s, pt(60.0, 25.0));
    assert!(selected(&mut s).is_empty(), "between them nothing is hit");
    // A lasso (Alt at the press) along a line that crosses the clone's edges selects it.
    hold(&mut s, pt(95.0, 25.0), false, false, true);
    s.pointer_down(pt(95.0, 25.0), false);
    for x in [105.0, 120.0, 140.0, 150.0] {
        hold(&mut s, pt(x, 25.0), false, false, true);
    }
    s.pointer_up(pt(150.0, 25.0), false, false);
    hold(&mut s, pt(150.0, 25.0), false, false, false);
    assert_eq!(
        selected(&mut s),
        vec![c],
        "the lasso touched only the clone"
    );
    // Two stacked candidates: the clone sits exactly above a second copy; Alt-click cycles to the lower.
    let d = Document::new(1);
    let p = path_square(&d, 10.0, 10.0, 30.0);
    let c = mk(&d, p);
    let mut s = open(&d);
    click(&mut s, pt(25.0, 25.0));
    assert_eq!(selected(&mut s), vec![c], "the clone is above its original");
    s.modifiers_changed(false, false, true);
    s.pointer_hover(pt(25.0, 25.0), false, false);
    s.pointer_down(pt(25.0, 25.0), false);
    s.pointer_up(pt(25.0, 25.0), false, false);
    s.modifiers_changed(false, false, false);
    assert_eq!(
        selected(&mut s),
        vec![p],
        "Alt-click cycles to the original under it"
    );
}

#[test]
fn ac26_the_pen_does_not_continue_or_connect_a_clone() {
    let d = Document::new(1);
    let line = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(9, 900_001), pt(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(9, 900_002), pt(60.0, 10.0)),
        ],
        false,
    );
    let c = mk(&d, line);
    let d = forge(&d, &[(c, [1.0, 0.0, 0.0, 1.0, 0.0, 50.0])]);
    let mut s = open(&d);
    s.set_tool(Tool::Pen);
    let (root, cl) = (out(&s, line), out(&s, c));
    // Clicks on the clone's end node and on the root's end node.
    for p in [pt(60.0, 60.0), pt(10.0, 60.0)] {
        hold(&mut s, p, false, false, false);
        s.pointer_down(p, false);
        s.pointer_up(p, false, false);
    }
    let _ = s.escape();
    assert!(is_clone(&s, c), "the clone is still a clone");
    assert_eq!(out(&s, c), cl, "its outline is not extended");
    assert_eq!(out(&s, line), root);
    assert_eq!(origin_of(&s, c), line);
}

#[test]
fn ac11_more_than_500_downstream_clones_get_no_drag_preview_and_jump_on_release() {
    let measure = |n: usize| {
        let d = Document::new(1);
        let p = path_square(&d, 10.0, 10.0, 20.0);
        let clones: Vec<NodeId> = (0..n).map(|_| mk(&d, p)).collect();
        let edits: Vec<(NodeId, [f64; 6])> = clones
            .iter()
            .map(|&c| (c, [1.0, 0.0, 0.0, 1.0, 100.0, 0.0]))
            .collect();
        let d = forge(&d, &edits);
        let mut s = open(&d);
        let pre = s.draw_list().triangles.len();
        let a = pt(20.0, 20.0);
        hold(&mut s, a, false, false, false);
        s.pointer_down(a, false);
        hold(&mut s, pt(30.0, 20.0), false, false, false);
        hold(&mut s, pt(40.0, 20.0), false, false, false);
        let mid = s.draw_list().triangles.len();
        let written = labels(&s).len();
        s.pointer_up(pt(40.0, 20.0), false, false);
        let moved = out(&s, clones[0]);
        assert_eq!(labels(&s).len(), written + 1);
        (
            i64::try_from(mid).unwrap() - i64::try_from(pre).unwrap(),
            moved,
        )
    };
    let (delta_500, _) = measure(500);
    let (delta_501, moved) = measure(501);
    assert!(
        delta_500 > delta_501 + 500,
        "with 500 clones the previews add triangles ({delta_500}), with 501 they do not ({delta_501})"
    );
    // On release the clones follow: the first clone's outline moved by the drag (20 mm right).
    assert_box(&moved, (130.0, 10.0, 150.0, 30.0), "clone after release");
}
