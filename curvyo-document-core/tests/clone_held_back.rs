//! The held-back rule of own placement (`specs/0057-clone-overrides` criteria 3 to 8 and 35, ADR
//! 0016 §2) through every transform writer of the document core.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names,
    clippy::type_complexity
)]

use curvyo_document_core::{
    Affine, AnchorId, Angle, CloneHold, CloneSnapshot, Document, NewAnchor, NodeId, ObjectSnapshot,
    PathSnapshot, Point, Vec2,
};

fn square(d: &Document) -> NodeId {
    let corner = |k: u64, x: f64, y: f64| NewAnchor::corner(AnchorId::new(1, k), Point::new(x, y));
    d.create_path(
        &[
            corner(0, 0.0, 0.0),
            corner(1, 10.0, 0.0),
            corner(2, 10.0, 10.0),
            corner(3, 0.0, 10.0),
        ],
        true,
    )
}

fn clone_of(d: &Document, id: NodeId) -> CloneSnapshot {
    match d.object(id) {
        Some(ObjectSnapshot::Clone(clone)) => clone,
        other => panic!("not a clone: {other:?}"),
    }
}

fn path_of(d: &Document, id: NodeId) -> PathSnapshot {
    match d.object(id) {
        Some(ObjectSnapshot::Path(path)) => path,
        other => panic!("not a path: {other:?}"),
    }
}

fn drawn(d: &Document, id: NodeId) -> PathSnapshot {
    match d.object(id).unwrap() {
        ObjectSnapshot::Path(path) => path,
        ObjectSnapshot::Clone(clone) => clone.resolved,
        ObjectSnapshot::Primitive(_) => panic!("a primitive"),
    }
}

fn xs(path: &PathSnapshot) -> (f64, f64) {
    let xs: Vec<f64> = path.all_anchors().map(|a| a.point.x).collect();
    (
        xs.iter().copied().fold(f64::INFINITY, f64::min),
        xs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    )
}

fn same(a: &PathSnapshot, b: &PathSnapshot, tolerance: f64) {
    for (x, y) in a.all_anchors().zip(b.all_anchors()) {
        assert!((x.point.x - y.point.x).abs() < tolerance, "{x:?} {y:?}");
        assert!((x.point.y - y.point.y).abs() < tolerance, "{x:?} {y:?}");
    }
}

fn link(d: &Document, original: NodeId, x: f64, y: f64) -> NodeId {
    let clone = d.linked_clone(&[original], &[]).unwrap()[0];
    d.translate_objects(&[clone], Vec2::new(x, y)).unwrap();
    clone
}

/// O the square at 0..10, C own placement at translate (30, 0), F following at translate (60, 0).
fn setup() -> (Document, NodeId, NodeId, NodeId) {
    let d = Document::new(1);
    let o = square(&d);
    let c = link(&d, o, 30.0, 0.0);
    let f = link(&d, o, 60.0, 0.0);
    d.set_own_placement(&[c], true).unwrap();
    (d, o, c, f)
}

/// Every transform writer of the core, applied to O by the map `g` (ADR 0016 Consequences: "a
/// test lists the writers").
fn every_writer() -> Vec<(&'static str, Box<dyn Fn(&Document, NodeId)>)> {
    vec![
        (
            "translate_objects",
            Box::new(|d: &Document, o| d.translate_objects(&[o], Vec2::new(5.0, 0.0)).unwrap()),
        ),
        (
            "rotate_object",
            Box::new(|d: &Document, o| {
                let turned = d
                    .object(o)
                    .unwrap()
                    .rotated(Point::new(5.0, 5.0), Angle::from_radians(0.7));
                d.rotate_object(&turned).unwrap();
            }),
        ),
        (
            "resize_path",
            Box::new(|d: &Document, o| {
                let g = Affine::new(2.0, 0.0, 0.5, 1.5, 3.0, -2.0);
                let anchors: Vec<_> = path_of(d, o)
                    .anchors
                    .iter()
                    .map(|a| {
                        (
                            a.id,
                            g.apply_point(a.point),
                            g.apply_vector(a.handle_in),
                            g.apply_vector(a.handle_out),
                        )
                    })
                    .collect();
                d.resize_path(o, &anchors, None).unwrap();
            }),
        ),
        (
            "transform_objects (flip)",
            Box::new(|d: &Document, o| {
                let flipped = path_of(d, o).mapped(Affine::new(-1.0, 0.0, 0.0, 1.0, 10.0, 0.0));
                d.transform_objects(&[ObjectSnapshot::Path(flipped)], false)
                    .unwrap();
            }),
        ),
    ]
}

