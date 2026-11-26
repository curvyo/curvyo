//! The stored links of sketch instances (`specs/0099-sketch-pages` criterion 37, `adrs.md` A13):
//! the golden of format 64, the damaged-file golden, a file of 62 that opens unchanged and the
//! residue cases (a member of another kind, a group of one, a mark without an own value) that
//! read as stated and save back equal.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, CURRENT_FORMAT_VERSION, CloneSnapshot, Document, ExposedKey, InstanceValue,
    InstanceWrite, NodeId, ObjectSnapshot, OpenError, Style, ValueSet, pack, unpack,
};

#[path = "support/sketch_links_graph.rs"]
mod links;

const GOLDEN: &str = "sketch_instance_links_v64.curvyo";
const DAMAGED: &str = "sketch_instance_links_damaged_v64.curvyo";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn dim(id: &str) -> ExposedKey {
    ExposedKey::dimension(&id.into())
}

fn write(d: &Document, clone: NodeId, pairs: &[(&str, f64)], unlinked: &[&str], label: &str) {
    let mut own = ValueSet::default();
    for (key, number) in pairs {
        let value = if key.starts_with("ang") {
            InstanceValue::from_deg(*number)
        } else {
            InstanceValue::from_mm(*number)
        };
        own.insert(dim(key), value);
    }
    d.write_instance_values(
        &[InstanceWrite {
            clone,
            own,
            last_solvable: None,
            unlinked: unlinked.iter().map(|key| dim(key)).collect(),
        }],
        label,
        None,
    )
    .unwrap();
}

/// A sketch with a group of two lengths, a group of two angles and an entry with an invalid link
/// kept; clones: none own, coupled own values, one value unlinked, a chain whose second clone has
/// its own coupled values.
fn golden_document() -> (Document, NodeId, Vec<NodeId>) {
    let d = Document::new(1);
    let sk = d
        .create_sketch(&links::graph(), Affine::IDENTITY, &Style::default())
        .unwrap();
    for (key, name) in [
        ("len_a", "Leg 1"),
        ("len_b", "Leg 2"),
        ("ang_a", "Angle 1"),
        ("ang_b", "Angle 2"),
    ] {
        d.expose_parameter(sk, &dim(key), Some(name)).unwrap();
        // A step between two exposes, so no two rows of the history share a label.
        d.set_exposed_limits(sk, &dim(key), Some(-500.0), Some(500.0), Some(name))
            .unwrap();
    }
    d.expose_parameter(sk, &ExposedKey::parameter(&"w".into()), None)
        .unwrap();
    d.link_exposed(sk, &dim("len_a"), &dim("len_b")).unwrap();
    // A step between the two links, so no two rows of the history share a label.
    d.set_exposed_limits(sk, &dim("ang_a"), Some(-90.0), Some(90.0), Some("Angle 1"))
        .unwrap();
    d.link_exposed(sk, &dim("ang_a"), &dim("ang_b")).unwrap();
    let plain = d.linked_clone(&[sk], &[]).unwrap()[0];
    // A step between two linked clones, so no two rows of the history share a label.
    d.set_exposed_limits(sk, &dim("len_a"), Some(0.0), Some(500.0), Some("Leg 1"))
        .unwrap();
    let coupled = d.linked_clone(&[sk], &[]).unwrap()[0];
    write(
        &d,
        coupled,
        &[("len_a", 70.0), ("len_b", 70.0)],
        &[],
        "change_parameter",
    );
    let unlinked = d.linked_clone(&[sk], &[]).unwrap()[0];
    write(
        &d,
        unlinked,
        &[("ang_a", 10.0), ("ang_b", 90.0)],
        &["ang_a"],
        "own_parameters",
    );
    let chained = d.linked_clone(&[coupled], &[]).unwrap()[0];
    write(
        &d,
        chained,
        &[("len_a", 50.0), ("len_b", 50.0)],
        &[],
        "change_parameter",
    );
    (d, sk, vec![plain, coupled, unlinked, chained])
}

/// Rewrites the stored document with `edit` on the Loro side, as a file from elsewhere would be.
fn edited(d: &Document, edit: impl FnOnce(&loro::LoroTree)) -> Vec<u8> {
    let loro = loro::LoroDoc::new();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    loro.set_peer_id(7).unwrap();
    edit(&loro.get_tree("paths"));
    loro.commit();
    let mut files = members(&pack(d, "0.1.0").unwrap());
    files.retain(|(name, _)| name != "document.loro");
    files.push((
        "document.loro".into(),
        loro.export(loro::ExportMode::Snapshot).unwrap(),
    ));
    zip_of(&files)
}

/// The golden with the invalid link entry added on the sketch.
fn golden_bytes() -> Vec<u8> {
    let (d, sk, _) = golden_document();
    edited(&d, |tree| {
        let meta = tree.get_meta(tree_id(sk)).unwrap();
        let mut items = match meta.get("exposed_params") {
            Some(loro::ValueOrContainer::Value(value)) => match serde_json::Value::from(value) {
                serde_json::Value::Array(items) => items,
                _ => panic!("list"),
            },
            _ => panic!("register"),
        };
        items[4]["link"] = serde_json::json!(42);
        meta.insert(
            "exposed_params",
            loro::LoroValue::from(serde_json::Value::Array(items)),
        )
        .unwrap();
    })
}

/// The golden with `instance_unlinked` of one clone as a container, the wrong register shape.
fn damaged_bytes() -> Vec<u8> {
    let (d, _, clones) = golden_document();
    edited(&d, |tree| {
        let meta = tree.get_meta(tree_id(clones[2])).unwrap();
        meta.delete("instance_unlinked").unwrap();
        let map = meta
            .insert_container("instance_unlinked", loro::LoroMap::new())
            .unwrap();
        map.insert("d:ang_a", true).unwrap();
    })
}

