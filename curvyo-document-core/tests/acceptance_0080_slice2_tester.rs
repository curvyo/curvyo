//! Independent tester cases for `specs/0080-text-on-path/specification.md` slice 2 on
//! `curvyo-document-core` (ADR 0022 §4): Put text on paths (criterion 37), content edits on any
//! member write the source (38), per-clone placement and Release (39), own style and Spread
//! colours (40), Unlink and the removed source (41), the emptied string (41a), linked and text
//! clones together (41b), Duplicate, two peers, orphan repair, format 23 and older files.
//!
//! Written from the specification, ADR 0022 and the public names before the implementation was
//! read.

#![allow(
    clippy::many_single_char_names,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    missing_docs
)]

use std::io::{Cursor, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, AnchorId, CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, CarrierRef,
    CharValues, CloneColours, Color, CopySource, Document, EllipseFrame, FontKind, FontRef, Length,
    NewAnchor, NodeId, ObjectSnapshot, OnPathLabel, OnPathValues, PathSide, Point, StyleEdit,
    TextChars, TextCharsLabel, TextLayoutLabel, TextLayoutValues, TextSnapshot, Vec2, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeID};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn face(family: &str) -> FontRef {
    FontRef {
        family: family.to_owned(),
        style: "Regular".to_owned(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind: FontKind::Outline,
    }
}

fn circle_at(d: &Document, cx: f64) -> NodeId {
    d.create_ellipse(EllipseFrame {
        center: pt(cx, 100.0),
        rx: Length::from_mm(50.0),
        ry: Length::from_mm(50.0),
    })
}

const STRAIGHT: Affine = Affine::translation(Vec2::new(10.0, 20.0));

fn text(d: &Document, s: &str) -> NodeId {
    d.create_text(
        s,
        &face("Source Sans 3"),
        Length::from_mm(10.0),
        STRAIGHT,
        &TextLayoutValues::DEFAULT,
    )
    .unwrap()
}

fn on(node: NodeId) -> CarrierRef {
    CarrierRef { node, outline: 0 }
}

fn at(mm: f64) -> OnPathValues {
    OnPathValues::at(Length::from_mm(mm))
}

/// A source text "Ring" on three circles: (doc, source, clones, circles).
fn three() -> (Document, NodeId, Vec<NodeId>, Vec<NodeId>) {
    let d = Document::new(1);
    let cs: Vec<NodeId> = [100.0, 250.0, 400.0]
        .iter()
        .map(|&x| circle_at(&d, x))
        .collect();
    let t = text(&d, "Ring");
    let carriers: Vec<(CarrierRef, OnPathValues)> = cs.iter().map(|&c| (on(c), at(78.5))).collect();
    let clones = d.put_text_on_paths(t, &carriers, true).unwrap();
    (d, t, clones, cs)
}

fn snap(d: &Document, id: NodeId) -> TextSnapshot {
    d.text(id).unwrap()
}

fn guide_of(d: &Document, id: NodeId) -> bool {
    d.read_nodes().iter().find(|n| n.id == id).unwrap().guide
}

fn labels(d: &Document) -> Vec<String> {
    d.own_steps().into_iter().map(|s| s.label).collect()
}

fn undo_last(d: &Document) {
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
}

fn red() -> Color {
    Color { r: 200, g: 0, b: 0 }
}

fn blue() -> Color {
    Color { r: 0, g: 0, b: 200 }
}

fn src(id: NodeId) -> CopySource {
    CopySource {
        id,
        anchor_ids: Vec::new(),
    }
}

fn texts_in(d: &Document) -> Vec<TextSnapshot> {
    d.read_objects()
        .objects
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Text(t) => Some((**t).clone()),
            _ => None,
        })
        .collect()
}

fn tree_id(id: NodeId) -> TreeID {
    let json = serde_json::to_value(id).unwrap();
    TreeID::new(
        json["peer"].as_str().unwrap().parse().unwrap(),
        i32::try_from(json["counter"].as_i64().unwrap()).unwrap(),
    )
}

