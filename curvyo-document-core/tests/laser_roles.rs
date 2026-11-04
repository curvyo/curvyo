//! The laser roles in the project file (`specs/0027-manufacturing-roles` criteria 4, 5, 17 to 20;
//! ADR 0040): one `x.curvyo.laser.role.<layer id>` register per layer on the manufacturing, written
//! by the generic process register command, merged per layer, kept when stale or unknown, a
//! container value refused on open, and the golden `laser_roles_v36.curvyo`.
//! `CURVYO_WRITE_FIXTURES=1` rewrites it.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Document, DocumentSize, DocumentVersion, ManufacturingError,
    NewManufacturing, NodeId, OpenError, Point, RectBounds, pack, unpack,
};
use loro::{LoroDoc, LoroValue};
use serde_json::{Value, json};

const GOLDEN: &str = "laser_roles_v36.curvyo";
const LABEL: &str = "set_role";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn key(layer: NodeId) -> String {
    format!("x.curvyo.laser.role.{}", layer.to_register_text())
}

/// A layer named `name` holding one rectangle.
fn layer_with_rect(document: &Document, name: &str, x: f64) -> NodeId {
    let layer = document.new_layer(None);
    document.rename_node(layer, name).unwrap();
    document.set_draw_parent(Some(layer));
    let _ = document.create_rect(RectBounds::from_corners(
        Point::new(x, 10.0),
        Point::new(x + 10.0, 20.0),
    ));
    document.set_draw_parent(None);
    layer
}

fn add_laser(document: &Document) -> NodeId {
    document
        .add_manufacturing(NewManufacturing {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
            place: Some("first"),
        })
        .unwrap()
}

fn set(document: &Document, m: NodeId, layer: NodeId, role: Option<&str>) -> bool {
    document
        .set_process_register(m, &key(layer), role.map(Value::from).as_ref(), LABEL)
        .unwrap()
}

/// What the drawing holds: every node with its registers, and the pages.
fn drawing(document: &Document) -> String {
    format!("{:?} {:?}", document.read_nodes(), document.pages())
}

/// Layers A (Cut), B (Engrave), C (Ignore), D (no role), E (the unknown role "score") and the
/// role of a deleted layer (criterion 19).
fn golden_document() -> Document {
    // Each role is set right after its layer is drawn, so no two adjacent history rows share a
    // label (`0020` criterion 36 reads every fixture).
    let document = Document::new(1);
    let made = add_laser(&document);
    for (name, x, role) in [
        ("A", 10.0, Some("cut")),
        ("B", 30.0, Some("engrave")),
        ("C", 50.0, Some("ignore")),
        ("D", 70.0, None),
        ("E", 90.0, Some("score")),
        ("Gone", 110.0, Some("cut")),
    ] {
        let layer = layer_with_rect(&document, name, x);
        if let Some(role) = role {
            set(&document, made, layer, Some(role));
        }
        if name == "Gone" {
            document.delete_layer(layer).unwrap();
        }
    }
    document
}

fn write_fixtures_on_request() {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(
            fixture_path(GOLDEN),
            pack(&golden_document(), "0.1.0").unwrap(),
        )
        .unwrap();
    }
}

fn roles(document: &Document) -> serde_json::Map<String, Value> {
    document.manufacturings()[0].process_values.clone()
}

#[test]
fn the_format_stays_36() {
    assert_eq!(CURRENT_FORMAT_VERSION, 36);
}

#[test]
fn a_node_id_round_trips_through_its_register_text() {
    let document = Document::new(7);
    let layer = document.new_layer(None);
    let text = layer.to_register_text();
    assert!(text.starts_with("7:"), "{text}");
    assert_eq!(NodeId::from_register_text(&text), Some(layer));
    for bad in ["outside", "7", "7:x", ":3", ""] {
        assert_eq!(NodeId::from_register_text(bad), None, "{bad}");
    }
}