#[test]
fn ac3_35_every_transform_writer_holds_back_an_own_placement_clone() {
    for (name, write) in every_writer() {
        let (d, o, c, f) = setup();
        let (c_before, f_before) = (drawn(&d, c), drawn(&d, f));
        let f_matrix = clone_of(&d, f).chain.matrix;
        write(&d, o);
        same(&drawn(&d, c), &c_before, 1e-6);
        assert_eq!(
            clone_of(&d, f).chain.matrix,
            f_matrix,
            "{name}: follower not written"
        );
        assert_ne!(drawn(&d, f), f_before, "{name}: the follower moves");
        assert_eq!(
            d.take_clone_hold(),
            CloneHold {
                held: 1,
                unlinked: 0
            },
            "{name}"
        );
        assert!(clone_of(&d, c).chain.own_placement, "{name}");
    }
}

#[test]
fn ac3_the_move_example_of_the_spec() {
    let (d, o, c, f) = setup();
    d.translate_objects(&[o], Vec2::new(5.0, 0.0)).unwrap();
    assert_eq!(xs(&drawn(&d, o)), (5.0, 15.0));
    assert_eq!(xs(&drawn(&d, c)), (30.0, 40.0));
    assert_eq!(xs(&drawn(&d, f)), (65.0, 75.0));
    let m = clone_of(&d, c).chain.matrix;
    assert_eq!(m, Affine::translation(Vec2::new(25.0, 0.0)));
    assert_eq!(d.own_steps().last().unwrap().label, "translate_objects");
}

#[test]
fn ac4_a_node_edit_reaches_an_own_placement_clone() {
    let (d, o, c, _) = setup();
    let top_right = path_of(&d, o).anchors[1].id;
    d.move_anchors(&[(o, top_right, Point::new(15.0, 5.0))])
        .unwrap();
    assert_eq!(
        clone_of(&d, c).resolved.anchors[1].point,
        Point::new(45.0, 5.0)
    );
    assert_eq!(
        d.take_clone_hold(),
        CloneHold::default(),
        "no notice for a node edit"
    );
}

#[test]
fn ac4_dragging_every_node_is_an_edit_not_a_transform() {
    let (d, o, c, _) = setup();
    let moves: Vec<_> = path_of(&d, o)
        .anchors
        .iter()
        .map(|a| (o, a.id, Point::new(a.point.x + 5.0, a.point.y)))
        .collect();
    d.move_anchors(&moves).unwrap();
    assert_eq!(xs(&drawn(&d, c)), (35.0, 45.0));
}

#[test]
fn ac5b_a_held_clone_behind_a_following_one() {
    let d = Document::new(1);
    let o = square(&d);
    let a = link(&d, o, 30.0, 0.0);
    let b = link(&d, a, 0.0, 30.0);
    d.set_own_placement(&[b], true).unwrap();
    let b_before = drawn(&d, b);
    d.translate_objects(&[o], Vec2::new(5.0, 0.0)).unwrap();
    assert_eq!(xs(&drawn(&d, a)), (35.0, 45.0));
    same(&drawn(&d, b), &b_before, 1e-9);
    assert_eq!(
        clone_of(&d, b).chain.matrix,
        Affine::translation(Vec2::new(-5.0, 30.0))
    );
}

#[test]
fn ac5a_5c_a_held_middle_clone_and_a_moved_one() {
    let d = Document::new(1);
    let o = square(&d);
    let a = link(&d, o, 30.0, 0.0);
    let b = link(&d, a, 0.0, 30.0);
    d.set_own_placement(&[a], true).unwrap();
    let (a_before, b_before) = (drawn(&d, a), drawn(&d, b));
    d.translate_objects(&[o], Vec2::new(5.0, 0.0)).unwrap();
    same(&drawn(&d, a), &a_before, 1e-9);
    same(&drawn(&d, b), &b_before, 1e-9);
    // (c): B held, A moved on its own.
    d.set_own_placement(&[a], false).unwrap();
    d.set_own_placement(&[b], true).unwrap();
    let b_before = drawn(&d, b);
    d.translate_objects(&[a], Vec2::new(0.0, 7.0)).unwrap();
    same(&drawn(&d, b), &b_before, 1e-9);
}

#[test]
fn ac6_selected_together_the_state_makes_no_difference() {
    let (d, o, c, _) = setup();
    d.translate_objects(&[o, c], Vec2::new(5.0, 0.0)).unwrap();
    assert_eq!(xs(&drawn(&d, c)), (35.0, 45.0));
    assert_eq!(d.take_clone_hold(), CloneHold::default());
    d.translate_objects(&[c], Vec2::new(5.0, 0.0)).unwrap();
    assert_eq!(xs(&drawn(&d, c)), (40.0, 50.0), "moved on its own: G·M");
}

