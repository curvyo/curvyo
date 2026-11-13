//! Format 45 (`specs/0114-image-trace-mask` criterion 22, ADR 0061 §7, §11, §12): the goldens
//! `trace_mask_v45.curvyo` and `trace_mask_damaged_v45.curvyo` read back every part, region and
//! setting, the damaged one as decision 7 says, and both save back with equal content; every older
//! golden opens with no mask; a format 47 file is refused. `CURVYO_WRITE_FIXTURES=1` rewrites the
//! goldens of this file only; `trace_mask_clone_v45.curvyo` holds a linked clone and an unlinked
//! copy of a masked image (criterion 15).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

use curvyo_document_core::{
    Affine, CURRENT_FORMAT_VERSION, Document, ImplicitTracePart, NewImage, NewTraceRegion, NodeId,
    ObjectSnapshot, OpenError, TracePartRef, TracePartSettings, TraceRegionKind, UnitPoint, pack,
    unpack,
};
use loro::{LoroDoc, LoroValue, TreeParentId};
use serde_json::{Value, json};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn read_or_write(name: &str, build: impl Fn() -> Vec<u8>) -> Vec<u8> {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(fixture_path(name), build()).unwrap();
    }
    std::fs::read(fixture_path(name)).unwrap()
}

fn settings(value: Value) -> TracePartSettings {
    serde_json::from_value(value).unwrap()
}

fn points(list: &[(f64, f64)]) -> Vec<UnitPoint> {
    list.iter().map(|&(u, v)| UnitPoint::new(u, v)).collect()
}

fn region(kind: TraceRegionKind, subtract: bool, list: &[(f64, f64)]) -> NewTraceRegion {
    NewTraceRegion {
        kind,
        subtract,
        points: points(list),
    }
}

