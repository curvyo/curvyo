#![allow(clippy::assert_is_empty)]

//! Session-level tests of Detach part and Attach part (`specs/0147-rounded-box` criterion 15,
//! ADR 0082 §10): a new box has one operation for all parts; after Detach part on the lid the job
//! cuts the lid with its own operation and the rest with the box's, a thickness change
//! regenerates the lid and keeps its key, moving the box moves it, save and reopen keep both, and
//! Attach part puts it back.

use curvyo_document_core::{NodeId, OperationSnapshot};
use curvyo_laser_core::job::{BlockInput, LeafGeometry};
use serde_json::Value;

use crate::session::{Session, Tool};

/// A laser page with a rounded box placed on it and assigned to a new Cut operation.
fn boxed() -> (Session, NodeId) {
    let mut s = Session::new(300_101);
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    let page = s.active_manufacturing().unwrap().page.unwrap();
    s.set_tool(Tool::Generators);
    s.set_generator_kind("curvyo.generators/rounded-box");
    s.resize_viewport(1600.0, 900.0);
    assert!(s.place_generated(page.origin.x + 200.0, page.origin.y + 150.0));
    s.set_tool(Tool::Select);
    let id = s.selected_generated()[0];
    assert!(s.press_operation_kind("cut", true).is_some());
    (s, id)
}

fn ops(s: &Session) -> Vec<OperationSnapshot> {
    let manufacturing = s.active_manufacturing().unwrap();
    s.document.operations(manufacturing.id).unwrap()
}

fn blocks(s: &Session) -> Vec<BlockInput> {
    s.page_input().unwrap().input.blocks
}

fn outline_count(block: &BlockInput) -> usize {
    block
        .leaves
        .iter()
        .map(|leaf| match &leaf.geometry {
            LeafGeometry::Outlines(outlines) => outlines.len(),
            _ => 0,
        })
        .sum()
}

fn panel(s: &Session) -> Value {
    serde_json::from_str(&s.generated_panel_json()).unwrap()
}

fn lid_row(s: &Session) -> Value {
    panel(s)["parts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|part| part["key"] == "lid")
        .cloned()
        .unwrap()
}

#[test]
fn a_new_box_has_one_operation_for_all_parts_and_the_block_lists_its_parts() {
    let (s, _) = boxed();
    assert_eq!(ops(&s).len(), 1);
    let rows = panel(&s)["parts"].as_array().unwrap().clone();
    let keys: Vec<&str> = rows.iter().map(|r| r["key"].as_str().unwrap()).collect();
    assert_eq!(keys, ["strip", "cover", "lid", "base", "ring"], "{rows:?}");
    assert!(rows.iter().all(|r| r["detached"] == false));
    assert!(rows.iter().all(|r| r["operation"] == "Cut 1"), "{rows:?}");
    let all = blocks(&s);
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].leaves.len(), 1, "one leaf for the whole box");
}

#[test]
fn detach_part_gives_the_lid_its_own_operation_in_one_undo_step() {
    let (mut s, id) = boxed();
    let whole = outline_count(&blocks(&s)[0]);
    s.wipe_history();
    assert!(s.detach_selected_part("lid"));
    assert_eq!(s.undo_len(), 1, "one step");
    let list = ops(&s);
    assert_eq!(list.len(), 2);
    assert_eq!(list[1].name, "Cut 1 · Lid");
    assert_eq!(list[1].kind, list[0].kind);
    assert_eq!(list[1].members, vec![vec![id.with_part("lid").unwrap()]]);
    let row = lid_row(&s);
    assert_eq!(row["detached"], true);
    assert_eq!(row["operation"], "Cut 1 · Lid");
    // The job: the lid in its own operation, the rest in the box's, nothing lost or doubled.
    let all = blocks(&s);
    assert_eq!(all.len(), 2);
    let (rest, lid) = (outline_count(&all[0]), outline_count(&all[1]));
    assert!(lid >= 1 && rest >= 1);
    assert_eq!(rest + lid, whole, "the same cut as before, split in two");
    // Undo brings the box back to one operation and one leaf.
    let _ = s.undo();
    assert_eq!(ops(&s).len(), 1);
    assert_eq!(blocks(&s).len(), 1);
    assert_eq!(outline_count(&blocks(&s)[0]), whole);
}

