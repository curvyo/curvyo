//! Format 70 (`specs/0144-qr-and-barcodes` slice 4, ADR 0091 "Per-part paint"): the record's
//! `part_paints`. The golden `generated_part_paint_v70.curvyo` holds a three-part code (Background
//! first, finders last) with the finders detached into their own operation; the damaged one
//! (`generated_part_paint_damaged_v70.curvyo`, a paint key with no part) is checked in the crate.
//! `CURVYO_WRITE_FIXTURES=1` rewrites the golden.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, CURRENT_FORMAT_VERSION, Color, Document, DocumentSize, GeneratedRecord, NewOperation,
    NewOutputPage, ObjectSnapshot, OpenError, PartError, PartPaint, pack, unpack,
};

const GOLDEN: &str = "generated_part_paint_v70.curvyo";

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn square(at: f64, side: f64) -> Vec<f64> {
    let (a, b) = (at, at + side);
    vec![
        a, a, a, a, b, a, b, a, b, a, b, b, b, b, a, b, a, b, a, b, a, b, a, a, a, a,
    ]
}

fn record() -> GeneratedRecord {
    let mut part_paints = BTreeMap::new();
    part_paints.insert(
        "background".to_owned(),
        PartPaint {
            fill: Color {
                r: 255,
                g: 255,
                b: 255,
            },
            job: false,
        },
    );
    part_paints.insert(
        "finder".to_owned(),
        PartPaint {
            fill: Color { r: 200, g: 0, b: 0 },
            job: true,
        },
    );
    GeneratedRecord {
        kind: "curvyo.codes/qr".into(),
        seed: 5,
        params: serde_json::Map::new(),
        parts: vec![
            vec![square(-2.0, 20.0)],
            vec![square(0.0, 4.0), square(6.0, 4.0)],
            vec![square(10.0, 4.0)],
        ],
        part_keys: vec!["background".into(), "code".into(), "finder".into()],
        part_paints,
    }
}

fn golden_bytes() -> Vec<u8> {
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let m = laser.kind.manufacturing().unwrap();
    let code = d.create_generated(&record(), Affine::IDENTITY).unwrap();
    let new = NewOperation {
        kind: "engrave",
        name: "Engrave",
        settings: None,
    };
    let engrave = d.add_operation(m, &new, 0, &[vec![code]]).unwrap();
    d.detach_part(code, "finder", "Finder", &[engrave], &[])
        .unwrap();
    pack(&d, "0.1.0").unwrap()
}

fn golden() -> Vec<u8> {
    let path = fixture(GOLDEN);
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(&path, golden_bytes()).unwrap();
    }
    std::fs::read(path).unwrap()
}

fn with_version(bytes: &[u8], version: u64) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let name = file.name().unwrap().to_string();
        let mut content = Vec::new();
        file.read_to_end(&mut content).unwrap();
        if name == "manifest.json" {
            let mut json: serde_json::Value = serde_json::from_slice(&content).unwrap();
            json["format_version"] = serde_json::json!(version);
            content = serde_json::to_vec(&json).unwrap();
        }
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&content).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn manifest_version(bytes: &[u8]) -> u64 {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut text = String::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    serde_json::from_str::<serde_json::Value>(&text).unwrap()["format_version"]
        .as_u64()
        .unwrap()
}

#[test]
fn a_file_with_part_paints_is_format_70_and_opens_exactly_as_stored() {
    let bytes = golden();
    assert_eq!(manifest_version(&bytes), 70, "the golden is a 70 file");
    const { assert!(CURRENT_FORMAT_VERSION >= 70) };
    let d = unpack(2, &bytes).unwrap();
    assert_eq!(d.own_step_count(), 0, "opening writes nothing");
    let id = d.read_objects().objects.first().unwrap().id();
    assert_eq!(d.generated(id).unwrap().record, record());
    assert_eq!(d.detached_parts(id), ["finder"]);
    // Saved again, the same record.
    let again = unpack(3, &pack(&d, "0.1.0").unwrap()).unwrap();
    assert_eq!(again.generated(id).unwrap().record, record());
}

#[test]
fn a_build_that_stops_at_69_refuses_the_file_by_its_version() {
    // A reader of 69 compares with its own 69: a file of 70 is newer, so it is refused whole and
    // never drawn as a solid square over the code. This build plays it one version up.
    const OLD_BUILD: u64 = 69;
    assert!(manifest_version(&golden()) > OLD_BUILD);
    let newer = with_version(&golden(), u64::from(CURRENT_FORMAT_VERSION) + 1);
    assert!(matches!(
        unpack(2, &newer),
        Err(OpenError::FormatTooNew { .. })
    ));
}

#[test]
fn a_file_without_part_paints_opens_unchanged_and_writes_nothing() {
    let bytes = std::fs::read(fixture("generated_part_ops_v65.curvyo")).unwrap();
    let d = unpack(2, &bytes).unwrap();
    let id = d.read_objects().objects.first().unwrap().id();
    assert_eq!(d.generated(id).unwrap().record.part_paints.len(), 0);
    let ObjectSnapshot::Path(path) = d.object(id).unwrap() else {
        panic!("a generated object reads as a path")
    };
    assert_eq!(path.paint_runs.len(), 0);
    assert_eq!(d.own_step_count(), 0);
}

#[test]
fn the_outline_carries_one_run_per_painted_part_in_part_order() {
    let d = Document::new(1);
    let id = d.create_generated(&record(), Affine::IDENTITY).unwrap();
    let ObjectSnapshot::Path(path) = d.object(id).unwrap() else {
        panic!("a path")
    };
    let runs: Vec<(u32, u32, bool)> = path
        .paint_runs
        .iter()
        .map(|run| (run.first, run.count, run.paint.job))
        .collect();
    assert_eq!(runs, [(0, 1, false), (3, 1, true)]);
    // The groups cover every outline in order: background, the code in the node's style, finders.
    let groups: Vec<(usize, usize, bool)> = path
        .paint_groups()
        .iter()
        .map(|g| (g.first, g.count, g.own_style))
        .collect();
    assert_eq!(groups, [(0, 1, false), (1, 2, true), (3, 1, false)]);
    // The background reaches no job; the other three outlines do.
    assert_eq!(path.job_subpaths().count(), 3);
    assert_eq!(path.subpaths().count(), 4);
    let background = &path.paint_groups()[0].style;
    assert!(background.fill.enabled && !background.stroke.enabled);
}

#[test]
fn detach_part_refuses_a_part_that_never_reaches_a_job() {
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let m = laser.kind.manufacturing().unwrap();
    let code = d.create_generated(&record(), Affine::IDENTITY).unwrap();
    let new = NewOperation {
        kind: "engrave",
        name: "Engrave",
        settings: None,
    };
    let engrave = d.add_operation(m, &new, 0, &[vec![code]]).unwrap();
    assert_eq!(
        d.detach_part(code, "background", "Background", &[engrave], &[]),
        Err(PartError::NoSuchPart)
    );
    assert_eq!(d.detached_parts(code).len(), 0);
}
