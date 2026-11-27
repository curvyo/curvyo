//! Tester acceptance tests for `specs/0147-rounded-box` slice 3, Detach part (criterion 15 and
//! criterion 8 for the format), written from the specification through the session's public
//! surface: a new box runs one operation for all parts; Detach part gives the lid its own copy
//! that still regenerates and moves with the box; save and reopen keep both; Attach part undoes
//! it in one step; format 65 with 64 and 66 checked; deleting the box leaves no stale keys.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::Point;
use curvyo_document_core::{OpenError, unpack};
use curvyo_editor_wasm::{Session, Tool};
use serde_json::Value;

const ROUNDED_BOX: &str = "curvyo.generators/rounded-box";

/// A laser page with a rounded box on it assigned to one cut operation; returns the session.
fn boxed_in_cut(peer: u64) -> Session {
    let mut s = Session::new(peer);
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    s.set_tool(Tool::Generators);
    s.set_generator_kind(ROUNDED_BOX);
    assert!(s.place_generated(150.0, 120.0));
    assert!(s.press_operation_kind("cut", true).is_some());
    s
}

fn rows(s: &Session) -> Vec<Value> {
    let panel: Value = serde_json::from_str(&s.generated_panel_json()).unwrap();
    panel["parts"].as_array().cloned().unwrap_or_default()
}

fn part(s: &Session, key: &str) -> Value {
    rows(s)
        .into_iter()
        .find(|row| row["key"] == key)
        .unwrap_or_else(|| panic!("no part {key} in {:?}", rows(s)))
}

/// Opens a packed file; its document, its laser page and its first generated object, if any.
fn read_back(
    bytes: &[u8],
) -> (
    curvyo_document_core::Document,
    curvyo_document_core::NodeId,
    Option<curvyo_document_core::NodeId>,
) {
    let d = unpack(2, bytes).unwrap();
    let laser = d.manufacturings()[0].id;
    let boxed = d
        .read_objects()
        .objects
        .first()
        .map(curvyo_document_core::ObjectSnapshot::id);
    (d, laser, boxed)
}

/// The manifest.s `format_version` of a packed file.
fn format_of(bytes: &[u8]) -> u64 {
    let (_, manifest) = members(bytes)
        .into_iter()
        .find(|(name, _)| name == "manifest.json")
        .unwrap();
    serde_json::from_slice::<Value>(&manifest).unwrap()["format_version"]
        .as_u64()
        .unwrap()
}

fn members(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut file = archive.by_index(i).unwrap();
            let mut content = Vec::new();
            file.read_to_end(&mut content).unwrap();
            (
                String::from_utf8_lossy(file.name_raw()).into_owned(),
                content,
            )
        })
        .collect()
}

