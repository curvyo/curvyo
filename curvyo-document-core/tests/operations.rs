//! The operation commands of `specs/0101-laser-operations` (ADR 0046 §1 to §3): each writes one
//! history step under its label, memberships are `member.<path>` keys (a clone member by path),
//! and deleting an operation, a target or a placement takes its keys with it in the same step.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::assert_is_empty,
    clippy::many_single_char_names
)]

use curvyo_document_core::{
    Affine, AnchorId, Document, DocumentSize, Length, MovePlan, MoveTarget, NewAnchor,
    NewOperation, NewOutputPage, NodeId, OperationError, OperationSnapshot, OperationSwitch, Page,
    Point, RectBounds, StepRef, Vec2,
};

fn laser(d: &Document) -> (Page, NodeId) {
    let page = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let mfg = page.kind.manufacturing().unwrap();
    (page, mfg)
}

fn op(kind: &'static str, name: &'static str) -> NewOperation<'static> {
    NewOperation { kind, name }
}

fn last_step(d: &Document) -> StepRef {
    d.own_steps().last().unwrap().step
}

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

fn rect(d: &Document) -> NodeId {
    d.create_rect(RectBounds::from_corners(
        Point::new(0.0, 0.0),
        Point::new(20.0, 10.0),
    ))
}

fn ops(d: &Document, mfg: NodeId) -> Vec<OperationSnapshot> {
    d.operations(mfg).unwrap()
}

#[test]
fn a_new_laser_page_has_no_operation_and_an_added_one_is_on_and_shown() {
    let d = Document::new(1);
    let (_, mfg) = laser(&d);
    assert!(ops(&d, mfg).is_empty(), "criterion 1: an empty list");
    let id = d.add_operation(mfg, &op("cut", "Cut 1"), 0, &[]).unwrap();
    assert_eq!(last_label(&d), "add_operation");
    let list = ops(&d, mfg);
    assert_eq!(list.len(), 1);
    let cut = &list[0];
    assert_eq!(cut.id, id);
    assert_eq!(cut.name, "Cut 1");
    assert_eq!(cut.kind.as_deref(), Some("cut"));
    assert!(cut.show && cut.output);
    assert_eq!(cut.margin.as_mm(), 0.0);
    assert!(cut.members.is_empty());
    assert!(
        d.is_in_manufacturing(id),
        "operations are skipped by every drawing walk"
    );
    assert!(d.read_objects().objects.iter().all(|o| o.id() != id));
}

#[test]
fn an_operation_lands_at_its_index_and_moves_by_one_step() {
    let d = Document::new(1);
    let (_, mfg) = laser(&d);
    let a = d
        .add_operation(mfg, &op("engrave", "Engrave 1"), 0, &[])
        .unwrap();
    let c = d.add_operation(mfg, &op("cut", "Cut 1"), 9, &[]).unwrap();
    let b = d
        .add_operation(mfg, &op("score", "Score 1"), 1, &[])
        .unwrap();
    let order = |d: &Document| ops(d, mfg).iter().map(|o| o.id).collect::<Vec<_>>();
    assert_eq!(order(&d), vec![a, b, c]);
    assert!(d.move_operation(c, 0).unwrap());
    assert_eq!(last_label(&d), "move_operation");
    assert_eq!(order(&d), vec![c, a, b]);
    assert!(
        !d.move_operation(c, 0).unwrap(),
        "no step when it already sits there"
    );
    assert!(d.move_operation(c, 99).unwrap());
    assert_eq!(order(&d), vec![a, b, c]);
    d.undo_step(last_step(&d)).unwrap();
    assert_eq!(order(&d), vec![c, a, b]);
}