/// ADR 0061 §12: M1 (rotated and skewed) with part `0` holding R1, L1 and a subtracted rectangle,
/// part 2 Outside with a lasso and other settings, a second image untouched.
fn golden_document() -> Document {
    let d = Document::new(1);
    let turned = Affine::new(86.6, 50.0, -40.0, 86.6, 20.0, 10.0);
    // One import of both pictures, so no two adjacent steps share a label (the legacy-history
    // oracle of `0020`).
    let ids = d
        .add_images(&[
            NewImage {
                bytes: Arc::from(&b"m1 picture"[..]),
                placement: turned,
                name: "M1".into(),
            },
            NewImage {
                bytes: Arc::from(&b"m2 picture"[..]),
                placement: Affine::new(100.0, 0.0, 0.0, 100.0, 150.0, 0.0),
                name: "M2".into(),
            },
        ])
        .unwrap();
    let m1 = ids[0];
    let first = ImplicitTracePart {
        outside: false,
        settings: settings(json!({"threshold": 50, "smooth": 80, "method": "brightness"})),
    };
    let rect = TraceRegionKind::Rect;
    let lasso = TraceRegionKind::Lasso;
    d.draw_trace_region(
        m1,
        TracePartRef::Implicit(&first),
        &region(rect, false, &[(0.0, 0.0), (0.5, 1.0)]),
    )
    .unwrap();
    d.draw_trace_region(
        m1,
        TracePartRef::Stored("0"),
        &region(lasso, false, &[(0.0, 0.0), (0.6, 0.0), (0.0, 1.0)]),
    )
    .unwrap();
    d.draw_trace_region(
        m1,
        TracePartRef::Stored("0"),
        &region(rect, true, &[(0.2, 0.2), (0.3, 0.35)]),
    )
    .unwrap();
    let two = d
        .add_trace_part(
            m1,
            &first,
            &settings(
                json!({"threshold": null, "smooth": 20, "method": "pick_colours",
                             "swatches": ["#ff0000"], "output": "multi", "colours": 4}),
            ),
        )
        .unwrap();
    d.draw_trace_region(
        m1,
        TracePartRef::Stored(&two),
        &region(
            lasso,
            false,
            &[(0.6, 0.6), (0.95, 0.65), (0.8, 0.95), (0.62, 0.9)],
        ),
    )
    .unwrap();
    d.set_trace_part_outside(m1, &two, true).unwrap();
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

fn rezip(files: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, content) in files {
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(content).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

/// The raw meta maps of the image nodes, by name.
fn image_metas(bytes: &[u8]) -> BTreeMap<String, Value> {
    let loro = LoroDoc::new();
    loro.import(&members(bytes)["document.loro"]).unwrap();
    let tree = loro.get_tree("paths");
    let mut out = BTreeMap::new();
    let mut stack = tree.children(TreeParentId::Root).unwrap();
    while let Some(node) = stack.pop() {
        let meta = tree.get_meta(node).unwrap();
        if meta.get("shape").and_then(|v| v.into_value().ok()) == Some(LoroValue::from("image")) {
            let value = serde_json::to_value(meta.get_deep_value()).unwrap();
            let name = value["name"].as_str().unwrap_or_default().to_owned();
            out.insert(name, value);
        }
        stack.extend(tree.children(TreeParentId::Node(node)).unwrap_or_default());
    }
    out
}

fn image_named(d: &Document, name: &str) -> NodeId {
    d.read_objects()
        .objects
        .iter()
        .find_map(|object| match object {
            ObjectSnapshot::Image(image)
                if d.node_registers(image.id).and_then(|r| r.name).as_deref() == Some(name) =>
            {
                Some(image.id)
            }
            _ => None,
        })
        .unwrap()
}

/// The `document.json` entry named `name`, at any depth.
fn find_named(objects: &Value, name: &str) -> Option<Value> {
    objects.as_array()?.iter().find_map(|o| {
        if o["name"] == json!(name) {
            Some(o.clone())
        } else {
            find_named(&o["objects"], name)
        }
    })
}

/// The damage of ADR 0061 §12, written raw into M1's meta map.
fn damaged_bytes() -> Vec<u8> {
    let bytes = pack(&golden_document(), "0.1.0").unwrap();
    let loro = LoroDoc::new();
    loro.import(&members(&bytes)["document.loro"]).unwrap();
    let tree = loro.get_tree("paths");
    let raw = |value: Value| -> LoroValue { serde_json::from_value(value).unwrap() };
    for node in tree.children(TreeParentId::Root).unwrap() {
        let mut stack = vec![node];
        while let Some(node) = stack.pop() {
            let meta = tree.get_meta(node).unwrap();
            if meta.get("name").and_then(|v| v.into_value().ok()) == Some(LoroValue::from("M1")) {
                let region = |kind: &str, points: Value| {
                    raw(json!({"order": 9, "kind": kind, "subtract": false, "points": points}))
                };
                let damage = [
                    (
                        "trace_region.0.two@9",
                        region("lasso", json!([0.0, 0.0, 1.0, 1.0])),
                    ),
                    (
                        "trace_region.0.far@9",
                        region("rect", json!([0.0, 0.0, 1001.0, 1.0])),
                    ),
                    (
                        "trace_region.0.oval@9",
                        region("ellipse", json!([0.0, 0.0, 1.0, 1.0])),
                    ),
                    ("trace_region.0.text@9", region("rect", json!("0 0 1 1"))),
                    (
                        "trace_region.lost@9.d@9",
                        region("rect", json!([0.0, 0.0, 1.0, 1.0])),
                    ),
                    ("trace_part.bad@9", raw(json!("Outside"))),
                    (
                        "trace_settings.0",
                        raw(json!({"threshold": 300, "future_key": "kept"})),
                    ),
                    ("trace_future", raw(json!({"a": 1}))),
                ];
                for (key, value) in damage {
                    meta.insert(key, value).unwrap();
                }
                let nan = LoroValue::from(
                    [
                        ("order".to_owned(), LoroValue::I64(9)),
                        ("kind".to_owned(), LoroValue::from("rect")),
                        ("subtract".to_owned(), LoroValue::Bool(false)),
                        (
                            "points".to_owned(),
                            LoroValue::from(vec![
                                LoroValue::Double(f64::NAN),
                                LoroValue::Double(0.0),
                                LoroValue::Double(1.0),
                                LoroValue::Double(1.0),
                            ]),
                        ),
                    ]
                    .into_iter()
                    .collect::<std::collections::HashMap<_, _>>(),
                );
                meta.insert("trace_region.0.nan@9", nan).unwrap();
            }
            stack.extend(tree.children(TreeParentId::Node(node)).unwrap_or_default());
        }
    }
    loro.commit();
    let mut files = members(&bytes);
    files.insert(
        "document.loro".to_owned(),
        loro.export(loro::ExportMode::Snapshot).unwrap(),
    );
    rezip(&files)
}

#[test]
fn c22_the_golden_reads_back_every_part_region_and_setting() {
    let bytes = read_or_write("trace_mask_v45.curvyo", || {
        pack(&golden_document(), "0.1.0").unwrap()
    });
    let d = unpack(3, &bytes).unwrap();
    let m1 = image_named(&d, "M1");
    let mask = d.trace_mask(m1).unwrap();
    assert_eq!(mask.parts.len(), 2);
    let (first, second) = (&mask.parts[0], &mask.parts[1]);
    assert_eq!(first.id, "0");
    assert!(!first.outside && second.outside);
    assert_eq!(first.settings["threshold"], json!(50));
    assert_eq!(second.settings["method"], json!("pick_colours"));
    assert_eq!(second.settings["threshold"], Value::Null);
    let kinds: Vec<(TraceRegionKind, bool)> =
        first.regions.iter().map(|r| (r.kind, r.subtract)).collect();
    assert_eq!(
        kinds,
        [
            (TraceRegionKind::Rect, false),
            (TraceRegionKind::Lasso, false),
            (TraceRegionKind::Rect, true),
        ]
    );
    assert_eq!(
        first.regions[1].points,
        points(&[(0.0, 0.0), (0.6, 0.0), (0.0, 1.0)])
    );
    assert_eq!(second.regions[0].points.len(), 4);
    assert!(d.trace_mask(image_named(&d, "M2")).unwrap().is_empty());
    // It is what this build writes, and it saves back with the same keys.
    assert_eq!(
        d.trace_mask(m1),
        golden_document().trace_mask(image_named(&golden_document(), "M1"))
    );
    assert_eq!(
        image_metas(&pack(&d, "0.1.0").unwrap()),
        image_metas(&bytes)
    );
    let json: Value = serde_json::from_slice(&d.export_json().unwrap()).unwrap();
    assert_eq!(json["format_version"], json!(46));
    let listed = find_named(&json["objects"], "M1").unwrap();
    assert_eq!(
        listed["trace_mask"]["parts"][0]["regions"][0]["points"],
        json!([0.0, 0.0, 0.5, 1.0])
    );
}

#[test]
fn c22_the_damaged_golden_opens_reads_as_decision_7_and_saves_back_equal() {
    let bytes = read_or_write("trace_mask_damaged_v45.curvyo", damaged_bytes);
    let d = unpack(3, &bytes).unwrap();
    let m1 = image_named(&d, "M1");
    let mask = d.trace_mask(m1).unwrap();
    let parts: Vec<(&str, i64, bool)> = mask
        .parts
        .iter()
        .map(|p| (p.id.as_str(), p.order, p.outside))
        .collect();
    assert_eq!(
        parts.len(),
        3,
        "a part key that is not a map is a part, Inside at order 0"
    );
    assert_eq!(&parts[..2], [("0", 0, false), ("bad@9", 0, false)]);
    assert_eq!((parts[2].1, parts[2].2), (1, true));
    assert_eq!(
        mask.parts[0].regions.len(),
        3,
        "only the three regions that read"
    );
    assert_eq!(
        mask.parts[0].settings["threshold"],
        json!(300),
        "kept as stored"
    );
    assert_eq!(mask.parts[0].settings["future_key"], json!("kept"));
    assert_eq!(
        image_metas(&pack(&d, "0.1.0").unwrap()),
        image_metas(&bytes),
        "nothing is repaired on read or save"
    );
}

#[test]
fn c22_every_older_golden_opens_with_no_mask() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut opened = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if !name.ends_with(".curvyo") || name.starts_with("trace_mask") {
            continue;
        }
        let Ok(d) = unpack(4, &std::fs::read(&path).unwrap()) else {
            continue;
        };
        for object in d.read_objects().objects.iter() {
            if let ObjectSnapshot::Image(image) = object {
                assert!(d.trace_mask(image.id).unwrap().is_empty(), "{name}");
            }
        }
        opened += 1;
    }
    assert!(opened > 50, "{opened} goldens");
}

