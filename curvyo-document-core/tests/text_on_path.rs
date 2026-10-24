//! A text on path in the document and the file (`specs/0080-text-on-path` slice 1; ADR 0022): the
//! put with the carrier made a guide, the values and their steps, Release, the outline per
//! carrier kind, the kept copy of a removed carrier, Duplicate, a clone carrier, the golden file
//! of format 22 and the read rules of criterion 35.

#![allow(clippy::unwrap_used)]

use std::collections::HashMap;
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, AnchorId, Angle, ArcKind, CURRENT_FORMAT_VERSION, CarrierRef, CopySource, Curve,
    Document, EllipseArc, EllipseFrame, FontKind, FontRef, Length, LineOnPath, NewAnchor, NodeId,
    ObjectSnapshot, OnPathLabel, OnPathValues, PathSide, Point, PointCount, RectBounds, Shape,
    StarFrame, TextLayoutValues, TextOnPath, TextSnapshot, Vec2, pack, unpack,
};

const GOLDEN: &str = "text_on_path_v22.curvyo";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn sans() -> FontRef {
    FontRef {
        family: "Source Sans 3".to_owned(),
        style: "Regular".to_owned(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind: FontKind::Outline,
    }
}

fn text(d: &Document, string: &str) -> NodeId {
    d.create_text(
        string,
        &sans(),
        Length::from_mm(10.0),
        Affine::translation(Vec2::new(5.0, 7.0)),
        &TextLayoutValues::DEFAULT,
    )
    .unwrap()
}

fn line(d: &Document, n: u64, from: (f64, f64), to: (f64, f64)) -> NodeId {
    d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(9, n), Point::new(from.0, from.1)),
            NewAnchor::corner(AnchorId::new(9, n + 1), Point::new(to.0, to.1)),
        ],
        false,
    )
}

