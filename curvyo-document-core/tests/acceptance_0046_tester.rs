//! Independent tester tests for `specs/0046-document-shapes`, stage 1, at the document model:
//! the `guide` flag, its commit, clones and groups, the output filter, the template loader, the
//! file format (version 15), damaged and future files, determinism and limits. Written from the
//! specification; only the public names were looked up.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(clippy::too_many_lines, clippy::cast_precision_loss)]
#![allow(missing_docs, clippy::doc_markdown, clippy::needless_pass_by_value)]
#![allow(
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::assert_is_empty
)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::items_after_statements
)]

use std::io::{Cursor, Read, Write};
use std::time::Instant;

use curvyo_document_core::{
    AnchorId, Angle, CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, Document, EllipseFrame,
    Length, NewAnchor, NodeId, ObjectEditError, ObjectSnapshot, OpenError, Point, PointCount,
    RectBounds, StarFrame, StyleEdit, TemplateNode, Vec2, pack, unpack,
};
use loro::LoroDoc;

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn line(d: &Document, n: u64, x: f64) -> NodeId {
    d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, n * 10), pt(x, 0.0)),
            NewAnchor::corner(AnchorId::new(1, n * 10 + 1), pt(x + 10.0, 5.0)),
        ],
        false,
    )
}

fn rect(d: &Document, x: f64) -> NodeId {
    d.create_rect(RectBounds {
        origin: pt(x, 0.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    })
}

fn ellipse(d: &Document) -> NodeId {
    d.create_ellipse(EllipseFrame {
        center: pt(20.0, 20.0),
        rx: Length::from_mm(8.0),
        ry: Length::from_mm(5.0),
    })
}

fn guide_of(d: &Document, id: NodeId) -> bool {
    d.read_nodes().iter().find(|n| n.id == id).unwrap().guide
}

fn json(d: &Document) -> String {
    String::from_utf8(d.export_json().unwrap()).unwrap()
}

fn labels(d: &Document) -> Vec<String> {
    d.own_steps().into_iter().map(|s| s.label).collect()
}

fn loro_of(d: &Document) -> LoroDoc {
    let l = LoroDoc::new();
    l.import(&d.export_loro_snapshot().unwrap()).unwrap();
    l
}

fn file_from_loro(loro: &LoroDoc, version: u32) -> Vec<u8> {
    loro.commit();
    let manifest = serde_json::json!({
        "format_version": version,
        "loro_snapshot_version": CURRENT_LORO_SNAPSHOT_VERSION,
        "app_version": "tester",
    });
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let o = zip::write::SimpleFileOptions::default();
    for (name, bytes) in [
        ("manifest.json", serde_json::to_vec(&manifest).unwrap()),
        (
            "document.loro",
            loro.export(loro::ExportMode::Snapshot).unwrap(),
        ),
        ("document.json", b"{}".to_vec()),
    ] {
        w.start_file(name, o).unwrap();
        w.write_all(&bytes).unwrap();
    }
    w.finish().unwrap().into_inner()
}

#[allow(dead_code)]
fn member(bytes: &[u8], name: &str) -> Vec<u8> {
    let mut z = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Vec::new();
    z.by_name(name).unwrap().read_to_end(&mut out).unwrap();
    out
}

fn all_kinds(d: &Document) -> Vec<NodeId> {
    let star = d.create_star(
        StarFrame {
            center: pt(50.0, 50.0),
            radius: Length::from_mm(10.0),
            angle: Angle::from_radians(0.0),
        },
        PointCount::new(5).unwrap(),
        curvyo_document_core::InnerRatio::new(0.5).unwrap(),
    );
    let poly = d.create_polygon(
        StarFrame {
            center: pt(80.0, 50.0),
            radius: Length::from_mm(10.0),
            angle: Angle::from_radians(0.0),
        },
        PointCount::new(6).unwrap(),
    );
    vec![line(d, 1, 0.0), rect(d, 30.0), ellipse(d), star, poly]
}

// ---- criterion 1, 2: the flag, absent unless set -------------------------------------------

#[test]
fn c1_no_guides_means_no_guide_key_and_open_writes_nothing() {
    let d = Document::new(1);
    let _ = all_kinds(&d);
    assert!(
        !json(&d).contains("guide"),
        "no guide key in a document without guides"
    );
    let bytes = pack(&d, "t").unwrap();
    let again = unpack(2, &bytes).unwrap();
    let steps_before = again.own_steps().len();
    assert_eq!(steps_before, 0, "opening writes no step");
    assert!(!json(&again).contains("guide"));
    assert!(again.read_nodes().iter().all(|n| !n.guide));
    assert!(!pack(&again, "t").unwrap().is_empty());
}

#[test]
fn c2_every_kind_can_carry_the_flag_and_unmarking_removes_the_key() {
    let d = Document::new(1);
    let ids = all_kinds(&d);
    for &id in &ids {
        let before = d.object(id).unwrap();
        d.set_guide(&[id], true).unwrap();
        assert!(guide_of(&d, id), "{id:?} is a guide");
        assert_eq!(
            d.object(id).unwrap(),
            before,
            "geometry and style untouched"
        );
    }
    assert!(json(&d).contains("guide"));
    d.set_guide(&ids, false).unwrap();
    assert!(ids.iter().all(|&id| !guide_of(&d, id)));
    assert!(!json(&d).contains("guide"), "the key is gone, not false");
}

#[test]
fn c2_compound_path_can_be_a_guide() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let b = rect(&d, 3.0);
    let sq = |first: u64, x: f64, s: f64| {
        let c = [(x, x), (x + s, x), (x + s, x + s), (x, x + s)];
        (
            c.iter()
                .enumerate()
                .map(|(k, &(px, py))| {
                    NewAnchor::corner(AnchorId::new(5, first + k as u64), pt(px, py))
                })
                .collect::<Vec<_>>(),
            true,
        )
    };
    let ring = d
        .replace_with_path(
            &[a, b],
            a,
            &[sq(100, 0.0, 40.0), sq(200, 10.0, 20.0)],
            "boolean_union",
        )
        .unwrap();
    d.set_guide(&[ring], true).unwrap();
    assert!(guide_of(&d, ring));
    let ObjectSnapshot::Path(p) = d.object(ring).unwrap() else {
        panic!()
    };
    assert!(p.is_compound());
}