fn file_from_loro(loro: &LoroDoc) -> Vec<u8> {
    loro.commit();
    let manifest = serde_json::json!({
        "format_version": CURRENT_FORMAT_VERSION,
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

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

/// Criterion 37: the text goes on the first carrier, a text clone per further carrier with its own
/// placement on it, every carrier a guide, one step "Put text on paths"; Undo removes the clones
/// and gives back the straight text and the carriers' roles.
#[test]
fn c37_put_on_paths_first_carrier_takes_the_text_others_get_clones_one_step() {
    let d = Document::new(1);
    let cs: Vec<NodeId> = [100.0, 250.0, 400.0]
        .iter()
        .map(|&x| circle_at(&d, x))
        .collect();
    let t = text(&d, "Ring");
    let n = d.own_steps().len();
    let carriers: Vec<(CarrierRef, OnPathValues)> = cs
        .iter()
        .enumerate()
        .map(|(i, &c)| (on(c), at(10.0 + f64::from(u8::try_from(i).unwrap()))))
        .collect();
    let clones = d.put_text_on_paths(t, &carriers, true).unwrap();
    assert_eq!(d.own_steps().len(), n + 1, "one step");
    assert_eq!(labels(&d).last().unwrap(), "put_text_on_paths");
    assert_eq!(clones.len(), 2, "a clone per further carrier");
    let s = snap(&d, t);
    assert!(s.source.is_none(), "the picked text is the source");
    assert_eq!(
        s.on_path.as_ref().unwrap().carrier.node,
        cs[0],
        "first carrier"
    );
    for (k, &c) in clones.iter().enumerate() {
        let cl = snap(&d, c);
        assert_eq!(cl.source.unwrap().node, t);
        assert!(!cl.source.unwrap().own_style, "Original's style by default");
        let p = cl.on_path.as_ref().unwrap();
        assert_eq!(p.carrier.node, cs[k + 1]);
        assert!(p.curve.is_some());
        assert!(
            (p.values.position.as_mm() - (11.0 + f64::from(u8::try_from(k).unwrap()))).abs() < 1e-9
        );
        assert_eq!(cl.string, "Ring", "reads the source string");
        assert_eq!(cl.size, s.size);
        assert_eq!(cl.font, s.font);
    }
    for &c in &cs {
        assert!(guide_of(&d, c), "every carrier is a guide");
    }
    undo_last(&d);
    for &c in &clones {
        assert!(d.text(c).is_none(), "Undo removes the clones");
    }
    assert!(snap(&d, t).on_path.is_none());
    assert_eq!(snap(&d, t).placement, STRAIGHT);
    for &c in &cs {
        assert!(!guide_of(&d, c));
    }
}

/// Criterion 38: string, kerns, font, size, layout and style edited from a clone write the
/// source, every member shows it, one step each.
#[test]
fn c38_content_edits_from_a_clone_write_the_source_for_all() {
    let (d, t, clones, _) = three();
    let all = [t, clones[0], clones[1]];
    let check = |what: &str, f: &dyn Fn(&TextSnapshot) -> bool| {
        for &id in &all {
            assert!(f(&snap(&d, id)), "{what} on {id:?}");
        }
    };
    let step = |f: &dyn Fn()| {
        let n = d.own_steps().len();
        f();
        assert_eq!(d.own_steps().len(), n + 1, "one step for all");
    };
    step(&|| {
        d.edit_text(clones[1], "Rings", &TextChars::default())
            .unwrap();
    });
    check("string", &|s| s.string == "Rings");
    let kern = TextChars::new([(
        1,
        CharValues {
            kern_x: 0.7,
            ..CharValues::DEFAULT
        },
    )]);
    step(&|| {
        d.set_text_chars(&[(clones[0], kern.clone())], TextCharsLabel::Kerning)
            .unwrap();
    });
    check("kern", &|s| s.chars == kern);
    step(&|| {
        d.set_text_fonts(&[(clones[0], face("Other Face"))])
            .unwrap();
    });
    check("font", &|s| s.font.family == "Other Face");
    step(&|| d.set_text_size(&[clones[1]], Length::from_mm(7.5)).unwrap());
    check("size", &|s| (s.size.as_mm() - 7.5).abs() < 1e-9);
    let layout = TextLayoutValues {
        letter_spacing: 0.25,
        ..TextLayoutValues::DEFAULT
    };
    let clone_place = snap(&d, clones[0]).placement;
    step(&|| {
        d.set_text_layout(
            &[(clones[0], layout, clone_place)],
            TextLayoutLabel::LetterSpacing,
        )
        .unwrap();
    });
    check("layout", &|s| s.layout.letter_spacing == 0.25);
    // The style of the source reaches every clone that has Original's style.
    step(&|| d.edit_style(&[t], &StyleEdit::FillColor(red())).unwrap());
    check("style", &|s| s.style.fill.color == red());
    // Every clone still reads its own carrier.
    assert_ne!(
        snap(&d, clones[0]).on_path.unwrap().carrier.node,
        snap(&d, t).on_path.unwrap().carrier.node
    );
}

/// Criterion 39: Position, Side and Release on one clone change that clone alone; a released
/// clone is straight and still follows the source string (criterion 41, last sentence).
#[test]
fn c39_placement_is_each_clones_own_and_release_keeps_the_link() {
    let (d, t, clones, _) = three();
    let mine = OnPathValues {
        side: PathSide::Below,
        ..at(33.0)
    };
    d.set_text_path(&[(clones[0], mine)], OnPathLabel::Values)
        .unwrap();
    assert_eq!(snap(&d, clones[0]).on_path.unwrap().values, mine);
    assert_eq!(
        snap(&d, t).on_path.unwrap().values,
        at(78.5),
        "source untouched"
    );
    assert_eq!(snap(&d, clones[1]).on_path.unwrap().values, at(78.5));

    let straight = Affine::translation(Vec2::new(300.0, 300.0));
    d.release_from_path(&[(clones[1], straight)]).unwrap();
    let r = snap(&d, clones[1]);
    assert!(r.on_path.is_none());
    assert_eq!(r.placement, straight);
    assert_eq!(r.source.unwrap().node, t, "still a clone");
    assert!(
        snap(&d, t).on_path.is_some(),
        "the source stays on its path"
    );
    d.edit_text(t, "Changed", &TextChars::default()).unwrap();
    assert_eq!(snap(&d, clones[1]).string, "Changed");
}

/// Criterion 40: Own style on one text clone lets it have its own fill; the source's later style
/// changes reach only the clones with Original's style; back to Original's follows again.
#[test]
fn c40_own_style_per_text_clone() {
    let (d, t, clones, _) = three();
    let n = d.own_steps().len();
    d.set_own_style_and_texts(&[], &[clones[0]], true).unwrap();
    assert_eq!(d.own_steps().len(), n + 1);
    assert!(snap(&d, clones[0]).source.unwrap().own_style);
    d.edit_style(&[clones[0]], &StyleEdit::FillColor(blue()))
        .unwrap();
    d.edit_style(&[t], &StyleEdit::FillColor(red())).unwrap();
    assert_eq!(snap(&d, clones[0]).style.fill.color, blue(), "own colour");
    assert_eq!(snap(&d, clones[1]).style.fill.color, red(), "follows");
    assert_eq!(snap(&d, t).style.fill.color, red());
    d.set_own_style_and_texts(&[], &[clones[0]], false).unwrap();
    assert_eq!(
        snap(&d, clones[0]).style.fill.color,
        red(),
        "Original's again"
    );
}

/// Criteria 40 and 41b: Spread colours recolours text clones (taking own style) in one step,
/// together with a linked clone.
#[test]
fn c40_c41b_spread_colours_takes_text_clones_with_linked_clones_one_step() {
    let (d, t, clones, _) = three();
    let path = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(9, 1), pt(0.0, 300.0)),
            NewAnchor::corner(AnchorId::new(9, 2), pt(50.0, 300.0)),
        ],
        false,
    );
    let linked = d.linked_clone(&[path], &[]).unwrap()[0];
    let n = d.own_steps().len();
    let colours = [
        CloneColours {
            clone: clones[0],
            fill: blue(),
            stroke: None,
        },
        CloneColours {
            clone: clones[1],
            fill: red(),
            stroke: None,
        },
        CloneColours {
            clone: linked,
            fill: red(),
            stroke: None,
        },
    ];
    assert_eq!(d.recolour_clones(&colours).unwrap(), 3);
    assert_eq!(d.own_steps().len(), n + 1, "one step");
    assert_eq!(snap(&d, clones[0]).style.fill.color, blue());
    assert_eq!(snap(&d, clones[1]).style.fill.color, red());
    assert_ne!(
        snap(&d, t).style.fill.color,
        blue(),
        "the source is not written"
    );
    assert_eq!(snap(&d, clones[0]).string, "Ring", "still clones");
    assert!(snap(&d, clones[0]).source.is_some());
}

