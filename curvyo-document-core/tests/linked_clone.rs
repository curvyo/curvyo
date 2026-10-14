//! Linked clone and the read of clones (`specs/0049-linked-clone` criteria 1 to 3, 7 to 9, 50,
//! 52 and 63, `adrs.md` decisions 1 to 3).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use curvyo_document_core::{
    Affine, AnchorId, CloneError, CloneSnapshot, Color, CornerRadii, Document,
    LINKED_CLONE_COMMIT_LABEL, Length, MAX_CLONE_DEPTH, NewAnchor, NodeId, ObjectSnapshot,
    PathSnapshot, Point, RectBounds, StyleEdit, Vec2, outline_of_rotated,
};

fn square(document: &Document, n: u64, at: Point) -> NodeId {
    let corner = |k: u64, dx: f64, dy: f64| {
        NewAnchor::corner(
            AnchorId::new(1, n * 10 + k),
            Point::new(at.x + dx, at.y + dy),
        )
    };
    document.create_path(
        &[
            corner(0, 0.0, 0.0),
            corner(1, 10.0, 0.0),
            corner(2, 10.0, 10.0),
            corner(3, 0.0, 10.0),
        ],
        true,
    )
}

fn origin() -> Point {
    Point::new(0.0, 0.0)
}

fn clone_of(document: &Document, id: NodeId) -> CloneSnapshot {
    match document.object(id).unwrap() {
        ObjectSnapshot::Clone(clone) => clone,
        other => panic!("not a clone: {other:?}"),
    }
}

fn path_of(document: &Document, id: NodeId) -> PathSnapshot {
    match document.object(id).unwrap() {
        ObjectSnapshot::Path(path) => path,
        other => panic!("not a path: {other:?}"),
    }
}

fn last_label(document: &Document) -> String {
    document.own_steps().last().unwrap().label.clone()
}

/// The same points, every one within 1e-9 mm.
fn same_outline(a: &PathSnapshot, b: &PathSnapshot) {
    let (a, b): (Vec<_>, Vec<_>) = (a.all_anchors().collect(), b.all_anchors().collect());
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        assert!((x.point.x - y.point.x).abs() < 1e-9, "{x:?} {y:?}");
        assert!((x.point.y - y.point.y).abs() < 1e-9, "{x:?} {y:?}");
        assert!((x.handle_in.x - y.handle_in.x).abs() < 1e-9);
        assert!((x.handle_out.y - y.handle_out.y).abs() < 1e-9);
    }
}

#[test]
fn ac1_one_clone_above_each_source_in_one_commit() {
    let d = Document::new(1);
    let a = square(&d, 1, origin());
    let b = square(&d, 2, Point::new(20.0, 0.0));
    let c = square(&d, 3, Point::new(40.0, 0.0));
    let steps = d.own_step_count();
    let clones = d.linked_clone(&[a, b, c, a], &[]).unwrap();
    assert_eq!(clones.len(), 3, "a source named twice is cloned once");
    assert_eq!(d.own_step_count(), steps + 1, "one commit");
    assert_eq!(last_label(&d), LINKED_CLONE_COMMIT_LABEL);
    assert_eq!(LINKED_CLONE_COMMIT_LABEL, "linked_clone");
    assert_eq!(
        d.object_ids(),
        vec![a, clones[0], b, clones[1], c, clones[2]]
    );
    for (&clone, source) in clones.iter().zip([a, b, c]) {
        let snapshot = clone_of(&d, clone);
        assert_eq!(snapshot.chain.origin, source);
        assert_eq!(snapshot.chain.root, source);
        assert_eq!(snapshot.chain.depth, 1);
        assert_eq!(snapshot.chain.matrix, Affine::IDENTITY);
        assert_eq!(snapshot.resolved.id, clone);
        same_outline(&snapshot.resolved, &path_of(&d, source));
    }
}

#[test]
fn ac1_a_clone_is_made_inside_the_group_of_its_source() {
    let d = Document::new(1);
    let a = square(&d, 1, origin());
    let b = square(&d, 2, Point::new(20.0, 0.0));
    let group = d.group_objects(&[a, b]).unwrap();
    let clone = d.linked_clone(&[a], &[]).unwrap()[0];
    assert_eq!(d.children(Some(group)), vec![a, clone, b]);
}