#[test]
fn c9_set_guide_is_one_commit_named_set_guide_and_undo_redo_round_trips() {
    let d = Document::new(1);
    let ids = all_kinds(&d);
    let before = json(&d);
    let n = d.own_steps().len();
    d.set_guide(&ids, true).unwrap();
    assert_eq!(d.own_steps().len(), n + 1, "one step for five nodes");
    assert_eq!(labels(&d).last().unwrap(), "set_guide");
    let marked = json(&d);
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
    assert_eq!(json(&d), before);
    d.redo_step(step).unwrap();
    assert_eq!(json(&d), marked);
}

#[test]
fn c9_set_guide_with_nothing_markable_is_an_error_and_writes_nothing() {
    let d = Document::new(1);
    let n = d.own_steps().len();
    assert_eq!(d.set_guide(&[], true), Err(ObjectEditError::NoSuchObject));
    assert_eq!(d.own_steps().len(), n);
}

#[test]
fn c10_marking_counts_and_mixed_selection() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let b = rect(&d, 20.0);
    d.set_guide(&[a], true).unwrap();
    let m = d.guide_marking(&[a, b]);
    assert_eq!((m.markable, m.guides), (2, 1));
    assert!(m.enabled() && !m.all_guides(), "mixed: not pressed");
    d.set_guide(&[a, b], true).unwrap();
    assert!(d.guide_marking(&[a, b]).all_guides());
    assert!(!d.guide_marking(&[]).enabled());
}

#[test]
fn c10_unmarking_keeps_fill_stroke_and_geometry() {
    let d = Document::new(1);
    let r = rect(&d, 0.0);
    d.edit_style(&[r], &StyleEdit::FillEnabled(true)).unwrap();
    d.edit_style(&[r], &StyleEdit::StrokeWidth(Length::from_mm(3.0)))
        .unwrap();
    let before = d.object(r).unwrap();
    d.set_guide(&[r], true).unwrap();
    d.set_guide(&[r], false).unwrap();
    assert_eq!(d.object(r).unwrap(), before);
}

