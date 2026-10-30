//! One text on several paths (`specs/0080-text-on-path` slice 2, criteria 37 to 42; ADR 0022 §4):
//! the source and its text clones in the document and the file. A text clone reads the source's
//! string, kerns, face, size, layout and (without own style) style; its carrier placement is its
//! own.

#![allow(clippy::unwrap_used)]

#[path = "support/pre_pages_ids.rs"]
mod pre_pages_ids;

use std::path::PathBuf;

use curvyo_document_core::{
    Affine, Angle, CURRENT_FORMAT_VERSION, CarrierRef, Color, CopySource, Curve, Document,
    EllipseFrame, FontKind, FontRef, Length, NewAnchor, NodeId, ObjectSnapshot, OnPathLabel,
    OnPathValues, PathSide, Point, Shape, StyleEdit, TextAlign, TextChars, TextConversion,
    TextLayoutLabel, TextLayoutValues, TextSnapshot, Vec2, pack, unpack,
};

const GOLDEN: &str = "text_clones_v23.curvyo";

fn sans(style: &str) -> FontRef {
    FontRef {
        family: "Source Sans 3".to_owned(),
        style: style.to_owned(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind: FontKind::Outline,
    }
}

fn text(d: &Document, string: &str) -> NodeId {
    d.create_text(
        string,
        &sans("Regular"),
        Length::from_mm(10.0),
        Affine::translation(Vec2::new(5.0, 7.0)),
        &TextLayoutValues::DEFAULT,
    )
    .unwrap()
}

fn circle(d: &Document, x: f64) -> NodeId {
    d.create_primitive(
        Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(x, 100.0),
                rx: Length::from_mm(40.0),
                ry: Length::from_mm(40.0),
            },
            arc: None,
            curve: Curve::ELLIPSE_DEFAULT,
        },
        Angle::from_radians(0.0),
    )
}

fn on(carrier: NodeId) -> CarrierRef {
    CarrierRef {
        node: carrier,
        outline: 0,
    }
}

fn at(mm: f64) -> OnPathValues {
    OnPathValues::at(Length::from_mm(mm))
}

fn read_text(d: &Document, id: NodeId) -> TextSnapshot {
    match d.read_objects().get(id) {
        Some(ObjectSnapshot::Text(text)) => (**text).clone(),
        other => panic!("not a text: {other:?}"),
    }
}

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

fn undo_last(d: &Document) {
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
}

fn is_guide(d: &Document, id: NodeId) -> bool {
    d.read_nodes().iter().find(|n| n.id == id).unwrap().guide
}

/// A text on three circles: the text and its two clones.
fn three(d: &Document) -> (NodeId, [NodeId; 3], Vec<NodeId>) {
    let t = text(d, "Hello");
    let circles = [circle(d, 100.0), circle(d, 200.0), circle(d, 300.0)];
    let clones = d
        .put_text_on_paths(
            t,
            &[
                (on(circles[0]), at(10.0)),
                (on(circles[1]), at(20.0)),
                (on(circles[2]), at(30.0)),
            ],
            true,
        )
        .unwrap();
    (t, circles, clones)
}

/// Criterion 37: the text goes on the first carrier, a text clone on each further one with its
/// own values; every carrier is a guide; one step "Put text on paths".
#[test]
fn c37_put_on_three_paths_gives_the_text_and_two_clones_in_one_step() {
    let d = Document::new(1);
    let (t, circles, clones) = three(&d);
    assert_eq!(clones.len(), 2);
    assert_eq!(last_label(&d), "put_text_on_paths");
    assert_eq!(d.own_steps().len(), 5, "three circles, the text, one put");
    let source = read_text(&d, t);
    assert_eq!(source.source, None);
    assert_eq!(source.on_path.as_ref().unwrap().carrier, on(circles[0]));
    for (clone, (circle, position)) in clones.iter().zip([(circles[1], 20.0), (circles[2], 30.0)]) {
        let read = read_text(&d, *clone);
        assert_eq!(read.source.unwrap().node, t);
        assert!(!read.source.unwrap().own_style);
        assert_eq!(read.string, "Hello");
        assert_eq!(read.font, source.font);
        assert_eq!(read.size, source.size);
        assert_eq!(read.style, source.style);
        let on_path = read.on_path.unwrap();
        assert_eq!(on_path.carrier, on(circle));
        assert_eq!(on_path.values.position, Length::from_mm(position));
        assert!(on_path.curve.is_some());
    }
    assert!(circles.iter().all(|c| is_guide(&d, *c)));
    assert_eq!(d.read_objects().clones_of[&t], clones);
    undo_last(&d);
    assert!(d.read_objects().get(clones[0]).is_none());
    assert!(read_text(&d, t).on_path.is_none());
    assert!(circles.iter().all(|c| !is_guide(&d, *c)));
}

