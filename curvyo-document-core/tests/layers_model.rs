//! The document model of layers and node registers (`specs/0050-layers-and-objects-panel`
//! criteria 16 to 22, 24, 30, 31, 34, 36, 37, 42, 45 and 50, `adrs.md` decisions 1 to 4 and 8):
//! the registers, the read pass with effective values, the setters, New and Delete layer, moves
//! and their undo.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]

use curvyo_document_core::{
    Document, MovePlan, MoveRefusal, MoveTarget, NodeId, NodeKind, NodeRead, ObjectEditError,
    Opacity, Point, RectBounds, StepRef,
};

fn rect(document: &Document, x: f64) -> NodeId {
    document.create_rect(RectBounds::from_corners(
        Point::new(x, 0.0),
        Point::new(x + 10.0, 10.0),
    ))
}

fn read_of(document: &Document, id: NodeId) -> NodeRead {
    document
        .read_nodes()
        .into_iter()
        .find(|read| read.id == id)
        .unwrap()
}

fn json(document: &Document) -> serde_json::Value {
    serde_json::from_slice(&document.export_json().unwrap()).unwrap()
}

fn last_step(document: &Document) -> StepRef {
    document.own_steps().last().unwrap().step
}

fn last_label(document: &Document) -> String {
    document.own_steps().last().unwrap().label.clone()
}

/// The id of a node that existed and is gone.
fn gone(document: &Document) -> NodeId {
    let id = rect(document, 500.0);
    document.delete_objects(&[id]).unwrap();
    id
}

fn move_plan(nodes: &[NodeId], target: MoveTarget) -> MovePlan {
    MovePlan {
        nodes: nodes.to_vec(),
        target,
    }
}

/// A layer holding `a` (into it by a move) and a loose `b` at the top.
fn layer_with_one(document: &Document) -> (NodeId, NodeId, NodeId) {
    let a = rect(document, 0.0);
    let layer = document.new_layer(None);
    document
        .move_nodes(&move_plan(&[a], MoveTarget::FrontOf(Some(layer))))
        .unwrap();
    let b = rect(document, 20.0);
    (layer, a, b)
}

#[test]
fn new_layer_is_an_empty_named_top_level_group_in_front() {
    let document = Document::new(1);
    let a = rect(&document, 0.0);
    let first = document.new_layer(None);
    assert_eq!(last_label(&document), "new_layer");
    assert!(document.is_layer(first));
    assert!(document.is_group(first));
    assert_eq!(document.children(None), vec![a, first]);
    assert_eq!(
        document.node_registers(first).unwrap().name.as_deref(),
        Some("Layer 1")
    );
    let top = document.new_layer(None);
    assert_eq!(
        document.node_registers(top).unwrap().name.as_deref(),
        Some("Layer 2")
    );
    // Directly in front of the active layer.
    let middle = document.new_layer(Some(first));
    assert_eq!(document.children(None), vec![a, first, middle, top]);
    assert_eq!(
        document.node_registers(middle).unwrap().name.as_deref(),
        Some("Layer 3")
    );
    // A layer that is gone means the front.
    document.delete_layer(top).unwrap();
    let front = document.new_layer(Some(top));
    assert_eq!(document.children(None).last(), Some(&front));
}

#[test]
fn the_read_lists_every_node_pre_order_with_kinds_and_depths() {
    let document = Document::new(1);
    let (layer, a, b) = layer_with_one(&document);
    let c = rect(&document, 40.0);
    let group = document.group_objects(&[b, c]).unwrap();
    let reads = document.read_nodes();
    let ids: Vec<NodeId> = reads.iter().map(|r| r.id).collect();
    assert_eq!(ids, vec![layer, a, group, b, c]);
    assert_eq!(reads[0].kind, NodeKind::Layer);
    assert_eq!(reads[0].depth, 0);
    assert!(matches!(reads[1].kind, NodeKind::Leaf(_)));
    assert_eq!(reads[1].parent, Some(layer));
    assert_eq!(reads[1].depth, 1);
    assert_eq!(reads[2].kind, NodeKind::Group);
    assert_eq!(reads[3].parent, Some(group));
}