/// Criterion 41: Unlink clone makes an independent text on its carrier with the values copied
/// and the same placement; the source's later edits no longer reach it.
#[test]
fn c41_unlink_text_clone_makes_an_independent_text() {
    let (d, t, clones, _) = three();
    d.edit_style(&[t], &StyleEdit::FillColor(red())).unwrap();
    let before = snap(&d, clones[0]);
    let n = d.own_steps().len();
    d.unlink_clones_and_texts(&[], &[clones[0]]).unwrap();
    assert_eq!(d.own_steps().len(), n + 1);
    let u = snap(&d, clones[0]);
    assert!(u.source.is_none());
    assert_eq!(u.string, before.string);
    assert_eq!(u.font, before.font);
    assert_eq!(u.size, before.size);
    assert_eq!(u.style, before.style);
    assert_eq!(u.layout, before.layout);
    assert_eq!(
        u.on_path.as_ref().unwrap().values,
        before.on_path.as_ref().unwrap().values
    );
    assert_eq!(u.on_path.unwrap().carrier, before.on_path.unwrap().carrier);
    d.edit_text(t, "Source only", &TextChars::default())
        .unwrap();
    assert_eq!(snap(&d, clones[0]).string, "Ring");
    assert_eq!(
        snap(&d, clones[1]).string,
        "Source only",
        "the other clone follows"
    );
}

