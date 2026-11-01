//! The two overrides of a linked clone (`specs/0057-clone-overrides`, ADR 0016): the switches
//! (criteria 2, 11, 14), the style source of a chain (criterion 15) and the effective opacity
//! (criterion 16), on the document model.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names,
    clippy::items_after_statements
)]

use curvyo_document_core::{
    AnchorId, CloneError, CloneSnapshot, Color, Document, FOLLOW_PLACEMENT_COMMIT_LABEL, MovePlan,
    MoveTarget, NewAnchor, NodeId, NodeRead, ORIGINAL_STYLE_COMMIT_LABEL,
    OWN_PLACEMENT_COMMIT_LABEL, OWN_STYLE_COMMIT_LABEL, ObjectSnapshot, Opacity, PathSnapshot,
    Point, Style, StyleEdit, Vec2,
};

const RED: Color = Color { r: 255, g: 0, b: 0 };
const GREEN: Color = Color { r: 0, g: 255, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };

fn square(d: &Document, n: u64, at: Point) -> NodeId {
    let corner = |k: u64, dx: f64, dy: f64| {
        NewAnchor::corner(
            AnchorId::new(1, n * 10 + k),
            Point::new(at.x + dx, at.y + dy),
        )
    };
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

fn style_of(d: &Document, id: NodeId) -> Style {
    match d.object(id).unwrap() {
        ObjectSnapshot::Image(_) => unreachable!("no image in this test"),
        ObjectSnapshot::Dimension(_) => unreachable!("no dimension in this test"),
        ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
        ObjectSnapshot::Sketch(_) => unreachable!("no sketch in this test"),
        ObjectSnapshot::Path(path) => path.style,
        ObjectSnapshot::Clone(clone) => clone.resolved.style,
        ObjectSnapshot::Primitive(p) => p.style,
    }
}

fn drawn(d: &Document, id: NodeId) -> PathSnapshot {
    clone_of(d, id).resolved
}

fn link(d: &Document, original: NodeId, x: f64, y: f64) -> NodeId {
    let clone = d.linked_clone(&[original], &[]).unwrap()[0];
    d.translate_objects(&[clone], Vec2::new(x, y)).unwrap();
    clone
}

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

fn undo_last(d: &Document) {
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
}

fn node(d: &Document, id: NodeId) -> NodeRead {
    d.read_nodes().into_iter().find(|n| n.id == id).unwrap()
}

fn opacity(value: f64) -> Opacity {
    Opacity::new(value).unwrap()
}

/// A root with a red 0.5 mm stroke, a blue fill and opacity 80 % (criterion 11).
fn styled_root(d: &Document) -> NodeId {
    let p = square(d, 1, Point::new(0.0, 0.0));
    for edit in [
        StyleEdit::StrokeColor(RED),
        StyleEdit::StrokeWidth(curvyo_document_core::Length::from_mm(0.5)),
        StyleEdit::FillEnabled(true),
        StyleEdit::FillColor(BLUE),
    ] {
        d.edit_style(&[p], &edit).unwrap();
    }
    d.set_node_opacity(&[p], opacity(0.8)).unwrap();
    p
}

fn into_layer(d: &Document, nodes: &[NodeId], layer_opacity: f64) -> NodeId {
    let layer = d.new_layer(None);
    d.move_nodes(&MovePlan {
        nodes: nodes.to_vec(),
        target: MoveTarget::FrontOf(Some(layer)),
    })
    .unwrap();
    d.set_node_opacity(&[layer], opacity(layer_opacity))
        .unwrap();
    layer
}

#[test]
fn ac1_10_every_new_clone_starts_following_with_the_originals_style() {
    let d = Document::new(1);
    let p = square(&d, 1, Point::new(0.0, 0.0));
    let c = link(&d, p, 30.0, 0.0);
    let chain = clone_of(&d, c).chain;
    assert!(!chain.own_placement);
    assert!(!chain.own_style);
    assert_eq!(chain.style_source, p);
}

#[test]
fn ac2_switching_placement_moves_nothing_and_is_one_commit_each_way() {
    let d = Document::new(1);
    let p = square(&d, 1, Point::new(0.0, 0.0));
    let c = link(&d, p, 30.0, 0.0);
    let before = clone_of(&d, c);
    assert_eq!(d.set_own_placement(&[c], true).unwrap(), 1);
    assert_eq!(last_label(&d), OWN_PLACEMENT_COMMIT_LABEL);
    let own = clone_of(&d, c);
    assert!(own.chain.own_placement);
    assert_eq!(own.resolved, before.resolved, "every point unchanged");
    assert_eq!(own.chain.matrix, before.chain.matrix);
    let steps = d.own_steps().len();
    assert_eq!(d.set_own_placement(&[c], true).unwrap(), 0);
    assert_eq!(d.own_steps().len(), steps, "the same state commits nothing");
    assert_eq!(d.set_own_placement(&[c], false).unwrap(), 1);
    assert_eq!(last_label(&d), FOLLOW_PLACEMENT_COMMIT_LABEL);
    assert_eq!(clone_of(&d, c), before);
}

#[test]
fn ac2_a_switch_sets_every_named_clone_in_one_commit() {
    let d = Document::new(1);
    let p = square(&d, 1, Point::new(0.0, 0.0));
    let (a, b) = (link(&d, p, 30.0, 0.0), link(&d, p, 60.0, 0.0));
    d.set_own_placement(&[a], true).unwrap();
    let steps = d.own_steps().len();
    assert_eq!(
        d.set_own_placement(&[a, b], true).unwrap(),
        1,
        "only b changes"
    );
    assert_eq!(d.own_steps().len(), steps + 1);
    assert!(clone_of(&d, a).chain.own_placement && clone_of(&d, b).chain.own_placement);
}

#[test]
fn ac2_11_a_switch_refuses_what_is_not_a_clone_and_writes_nothing() {
    let d = Document::new(1);
    let p = square(&d, 1, Point::new(0.0, 0.0));
    let c = link(&d, p, 30.0, 0.0);
    let gone = square(&d, 2, Point::new(50.0, 0.0));
    d.delete_objects(&[gone]).unwrap();
    let steps = d.own_steps().len();
    assert_eq!(
        d.set_own_placement(&[c, p], true),
        Err(CloneError::NotAClone)
    );
    assert_eq!(
        d.set_own_style(&[c, gone], true),
        Err(CloneError::NoSuchObject)
    );
    assert_eq!(d.own_steps().len(), steps);
    assert!(!clone_of(&d, c).chain.own_placement);
    assert!(!clone_of(&d, c).chain.own_style);
}

#[test]
fn ac11_own_style_copies_the_effective_style_and_opacity_and_draws_the_same() {
    let d = Document::new(1);
    let p = styled_root(&d);
    let c = link(&d, p, 30.0, 0.0);
    let (before, before_opacity) = (drawn(&d, c), node(&d, c).opacity);
    assert_eq!(d.set_own_style(&[c], true).unwrap(), 1);
    assert_eq!(last_label(&d), OWN_STYLE_COMMIT_LABEL);
    let after = clone_of(&d, c);
    assert!(after.chain.own_style);
    assert_eq!(after.chain.style_source, c);
    assert_eq!(after.resolved, before, "outline and style unchanged");
    assert_eq!(after.resolved.style.stroke.color, RED);
    assert_eq!(after.resolved.style.stroke.width.as_mm(), 0.5);
    assert_eq!(node(&d, c).registers.opacity, opacity(0.8));
    assert_eq!(node(&d, c).opacity, before_opacity);
}

#[test]
fn ac12_a_later_change_of_the_original_does_not_reach_an_own_style() {
    let d = Document::new(1);
    let p = styled_root(&d);
    let own = link(&d, p, 30.0, 0.0);
    let follower = link(&d, p, 60.0, 0.0);
    d.set_own_style(&[own], true).unwrap();
    d.edit_style(&[p], &StyleEdit::StrokeColor(GREEN)).unwrap();
    d.set_node_opacity(&[p], opacity(0.5)).unwrap();
    assert_eq!(style_of(&d, own).stroke.color, RED);
    assert_eq!(node(&d, own).opacity, opacity(0.8));
    assert_eq!(style_of(&d, follower).stroke.color, GREEN);
    assert_eq!(node(&d, follower).opacity, opacity(0.5));
}

#[test]
fn ac14_back_to_the_originals_style_removes_the_own_one_and_undo_brings_it_back() {
    let d = Document::new(1);
    let p = styled_root(&d);
    let c = link(&d, p, 30.0, 0.0);
    d.set_own_style(&[c], true).unwrap();
    // The root turns green; the clone keeps its own red.
    d.edit_style(&[p], &StyleEdit::StrokeColor(GREEN)).unwrap();
    let own = clone_of(&d, c);
    assert_eq!(d.set_own_style(&[c], false).unwrap(), 1);
    assert_eq!(last_label(&d), ORIGINAL_STYLE_COMMIT_LABEL);
    let back = clone_of(&d, c);
    assert!(!back.chain.own_style);
    assert_eq!(back.resolved.style, style_of(&d, p));
    assert_eq!(node(&d, c).registers.opacity, Opacity::OPAQUE, "removed");
    let json: serde_json::Value = serde_json::from_slice(&d.export_json().unwrap()).unwrap();
    let entry = &json["objects"][1];
    assert_eq!(entry["shape"], "clone");
    let keys: Vec<_> = entry.as_object().unwrap().keys().collect();
    assert_eq!(
        keys,
        ["id", "matrix", "origin", "shape"],
        "nothing kept behind"
    );
    undo_last(&d);
    assert_eq!(clone_of(&d, c), own, "every value back");
    assert_eq!(node(&d, c).registers.opacity, opacity(0.8));
}

#[test]
fn ac15_a_chain_reads_the_nearest_own_style_and_falls_back_to_the_root() {
    let d = Document::new(1);
    let o = styled_root(&d);
    let a = link(&d, o, 30.0, 0.0);
    let b = link(&d, a, 0.0, 30.0);
    d.set_own_style(&[a], true).unwrap();
    d.edit_style(&[o], &StyleEdit::FillColor(GREEN)).unwrap();
    assert_eq!(style_of(&d, a).fill.color, BLUE);
    assert_eq!(style_of(&d, b).fill.color, BLUE);
    assert_eq!(clone_of(&d, b).chain.style_source, a);
    d.set_own_style(&[a], false).unwrap();
    assert_eq!(style_of(&d, a).fill.color, GREEN);
    assert_eq!(style_of(&d, b).fill.color, GREEN);
}

#[test]
fn ac11_switching_a_chain_together_copies_what_each_showed_before_any_write() {
    let d = Document::new(1);
    let o = styled_root(&d);
    let a = link(&d, o, 30.0, 0.0);
    let b = link(&d, a, 0.0, 30.0);
    let (a_before, b_before) = (drawn(&d, a), drawn(&d, b));
    assert_eq!(d.set_own_style(&[b, a], true).unwrap(), 2);
    assert_eq!(drawn(&d, a), a_before);
    assert_eq!(drawn(&d, b), b_before);
    assert_eq!(clone_of(&d, b).chain.style_source, b);
}

#[test]
fn ac16_effective_opacity_of_own_and_originals_clones() {
    // Criterion 16: root P at 50 %, clone C Original's in a layer at 50 %: 25 %.
    let d = Document::new(1);
    let p = square(&d, 1, Point::new(0.0, 0.0));
    d.set_node_opacity(&[p], opacity(0.5)).unwrap();
    let c = link(&d, p, 30.0, 0.0);
    into_layer(&d, &[c], 0.5);
    assert_eq!(node(&d, c).opacity, opacity(0.25));
    d.set_own_style(&[c], true).unwrap();
    assert_eq!(node(&d, c).registers.opacity, opacity(0.5), "copied");
    assert_eq!(node(&d, c).opacity, opacity(0.25), "shown the same");
    d.set_own_style(&[c], false).unwrap();
    d.set_own_style(&[c], true).unwrap();
    d.set_node_opacity(&[p], opacity(0.1)).unwrap();
    assert_eq!(
        node(&d, c).opacity,
        opacity(0.25),
        "the root's change does not reach it"
    );
}

#[test]
fn ac16_the_opacity_field_writes_own_style_clones_only() {
    let d = Document::new(1);
    let p = square(&d, 1, Point::new(0.0, 0.0));
    let (own, follower) = (link(&d, p, 30.0, 0.0), link(&d, p, 60.0, 0.0));
    d.set_own_style(&[own], true).unwrap();
    d.set_node_opacity(&[own, follower], opacity(0.3)).unwrap();
    assert_eq!(node(&d, own).registers.opacity, opacity(0.3));
    assert_eq!(node(&d, follower).opacity, Opacity::OPAQUE, "left alone");
}

#[test]
fn ac16_a_chain_reads_the_register_of_its_source_not_its_layers() {
    // A Own at 40 % in a layer of 50 %, B Original's of A in a layer of 100 %: B at 40 %.
    let d = Document::new(1);
    let o = square(&d, 1, Point::new(0.0, 0.0));
    let a = link(&d, o, 30.0, 0.0);
    let b = link(&d, a, 0.0, 30.0);
    into_layer(&d, &[a], 0.5);
    into_layer(&d, &[b], 1.0);
    d.set_own_style(&[a], true).unwrap();
    d.set_node_opacity(&[a], opacity(0.4)).unwrap();
    assert_eq!(node(&d, a).opacity, opacity(0.2));
    assert!((node(&d, b).opacity.get() - 0.4).abs() < 1e-12);
}

fn path_of(d: &Document, id: NodeId) -> PathSnapshot {
    match d.object(id) {
        Some(ObjectSnapshot::Path(path)) => path,
        other => panic!("not a path: {other:?}"),
    }
}

fn unlink(d: &Document, id: NodeId, start: u64) {
    let count = drawn(d, id).all_anchors().count();
    d.unlink_clones(&[curvyo_document_core::CopySource {
        id,
        anchor_ids: (0..count as u64)
            .map(|k| AnchorId::new(9, start + k))
            .collect(),
    }])
    .unwrap();
}

fn keys_of(d: &Document, id: NodeId) -> Vec<String> {
    let json: serde_json::Value = serde_json::from_slice(&d.export_json().unwrap()).unwrap();
    fn find(nodes: &serde_json::Value, id: &serde_json::Value) -> Option<serde_json::Value> {
        nodes.as_array()?.iter().find_map(|n| {
            if &n["id"] == id {
                Some(n.clone())
            } else {
                find(&n["objects"], id)
            }
        })
    }
    let entry = find(&json["objects"], &serde_json::to_value(id).unwrap()).unwrap();
    entry.as_object().unwrap().keys().cloned().collect()
}

#[test]
fn ac12_style_edits_write_an_own_style_clone_only_and_markers_too() {
    let d = Document::new(1);
    let p = styled_root(&d);
    let (own, follower) = (link(&d, p, 30.0, 0.0), link(&d, p, 60.0, 0.0));
    d.set_own_style(&[own], true).unwrap();
    d.edit_style(&[own, follower], &StyleEdit::StrokeColor(GREEN))
        .unwrap();
    assert_eq!(style_of(&d, own).stroke.color, GREEN);
    assert_eq!(style_of(&d, follower).stroke.color, RED, "left alone");
    assert_eq!(style_of(&d, p).stroke.color, RED);
    d.edit_style(
        &[own],
        &StyleEdit::MarkerEnd(curvyo_document_core::MarkerShape::Arrow),
    )
    .unwrap();
    assert_eq!(
        style_of(&d, own).stroke.markers.end,
        curvyo_document_core::MarkerShape::Arrow
    );
    let steps = d.own_steps().len();
    d.edit_style(&[follower], &StyleEdit::StrokeColor(GREEN))
        .unwrap();
    assert_eq!(
        d.own_steps().len(),
        steps,
        "only Original's clones: nothing written"
    );
}

#[test]
fn ac13_twelve_clones_twelve_colours_to_the_last_bit() {
    let d = Document::new(1);
    let p = square(&d, 1, Point::new(0.0, 0.0));
    let clones: Vec<NodeId> = (1..=12)
        .map(|k| link(&d, p, 20.0 * f64::from(k), 0.0))
        .collect();
    d.set_own_style(&clones, true).unwrap();
    d.edit_style(&clones, &StyleEdit::FillEnabled(true))
        .unwrap();
    let colour = |k: u8| Color {
        r: k.wrapping_mul(21),
        g: 255 - k,
        b: k.wrapping_mul(7),
    };
    for (k, &c) in clones.iter().enumerate() {
        let k = u8::try_from(k).unwrap();
        d.edit_style(&[c], &StyleEdit::FillColor(colour(k)))
            .unwrap();
        d.edit_style(&[c], &StyleEdit::StrokeColor(colour(k + 100)))
            .unwrap();
    }
    let reopened =
        curvyo_document_core::unpack(2, &curvyo_document_core::pack(&d, "t").unwrap()).unwrap();
    for (k, &c) in clones.iter().enumerate() {
        let k = u8::try_from(k).unwrap();
        assert_eq!(style_of(&reopened, c).fill.color, colour(k));
        assert_eq!(style_of(&reopened, c).stroke.color, colour(k + 100));
    }
}

#[test]
fn ac30_unlink_keeps_the_effective_style_and_the_source_register_and_drops_the_states() {
    let d = Document::new(1);
    let o = styled_root(&d);
    let a = link(&d, o, 30.0, 0.0);
    let b = link(&d, a, 0.0, 30.0);
    d.set_own_style(&[a], true).unwrap();
    d.set_node_opacity(&[a], opacity(0.4)).unwrap();
    d.set_own_placement(&[b], true).unwrap();
    into_layer(&d, &[b], 0.5);
    let (before, shown) = (drawn(&d, b), node(&d, b).opacity);
    unlink(&d, b, 100);
    let path = path_of(&d, b);
    assert_eq!(path.style, before.style, "A's style");
    for (x, y) in path.all_anchors().zip(before.all_anchors()) {
        assert!((x.point.x - y.point.x).abs() < 1e-9 && (x.point.y - y.point.y).abs() < 1e-9);
    }
    assert_eq!(node(&d, b).registers.opacity, opacity(0.4), "the register");
    assert_eq!(node(&d, b).opacity, shown, "the picture does not change");
    let keys = keys_of(&d, b);
    assert!(!keys.contains(&"own_placement".to_owned()) && !keys.contains(&"own_style".to_owned()));
}

#[test]
fn ac31_deleting_a_style_source_copies_its_style_onto_the_clone_behind() {
    let d = Document::new(1);
    let o = styled_root(&d);
    let a = link(&d, o, 30.0, 0.0);
    let b = link(&d, a, 0.0, 30.0);
    let c = link(&d, b, 0.0, 30.0);
    d.set_own_style(&[a], true).unwrap();
    d.edit_style(&[a], &StyleEdit::FillColor(GREEN)).unwrap();
    d.set_node_opacity(&[a], opacity(0.3)).unwrap();
    d.set_own_placement(&[b], true).unwrap();
    let (b_before, c_before) = (drawn(&d, b), drawn(&d, c));
    d.delete_objects(&[a]).unwrap();
    let (b_now, c_now) = (clone_of(&d, b), clone_of(&d, c));
    assert_eq!(b_now.chain.origin, o, "re-pointed");
    assert!(
        b_now.chain.own_style && b_now.chain.own_placement,
        "kept and copied"
    );
    assert_eq!(b_now.resolved, b_before);
    assert_eq!(node(&d, b).registers.opacity, opacity(0.3));
    assert!(
        c_now.chain.own_style,
        "its source was A too: it gets the copy as well"
    );
    assert_eq!(c_now.resolved, c_before);
}

#[test]
fn ac31_a_clone_whose_source_stays_gets_no_copy() {
    let d = Document::new(1);
    let o = square(&d, 1, Point::new(0.0, 0.0));
    let a = link(&d, o, 30.0, 0.0);
    let b = link(&d, a, 0.0, 30.0);
    d.delete_objects(&[a]).unwrap();
    assert!(!clone_of(&d, b).chain.own_style);
}

#[test]
fn ac31c_deleting_the_root_unlinks_with_the_effective_style_and_opacity() {
    let d = Document::new(1);
    let o = styled_root(&d);
    let c = link(&d, o, 30.0, 0.0);
    d.set_own_placement(&[c], true).unwrap();
    let before = drawn(&d, c);
    d.delete_objects(&[o]).unwrap();
    assert_eq!(path_of(&d, c).style, before.style);
    assert_eq!(node(&d, c).registers.opacity, opacity(0.8));
    assert!(!keys_of(&d, c).contains(&"own_placement".to_owned()));
}

#[test]
fn ac32_a_copy_carries_both_states_and_an_independent_own_style() {
    let d = Document::new(1);
    let o = styled_root(&d);
    let c = link(&d, o, 30.0, 0.0);
    d.set_own_placement(&[c], true).unwrap();
    d.set_own_style(&[c], true).unwrap();
    let copy = d
        .duplicate_objects(
            &[curvyo_document_core::CopySource {
                id: c,
                anchor_ids: Vec::new(),
            }],
            Vec2::new(5.0, 5.0),
        )
        .unwrap()[0];
    let chain = clone_of(&d, copy).chain;
    assert!(chain.own_placement && chain.own_style);
    assert_eq!(node(&d, copy).registers.opacity, opacity(0.8));
    d.edit_style(&[c], &StyleEdit::StrokeColor(GREEN)).unwrap();
    assert_eq!(style_of(&d, copy).stroke.color, RED, "independent");
}

#[test]
fn ac33_object_undo_brings_the_states_back_field_by_field() {
    let d = Document::new(1);
    let o = styled_root(&d);
    let c = link(&d, o, 30.0, 0.0);
    d.set_own_style(&[c], true).unwrap();
    d.set_own_placement(&[c], true).unwrap();
    d.undo_object_step(c).unwrap();
    assert!(!clone_of(&d, c).chain.own_placement);
    assert!(clone_of(&d, c).chain.own_style);
    d.undo_object_step(c).unwrap();
    assert!(!clone_of(&d, c).chain.own_style);
    d.redo_object_step(c).unwrap();
    assert!(clone_of(&d, c).chain.own_style);
    assert_eq!(style_of(&d, c).stroke.color, RED);
}

fn peer(d: &Document, id: u64) -> Document {
    curvyo_document_core::unpack(id, &curvyo_document_core::pack(d, "t").unwrap()).unwrap()
}

fn sync(a: &Document, b: &Document) {
    let (va, vb) = (a.version(), b.version());
    let to_b = a.export_updates(&vb).unwrap();
    let to_a = b.export_updates(&va).unwrap();
    b.import_updates(&to_b).unwrap();
    a.import_updates(&to_a).unwrap();
}

#[test]
fn ac34b_own_style_against_a_concurrent_edit_of_the_original() {
    let a = Document::new(1);
    let o = styled_root(&a);
    let c = link(&a, o, 30.0, 0.0);
    let b = peer(&a, 2);
    a.set_own_style(&[c], true).unwrap();
    b.edit_style(&[o], &StyleEdit::StrokeColor(GREEN)).unwrap();
    sync(&a, &b);
    assert_eq!(
        style_of(&a, c).stroke.color,
        RED,
        "the copy taken at the switch"
    );
    assert_eq!(a.read_objects(), b.read_objects(), "replicas converge");
}

#[test]
fn ac34a_two_peers_edit_fill_and_stroke_of_one_own_style() {
    let a = Document::new(1);
    let o = styled_root(&a);
    let c = link(&a, o, 30.0, 0.0);
    a.set_own_style(&[c], true).unwrap();
    let b = peer(&a, 2);
    a.edit_style(&[c], &StyleEdit::FillColor(GREEN)).unwrap();
    b.edit_style(&[c], &StyleEdit::StrokeColor(BLUE)).unwrap();
    sync(&a, &b);
    assert_eq!(style_of(&a, c).fill.color, GREEN);
    assert_eq!(style_of(&a, c).stroke.color, BLUE);
    assert_eq!(a.read_objects(), b.read_objects());
}

#[test]
fn ac34_back_to_originals_against_a_concurrent_key_edit_reads_the_original() {
    let a = Document::new(1);
    let o = styled_root(&a);
    let c = link(&a, o, 30.0, 0.0);
    a.set_own_style(&[c], true).unwrap();
    let b = peer(&a, 2);
    a.set_own_style(&[c], false).unwrap();
    b.edit_style(&[c], &StyleEdit::FillColor(GREEN)).unwrap();
    b.set_node_opacity(&[c], opacity(0.2)).unwrap();
    sync(&a, &b);
    assert!(!clone_of(&a, c).chain.own_style);
    assert_eq!(style_of(&a, c), style_of(&a, o), "stray values ignored");
    assert_eq!(node(&a, c).opacity, node(&a, o).opacity);
    assert_eq!(a.read_nodes(), b.read_nodes());
}

#[test]
fn ac34c_own_placement_against_a_concurrent_transform_loses_nothing() {
    let a = Document::new(1);
    let o = square(&a, 1, Point::new(0.0, 0.0));
    let c = link(&a, o, 30.0, 0.0);
    let b = peer(&a, 2);
    a.set_own_placement(&[c], true).unwrap();
    b.translate_objects(&[o], Vec2::new(5.0, 0.0)).unwrap();
    sync(&a, &b);
    assert!(clone_of(&a, c).chain.own_placement);
    assert_eq!(a.read_objects().orphans, Vec::<NodeId>::new());
    assert_eq!(a.read_objects(), b.read_objects());
}

#[test]
fn ac34_unlink_against_an_override_edit_leaves_a_path() {
    let a = Document::new(1);
    let o = styled_root(&a);
    let c = link(&a, o, 30.0, 0.0);
    a.set_own_style(&[c], true).unwrap();
    let b = peer(&a, 2);
    unlink(&a, c, 500);
    b.set_own_placement(&[c], true).unwrap();
    b.edit_style(&[c], &StyleEdit::FillColor(GREEN)).unwrap();
    sync(&a, &b);
    assert!(matches!(a.object(c), Some(ObjectSnapshot::Path(_))));
    assert_eq!(a.read_objects(), b.read_objects());
}

#[test]
fn ac40_the_read_of_a_clone_with_overrides_equals_its_unlinked_path() {
    let d = Document::new(1);
    let o = styled_root(&d);
    let c = link(&d, o, 30.0, 0.0);
    d.set_own_style(&[c], true).unwrap();
    d.edit_style(&[c], &StyleEdit::StrokeColor(GREEN)).unwrap();
    d.set_own_placement(&[c], true).unwrap();
    let read = drawn(&d, c);
    unlink(&d, c, 700);
    let path = path_of(&d, c);
    assert_eq!(path.style, read.style);
    assert_eq!(path.closed, read.closed);
    for (x, y) in path.all_anchors().zip(read.all_anchors()) {
        assert!((x.point.x - y.point.x).abs() < 1e-9 && (x.point.y - y.point.y).abs() < 1e-9);
        assert_eq!(
            (x.handle_in, x.handle_out, x.kind),
            (y.handle_in, y.handle_out, y.kind)
        );
    }
}