#[test]
fn effective_flags_resolve_through_the_ancestors_and_name_the_nearest_cause() {
    let document = Document::new(1);
    let (layer, a, b) = layer_with_one(&document);
    document.set_node_visible(&[layer], false).unwrap();
    assert_eq!(last_label(&document), "set_node_visible");
    document.set_node_locked(&[layer], true).unwrap();
    assert_eq!(last_label(&document), "set_node_locked");
    let read = read_of(&document, a);
    assert!(read.registers.visible, "its own flag is untouched");
    assert!(!read.visible);
    assert!(read.locked);
    assert_eq!(read.hidden_by, Some(layer));
    assert_eq!(read.locked_by, Some(layer));
    assert!(read_of(&document, b).visible);
    // Showing and unlocking the layer restores the node as it was, its own flags included.
    document.set_node_visible(&[a], false).unwrap();
    document.set_node_visible(&[layer], true).unwrap();
    document.set_node_locked(&[layer], false).unwrap();
    let read = read_of(&document, a);
    assert!(!read.visible, "its own flag still hides it");
    assert_eq!(read.hidden_by, None);
    assert!(!read.locked);
}

#[test]
fn effective_opacity_is_the_product_of_the_node_and_its_ancestors() {
    let document = Document::new(1);
    let (layer, a, _) = layer_with_one(&document);
    let b = rect(&document, 40.0);
    document
        .move_nodes(&move_plan(&[b], MoveTarget::FrontOf(Some(layer))))
        .unwrap();
    let group = document.group_objects(&[a, b]).unwrap();
    document
        .set_node_opacity(&[layer], Opacity::new(0.5).unwrap())
        .unwrap();
    assert_eq!(last_label(&document), "set_node_opacity");
    document
        .set_node_opacity(&[group], Opacity::new(0.5).unwrap())
        .unwrap();
    document
        .set_node_opacity(&[a], Opacity::new(0.8).unwrap())
        .unwrap();
    assert!((read_of(&document, a).opacity.get() - 0.2).abs() < 1e-12);
    assert!((read_of(&document, b).opacity.get() - 0.25).abs() < 1e-12);
    assert_eq!(read_of(&document, layer).opacity.get(), 0.5);
}

#[test]
fn opacity_of_a_locked_node_or_a_node_in_a_locked_container_is_refused() {
    let document = Document::new(1);
    let (layer, a, b) = layer_with_one(&document);
    document.set_node_locked(&[layer], true).unwrap();
    let steps = document.own_step_count();
    assert_eq!(
        document.set_node_opacity(&[b, a], Opacity::new(0.5).unwrap()),
        Err(ObjectEditError::Locked)
    );
    assert_eq!(document.own_step_count(), steps, "nothing was written");
    assert_eq!(read_of(&document, b).registers.opacity.get(), 1.0);
    // Hide, show, unlock and rename work on a locked node.
    document.set_node_visible(&[a], false).unwrap();
    document.rename_node(a, "Fixture").unwrap();
    document.set_node_locked(&[layer], false).unwrap();
}

#[test]
fn rename_cleans_the_name_and_an_empty_one_removes_it() {
    let document = Document::new(1);
    let a = rect(&document, 0.0);
    document.rename_node(a, "  Cut \u{202E}part\u{7} ").unwrap();
    assert_eq!(last_label(&document), "rename_node");
    assert_eq!(
        document.node_registers(a).unwrap().name.as_deref(),
        Some("Cut part")
    );
    document.rename_node(a, &"n".repeat(300)).unwrap();
    assert_eq!(
        document
            .node_registers(a)
            .unwrap()
            .name
            .unwrap()
            .chars()
            .count(),
        128
    );
    document.rename_node(a, "   ").unwrap();
    assert_eq!(document.node_registers(a).unwrap().name, None);
    assert_eq!(
        document.rename_node(gone(&document), "x"),
        Err(ObjectEditError::NoSuchObject)
    );
}

#[test]
fn one_toggle_over_many_nodes_is_one_step() {
    let document = Document::new(1);
    let ids: Vec<NodeId> = (0..5)
        .map(|i| rect(&document, f64::from(i) * 20.0))
        .collect();
    let before = document.own_step_count();
    document.set_node_visible(&ids, false).unwrap();
    assert_eq!(document.own_step_count(), before + 1);
    assert!(document.read_nodes().iter().all(|read| !read.visible));
}

