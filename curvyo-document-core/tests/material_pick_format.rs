//! Format 66 (`specs/0146-material-test-picks` criterion 17; ADR 0093 decision 9): an operation's
//! `material` link may carry `pick`. The document core keeps it as plain values, so the goldens
//! read back and save unchanged, damaged links included; the laser extension's reading is in
//! `curvyo-laser-core/tests/material_pick_0146.rs`. `CURVYO_WRITE_FIXTURES=1` rewrites them.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::PathBuf;

use curvyo_document_core::{
    Document, DocumentSize, FixtureParams, NewOperation, NewOutputPage, NodeId, pack, unpack,
};
use serde_json::{Map, Value, json};

const GOLDEN: &str = "material_pick_v66.curvyo";
const DAMAGED: &str = "material_pick_damaged_v66.curvyo";
const MATERIAL: &str = "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn map(value: Value) -> Map<String, Value> {
    let Value::Object(map) = value else {
        panic!("a map")
    };
    map
}

fn copy() -> FixtureParams {
    map(json!({
        "id": "generic-laser", "name": "Generic laser", "origin": "bottom_left",
        "max_feed_mm_per_min": 6000, "max_power": 1000, "laser_mode": true,
        "motorised_z": false, "air_assist": false, "output_format": "grbl",
    }))
}

fn laser(d: &Document) -> NodeId {
    d.add_output_page(NewOutputPage {
        process: "curvyo.laser",
        label: "Laser",
        profile: Some(&copy()),
        work_area: DocumentSize::from_mm(400.0, 300.0),
    })
    .unwrap()
    .kind
    .manufacturing()
    .unwrap()
}

fn linked(pick: Option<Value>, extra: Option<(&str, Value)>) -> Value {
    let mut material =
        json!({"id": MATERIAL, "name": "Birch 3 mm", "modified": 1_762_862_400_000_u64});
    if let Some(pick) = pick {
        material["pick"] = pick;
    }
    let mut value = json!({
        "main": {"power_pct": 60.0, "speed_mm_min": 1950, "passes": 1},
        "material": material,
    });
    if let Some((key, extra)) = extra {
        value[key] = extra;
    }
    value
}

fn add(d: &Document, mfg: NodeId, name: &str, settings: Value) {
    let settings = map(settings);
    let id = d
        .add_operation(
            mfg,
            &NewOperation {
                kind: "cut",
                name: "New",
                settings: Some(&settings),
            },
            usize::MAX,
            &[],
        )
        .unwrap();
    assert!(d.rename_operation(id, name).unwrap());
}

/// Four Cut operations linked to Best (no key), Second best, Fastest and Deepest (with a typed
/// number on top).
fn golden_document() -> Document {
    let d = Document::new(1);
    let page = laser(&d);
    add(&d, page, "Cut Best", linked(None, None));
    add(
        &d,
        page,
        "Cut Second",
        linked(Some(json!("second_best")), None),
    );
    add(
        &d,
        page,
        "Cut Fastest",
        linked(Some(json!("fastest")), None),
    );
    add(
        &d,
        page,
        "Cut Deepest",
        linked(
            Some(json!("deepest")),
            Some(("overrides", json!({"main": {"power_pct": 65.0}}))),
        ),
    );
    d
}

/// Three links whose `pick` is `"best"`, an unknown word and a number.
fn damaged_document() -> Document {
    let d = Document::new(1);
    let page = laser(&d);
    add(&d, page, "Cut best", linked(Some(json!("best")), None));
    add(
        &d,
        page,
        "Cut cleanest",
        linked(Some(json!("cleanest")), None),
    );
    add(&d, page, "Cut three", linked(Some(json!(3)), None));
    d
}

fn members(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
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

fn settings(d: &Document) -> Vec<(String, Option<Value>)> {
    let page = d.manufacturings()[0].id;
    d.operations(page)
        .unwrap()
        .into_iter()
        .map(|op| (op.name, op.settings))
        .collect()
}

fn write_on_request() {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        for (name, doc) in [(GOLDEN, golden_document()), (DAMAGED, damaged_document())] {
            std::fs::write(fixture_path(name), pack(&doc, "0.1.0").unwrap()).unwrap();
        }
    }
}

#[test]
fn the_goldens_are_format_66_read_back_and_save_unchanged() {
    write_on_request();
    for (name, expected) in [(GOLDEN, golden_document()), (DAMAGED, damaged_document())] {
        let bytes = std::fs::read(fixture_path(name)).unwrap();
        let manifest: Value = serde_json::from_slice(&members(&bytes)["manifest.json"]).unwrap();
        assert_eq!(manifest["format_version"], 66, "{name}");
        let d = unpack(2, &bytes).unwrap();
        assert_eq!(d.own_step_count(), 0, "opening writes no step");
        assert_eq!(settings(&d), settings(&expected), "{name}");
        let saved = pack(&d, "0.1.0").unwrap();
        assert_eq!(
            members(&saved)["document.loro"],
            members(&bytes)["document.loro"],
            "{name}: saved back unchanged"
        );
    }
}