fn circle(d: &Document) -> NodeId {
    d.create_primitive(
        Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(100.0, 100.0),
                rx: Length::from_mm(50.0),
                ry: Length::from_mm(50.0),
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

fn read_text(d: &Document, id: NodeId) -> TextSnapshot {
    let read = d.read_objects();
    match read.get(id) {
        Some(ObjectSnapshot::Text(text)) => (**text).clone(),
        other => panic!("not a text: {other:?}"),
    }
}

fn on_path(d: &Document, id: NodeId) -> TextOnPath {
    read_text(d, id).on_path.unwrap()
}

fn is_guide(d: &Document, id: NodeId) -> bool {
    d.read_nodes().iter().find(|n| n.id == id).unwrap().guide
}

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

fn undo_last(d: &Document) {
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
}

/// Criteria 4 to 6: one step "Put text on path", the carrier a guide in it, nothing else of the
/// carrier changed; Undo gives the straight text and the carrier's old role.
#[test]
fn c04_c06_put_marks_the_carrier_and_undoes_in_one_step() {
    let d = Document::new(1);
    let t = text(&d, "Hello");
    let c = circle(&d);
    let shape_before = d.read_objects().get(c).cloned();
    d.put_text_on_path(t, on(c), &OnPathValues::at(Length::from_mm(30.0)), true)
        .unwrap();
    assert_eq!(last_label(&d), "put_text_on_path");
    assert!(is_guide(&d, c));
    let read = on_path(&d, t);
    assert_eq!(read.carrier, on(c));
    assert_eq!(read.values.position, Length::from_mm(30.0));
    assert!(read.curve.is_some() && !read.kept);
    assert_eq!(
        read_text(&d, t).placement,
        Affine::translation(Vec2::new(5.0, 7.0))
    );
    let ObjectSnapshot::Primitive(after) = d.read_objects().get(c).cloned().unwrap() else {
        panic!("the circle stays a primitive")
    };
    let Some(ObjectSnapshot::Primitive(before)) = shape_before else {
        panic!()
    };
    assert_eq!(after.shape, before.shape);
    assert_eq!(d.read_objects().texts_on.get(&c), Some(&vec![t]));
    undo_last(&d);
    assert!(read_text(&d, t).on_path.is_none());
    assert!(!is_guide(&d, c));
}

/// Criteria 4 and 7: a carrier on a locked layer is not made a guide; the text goes on anyway.
#[test]
fn c07_a_locked_carrier_is_not_marked() {
    let d = Document::new(1);
    let t = text(&d, "Hi");
    let c = line(&d, 1, (0.0, 0.0), (100.0, 0.0));
    d.set_node_locked(&[c], true).unwrap();
    d.put_text_on_path(t, on(c), &OnPathValues::at(Length::from_mm(0.0)), true)
        .unwrap();
    assert!(!is_guide(&d, c));
    assert!(on_path(&d, t).curve.is_some());
}

/// Criteria 17, 18, 23: the values with their labels; Release deletes every key, writes the
/// straight placement, and leaves the carrier a guide (4a).
#[test]
fn c17_c23_values_and_release() {
    let d = Document::new(1);
    let t = text(&d, "Hi");
    let c = line(&d, 1, (0.0, 0.0), (100.0, 0.0));
    d.put_text_on_path(t, on(c), &OnPathValues::at(Length::from_mm(0.0)), true)
        .unwrap();
    let mirrored = OnPathValues::at(Length::from_mm(0.0)).mirrored();
    d.set_text_path(&[(t, mirrored)], OnPathLabel::Mirror)
        .unwrap();
    assert_eq!(last_label(&d), "mirror_text_on_path");
    let read = on_path(&d, t).values;
    assert_eq!(
        (read.side, read.line),
        (PathSide::Below, LineOnPath::TopEdge)
    );
    let steps = d.own_steps().len();
    d.set_text_path(&[(t, mirrored)], OnPathLabel::Mirror)
        .unwrap();
    assert_eq!(d.own_steps().len(), steps, "unchanged writes no step");
    let straight = Affine::translation(Vec2::new(1.0, 2.0));
    d.release_from_path(&[(t, straight)]).unwrap();
    assert_eq!(last_label(&d), "release_from_path");
    let released = read_text(&d, t);
    assert!(released.on_path.is_none());
    assert_eq!(released.placement, straight);
    assert!(is_guide(&d, c), "the carrier stays a guide");
}

/// Spec "Terms", carrier start and direction: per kind, the first node of the outline the text
/// sits on, clockwise on screen.
#[test]
fn the_start_of_each_carrier_kind_is_pinned() {
    let d = Document::new(1);
    let rect = d.create_primitive(
        Shape::Rect {
            bounds: RectBounds::from_corners(Point::new(0.0, 0.0), Point::new(40.0, 20.0)),
            corner_radii: curvyo_document_core::CornerRadii::uniform(Length::from_mm(5.0)),
        },
        Angle::from_radians(0.0),
    );
    let ellipse = circle(&d);
    let star = d.create_primitive(
        Shape::Star {
            frame: StarFrame::from_center_and_vertex(Point::new(0.0, 0.0), Point::new(0.0, -10.0)),
            point_count: PointCount::new(5).unwrap(),
            inner_ratio: curvyo_document_core::InnerRatio::new(0.5).unwrap(),
            curve: Curve::POLYGON_DEFAULT,
        },
        Angle::from_radians(0.0),
    );
    let path = line(&d, 1, (3.0, 4.0), (50.0, 4.0));
    let expected = [
        (rect, Point::new(5.0, 0.0), true),
        (ellipse, Point::new(150.0, 100.0), true),
        (star, Point::new(0.0, -10.0), true),
        (path, Point::new(3.0, 4.0), false),
    ];
    for (carrier, start, closed) in expected {
        let t = text(&d, "A");
        d.put_text_on_path(
            t,
            on(carrier),
            &OnPathValues::at(Length::from_mm(0.0)),
            false,
        )
        .unwrap();
        let curve = on_path(&d, t).curve.unwrap();
        assert_eq!(curve.closed, closed);
        assert!(
            curve.anchors[0].point.vector_to(start).length() < 1e-9,
            "{:?} != {start:?}",
            curve.anchors[0].point
        );
        assert_eq!(curve.ellipse.is_some(), carrier == ellipse);
    }
    // Clockwise on screen: the rectangle's second node is further right on its top edge.
    let t = text(&d, "A");
    d.put_text_on_path(t, on(rect), &OnPathValues::at(Length::from_mm(0.0)), false)
        .unwrap();
    let curve = on_path(&d, t).curve.unwrap();
    assert!(curve.anchors[1].point.x > curve.anchors[0].point.x);
}

/// Criterion 21 of `0021`, as a carrier: an open arc gives an open outline.
#[test]
fn an_open_arc_is_an_open_carrier() {
    let d = Document::new(1);
    let arc = d.create_primitive(
        Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(0.0, 0.0),
                rx: Length::from_mm(20.0),
                ry: Length::from_mm(10.0),
            },
            arc: EllipseArc::new(
                Angle::from_radians(0.0),
                Angle::from_radians(std::f64::consts::PI),
                ArcKind::Arc,
            ),
            curve: Curve::ELLIPSE_DEFAULT,
        },
        Angle::from_radians(0.0),
    );
    let t = text(&d, "A");
    d.put_text_on_path(t, on(arc), &OnPathValues::at(Length::from_mm(0.0)), true)
        .unwrap();
    assert!(!on_path(&d, t).curve.unwrap().closed);
}