#[test]
fn c22_a_file_of_format_47_is_refused() {
    assert_eq!(CURRENT_FORMAT_VERSION, 46);
    let bytes = pack(&golden_document(), "0.1.0").unwrap();
    let mut files = members(&bytes);
    let mut manifest: Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    assert_eq!(manifest["format_version"], json!(46));
    manifest["format_version"] = json!(CURRENT_FORMAT_VERSION + 1);
    files.insert(
        "manifest.json".to_owned(),
        serde_json::to_vec(&manifest).unwrap(),
    );
    assert!(matches!(
        unpack(2, &rezip(&files)),
        Err(OpenError::FormatTooNew {
            found: 47,
            supported: 46
        })
    ));
}

/// ADR 0061 §8 and §12, `0114` criterion 15: a linked clone of a masked image stores no mask of
/// its own (it shows its root's); a clone unlinked becomes a copy of the root image with a copy of
/// the mask (ADR 0045 §5 via `copy_subtree`); deleting a clone leaves the root's mask.
fn clone_document() -> Document {
    let d = Document::new(1);
    let m1 = d
        .add_images(&[NewImage {
            bytes: Arc::from(&b"m1 picture"[..]),
            placement: Affine::new(100.0, 0.0, 0.0, 100.0, 0.0, 0.0),
            name: "M1".into(),
        }])
        .unwrap()[0];
    let first = ImplicitTracePart {
        outside: false,
        settings: settings(json!({"threshold": 50})),
    };
    d.draw_trace_region(
        m1,
        TracePartRef::Implicit(&first),
        &region(
            TraceRegionKind::Lasso,
            false,
            &[(0.0, 0.0), (0.6, 0.0), (0.0, 1.0)],
        ),
    )
    .unwrap();
    d.linked_clone(&[m1], &[]).unwrap();
    let unlinked = d.linked_clone(&[m1], &[]).unwrap()[0];
    d.unlink_clones(&[curvyo_document_core::CopySource {
        id: unlinked,
        anchor_ids: Vec::new(),
    }])
    .unwrap();
    d
}