#[test]
fn ac4_a_role_is_one_step_per_manufacturing_and_the_drawing_never_changes() {
    let document = Document::new(1);
    let a = layer_with_rect(&document, "A", 10.0);
    let _b = layer_with_rect(&document, "B", 30.0);
    let m1 = add_laser(&document);
    let m2 = add_laser(&document);
    let before = drawing(&document);
    let steps = document.own_step_count();
    assert!(set(&document, m1, a, Some("cut")));
    assert_eq!(document.own_step_count(), steps + 1);
    assert_eq!(document.own_steps().last().unwrap().label, LABEL);
    assert!(!set(&document, m1, a, Some("cut")), "the same state again");
    assert_eq!(document.own_step_count(), steps + 1);
    assert_eq!(
        document.manufacturing(m1).unwrap().process_values[&key(a)],
        "cut"
    );
    assert!(
        document
            .manufacturing(m2)
            .unwrap()
            .process_values
            .is_empty()
    );
    assert_eq!(drawing(&document), before);
    assert!(set(&document, m1, a, None), "clearing deletes the register");
    assert!(roles(&document).is_empty());
    assert!(!set(&document, m1, a, None));
    assert_eq!(drawing(&document), before);
}

#[test]
fn ac5_a_key_outside_the_process_or_an_unknown_manufacturing_is_refused() {
    let document = Document::new(1);
    let a = layer_with_rect(&document, "A", 10.0);
    let m = add_laser(&document);
    let steps = document.own_step_count();
    for bad in [
        "x.other.role.1:1",
        "x.curvyo.laser.",
        "mfg_page",
        "x.curvyo.lasers.role",
    ] {
        assert!(matches!(
            document.set_process_register(m, bad, Some(&json!("cut")), LABEL),
            Err(ManufacturingError::OutsideProcess)
        ));
    }
    assert!(matches!(
        document.set_process_register(a, &key(a), Some(&json!("cut")), LABEL),
        Err(ManufacturingError::NotFound)
    ));
    assert_eq!(document.own_step_count(), steps);
}

#[test]
fn ac5_two_peers_setting_two_layers_both_keep_their_role() {
    let one = Document::new(1);
    let a = layer_with_rect(&one, "A", 10.0);
    let b = layer_with_rect(&one, "B", 30.0);
    let m = add_laser(&one);
    let two = unpack(2, &pack(&one, "0.1.0").unwrap()).unwrap();
    set(&one, m, a, Some("cut"));
    set(&two, m, b, Some("engrave"));
    let from_one = one.export_updates(&DocumentVersion::never()).unwrap();
    let from_two = two.export_updates(&DocumentVersion::never()).unwrap();
    one.import_updates(&from_two).unwrap();
    two.import_updates(&from_one).unwrap();
    for document in [&one, &two] {
        let values = roles(document);
        assert_eq!(values[&key(a)], "cut");
        assert_eq!(values[&key(b)], "engrave");
    }
    // The same layer set by both: one value wins on both sides.
    set(&one, m, a, Some("ignore"));
    set(&two, m, a, Some("engrave"));
    let from_one = one.export_updates(&DocumentVersion::never()).unwrap();
    let from_two = two.export_updates(&DocumentVersion::never()).unwrap();
    one.import_updates(&from_two).unwrap();
    two.import_updates(&from_one).unwrap();
    assert_eq!(roles(&one)[&key(a)], roles(&two)[&key(a)]);
}

#[test]
fn ac17_deleting_a_layer_keeps_its_role_and_ac20_deleting_the_manufacturing_takes_them() {
    let document = Document::new(1);
    let a = layer_with_rect(&document, "A", 10.0);
    let m = add_laser(&document);
    set(&document, m, a, Some("cut"));
    document.delete_layer(a).unwrap();
    assert_eq!(roles(&document)[&key(a)], "cut");
    document.delete_manufacturing(m).unwrap();
    assert_eq!(document.manufacturings(), Vec::new());
    assert_eq!(document.read_nodes().len(), 0, "the drawing is untouched");
}