#[test]
fn rename_switches_and_margin_are_one_step_each() {
    let d = Document::new(1);
    let (_, mfg) = laser(&d);
    let id = d
        .add_operation(mfg, &op("cut_out", "Cut-out 1"), 0, &[])
        .unwrap();
    assert!(d.rename_operation(id, "Outline").unwrap());
    assert_eq!(last_label(&d), "rename_operation");
    assert!(
        !d.rename_operation(id, "   ").unwrap(),
        "an empty name reverts"
    );
    assert!(!d.rename_operation(id, "Outline").unwrap());
    assert!(
        d.set_operation_switch(id, OperationSwitch::Show, false)
            .unwrap()
    );
    assert_eq!(last_label(&d), "set_operation_show");
    assert!(
        d.set_operation_switch(id, OperationSwitch::Output, false)
            .unwrap()
    );
    assert_eq!(last_label(&d), "set_operation_output");
    assert!(
        !d.set_operation_switch(id, OperationSwitch::Output, false)
            .unwrap()
    );
    assert!(d.set_operation_margin(id, Length::from_mm(1.5)).unwrap());
    assert_eq!(last_label(&d), "set_operation_margin");
    assert_eq!(
        d.set_operation_margin(id, Length::from_mm(-1.0)),
        Err(OperationError::BadMargin)
    );
    assert_eq!(
        d.set_operation_margin(id, Length::from_mm(f64::NAN)),
        Err(OperationError::BadMargin)
    );
    let read = &ops(&d, mfg)[0];
    assert_eq!(read.name, "Outline");
    assert!(!read.show && !read.output);
    assert_eq!(read.margin.as_mm(), 1.5);
    assert_eq!(read.kind.as_deref(), Some("cut_out"));
}

#[test]
fn assign_and_remove_touch_only_the_named_memberships() {
    let d = Document::new(1);
    let (_, mfg) = laser(&d);
    let (x, y) = (rect(&d), rect(&d));
    let cut = d
        .add_operation(mfg, &op("cut", "Cut 1"), 0, &[vec![x]])
        .unwrap();
    assert_eq!(
        last_label(&d),
        "assign_to_operation",
        "criterion 8: create and assign"
    );
    let engrave = d
        .add_operation(mfg, &op("engrave", "Engrave 1"), 1, &[])
        .unwrap();
    assert!(
        d.set_operation_members(engrave, &[vec![x], vec![y]], true)
            .unwrap()
    );
    assert_eq!(last_label(&d), "assign_to_operation");
    assert!(!d.set_operation_members(engrave, &[vec![x]], true).unwrap());
    assert!(d.set_operation_members(engrave, &[vec![x]], false).unwrap());
    assert_eq!(last_label(&d), "remove_from_operation");
    let list = ops(&d, mfg);
    assert_eq!(
        list[0].members,
        vec![vec![x]],
        "criterion 9: the other membership stays"
    );
    assert_eq!(list[1].members, vec![vec![y]]);
    let gone = rect(&d);
    d.delete_objects(&[gone]).unwrap();
    assert_eq!(
        d.set_operation_members(cut, &[vec![gone]], true),
        Err(OperationError::NoSuchTarget)
    );
}

#[test]
fn deleting_an_operation_takes_its_memberships_and_undo_restores_them() {
    let d = Document::new(1);
    let (_, mfg) = laser(&d);
    let x = rect(&d);
    let id = d
        .add_operation(mfg, &op("cut", "Cut 1"), 0, &[vec![x]])
        .unwrap();
    let before = ops(&d, mfg);
    d.delete_operation(id).unwrap();
    assert_eq!(last_label(&d), "delete_operation");
    assert!(ops(&d, mfg).is_empty());
    assert!(
        d.read_objects().objects.iter().any(|o| o.id() == x),
        "no object deleted"
    );
    d.undo_step(last_step(&d)).unwrap();
    assert_eq!(ops(&d, mfg), before, "criterion 4: restored exactly");
}

#[test]
fn deleting_a_target_drops_its_key_in_the_same_step_and_undo_restores_it() {
    let d = Document::new(1);
    let (_, mfg) = laser(&d);
    let x = rect(&d);
    d.add_operation(mfg, &op("cut", "Cut 1"), 0, &[vec![x]])
        .unwrap();
    d.delete_objects(&[x]).unwrap();
    assert_eq!(last_label(&d), "delete_objects");
    let read = &ops(&d, mfg)[0];
    assert!(read.members.is_empty(), "Frage 169 A: no Missing item row");
    d.undo_step(last_step(&d)).unwrap();
    assert_eq!(
        ops(&d, mfg)[0].members,
        vec![vec![x]],
        "criterion 27: undo of the deletion"
    );
}

#[test]
fn deleting_a_placement_removes_its_own_keys_in_the_same_step() {
    let d = Document::new(1);
    let drawing = d.pages()[0].clone();
    let (page, mfg) = laser(&d);
    let placement = d
        .place_page(
            &page.id,
            &drawing.id,
            Affine::translation(Vec2::new(10.0, 10.0)),
        )
        .unwrap();
    let id = d
        .add_operation(mfg, &op("cut_out", "Cut-out 1"), 0, &[vec![placement]])
        .unwrap();
    assert_eq!(ops(&d, mfg)[0].members, vec![vec![placement]]);
    let steps = d.own_steps().len();
    d.delete_placement(placement).unwrap();
    assert_eq!(d.own_steps().len(), steps + 1);
    let read = &ops(&d, mfg)[0];
    assert_eq!(read.id, id);
    assert!(read.members.is_empty());
}

