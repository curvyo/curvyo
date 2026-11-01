//! Independent format-29 checks for 0066 customer revision 4 (horizontal and vertical distance).
//!
//! The version-29 goldens round-trip, the version-28 goldens still open, a file holding the new
//! relations declares a version a version-28 reader refuses, and malformed new records written
//! into the stored Loro snapshot are refused on open.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::needless_pass_by_value,
    clippy::collapsible_if
)]
use std::io::{Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, CURRENT_FORMAT_VERSION, Document, OpenError, Style, pack, unpack,
};
use curvyo_sketch_core::{Constraint, Expression, Quantity, Relation, Sketch};
use loro::{LoroDoc, LoroValue, ValueOrContainer};
use serde_json::{Value, json};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(path).unwrap()
}
/// (name, opens) for every sketch golden of a version.
fn goldens(v: u32) -> Vec<(String, bool)> {
    let invalid = [
        "sketch_checkpoint_invalid",
        "sketch_record_invalid",
        "sketch_page_claim_invalid",
        "sketch_page_assignment_invalid",
        "sketch_label_invalid",
    ];
    let mut out = Vec::new();
    for base in [
        "sketch_plate",
        "sketch_dangling",
        "sketch_slot",
        "sketch_converted",
        "sketch_page_owner",
        "sketch_dimension_label",
    ]
    .iter()
    .chain(invalid.iter())
    {
        out.push((format!("{base}_v{v}.curvyo"), !invalid.contains(base)));
    }
    out
}
fn members(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    (0..zip.len())
        .map(|i| {
            let mut f = zip.by_index(i).unwrap();
            let mut data = Vec::new();
            f.read_to_end(&mut data).unwrap();
            (f.name().unwrap().into_owned(), data)
        })
        .collect()
}
fn repack(parts: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut out);
        for (name, data) in parts {
            zip.start_file(name.as_str(), zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }
    out.into_inner()
}
fn manifest(bytes: &[u8]) -> Value {
    let m = members(bytes);
    serde_json::from_slice(&m.iter().find(|(n, _)| n == "manifest.json").unwrap().1).unwrap()
}
fn relation_kinds(doc: &Document) -> Vec<String> {
    let mut out = Vec::new();
    for id in doc.object_ids() {
        if let Some(s) = doc.sketch(id) {
            for c in s.graph.constraints.values() {
                let v = serde_json::to_value(&c.relation).unwrap();
                if let Some(o) = v.as_object() {
                    out.extend(o.keys().cloned());
                }
            }
        }
    }
    out
}

#[test]
fn v29_goldens_open_as_declared_and_round_trip() {
    for (name, valid) in goldens(29) {
        let bytes = fixture(&name);
        assert_eq!(manifest(&bytes)["format_version"], 29, "{name}");
        let doc = unpack(9301, &bytes);
        assert_eq!(doc.is_ok(), valid, "{name}: {:?}", doc.as_ref().err());
        if let Ok(doc) = doc {
            let again = unpack(9302, &pack(&doc, "rev4 round trip").unwrap()).unwrap();
            assert_eq!(
                again.export_json().unwrap(),
                doc.export_json().unwrap(),
                "{name}: round trip"
            );
        }
    }
    let plate = unpack(9303, &fixture("sketch_plate_v29.curvyo")).unwrap();
    let kinds = relation_kinds(&plate);
    assert!(kinds.iter().any(|k| k == "HorizontalDistance"), "{kinds:?}");
    assert!(kinds.iter().any(|k| k == "VerticalDistance"), "{kinds:?}");
}

#[test]
fn v28_goldens_still_open_unchanged() {
    for (name, valid) in goldens(28) {
        let bytes = fixture(&name);
        assert_eq!(manifest(&bytes)["format_version"], 28, "{name}");
        let doc = unpack(9311, &bytes);
        assert_eq!(doc.is_ok(), valid, "{name}: {:?}", doc.as_ref().err());
    }
    let plate = unpack(9312, &fixture("sketch_plate_v28.curvyo")).unwrap();
    let kinds = relation_kinds(&plate);
    assert!(
        !kinds
            .iter()
            .any(|k| k == "HorizontalDistance" || k == "VerticalDistance")
    );
    // re-saving a version-28 document writes the current version
    let saved = pack(&plate, "rev4 upgrade").unwrap();
    assert_eq!(manifest(&saved)["format_version"], CURRENT_FORMAT_VERSION);
}

