//! Independent tester tests for `specs/0057-clone-overrides` at the editor session: real
//! gestures (drag, nudge, typed scale/rotate/skew), the Node tool, the clone block and its
//! notices, and Spread colours. Written from the specification; only the names of the public
//! session seams were looked up.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(
    missing_docs,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation
)]
#![allow(
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::doc_markdown
)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use curvyo_document_core::{
    AnchorId, Color, Document, NewAnchor, NodeId, ObjectSnapshot, Point, StyleEdit, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, Session, Tool};
use curvyo_ui_core::{OverrideState, StyleField};
use loro::{LoroDoc, LoroValue, TreeID};

static TIME: AtomicU64 = AtomicU64::new(1_000);
static AID: AtomicU64 = AtomicU64::new(70_000);

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}
fn path_square(d: &Document, x: f64, y: f64, s: f64) -> NodeId {
    let n = |px, py| {
        NewAnchor::corner(
            AnchorId::new(9, AID.fetch_add(1, Ordering::SeqCst)),
            pt(px, py),
        )
    };
    let id = d.create_path(&[n(x, y), n(x + s, y), n(x + s, y + s), n(x, y + s)], true);
    d.edit_style(&[id], &StyleEdit::FillColor(Color { r: 255, g: 0, b: 0 }))
        .unwrap();
    d.edit_style(&[id], &StyleEdit::FillEnabled(true)).unwrap();
    id
}
fn tid(id: NodeId) -> TreeID {
    let j = serde_json::to_value(id).unwrap();
    TreeID::new(
        j["peer"].as_str().unwrap().parse().unwrap(),
        j["counter"].as_i64().unwrap() as i32,
    )
}
/// Rezips `d` with the matrix of each listed clone replaced.
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
    unpack(1, &buf).unwrap()
}
fn open(d: &Document) -> Session {
    let mut s = Session::open(2, &pack(d, "0.1.0").unwrap()).unwrap();
    s.set_tool(Tool::Select);
    s.resize_viewport(1600.0, 1000.0);
    s
}
fn doc_of(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}
fn out(s: &Session, id: NodeId) -> Vec<(f64, f64)> {
    match doc_of(s).object(id).unwrap() {
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
        ObjectSnapshot::Primitive(_) => panic!(),
    }
}
fn near(a: &[(f64, f64)], b: &[(f64, f64)], tol: f64) {
    assert_eq!(a.len(), b.len());
    for (p, q) in a.iter().zip(b) {
        assert!(
            (p.0 - q.0).abs() <= tol && (p.1 - q.1).abs() <= tol,
            "{a:?} vs {b:?}"
        );
    }
}
fn hold(s: &mut Session, at: Point) {
    s.modifiers_changed(false, false, false);
    s.pointer_hover(at, false, false);
}
fn click(s: &mut Session, p: Point) {
    hold(s, p);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}
fn drag(s: &mut Session, a: Point, b: Point) {
    hold(s, a);
    s.pointer_down(a, false);
    hold(s, pt(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y)));
    hold(s, b);
    s.pointer_up(b, false, false);
}
fn key(s: &mut Session, k: &str, ctrl: bool, shift: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        ctrl,
        shift,
        time_ms: TIME.fetch_add(1_000, Ordering::SeqCst) as f64,
        ..KeyInput::default()
    })
}
fn commits(s: &Session) -> usize {
    let l = LoroDoc::new();
    l.import(&doc_of(s).export_loro_snapshot().unwrap())
        .unwrap();
    l.len_changes()
}
fn is_clone(s: &Session, id: NodeId) -> bool {
    matches!(doc_of(s).object(id), Some(ObjectSnapshot::Clone(_)))
}

/// An original (10..30 square) with an own-placement clone at +60 and a following clone at +60, y +40.
fn scene() -> (Document, NodeId, NodeId, NodeId) {
    let d = Document::new(1);
    let p = path_square(&d, 10.0, 10.0, 20.0);
    let held = d.linked_clone(&[p], &[]).unwrap()[0];
    let free = d.linked_clone(&[p], &[]).unwrap()[0];
    let d = forge(
        &d,
        &[
            (held, [1.0, 0.0, 0.0, 1.0, 60.0, 0.0]),
            (free, [1.0, 0.0, 0.0, 1.0, 60.0, 40.0]),
        ],
    );
    d.set_own_placement(&[held], true).unwrap();
    (d, p, held, free)
}