/// Criterion 41: removing the source turns each clone into a text in the same step with the
/// count for the notice; Undo brings back the source and the links.
#[test]
fn c41_removed_source_unlinks_its_clones_in_the_same_step() {
    let (d, t, clones, _) = three();
    let n = d.own_steps().len();
    d.delete_objects(&[t]).unwrap();
    assert_eq!(d.own_steps().len(), n + 1);
    assert!(d.text(t).is_none());
    for &c in &clones {
        let s = snap(&d, c);
        assert!(s.source.is_none(), "an independent text");
        assert_eq!(s.string, "Ring");
        assert!(s.on_path.unwrap().curve.is_some(), "still on its carrier");
    }
    assert_eq!(
        d.take_text_clones_unlinked(),
        2,
        "notice: 2 text clones became texts."
    );
    undo_last(&d);
    assert!(d.text(t).is_some());
    for &c in &clones {
        assert_eq!(snap(&d, c).source.unwrap().node, t, "links back");
    }
}

/// Criterion 41a: an emptied clone is removed alone; the source and the other clone keep the
/// string. One step "Edit text"; Undo brings it back.
#[test]
fn c41a_emptied_clone_is_removed_alone() {
    let (d, t, clones, _) = three();
    let n = d.own_steps().len();
    d.edit_text(clones[0], "", &TextChars::default()).unwrap();
    assert_eq!(d.own_steps().len(), n + 1);
    assert!(d.text(clones[0]).is_none(), "the emptied clone goes");
    assert_eq!(snap(&d, t).string, "Ring");
    assert_eq!(snap(&d, clones[1]).string, "Ring");
    assert_eq!(snap(&d, clones[1]).source.unwrap().node, t);
    undo_last(&d);
    assert_eq!(snap(&d, clones[0]).string, "Ring");
}

