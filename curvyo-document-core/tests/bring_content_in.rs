//! Clones of groups, texts, images and primitives, and placing linked clones by a drop
//! (`specs/0100-bring-content-in` criteria 1, 6 to 12; ADR 0045 §1 to §6).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names
)]

use std::sync::Arc;

use curvyo_document_core::{
    Affine, AnchorId, CloneError, CloneSnapshot, Document, EllipseFrame, FontKind, FontRef, Length,
    MovePlan, MoveTarget, NewAnchor, NewImage, NodeId, ObjectSnapshot, Opacity,
    PLACE_LINKED_CLONES_COMMIT_LABEL, Point, RectBounds, TextLayoutValues, Vec2,
};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn square(d: &Document, n: u64, x: f64, y: f64) -> NodeId {
    let c = |k: u64, dx: f64, dy: f64| {
        NewAnchor::corner(AnchorId::new(1, n * 10 + k), pt(x + dx, y + dy))
    };
    d.create_path(
        &[
            c(0, 0.0, 0.0),
            c(1, 10.0, 0.0),
            c(2, 10.0, 10.0),
            c(3, 0.0, 10.0),
        ],
        true,
    )
}

fn text(d: &Document, s: &str) -> NodeId {
    d.create_text(
        s,
        &FontRef {
            family: "Source Sans 3".into(),
            style: "Regular".into(),
            face_hash: "c".repeat(64),
            face_index: 0,
            kind: FontKind::Outline,
        },
        Length::from_mm(10.0),
        Affine::translation(Vec2::new(5.0, 40.0)),
        &TextLayoutValues::DEFAULT,
    )
    .unwrap()
}

fn image(d: &Document) -> NodeId {
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-image-core/tests/fixtures/images/png_rgb_8.png"),
    )
    .unwrap();
    d.add_images(&[NewImage {
        bytes: Arc::from(bytes),
        placement: Affine::new(20.0, 0.0, 0.0, 10.0, 30.0, 0.0),
        name: "photo".into(),
    }])
    .unwrap()[0]
}

/// A group G of the text "ANNA", a rectangle, a circle, a path and an image: (doc, G, members).
fn group() -> (Document, NodeId, Vec<NodeId>) {
    let d = Document::new(1);
    let t = text(&d, "ANNA");
    let r = d.create_rect(RectBounds::from_corners(pt(0.0, 0.0), pt(20.0, 10.0)));
    let c = d.create_ellipse(EllipseFrame {
        center: pt(50.0, 5.0),
        rx: Length::from_mm(5.0),
        ry: Length::from_mm(5.0),
    });
    let p = square(&d, 1, 60.0, 0.0);
    let i = image(&d);
    let members = vec![t, r, c, p, i];
    let g = d.group_objects(&members).unwrap();
    (d, g, members)
}

fn clone_of(d: &Document, id: NodeId) -> CloneSnapshot {
    match d.read_objects().get(id) {
        Some(ObjectSnapshot::Clone(clone)) => clone.clone(),
        other => panic!("not a clone: {other:?}"),
    }
}

fn turned() -> Affine {
    Affine::translation(Vec2::new(300.0, 20.0))
        .compose(Affine::rotation_about(
            pt(0.0, 0.0),
            curvyo_document_core::Angle::from_radians(30f64.to_radians()),
        ))
        .compose(Affine::scale(0.5, 0.5))
}

/// Every anchor of `a` under `m` equals the anchor of `b` within `tol`.
fn same_outline(
    a: &curvyo_document_core::PathSnapshot,
    m: Affine,
    b: &curvyo_document_core::PathSnapshot,
    tol: f64,
) {
    let a: Vec<Point> = a.all_anchors().map(|x| m.apply_point(x.point)).collect();
    let b: Vec<Point> = b.all_anchors().map(|x| x.point).collect();
    assert_eq!(a.len(), b.len());
    for (p, q) in a.iter().zip(&b) {
        assert!(
            (p.x - q.x).abs() < tol && (p.y - q.y).abs() < tol,
            "{p:?} vs {q:?}"
        );
    }
}

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

