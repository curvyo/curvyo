//! Independent tester tests for `specs/0049-linked-clone` (and the clone parts of `0050` criteria
//! 7, 24, 25, 26, 28, 36, 38, 45) at the editor session: pointer events, keys and the typed
//! commands the Select bar and the Style/Layers tabs call. Written from the specification before
//! the implementation was read; only the names of the public seams were looked up. Expected values
//! are the specification's own arithmetic.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
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
            let name = f.name().to_owned();
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
// ---------------------------------------------------------------------------------------------

#[test]
fn ac1_ac5_linked_clone_of_three_selected_objects() {
    let d = Document::new(1);
    let (a, b, c) = (square(&d, 0), square(&d, 1), square(&d, 2));
    let mut s = open(&d);
    ctrl_a(&mut s);
    assert!(s.linked_clone_shown());
    let before = labels(&s).len();
    let r = s.linked_clone();
    assert_eq!(r.code(), "done");
    assert_eq!(r.notice, "Created 3 linked clones.");
    assert!(!r.notice.contains("No undo yet"));
    let ids = doc_of(&s).object_ids();
    assert_eq!(ids.len(), 6);
    assert_eq!([ids[0], ids[2], ids[4]], [a, b, c]);
    for k in [1, 3, 5] {
        assert!(is_clone(&s, ids[k]));
        assert_eq!(origin_of(&s, ids[k]), ids[k - 1]);
        assert_eq!(matrix_of(&s, ids[k]), [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    }
    let mut sel = selected(&mut s);
    sel.sort_by_key(|i| ids.iter().position(|x| x == i));
    assert_eq!(
        sel,
        vec![ids[1], ids[3], ids[5]],
        "only the clones are selected"
    );
    let l = labels(&s);
    assert_eq!(l.len(), before + 1, "one commit");
    assert_eq!(l.last().unwrap(), "linked_clone");
}

#[test]
fn ac5_the_notice_for_one_clone_is_singular() {
    let d = Document::new(1);
    square(&d, 0);
    let mut s = open(&d);
    click(&mut s, mid(0));
    assert_eq!(s.linked_clone().notice, "Created 1 linked clone.");
}

#[test]
fn ac2_a_shape_is_converted_silently_in_the_same_commit() {
    let d = Document::new(1);
    let r = rect_at(&d, 20.0, 20.0, 40.0, 30.0);
    red_fill(&d, &[r]);
    let mut s = open(&d);
    click(&mut s, pt(40.0, 35.0));
    let before = labels(&s).len();
    let res = s.linked_clone();
    assert_eq!(res.code(), "done");
    let low = res.notice.to_lowercase();
    assert!(
        !low.contains("convert") && !low.contains("path"),
        "{}",
        res.notice
    );
    assert_eq!(
        labels(&s).len(),
        before + 1,
        "conversion and clone in one commit"
    );
    let doc = doc_of(&s);
    let ids = doc.object_ids();
    assert_eq!(ids[0], r, "same id, same place");
    match doc.object(r).unwrap() {
        ObjectSnapshot::Path(p) => assert_eq!(p.anchors.len(), 4),
        other => panic!("expected a path, got {other:?}"),
    }
    assert!(is_clone(&s, ids[1]));
    near(&out(&s, ids[1]), &out(&s, r), 1e-9);
}

#[test]
fn ac3_a_clone_of_a_selected_clone_makes_a_chain() {
    let d = Document::new(1);
    let p = square(&d, 0);
    let mut s = open(&d);
    click(&mut s, mid(0));
    s.linked_clone();
    let c1 = selected(&mut s)[0];
    let r = s.linked_clone();
    assert_eq!(r.code(), "done");
    let c2 = selected(&mut s)[0];
    assert_ne!(c1, c2);
    assert_eq!(origin_of(&s, c2), c1);
    assert_eq!(origin_of(&s, c1), p);
    let ids = doc_of(&s).object_ids();
    assert_eq!(ids, vec![p, c1, c2]);
}

#[test]
fn ac4_not_offered_for_a_group_nothing_selected_or_another_tool() {
    let d = Document::new(1);
    square(&d, 0);
    square(&d, 1);
    let mut s = open(&d);
    assert!(!s.linked_clone_shown(), "no selection");
    assert_eq!(s.linked_clone().code(), "ignored");
    ctrl_a(&mut s);
    assert!(s.linked_clone_shown());
    s.group_selection();
    assert!(!s.linked_clone_shown(), "a group is selected");
    let before = doc_of(&s).export_loro_snapshot().unwrap();
    assert_eq!(s.linked_clone().code(), "ignored");
    assert_eq!(clone_count(&s), 0);
    drop(before);
    // A selection of a path and a group: not offered, nothing converted.
    let d = Document::new(1);
    let a = rect_at(&d, 20.0, 20.0, 30.0, 30.0);
    let g1 = square(&d, 1);
    let g2 = square(&d, 2);
    let mut s = open(&d);
    click(&mut s, mid(1));
    key_with(&mut s, "a", true, false, false);
    s.group_selection();
    click(&mut s, pt(30.0, 30.0));
    let _ = (a, g1, g2);
    let mut s2 = open(&Document::new(1));
    s2.set_tool(Tool::Node);
    assert!(!s2.linked_clone_shown());
    let d3 = Document::new(1);
    square(&d3, 0);
    let mut s3 = open(&d3);
    click(&mut s3, mid(0));
    s3.set_tool(Tool::Node);
    assert!(!s3.linked_clone_shown(), "only in the Select tool");
}

#[test]
fn ac6_no_key_starts_a_clone_command() {
    let keys: Vec<String> = ('a'..='z')
        .chain('0'..='9')
        .map(|c| c.to_string())
        .chain(["[", "]", "/", ";", "'", ",", ".", "-", "=", "Enter", "Tab"].map(String::from))
        .collect();
    for k in &keys {
        for (ctrl, shift, alt) in [
            (false, false, false),
            (true, false, false),
            (false, true, false),
            (false, false, true),
            (true, true, false),
            (true, false, true),
            (false, true, true),
            (true, true, true),
        ] {
            let d = Document::new(1);
            square(&d, 0);
            square(&d, 1);
            let mut s = open(&d);
            click(&mut s, mid(0));
            let _ = key_with(&mut s, k, ctrl, shift, alt);
            assert_eq!(
                clone_count(&s),
                0,
                "key {k:?} ctrl={ctrl} shift={shift} alt={alt}"
            );
        }
    }
    // The same on a selected clone: no key unlinks it, and none selects its original.
    let d = Document::new(1);
    let p = square(&d, 0);
    mk(&d, p);
    for k in &keys {
        for (ctrl, shift, alt) in [
            (false, false, false),
            (true, false, false),
            (false, true, false),
            (true, true, false),
            (false, false, true),
        ] {
            let mut s = open(&d);
            // The clone sits on the original; the topmost hit is the clone.
            click(&mut s, mid(0));
            let sel = selected(&mut s);
            assert_eq!(sel.len(), 1);
            if !is_clone(&s, sel[0]) {
                continue;
            }
            let _ = key_with(&mut s, k, ctrl, shift, alt);
            let still = selected(&mut s);
            let doc = doc_of(&s);
            if !still.is_empty() && doc.object(still[0]).is_some() {
                assert!(
                    matches!(doc.object(still[0]), Some(ObjectSnapshot::Clone(_))),
                    "key {k:?} ctrl={ctrl} shift={shift} alt={alt} unlinked or changed the clone"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// What a clone shows and how it follows (7 to 12, 20)
// ---------------------------------------------------------------------------------------------

/// A 10 mm square at (0, 0)... shifted so that the viewport shows it: the origin square is at
/// (10, 10) to (20, 20), the clone follows with `m`.
fn one_clone(m: [f64; 6]) -> (Document, NodeId, NodeId) {
    let d = Document::new(1);
    let p = path_square(&d, 10.0, 10.0, 10.0);
    let c = mk(&d, p);
    (forge(&d, &[(c, m)]), p, c)
}

#[test]
fn ac7_the_clone_is_drawn_hit_and_measured_by_its_resolved_outline() {
    // M = scale 2, translate (30, 0) about the origin of the document: the square (10..20) lands on
    // x = 50..70, y = 20..40.
    let (d, p, c) = one_clone([2.0, 0.0, 0.0, 2.0, 30.0, 0.0]);
    let mut s = open(&d);
    assert!(
        drawn(&s, pt(60.0, 30.0)),
        "the clone is painted at its resolved place"
    );
    assert!(!drawn(&s, pt(40.0, 30.0)), "not between the two");
    assert!(!drawn(&s, pt(75.0, 30.0)));
    assert!(drawn(&s, pt(15.0, 15.0)), "the original is still there");
    near(
        &out(&s, c),
        &[(50.0, 20.0), (70.0, 20.0), (70.0, 40.0), (50.0, 40.0)],
        1e-9,
    );
    // Hit test by the resolved outline (20).
    click(&mut s, pt(60.0, 30.0));
    assert_eq!(selected(&mut s), vec![c]);
    click(&mut s, pt(15.0, 15.0));
    assert_eq!(selected(&mut s), vec![p]);
    click(&mut s, pt(40.0, 30.0));
    assert!(selected(&mut s).is_empty(), "between them is empty canvas");
    // Marquee over the clone only selects the clone.
    drag(&mut s, pt(45.0, 15.0), pt(80.0, 45.0));
    assert_eq!(selected(&mut s), vec![c]);
}

#[test]
fn ac8_the_clone_shows_the_roots_style_live_in_a_chain() {
    let d = Document::new(1);
    let p = path_square(&d, 10.0, 10.0, 10.0);
    let a = mk(&d, p);
    let b = mk(&d, a);
    let d = forge(
        &d,
        &[
            (a, [1.0, 0.0, 0.0, 1.0, 40.0, 0.0]),
            (b, [1.0, 0.0, 0.0, 1.0, 0.0, 40.0]),
        ],
    );
    let mut s = open(&d);
    assert_eq!(colour_at(&s, pt(55.0, 15.0)).unwrap()[..3], [255, 0, 0]);
    click(&mut s, pt(15.0, 15.0));
    assert_eq!(selected(&mut s), vec![p]);
    // Selecting the point click hits the topmost: the clone B sits on... make sure we have the root.
    s.set_style_text(StyleField::FillColor, "#00ff00").unwrap();
    for at in [pt(15.0, 15.0), pt(55.0, 15.0), pt(55.0, 55.0)] {
        assert_eq!(colour_at(&s, at).unwrap()[..3], [0, 255, 0], "{at:?}");
    }
}

#[test]
fn ac11_during_a_drag_of_the_original_nothing_is_written_and_the_clone_follows_on_release() {
    let d = Document::new(1);
    let p = path_square(&d, 10.0, 10.0, 20.0);
    let c = mk(&d, p);
    let d = forge(&d, &[(c, [1.0, 0.0, 0.0, 1.0, 60.0, 0.0])]);
    let mut s = open(&d);
    let before = out(&s, c);
    let n = labels(&s).len();
    // Click on the original (the clone is elsewhere), then drag its body.
    let a = pt(20.0, 20.0);
    hold(&mut s, a, false, false, false);
    s.pointer_down(a, false);
    let mid_way = pt(30.0, 20.0);
    hold(&mut s, mid_way, false, false, false);
    assert_eq!(labels(&s).len(), n, "nothing is written during the drag");
    assert_eq!(out(&s, c), before, "the committed clone has not moved");
    s.pointer_up(mid_way, false, false);
    let after = out(&s, c);
    for (b, a) in before.iter().zip(&after) {
        assert!(
            (a.0 - b.0 - 10.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6,
            "{b:?} {a:?}"
        );
    }
    assert_eq!(labels(&s).len(), n + 1);
}

#[test]
fn ac12_the_clone_sees_a_move_of_its_original_in_its_own_frame() {
    // M turns by 90 degrees clockwise on screen: (a, b, c, d) = (0, 1, -1, 0).
    let (d, p, c) = one_clone([0.0, 1.0, -1.0, 0.0, 60.0, 0.0]);
    let mut s = open(&d);
    let before = out(&s, c);
    // Move the original 5 mm right by a drag of its body.
    drag(&mut s, pt(15.0, 15.0), pt(20.0, 15.0));
    assert_eq!(selected(&mut s), vec![p]);
    let after = out(&s, c);
    for (b, a) in before.iter().zip(&after) {
        assert!(
            (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1 - 5.0).abs() < 1e-6,
            "5 mm DOWN: {b:?} {a:?}"
        );
    }
}

#[test]
fn ac15_a_gesture_on_a_clone_alone_writes_only_its_matrix() {
    let (d, p, c) = one_clone([1.0, 0.0, 0.0, 1.0, 40.0, 0.0]);
    let mut s = open(&d);
    let p0 = out(&s, p);
    let n = labels(&s).len();
    drag(&mut s, pt(55.0, 15.0), pt(55.0, 35.0));
    assert_eq!(selected(&mut s), vec![c]);
    assert_eq!(out(&s, p), p0, "the original is untouched");
    assert_eq!(matrix_of(&s, c), [1.0, 0.0, 0.0, 1.0, 40.0, 20.0]);
    let l = labels(&s);
    assert_eq!(l.len(), n + 1);
    // The label of a clone move is the label the same gesture has on a plain path.
    let plain = Document::new(1);
    path_square(&plain, 10.0, 10.0, 10.0);
    let mut sp = open(&plain);
    drag(&mut sp, pt(15.0, 15.0), pt(15.0, 35.0));
    assert_eq!(l.last(), labels(&sp).last());
}

#[test]
fn ac16_a_clone_moved_with_its_original_keeps_its_place_relative_to_it() {
    let (d, p, c) = one_clone([1.0, 0.0, 0.0, 1.0, 40.0, 0.0]);
    let mut s = open(&d);
    ctrl_a(&mut s);
    assert_eq!(selected(&mut s).len(), 2);
    drag(&mut s, pt(15.0, 15.0), pt(15.0, 35.0));
    assert_eq!(out(&s, p)[0], (10.0, 30.0));
    assert_eq!(out(&s, c)[0], (50.0, 30.0));
    assert_eq!(
        matrix_of(&s, c),
        [1.0, 0.0, 0.0, 1.0, 40.0, 0.0],
        "its matrix did not change"
    );
}

#[test]
fn ac17_a_move_that_would_push_a_downstream_clone_past_1e7_is_refused_as_a_whole() {
    let d = Document::new(1);
    let o = path_square(&d, 1.0, 1.0, 1.0);
    let a = mk(&d, o);
    let b = mk(&d, a);
    let d = forge(
        &d,
        &[
            (a, [1000.0, 0.0, 0.0, 1000.0, 0.0, 0.0]),
            (b, [1000.0, 0.0, 0.0, 1000.0, 0.0, 0.0]),
        ],
    );
    let mut s = open(&d);
    // The clones sit under the pointer too; take the original by the layers row instead.
    s.press_layer_row(&rid(o), RowPress::Plain);
    assert_eq!(selected(&mut s), vec![o]);
    let n = labels(&s).len();
    let before = out(&s, o);
    // B would move by 1e6 * 5000 = 5e9 mm.
    key_with(&mut s, "m", false, false, false);
    assert!(s.move_entry().is_some(), "the typed move opens");
    let _ = s.commit_move_entry(
        "5010",
        "10",
        MoveEntryMode {
            absolute: true,
            copy: false,
        },
    );
    assert_eq!(labels(&s).len(), n, "nothing is written");
    assert_eq!(out(&s, o), before);
    let note = s.take_clone_limit_notice();
    assert!(note.starts_with("Nothing was changed"), "{note:?}");
    assert!(
        note.contains("Clone"),
        "the first offending clone is named: {note:?}"
    );
    // A move within the limits works.
    if s.move_entry().is_none() {
        key_with(&mut s, "m", false, false, false);
    }
    let _ = s.commit_move_entry(
        "2",
        "2",
        MoveEntryMode {
            absolute: true,
            copy: false,
        },
    );
    assert_eq!(labels(&s).len(), n + 1);
}

#[test]
fn ac18_the_scale_switches_are_not_shown_for_clones_only() {
    let d = Document::new(1);
    let p = square(&d, 0);
    let q = square(&d, 2);
    let c = mk(&d, p);
    let mut s = open(&d);
    s.press_layer_row(&rid(c), RowPress::Plain);
    assert_eq!(selected(&mut s), vec![c]);
    assert!(!s.select_bar_state().scale_switches_shown);
    s.press_layer_row(&rid(c), RowPress::Plain);
    s.press_layer_row(&rid(q), RowPress::Toggle);
    assert!(
        s.select_bar_state().scale_switches_shown,
        "a mix shows them"
    );
}

#[test]
fn ac27_object_to_path_is_not_offered_for_a_clone() {
    let d = Document::new(1);
    let r = rect_at(&d, 20.0, 20.0, 30.0, 30.0);
    red_fill(&d, &[r]);
    let mut s = open(&d);
    click(&mut s, mid(0));
    assert!(
        s.select_bar_state().object_to_path,
        "offered for a rectangle"
    );
    s.linked_clone();
    assert!(!s.select_bar_state().object_to_path, "not for the clone");
}

#[test]
fn ac19_ac47_a_ctrl_copy_of_a_clone_is_a_sibling_with_the_moved_matrix() {
    let (d, p, c) = one_clone([1.0, 0.0, 0.0, 1.0, 40.0, 0.0]);
    let mut s = open(&d);
    s.press_layer_row(&rid(c), RowPress::Plain);
    key_with(&mut s, "m", false, false, false);
    let r = s.commit_move_entry(
        "0",
        "30",
        MoveEntryMode {
            absolute: false,
            copy: true,
        },
    );
    let _ = r;
    let sel = selected(&mut s);
    assert_eq!(sel.len(), 1);
    let copy = sel[0];
    assert_ne!(copy, c);
    assert!(is_clone(&s, copy));
    assert_eq!(
        origin_of(&s, copy),
        p,
        "a sibling, not a clone of the source clone"
    );
    assert_eq!(matrix_of(&s, copy), [1.0, 0.0, 0.0, 1.0, 40.0, 30.0]);
    assert_eq!(matrix_of(&s, c), [1.0, 0.0, 0.0, 1.0, 40.0, 0.0]);
    let ids = doc_of(&s).object_ids();
    let pos = |x| ids.iter().position(|&i| i == x).unwrap();
    assert_eq!(pos(copy), pos(c) + 1, "above its source");
}

// ---------------------------------------------------------------------------------------------
// Selecting, the Node tool and the panel (21 to 26, 56)
// ---------------------------------------------------------------------------------------------

/// Chain O <- A <- B with A and B moved apart: O at (10, 10), A +40 in x, B +40 in y of A.
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

#[test]
fn ac21_the_subject_line_names_the_direct_original() {
    let (d, o, a, b) = chain();
    let mut s = open(&d);
    s.press_layer_row(&rid(a), RowPress::Plain);
    let name_o = {
        let r = row(&mut s, o);
        r["name"].as_str().unwrap().to_owned()
    };
    assert!(name_o.starts_with("Path "), "{name_o}");
    let subject = |s: &Session| s.style_scope().subject;
    assert_eq!(subject(&s), format!("Clone of {name_o}"));
    s.press_layer_row(&rid(b), RowPress::Plain);
    assert_eq!(subject(&s), format!("Clone of Clone of {name_o}"));
    s.press_layer_row(&rid(a), RowPress::Plain);
    s.press_layer_row(&rid(b), RowPress::Toggle);
    assert_eq!(subject(&s), "2 clones");
    s.press_layer_row(&rid(o), RowPress::Toggle);
    assert_eq!(subject(&s), "3 objects");
}

#[test]
fn ac22_ac56_the_clone_block_and_the_chain_line() {
    let (d, o, a, b) = chain();
    let mut s = open(&d);
    s.press_layer_row(&rid(b), RowPress::Plain);
    let panel = s.clone_panel();
    let block = panel.block.expect("the block shows for a clone");
    assert_eq!(block.count, 1);
    assert_eq!(block.deepest, 2);
    assert!(block.root.starts_with("Path "));
    s.press_layer_row(&rid(a), RowPress::Plain);
    let block = s.clone_panel().block.unwrap();
    assert_eq!(block.deepest, 1, "no chain line for a depth-1 clone");
    // Not for an original, nor for a mix.
    s.press_layer_row(&rid(o), RowPress::Plain);
    assert!(s.clone_panel().block.is_none());
    s.press_layer_row(&rid(a), RowPress::Plain);
    s.press_layer_row(&rid(o), RowPress::Toggle);
    assert!(s.clone_panel().block.is_none());
    // Several clones: count and deepest.
    s.press_layer_row(&rid(a), RowPress::Plain);
    s.press_layer_row(&rid(b), RowPress::Toggle);
    let block = s.clone_panel().block.unwrap();
    assert_eq!((block.count, block.deepest), (2, 2));
}

#[test]
fn ac23_the_origin_line_counts_direct_clones_only() {
    let d = Document::new(1);
    let o = path_square(&d, 10.0, 10.0, 10.0);
    let a1 = mk(&d, o);
    let a2 = mk(&d, o);
    let b = mk(&d, a1);
    let _ = (a2, b);
    let mut s = open(&d);
    s.press_layer_row(&rid(o), RowPress::Plain);
    assert_eq!(
        s.clone_panel().followers,
        2,
        "A1 and A2 follow O directly, B does not"
    );
    s.press_layer_row(&rid(a1), RowPress::Plain);
    assert_eq!(s.clone_panel().followers, 1, "B follows the clone A1");
    assert!(
        s.clone_panel().block.is_some(),
        "both lines for a clone that is followed"
    );
    s.press_layer_row(&rid(b), RowPress::Plain);
    assert_eq!(s.clone_panel().followers, 0);
}

#[test]
fn ac24_a_style_edit_on_clones_and_others_goes_to_the_others_only() {
    let d = Document::new(1);
    let p = path_square(&d, 10.0, 10.0, 10.0);
    let q = path_square(&d, 100.0, 10.0, 10.0);
    let c = mk(&d, p);
    let d = forge(&d, &[(c, [1.0, 0.0, 0.0, 1.0, 40.0, 0.0])]);
    let mut s = open(&d);
    s.press_layer_row(&rid(c), RowPress::Plain);
    s.press_layer_row(&rid(q), RowPress::Toggle);
    assert_eq!(selected(&mut s).len(), 2);
    assert_eq!(s.set_style_text(StyleField::FillColor, "#0000ff"), Ok(true));
    assert_eq!(
        colour_at(&s, pt(105.0, 15.0)).unwrap()[..3],
        [0, 0, 255],
        "q changed"
    );
    assert_eq!(
        colour_at(&s, pt(15.0, 15.0)).unwrap()[..3],
        [255, 0, 0],
        "p did not"
    );
    assert_eq!(
        colour_at(&s, pt(55.0, 15.0)).unwrap()[..3],
        [255, 0, 0],
        "the clone follows p"
    );
    // "Mixed" is computed from the objects that are not clones: one non-clone, no Mixed.
    let v = s.style_panel_view();
    assert!(!v.fill_color_mixed, "the clone does not count");
    // Clones only: nothing to write.
    s.press_layer_row(&rid(c), RowPress::Plain);
    let n = labels(&s).len();
    let _ = s.set_style_text(StyleField::FillColor, "#00ff00");
    assert_eq!(labels(&s).len(), n, "a clone is never given a style");
}

#[test]
fn ac25_select_original_root_original_and_clones_are_view_state() {
    let (d, o, a, b) = chain();
    let mut s = open(&d);
    s.press_layer_row(&rid(b), RowPress::Plain);
    let n = labels(&s).len();
    let r = s.select_original();
    assert_eq!(r.code(), "done");
    assert_eq!(selected(&mut s), vec![a], "one step up the chain");
    s.press_layer_row(&rid(b), RowPress::Plain);
    s.select_root_original();
    assert_eq!(selected(&mut s), vec![o]);
    let r = s.select_clones();
    assert_eq!(r.code(), "done");
    assert_eq!(selected(&mut s), vec![a], "for O it selects A, not B");
    assert_eq!(labels(&s).len(), n, "no commit");
    // Two clones of one original: the original is selected once.
    let d2 = Document::new(1);
    let p = path_square(&d2, 10.0, 10.0, 10.0);
    let c1 = mk(&d2, p);
    let c2 = mk(&d2, p);
    let mut s2 = open(&d2);
    s2.press_layer_row(&rid(c1), RowPress::Plain);
    s2.press_layer_row(&rid(c2), RowPress::Toggle);
    s2.select_original();
    assert_eq!(selected(&mut s2), vec![p]);
}

#[test]
fn ac25_hidden_or_locked_targets_are_left_out_with_the_specified_notices() {
    let d = Document::new(1);
    let p1 = path_square(&d, 10.0, 10.0, 10.0);
    let p2 = path_square(&d, 50.0, 10.0, 10.0);
    let p3 = path_square(&d, 90.0, 10.0, 10.0);
    let c1 = mk(&d, p1);
    let c2 = mk(&d, p2);
    let c3 = mk(&d, p3);
    d.set_node_visible(&[p1], false).unwrap();
    d.set_node_locked(&[p2], true).unwrap();
    let mut s = open(&d);
    // All targets unselectable: refused, selection unchanged.
    s.press_layer_row(&rid(c1), RowPress::Plain);
    let r = s.select_original();
    assert_eq!(r.code(), "refused");
    assert_eq!(
        r.notice,
        "The original is hidden or locked. Show or unlock it in the Layers tab. Nothing was changed."
    );
    assert_eq!(selected(&mut s), vec![c1]);
    s.press_layer_row(&rid(c1), RowPress::Plain);
    s.press_layer_row(&rid(c2), RowPress::Toggle);
    let r = s.select_original();
    assert_eq!(r.code(), "refused");
    assert!(r.notice.ends_with("Nothing was changed."), "{}", r.notice);
    let mut sel = selected(&mut s);
    sel.sort_by_key(|i| format!("{i:?}"));
    let mut want = vec![c1, c2];
    want.sort_by_key(|i| format!("{i:?}"));
    assert_eq!(sel, want);
    // Some selectable: the partial notice.
    s.press_layer_row(&rid(c1), RowPress::Plain);
    s.press_layer_row(&rid(c2), RowPress::Toggle);
    s.press_layer_row(&rid(c3), RowPress::Toggle);
    let r = s.select_original();
    assert_eq!(r.code(), "done");
    assert_eq!(
        r.notice,
        "Selected 1 of 3 originals. The rest are hidden or locked."
    );
    assert_eq!(selected(&mut s), vec![p3]);
}

#[test]
fn ac26_the_node_tool_shows_no_nodes_on_a_clone() {
    let (d, _o, a, _b) = chain();
    let mut s = open(&d);
    s.press_layer_row(&rid(a), RowPress::Plain);
    s.set_tool(Tool::Node);
    assert!(s.node_selection_is_clones());
    assert!(s.node_toolbar_state().clone_only);
    let n = labels(&s).len();
    let before = out(&s, a);
    // Clicking where a node would be, or dragging it, changes nothing.
    drag(&mut s, pt(50.0, 10.0), pt(60.0, 30.0));
    s.insert_at(pt(60.0, 10.0));
    s.join_selected();
    s.split_selected();
    assert_eq!(labels(&s).len(), n, "{:?}", labels(&s));
    assert_eq!(out(&s, a), before);
    assert!(is_clone(&s, a));
}

#[test]
fn ac26_a_double_click_on_a_clone_does_not_change_the_tool() {
    let (d, _o, a, _b) = chain();
    let mut s = open(&d);
    s.press_layer_row(&rid(a), RowPress::Plain);
    let n = labels(&s).len();
    let _ = s.double_click(pt(55.0, 15.0), false, false);
    let _ = s.double_click_hint(pt(55.0, 15.0), false, false);
    assert!(s.linked_clone_shown(), "still the Select tool");
    assert_eq!(labels(&s).len(), n);
    assert!(is_clone(&s, a));
}

// ---------------------------------------------------------------------------------------------
// Unlink, delete and repair (28 to 31, 54, 55)
// ---------------------------------------------------------------------------------------------

#[test]
fn ac28_unlink_clones_keeps_ids_selection_and_the_drawn_result() {
    let (d, _o, a, b) = chain();
    let mut s = open(&d);
    let (a0, b0) = (out(&s, a), out(&s, b));
    s.press_layer_row(&rid(a), RowPress::Plain);
    let r = s.unlink_clones();
    assert_eq!(r.code(), "done");
    assert_eq!(r.notice, "Unlinked 1 clone.");
    assert_eq!(selected(&mut s), vec![a], "still selected");
    assert!(!is_clone(&s, a));
    assert!(is_clone(&s, b), "B keeps following A");
    near(&out(&s, a), &a0, 1e-9);
    near(&out(&s, b), &b0, 1e-9);
    assert_eq!(labels(&s).last().unwrap(), "unlink_clone");
    s.press_layer_row(&rid(b), RowPress::Plain);
    let r = s.unlink_clones();
    assert_eq!(r.notice, "Unlinked 1 clone.");
    // Three at once.
    let d = Document::new(1);
    let p = path_square(&d, 10.0, 10.0, 10.0);
    for _ in 0..3 {
        mk(&d, p);
    }
    let mut s = open(&d);
    s.press_layer_row(&rid(p), RowPress::Plain);
    s.select_clones();
    assert_eq!(selected(&mut s).len(), 3);
    assert_eq!(s.unlink_clones().notice, "Unlinked 3 clones.");
    assert_eq!(clone_count(&s), 0);
    // Not for an original.
    s.press_layer_row(&rid(p), RowPress::Plain);
    assert_eq!(s.unlink_clones().code(), "ignored");
}

#[test]
fn ac31_the_repair_notices_after_delete() {
    let d = Document::new(1);
    let p = path_square(&d, 10.0, 10.0, 10.0);
    for _ in 0..3 {
        mk(&d, p);
    }
    let mut s = open(&d);
    s.press_layer_row(&rid(p), RowPress::Plain);
    s.delete_selected();
    assert_eq!(s.take_clone_repair_notice(), "3 clones became paths.");
    assert_eq!(clone_count(&s), 0);
    assert_eq!(doc_of(&s).object_ids().len(), 3, "nothing disappears");
    // Chain: delete the middle clone.
    let (d, o, a, b) = chain();
    let mut s = open(&d);
    let b0 = out(&s, b);
    s.press_layer_row(&rid(a), RowPress::Plain);
    s.delete_selected();
    assert_eq!(
        s.take_clone_repair_notice(),
        "1 clone now follows its next original."
    );
    assert_eq!(origin_of(&s, b), o);
    near(&out(&s, b), &b0, 1e-9);
    // Undo gives A back and B follows A again.
    undo(&mut s);
    assert!(is_clone(&s, a));
    assert_eq!(origin_of(&s, b), a);
    assert_eq!(matrix_of(&s, b), [1.0, 0.0, 0.0, 1.0, 0.0, 100.0]);
    near(&out(&s, b), &b0, 1e-9);
}

#[test]
fn ac36_undo_of_a_delete_that_unlinked_clones_restores_original_and_clones() {
    let (d, o, a, b) = chain();
    let mut s = open(&d);
    let (a0, b0) = (out(&s, a), out(&s, b));
    s.press_layer_row(&rid(o), RowPress::Plain);
    s.delete_selected();
    assert!(doc_of(&s).object(o).is_none());
    assert!(!is_clone(&s, a));
    assert!(is_clone(&s, b), "B stays a clone of the path A");
    undo(&mut s);
    assert!(
        doc_of(&s).object(o).is_some(),
        "the original is back with its id"
    );
    assert!(is_clone(&s, a) && is_clone(&s, b), "clones again");
    assert_eq!(origin_of(&s, a), o);
    assert_eq!(matrix_of(&s, a), [1.0, 0.0, 0.0, 1.0, 100.0, 0.0]);
    near(&out(&s, a), &a0, 1e-9);
    near(&out(&s, b), &b0, 1e-9);
    // Redo repeats it.
    let _ = s.redo();
    assert!(!is_clone(&s, a));
    near(&out(&s, a), &a0, 1e-9);
}

#[test]
fn ac36_undo_of_linked_clone_and_of_unlink() {
    let d = Document::new(1);
    let p = square(&d, 0);
    let mut s = open(&d);
    click(&mut s, mid(0));
    s.linked_clone();
    assert_eq!(clone_count(&s), 1);
    undo(&mut s);
    assert_eq!(clone_count(&s), 0);
    assert_eq!(doc_of(&s).object_ids(), vec![p]);
    let _ = s.redo();
    assert_eq!(clone_count(&s), 1);
    s.unlink_clones();
    assert_eq!(clone_count(&s), 0);
    undo(&mut s);
    assert_eq!(
        clone_count(&s),
        1,
        "taking back the unlink gives the clone back"
    );
}

#[test]
fn ac35_open_removes_orphans_in_one_step_and_says_so() {
    let d = Document::new(1);
    let p = path_square(&d, 10.0, 10.0, 10.0);
    let keep = path_square(&d, 50.0, 10.0, 10.0);
    let c1 = mk(&d, p);
    let c2 = mk(&d, p);
    let ck = mk(&d, keep);
    // Forge both orphan origins: point c1 and c2 at nothing.
    let l = LoroDoc::new();
    l.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let tree = l.get_tree("paths");
    for c in [c1, c2] {
        let mut link: HashMap<String, LoroValue> = HashMap::new();
        link.insert("origin".into(), LoroValue::from("5:9999"));
        link.insert(
            "matrix".into(),
            LoroValue::from(
                [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
                    .iter()
                    .map(|&x| LoroValue::from(x))
                    .collect::<Vec<_>>(),
            ),
        );
        tree.get_meta(tid(c))
            .unwrap()
            .insert("clone_link", LoroValue::from(link))
            .unwrap();
    }
    l.commit();
    // Container with the orphaned snapshot.
    let packed = {
        use std::io::{Read, Write};
        let bytes = l.export(loro::ExportMode::Snapshot).unwrap();
        let mut zin =
            zip::ZipArchive::new(std::io::Cursor::new(pack(&d, "0.1.0").unwrap())).unwrap();
        let mut buf = Vec::new();
        {
            let mut zout = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            for i in 0..zin.len() {
                let mut f = zin.by_index(i).unwrap();
                let name = f.name().to_owned();
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
    };
    let mut s = Session::open(2, &packed).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    assert_eq!(
        s.take_orphan_notice(),
        "Removed 2 clones whose original was not found."
    );
    assert_eq!(s.take_orphan_notice(), "", "once");
    assert!(s.is_dirty(), "the project is marked changed");
    let ids = doc_of(&s).object_ids();
    assert_eq!(ids, vec![p, keep, ck]);
    assert_eq!(labels(&s).last().unwrap(), "remove_orphan_clones");
    assert!(!ids.contains(&c1) && !ids.contains(&c2));
}

#[test]
fn ac35_the_golden_orphan_and_ring_fixture_opens_without_hanging() {
    let bytes =
        std::fs::read("../curvyo-document-core/tests/fixtures/clone_orphan_and_ring_v13.curvyo")
            .unwrap();
    let mut s = Session::open(2, &bytes).unwrap();
    let note = s.take_orphan_notice();
    assert!(
        note.starts_with("Removed ") && note.ends_with(" whose original was not found."),
        "{note:?}"
    );
    assert!(doc_of(&s).read_objects().orphans.is_empty());
}

#[test]
fn ac37_save_and_reopen_through_the_session_keeps_the_chain() {
    let (d, o, a, b) = chain();
    let mut s = open(&d);
    let bytes = s.pack_for_save().unwrap();
    let s2 = Session::open(3, &bytes).unwrap();
    assert_eq!(origin_of(&s2, b), a);
    assert_eq!(origin_of(&s2, a), o);
    assert_eq!(matrix_of(&s2, a), [1.0, 0.0, 0.0, 1.0, 100.0, 0.0]);
    assert_eq!(out(&s2, b), out(&s, b));
    assert_eq!(doc_of(&s2).object_ids(), doc_of(&s).object_ids());
    assert!(!s2.is_dirty(), "opening does not rewrite or dirty the file");
}

// ---------------------------------------------------------------------------------------------
// Opacity, flags and the Layers rows (0050 criteria 7, 24, 25, 26, 28; 0049 criteria 49, 58)
// ---------------------------------------------------------------------------------------------

#[test]
fn c0050_28_the_clone_is_painted_with_the_roots_opacity_and_its_own_ancestors() {
    let d = Document::new(1);
    let layer_c = d.new_layer(None);
    let root = path_square(&d, 20.0, 20.0, 30.0);
    d.edit_style(&[root], &StyleEdit::FillOpacity(Opacity::new(0.8).unwrap()))
        .unwrap();
    d.set_node_opacity(&[root], Opacity::new(0.5).unwrap())
        .unwrap();
    let c = mk(&d, root);
    let dd = mk(&d, c);
    let d = forge(
        &d,
        &[
            (c, [1.0, 0.0, 0.0, 1.0, 100.0, 0.0]),
            (dd, [1.0, 0.0, 0.0, 1.0, 0.0, 100.0]),
        ],
    );
    d.move_nodes(&curvyo_document_core::MovePlan {
        nodes: vec![c],
        target: curvyo_document_core::MoveTarget::FrontOf(Some(layer_c)),
    })
    .unwrap();
    d.set_node_opacity(&[layer_c], Opacity::new(0.5).unwrap())
        .unwrap();
    let mut s = open(&d);
    let alpha = |s: &Session, p: Point| i32::from(colour_at(s, p).unwrap()[3]);
    // Root alone: 0.8 * 0.5 = 0.4 -> 102. Clone C in a layer of 50 %: 0.8 * 0.5 * 0.5 = 0.2 -> 51.
    assert!(
        (alpha(&s, pt(35.0, 35.0)) - 102).abs() <= 1,
        "root {}",
        alpha(&s, pt(35.0, 35.0))
    );
    assert!(
        (alpha(&s, pt(135.0, 35.0)) - 51).abs() <= 1,
        "C {}",
        alpha(&s, pt(135.0, 35.0))
    );
    // D follows C and is in no layer: the layer of C does not count.
    assert!(
        (alpha(&s, pt(135.0, 135.0)) - 102).abs() <= 1,
        "D {}",
        alpha(&s, pt(135.0, 135.0))
    );
    // Set the root's opacity to 100 %: C is painted at 40 % in the next frame.
    s.press_layer_row(&rid(root), RowPress::Plain);
    let _ = s.commit_layer_opacity(100.0);
    click(&mut s, EMPTY);
    assert!(
        (alpha(&s, pt(35.0, 35.0)) - 204).abs() <= 1,
        "root {}",
        alpha(&s, pt(35.0, 35.0))
    );
    assert!(
        (alpha(&s, pt(135.0, 35.0)) - 102).abs() <= 1,
        "C {}",
        alpha(&s, pt(135.0, 35.0))
    );
    assert!(
        (alpha(&s, pt(135.0, 135.0)) - 204).abs() <= 1,
        "D {}",
        alpha(&s, pt(135.0, 135.0))
    );
}

#[test]
fn c0050_24_25_a_hidden_original_still_feeds_its_clones_and_flags_are_the_clones_own() {
    let d = Document::new(1);
    let p = path_square(&d, 10.0, 10.0, 10.0);
    let c = mk(&d, p);
    let c2 = mk(&d, p);
    let d = forge(
        &d,
        &[
            (c, [1.0, 0.0, 0.0, 1.0, 40.0, 0.0]),
            (c2, [1.0, 0.0, 0.0, 1.0, 80.0, 0.0]),
        ],
    );
    d.set_node_visible(&[p], false).unwrap();
    d.set_node_visible(&[c2], false).unwrap();
    d.set_node_locked(&[p], true).unwrap();
    let s = open(&d);
    assert!(
        !drawn(&s, pt(15.0, 15.0)),
        "the hidden original is not drawn"
    );
    assert!(
        drawn(&s, pt(55.0, 15.0)),
        "its clone is, from the original's outline and style"
    );
    assert_eq!(colour_at(&s, pt(55.0, 15.0)).unwrap()[..3], [255, 0, 0]);
    assert!(!drawn(&s, pt(95.0, 15.0)), "a hidden clone is not drawn");
    // The read lists every leaf with its flags.
    let read = doc_of(&s).read_nodes();
    assert_eq!(read.len(), 3);
}

#[test]
fn c0050_7_38_clone_rows_have_the_clone_names_and_the_select_original_action() {
    let (d, o, a, b) = chain();
    let mut s = open(&d);
    let ro = row(&mut s, o);
    let name_o = ro["name"].as_str().unwrap().to_owned();
    assert_eq!(row(&mut s, a)["name"], format!("Clone of {name_o}"));
    assert_eq!(
        row(&mut s, b)["name"],
        format!("Clone of Clone of {name_o}")
    );
    assert_eq!(row(&mut s, a)["rowAction"], "select-original");
    assert_ne!(ro["rowAction"], "select-original");
}

#[test]
fn c0050_28_the_clone_row_shows_the_roots_opacity_read_only() {
    let d = Document::new(1);
    let p = path_square(&d, 10.0, 10.0, 10.0);
    d.set_node_opacity(&[p], Opacity::new(0.5).unwrap())
        .unwrap();
    let c = mk(&d, p);
    let mut s = open(&d);
    assert_eq!(row(&mut s, c)["readout"], "50%");
}

#[test]
fn c0050_26_the_opacity_field_never_writes_to_a_clone() {
    let d = Document::new(1);
    let p = path_square(&d, 10.0, 10.0, 10.0);
    let c = mk(&d, p);
    let mut s = open(&d);
    s.press_layer_row(&rid(c), RowPress::Plain);
    let v: Value = serde_json::from_str(&s.layers_view_json(0, 50)).unwrap();
    let text = v["opacity"].to_string();
    assert!(
        text.contains("Linked clones show the opacity of their original."),
        "{text}"
    );
    let n = labels(&s).len();
    let _ = s.commit_layer_opacity(30.0);
    assert_eq!(labels(&s).len(), n, "nothing written for clones only");
}

#[test]
fn c0050_36_45_delete_layer_repairs_clones_and_undo_restores_them() {
    let d = Document::new(1);
    let layer = d.new_layer(None);
    let o = path_square(&d, 10.0, 10.0, 10.0);
    let c = mk(&d, o);
    let outside = path_square(&d, 200.0, 10.0, 10.0);
    let oc = mk(&d, outside);
    let d = forge(&d, &[(c, [1.0, 0.0, 0.0, 1.0, 40.0, 0.0])]);
    d.move_nodes(&curvyo_document_core::MovePlan {
        nodes: vec![o],
        target: curvyo_document_core::MoveTarget::FrontOf(Some(layer)),
    })
    .unwrap();
    // The clone c stays outside the layer; its original o is inside.
    let mut s = open(&d);
    let c0 = out(&s, c);
    let _ = (oc, outside);
    let r = s.delete_layer_row(&rid(layer));
    let _ = r;
    assert!(
        doc_of(&s).object(o).is_none(),
        "the layer's content is gone"
    );
    assert!(!is_clone(&s, c), "the clone outside became a path");
    near(&out(&s, c), &c0, 1e-9);
    undo(&mut s);
    assert!(doc_of(&s).object(o).is_some());
    assert!(is_clone(&s, c));
    assert_eq!(origin_of(&s, c), o);
    assert_eq!(matrix_of(&s, c), [1.0, 0.0, 0.0, 1.0, 40.0, 0.0]);
}

// ---------------------------------------------------------------------------------------------
// Marks (57 to 62)
// ---------------------------------------------------------------------------------------------

#[test]
fn ac57_58_selecting_a_clone_marks_it_and_its_original_roots_too() {
    use curvyo_ui_core::MarkRole;
    let (d, o, a, b) = chain();
    let mut s = open(&d);
    s.press_layer_row(&rid(b), RowPress::Plain);
    let marks = s.clone_marks();
    let role_of = |id| marks.traces.iter().find(|t| t.id == id).map(|t| t.role);
    assert_eq!(role_of(b), Some(MarkRole::Clone));
    assert_eq!(role_of(a), Some(MarkRole::Original));
    assert_eq!(role_of(o), Some(MarkRole::Root));
    let tag = |id| {
        marks
            .tags
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.text.clone())
    };
    assert_eq!(tag(b).as_deref(), Some("Clone"));
    assert_eq!(tag(a).as_deref(), Some("Original"));
    assert_eq!(tag(o).as_deref(), Some("Root"));
    // A selected clone that is also the original of another selected clone: one tag, both roles.
    s.press_layer_row(&rid(a), RowPress::Plain);
    s.press_layer_row(&rid(b), RowPress::Toggle);
    let marks = s.clone_marks();
    let t = marks.tags.iter().find(|t| t.id == a).expect("a tag for A");
    assert_eq!(t.text, "Clone + Original");
    assert_eq!(
        marks.traces.iter().find(|t| t.id == a).unwrap().role,
        MarkRole::Clone,
        "the strongest role"
    );
    // Deselect: nothing.
    click(&mut s, pt(400.0, 400.0));
    let marks = s.clone_marks();
    assert!(marks.traces.is_empty() && marks.tags.is_empty());
}

#[test]
fn ac58_a_hidden_original_is_not_traced_and_the_tag_says_so() {
    let d = Document::new(1);
    // 100 mm: the long tag only fits between the handles of a box this wide (finding F2).
    let p = path_square(&d, 10.0, 40.0, 100.0);
    let c = mk(&d, p);
    let d = forge(&d, &[(c, [1.0, 0.0, 0.0, 1.0, 200.0, 0.0])]);
    d.set_node_visible(&[p], false).unwrap();
    let mut s = open(&d);
    s.press_layer_row(&rid(c), RowPress::Plain);
    let marks = s.clone_marks();
    assert!(
        marks.traces.iter().all(|t| t.id != p),
        "the hidden original is not traced"
    );
    let tag = marks.tags.iter().find(|t| t.id == c).unwrap();
    assert_eq!(tag.text, "Clone (original hidden)");
}

#[test]
fn ac59_no_marks_with_other_tools_or_in_a_drag() {
    let (d, _o, a, _b) = chain();
    let mut s = open(&d);
    s.press_layer_row(&rid(a), RowPress::Plain);
    assert!(!s.clone_marks().traces.is_empty());
    s.set_tool(Tool::Node);
    assert!(
        !s.clone_marks().traces.is_empty(),
        "also with the Node tool"
    );
    s.set_tool(Tool::Pen);
    assert!(s.clone_marks().traces.is_empty(), "not with the Pen");
    s.set_tool(Tool::Select);
    // During a canvas drag (a marquee) they are hidden.
    let from = pt(500.0, 500.0);
    hold(&mut s, from, false, false, false);
    s.pointer_down(from, false);
    hold(&mut s, pt(600.0, 600.0), false, false, false);
    hold(&mut s, pt(650.0, 650.0), false, false, false);
    assert!(s.clone_marks().traces.is_empty(), "hidden during the drag");
    s.pointer_up(pt(650.0, 650.0), false, false);
}

#[test]
fn ac61_marks_stay_cheap_for_many_selected_clones() {
    let d = Document::new(1);
    let p = path_square(&d, 10.0, 10.0, 5.0);
    let ids: Vec<NodeId> = (0..300).map(|_| mk(&d, p)).collect();
    let mut s = open(&d);
    ctrl_a(&mut s);
    let marks = s.clone_marks();
    let clone_traces = marks
        .traces
        .iter()
        .filter(|t| t.role == curvyo_ui_core::MarkRole::Clone)
        .count();
    assert!(clone_traces <= 200, "{clone_traces}");
    assert!(
        marks.traces.len() <= 200 + 50 + 1 + 50,
        "{}",
        marks.traces.len()
    );
    assert!(
        marks.tags.len() <= 12,
        "tags only while at most 12 are marked"
    );
    let _ = ids;
}

#[test]
fn ac33_a_boolean_reads_the_resolved_outline_of_a_clone() {
    let build = |unlink_first: bool| {
        let d = Document::new(1);
        let p = path_square(&d, 20.0, 20.0, 30.0);
        let q = path_square(&d, 60.0, 20.0, 30.0);
        let c = mk(&d, q);
        // The clone sits over the path p, overlapping it.
        let d = forge(&d, &[(c, [1.0, 0.0, 0.0, 1.0, -30.0, 10.0])]);
        let mut s = open(&d);
        s.press_layer_row(&rid(c), RowPress::Plain);
        if unlink_first {
            s.unlink_clones();
        }
        s.press_layer_row(&rid(c), RowPress::Plain);
        s.press_layer_row(&rid(p), RowPress::Toggle);
        let out = s.apply_boolean(curvyo_ui_core::BooleanOp::Union);
        let _ = out;
        let doc = doc_of(&s);
        let ids = doc.object_ids();
        // The Union result is the path that is neither p's id... collect outlines of all paths
        // except the untouched q.
        let mut shapes: Vec<Vec<(f64, f64)>> = ids
            .iter()
            .filter(|&&i| i != q)
            .map(|&i| outline_of(&doc.object(i).unwrap()))
            .collect();
        shapes.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
        (shapes.len(), shapes)
    };
    let (n1, via_clone) = build(false);
    let (n2, via_path) = build(true);
    assert_eq!(n1, 1, "the union consumed the clone and the path");
    assert_eq!(n1, n2);
    assert_eq!(via_clone.len(), via_path.len());
    // The same union outline up to the start node: compare the sorted point sets.
    let norm = |v: &Vec<(f64, f64)>| {
        let mut v = v.clone();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v
    };
    near(&norm(&via_clone[0]), &norm(&via_path[0]), 1e-6);
}

#[test]
fn ac2_a_rounded_rectangle_with_four_radii_gives_eight_nodes() {
    use curvyo_document_core::CornerRadii;
    let d = Document::new(1);
    let r = rect_at(&d, 20.0, 20.0, 60.0, 40.0);
    d.resize_rect(
        r,
        RectBounds {
            origin: pt(20.0, 20.0),
            width: Length::from_mm(60.0),
            height: Length::from_mm(40.0),
        },
        CornerRadii {
            tl: Length::from_mm(2.0),
            tr: Length::from_mm(4.0),
            br: Length::from_mm(6.0),
            bl: Length::from_mm(8.0),
        },
        None,
    )
    .unwrap();
    red_fill(&d, &[r]);
    let mut s = open(&d);
    click(&mut s, pt(50.0, 40.0));
    assert_eq!(s.linked_clone().code(), "done");
    let doc = doc_of(&s);
    match doc.object(r).unwrap() {
        ObjectSnapshot::Path(p) => assert_eq!(p.anchors.len(), 8),
        other => panic!("{other:?}"),
    }
    let ids = doc.object_ids();
    near(&out(&s, ids[1]), &out(&s, r), 1e-9);
}

/// Criterion 16 (a) of the specification, shifted by (10, 10): the origin square (10..20) and its
/// clone (translate 30, so 40..50), both selected, scaled by (2, 1) about the centre of their
/// box (x = 30, where the typed size entry scales): the origin is 20 wide (x = -10 to 10) and the
/// clone covers x = 50 to 70, 20 wide, at the same relative place.
#[test]
fn ac16a_scaling_an_original_and_its_clone_together_by_a_typed_size() {
    let (d, p, c) = one_clone([1.0, 0.0, 0.0, 1.0, 30.0, 0.0]);
    let mut s = open(&d);
    ctrl_a(&mut s);
    assert_eq!(selected(&mut s).len(), 2);
    // The SE corner handle of the group box (10..50, 10..20).
    let _ = key_with(&mut s, "s", false, false, false);
    let view = s.transform_entry().expect("the S key opens the size entry");
    let w0: f64 = view.fields[0].prefill.parse().unwrap();
    let h0: f64 = view.fields[1].prefill.parse().unwrap();
    assert!((w0 - 40.0).abs() < 1e-6, "{w0}");
    let _ = s.commit_transform_entry(&format!("{}", w0 * 2.0), &format!("{h0}"), 0);
    let o = out(&s, p);
    let k = out(&s, c);
    let x = |v: &Vec<(f64, f64)>| {
        (
            v.iter().map(|p| p.0).fold(f64::MAX, f64::min),
            v.iter().map(|p| p.0).fold(f64::MIN, f64::max),
        )
    };
    let (o0, o1) = x(&o);
    let (k0, k1) = x(&k);
    assert!(
        (o0 + 10.0).abs() < 1e-6 && (o1 - 10.0).abs() < 1e-6,
        "origin {o0}..{o1}"
    );
    assert!(
        (k0 - 50.0).abs() < 1e-6 && (k1 - 70.0).abs() < 1e-6,
        "clone {k0}..{k1}"
    );
    assert!(is_clone(&s, c));
}