fn rect_conversion(d: &Document, id: NodeId) -> Vec<(NodeId, Vec<NewAnchor>)> {
    let ObjectSnapshot::Primitive(rect) = d.object(id).unwrap() else {
        panic!("a rectangle")
    };
    let anchors = outline_of_rotated(&rect.shape, rect.rotation)
        .into_iter()
        .enumerate()
        .map(|(k, a)| NewAnchor {
            id: AnchorId::new(7, k as u64),
            point: a.point,
            handle_in: a.handle_in,
            handle_out: a.handle_out,
            kind: a.kind,
        })
        .collect();
    vec![(id, anchors)]
}

#[test]
fn ac2_a_rectangle_is_converted_in_the_same_commit() {
    let d = Document::new(1);
    let r = d.create_rect(RectBounds {
        origin: Point::new(5.0, 5.0),
        width: Length::from_mm(40.0),
        height: Length::from_mm(30.0),
    });
    d.set_corner_radii(&[(
        r,
        CornerRadii {
            tl: Length::from_mm(1.0),
            tr: Length::from_mm(2.0),
            br: Length::from_mm(3.0),
            bl: Length::from_mm(4.0),
        },
    )])
    .unwrap();
    d.edit_style(
        &[r],
        &StyleEdit::StrokeColor(Color {
            r: 200,
            g: 10,
            b: 10,
        }),
    )
    .unwrap();
    let style = d.object(r).unwrap().style().clone();
    let steps = d.own_step_count();
    let conversions = rect_conversion(&d, r);
    let clone = d.linked_clone(&[r], &conversions).unwrap()[0];
    assert_eq!(
        d.own_step_count(),
        steps + 1,
        "conversion and clone are one commit"
    );
    let path = path_of(&d, r);
    assert_eq!(path.anchors.len(), 8);
    assert_eq!(path.style, style, "same style");
    assert_eq!(d.object_ids(), vec![r, clone], "same id, same place");
    same_outline(&clone_of(&d, clone).resolved, &path);
}

#[test]
fn ac2_a_primitive_needs_exactly_its_conversion_and_nothing_is_written_otherwise() {
    let d = Document::new(1);
    let r = d.create_rect(RectBounds {
        origin: origin(),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    });
    let p = square(&d, 1, origin());
    let before = d.export_json().unwrap();
    assert_eq!(
        d.linked_clone(&[r], &[]),
        Err(CloneError::ConversionMismatch)
    );
    let mut wrong = rect_conversion(&d, r);
    wrong[0].0 = p;
    assert_eq!(
        d.linked_clone(&[r, p], &wrong),
        Err(CloneError::ConversionMismatch)
    );
    assert_eq!(d.export_json().unwrap(), before);
}

#[test]
fn ac3_a_clone_of_a_clone_is_a_chain() {
    let d = Document::new(1);
    let p = square(&d, 1, origin());
    let c1 = d.linked_clone(&[p], &[]).unwrap()[0];
    let c2 = d.linked_clone(&[c1], &[]).unwrap()[0];
    let second = clone_of(&d, c2);
    assert_eq!(
        (second.chain.origin, second.chain.root, second.chain.depth),
        (c1, p, 2)
    );
    assert_eq!(second.chain.matrix, Affine::IDENTITY);
    assert_eq!(d.object_ids(), vec![p, c1, c2]);
    same_outline(&second.resolved, &clone_of(&d, c1).resolved);
}

#[test]
fn ac4_a_group_or_a_missing_source_is_refused_and_nothing_is_converted() {
    let d = Document::new(1);
    let a = square(&d, 1, origin());
    let b = square(&d, 2, origin());
    let r = d.create_rect(RectBounds {
        origin: origin(),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    });
    let group = d.group_objects(&[a, b]).unwrap();
    let before = d.export_json().unwrap();
    let conversions = rect_conversion(&d, r);
    assert_eq!(
        d.linked_clone(&[r, group], &conversions),
        Err(CloneError::NotClonable)
    );
    d.delete_objects(&[r]).unwrap();
    let after_delete = d.export_json().unwrap();
    assert_ne!(before, after_delete);
    assert_eq!(d.linked_clone(&[a, r], &[]), Err(CloneError::NoSuchObject));
    assert_eq!(d.export_json().unwrap(), after_delete);
}