#[test]
fn criterion_8_linked_clone_takes_a_group_a_text_and_an_image() {
    let (d, g, members) = group();
    let made = d.linked_clone(&[g, members[0], members[4]], &[]).unwrap();
    assert_eq!(made.len(), 3);
    for &clone in &made {
        assert!(clone_of(&d, clone).is_picture());
    }
    let layer = d.new_layer(None);
    assert_eq!(d.linked_clone(&[layer], &[]), Err(CloneError::NotClonable));
}

#[test]
fn criterion_9_a_group_clone_draws_every_member_under_its_matrix() {
    let (d, g, members) = group();
    let clone = d.place_linked_clones(&[g], Vec2::new(0.0, 0.0)).unwrap()[0];
    let m = turned();
    let mut matrix_written = d.object(clone).unwrap();
    if let ObjectSnapshot::Clone(c) = &mut matrix_written {
        *c = c.mapped_by(m);
    }
    d.rotate_object(&matrix_written).unwrap();
    let read = d.read_objects();
    let ObjectSnapshot::Clone(shown) = read.get(clone).unwrap() else {
        panic!()
    };
    let picture = shown.chain.picture.as_ref().unwrap();
    assert_eq!(picture.len(), 5, "text, rectangle, circle, path, image");
    // The path member under the matrix within 1e-9 mm.
    let ObjectSnapshot::Path(path) = read.get(members[3]).unwrap() else {
        panic!()
    };
    let ObjectSnapshot::Path(drawn) = &picture[3] else {
        panic!("a path leaf")
    };
    same_outline(path, shown.chain.composed, drawn, 1e-9);
    // The image's frame is composed, the text's placement too.
    let ObjectSnapshot::Image(original) = read.get(members[4]).unwrap() else {
        panic!()
    };
    let ObjectSnapshot::Image(drawn) = &picture[4] else {
        panic!()
    };
    assert!(
        drawn
            .placement
            .approx_eq(shown.chain.composed.compose(original.placement), 1e-9)
    );
    assert!(matches!(picture[0], ObjectSnapshot::Text(_)));
    // The primitive members stay primitives (never converted), drawn as their outline.
    assert!(matches!(
        read.get(members[1]),
        Some(ObjectSnapshot::Primitive(_))
    ));
    assert_eq!(shown.chain.root, g);
    assert!(shown.chain.own_placement);
}

#[test]
fn criterion_9_hidden_members_guides_and_dimensions_are_left_out_but_not_the_groups_own_flag() {
    let (d, g, members) = group();
    let clone = d.linked_clone(&[g], &[]).unwrap()[0];
    d.set_node_visible(&[members[2]], false).unwrap();
    d.set_guide(&[members[3]], true).unwrap();
    d.set_node_visible(&[g], false).unwrap();
    let picture = clone_of(&d, clone).chain.picture.unwrap();
    assert_eq!(picture.len(), 3, "the circle is hidden, the path a guide");
    let ids: Vec<NodeId> = picture.iter().map(ObjectSnapshot::id).collect();
    assert_eq!(ids, vec![members[0], members[1], members[4]]);
}

#[test]
fn criterion_9_a_members_opacity_is_counted_through_the_group() {
    let (d, g, members) = group();
    let clone = d.linked_clone(&[g], &[]).unwrap()[0];
    d.set_node_opacity(&[members[3]], Opacity::new(0.5).unwrap())
        .unwrap();
    d.set_node_opacity(&[g], Opacity::new(0.5).unwrap())
        .unwrap();
    let picture = clone_of(&d, clone).chain.picture.unwrap();
    let ObjectSnapshot::Path(path) = &picture[3] else {
        panic!()
    };
    let stroke = d.path(members[3]).unwrap().style.stroke.opacity.get();
    assert!((path.style.stroke.opacity.get() - 0.25 * stroke).abs() < 1e-12);
}