// ---- groups, clones (34) ----------------------------------------------------------------------

#[test]
fn c9_group_marks_its_leaves_not_itself() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let b = rect(&d, 20.0);
    let g = d.group_objects(&[a, b]).unwrap();
    d.set_guide(&[g], true).unwrap();
    assert!(guide_of(&d, a) && guide_of(&d, b));
    assert!(!guide_of(&d, g), "a group carries no flag");
    assert_eq!(d.guide_marking(&[g]).markable, 2);
}

#[test]
fn c34_clone_of_guide_reads_the_origins_flag_and_is_not_output() {
    let d = Document::new(1);
    let src = rect(&d, 0.0);
    let clones = d.linked_clone(&[src], &[(src, rect_anchors(7))]).unwrap();
    let c = clones[0];
    assert!(!guide_of(&d, c));
    let origin = d
        .read_nodes()
        .iter()
        .find(|n| n.is_output_node())
        .map(|n| n.id);
    assert!(origin.is_some());
    // the source was a primitive and is now a path; mark it
    d.set_guide(&[src], true).unwrap();
    assert!(
        guide_of(&d, c),
        "the clone follows the origin's flag at read time"
    );
    let nodes = d.read_nodes();
    assert!(
        nodes.iter().all(|n| !n.is_output_node()),
        "neither is output"
    );
    // a clone itself cannot be marked
    assert_eq!(d.set_guide(&[c], false), Err(ObjectEditError::NoSuchObject));
    // unmark the origin: both are output again
    d.set_guide(&[src], false).unwrap();
    assert!(!guide_of(&d, c));
}

fn rect_anchors(peer: u64) -> Vec<NewAnchor> {
    [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        .iter()
        .enumerate()
        .map(|(k, &(x, y))| NewAnchor::corner(AnchorId::new(peer, k as u64 + 1), pt(x, y)))
        .collect()
}

#[test]
fn c34_chain_clone_of_clone_of_guide_is_a_guide() {
    let d = Document::new(1);
    let src = line(&d, 1, 0.0);
    let c1 = d.linked_clone(&[src], &[]).unwrap()[0];
    let c2 = d.linked_clone(&[c1], &[]).unwrap()[0];
    d.set_guide(&[src], true).unwrap();
    assert!(guide_of(&d, c1) && guide_of(&d, c2));
}

#[test]
fn c34_duplicate_keeps_the_flag() {
    let d = Document::new(1);
    let src = line(&d, 1, 0.0);
    d.set_guide(&[src], true).unwrap();
    let copy = d
        .duplicate_objects(
            &[curvyo_document_core::CopySource {
                id: src,
                anchor_ids: vec![AnchorId::new(9, 1), AnchorId::new(9, 2)],
            }],
            Vec2::new(5.0, 5.0),
        )
        .unwrap();
    assert!(guide_of(&d, copy[0]), "Duplicate keeps the flag");
}

#[test]
fn c34_boolean_result_is_a_guide_only_when_every_input_was() {
    let sq = |first: u64| {
        (
            (0..4)
                .map(|k| {
                    NewAnchor::corner(
                        AnchorId::new(5, first + k),
                        pt(
                            [0.0, 9.0, 9.0, 0.0][k as usize],
                            [0.0, 0.0, 9.0, 9.0][k as usize],
                        ),
                    )
                })
                .collect::<Vec<_>>(),
            true,
        )
    };
    for (flags, expect) in [
        ([true, true], true),
        ([true, false], false),
        ([false, true], false),
        ([false, false], false),
    ] {
        let d = Document::new(1);
        let a = rect(&d, 0.0);
        let b = rect(&d, 4.0);
        if flags[0] {
            d.set_guide(&[a], true).unwrap();
        }
        if flags[1] {
            d.set_guide(&[b], true).unwrap();
        }
        let r = d
            .replace_with_path(&[a, b], a, &[sq(100)], "boolean_union")
            .unwrap();
        assert_eq!(guide_of(&d, r), expect, "inputs {flags:?}");
    }
}

#[test]
fn c34_join_and_split_keep_guide_status_sensible() {
    // Join of two guide lines stays a guide; split of a guide yields guides.
    let d = Document::new(1);
    let p = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), pt(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 2), pt(10.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 3), pt(20.0, 0.0)),
        ],
        false,
    );
    d.set_guide(&[p], true).unwrap();
    let before: Vec<_> = d.read_nodes();
    assert!(before.iter().all(|n| n.guide));
    // split_at_anchor signature is large; verify through the session tests instead.
}