#[test]
fn c15_the_clone_golden_keeps_the_mask_on_the_root_and_on_the_unlinked_copy() {
    let bytes = read_or_write("trace_mask_clone_v45.curvyo", || {
        pack(&clone_document(), "0.1.0").unwrap()
    });
    let d = unpack(2, &bytes).unwrap();
    let images: Vec<NodeId> = d
        .read_objects()
        .objects
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Image(image) => Some(image.id),
            _ => None,
        })
        .collect();
    assert_eq!(images.len(), 2, "the root and the unlinked copy");
    let masks: Vec<_> = images.iter().map(|&id| d.trace_mask(id).unwrap()).collect();
    assert_eq!(
        masks[0], masks[1],
        "the unlinked copy keeps a copy of the mask"
    );
    assert_eq!(masks[0].parts[0].regions.len(), 1);
    let clone = d
        .read_objects()
        .objects
        .iter()
        .find_map(|o| match o {
            ObjectSnapshot::Clone(c) => Some(c.id()),
            _ => None,
        })
        .expect("the linked clone");
    assert_eq!(
        d.trace_mask(clone),
        None,
        "a clone stores no mask of its own"
    );
    // Saved back with equal content.
    let again = pack(&d, "0.1.0").unwrap();
    assert_eq!(image_metas(&again), image_metas(&bytes));
    // Deleting the clone leaves the root's mask.
    d.delete_objects(&[clone]).unwrap();
    assert_eq!(d.trace_mask(images[0]).unwrap(), masks[0]);
}
