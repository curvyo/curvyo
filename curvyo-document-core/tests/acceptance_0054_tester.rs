//! Independent file-level tests for `0054-combine-open-paths` criteria 28 to 30: the open or
//! closed state of every outline survives save and reopen, `closed` must be a boolean, a
//! single-node outline opens, `document.json` lists the state, the format version stays, and
//! earlier golden files open unchanged.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::items_after_statements,
    clippy::single_match_else,
    clippy::manual_let_else,
    clippy::cloned_instead_of_copied,
    clippy::cast_precision_loss
)]

use curvyo_document_core::{
    AnchorId, CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, Document, NewAnchor,
    OpenError, Point, pack, unpack,
};
use loro::{LoroDoc, LoroMap, LoroMovableList};
use std::io::{Cursor, Read, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}
fn poly(first: u64, pts: &[(f64, f64)]) -> Vec<NewAnchor> {
    pts.iter()
        .enumerate()
        .map(|(k, &(x, y))| NewAnchor::corner(AnchorId::new(54, first + k as u64), pt(x, y)))
        .collect()
}

fn mixed() -> (Document, curvyo_document_core::NodeId) {
    let d = Document::new(1);
    let seed = d.create_path(&poly(1, &[(500.0, 500.0), (501.0, 500.0)]), false);
    let id = d
        .replace_with_path(
            &[seed],
            seed,
            &[
                (
                    poly(10, &[(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)]),
                    true,
                ),
                (poly(20, &[(5.0, 5.0), (30.0, 8.0), (12.0, 40.0)]), false),
                (poly(30, &[(7.0, 7.0), (7.0, 7.0)]), false),
                (
                    poly(
                        40,
                        &[(10.0, 10.0), (30.0, 10.0), (30.0, 30.0), (10.0, 30.0)],
                    ),
                    true,
                ),
            ],
            "combine_paths",
        )
        .unwrap();
    (d, id)
}

#[test]
fn c28_c29_pack_unpack_keeps_state_nodes_direction_and_order() {
    let (d, id) = mixed();
    let want = d.path(id).unwrap();
    let again = unpack(2, &pack(&d, "0.1.0").unwrap()).unwrap();
    let got = again.path(id).unwrap();
    let states = |p: &curvyo_document_core::PathSnapshot| -> Vec<(bool, Vec<(f64, f64)>)> {
        p.subpaths()
            .map(|s| {
                (
                    s.closed,
                    s.anchors.iter().map(|a| (a.point.x, a.point.y)).collect(),
                )
            })
            .collect()
    };
    assert_eq!(states(&got), states(&want));
    assert_eq!(
        states(&got).iter().map(|s| s.0).collect::<Vec<_>>(),
        vec![true, false, false, true]
    );
}

#[test]
fn c29_the_format_version_is_at_least_14() {
    assert!(CURRENT_FORMAT_VERSION >= 14);
}