/// Criterion 41a: an emptied source is removed; its clones become texts with the string they
/// showed before the edit; count 2; Undo brings back the source and the links. Nothing else goes.
#[test]
fn c41a_emptied_source_is_removed_and_clones_keep_the_old_string() {
    let (d, t, clones, cs) = three();
    let other = text(&d, "Bystander");
    d.edit_text(t, "", &TextChars::default()).unwrap();
    assert!(d.text(t).is_none());
    for &c in &clones {
        let s = snap(&d, c);
        assert!(s.source.is_none());
        assert_eq!(s.string, "Ring", "the string shown before the edit");
    }
    assert_eq!(d.take_text_clones_unlinked(), 2);
    assert_eq!(
        snap(&d, other).string,
        "Bystander",
        "nothing else is removed"
    );
    for &c in &cs {
        assert!(d.object(c).is_some());
    }
    undo_last(&d);
    assert_eq!(snap(&d, t).string, "Ring");
    for &c in &clones {
        assert_eq!(snap(&d, c).source.unwrap().node, t);
    }
}

/// Criterion 41b: Unlink clone and Own style on a linked clone and a text clone together: one
/// step each.
#[test]
fn c41b_unlink_and_own_style_on_both_kinds_one_step_each() {
    let (d, _t, clones, _) = three();
    let path = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(9, 1), pt(0.0, 300.0)),
            NewAnchor::corner(AnchorId::new(9, 2), pt(50.0, 300.0)),
        ],
        false,
    );
    let linked = d.linked_clone(&[path], &[]).unwrap()[0];
    let n = d.own_steps().len();
    d.set_own_style_and_texts(&[linked], &[clones[0]], true)
        .unwrap();
    assert_eq!(d.own_steps().len(), n + 1, "own style: one step");
    assert!(snap(&d, clones[0]).source.unwrap().own_style);
    match d.object(linked) {
        Some(ObjectSnapshot::Clone(c)) => assert!(c.chain.own_style),
        other => panic!("linked clone expected, got {other:?}"),
    }
    let n = d.own_steps().len();
    let unlink = CopySource {
        id: linked,
        anchor_ids: vec![AnchorId::new(9, 101), AnchorId::new(9, 102)],
    };
    d.unlink_clones_and_texts(&[unlink], &[clones[0]]).unwrap();
    assert_eq!(d.own_steps().len(), n + 1, "unlink: one step");
    assert!(snap(&d, clones[0]).source.is_none());
    assert!(!matches!(d.object(linked), Some(ObjectSnapshot::Clone(_))));
}

/// Duplicate (criterion 31 with ADR 0022 §4): a clone duplicated alone is a clone of the same
/// source 5 mm further on; source and clone duplicated together give a copied source whose copied
/// clone follows the copy.
#[test]
fn duplicate_of_a_clone_and_of_source_with_clone() {
    let (d, t, clones, _) = three();
    let copy = d
        .duplicate_objects(&[src(clones[0])], Vec2::new(5.0, 5.0))
        .unwrap()[0];
    let s = snap(&d, copy);
    assert_eq!(
        s.source.map(|x| x.node),
        Some(t),
        "a clone of the same source"
    );
    assert!((s.on_path.unwrap().values.position.as_mm() - 83.5).abs() < 1e-6);

    let both = d
        .duplicate_objects(&[src(t), src(clones[1])], Vec2::new(5.0, 5.0))
        .unwrap();
    let new_source = *both
        .iter()
        .find(|&&id| snap(&d, id).source.is_none())
        .unwrap();
    let new_clone = *both
        .iter()
        .find(|&&id| snap(&d, id).source.is_some())
        .unwrap();
    assert_eq!(snap(&d, new_clone).source.unwrap().node, new_source);
    d.edit_text(new_source, "Copy", &TextChars::default())
        .unwrap();
    assert_eq!(snap(&d, new_clone).string, "Copy");
    assert_eq!(
        snap(&d, clones[1]).string,
        "Ring",
        "the old family is untouched"
    );
}