fn tree_id(id: NodeId) -> loro::TreeID {
    let json = serde_json::to_value(id).unwrap();
    loro::TreeID::new(
        json["peer"].as_str().unwrap().parse().unwrap(),
        i32::try_from(json["counter"].as_i64().unwrap()).unwrap(),
    )
}

fn members(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut file = archive.by_index(i).unwrap();
            let mut content = Vec::new();
            file.read_to_end(&mut content).unwrap();
            (file.name().unwrap().to_string(), content)
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

fn stored(name: &str) -> Vec<u8> {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        match name {
            GOLDEN => std::fs::write(fixture_path(GOLDEN), golden_bytes()).unwrap(),
            DAMAGED => std::fs::write(fixture_path(DAMAGED), damaged_bytes()).unwrap(),
            _ => {}
        }
    }
    std::fs::read(fixture_path(name)).unwrap()
}

fn manifest_format(bytes: &[u8]) -> u64 {
    let files = members(bytes);
    let manifest = &files.iter().find(|(n, _)| n == "manifest.json").unwrap().1;
    serde_json::from_slice::<serde_json::Value>(manifest).unwrap()["format_version"]
        .as_u64()
        .unwrap()
}

fn clone_of(d: &Document, id: NodeId) -> CloneSnapshot {
    match d.read_objects().get(id).unwrap() {
        ObjectSnapshot::Clone(clone) => clone.clone(),
        other => panic!("not a clone: {other:?}"),
    }
}

/// Criterion 37: the golden is format 63; the groups (the invalid link kept and in no group), the
/// coupled own values, the mark and the chain read back and save back unchanged.
#[test]
fn the_golden_reads_back_the_groups_and_marks_and_saves_back_unchanged() {
    assert_eq!(CURRENT_FORMAT_VERSION, 65);
    let bytes = stored(GOLDEN);
    assert_eq!(manifest_format(&bytes), 64);
    let d = unpack(2, &bytes).unwrap();
    let (_, sk, clones) = golden_document();
    let groups: Vec<Option<usize>> = d.exposed_params(sk).iter().map(|p| p.group).collect();
    assert_eq!(groups, [Some(0), Some(0), Some(1), Some(1), None]);
    assert!(
        clone_of(&d, clones[2])
            .chain
            .unlinked
            .contains(&dim("ang_a"))
    );
    assert!(clone_of(&d, clones[3]).chain.unlinked.is_empty());
    assert_eq!(
        clone_of(&d, clones[3]).chain.own_values.get(&dim("len_b")),
        Some(InstanceValue::from_mm(50.0))
    );
    let json = String::from_utf8(d.export_json().unwrap()).unwrap();
    assert!(
        json.replace([' ', '\n'], "").contains("\"link\":42"),
        "the invalid link is kept"
    );
    let reopened = unpack(3, &pack(&d, "0.1.0").unwrap()).unwrap();
    assert_eq!(reopened.export_json().unwrap(), d.export_json().unwrap());
}

/// Criterion 37: an `instance_unlinked` container refuses the file.
#[test]
fn a_container_in_the_unlinked_register_refuses_the_file() {
    let bytes = stored(DAMAGED);
    assert!(matches!(unpack(2, &bytes), Err(OpenError::Damaged)));
}

/// Criterion 37: a file of format 62 opens unchanged, with no group and no mark.
#[test]
fn a_file_of_format_62_opens_unchanged() {
    let bytes = std::fs::read(fixture_path("sketch_instances_v62.curvyo")).unwrap();
    let d = unpack(2, &bytes).unwrap();
    let again = unpack(3, &pack(&d, "0.1.0").unwrap()).unwrap();
    assert_eq!(again.export_json().unwrap(), d.export_json().unwrap());
    let json = String::from_utf8(d.export_json().unwrap()).unwrap();
    assert!(!json.contains("instance_unlinked") && !json.contains("\"link\""));
}

/// Criterion 37: a member of another kind, a group of one and a mark without an own value open,
/// read as stated and save back equal.
#[test]
fn residue_opens_reads_as_stated_and_saves_back_equal() {
    let (d, sk, clones) = golden_document();
    let bytes = edited(&d, |tree| {
        let meta = tree.get_meta(tree_id(sk)).unwrap();
        let items = serde_json::json!([
            {"key": "d:len_a", "name": "Leg 1", "link": "g"},
            {"key": "d:ang_a", "name": "Angle 1", "link": "g"},
            {"key": "d:len_b", "name": "Leg 2", "link": "g"},
            {"key": "d:ang_b", "name": "Angle 2", "link": "solo"},
        ]);
        meta.insert("exposed_params", loro::LoroValue::from(items))
            .unwrap();
        let clone = tree.get_meta(tree_id(clones[0])).unwrap();
        clone
            .insert(
                "instance_unlinked",
                loro::LoroValue::from(serde_json::json!(["d:len_a", "d:ang_b"])),
            )
            .unwrap();
    });
    let opened = unpack(2, &bytes).unwrap();
    let groups: Vec<Option<usize>> = opened.exposed_params(sk).iter().map(|p| p.group).collect();
    assert_eq!(
        groups,
        [Some(0), None, Some(0), None],
        "the angle is of another kind, solo is a group of one"
    );
    assert!(
        clone_of(&opened, clones[0]).chain.unlinked.is_empty(),
        "no own value, and ang_b is in no group"
    );
    let json = String::from_utf8(opened.export_json().unwrap()).unwrap();
    assert!(json.contains("solo") && json.contains("instance_unlinked"));
    let saved = unpack(3, &pack(&opened, "0.1.0").unwrap()).unwrap();
    assert_eq!(saved.export_json().unwrap(), opened.export_json().unwrap());
}