#[test]
fn cuts_last_writes_the_register_and_the_order_in_one_step() {
    let d = Document::new(1);
    let (_, mfg) = laser(&d);
    let cut = d.add_operation(mfg, &op("cut", "Cut 1"), 0, &[]).unwrap();
    let score = d
        .add_operation(mfg, &op("score", "Score 1"), 1, &[])
        .unwrap();
    let key = "x.curvyo.laser.cuts_last";
    assert!(
        !d.manufacturing(mfg)
            .unwrap()
            .process_values
            .contains_key(key)
    );
    assert!(d.set_cuts_last(mfg, false, &[]).unwrap());
    assert_eq!(last_label(&d), "cuts_last_off");
    assert_eq!(
        d.manufacturing(mfg).unwrap().process_values[key],
        serde_json::json!(false)
    );
    assert!(d.set_cuts_last(mfg, true, &[score, cut]).unwrap());
    assert_eq!(last_label(&d), "cuts_last_on");
    let order: Vec<NodeId> = ops(&d, mfg).iter().map(|o| o.id).collect();
    assert_eq!(order, vec![score, cut]);
    assert!(!d.set_cuts_last(mfg, true, &[score, cut]).unwrap());
    d.undo_step(last_step(&d)).unwrap();
    let order: Vec<NodeId> = ops(&d, mfg).iter().map(|o| o.id).collect();
    assert_eq!(
        order,
        vec![cut, score],
        "one undo restores order and switch"
    );
}

#[test]
fn the_chain_runs_from_the_node_up_to_the_top_level() {
    let d = Document::new(1);
    let c = |k: u64, x: f64| NewAnchor::corner(AnchorId::new(1, k), Point::new(x, 0.0));
    let a = d.create_path(&[c(1, 0.0), c(2, 5.0)], false);
    let b = d.create_path(&[c(3, 0.0), c(4, 5.0)], false);
    let inner = d.group_objects(&[a, b]).unwrap();
    let r = rect(&d);
    let outer = d.group_objects(&[inner, r]).unwrap();
    let chain = d.node_chain(a);
    assert_eq!(&chain[..3], &[a, inner, outer]);
    if let Some(&layer) = chain.get(3) {
        assert!(d.is_layer(layer));
        assert_eq!(chain.len(), 4);
    }
    let drawing = d.pages()[0].clone();
    let (page, _) = laser(&d);
    let placement = d
        .place_page(
            &page.id,
            &drawing.id,
            Affine::translation(Vec2::new(10.0, 10.0)),
        )
        .unwrap();
    assert_eq!(d.node_chain(placement), vec![placement]);
    d.delete_objects(&[r]).unwrap();
    assert!(d.node_chain(r).is_empty(), "a gone node is in no chain");
}

/// A group of two paths `a` and `b` and a linked clone of the group.
fn group_with_clone(d: &Document) -> (NodeId, NodeId, NodeId) {
    let c = |k: u64, x: f64, y: f64| NewAnchor::corner(AnchorId::new(1, k), Point::new(x, y));
    let a = d.create_path(&[c(1, 0.0, 0.0), c(2, 10.0, 0.0), c(3, 10.0, 10.0)], true);
    let b = d.create_path(&[c(4, 20.0, 0.0), c(5, 30.0, 5.0)], false);
    let group = d.group_objects(&[a, b]).unwrap();
    let clone = d
        .place_linked_clones(&[group], Vec2::new(100.0, 0.0))
        .unwrap()[0];
    (b, group, clone)
}

#[test]
fn a_clone_member_is_keyed_by_its_path_and_read_with_its_chain() {
    let d = Document::new(1);
    let (_, mfg) = laser(&d);
    let (b, _group, clone) = group_with_clone(&d);
    let path = d.clone_member_path(clone, b).unwrap();
    assert_eq!(
        path,
        vec![clone, b],
        "the clone, then the member (Frage 167 B)"
    );
    assert_eq!(d.clone_member_path(clone, clone), None);
    d.add_operation(mfg, &op("score", "Score 1"), 0, std::slice::from_ref(&path))
        .unwrap();
    assert_eq!(ops(&d, mfg)[0].members, vec![path.clone()]);
    assert_eq!(d.key_chain(&path), vec![path.clone(), vec![clone]]);
}