/// Criterion 37: only paths and shapes are carriers; another text or a group among the carriers
/// is skipped, as by `put_text_on_path`'s rules, and gets no clone.
#[test]
fn c37_texts_and_groups_among_the_carriers_are_skipped() {
    let d = Document::new(1);
    let t = text(&d, "Hello");
    let other = text(&d, "Other");
    let grouped = [circle(&d, 300.0), circle(&d, 400.0)];
    let group = d.group_objects(&grouped).unwrap();
    let c = circle(&d, 100.0);
    let clones = d
        .put_text_on_paths(
            t,
            &[
                (on(other), at(10.0)),
                (on(group), at(10.0)),
                (on(c), at(20.0)),
            ],
            true,
        )
        .unwrap();
    assert_eq!(clones, Vec::<NodeId>::new());
    assert_eq!(last_label(&d), "put_text_on_path");
    assert_eq!(read_text(&d, t).on_path.unwrap().carrier, on(c));
    assert!(read_text(&d, other).on_path.is_none());
    assert!(!is_guide(&d, other));
    assert!(is_guide(&d, c));
    assert!(d.read_objects().clones_of.get(&t).is_none_or(Vec::is_empty));
}

/// Criterion 38: string, kerns, face, size and layout edited from a clone change the source and
/// every clone, one step each.
#[test]
fn c38_content_edited_from_a_clone_changes_every_member() {
    let d = Document::new(1);
    let (t, _, clones) = three(&d);
    d.edit_text(clones[0], "Hello world", &TextChars::default())
        .unwrap();
    assert_eq!(last_label(&d), "edit_text");
    d.set_text_size(&[clones[1]], Length::from_mm(14.0))
        .unwrap();
    d.set_text_fonts(&[(clones[0], sans("Bold"))]).unwrap();
    let mut layout = TextLayoutValues::DEFAULT;
    layout.align = TextAlign::Centre;
    d.set_text_layout(
        &[(clones[1], layout, Affine::IDENTITY)],
        TextLayoutLabel::Alignment,
    )
    .unwrap();
    for id in [t, clones[0], clones[1]] {
        let read = read_text(&d, id);
        assert_eq!(read.string, "Hello world");
        assert_eq!(read.size, Length::from_mm(14.0));
        assert_eq!(read.font.style, "Bold");
        assert_eq!(read.layout.align, TextAlign::Centre);
    }
    // The clone's own straight placement took the placement of the layout change, the source's
    // stayed.
    assert_eq!(
        read_text(&d, t).placement,
        Affine::translation(Vec2::new(5.0, 7.0))
    );
}

/// Criterion 39: Side, Position and Release act on the clone alone; a released clone is a
/// straight text that still follows the source.
#[test]
fn c39_placement_is_each_clones_own_and_release_keeps_it_a_clone() {
    let d = Document::new(1);
    let (t, _, clones) = three(&d);
    let mut values = at(50.0);
    values.side = PathSide::Below;
    d.set_text_path(&[(clones[0], values)], OnPathLabel::Side)
        .unwrap();
    assert_eq!(read_text(&d, clones[0]).on_path.unwrap().values, values);
    assert_eq!(
        read_text(&d, t).on_path.unwrap().values.position,
        Length::from_mm(10.0)
    );
    assert_eq!(
        read_text(&d, clones[1]).on_path.unwrap().values.position,
        Length::from_mm(30.0)
    );
    let straight = Affine::translation(Vec2::new(40.0, 40.0));
    d.release_from_path(&[(clones[0], straight)]).unwrap();
    let released = read_text(&d, clones[0]);
    assert!(released.on_path.is_none());
    assert_eq!(released.placement, straight);
    assert_eq!(released.source.unwrap().node, t);
    d.edit_text(t, "Hi", &TextChars::default()).unwrap();
    assert_eq!(read_text(&d, clones[0]).string, "Hi");
}

