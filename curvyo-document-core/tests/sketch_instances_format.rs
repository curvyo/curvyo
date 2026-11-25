//! The stored registers of sketch instances (`specs/0099-sketch-pages` criterion 30, `adrs.md` A3):
//! the golden of format 61, the damaged-file golden, a file of 60 that opens unchanged and the
//! residue cases (stale keys, a wrong unit, a value outside its limits) that read as stated and
//! save back equal.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation
)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, CURRENT_FORMAT_VERSION, Document, ExposedKey, InstanceValue, InstanceWrite, NodeId,
    ObjectSnapshot, OpenError, Style, ValueSet, pack, unpack,
};
use curvyo_sketch_core as sketch;

const GOLDEN: &str = "sketch_instances_v62.curvyo";
const DAMAGED: &str = "sketch_instances_damaged_v62.curvyo";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn graph() -> sketch::Sketch {
    let mut s = sketch::Sketch::new("origin".into());
    s.entities.insert(
        "end".into(),
        sketch::Entity::Point {
            position: sketch::Position::mm(60.0, 0.0),
            fixed: false,
        },
    );
    s.entities.insert(
        "line".into(),
        sketch::Entity::Line {
            start: "origin".into(),
            end: "end".into(),
        },
    );
    for (id, value) in [("width", 80.0), ("margin", 10.0)] {
        s.parameters.insert(
            id.into(),
            sketch::Parameter::literal(id, sketch::Quantity::length(value).unwrap()),
        );
    }
    let formula = sketch::Expression::parse("width - 2 * margin", &s.parameters).unwrap();
    s.parameters.insert(
        "spacing".into(),
        sketch::Parameter::expression("spacing", sketch::Dimension::Length, formula),
    );
    s.constraints.insert(
        "horizontal".into(),
        sketch::Constraint::driving(sketch::Relation::Horizontal("line".into())),
    );
    s.constraints.insert(
        "length".into(),
        sketch::Constraint::driving(sketch::Relation::Length {
            curve: "line".into(),
            value: sketch::Expression::Reference("spacing".into()),
        }),
    );
    s.constraints.insert(
        "reach".into(),
        sketch::Constraint::driving(sketch::Relation::Distance {
            a: "origin".into(),
            b: "end".into(),
            value: sketch::Expression::Literal(sketch::Quantity::length(60.0).unwrap()),
        }),
    );
    s
}

fn width() -> ExposedKey {
    ExposedKey::parameter(&"width".into())
}

fn reach() -> ExposedKey {
    ExposedKey::dimension(&"reach".into())
}

fn values(pairs: &[(&ExposedKey, f64)]) -> ValueSet {
    let mut set = ValueSet::default();
    for (key, mm) in pairs {
        set.insert((*key).clone(), InstanceValue::from_mm(*mm));
    }
    set
}

fn write(d: &Document, clone: NodeId, own: ValueSet, last: Option<ValueSet>) {
    write_as(d, clone, own, last, "change_parameter");
}

/// [`write`] under another step label, so two steps on one clone never read as a repeated label.
fn write_as(d: &Document, clone: NodeId, own: ValueSet, last: Option<ValueSet>, label: &str) {
    d.write_instance_values(
        &[InstanceWrite {
            clone,
            own,
            last_solvable: last,
            unlinked: curvyo_document_core::KeySet::default(),
        }],
        label,
        None,
    )
    .unwrap();
}

/// A sketch exposing one parameter with limits and one named dimension, and the clones: none own,
/// one own, a chain whose second clone has its own pair, one with no solution and its fallback,
/// and a plain clone of that one.
fn golden_document() -> (Document, Vec<NodeId>) {
    let d = Document::new(1);
    let sk = d
        .create_sketch(&graph(), Affine::IDENTITY, &Style::default())
        .unwrap();
    d.expose_parameter(sk, &width(), None).unwrap();
    d.set_exposed_limits(sk, &width(), Some(50.0), Some(200.0), None)
        .unwrap();
    d.expose_parameter(sk, &reach(), Some("Reach 1")).unwrap();
    let plain = d.linked_clone(&[sk], &[]).unwrap()[0];
    // A step between two linked clones, so no two rows of the history share a label.
    d.set_exposed_limits(sk, &reach(), Some(0.0), Some(500.0), None)
        .unwrap();
    let own = d.linked_clone(&[sk], &[]).unwrap()[0];
    write(
        &d,
        own,
        values(&[(&width(), 100.0), (&reach(), 80.0)]),
        None,
    );
    let chained = d.linked_clone(&[own], &[]).unwrap()[0];
    write(
        &d,
        chained,
        values(&[(&width(), 110.0), (&reach(), 90.0)]),
        None,
    );
    let failed = d.linked_clone(&[sk], &[]).unwrap()[0];
    let good = values(&[(&width(), 100.0), (&reach(), 80.0)]);
    write_as(&d, failed, good.clone(), None, "own_parameters");
    write(
        &d,
        failed,
        values(&[(&width(), 120.0), (&reach(), 80.0)]),
        Some(good),
    );
    let follower = d.linked_clone(&[failed], &[]).unwrap()[0];
    (d, vec![plain, own, chained, failed, follower])
}