#[test]
fn deleting_the_original_member_drops_the_path_key_in_the_same_step_and_undo_restores_it() {
    let d = Document::new(1);
    let (_, mfg) = laser(&d);
    let (b, _group, clone) = group_with_clone(&d);
    let path = d.clone_member_path(clone, b).unwrap();
    d.add_operation(mfg, &op("score", "Score 1"), 0, std::slice::from_ref(&path))
        .unwrap();
    d.delete_objects(&[b]).unwrap();
    assert_eq!(last_label(&d), "delete_objects");
    assert!(ops(&d, mfg)[0].members.is_empty(), "no Missing item row");
    d.undo_step(last_step(&d)).unwrap();
    assert_eq!(ops(&d, mfg)[0].members, vec![path]);
}

#[test]
fn deleting_the_clone_drops_its_member_keys_in_the_same_step() {
    let d = Document::new(1);
    let (_, mfg) = laser(&d);
    let (b, _group, clone) = group_with_clone(&d);
    let path = d.clone_member_path(clone, b).unwrap();
    d.add_operation(mfg, &op("score", "Score 1"), 0, &[path])
        .unwrap();
    d.delete_objects(&[clone]).unwrap();
    assert!(ops(&d, mfg)[0].members.is_empty());
}

#[test]
fn ungrouping_drops_the_group_key_in_the_same_step_and_undo_restores_it() {
    let d = Document::new(1);
    let (_, mfg) = laser(&d);
    let x = rect(&d);
    let y = rect(&d);
    let group = d.group_objects(&[x, y]).unwrap();
    d.add_operation(mfg, &op("engrave", "Engrave 1"), 0, &[vec![group]])
        .unwrap();
    d.ungroup(&[group]).unwrap();
    assert!(ops(&d, mfg)[0].members.is_empty());
    d.undo_step(last_step(&d)).unwrap();
    assert_eq!(ops(&d, mfg)[0].members, vec![vec![group]]);
}

#[test]
fn deleting_a_layer_drops_the_keys_of_its_objects_in_the_same_step() {
    let d = Document::new(1);
    let (_, mfg) = laser(&d);
    let layer = d.new_layer(None);
    let x = rect(&d);
    d.move_nodes(&MovePlan {
        nodes: vec![x],
        target: MoveTarget::FrontOf(Some(layer)),
    })
    .unwrap();
    d.add_operation(mfg, &op("cut", "Cut 1"), 0, &[vec![x]])
        .unwrap();
    assert_eq!(
        d.delete_layer(layer),
        Ok(1),
        "the rectangle is in the layer"
    );
    assert!(ops(&d, mfg)[0].members.is_empty());
}

#[test]
fn deleting_a_drawing_page_drops_the_keys_of_its_placements_in_the_same_step() {
    let d = Document::new(1);
    let drawing = d.pages()[0].clone();
    let (page, mfg) = laser(&d);
    let placement = d
        .place_page(
            &page.id,
            &drawing.id,
            Affine::translation(Vec2::new(10.0, 10.0)),
        )
        .unwrap();
    d.add_operation(mfg, &op("cut_out", "Cut-out 1"), 0, &[vec![placement]])
        .unwrap();
    d.delete_page(&drawing.id).unwrap();
    assert!(ops(&d, mfg)[0].members.is_empty());
}

#[test]
fn joining_two_paths_drops_the_key_of_the_absorbed_path_in_the_same_step() {
    let d = Document::new(1);
    let (_, mfg) = laser(&d);
    let c = |k: u64, x: f64| NewAnchor::corner(AnchorId::new(1, k), Point::new(x, 0.0));
    let a = d.create_path(&[c(1, 0.0), c(2, 5.0)], false);
    let b = d.create_path(&[c(3, 10.0), c(4, 15.0)], false);
    d.add_operation(mfg, &op("cut", "Cut 1"), 0, &[vec![b]])
        .unwrap();
    d.join_endpoints(a, AnchorId::new(1, 2), b, AnchorId::new(1, 3))
        .unwrap();
    assert_eq!(last_label(&d), "join_endpoints");
    assert!(
        ops(&d, mfg)[0].members.is_empty(),
        "the absorbed path's key goes"
    );
    d.undo_step(last_step(&d)).unwrap();
    assert_eq!(ops(&d, mfg)[0].members, vec![vec![b]]);
}