/// Criterion 40: the original's style by default, own style for one clone; a style edit on a
/// clone without own style is left alone.
#[test]
fn c40_own_style_gives_one_clone_its_own_colour() {
    let d = Document::new(1);
    let (t, _, clones) = three(&d);
    let red = Color { r: 200, g: 0, b: 0 };
    let blue = Color { r: 0, g: 0, b: 200 };
    d.edit_style(&[clones[0]], &StyleEdit::FillColor(red))
        .unwrap();
    assert_ne!(read_text(&d, clones[0]).style.fill.color, red, "skipped");
    assert_eq!(
        d.set_own_style_and_texts(&[], &[clones[0]], true).unwrap(),
        1
    );
    assert_eq!(last_label(&d), "own_style");
    assert!(read_text(&d, clones[0]).source.unwrap().own_style);
    d.edit_style(&[clones[0]], &StyleEdit::FillColor(red))
        .unwrap();
    d.edit_style(&[t], &StyleEdit::FillColor(blue)).unwrap();
    assert_eq!(read_text(&d, clones[0]).style.fill.color, red);
    assert_eq!(read_text(&d, clones[1]).style.fill.color, blue);
    assert_eq!(
        d.set_own_style_and_texts(&[], &[clones[0]], false).unwrap(),
        1
    );
    assert_eq!(last_label(&d), "original_style");
    assert_eq!(read_text(&d, clones[0]).style.fill.color, blue);
}

/// Criterion 41: Unlink clone makes an independent text with the same values and placement.
#[test]
fn c41_unlink_clone_gives_an_independent_text_on_its_carrier() {
    let d = Document::new(1);
    let (t, circles, clones) = three(&d);
    d.set_own_style_and_texts(&[], &[clones[0]], true).unwrap();
    d.edit_style(
        &[clones[0]],
        &StyleEdit::FillColor(Color { r: 9, g: 9, b: 9 }),
    )
    .unwrap();
    let before = read_text(&d, clones[0]);
    assert_eq!(d.unlink_clones_and_texts(&[], &[clones[0]]).unwrap(), 1);
    assert_eq!(last_label(&d), "unlink_clone");
    let after = read_text(&d, clones[0]);
    assert_eq!(after.source, None);
    assert_eq!(after.string, before.string);
    assert_eq!(after.style, before.style);
    assert_eq!(after.on_path.as_ref().unwrap().carrier, on(circles[1]));
    assert_eq!(
        after.on_path.unwrap().values,
        before.on_path.unwrap().values
    );
    d.edit_text(t, "Other", &TextChars::default()).unwrap();
    assert_eq!(read_text(&d, clones[0]).string, "Hello");
    assert_eq!(read_text(&d, clones[1]).string, "Other");
    assert_eq!(d.read_objects().clones_of[&t], vec![clones[1]]);
}

/// Criterion 41: removing the source unlinks every clone in the same step, counted for the
/// notice; Undo gives the source and the links back.
#[test]
fn c41_removing_the_source_turns_its_clones_into_texts() {
    let d = Document::new(1);
    let (t, _, clones) = three(&d);
    d.set_text_size(&[t], Length::from_mm(12.0)).unwrap();
    d.delete_objects(&[t]).unwrap();
    assert_eq!(d.take_text_clones_unlinked(), 2);
    for clone in &clones {
        let read = read_text(&d, *clone);
        assert_eq!(read.source, None);
        assert_eq!(read.string, "Hello");
        assert_eq!(read.size, Length::from_mm(12.0));
        assert!(read.on_path.unwrap().curve.is_some());
    }
    undo_last(&d);
    assert_eq!(read_text(&d, clones[0]).source.unwrap().node, t);
    d.edit_text(t, "Back", &TextChars::default()).unwrap();
    assert_eq!(read_text(&d, clones[1]).string, "Back");
}