/// The golden with `instance_params` of one clone as a container, the wrong register shape.
fn damaged_document() -> Vec<u8> {
    let (d, clones) = golden_document();
    let loro = loro::LoroDoc::new();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    loro.set_peer_id(7).unwrap();
    let tree = loro.get_tree("paths");
    let target = tree_id(clones[1]);
    let meta = tree.get_meta(target).unwrap();
    meta.delete("instance_params").unwrap();
    meta.insert_container("instance_params", loro::LoroMap::new())
        .unwrap();
    loro.commit();
    let mut files = members(&pack(&d, "0.1.0").unwrap());
    files.retain(|(name, _)| name != "document.loro");
    files.push((
        "document.loro".into(),
        loro.export(loro::ExportMode::Snapshot).unwrap(),
    ));
    zip_of(&files)
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
            GOLDEN => std::fs::write(
                fixture_path(GOLDEN),
                pack(&golden_document().0, "0.1.0").unwrap(),
            )
            .unwrap(),
            DAMAGED => std::fs::write(fixture_path(DAMAGED), damaged_document()).unwrap(),
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

fn spans(d: &Document, clones: &[NodeId]) -> Vec<(i64, bool, bool)> {
    let read = d.read_objects();
    clones
        .iter()
        .map(|id| {
            let ObjectSnapshot::Clone(clone) = read.get(*id).unwrap() else {
                panic!("not a clone")
            };
            let xs: Vec<f64> = clone.resolved.anchors.iter().map(|a| a.point.x).collect();
            let span = xs.iter().copied().fold(f64::MIN, f64::max)
                - xs.iter().copied().fold(f64::MAX, f64::min);
            (
                span.round() as i64,
                clone.chain.own_params,
                clone.chain.no_solution,
            )
        })
        .collect()
}

const EXPECTED: [(i64, bool, bool); 5] = [
    (60, false, false),
    (80, true, false),
    (90, true, false),
    (80, true, true),
    (80, false, true),
];

/// Criterion 30: the golden is format 61; the exposed list, the own values, the chain, the saved
/// fallback and the plain clone of the failed one read back and save back unchanged.
#[test]
fn the_golden_reads_back_every_register_and_saves_back_unchanged() {
    const { assert!(CURRENT_FORMAT_VERSION >= 62) };
    let bytes = stored(GOLDEN);
    assert_eq!(manifest_format(&bytes), 61);
    let d = unpack(2, &bytes).unwrap();
    let (fresh, clones) = golden_document();
    assert_eq!(spans(&fresh, &clones), EXPECTED);
    assert_eq!(d.export_json().unwrap(), fresh.export_json().unwrap());
    let reopened = unpack(3, &pack(&d, "0.1.0").unwrap()).unwrap();
    assert_eq!(reopened.export_json().unwrap(), d.export_json().unwrap());
    let json = String::from_utf8(d.export_json().unwrap()).unwrap();
    for key in [
        "exposed_params",
        "instance_params",
        "instance_fallback",
        "Reach 1",
    ] {
        assert!(json.contains(key), "{key}");
    }
}

/// Criterion 30: an `instance_params` container is a wrong register shape and refuses the file.
#[test]
fn a_container_in_a_clone_register_refuses_the_file() {
    let bytes = stored(DAMAGED);
    assert!(matches!(unpack(2, &bytes), Err(OpenError::Damaged)));
}

/// Criterion 30: a file of format 60 opens unchanged and a sketch clone draws as before.
#[test]
fn a_file_of_format_60_opens_unchanged() {
    let bytes = std::fs::read(fixture_path("randomized_v60.curvyo")).unwrap();
    assert_eq!(manifest_format(&bytes), 60);
    let d = unpack(2, &bytes).unwrap();
    let again = unpack(3, &pack(&d, "0.1.0").unwrap()).unwrap();
    assert_eq!(again.export_json().unwrap(), d.export_json().unwrap());
}

/// Criterion 30: stale keys, a wrong unit and a value outside its limits open, read as stated and
/// save back equal.
#[test]
fn residue_opens_reads_as_stated_and_saves_back_equal() {
    let (d, clones) = golden_document();
    let loro = loro::LoroDoc::new();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    loro.set_peer_id(7).unwrap();
    let tree = loro.get_tree("paths");
    let target = tree_id(clones[1]);
    let meta = tree.get_meta(target).unwrap();
    meta.insert(
        "instance_params",
        loro::LoroValue::from(serde_json::json!({
            "p:width": {"mm": 500},
            "d:reach": {"deg": 30},
            "p:gone": {"mm": 5},
            "junk": 4,
        })),
    )
    .unwrap();
    loro.commit();
    let mut files = members(&pack(&d, "0.1.0").unwrap());
    files.retain(|(name, _)| name != "document.loro");
    files.push((
        "document.loro".into(),
        loro.export(loro::ExportMode::Snapshot).unwrap(),
    ));
    let opened = unpack(2, &zip_of(&files)).unwrap();
    // width 500 is clamped to the limit 200 (length 180) while reach (a wrong unit) reads absent.
    let read = opened.read_objects();
    let ObjectSnapshot::Clone(clone) = read.get(clones[1]).unwrap() else {
        panic!("clone")
    };
    assert_eq!(clone.chain.own_values.len(), 1, "only the width counts");
    assert!(clone.chain.own_params);
    let width_now = clone.chain.own_values.get(&width()).unwrap().number();
    assert!(
        (width_now - 200.0).abs() < 1e-9,
        "clamped at read: {width_now}"
    );
    let json = String::from_utf8(opened.export_json().unwrap()).unwrap();
    assert!(
        json.contains("500") && json.contains("p:gone") && json.contains("junk"),
        "kept as stored"
    );
    let saved = unpack(3, &pack(&opened, "0.1.0").unwrap()).unwrap();
    assert_eq!(saved.export_json().unwrap(), opened.export_json().unwrap());
}