#[test]
fn delete_layer_takes_its_content_in_one_step_and_undo_brings_every_id_back() {
    let document = Document::new(1);
    let (layer, a, b) = layer_with_one(&document);
    let c = rect(&document, 40.0);
    document
        .move_nodes(&move_plan(&[c, b], MoveTarget::FrontOf(Some(layer))))
        .unwrap();
    let group = document.group_objects(&[b, c]).unwrap();
    document.rename_node(layer, "Cut").unwrap();
    document.set_node_visible(&[a], false).unwrap();
    let before = json(&document);
    let steps = document.own_step_count();
    assert_eq!(document.delete_layer(layer), Ok(4));
    assert_eq!(document.own_step_count(), steps + 1);
    assert_eq!(last_label(&document), "delete_layer");
    assert_eq!(document.read_nodes(), Vec::new());
    let after = json(&document);
    let step = last_step(&document);
    for _ in 0..2 {
        let undo = document.undo_step(step).unwrap();
        assert_eq!(undo.skipped(), 0, "{undo:?}");
        assert_eq!(json(&document), before);
        assert!(document.is_layer(layer));
        assert_eq!(document.children(Some(layer)), vec![a, group]);
        assert_eq!(document.children(Some(group)), vec![b, c]);
        let redo = document.redo_step(step).unwrap();
        assert_eq!(redo.skipped(), 0, "{redo:?}");
        assert_eq!(json(&document), after);
    }
}

#[test]
fn delete_layer_refuses_a_locked_layer_and_a_non_layer() {
    let document = Document::new(1);
    let (layer, a, _) = layer_with_one(&document);
    document.set_node_locked(&[layer], true).unwrap();
    assert_eq!(document.delete_layer(layer), Err(ObjectEditError::Locked));
    assert!(document.is_layer(layer));
    assert_eq!(document.delete_layer(a), Err(ObjectEditError::NotALayer));
    assert_eq!(
        document.delete_layer(gone(&document)),
        Err(ObjectEditError::NoSuchObject)
    );
}

#[test]
fn an_emptied_layer_stays_and_an_emptied_group_goes() {
    let document = Document::new(1);
    let (layer, a, _) = layer_with_one(&document);
    document.delete_objects(&[a]).unwrap();
    assert!(
        document.is_layer(layer),
        "deleting the last child leaves the layer"
    );
    let empty = document.new_layer(None);
    let x = rect(&document, 60.0);
    document.delete_objects(&[x]).unwrap();
    assert!(document.is_layer(empty), "an empty layer is not cleaned up");
    // A group emptied by a move goes (`0023` criterion 14).
    let p = rect(&document, 0.0);
    let q = rect(&document, 20.0);
    let group = document.group_objects(&[p, q]).unwrap();
    document
        .move_nodes(&move_plan(&[p, q], MoveTarget::FrontOf(Some(layer))))
        .unwrap();
    assert!(!document.contains_node(group));
    assert_eq!(document.children(Some(layer)), vec![p, q]);
    // A layer emptied by a move stays.
    document
        .move_nodes(&move_plan(&[p, q], MoveTarget::FrontOf(None)))
        .unwrap();
    assert!(document.is_layer(layer));
}

#[test]
fn moves_reorder_among_siblings_and_keep_the_relative_order() {
    let document = Document::new(1);
    let ids: Vec<NodeId> = (0..5)
        .map(|i| rect(&document, f64::from(i) * 20.0))
        .collect();
    let [a, b, c, d, e] = ids[..] else { panic!() };
    document
        .move_nodes(&move_plan(&[d, b], MoveTarget::FrontOf(None)))
        .unwrap();
    assert_eq!(last_label(&document), "move_nodes");
    assert_eq!(document.children(None), vec![a, c, e, b, d]);
    document
        .move_nodes(&move_plan(&[d], MoveTarget::BackOf(None)))
        .unwrap();
    assert_eq!(document.children(None), vec![d, a, c, e, b]);
    document
        .move_nodes(&move_plan(&[d], MoveTarget::Above(c)))
        .unwrap();
    assert_eq!(document.children(None), vec![a, c, d, e, b]);
    document
        .move_nodes(&move_plan(&[b, a], MoveTarget::Below(e)))
        .unwrap();
    assert_eq!(document.children(None), vec![c, d, a, b, e]);
}

#[test]
fn a_move_to_another_parent_keeps_id_data_and_registers_and_undo_puts_it_back() {
    let document = Document::new(1);
    let (layer, a, b) = layer_with_one(&document);
    document.rename_node(b, "Mark").unwrap();
    document.set_node_locked(&[a], true).unwrap();
    document.set_node_locked(&[a], false).unwrap();
    document
        .set_node_opacity(&[b], Opacity::new(0.6).unwrap())
        .unwrap();
    let data = document.object(b);
    let registers = document.node_registers(b);
    let before = json(&document);
    document
        .move_nodes(&move_plan(&[b], MoveTarget::Below(a)))
        .unwrap();
    assert_eq!(document.children(Some(layer)), vec![b, a]);
    assert_eq!(document.object(b), data);
    assert_eq!(document.node_registers(b), registers);
    let after = json(&document);
    let step = last_step(&document);
    document.undo_step(step).unwrap();
    assert_eq!(json(&document), before);
    document.redo_step(step).unwrap();
    assert_eq!(json(&document), after);
}