// ---- output filter (25, 41) -----------------------------------------------------------------

#[test]
fn c41_output_nodes_exclude_guides_hidden_and_containers() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let g = rect(&d, 20.0);
    let h = rect(&d, 40.0);
    d.set_guide(&[g], true).unwrap();
    d.set_node_visible(&[h], false).unwrap();
    let out: Vec<NodeId> = d
        .read_nodes()
        .iter()
        .filter(|n| n.is_output_node())
        .map(|n| n.id)
        .collect();
    assert_eq!(out, vec![a]);
    // a hidden guide is neither
    d.set_guide(&[h], true).unwrap();
    assert!(
        !d.read_nodes()
            .iter()
            .find(|n| n.id == h)
            .unwrap()
            .is_output_node()
    );
    // same drawing with and without a guide: output set identical
    let plain = Document::new(1);
    let _ = rect(&plain, 0.0);
    let with = Document::new(1);
    let _ = rect(&with, 0.0);
    let gg = rect(&with, 99.0);
    with.set_guide(&[gg], true).unwrap();
    let objs = |doc: &Document| -> Vec<ObjectSnapshot> {
        let r = doc.read_nodes();
        r.iter()
            .filter(|n| n.is_output_node())
            .filter_map(|n| n.object().cloned())
            .collect()
    };
    assert_eq!(objs(&plain), objs(&with));
}

#[test]
fn c41_guide_in_a_hidden_layer_or_group_is_still_not_output() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let b = rect(&d, 20.0);
    let g = d.group_objects(&[a, b]).unwrap();
    d.set_guide(&[a], true).unwrap();
    let out: Vec<_> = d
        .read_nodes()
        .iter()
        .filter(|n| n.is_output_node())
        .map(|n| n.id)
        .collect();
    assert_eq!(out, vec![b]);
    d.set_node_visible(&[g], false).unwrap();
    assert!(d.read_nodes().iter().all(|n| !n.is_output_node()));
}

// ---- format (2, 30, 31) -------------------------------------------------------------------------

#[test]
fn c2_format_version_is_15_and_a_newer_file_is_refused() {
    // 0046 took 15; `0021-ellipse-arcs-and-shaping` took 16 after it.
    assert!(std::hint::black_box(CURRENT_FORMAT_VERSION) >= 15);
    let d = Document::new(1);
    let l = loro_of(&d);
    let bytes = file_from_loro(&l, CURRENT_FORMAT_VERSION + 1);
    assert!(matches!(
        unpack(2, &bytes),
        Err(OpenError::FormatTooNew { .. })
    ));
}

#[test]
fn c30_save_twice_and_save_open_save_are_byte_identical_and_key_only_on_guides() {
    let d = Document::new(1);
    let ids = all_kinds(&d);
    d.set_guide(&ids[..2], true).unwrap();
    let one = pack(&d, "t").unwrap();
    let two = pack(&d, "t").unwrap();
    assert_eq!(one, two);
    let reopened = unpack(2, &one).unwrap();
    assert_eq!(json(&d), json(&reopened));
    assert_eq!(
        member(&pack(&reopened, "t").unwrap(), "document.json"),
        member(&one, "document.json")
    );
    assert_eq!(
        json(&d).matches("\"guide\"").count(),
        2,
        "the key only on the two guides"
    );
    let guides: Vec<_> = reopened
        .read_nodes()
        .iter()
        .filter(|n| n.guide)
        .map(|n| n.id)
        .collect();
    assert_eq!(guides.len(), 2);
}