/// Lead decision 1: emptying a clone's string removes that clone alone; emptying the source's
/// removes the source alone and its clones become texts with the string they showed, one step.
#[test]
fn an_emptied_string_removes_the_edited_text_only() {
    let d = Document::new(1);
    let (t, _, clones) = three(&d);
    d.edit_text(clones[1], "", &TextChars::default()).unwrap();
    let read = d.read_objects();
    assert!(read.get(clones[1]).is_none());
    assert_eq!(read_text(&d, t).string, "Hello");
    assert_eq!(read_text(&d, clones[0]).source.unwrap().node, t);
    d.edit_text(t, "", &TextChars::default()).unwrap();
    assert_eq!(last_label(&d), "edit_text");
    assert!(d.read_objects().get(t).is_none());
    assert_eq!(d.take_text_clones_unlinked(), 1);
    let kept = read_text(&d, clones[0]);
    assert_eq!((kept.source, kept.string.as_str()), (None, "Hello"));
    undo_last(&d);
    assert_eq!(read_text(&d, clones[0]).source.unwrap().node, t);
    assert_eq!(read_text(&d, t).string, "Hello");
}

/// Criterion 42: Object to path of a clone converts that clone only.
#[test]
fn c42_object_to_path_of_a_clone_leaves_the_others_texts() {
    let d = Document::new(1);
    let (t, _, clones) = three(&d);
    let glyph = vec![(
        vec![
            NewAnchor::corner(
                curvyo_document_core::AnchorId::new(1, 900),
                Point::new(0.0, 0.0),
            ),
            NewAnchor::corner(
                curvyo_document_core::AnchorId::new(1, 901),
                Point::new(5.0, 0.0),
            ),
            NewAnchor::corner(
                curvyo_document_core::AnchorId::new(1, 902),
                Point::new(0.0, 5.0),
            ),
        ],
        true,
    )];
    let groups = d
        .convert_to_paths_with_texts(
            &[],
            &[TextConversion {
                id: clones[0],
                name: "Hello".to_owned(),
                glyphs: vec![glyph],
            }],
        )
        .unwrap();
    assert!(groups[0].is_some());
    assert_eq!(d.take_text_clones_unlinked(), 0);
    assert!(d.read_objects().get(clones[0]).is_none());
    assert_eq!(read_text(&d, clones[1]).source.unwrap().node, t);
    assert_eq!(read_text(&d, t).string, "Hello");
}

/// Duplicate of a clone alone gives another clone of the source on the same carrier, 5 mm on;
/// of the source with a clone, the copied clone follows the copied source.
#[test]
fn duplicate_keeps_or_remaps_the_source() {
    let d = Document::new(1);
    let (t, _, clones) = three(&d);
    let copy = d
        .duplicate_objects(
            &[CopySource {
                id: clones[0],
                anchor_ids: vec![],
            }],
            Vec2::new(0.0, 0.0),
        )
        .unwrap()[0];
    let read = read_text(&d, copy);
    assert_eq!(read.source.unwrap().node, t);
    assert_eq!(read.on_path.unwrap().values.position, Length::from_mm(25.0));
    let copies = d
        .duplicate_objects(
            &[
                CopySource {
                    id: t,
                    anchor_ids: vec![],
                },
                CopySource {
                    id: clones[1],
                    anchor_ids: vec![],
                },
            ],
            Vec2::new(0.0, 0.0),
        )
        .unwrap();
    assert_eq!(read_text(&d, copies[1]).source.unwrap().node, copies[0]);
    d.edit_text(copies[0], "Copy", &TextChars::default())
        .unwrap();
    assert_eq!(read_text(&d, copies[1]).string, "Copy");
    assert_eq!(read_text(&d, clones[1]).string, "Hello");
}

fn merge(a: &Document, b: &Document) {
    a.import_updates(&b.export_updates(&a.version()).unwrap())
        .unwrap();
    b.import_updates(&a.export_updates(&b.version()).unwrap())
        .unwrap();
}

/// Two peers: one edits the source string, the other moves a clone along its carrier; both
/// survive and the replicas converge.
#[test]
fn a_string_edit_and_a_clone_move_by_two_peers_merge() {
    let a = Document::new(1);
    let (t, _, clones) = three(&a);
    let b = unpack(2, &pack(&a, "0.1.0").unwrap()).unwrap();
    a.edit_text(t, "Hello there", &TextChars::default())
        .unwrap();
    b.set_text_path(&[(clones[0], at(70.0))], OnPathLabel::Move)
        .unwrap();
    merge(&a, &b);
    for d in [&a, &b] {
        let read = read_text(d, clones[0]);
        assert_eq!(read.string, "Hello there");
        assert_eq!(read.on_path.unwrap().values.position, Length::from_mm(70.0));
    }
    assert_eq!(a.read_objects(), b.read_objects());
}