/// Criteria 24 and 25: a carrier edit is followed without writing the text; a removed carrier
/// leaves its outline as the text's kept copy in the same step; Undo links it again.
#[test]
fn c24_c25_follow_and_keep_the_deleted_carrier() {
    let d = Document::new(1);
    let t = text(&d, "Hi");
    let c = line(&d, 1, (0.0, 0.0), (100.0, 0.0));
    d.put_text_on_path(t, on(c), &OnPathValues::at(Length::from_mm(0.0)), true)
        .unwrap();
    d.move_anchors(&[(c, AnchorId::new(9, 2), Point::new(100.0, 50.0))])
        .unwrap();
    let moved = on_path(&d, t).curve.unwrap();
    assert_eq!(moved.anchors[1].point, Point::new(100.0, 50.0));
    d.delete_objects(&[c]).unwrap();
    assert_eq!(d.take_kept_carriers(), 1);
    let kept = on_path(&d, t);
    assert!(kept.kept);
    assert_eq!(
        kept.curve.unwrap().anchors[1].point,
        Point::new(100.0, 50.0)
    );
    assert!(!d.read_objects().texts_on.contains_key(&c));
    undo_last(&d);
    let back = on_path(&d, t);
    assert!(!back.kept, "the live carrier wins");
    assert!(d.read_objects().texts_on.contains_key(&c));
}

/// Criterion 25: a command that replaces the carrier by a new node (Combine) keeps the copy.
#[test]
fn c25_a_replaced_carrier_is_kept_too() {
    let d = Document::new(1);
    let t = text(&d, "Hi");
    let c = line(&d, 1, (0.0, 0.0), (100.0, 0.0));
    d.put_text_on_path(t, on(c), &OnPathValues::at(Length::from_mm(0.0)), true)
        .unwrap();
    let other = line(&d, 5, (0.0, 50.0), (100.0, 50.0));
    let outlines = vec![
        (
            vec![
                NewAnchor::corner(AnchorId::new(8, 1), Point::new(0.0, 0.0)),
                NewAnchor::corner(AnchorId::new(8, 2), Point::new(100.0, 0.0)),
            ],
            false,
        ),
        (
            vec![
                NewAnchor::corner(AnchorId::new(8, 3), Point::new(0.0, 50.0)),
                NewAnchor::corner(AnchorId::new(8, 4), Point::new(100.0, 50.0)),
            ],
            false,
        ),
    ];
    let combined = d
        .replace_with_path(&[c, other], c, &outlines, "combine_paths")
        .unwrap();
    assert_ne!(combined, c);
    let kept = on_path(&d, t);
    assert!(kept.kept && kept.curve.is_some());
}