fn zip_of(files: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for (name, content) in files {
        writer.start_file(name.as_str(), options).unwrap();
        writer.write_all(content).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn criterion_15_a_new_box_has_one_operation_for_all_its_parts() {
    let s = boxed_in_cut(15_001);
    let parts = rows(&s);
    assert_eq!(parts.len(), 5, "strip, inner base, cover, ring, lid");
    let first = parts[0]["operation"].clone();
    assert_ne!(first, "Not assigned", "the box is assigned to the cut");
    assert!(
        parts.iter().all(|row| row["operation"] == first),
        "every part runs in the box's one operation: {parts:?}"
    );
    assert!(parts.iter().all(|row| row["detached"] == false));
}

#[test]
fn criterion_15_detach_part_gives_the_lid_its_own_operation_and_one_undo_step() {
    let mut s = boxed_in_cut(15_002);
    let box_op = part(&s, "strip")["operation"].clone();
    assert!(s.detach_selected_part("lid"), "Detach part on the lid");
    let lid = part(&s, "lid");
    assert_eq!(lid["detached"], true);
    assert_ne!(lid["operation"], box_op, "the lid has its own operation");
    assert_eq!(
        part(&s, "strip")["operation"],
        box_op,
        "the rest stays in the box's operation"
    );
    // One undo step takes the whole Detach part back.
    assert!(s.undo().changed, "undo has a step");
    assert_eq!(
        part(&s, "lid")["detached"],
        false,
        "one undo restores the shared operation"
    );
    assert_eq!(part(&s, "lid")["operation"], box_op);
    assert!(s.redo().changed);
    assert_eq!(
        part(&s, "lid")["detached"],
        true,
        "redo brings the lid back"
    );
}

#[test]
fn criterion_15_a_detached_lid_still_regenerates_and_moves_with_the_box() {
    let mut s = boxed_in_cut(15_003);
    assert!(s.detach_selected_part("lid"));
    let answer = s.set_generated_value("lid_thickness", "4");
    assert!(
        answer.contains("\"written\":true"),
        "Lid thickness is a field of the box: {answer}"
    );
    let lid = part(&s, "lid");
    assert_eq!(lid["detached"], true, "the thickness change keeps the copy");
    assert_ne!(lid["operation"], part(&s, "strip")["operation"]);
    // Moving the box: the lid is part of the box, so it stays a part after a move.
    let before = s.pack("0.1.0").unwrap();
    s.key_down(curvyo_editor_wasm::KeyInput {
        key: "ArrowRight",
        shift: true,
        ctrl: false,
        alt: false,
        repeat: false,
        dom_blocked: false,
        mac: false,
        time_ms: 0.0,
    });
    assert_eq!(part(&s, "lid")["detached"], true, "the move keeps the part");
    assert_ne!(s.pack("0.1.0").unwrap(), before, "the move is written");
}

#[test]
fn criterion_15_save_and_reopen_keep_the_box_parts_and_the_detached_lid() {
    let mut s = boxed_in_cut(15_004);
    assert!(s.detach_selected_part("lid"));
    let bytes = s.pack("0.1.0").unwrap();
    let (d, laser, boxed) = read_back(&bytes);
    let boxed = boxed.unwrap();
    let ops = d.operations(laser).unwrap();
    assert_eq!(ops.len(), 2, "the box op and the lid copy");
    assert_eq!(ops[0].members, [vec![boxed]], "the box keeps its operation");
    assert_eq!(ops[1].members.len(), 1, "the lid copy keeps one member");
    let member = &ops[1].members[0];
    assert_eq!(member.len(), 1);
    assert_eq!(
        member[0].part().map(|p| p.to_string()),
        Some("lid".to_owned()),
        "the lid copy keeps the lid"
    );
    assert!(
        d.generated(boxed)
            .unwrap()
            .record
            .part_index("lid")
            .is_some()
    );
    let reopened = Session::open(15_005, &bytes).unwrap();
    assert_eq!(
        reopened.pack("0.1.0").unwrap(),
        bytes,
        "opens as stored and saves back unchanged"
    );
}

#[test]
fn criterion_15_attach_part_returns_the_lid_in_one_step_and_refuses_a_second() {
    let mut s = boxed_in_cut(15_006);
    let box_op = part(&s, "strip")["operation"].clone();
    assert!(s.detach_selected_part("lid"));
    assert!(
        s.attach_selected_part("lid"),
        "Attach part on the detached lid"
    );
    assert_eq!(part(&s, "lid")["detached"], false);
    assert_eq!(part(&s, "lid")["operation"], box_op);
    assert!(
        !s.attach_selected_part("lid"),
        "nothing to attach the second time"
    );
    assert!(s.undo().changed);
    assert_eq!(
        part(&s, "lid")["detached"],
        true,
        "one undo takes Attach part back"
    );
}

#[test]
fn criterion_8_pack_writes_format_68_and_a_newer_file_is_refused_with_format_too_new() {
    let mut s = boxed_in_cut(15_007);
    assert!(s.detach_selected_part("lid"));
    let bytes = s.pack("0.1.0").unwrap();
    assert_eq!(
        format_of(&bytes),
        68,
        "format_version 68 (65 added the part keys)"
    );
    assert!(unpack(2, &bytes).is_ok());

    let mut files = members(&bytes);
    for (name, content) in &mut files {
        if name == "manifest.json" {
            let mut manifest: Value = serde_json::from_slice(content).unwrap();
            manifest["format_version"] = serde_json::json!(69);
            *content = serde_json::to_vec(&manifest).unwrap();
        }
    }
    assert!(
        matches!(
            Session::open(15_008, &zip_of(&files)),
            Err(OpenError::FormatTooNew {
                found: 69,
                supported: 68
            })
        ),
        "a file of 69 gets FormatTooNew"
    );
}

#[test]
fn criterion_8_a_file_of_64_opens_unchanged_and_saves_back() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-document-core/tests/fixtures/sketch_instance_links_v64.curvyo");
    let bytes = std::fs::read(fixture).unwrap();
    let s = Session::open(15_009, &bytes).expect("a file of 64 opens");
    assert_eq!(
        format_of(&s.pack("0.1.0").unwrap()),
        68,
        "saved as 68 by this build"
    );
}

#[test]
fn criterion_15_deleting_the_box_leaves_no_detached_operation_behind() {
    let mut s = boxed_in_cut(15_010);
    assert!(s.detach_selected_part("lid"));
    let (_, laser, boxed) = read_back(&s.pack("0.1.0").unwrap());
    let boxed = boxed.unwrap();
    s.set_tool(Tool::Select);
    s.pointer_down(Point::new(-5000.0, -5000.0), false);
    s.pointer_up(Point::new(5000.0, 5000.0), false, false);
    s.delete_selected();
    let (d, _, _) = read_back(&s.pack("0.1.0").unwrap());
    assert!(
        d.operations(laser).unwrap().iter().all(|op| op
            .members
            .iter()
            .flatten()
            .all(|n| n.part().is_none())),
        "no stale part key"
    );
    assert!(d.generated(boxed).is_none(), "the box is gone");
}

#[test]
fn criterion_15_convert_to_paths_leaves_no_part_key_behind() {
    let mut s = boxed_in_cut(15_013);
    assert!(s.detach_selected_part("lid"));
    assert!(s.convert_selected_generated());
    let (d, laser, boxed) = read_back(&s.pack("0.1.0").unwrap());
    assert!(
        boxed.is_none_or(|id| d.generated(id).is_none()),
        "the box became paths"
    );
    assert!(
        d.operations(laser).unwrap().iter().all(|op| op
            .members
            .iter()
            .flatten()
            .all(|n| n.part().is_none())),
        "no stale part key"
    );
}