#[test]
fn ac3_ac8_every_gesture_kind_holds_the_clone_and_says_so_once() {
    type Act = fn(&mut Session);
    let gestures: [(&str, Act); 5] = [
        ("drag move", |s| drag(s, pt(15.0, 15.0), pt(25.0, 22.0))),
        ("nudge", |s| {
            key(s, "ArrowRight", false, false);
        }),
        ("typed scale", |s| {
            assert_eq!(key(s, "s", false, false), KeyOutcome::EntryOpened);
            let second = s
                .transform_entry()
                .unwrap()
                .fields
                .get(1)
                .map(|f| f.prefill.clone())
                .unwrap_or_default();
            s.commit_transform_entry("300", &second, 0);
        }),
        ("typed rotate", |s| {
            assert_eq!(key(s, "r", false, false), KeyOutcome::EntryOpened);
            s.commit_transform_entry("15", "", 0);
        }),
        ("typed skew", |s| {
            assert_eq!(key(s, "k", false, false), KeyOutcome::EntryOpened);
            s.commit_transform_entry("20", "", 0);
        }),
    ];
    for (name, act) in gestures {
        let (d, p, held, free) = scene();
        let mut s = open(&d);
        click(&mut s, pt(20.0, 20.0));
        let (p0, h0, f0) = (out(&s, p), out(&s, held), out(&s, free));
        let n = commits(&s);
        act(&mut s);
        assert_eq!(commits(&s), n + 1, "{name}: one commit");
        assert_ne!(out(&s, p), p0, "{name}: the original changed");
        near(&out(&s, held), &h0, 1e-6);
        assert_ne!(out(&s, free), f0, "{name}: the following clone moved");
        assert_eq!(
            s.take_clone_hold_notice(),
            "1 clone with own placement stayed in place.",
            "{name}"
        );
        assert_eq!(s.take_clone_hold_notice(), "", "{name}: once");
    }
}

#[test]
fn ac8_no_notice_when_nothing_is_held_back_and_none_for_a_node_edit() {
    let d = Document::new(1);
    let p = path_square(&d, 10.0, 10.0, 20.0);
    let free = d.linked_clone(&[p], &[]).unwrap()[0];
    let d = forge(&d, &[(free, [1.0, 0.0, 0.0, 1.0, 60.0, 0.0])]);
    let mut s = open(&d);
    click(&mut s, pt(20.0, 20.0));
    drag(&mut s, pt(15.0, 15.0), pt(25.0, 22.0));
    assert_eq!(s.take_clone_hold_notice(), "");
    // Node edit with an own-placement clone: the clone follows the edit, no notice.
    let (d, p, held, _) = scene();
    let mut s = open(&d);
    s.set_tool(Tool::Node);
    click(&mut s, pt(20.0, 20.0));
    let h0 = out(&s, held);
    let corner = pt(30.0, 30.0);
    hold(&mut s, corner);
    s.pointer_down(corner, false);
    hold(&mut s, pt(35.0, 35.0));
    s.pointer_up(pt(35.0, 35.0), false, false);
    assert_eq!(out(&s, p)[2], (35.0, 35.0), "the node moved");
    assert_eq!(
        out(&s, held)[2],
        (95.0, 35.0),
        "a node edit of the original reaches the held clone"
    );
    assert_eq!(out(&s, held)[0], h0[0]);
    assert_eq!(s.take_clone_hold_notice(), "");
}

#[test]
fn ac9_a_transform_drag_does_not_preview_a_held_clone() {
    let (d, _, held, free) = scene();
    let mut s = open(&d);
    click(&mut s, pt(20.0, 20.0));
    let (h0, f0) = (out(&s, held), out(&s, free));
    let n = commits(&s);
    hold(&mut s, pt(15.0, 15.0));
    s.pointer_down(pt(15.0, 15.0), false);
    hold(&mut s, pt(40.0, 15.0));
    // Mid-drag: nothing written.
    assert_eq!(commits(&s), n);
    assert_eq!(out(&s, held), h0);
    assert_eq!(out(&s, free), f0);
    s.pointer_up(pt(40.0, 15.0), false, false);
    near(&out(&s, held), &h0, 1e-6);
}