#[test]
fn undo_of_hide_lock_opacity_and_rename_restores_the_previous_value() {
    let document = Document::new(1);
    let a = rect(&document, 0.0);
    let edits: [&dyn Fn(); 4] = [
        &|| document.set_node_visible(&[a], false).unwrap(),
        &|| {
            document
                .set_node_opacity(&[a], Opacity::new(0.3).unwrap())
                .unwrap();
        },
        &|| document.set_node_locked(&[a], true).unwrap(),
        &|| document.rename_node(a, "Cut").unwrap(),
    ];
    for edit in edits {
        let before = json(&document);
        edit();
        let after = json(&document);
        assert_ne!(before, after);
        let step = last_step(&document);
        document.undo_step(step).unwrap();
        assert_eq!(json(&document), before);
        document.redo_step(step).unwrap();
        assert_eq!(json(&document), after);
    }
}

#[test]
fn refused_moves_write_nothing_and_say_why() {
    let document = Document::new(1);
    let (layer, a, b) = layer_with_one(&document);
    let c = rect(&document, 40.0);
    let group = document.group_objects(&[b, c]).unwrap();
    let other = document.new_layer(None);
    let missing = gone(&document);
    let steps = document.own_step_count();
    let cases = [
        (
            move_plan(&[], MoveTarget::FrontOf(None)),
            MoveRefusal::Nothing,
        ),
        (
            move_plan(&[group], MoveTarget::FrontOf(Some(group))),
            MoveRefusal::IntoItself,
        ),
        (
            move_plan(&[group], MoveTarget::Above(b)),
            MoveRefusal::IntoItself,
        ),
        (
            move_plan(&[layer], MoveTarget::FrontOf(Some(other))),
            MoveRefusal::LayerNotTopLevel,
        ),
        (
            move_plan(&[layer], MoveTarget::Above(a)),
            MoveRefusal::IntoItself,
        ),
        (
            move_plan(&[b], MoveTarget::FrontOf(Some(c))),
            MoveRefusal::NotAContainer,
        ),
        (
            move_plan(&[missing], MoveTarget::FrontOf(None)),
            MoveRefusal::NoSuchNode,
        ),
    ];
    for (plan, refusal) in cases {
        assert_eq!(document.check_move(&plan), Err(refusal), "{plan:?}");
        assert_eq!(document.move_nodes(&plan), Err(refusal), "{plan:?}");
    }
    document.set_node_locked(&[other], true).unwrap();
    let locked_target = move_plan(&[a], MoveTarget::FrontOf(Some(other)));
    assert_eq!(
        document.move_nodes(&locked_target),
        Err(MoveRefusal::LockedTarget)
    );
    document.set_node_locked(&[layer], true).unwrap();
    assert_eq!(
        document.move_nodes(&move_plan(&[a], MoveTarget::FrontOf(None))),
        Err(MoveRefusal::Locked),
        "a node in a locked layer is locked"
    );
    assert_eq!(document.own_step_count(), steps + 2, "only the two locks");
    // A layer moves among the top-level rows.
    document.set_node_locked(&[layer, other], false).unwrap();
    document
        .move_nodes(&move_plan(&[other], MoveTarget::BackOf(None)))
        .unwrap();
    assert_eq!(document.children(None)[0], other);
}

#[test]
fn a_move_that_would_nest_deeper_than_32_is_refused() {
    let document = Document::new(1);
    let mut inner = rect(&document, 0.0);
    let mut outer = inner;
    for i in 0..32 {
        let extra = rect(&document, f64::from(i));
        document
            .move_nodes(&move_plan(&[extra], MoveTarget::Above(inner)))
            .unwrap();
        outer = document.group_objects(&[inner, extra]).unwrap();
        inner = outer;
    }
    let layer = document.new_layer(None);
    assert_eq!(
        document.check_move(&move_plan(&[outer], MoveTarget::FrontOf(Some(layer)))),
        Err(MoveRefusal::TooDeep)
    );
}

#[test]
fn layers_are_transparent_to_the_group_reads_of_the_select_tool() {
    let document = Document::new(1);
    let (layer, a, b) = layer_with_one(&document);
    let c = rect(&document, 40.0);
    document
        .move_nodes(&move_plan(&[b, c], MoveTarget::FrontOf(Some(layer))))
        .unwrap();
    let group = document.group_objects(&[b, c]).unwrap();
    assert_eq!(
        document.parent_group(a),
        None,
        "a child of a layer is top level"
    );
    assert_eq!(document.parent_node(a), Some(layer));
    assert_eq!(document.parent_group(b), Some(group));
    assert_eq!(document.ancestor_groups(b), vec![group]);
    assert_eq!(
        document.child_groups(None),
        vec![(group, vec![b, c])],
        "the layer is not a unit; its group is"
    );
    assert_eq!(
        document.child_groups(Some(layer)),
        vec![(group, vec![b, c])]
    );
    assert_eq!(document.object_ids(), vec![a, b, c]);
}