#[test]
fn ac19_the_golden_reads_back_every_role_the_stale_entry_and_the_unknown_value() {
    write_fixtures_on_request();
    let bytes = std::fs::read(fixture_path(GOLDEN)).unwrap();
    let document = unpack(2, &bytes).unwrap();
    let version = document.version();
    let values = roles(&document);
    let mut stored: Vec<&str> = values.values().filter_map(Value::as_str).collect();
    stored.sort_unstable();
    assert_eq!(stored, vec!["cut", "cut", "engrave", "ignore", "score"]);
    assert!(values.keys().all(|k| k.starts_with("x.curvyo.laser.role.")));
    let layers: Vec<String> = document
        .read_nodes()
        .iter()
        .filter(|n| n.depth == 0)
        .map(|n| n.id.to_register_text())
        .collect();
    let stale = values
        .keys()
        .filter(|k| !layers.iter().any(|l| k.ends_with(l.as_str())))
        .count();
    assert_eq!(stale, 1, "the deleted layer's role stays");
    assert_eq!(document.version(), version, "opening writes nothing");
    let again = unpack(3, &pack(&document, "0.1.0").unwrap()).unwrap();
    assert_eq!(roles(&again), values, "saved back unchanged");
}

#[test]
fn ac19_files_from_before_the_slice_open_with_no_role() {
    for name in ["laser_workbench_v36.curvyo", "machine_profile_v36.curvyo"] {
        let bytes = std::fs::read(fixture_path(name)).unwrap();
        let document = unpack(2, &bytes).unwrap();
        assert!(
            document
                .manufacturings()
                .iter()
                .all(|m| m.process_values.is_empty()),
            "{name}"
        );
    }
}

/// The manufacturing's meta map of a Loro document read from `bytes`.
fn with_meta(bytes: &[u8], edit: impl FnOnce(&loro::LoroMap)) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut stored = Vec::new();
    archive
        .by_name("document.loro")
        .unwrap()
        .read_to_end(&mut stored)
        .unwrap();
    let loro = LoroDoc::new();
    loro.import(&stored).unwrap();
    let tree = loro.get_tree("paths");
    let node = tree
        .children(loro::TreeParentId::Root)
        .unwrap()
        .into_iter()
        .find(|&n| tree.get_meta(n).unwrap().get("manufacturing").is_some())
        .unwrap();
    edit(&tree.get_meta(node).unwrap());
    loro.commit();
    let manifest = json!({
        "format_version": CURRENT_FORMAT_VERSION,
        "loro_snapshot_version": curvyo_document_core::CURRENT_LORO_SNAPSHOT_VERSION,
        "app_version": "0.1.0",
    });
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    writer.start_file("manifest.json", options).unwrap();
    writer.write_all(manifest.to_string().as_bytes()).unwrap();
    writer.start_file("document.loro", options).unwrap();
    writer
        .write_all(&loro.export(loro::ExportMode::Snapshot).unwrap())
        .unwrap();
    writer.start_file("document.json", options).unwrap();
    writer.write_all(b"{}").unwrap();
    writer.finish().unwrap().into_inner()
}

#[test]
fn ac18_a_role_key_holding_a_container_is_refused_as_damaged() {
    let bytes = pack(&golden_document(), "0.1.0").unwrap();
    let damaged = with_meta(&bytes, |meta| {
        meta.insert_container("x.curvyo.laser.role.outside", loro::LoroMap::new())
            .unwrap();
    });
    assert!(matches!(unpack(2, &damaged), Err(OpenError::Damaged)));
    let plain = with_meta(&bytes, |meta| {
        meta.insert("x.curvyo.laser.role.outside", LoroValue::I64(3))
            .unwrap();
    });
    let document = unpack(2, &plain).unwrap();
    assert_eq!(roles(&document)["x.curvyo.laser.role.outside"], 3);
}