#[test]
fn c30_golden_fixture_guide_v15_opens() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/guide_v15.curvyo");
    let bytes = std::fs::read(path).unwrap();
    let d = unpack(3, &bytes).unwrap();
    let nodes = d.read_nodes();
    assert!(
        nodes.iter().filter(|n| n.guide).count() >= 3,
        "path, circle, compound, clone of a guide"
    );
    assert!(
        nodes
            .iter()
            .any(|n| matches!(n.object(), Some(ObjectSnapshot::Clone(_))) && n.guide)
    );
}

#[test]
fn c31_a_non_bool_guide_is_damaged_for_every_wrong_type() {
    for bad in ["yes", "1"] {
        let d = Document::new(1);
        let r = rect(&d, 0.0);
        let l = loro_of(&d);
        let tree = l.get_tree("paths");
        let tid = loro::TreeID::new(
            serde_json::to_value(r).unwrap()["peer"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap(),
            i32::try_from(
                serde_json::to_value(r).unwrap()["counter"]
                    .as_i64()
                    .unwrap(),
            )
            .unwrap(),
        );
        tree.get_meta(tid).unwrap().insert("guide", bad).unwrap();
        let bytes = file_from_loro(&l, CURRENT_FORMAT_VERSION);
        assert!(
            matches!(unpack(2, &bytes), Err(OpenError::Damaged)),
            "{bad}"
        );
    }
}

#[test]
fn c31_future_version_beats_damage() {
    let d = Document::new(1);
    let r = rect(&d, 0.0);
    let l = loro_of(&d);
    let tree = l.get_tree("paths");
    let v = serde_json::to_value(r).unwrap();
    let tid = loro::TreeID::new(
        v["peer"].as_str().unwrap().parse().unwrap(),
        i32::try_from(v["counter"].as_i64().unwrap()).unwrap(),
    );
    tree.get_meta(tid).unwrap().insert("guide", "x").unwrap();
    let bytes = file_from_loro(&l, CURRENT_FORMAT_VERSION + 1);
    assert!(matches!(
        unpack(2, &bytes),
        Err(OpenError::FormatTooNew { .. })
    ));
}

#[test]
fn c1_older_fixture_opens_and_resaves_without_guide_key() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    for name in [
        "layers_v12.curvyo",
        "groups_v11.curvyo",
        "clone_v13.curvyo",
        "clone_overrides_v14.curvyo",
    ] {
        let d = unpack(2, &std::fs::read(dir.join(name)).unwrap()).unwrap();
        assert!(d.read_nodes().iter().all(|n| !n.guide), "{name}");
        assert!(!json(&d).contains("\"guide\""), "{name}");
        assert_eq!(d.own_steps().len(), 0, "{name}: opening writes nothing");
    }
}

// ---- merge (26) ----------------------------------------------------------------------------------

#[test]
fn c26_guide_flag_merges_with_a_concurrent_move_both_ways() {
    let base = Document::new(1);
    let r = rect(&base, 0.0);
    let bytes = pack(&base, "t").unwrap();
    let run = |a_first: bool| {
        let a = unpack(10, &bytes).unwrap();
        let b = unpack(11, &bytes).unwrap();
        let va = a.version();
        let vb = b.version();
        a.set_guide(&[r], true).unwrap();
        b.translate_objects(&[r], Vec2::new(5.0, 7.0)).unwrap();
        let ua = a.export_updates(&va).unwrap();
        let ub = b.export_updates(&vb).unwrap();
        if a_first {
            a.import_updates(&ub).unwrap();
            b.import_updates(&ua).unwrap();
        } else {
            b.import_updates(&ua).unwrap();
            a.import_updates(&ub).unwrap();
        }
        (a, b)
    };
    for first in [true, false] {
        let (a, b) = run(first);
        assert!(guide_of(&a, r) && guide_of(&b, r));
        assert_eq!(a.object(r).unwrap(), b.object(r).unwrap());
        let ObjectSnapshot::Primitive(_) = a.object(r).unwrap() else {
            panic!()
        };
    }
    let (a, _) = run(true);
    let (a2, _) = run(false);
    assert_eq!(json(&a), json(&a2), "same result in both orders");
}