#[test]
fn new_objects_go_on_top_of_the_draw_parent_layer() {
    let document = Document::new(1);
    let layer = document.new_layer(None);
    let loose = rect(&document, 0.0);
    document.set_draw_parent(Some(layer));
    let a = rect(&document, 20.0);
    let b = rect(&document, 40.0);
    assert_eq!(document.children(Some(layer)), vec![a, b]);
    assert_eq!(document.children(None), vec![layer, loose]);
}

#[test]
fn ten_thousand_nodes_in_a_layer_take_one_register_write_to_hide() {
    let document = Document::new(1);
    let layer = document.new_layer(None);
    document.set_draw_parent(Some(layer));
    document.batch("create_rect", |document| {
        for i in 0..10_000 {
            rect(document, f64::from(i % 100));
        }
    });
    let before = document.version();
    document.set_node_visible(&[layer], false).unwrap();
    let ops = document.export_updates(&before).unwrap();
    assert!(ops.len() < 400, "one register, {} bytes", ops.len());
    assert!(
        document
            .read_nodes()
            .iter()
            .skip(1)
            .all(|read| !read.visible)
    );
}

#[test]
fn group_ungroup_and_delete_refuse_a_layer_before_the_first_write() {
    let document = Document::new(1);
    let (layer, a, b) = layer_with_one(&document);
    let other = document.new_layer(None);
    let steps = document.own_step_count();
    assert_eq!(
        document.group_objects(&[layer, other]),
        Err(ObjectEditError::IsALayer)
    );
    assert_eq!(
        document.group_objects(&[b, other]),
        Err(ObjectEditError::IsALayer)
    );
    assert_eq!(document.ungroup(&[layer]), Err(ObjectEditError::IsALayer));
    assert_eq!(
        document.delete_objects(&[a, layer]),
        Err(ObjectEditError::IsALayer)
    );
    assert_eq!(document.own_step_count(), steps, "nothing was written");
    assert!(document.is_layer(layer) && document.is_layer(other));
    assert_eq!(document.children(Some(layer)), vec![a]);
}

#[test]
fn moving_a_group_with_one_of_its_members_keeps_the_member_inside() {
    let document = Document::new(1);
    let (a, b, c) = (
        rect(&document, 0.0),
        rect(&document, 20.0),
        rect(&document, 40.0),
    );
    let group = document.group_objects(&[a, b]).unwrap();
    let layer = document.new_layer(None);
    document
        .move_nodes(&move_plan(&[b, group], MoveTarget::FrontOf(Some(layer))))
        .unwrap();
    assert_eq!(document.children(Some(layer)), vec![group]);
    assert_eq!(document.children(Some(group)), vec![a, b]);
    assert_eq!(document.children(None), vec![c, layer]);
}

#[test]
fn the_read_of_a_deep_chain_carries_flags_and_opacity_to_the_bottom() {
    let document = Document::new(1);
    let layer = document.new_layer(None);
    let mut inner = rect(&document, 0.0);
    document
        .move_nodes(&move_plan(&[inner], MoveTarget::FrontOf(Some(layer))))
        .unwrap();
    let leaf = inner;
    // The layer counts toward the 32 levels: it holds at most 31 nested groups.
    for i in 0..31 {
        let extra = rect(&document, f64::from(i));
        document
            .move_nodes(&move_plan(&[extra], MoveTarget::Above(inner)))
            .unwrap();
        inner = document.group_objects(&[inner, extra]).unwrap();
    }
    let extra = rect(&document, 99.0);
    document
        .move_nodes(&move_plan(&[extra], MoveTarget::Above(inner)))
        .unwrap();
    assert_eq!(
        document.group_objects(&[inner, extra]),
        Err(ObjectEditError::TooDeep)
    );
    document
        .set_node_opacity(&[layer], Opacity::new(0.5).unwrap())
        .unwrap();
    document.set_node_locked(&[layer], true).unwrap();
    let read = read_of(&document, leaf);
    assert_eq!(read.depth, 32);
    assert!(read.locked);
    assert_eq!(read.locked_by, Some(layer));
    assert_eq!(read.opacity.get(), 0.5);
    assert_eq!(document.read_nodes().len(), 1 + 31 + 32 + 1);
}