/// Two peers: one edits the string through a clone while the other moves that clone along its
/// carrier; then one deletes the source while the other moves a clone. Replicas converge.
#[test]
fn two_peers_converge_on_string_placement_and_removed_source() {
    let a = Document::new(1);
    let c1 = circle_at(&a, 100.0);
    let c2 = circle_at(&a, 250.0);
    let t = text(&a, "Old");
    let clone = a
        .put_text_on_paths(t, &[(on(c1), at(10.0)), (on(c2), at(10.0))], true)
        .unwrap()[0];
    let b = Document::new(2);
    b.import_updates(&a.export_updates(&b.version()).unwrap())
        .unwrap();
    let sync = |x: &Document, y: &Document| {
        let to_y = x.export_updates(&y.version()).unwrap();
        let to_x = y.export_updates(&x.version()).unwrap();
        y.import_updates(&to_y).unwrap();
        x.import_updates(&to_x).unwrap();
    };
    a.edit_text(clone, "New", &TextChars::default()).unwrap();
    b.set_text_path(&[(clone, at(40.0))], OnPathLabel::Move)
        .unwrap();
    sync(&a, &b);
    for d in [&a, &b] {
        assert_eq!(snap(d, t).string, "New");
        let s = snap(d, clone);
        assert_eq!(s.string, "New");
        assert!((s.on_path.unwrap().values.position.as_mm() - 40.0).abs() < 1e-9);
    }

    a.delete_objects(&[t]).unwrap();
    b.set_text_path(&[(clone, at(55.0))], OnPathLabel::Move)
        .unwrap();
    sync(&a, &b);
    for d in [&a, &b] {
        assert!(d.text(t).is_none());
        let s = snap(d, clone);
        assert!(s.source.is_none(), "unlinked by the removal");
        assert_eq!(s.string, "New");
        assert!((s.on_path.unwrap().values.position.as_mm() - 55.0).abs() < 1e-9);
    }
}

/// Two peers: one removes the source while the other puts the source on more paths; after the
/// merge the replicas agree and no orphan clone is drawn (ADR 0022 §4, ADR 0015 §5).
#[test]
fn two_peers_remove_source_against_new_clone_converge_without_orphans() {
    let a = Document::new(1);
    let c1 = circle_at(&a, 100.0);
    let c2 = circle_at(&a, 250.0);
    let t = text(&a, "Gone");
    a.put_text_on_paths(t, &[(on(c1), at(10.0))], true).unwrap();
    let b = Document::new(2);
    b.import_updates(&a.export_updates(&b.version()).unwrap())
        .unwrap();
    a.delete_objects(&[t]).unwrap();
    // b releases and puts the source on two paths again: a new clone of a source a removed.
    b.release_from_path(&[(t, STRAIGHT)]).unwrap();
    let late = b
        .put_text_on_paths(t, &[(on(c1), at(10.0)), (on(c2), at(10.0))], true)
        .unwrap();
    let to_b = a.export_updates(&b.version()).unwrap();
    let to_a = b.export_updates(&a.version()).unwrap();
    b.import_updates(&to_b).unwrap();
    a.import_updates(&to_a).unwrap();
    let _ = a.remove_orphan_clones();
    let _ = b.remove_orphan_clones();
    for d in [&a, &b] {
        for s in texts_in(d) {
            if let Some(source) = s.source {
                assert!(d.text(source.node).is_some(), "no drawn orphan: {:?}", s.id);
            }
        }
    }
    let ids = |d: &Document| {
        let mut v: Vec<String> = texts_in(d)
            .iter()
            .map(|s| format!("{:?}{}", s.id, s.string))
            .collect();
        v.sort();
        v
    };
    assert_eq!(ids(&a), ids(&b), "replicas converge");
    let _ = late;
}

/// ADR 0022 §4: a clone whose source names nothing, is not a text, or is itself a text clone is an
/// orphan: not drawn, removed by the repair after Open; nothing crashes.
#[test]
fn orphan_text_clones_are_repaired_on_open() {
    let (d, t, clones, cs) = three();
    let base = LoroDoc::new();
    base.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let bytes = base.export(loro::ExportMode::Snapshot).unwrap();
    let enc = |id: NodeId| {
        let tid = tree_id(id);
        format!("{}:{}", tid.peer, tid.counter)
    };
    for (name, target) in [
        ("names nothing", "424242:99".to_owned()),
        ("not a text", enc(cs[0])),
        ("a chain", enc(clones[1])),
    ] {
        let l = LoroDoc::new();
        l.import(&bytes).unwrap();
        l.set_peer_id(7).unwrap();
        let meta = l.get_tree("paths").get_meta(tree_id(clones[0])).unwrap();
        meta.insert("text_source", LoroValue::from(target)).unwrap();
        let opened = unpack(5, &file_from_loro(&l)).expect("opens, never refused");
        assert!(
            opened.text(clones[0]).is_none(),
            "{name}: the orphan is not drawn"
        );
        assert_eq!(
            opened.text(t).unwrap().string,
            "Ring",
            "{name}: source intact"
        );
        assert_eq!(
            opened.text(clones[1]).unwrap().source.unwrap().node,
            t,
            "{name}: the healthy clone stays"
        );
    }
}