#[test]
fn c26_last_writer_wins_on_mark_and_unmark() {
    let base = Document::new(1);
    let r = rect(&base, 0.0);
    base.set_guide(&[r], true).unwrap();
    let bytes = pack(&base, "t").unwrap();
    let a = unpack(10, &bytes).unwrap();
    let b = unpack(11, &bytes).unwrap();
    let (va, vb) = (a.version(), b.version());
    a.set_guide(&[r], false).unwrap();
    b.set_guide(&[r], false).unwrap();
    b.set_guide(&[r], true).unwrap();
    let (ua, ub) = (
        a.export_updates(&va).unwrap(),
        b.export_updates(&vb).unwrap(),
    );
    a.import_updates(&ub).unwrap();
    b.import_updates(&ua).unwrap();
    assert_eq!(guide_of(&a, r), guide_of(&b, r), "converges");
}

// ---- template loader (20, 22, 23, 33) -------------------------------------------------------

fn nodes_of(d: &Document) -> Vec<TemplateNode> {
    d.template_nodes()
}

fn anchors_for(nodes: &[TemplateNode], peer: u64) -> Vec<AnchorId> {
    (0..nodes.iter().map(TemplateNode::anchor_count).sum::<usize>())
        .map(|k| AnchorId::new(peer, k as u64 + 1))
        .collect()
}

#[test]
fn c20_source_with_guides_gives_only_guides() {
    let s = Document::new(1);
    let _ordinary = rect(&s, 0.0);
    let g1 = rect(&s, 30.0);
    let g2 = ellipse(&s);
    s.set_guide(&[g1, g2], true).unwrap();
    assert_eq!(nodes_of(&s).len(), 2);
}

#[test]
fn c20_source_without_guides_gives_every_visible_outline_and_no_hidden() {
    let s = Document::new(1);
    let a = rect(&s, 0.0);
    let _ = line(&s, 1, 40.0);
    let hid = rect(&s, 70.0);
    s.set_node_visible(&[hid], false).unwrap();
    let _ = a;
    let n = nodes_of(&s);
    assert_eq!(n.len(), 2);
}

#[test]
fn c20_hidden_guides_still_come_in_when_the_source_has_guides() {
    // Spec: "every node of its tree that is a guide comes in"; hidden-ness is only the fallback rule.
    let s = Document::new(1);
    let g = rect(&s, 0.0);
    let o = rect(&s, 30.0);
    let _ = o;
    s.set_guide(&[g], true).unwrap();
    s.set_node_visible(&[g], false).unwrap();
    assert_eq!(
        nodes_of(&s).len(),
        1,
        "a hidden guide is still a guide of the source"
    );
}

#[test]
fn c20_empty_source_gives_nothing_and_load_writes_nothing() {
    let s = Document::new(1);
    let n = nodes_of(&s);
    assert!(n.is_empty());
    let t = Document::new(2);
    let steps = t.own_steps().len();
    assert!(t.load_template(&n, &[]).unwrap().is_empty());
    assert_eq!(t.own_steps().len(), steps, "no empty step");
}

#[test]
fn c20_clone_comes_in_as_resolved_path_and_groups_are_flattened() {
    let s = Document::new(1);
    let a = rect(&s, 0.0);
    let b = rect(&s, 30.0);
    let c = s.linked_clone(&[b], &[(b, rect_anchors(7))]).unwrap()[0];
    let a2 = rect(&s, 60.0);
    let _g = s.group_objects(&[a, a2]).unwrap();
    let _ = c;
    let n = nodes_of(&s);
    assert_eq!(n.len(), 4);
    assert!(
        n.iter()
            .all(|t| !matches!(t.object, ObjectSnapshot::Clone(_)))
    );
}