#[test]
fn a_thickness_change_regenerates_the_lid_and_moving_the_box_moves_it() {
    let (mut s, id) = boxed();
    assert!(s.detach_selected_part("lid"));
    let before = blocks(&s)[1].clone();
    let answer: Value = serde_json::from_str(&s.set_generated_value("thickness", "6")).unwrap();
    assert_eq!(answer["written"], true, "{answer}");
    let after = blocks(&s)[1].clone();
    assert_ne!(before.leaves, after.leaves, "the lid was regenerated");
    assert_eq!(s.document.detached_parts(id), ["lid"], "its key stays");
    assert_eq!(outline_count(&after), outline_count(&before));
    let first = |b: &BlockInput| match &b.leaves[0].geometry {
        LeafGeometry::Outlines(o) => o[0].anchors[0].0,
        _ => panic!("outlines"),
    };
    let at = first(&after);
    s.document
        .translate_objects(&[id], curvyo_document_core::Vec2::new(10.0, 0.0))
        .unwrap();
    let moved = first(&blocks(&s)[1]);
    assert!((moved.x - at.x - 10.0).abs() < 1e-6, "{at:?} {moved:?}");
    assert_eq!(s.document.detached_parts(id), ["lid"]);
}

#[test]
fn attach_part_runs_the_lid_with_the_box_again_in_one_step() {
    let (mut s, _) = boxed();
    assert!(s.detach_selected_part("lid"));
    assert!(!s.detach_selected_part("lid"), "already detached");
    s.wipe_history();
    assert!(s.attach_selected_part("lid"));
    assert_eq!(s.undo_len(), 1);
    assert_eq!(lid_row(&s)["detached"], false);
    assert_eq!(blocks(&s)[0].leaves.len(), 1);
    assert!(
        blocks(&s).iter().skip(1).all(|b| b.leaves.is_empty()),
        "the copy is left, with nothing in it"
    );
    assert!(!s.attach_selected_part("lid"), "not detached");
}

#[test]
fn detach_needs_an_operation_to_copy() {
    let (mut s, id) = boxed();
    let op = ops(&s)[0].id;
    s.selection.set(&[id]);
    assert!(s.delete_operation(op));
    assert!(!s.detach_selected_part("lid"));
    assert_eq!(
        s.transform_notice,
        Some("Assign the box to an operation first. Nothing was changed.")
    );
    assert!(ops(&s).is_empty());
}

#[test]
fn a_saved_and_reopened_box_keeps_the_detached_lid() {
    let (mut s, id) = boxed();
    assert!(s.detach_selected_part("lid"));
    let page = s.active_manufacturing().unwrap().page.unwrap().id;
    let bytes = s.pack("0.1.0").unwrap();
    let mut again = Session::open(9, &bytes).unwrap();
    assert!(again.activate_page(&page));
    assert_eq!(again.document.detached_parts(id), ["lid"]);
    assert_eq!(blocks(&again).len(), 2);
}

#[test]
fn convert_to_paths_keeps_the_lid_in_its_own_operation() {
    let (mut s, id) = boxed();
    assert!(s.detach_selected_part("lid"));
    let before = blocks(&s);
    let total: usize = before.iter().map(outline_count).sum();
    assert!(s.convert_selected_generated());
    let after = blocks(&s);
    assert_eq!(after.len(), 2);
    assert_eq!(after.iter().map(outline_count).sum::<usize>(), total);
    assert_eq!(
        outline_count(&after[1]),
        outline_count(&before[1]),
        "the lid's own path still runs in the lid's operation"
    );
    assert!(
        s.document.detached_parts(id).is_empty(),
        "no part key is left"
    );
}

#[test]
fn criterion_15_the_copy_counts_the_detached_lid_as_its_item() {
    let (mut s, _) = boxed();
    assert!(s.detach_selected_part("lid"));
    let view: Value = serde_json::from_str(&s.operations_view_json()).unwrap();
    let items: Vec<(String, u64)> = view["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["name"].as_str().unwrap().to_owned(),
                row["items"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        items,
        [("Cut 1".to_owned(), 1), ("Cut 1 · Lid".to_owned(), 1)]
    );
}

#[test]
fn criterion_16_a_curved_top_lists_strip_end_plates_front_base() {
    let (mut s, _) = boxed();
    let answer = s.set_generated_value("top_shape", "\"curved\"");
    assert!(answer.contains("\"written\":true"), "{answer}");
    let keys: Vec<String> = panel(&s)["parts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["key"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(keys, ["strip", "left", "right", "front", "base"]);
}