#[test]
fn criterion_7_edits_in_the_group_flow_into_the_clone_and_a_whole_move_of_g_does_not() {
    let (d, g, members) = group();
    let clone = d.place_linked_clones(&[g], Vec2::new(300.0, 0.0)).unwrap()[0];
    let before = clone_of(&d, clone).chain.picture.unwrap().len();
    // (d) a path drawn into G.
    d.set_draw_parent(Some(g));
    let added = square(&d, 7, 80.0, 0.0);
    assert_eq!(d.parent_group(added), Some(g));
    assert_eq!(clone_of(&d, clone).chain.picture.unwrap().len(), before + 1);
    // (e) the circle moved out of G.
    d.move_nodes(&MovePlan {
        nodes: vec![members[2]],
        target: MoveTarget::FrontOf(None),
    })
    .unwrap();
    let picture = clone_of(&d, clone).chain.picture.unwrap();
    assert_eq!(picture.len(), before);
    assert!(!picture.iter().any(|leaf| leaf.id() == members[2]));
    // (a) a node edit of the path member shows at once.
    let path_before = clone_of(&d, clone).chain.picture.unwrap();
    d.translate_objects(&[members[3]], Vec2::new(1.0, 0.0))
        .unwrap();
    let path_after = clone_of(&d, clone).chain.picture.unwrap();
    assert_ne!(path_before, path_after);
    // (f) G moved as a whole: the clone with own placement stays.
    let shown = clone_of(&d, clone).chain.picture.unwrap();
    let leaves = d.leaves_of(g);
    d.translate_objects(&leaves, Vec2::new(0.0, 50.0)).unwrap();
    let after = clone_of(&d, clone).chain.picture.unwrap();
    for (a, b) in shown.iter().zip(after.iter()) {
        let (a, b) = (bounds(a), bounds(b));
        assert!(
            (a.0.x - b.0.x).abs() < 1e-9 && (a.0.y - b.0.y).abs() < 1e-9,
            "{a:?} {b:?}"
        );
    }
}

fn bounds(leaf: &ObjectSnapshot) -> (Point, Point) {
    match leaf {
        ObjectSnapshot::Image(image) => {
            let p = image.placement.apply_point(pt(0.0, 0.0));
            (p, p)
        }
        ObjectSnapshot::Text(text) => {
            let p = text.placement.apply_point(pt(0.0, 0.0));
            (p, p)
        }
        other => {
            let path = other.drawn_path().unwrap();
            let p = path.anchors[0].point;
            (p, p)
        }
    }
}

#[test]
fn a_chain_through_a_group_clone_composes() {
    let (d, g, _) = group();
    let a = d.place_linked_clones(&[g], Vec2::new(100.0, 0.0)).unwrap()[0];
    let b = d.place_linked_clones(&[a], Vec2::new(0.0, 100.0)).unwrap()[0];
    let (ca, cb) = (clone_of(&d, a), clone_of(&d, b));
    assert_eq!((ca.chain.depth, cb.chain.depth), (1, 2));
    assert_eq!(cb.chain.root, g);
    assert!(
        cb.chain
            .composed
            .approx_eq(Affine::translation(Vec2::new(100.0, 100.0)), 1e-12)
    );
}

#[test]
fn a_clone_of_a_primitive_is_not_converted_and_draws_its_outline() {
    let d = Document::new(1);
    let r = d.create_rect(RectBounds::from_corners(pt(0.0, 0.0), pt(20.0, 10.0)));
    let clone = d.place_linked_clones(&[r], Vec2::new(5.0, 0.0)).unwrap()[0];
    assert!(matches!(d.object(r), Some(ObjectSnapshot::Primitive(_))));
    let shown = clone_of(&d, clone);
    assert!(!shown.is_picture());
    let xs: Vec<f64> = shown.resolved.anchors.iter().map(|a| a.point.x).collect();
    assert!(xs.iter().all(|x| (5.0 - 1e-9..=25.0 + 1e-9).contains(x)));
}