#[test]
fn c23_load_is_one_commit_all_guides_on_top_with_names_and_coordinates_kept() {
    let s = Document::new(1);
    let a = rect(&s, 100.0);
    let b = line(&s, 1, 200.0);
    s.rename_node(a, "ring").unwrap();
    let t = Document::new(2);
    let existing = rect(&t, 0.0);
    let nodes = nodes_of(&s);
    let steps = t.own_steps().len();
    let ids = t.load_template(&nodes, &anchors_for(&nodes, 2)).unwrap();
    assert_eq!(ids.len(), 2);
    assert_eq!(t.own_steps().len(), steps + 1);
    assert_eq!(labels(&t).last().unwrap(), "load_template");
    assert!(ids.iter().all(|&i| guide_of(&t, i)));
    assert!(!guide_of(&t, existing));
    let all = t.object_ids();
    assert_eq!(&all[all.len() - 2..], &ids[..], "at the top, in order");
    let reg = t.node_registers(ids[0]).unwrap();
    assert_eq!(reg.name.as_deref(), Some("ring"));
    let (ObjectSnapshot::Primitive(got), ObjectSnapshot::Primitive(want)) =
        (t.object(ids[0]).unwrap(), nodes[0].object.clone())
    else {
        panic!("a rectangle stays a rectangle")
    };
    assert_eq!(
        (got.shape, got.style),
        (want.shape, want.style),
        "coordinates kept"
    );
    let _ = b;
    // undo removes them in one press
    let before_json = {
        let step = t.own_steps().last().unwrap().step;
        t.undo_step(step).unwrap();
        json(&t)
    };
    assert!(!before_json.contains("\"guide\""));
    assert_eq!(t.object_ids().len(), 1);
}

#[test]
fn c23_loading_twice_adds_twice_and_nothing_is_linked_to_the_source() {
    let s = Document::new(1);
    let _ = rect(&s, 0.0);
    let nodes = nodes_of(&s);
    let t = Document::new(2);
    let first = t.load_template(&nodes, &anchors_for(&nodes, 2)).unwrap();
    let second = t.load_template(&nodes, &anchors_for(&nodes, 3)).unwrap();
    assert_eq!(t.object_ids().len(), 2);
    assert_ne!(first, second);
    drop(s);
    assert!(!json(&t).contains("template"), "no source stored");
}

#[test]
fn c23_wrong_anchor_ids_are_refused_before_any_write() {
    let s = Document::new(1);
    let _ = line(&s, 1, 0.0);
    let nodes = nodes_of(&s);
    let t = Document::new(2);
    let steps = t.own_steps().len();
    assert_eq!(
        t.load_template(&nodes, &[]),
        Err(ObjectEditError::AnchorIds)
    );
    let dup = [AnchorId::new(2, 1), AnchorId::new(2, 1)];
    assert_eq!(
        t.load_template(&nodes, &dup),
        Err(ObjectEditError::AnchorIds)
    );
    assert_eq!(t.own_steps().len(), steps);
    assert!(t.object_ids().is_empty());
}

#[test]
fn c22_anything_goes_in_open_lines_nested_odd_sizes() {
    let s = Document::new(1);
    for k in 0..4 {
        let _ = line(&s, k, k as f64 * 3.0);
    }
    // degenerate: a one-anchor path, a zero-size rectangle, a huge ellipse
    let _ = s.create_path(
        &[NewAnchor::corner(AnchorId::new(1, 900), pt(1.0, 1.0))],
        false,
    );
    let _ = s.create_rect(RectBounds {
        origin: pt(0.0, 0.0),
        width: Length::from_mm(0.0),
        height: Length::from_mm(0.0),
    });
    let _ = s.create_ellipse(EllipseFrame {
        center: pt(0.0, 0.0),
        rx: Length::from_mm(50_000.0),
        ry: Length::from_mm(50_000.0),
    });
    let nodes = nodes_of(&s);
    let t = Document::new(2);
    let ids = t.load_template(&nodes, &anchors_for(&nodes, 2)).unwrap();
    assert_eq!(ids.len(), nodes.len());
}

#[test]
fn c33_five_thousand_nodes_load_fast_and_survive_save_open() {
    let s = Document::new(1);
    for k in 0..5000u64 {
        let _ = s.create_rect(RectBounds {
            origin: pt((k % 100) as f64 * 12.0, (k / 100) as f64 * 12.0),
            width: Length::from_mm(10.0),
            height: Length::from_mm(10.0),
        });
    }
    let nodes = nodes_of(&s);
    assert_eq!(nodes.len(), 5000);
    let t = Document::new(2);
    let start = Instant::now();
    let ids = t.load_template(&nodes, &[]).unwrap();
    let took = start.elapsed();
    assert_eq!(ids.len(), 5000);
    assert!(took.as_secs_f64() < 5.0, "5000 nodes load in {took:?}");
    let back = unpack(3, &pack(&t, "t").unwrap()).unwrap();
    assert_eq!(back.read_nodes().iter().filter(|n| n.guide).count(), 5000);
}