/// Format 23: save and reopen keeps sources, clone placements and own style; the golden file
/// opens with text clones; a format 22 file opens with no clones.
#[test]
fn format_23_round_trip_golden_and_older_file() {
    const _: () = assert!(CURRENT_FORMAT_VERSION >= 23);
    let (d, t, clones, _) = three();
    d.set_text_path(&[(clones[0], at(12.25))], OnPathLabel::Move)
        .unwrap();
    d.set_own_style_and_texts(&[], &[clones[1]], true).unwrap();
    d.edit_style(&[clones[1]], &StyleEdit::FillColor(blue()))
        .unwrap();
    let again = unpack(2, &pack(&d, "tester").unwrap()).unwrap();
    for &c in &clones {
        let before = snap(&d, c);
        let after = snap(&again, c);
        assert_eq!(after.source, before.source);
        assert_eq!(
            after.on_path.as_ref().unwrap().values,
            before.on_path.as_ref().unwrap().values
        );
        assert_eq!(after.style, before.style);
        assert_eq!(after.string, "Ring");
    }
    assert_eq!(snap(&again, t).string, "Ring");

    let golden = unpack(3, &fixture("text_clones_v23.curvyo")).expect("golden opens");
    let gt = texts_in(&golden);
    let clones_in: Vec<&TextSnapshot> = gt.iter().filter(|s| s.source.is_some()).collect();
    assert!(!clones_in.is_empty(), "the golden file has text clones");
    for c in clones_in {
        let s = golden.text(c.source.unwrap().node).unwrap();
        assert_eq!(c.string, s.string);
    }

    let old = unpack(4, &fixture("text_on_path_v22.curvyo")).expect("v22 opens");
    assert!(texts_in(&old).iter().all(|s| s.source.is_none()));
}

/// White box: putting a text clone on several paths makes clones of its source (no chains, ADR
/// 0022 §4).
#[test]
fn wb_put_a_clone_on_paths_makes_clones_of_the_source() {
    let (d, t, clones, _) = three();
    let more = [circle_at(&d, 550.0), circle_at(&d, 700.0)];
    let made = d
        .put_text_on_paths(
            clones[0],
            &[(on(more[0]), at(5.0)), (on(more[1]), at(5.0))],
            true,
        )
        .unwrap();
    assert_eq!(made.len(), 1);
    assert_eq!(snap(&d, made[0]).source.unwrap().node, t);
    assert_eq!(snap(&d, clones[0]).source.unwrap().node, t);
    assert_eq!(snap(&d, made[0]).string, "Ring", "drawn, not an orphan");
}

/// White box: removing the source together with one of its clones unlinks only the other clone
/// (count 1); Redo after Undo of Put text on paths gives the clones back.
#[test]
fn wb_remove_source_with_one_clone_and_redo_of_put() {
    let (d, t, clones, _) = three();
    d.delete_objects(&[t, clones[0]]).unwrap();
    assert!(d.text(clones[0]).is_none());
    assert!(snap(&d, clones[1]).source.is_none());
    assert_eq!(d.take_text_clones_unlinked(), 1);

    let (d, t, clones, _) = three();
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
    assert!(d.text(clones[0]).is_none());
    d.redo_step(step).unwrap();
    assert_eq!(
        snap(&d, clones[0]).string,
        "Ring",
        "Redo puts the clones back"
    );
    assert_eq!(snap(&d, clones[0]).source.unwrap().node, t);
}