#[test]
fn ac1_ac2_ac10_ac14_the_clone_block_rows_and_notices() {
    let (d, _, held, free) = scene();
    let mut s = open(&d);
    // Select the held clone by its position.
    click(&mut s, pt(70.0, 20.0));
    let b = s
        .clone_panel()
        .block
        .expect("a clone block for a clone selection");
    assert_eq!(
        (b.placement, b.style),
        (OverrideState::Own, OverrideState::Original)
    );
    // Both clones: mixed placement.
    key(&mut s, "a", true, false);
    assert_eq!(
        s.clone_panel().block,
        None,
        "a path is selected too: no block"
    );
    key(&mut s, "Escape", false, false);
    click(&mut s, pt(70.0, 20.0));
    s.select_clones();
    key(&mut s, "Escape", false, false);
    click(&mut s, pt(70.0, 20.0));
    assert_eq!(s.clone_panel().block.unwrap().count, 1);
    let n = commits(&s);
    let r = s.set_clone_placement(true);
    assert_eq!(
        (r.code(), r.notice.as_str()),
        ("ignored", ""),
        "already own: nothing posted"
    );
    assert_eq!(commits(&s), n);
    let r = s.set_clone_placement(false);
    assert_eq!(r.notice, "Follows its original's placement again.");
    let r = s.set_clone_placement(true);
    assert_eq!(
        r.notice,
        "Own placement. It stays put when the original moves."
    );
    let before = out(&s, held);
    let r = s.set_clone_style(true);
    assert_eq!(r.code(), "done");
    near(&out(&s, held), &before, 1e-9);
    assert_eq!(s.clone_panel().block.unwrap().style, OverrideState::Own);
    let r = s.set_clone_style(false);
    assert_eq!(
        r.notice,
        "Shows its original's style again. Its own style was removed."
    );
    let r = s.set_clone_style(false);
    assert_eq!(r.code(), "ignored");
    let _ = free;
}

#[test]
fn ac18_a_style_edit_on_a_mixed_selection_names_the_clones_left_alone() {
    let (d, p, held, free) = scene();
    d.set_own_style(&[held], true).unwrap();
    let mut s = open(&d);
    key(&mut s, "a", true, false);
    assert_eq!(s.set_style_text(StyleField::FillColor, "#00ff00"), Ok(true));
    assert_eq!(
        s.take_style_skip_notice(),
        "1 clone shows its original's style and was not changed."
    );
    assert_eq!(s.take_style_skip_notice(), "");
    let doc = doc_of(&s);
    let fill = |id| doc.object(id).unwrap().style().fill.color;
    assert_eq!(fill(p), Color { r: 0, g: 255, b: 0 });
    assert_eq!(fill(held), Color { r: 0, g: 255, b: 0 });
    // `free` shows the original's, which is now green, but nothing was written to it.
    assert!(is_clone(&s, free));
    if let Some(ObjectSnapshot::Clone(c)) = doc.object(free) {
        assert!(!c.chain.own_style);
    }
}

#[test]
fn ac42_spread_colours_disabled_with_one_clone_and_posts_nothing() {
    let (d, _, _, _) = scene();
    let mut s = open(&d);
    click(&mut s, pt(70.0, 20.0));
    assert!(!s.spread_colours_ready());
    let n = commits(&s);
    let r = s.spread_colours();
    assert_eq!((r.code(), r.notice.as_str()), ("ignored", ""));
    assert_eq!(commits(&s), n);
    key(&mut s, "a", true, false);
    // A path is in the selection: not a clone-only selection, so nothing is enabled.
    key(&mut s, "Escape", false, false);
    click(&mut s, pt(70.0, 20.0));
    click(&mut s, pt(70.0, 60.0));
    let mut two = open(&d);
    click(&mut two, pt(20.0, 20.0));
    two.select_clones();
    assert!(two.spread_colours_ready());
    let n = commits(&two);
    assert_eq!(two.spread_colours().code(), "done");
    assert_eq!(commits(&two), n + 1);
}

#[test]
fn ac33_undo_of_a_transform_that_held_a_clone_back_restores_it() {
    let (d, p, held, _) = scene();
    let mut s = open(&d);
    click(&mut s, pt(20.0, 20.0));
    let (p0, h0) = (out(&s, p), out(&s, held));
    drag(&mut s, pt(15.0, 15.0), pt(25.0, 22.0));
    assert_ne!(out(&s, p), p0);
    match key(&mut s, "z", true, false) {
        KeyOutcome::Undo => {}
        KeyOutcome::UndoBusy => {
            let _ = s.undo();
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(out(&s, p), p0);
    near(&out(&s, held), &h0, 1e-9);
}

#[test]
fn ac37_a_session_save_keeps_the_states() {
    let (d, _, held, free) = scene();
    d.set_own_style(&[held, free], true).unwrap();
    let s = open(&d);
    let again = doc_of(&s);
    for (id, placement) in [(held, true), (free, false)] {
        let Some(ObjectSnapshot::Clone(c)) = again.object(id) else {
            panic!()
        };
        assert_eq!(
            (c.chain.own_placement, c.chain.own_style),
            (placement, true)
        );
    }
}