#[test]
fn criterion_1_place_linked_clones_is_one_step_on_top_and_undo_removes_them() {
    let d = Document::new(1);
    let a = square(&d, 1, 0.0, 0.0);
    let b = square(&d, 2, 20.0, 0.0);
    let before_a = d.object(a).unwrap();
    let made = d
        .place_linked_clones(&[a, b], Vec2::new(300.0, 0.0))
        .unwrap();
    assert_eq!(last_label(&d), PLACE_LINKED_CLONES_COMMIT_LABEL);
    assert_eq!(d.object(a).unwrap(), before_a);
    let ids: Vec<NodeId> = d
        .read_objects()
        .objects
        .iter()
        .map(ObjectSnapshot::id)
        .collect();
    assert_eq!(&ids[2..], &made[..], "on top, in selection order");
    for &clone in &made {
        let c = clone_of(&d, clone);
        assert!(c.chain.own_placement);
        assert_eq!(c.chain.matrix, Affine::translation(Vec2::new(300.0, 0.0)));
    }
    d.undo_step(d.own_steps().last().unwrap().step).unwrap();
    assert_eq!(d.read_objects().objects.len(), 2);
}

#[test]
fn criterion_6_a_clone_eight_deep_refuses_the_whole_drop() {
    let d = Document::new(1);
    let mut last = square(&d, 1, 0.0, 0.0);
    for _ in 0..8 {
        last = d.linked_clone(&[last], &[]).unwrap()[0];
    }
    let other = square(&d, 2, 50.0, 0.0);
    let count = d.read_objects().objects.len();
    assert_eq!(
        d.place_linked_clones(&[other, last], Vec2::new(1.0, 0.0)),
        Err(CloneError::TooDeep(vec![last]))
    );
    assert_eq!(d.read_objects().objects.len(), count);
}

#[test]
fn criterion_12_a_move_that_puts_a_clone_inside_its_original_is_refused() {
    let (d, g, members) = group();
    let clone = d.linked_clone(&[g], &[]).unwrap()[0];
    let plan = MovePlan {
        nodes: vec![clone],
        target: MoveTarget::FrontOf(Some(g)),
    };
    assert_eq!(
        d.move_nodes(&plan),
        Err(curvyo_document_core::MoveRefusal::InsideOwnOriginal)
    );
    assert!(d.would_cycle(&[clone], Some(g)));
    // A clone of a member inside G is fine.
    let member_clone = d.linked_clone(&[members[3]], &[]).unwrap()[0];
    assert!(!d.would_cycle(&[member_clone], Some(g)));
}

#[test]
fn criterion_12_two_groups_that_would_draw_each_other_are_refused() {
    let d = Document::new(1);
    let a = square(&d, 1, 0.0, 0.0);
    let a2 = square(&d, 3, 0.0, 20.0);
    let b = square(&d, 2, 20.0, 0.0);
    let b2 = square(&d, 4, 20.0, 20.0);
    let g1 = d.group_objects(&[a, a2]).unwrap();
    let g2 = d.group_objects(&[b, b2]).unwrap();
    let c1 = d.linked_clone(&[g1], &[]).unwrap()[0];
    d.move_nodes(&MovePlan {
        nodes: vec![c1],
        target: MoveTarget::FrontOf(Some(g2)),
    })
    .unwrap();
    let c2 = d.linked_clone(&[g2], &[]).unwrap()[0];
    let before = d.export_json().unwrap();
    assert_eq!(
        d.move_nodes(&MovePlan {
            nodes: vec![c2],
            target: MoveTarget::FrontOf(Some(g1)),
        }),
        Err(curvyo_document_core::MoveRefusal::InsideOwnOriginal)
    );
    assert_eq!(d.export_json().unwrap(), before);
}