/// A clone whose source a peer removed while the clone was made is an orphan: not read; the
/// repair after the merge removes it.
#[test]
fn a_clone_of_a_concurrently_removed_source_is_an_orphan() {
    let a = Document::new(1);
    let t = text(&a, "Hello");
    let c1 = circle(&a, 100.0);
    let c2 = circle(&a, 200.0);
    let b = unpack(2, &pack(&a, "0.1.0").unwrap()).unwrap();
    let clones = a
        .put_text_on_paths(t, &[(on(c1), at(0.0)), (on(c2), at(0.0))], true)
        .unwrap();
    b.delete_objects(&[t]).unwrap();
    merge(&a, &b);
    let read = a.read_objects();
    assert!(read.get(clones[0]).is_none());
    assert_eq!(read.orphans, vec![clones[0]]);
    assert_eq!(a.remove_orphan_clones(), 1);
    assert_eq!(a.read_objects().orphans, Vec::<NodeId>::new());
}

/// Criterion 44, the document part: 20 clones of a 100-character source read in one pass.
#[test]
fn twenty_clones_read_in_one_pass() {
    let d = Document::new(1);
    let t = text(&d, &"x".repeat(100));
    let carriers: Vec<(CarrierRef, OnPathValues)> = (0..21)
        .map(|i| (on(circle(&d, f64::from(i) * 100.0)), at(0.0)))
        .collect();
    let clones = d.put_text_on_paths(t, &carriers, true).unwrap();
    assert_eq!(clones.len(), 20);
    d.edit_text(t, &"y".repeat(100), &TextChars::default())
        .unwrap();
    let read = d.read_objects();
    assert!(clones.iter().all(|c| matches!(
        read.get(*c),
        Some(ObjectSnapshot::Text(text)) if text.string == "y".repeat(100)
    )));
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// The golden file of format 23: a source on a circle, a clone with the original's style, a
/// clone with its own style and Side Below, a released clone.
#[test]
fn golden_file_of_format_23() {
    const _: () = assert!(CURRENT_FORMAT_VERSION >= 23);
    let d = Document::new(1);
    // No two steps in a row with the same label (the history rows of old files, `0020`).
    let t = text(&d, "Hello");
    let c1 = circle(&d, 100.0);
    let c2 = d.create_path(
        &[
            NewAnchor::corner(
                curvyo_document_core::AnchorId::new(1, 800),
                Point::new(0.0, 200.0),
            ),
            NewAnchor::corner(
                curvyo_document_core::AnchorId::new(1, 801),
                Point::new(90.0, 200.0),
            ),
        ],
        false,
    );
    let c3 = circle(&d, 300.0);
    let clones = d
        .put_text_on_paths(
            t,
            &[(on(c1), at(10.0)), (on(c2), at(0.0)), (on(c3), at(30.0))],
            true,
        )
        .unwrap();
    d.set_own_style_and_texts(&[], &[clones[0]], true).unwrap();
    d.edit_style(
        &[clones[0]],
        &StyleEdit::FillColor(Color { r: 200, g: 0, b: 0 }),
    )
    .unwrap();
    let mut below = at(40.0);
    below.side = PathSide::Below;
    d.set_text_path(&[(clones[0], below)], OnPathLabel::Side)
        .unwrap();
    let released = text(&d, "Free");
    let c = circle(&d, 400.0);
    let more = d
        .put_text_on_paths(released, &[(on(c), at(0.0)), (on(c), at(60.0))], true)
        .unwrap();
    d.release_from_path(&[(more[0], Affine::translation(Vec2::new(1.0, 2.0)))])
        .unwrap();
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(fixture_path(GOLDEN), pack(&d, "0.1.0").unwrap()).unwrap();
    }
    let stored = std::fs::read(fixture_path(GOLDEN)).unwrap();
    let opened = unpack(3, &stored).unwrap();
    assert_eq!(
        serde_json::to_value(opened.read_objects().objects).unwrap(),
        pre_pages_ids::value(&d.read_objects().objects)
    );
    let again = unpack(4, &pack(&opened, "0.1.0").unwrap()).unwrap();
    assert_eq!(again.read_objects().objects, opened.read_objects().objects);
}