/// Criterion 2 Terms: a compound path's outline by index; past the last reads the last.
#[test]
fn a_compound_outline_by_index() {
    let d = Document::new(1);
    let a = line(&d, 1, (0.0, 0.0), (10.0, 0.0));
    let outlines = vec![
        (
            vec![
                NewAnchor::corner(AnchorId::new(8, 1), Point::new(0.0, 0.0)),
                NewAnchor::corner(AnchorId::new(8, 2), Point::new(10.0, 0.0)),
            ],
            false,
        ),
        (
            vec![
                NewAnchor::corner(AnchorId::new(8, 3), Point::new(0.0, 30.0)),
                NewAnchor::corner(AnchorId::new(8, 4), Point::new(10.0, 30.0)),
            ],
            false,
        ),
    ];
    let compound = d
        .replace_with_path(&[a], a, &outlines, "combine_paths")
        .unwrap();
    for (outline, y) in [(1, 30.0), (7, 30.0), (0, 0.0)] {
        let t = text(&d, "A");
        d.put_text_on_path(
            t,
            CarrierRef {
                node: compound,
                outline,
            },
            &OnPathValues::at(Length::from_mm(0.0)),
            false,
        )
        .unwrap();
        assert_eq!(on_path(&d, t).curve.unwrap().anchors[0].point.y, y);
    }
}

/// Criterion 31: Duplicate of the text alone shifts it 5 mm along the same carrier; of the text
/// with its carrier, the copy sits on the carrier's copy.
#[test]
fn c31_duplicate() {
    let d = Document::new(1);
    let t = text(&d, "Hi");
    let c = line(&d, 1, (0.0, 0.0), (100.0, 0.0));
    d.put_text_on_path(t, on(c), &OnPathValues::at(Length::from_mm(10.0)), true)
        .unwrap();
    let copies = d
        .duplicate_objects(
            &[CopySource {
                id: t,
                anchor_ids: Vec::new(),
            }],
            Vec2::new(3.0, 3.0),
        )
        .unwrap();
    let copy = on_path(&d, copies[0]);
    assert_eq!(copy.carrier, on(c));
    assert_eq!(copy.values.position, Length::from_mm(15.0));
    let pair = d
        .duplicate_objects_shifting(
            &[
                CopySource {
                    id: t,
                    anchor_ids: Vec::new(),
                },
                CopySource {
                    id: c,
                    anchor_ids: vec![AnchorId::new(7, 1), AnchorId::new(7, 2)],
                },
            ],
            Vec2::new(0.0, 20.0),
            &HashMap::new(),
        )
        .unwrap();
    let text_copy = on_path(&d, pair[0]);
    assert_eq!(text_copy.carrier.node, pair[1]);
    assert_eq!(text_copy.values.position, Length::from_mm(10.0));
}

/// ADR 0022 §2: a linked clone as carrier is marked alone; its original stays artwork, and Guide
/// off turns the clone back.
#[test]
fn a_clone_carrier_is_marked_alone() {
    let d = Document::new(1);
    let origin = line(&d, 1, (0.0, 0.0), (100.0, 0.0));
    let clone = d.linked_clone(&[origin], &[]).unwrap()[0];
    let t = text(&d, "Hi");
    d.put_text_on_path(t, on(clone), &OnPathValues::at(Length::from_mm(0.0)), true)
        .unwrap();
    assert!(is_guide(&d, clone));
    assert!(!is_guide(&d, origin));
    assert!(on_path(&d, t).curve.is_some());
    d.set_guide(&[clone], false).unwrap();
    assert!(!is_guide(&d, clone));
}

/// Criterion 32: several texts on one carrier, each its own values.
#[test]
fn c32_several_texts_on_one_carrier() {
    let d = Document::new(1);
    let c = circle(&d);
    let top = text(&d, "TOP");
    let bottom = text(&d, "BOTTOM");
    d.put_text_on_path(top, on(c), &OnPathValues::at(Length::from_mm(0.0)), true)
        .unwrap();
    d.put_text_on_path(
        bottom,
        on(c),
        &OnPathValues::at(Length::from_mm(80.0)).mirrored(),
        true,
    )
    .unwrap();
    assert_eq!(d.read_objects().texts_on.get(&c).unwrap().len(), 2);
    assert_eq!(on_path(&d, bottom).values.side, PathSide::Below);
    assert_eq!(on_path(&d, top).values.side, PathSide::Above);
}