#[test]
fn ac7_a_collapse_unlinks_held_clones_with_their_outline_and_says_so() {
    let d = Document::new(1);
    let o = square(&d);
    let (c1, c2) = (link(&d, o, 30.0, 0.0), link(&d, o, 60.0, 0.0));
    d.set_own_placement(&[c1, c2], true).unwrap();
    let (b1, b2) = (drawn(&d, c1), drawn(&d, c2));
    let flat: Vec<_> = path_of(&d, o)
        .anchors
        .iter()
        .map(|a| (a.id, Point::new(0.0, a.point.y), a.handle_in, a.handle_out))
        .collect();
    d.resize_path(o, &flat, None).unwrap();
    same(&path_of(&d, c1), &b1, 1e-9);
    same(&path_of(&d, c2), &b2, 1e-9);
    assert_eq!(
        d.take_clone_hold(),
        CloneHold {
            held: 0,
            unlinked: 2
        }
    );
    assert_eq!(
        d.own_steps().last().unwrap().label,
        "resize_path",
        "same commit"
    );
    let json: serde_json::Value = serde_json::from_slice(&d.export_json().unwrap()).unwrap();
    assert!(
        json["objects"][1].get("own_placement").is_none(),
        "flags gone"
    );
}

#[test]
fn ac7_a_matrix_beyond_the_limit_refuses_everything() {
    let d = Document::new(1);
    let o = square(&d);
    let c = link(&d, o, 30.0, 0.0);
    d.set_own_placement(&[c], true).unwrap();
    let tiny: Vec<_> = path_of(&d, o)
        .anchors
        .iter()
        .map(|a| {
            (
                a.id,
                Point::new(a.point.x * 1e-8, a.point.y),
                a.handle_in,
                a.handle_out,
            )
        })
        .collect();
    let before = d.export_json().unwrap();
    assert_eq!(
        d.resize_path(o, &tiny, None),
        Err(curvyo_document_core::PathEditError::CloneOutOfRange(c))
    );
    assert_eq!(d.export_json().unwrap(), before, "nothing written");
}

#[test]
fn ac8_the_count_is_one_per_held_clone_and_zero_without() {
    let d = Document::new(1);
    let o = square(&d);
    let cs: Vec<NodeId> = (1..=3)
        .map(|k| link(&d, o, 30.0 * f64::from(k), 0.0))
        .collect();
    d.translate_objects(&[o], Vec2::new(1.0, 0.0)).unwrap();
    assert_eq!(d.take_clone_hold(), CloneHold::default());
    d.set_own_placement(&cs, true).unwrap();
    d.translate_objects(&[o], Vec2::new(1.0, 0.0)).unwrap();
    assert_eq!(d.take_clone_hold().held, 3);
    assert_eq!(d.take_clone_hold(), CloneHold::default(), "taken once");
}

#[test]
fn the_document_fit_moves_every_clone_whatever_its_flag() {
    let (d, o, c, _) = setup();
    let (o_before, c_before) = (drawn(&d, o), drawn(&d, c));
    d.fit_to_content((Point::new(-10.0, -10.0), Point::new(80.0, 20.0)))
        .unwrap();
    let o_shift = drawn(&d, o).anchors[0].point.x - o_before.anchors[0].point.x;
    let c_shift = drawn(&d, c).anchors[0].point.x - c_before.anchors[0].point.x;
    assert!(o_shift.abs() > 1.0, "the fit moved the content");
    assert!(
        (o_shift - c_shift).abs() < 1e-9,
        "the clone moves with everything"
    );
}

#[test]
fn undo_of_a_transform_restores_the_held_matrix() {
    let (d, o, c, _) = setup();
    let m = clone_of(&d, c).chain.matrix;
    d.translate_objects(&[o], Vec2::new(5.0, 0.0)).unwrap();
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
    assert_eq!(clone_of(&d, c).chain.matrix, m);
}

/// Review F1: a 100 mm by 1e-4 mm root, as drawn or turned by 45°, sheared through
/// `transform_objects`, holds its clone back within the limit.
#[test]
fn a_thin_sheared_root_holds_its_clone_within_the_limit() {
    for turn in [0.0, std::f64::consts::FRAC_PI_4] {
        thin_sheared_root(turn);
    }
}

fn thin_sheared_root(turn: f64) {
    let d = Document::new(1);
    let corner = |k: u64, x: f64, y: f64| NewAnchor::corner(AnchorId::new(1, k), Point::new(x, y));
    let o = d.create_path(
        &[
            corner(0, 0.0, 0.0),
            corner(1, 100.0, 0.0),
            corner(2, 100.0, 1e-4),
            corner(3, 0.0, 1e-4),
        ],
        true,
    );
    let turned = path_of(&d, o).mapped(Affine::rotation_about(
        Point::new(0.0, 0.0),
        Angle::from_radians(turn),
    ));
    d.transform_objects(&[ObjectSnapshot::Path(turned)], false)
        .unwrap();
    let c = link(&d, o, 0.0, 30.0);
    d.set_own_placement(&[c], true).unwrap();
    let before = drawn(&d, c);
    let sheared = path_of(&d, o).mapped(Affine::new(1.0, 0.0, 0.7, 1.0, 2.0, 3.0));
    d.transform_objects(&[ObjectSnapshot::Path(sheared)], false)
        .unwrap();
    same(&drawn(&d, c), &before, 1e-6);
    assert!(clone_of(&d, c).chain.matrix.is_within_limit());
    assert_eq!(d.take_clone_hold().held, 1);
}