#[test]
fn c30_document_json_lists_the_state_of_every_outline() {
    let (d, _) = mixed();
    let text = String::from_utf8(d.export_json().unwrap()).unwrap();
    assert!(text.contains("\"closed\":true") || text.contains("\"closed\": true"));
    assert!(text.contains("\"closed\":false") || text.contains("\"closed\": false"));
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    let mut found = Vec::new();
    fn walk(v: &serde_json::Value, out: &mut Vec<bool>) {
        match v {
            serde_json::Value::Object(m) => {
                for (k, x) in m {
                    if k == "extra_subpaths" {
                        for sp in x.as_array().unwrap() {
                            out.push(
                                sp["closed"]
                                    .as_bool()
                                    .expect("closed is a bool in the view"),
                            );
                        }
                    } else {
                        walk(x, out);
                    }
                }
            }
            serde_json::Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    walk(&v, &mut found);
    assert_eq!(found, vec![false, false, true]);
}

fn damaged_with(edit: impl FnOnce(&LoroMap, &LoroMovableList)) -> Result<Document, OpenError> {
    let (d, _) = mixed();
    let loro = LoroDoc::new();
    loro.set_peer_id(1).unwrap();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let tree = loro.get_tree("paths");
    let meta = tree.get_meta(tree.roots()[0]).unwrap();
    let extras = match meta.get("extra_subpaths") {
        Some(loro::ValueOrContainer::Container(loro::Container::MovableList(l))) => l,
        _ => panic!("extra_subpaths"),
    };
    edit(&meta, &extras);
    loro.commit();
    unpack(
        2,
        &container(
            CURRENT_FORMAT_VERSION,
            &loro.export(loro::ExportMode::Snapshot).unwrap(),
        ),
    )
}

fn outline_map(extras: &LoroMovableList, index: usize) -> LoroMap {
    match extras.get(index) {
        Some(loro::ValueOrContainer::Container(loro::Container::Map(m))) => m,
        _ => panic!("outline map"),
    }
}

#[test]
fn c30_a_closed_register_that_is_not_a_boolean_is_damaged() {
    assert!(damaged_with(|_, _| {}).is_ok());
    for bad in ["yes", "true"] {
        let r = damaged_with(|_, e| outline_map(e, 1).insert("closed", bad).unwrap());
        assert!(matches!(r, Err(OpenError::Damaged)), "string {bad}");
    }
    let r = damaged_with(|_, e| outline_map(e, 0).insert("closed", 1.0).unwrap());
    assert!(matches!(r, Err(OpenError::Damaged)), "number");
    let r = damaged_with(|_, e| outline_map(e, 2).delete("closed").unwrap());
    assert!(matches!(r, Err(OpenError::Damaged)), "absent");
}

fn container(format_version: u32, loro_bytes: &[u8]) -> Vec<u8> {
    let manifest = serde_json::json!({
        "format_version": format_version,
        "loro_snapshot_version": CURRENT_LORO_SNAPSHOT_VERSION,
        "app_version": "0.1.0",
    });
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, bytes) in [
        ("manifest.json", serde_json::to_vec(&manifest).unwrap()),
        ("document.loro", loro_bytes.to_vec()),
        ("document.json", b"{}".to_vec()),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(&bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn member(container: &[u8], name: &str) -> Vec<u8> {
    let mut archive = ZipArchive::new(Cursor::new(container)).unwrap();
    let mut bytes = Vec::new();
    archive
        .by_name(name)
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    bytes
}

#[test]
fn c29_golden_compound_open_has_the_four_documented_outlines() {
    let bytes = fixture("compound_open.curvyo");
    let d = unpack(2, &bytes).unwrap();
    let paths: Vec<_> = d
        .object_ids()
        .into_iter()
        .filter_map(|i| d.path(i))
        .collect();
    let compound: Vec<_> = paths.iter().filter(|p| p.is_compound()).collect();
    assert_eq!(compound.len(), 1);
    let sp: Vec<_> = compound[0].subpaths().collect();
    assert_eq!(
        sp.len(),
        4,
        "closed, open, zero-length open, closed crossing the first"
    );
    assert_eq!(
        sp.iter().map(|s| s.closed).collect::<Vec<_>>(),
        vec![true, false, false, true]
    );
    let z = sp[2].anchors;
    assert!(z.len() >= 2);
    assert!(
        z.iter().all(|a| a.point == z[0].point),
        "outline 3 is zero-length"
    );
    // The crossing outline really crosses the first: some node of 4 lies inside bbox of 1 and some outside.
    let bbox = |a: &[NewAnchor]| {
        let xs: Vec<f64> = a.iter().map(|n| n.point.x).collect();
        let ys: Vec<f64> = a.iter().map(|n| n.point.y).collect();
        (
            xs.iter().cloned().fold(f64::MAX, f64::min),
            ys.iter().cloned().fold(f64::MAX, f64::min),
            xs.iter().cloned().fold(f64::MIN, f64::max),
            ys.iter().cloned().fold(f64::MIN, f64::max),
        )
    };
    let b1 = bbox(sp[0].anchors);
    let inside = sp[3]
        .anchors
        .iter()
        .filter(|n| n.point.x > b1.0 && n.point.x < b1.2 && n.point.y > b1.1 && n.point.y < b1.3)
        .count();
    assert!(
        inside > 0 && inside < sp[3].anchors.len(),
        "outline 4 crosses outline 1"
    );
    // Opening does not rewrite the document; a save and reopen is stable.
    let stored = LoroDoc::new();
    stored.import(&member(&bytes, "document.loro")).unwrap();
    let reread = LoroDoc::new();
    reread.import(&d.export_loro_snapshot().unwrap()).unwrap();
    assert_eq!(reread.get_deep_value(), stored.get_deep_value());
    let again = unpack(3, &pack(&d, "0.1.0").unwrap()).unwrap();
    let id = compound[0].id;
    assert_eq!(
        again
            .path(id)
            .unwrap()
            .subpaths()
            .map(|s| s.closed)
            .collect::<Vec<_>>(),
        vec![true, false, false, true]
    );
}

#[test]
fn c29_earlier_golden_files_open_unchanged() {
    for name in [
        "paths_v2.curvyo",
        "compound_v8.curvyo",
        "groups_v11.curvyo",
        "layers_v12.curvyo",
        "clone_v13.curvyo",
        "clone_overrides_v14.curvyo",
    ] {
        let bytes = fixture(name);
        let d = unpack(2, &bytes).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let stored = LoroDoc::new();
        stored.import(&member(&bytes, "document.loro")).unwrap();
        let reread = LoroDoc::new();
        reread.import(&d.export_loro_snapshot().unwrap()).unwrap();
        assert_eq!(
            reread.get_deep_value(),
            stored.get_deep_value(),
            "{name} rewritten on open"
        );
    }
}

#[test]
fn c30_a_single_node_outline_opens() {
    let d = Document::new(1);
    let seed = d.create_path(&poly(1, &[(500.0, 500.0), (501.0, 500.0)]), false);
    let id = d
        .replace_with_path(
            &[seed],
            seed,
            &[
                (poly(10, &[(0.0, 0.0), (20.0, 0.0), (20.0, 20.0)]), true),
                (poly(20, &[(5.0, 5.0)]), false),
            ],
            "combine_paths",
        )
        .unwrap();
    let again = unpack(2, &pack(&d, "0.1.0").unwrap()).unwrap();
    assert_eq!(again.path(id).unwrap().subpaths().count(), 2);
}