/// Criterion 36: a peer moves the carrier while this one edits the string; both survive.
#[test]
fn c36_a_moved_carrier_and_an_edited_string_converge() {
    let a = Document::new(1);
    let t = text(&a, "Hello");
    let c = line(&a, 1, (0.0, 0.0), (100.0, 0.0));
    a.put_text_on_path(t, on(c), &OnPathValues::at(Length::from_mm(0.0)), true)
        .unwrap();
    let b = unpack(2, &pack(&a, "0.1.0").unwrap()).unwrap();
    b.move_anchors(&[(c, AnchorId::new(9, 2), Point::new(100.0, 40.0))])
        .unwrap();
    a.edit_text(
        t,
        "Hello world",
        &curvyo_document_core::TextChars::default(),
    )
    .unwrap();
    a.import_updates(&b.export_updates(&a.version()).unwrap())
        .unwrap();
    b.import_updates(&a.export_updates(&b.version()).unwrap())
        .unwrap();
    for d in [&a, &b] {
        let read = read_text(d, t);
        assert_eq!(read.string, "Hello world");
        assert_eq!(
            read.on_path.unwrap().curve.unwrap().anchors[1].point,
            Point::new(100.0, 40.0)
        );
    }
}

#[allow(clippy::too_many_lines)]
fn golden_document() -> (Document, Vec<NodeId>) {
    let d = Document::new(1);
    let mut texts = Vec::new();
    let mut put = |carrier: CarrierRef, values: OnPathValues| {
        let t = text(&d, "Curvyo");
        d.put_text_on_path(t, carrier, &values, true).unwrap();
        texts.push(t);
        t
    };
    let open = line(&d, 1, (0.0, 0.0), (120.0, 30.0));
    put(on(open), OnPathValues::at(Length::from_mm(0.0)));
    let c = circle(&d);
    for (side, line_on) in [
        (PathSide::Above, LineOnPath::Baseline),
        (PathSide::Below, LineOnPath::TopEdge),
        (PathSide::Above, LineOnPath::TopEdge),
        (PathSide::Below, LineOnPath::Baseline),
    ] {
        put(
            on(c),
            OnPathValues {
                side,
                line: line_on,
                position: Length::from_mm(40.0),
                distance: Length::from_mm(1.5),
                mirror_letters: side == PathSide::Below && line_on == LineOnPath::Baseline,
            },
        );
    }
    let rect = d.create_primitive(
        Shape::Rect {
            bounds: RectBounds::from_corners(Point::new(200.0, 0.0), Point::new(260.0, 30.0)),
            corner_radii: curvyo_document_core::CornerRadii::uniform(Length::from_mm(6.0)),
        },
        Angle::from_radians(0.0),
    );
    put(on(rect), OnPathValues::at(Length::from_mm(10.0)));
    let star = d.create_primitive(
        Shape::Star {
            frame: StarFrame::from_center_and_vertex(
                Point::new(300.0, 100.0),
                Point::new(300.0, 60.0),
            ),
            point_count: PointCount::new(5).unwrap(),
            inner_ratio: curvyo_document_core::InnerRatio::new(0.5).unwrap(),
            curve: Curve::POLYGON_DEFAULT,
        },
        Angle::from_radians(0.0),
    );
    put(on(star), OnPathValues::at(Length::from_mm(5.0)));
    let arc = d.create_primitive(
        Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(100.0, 250.0),
                rx: Length::from_mm(60.0),
                ry: Length::from_mm(30.0),
            },
            arc: EllipseArc::new(
                Angle::from_radians(std::f64::consts::PI),
                Angle::from_radians(0.0),
                ArcKind::Arc,
            ),
            curve: Curve::ELLIPSE_DEFAULT,
        },
        Angle::from_radians(0.0),
    );
    put(on(arc), OnPathValues::at(Length::from_mm(3.0)));
    let a = line(&d, 20, (0.0, 300.0), (50.0, 300.0));
    let outlines = vec![
        (
            vec![
                NewAnchor::corner(AnchorId::new(8, 1), Point::new(0.0, 300.0)),
                NewAnchor::corner(AnchorId::new(8, 2), Point::new(50.0, 300.0)),
            ],
            false,
        ),
        (
            vec![
                NewAnchor::corner(AnchorId::new(8, 3), Point::new(0.0, 330.0)),
                NewAnchor::corner(AnchorId::new(8, 4), Point::new(80.0, 330.0)),
            ],
            false,
        ),
    ];
    let compound = d
        .replace_with_path(&[a], a, &outlines, "combine_paths")
        .unwrap();
    put(
        CarrierRef {
            node: compound,
            outline: 1,
        },
        OnPathValues::at(Length::from_mm(2.0)),
    );
    let doomed = line(&d, 30, (0.0, 400.0), (100.0, 420.0));
    put(on(doomed), OnPathValues::at(Length::from_mm(0.0)));
    d.delete_objects(&[doomed]).unwrap();
    let origin = line(&d, 40, (0.0, 500.0), (100.0, 500.0));
    let clone = d.linked_clone(&[origin], &[]).unwrap()[0];
    put(on(clone), OnPathValues::at(Length::from_mm(0.0)));
    // A carrier id that names nothing (criterion 35): a text pointed at a node deleted with no
    // kept copy, by a peer's merge.
    let t = text(&d, "Nowhere");
    let ghost = line(&d, 50, (0.0, 600.0), (10.0, 600.0));
    let peer = unpack(2, &pack(&d, "0.1.0").unwrap()).unwrap();
    peer.delete_objects(&[ghost]).unwrap();
    d.put_text_on_path(t, on(ghost), &OnPathValues::at(Length::from_mm(0.0)), false)
        .unwrap();
    d.import_updates(&peer.export_updates(&d.version()).unwrap())
        .unwrap();
    texts.push(t);
    (d, texts)
}