/// The drawn path leaves of `ids` (a group's leaves, or one object), mapped as drawn.
fn drawn_paths(d: &Document, ids: &[NodeId]) -> Vec<curvyo_document_core::PathSnapshot> {
    let read = d.read_objects();
    ids.iter()
        .filter_map(|&id| match read.get(id)? {
            ObjectSnapshot::Path(path) => Some(path.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn criterion_10_unlink_of_a_group_clone_gives_a_group_of_copies_drawing_the_same() {
    let (d, g, members) = group();
    let clone = d.place_linked_clones(&[g], Vec2::new(200.0, 30.0)).unwrap()[0];
    let shown: Vec<curvyo_document_core::PathSnapshot> = clone_of(&d, clone)
        .chain
        .picture
        .unwrap()
        .iter()
        .filter_map(|leaf| match leaf {
            ObjectSnapshot::Path(p) if p.id == members[3] => Some(p.clone()),
            _ => None,
        })
        .collect();
    d.unlink_clones(&[curvyo_document_core::CopySource {
        id: clone,
        anchor_ids: Vec::new(),
    }])
    .unwrap();
    assert_eq!(
        d.own_steps().last().unwrap().label,
        curvyo_document_core::UNLINK_CLONE_COMMIT_LABEL
    );
    let copy = clone;
    assert!(d.is_group(copy), "the clone's node is the copy now");
    let leaves = d.leaves_of(copy);
    assert_eq!(leaves.len(), 5);
    let paths = drawn_paths(&d, &leaves);
    // The copied path draws where the clone drew it, within 0.01 mm.
    same_outline(&shown[0], Affine::IDENTITY, &paths[0], 0.01);
    // A translated rectangle stays a primitive.
    assert!(
        leaves
            .iter()
            .any(|&id| matches!(d.object(id), Some(ObjectSnapshot::Primitive(_))))
    );
    // The original is unchanged.
    assert_eq!(d.leaves_of(g), members);
}

#[test]
fn criterion_11_deleting_g_turns_its_clones_into_copies_in_one_step_and_undo_restores() {
    let (d, g, _) = group();
    let clone = d.place_linked_clones(&[g], Vec2::new(200.0, 0.0)).unwrap()[0];
    let chained = d
        .place_linked_clones(&[clone], Vec2::new(0.0, 50.0))
        .unwrap()[0];
    let steps = d.own_steps().len();
    d.delete_objects(&[g]).unwrap();
    assert_eq!(d.own_steps().len(), steps + 1, "one step");
    assert!(d.is_group(clone));
    assert_eq!(d.leaves_of(clone).len(), 5);
    // The chained clone follows the copy now: same id.
    let follower = clone_of(&d, chained);
    assert_eq!(follower.chain.origin, clone);
    assert!(follower.is_picture());
    d.undo_step(d.own_steps().last().unwrap().step).unwrap();
    assert!(d.is_group(g));
    assert!(clone_of(&d, clone).is_picture());
    assert_eq!(clone_of(&d, chained).chain.origin, clone);
}

#[test]
fn criterion_10_a_clone_of_a_text_and_of_an_image_unlink_into_a_text_and_an_image() {
    let d = Document::new(1);
    let t = text(&d, "ANNE");
    let i = image(&d);
    let clones = d
        .place_linked_clones(&[t, i], Vec2::new(0.0, 100.0))
        .unwrap();
    let sources: Vec<_> = clones
        .iter()
        .map(|&id| curvyo_document_core::CopySource {
            id,
            anchor_ids: Vec::new(),
        })
        .collect();
    d.unlink_clones(&sources).unwrap();
    let Some(ObjectSnapshot::Text(text)) = d.object(clones[0]) else {
        panic!("a text")
    };
    assert_eq!(text.string, "ANNE");
    assert!(
        text.placement
            .approx_eq(Affine::translation(Vec2::new(5.0, 140.0)), 1e-9)
    );
    let Some(ObjectSnapshot::Image(image)) = d.object(clones[1]) else {
        panic!("an image")
    };
    assert!(
        image
            .placement
            .approx_eq(Affine::new(20.0, 0.0, 0.0, 10.0, 30.0, 100.0), 1e-9)
    );
}