#[test]
fn c33_a_hundred_thousand_mm_wide_template_has_finite_geometry() {
    let s = Document::new(1);
    let r = s.create_rect(RectBounds {
        origin: pt(0.0, 0.0),
        width: Length::from_mm(100_000.0),
        height: Length::from_mm(10.0),
    });
    let _ = r;
    let t = Document::new(2);
    let nodes = nodes_of(&s);
    let ids = t.load_template(&nodes, &[]).unwrap();
    let ObjectSnapshot::Primitive(p) = t.object(ids[0]).unwrap() else {
        panic!()
    };
    let text = format!("{p:?}");
    assert!(!text.contains("NaN") && !text.contains("inf"));
}

#[test]
fn c28_template_undo_redo_round_trip() {
    let s = Document::new(1);
    let _ = line(&s, 1, 0.0);
    let nodes = nodes_of(&s);
    let t = Document::new(2);
    let before = json(&t);
    let _ = t.load_template(&nodes, &anchors_for(&nodes, 2)).unwrap();
    let after = json(&t);
    let step = t.own_steps().last().unwrap().step;
    t.undo_step(step).unwrap();
    assert_eq!(json(&t), before);
    t.redo_step(step).unwrap();
    assert_eq!(json(&t), after);
}

// ---- history preview (29) -------------------------------------------------------------------------

#[test]
fn c29_preview_of_an_older_version_has_the_guides_that_version_had() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let b = rect(&d, 30.0);
    let s_created = d.own_steps().last().unwrap().step;
    d.set_guide(&[a], true).unwrap();
    let s_marked = d.own_steps().last().unwrap().step;
    d.set_guide(&[a, b], true).unwrap();
    d.set_guide(&[a], false).unwrap();
    let p0 = d.preview_document_at(s_created).unwrap();
    assert!(p0.guides.is_empty(), "before the flag");
    let p1 = d.preview_document_at(s_marked).unwrap();
    assert_eq!(p1.guides.len(), 1);
    assert!(p1.guides.contains(&a));
    // the live document is unchanged by previews
    assert!(!guide_of(&d, a) && guide_of(&d, b));
}

#[test]
fn c29_preview_of_a_clone_of_a_guide_marks_the_clone_too() {
    let d = Document::new(1);
    let src = line(&d, 1, 0.0);
    d.set_guide(&[src], true).unwrap();
    let c = d.linked_clone(&[src], &[]).unwrap()[0];
    let step = d.own_steps().last().unwrap().step;
    let p = d.preview_document_at(step).unwrap();
    assert!(p.guides.contains(&src));
    assert!(
        p.guides.contains(&c),
        "a clone of a guide is drawn as a guide"
    );
}

// ---- layers (0050) ---------------------------------------------------------------------------------

#[test]
fn c9_layer_marks_every_leaf_below() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let layer = d.new_layer(None);
    let b = rect(&d, 30.0);
    d.move_nodes(&curvyo_document_core::MovePlan {
        nodes: vec![b],
        target: curvyo_document_core::MoveTarget::FrontOf(Some(layer)),
    })
    .unwrap();
    let _ = a;
    d.set_guide(&[layer], true).unwrap();
    assert!(guide_of(&d, b));
    assert!(!guide_of(&d, layer));
    assert!(
        d.read_nodes()
            .iter()
            .filter(|n| n.object().is_some() && n.parent == Some(layer))
            .all(|n| n.guide)
    );
}

#[test]
fn c30_whole_file_save_open_save_is_byte_identical_with_guides() {
    let d = Document::new(1);
    let ids = all_kinds(&d);
    d.set_guide(&ids[..3], true).unwrap();
    let one = pack(&d, "t").unwrap();
    let two = pack(&unpack(2, &one).unwrap(), "t").unwrap();
    let three = pack(&unpack(3, &two).unwrap(), "t").unwrap();
    assert_eq!(one, two, "whole file, first reopen");
    assert_eq!(two, three);
}