/// Criterion 34: format 22; the golden file opens with every text on its carrier, the kept copy,
/// the clone guide, and a text whose carrier names nothing reads straight (criterion 35).
#[test]
fn c34_golden_file_of_format_22() {
    assert_eq!(CURRENT_FORMAT_VERSION, 22);
    let (document, texts) = golden_document();
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(fixture_path(GOLDEN), pack(&document, "0.1.0").unwrap()).unwrap();
    }
    let stored = std::fs::read(fixture_path(GOLDEN)).unwrap();
    let opened = unpack(3, &stored).unwrap();
    let ids: Vec<NodeId> = opened
        .object_ids()
        .into_iter()
        .filter(|id| opened.text(*id).is_some())
        .collect();
    assert_eq!(ids.len(), texts.len());
    let read: Vec<TextOnPath> = ids
        .iter()
        .filter_map(|id| read_text(&opened, *id).on_path)
        .collect();
    let original: Vec<TextOnPath> = texts
        .iter()
        .filter_map(|id| read_text(&document, *id).on_path)
        .collect();
    assert_eq!(read.len(), original.len());
    for (a, b) in read.iter().zip(&original) {
        assert_eq!(a.carrier, b.carrier);
        assert_eq!(a.values, b.values);
        assert_eq!(a.kept, b.kept);
        assert_eq!(a.curve, b.curve);
    }
    assert!(read.iter().any(|p| p.kept), "a kept outline");
    assert!(
        read.iter().any(|p| p.curve.is_none()),
        "a carrier that names nothing reads straight"
    );
    let guides = opened.read_nodes().iter().filter(|n| n.guide).count();
    assert_eq!(guides, 7, "open, circle, rect, star, arc, compound, clone");
    // Saved again, it reads the same.
    let again = unpack(4, &pack(&opened, "0.1.0").unwrap()).unwrap();
    let again_read: Vec<TextOnPath> = ids
        .iter()
        .filter_map(|id| read_text(&again, *id).on_path)
        .collect();
    assert_eq!(again_read, read);
}