#[test]
fn a_file_with_axis_distances_is_too_new_for_a_version_28_reader() {
    assert_eq!(CURRENT_FORMAT_VERSION, 29);
    let doc = Document::new(9321);
    let mut sketch = Sketch::new("origin".into());
    for (id, x) in [("a", 10.0), ("b", 40.0)] {
        sketch.entities.insert(
            id.into(),
            curvyo_sketch_core::Entity::Point {
                position: curvyo_sketch_core::Position::mm(x, 5.0),
                fixed: false,
            },
        );
    }
    sketch.constraints.insert(
        "h".into(),
        Constraint::driving(Relation::HorizontalDistance {
            a: "a".into(),
            b: "b".into(),
            value: Expression::Literal(Quantity::length(30.0).unwrap()),
        }),
    );
    doc.create_sketch(&sketch, Affine::IDENTITY, &Style::default())
        .unwrap();
    let bytes = pack(&doc, "rev4").unwrap();
    let found = manifest(&bytes)["format_version"].as_u64().unwrap();
    assert!(
        found > 28,
        "a version-28 reader's gate must refuse: {found}"
    );
    // the gate itself: a build refuses a manifest one version newer than it supports
    let mut parts = members(&bytes);
    for (name, data) in &mut parts {
        if name == "manifest.json" {
            let mut m: Value = serde_json::from_slice(data).unwrap();
            m["format_version"] = json!(CURRENT_FORMAT_VERSION + 1);
            *data = serde_json::to_vec(&m).unwrap();
        }
    }
    assert!(matches!(
        unpack(9322, &repack(&parts)),
        Err(OpenError::FormatTooNew {
            found: 30,
            supported: 29
        })
    ));
}

/// Rewrites the `guide_run` horizontal-distance record of the version-29 plate golden.
fn tampered(record: Option<Value>) -> Vec<u8> {
    let bytes = fixture("sketch_plate_v29.curvyo");
    let mut parts = members(&bytes);
    let loro_bytes = parts
        .iter()
        .find(|(n, _)| n == "document.loro")
        .unwrap()
        .1
        .clone();
    let doc = LoroDoc::new();
    doc.import(&loro_bytes).unwrap();
    let tree = doc.get_tree("paths");
    let mut hit = 0;
    for node in tree.nodes() {
        let Ok(meta) = tree.get_meta(node) else {
            continue;
        };
        if let Some(ValueOrContainer::Container(loro::Container::Map(c))) =
            meta.get("sketch_constraints")
        {
            if c.get("guide_run").is_some() {
                if let Some(r) = &record {
                    c.insert("guide_run", LoroValue::from(r.clone())).unwrap();
                }
                hit += 1;
            }
        }
    }
    assert!(hit >= 1, "the plate golden holds guide_run");
    doc.commit();
    let snapshot = doc.export(loro::ExportMode::Snapshot).unwrap();
    for (name, data) in &mut parts {
        if name == "document.loro" {
            data.clone_from(&snapshot);
        }
    }
    repack(&parts)
}
fn hd(a: &str, b: &str, value: Value) -> Value {
    json!({"relation": {"HorizontalDistance": {"a": a, "b": b, "value": value}}, "reference": false})
}
fn mm(v: f64) -> Value {
    json!({"Literal": {"value": v, "dimension": "Length"}})
}

/// Open outcome of a tampered plate: `None` when refused, else whether the sketch is stale
/// (its saved solution is not trusted and the record is diagnosed, the established 0066
/// handling of semantically damaged relation records).
fn outcome(bytes: &[u8]) -> Option<bool> {
    let doc = unpack(9333, bytes).ok()?;
    let id = doc
        .object_ids()
        .into_iter()
        .find(|id| {
            doc.sketch(*id)
                .is_some_and(|s| s.graph.constraints.contains_key(&"guide_run".into()))
        })
        .unwrap();
    Some(doc.sketch(id).unwrap().stale)
}

#[test]
fn malformed_axis_distance_records_are_refused_on_open() {
    assert_eq!(
        outcome(&tampered(None)),
        Some(false),
        "untouched rewrite opens"
    );
    assert_eq!(
        outcome(&tampered(Some(hd("guide:p0", "guide:p1", mm(20.0))))),
        Some(false),
        "valid rewritten record opens clean"
    );
    // Semantic damage: refused, or opened with the record diagnosed and the solution untrusted.
    let semantic: Vec<(&str, Value)> = vec![
        ("negative value", hd("guide:p0", "guide:p1", mm(-5.0))),
        ("line operand", hd("guide:line0", "guide:p1", mm(20.0))),
        ("missing operand", hd("guide:p0", "no_such_point", mm(20.0))),
        (
            "angle value",
            hd(
                "guide:p0",
                "guide:p1",
                json!({"Literal": {"value": 0.5, "dimension": "Angle"}}),
            ),
        ),
    ];
    // Structural damage: the record cannot be decoded, so the file is refused.
    let structural: Vec<(&str, Value)> = vec![
        (
            "missing field",
            json!({"relation": {"HorizontalDistance": {"a": "guide:p0", "value": mm(20.0)}}, "reference": false}),
        ),
        (
            "unknown kind",
            json!({"relation": {"DiagonalDistance": {"a": "guide:p0", "b": "guide:p1", "value": mm(20.0)}}, "reference": false}),
        ),
        (
            "non-finite value",
            hd(
                "guide:p0",
                "guide:p1",
                json!({"Literal": {"value": null, "dimension": "Length"}}),
            ),
        ),
    ];
    let mut failures = Vec::new();
    for (what, record) in semantic {
        if outcome(&tampered(Some(record))) == Some(false) {
            failures.push(format!("{what}: opened as a clean sketch"));
        }
    }
    for (what, record) in structural {
        if outcome(&tampered(Some(record))).is_some() {
            failures.push(format!("{what}: opened"));
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}