#[test]
fn ac52_the_ninth_level_is_refused_all_or_nothing() {
    let d = Document::new(1);
    let p = square(&d, 1, origin());
    let other = square(&d, 2, Point::new(50.0, 0.0));
    let mut deepest = p;
    for level in 1..=MAX_CLONE_DEPTH {
        deepest = d.linked_clone(&[deepest], &[]).unwrap()[0];
        assert_eq!(clone_of(&d, deepest).chain.depth, level);
    }
    let before = d.export_json().unwrap();
    assert_eq!(
        d.linked_clone(&[other, deepest], &[]),
        Err(CloneError::TooDeep(vec![deepest]))
    );
    assert_eq!(d.export_json().unwrap(), before, "nothing created");
    // Select root original, then Linked clone: depth 1.
    let fresh = d
        .linked_clone(&[clone_of(&d, deepest).chain.root], &[])
        .unwrap()[0];
    assert_eq!(clone_of(&d, fresh).chain.depth, 1);
}

#[test]
fn ac7_8_9_the_clone_shows_the_root_outline_style_and_stroke_width() {
    let d = Document::new(1);
    let p = square(&d, 1, origin());
    d.edit_style(&[p], &StyleEdit::StrokeWidth(Length::from_mm(0.5)))
        .unwrap();
    let c = d.linked_clone(&[p], &[]).unwrap()[0];
    let c2 = d.linked_clone(&[c], &[]).unwrap()[0];
    d.edit_style(&[p], &StyleEdit::StrokeColor(Color { r: 255, g: 0, b: 0 }))
        .unwrap();
    let read = d.read_objects();
    for object in read.objects.iter() {
        assert_eq!(object.style(), &path_of(&d, p).style);
    }
    let resolved = clone_of(&d, c2).resolved;
    assert_eq!(resolved.style.stroke.width, Length::from_mm(0.5));
    assert_eq!(resolved.style.stroke.color, Color { r: 255, g: 0, b: 0 });
}

#[test]
fn ac50_the_read_of_a_chain_and_its_clones_of() {
    let d = Document::new(1);
    let o = square(&d, 1, origin());
    let a = d.linked_clone(&[o], &[]).unwrap()[0];
    let b = d.linked_clone(&[a], &[]).unwrap()[0];
    let read = d.read_objects();
    assert_eq!(read.objects.len(), 3);
    assert_eq!(read.clones_of[&o], vec![a]);
    assert_eq!(read.clones_of[&a], vec![b]);
    assert_eq!(read.orphans, Vec::<NodeId>::new());
    let ids: Vec<NodeId> = read.objects.iter().map(ObjectSnapshot::id).collect();
    assert_eq!(ids, d.object_ids());
    for id in [o, a, b] {
        assert_eq!(
            read.objects.iter().find(|x| x.id() == id).cloned(),
            d.object(id),
            "one object read and the whole read agree"
        );
    }
}

#[test]
fn ac7_a_clone_moves_with_its_original() {
    let d = Document::new(1);
    let p = square(&d, 1, origin());
    let c = d.linked_clone(&[p], &[]).unwrap()[0];
    d.translate_objects(&[p], Vec2::new(5.0, 7.0)).unwrap();
    same_outline(&clone_of(&d, c).resolved, &path_of(&d, p));
}

#[test]
fn ac27_object_to_path_refuses_a_clone() {
    let d = Document::new(1);
    let p = square(&d, 1, origin());
    let c = d.linked_clone(&[p], &[]).unwrap()[0];
    let before = d.export_json().unwrap();
    assert!(d.convert_to_paths(&[(c, Vec::new())]).is_err());
    assert_eq!(d.export_json().unwrap(), before);
}
