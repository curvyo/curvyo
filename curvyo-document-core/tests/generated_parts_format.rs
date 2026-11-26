//! Format 65 (`specs/0147-rounded-box` criterion 15, ADR 0082 §10): the record's part keys and
//! `member.<node>#<part>` memberships. The golden `generated_part_ops_v65.curvyo` holds a rounded
//! box with the lid detached into a second operation and a dangling `#ring` key (Lid seat off);
//! the damaged one (`generated_part_keys_damaged_v65.curvyo`) is checked in the crate. The goldens
//! are rewritten with `CURVYO_WRITE_FIXTURES=1`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, CURRENT_FORMAT_VERSION, Document, DocumentSize, GeneratedRecord, NewOperation,
    NewOutputPage, OpenError, pack, unpack,
};

const GOLDEN: &str = "generated_part_ops_v65.curvyo";

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn square(at: f64) -> Vec<f64> {
    let (a, b) = (at, at + 4.0);
    vec![
        a, a, a, a, b, a, b, a, b, a, b, b, b, b, a, b, a, b, a, b, a, b, a, a, a, a,
    ]
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
    let record = GeneratedRecord {
        kind: "curvyo.generators/rounded-box".into(),
        seed: 5,
        params: serde_json::Map::new(),
        parts: vec![vec![square(0.0)], vec![square(10.0)], vec![square(20.0)]],
        part_keys: vec!["strip".into(), "base".into(), "lid".into()],
    };
    let boxed = d.create_generated(&record, Affine::IDENTITY).unwrap();
    let new = NewOperation {
        kind: "cut",
        name: "Cut 3 mm",
        settings: None,
    };
    let cut = d.add_operation(m, &new, 0, &[vec![boxed]]).unwrap();
    d.detach_part(boxed, "lid", "Lid", &[cut], &[]).unwrap();
    // The ring does not exist (Lid seat off): its key stays, ignored.
    let second = d.operations(m).unwrap()[1].id;
    d.set_operation_members(second, &[vec![boxed.with_part("ring").unwrap()]], true)
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
fn a_file_with_a_detached_part_is_format_65_and_opens_exactly_as_stored() {
    let bytes = golden();
    assert_eq!(manifest_version(&bytes), 65, "the golden is a 65 file");
    const { assert!(CURRENT_FORMAT_VERSION >= 65) };
    let d = unpack(2, &bytes).unwrap();
    assert_eq!(d.own_step_count(), 0, "opening writes nothing");
    let boxed = d.read_objects().objects.first().unwrap().id();
    assert_eq!(
        d.generated(boxed).unwrap().record.part_keys,
        ["strip", "base", "lid"]
    );
    let mut parts = d.detached_parts(boxed);
    parts.sort();
    assert_eq!(parts, ["lid", "ring"], "the dangling #ring key is kept");
    // Saved again, the same keys and the same memberships.
    let again = unpack(3, &pack(&d, "0.1.0").unwrap()).unwrap();
    let mut parts = again.detached_parts(boxed);
    parts.sort();
    assert_eq!(parts, ["lid", "ring"]);
}

#[test]
fn a_build_that_stops_at_64_refuses_the_file_by_its_version() {
    // The reader of 64 compares `format_version` with its own 64 (ADR 0033 §7): a file of 65 is
    // newer, so it is refused whole and never opened with the part keys read as dangling ones.
    const OLD_BUILD: u64 = 64;
    assert!(manifest_version(&golden()) > OLD_BUILD);
    // This build plays the check one version up: a file one version newer is refused.
    let bytes = golden();
    let mut archive = zip::ZipArchive::new(Cursor::new(&bytes[..])).unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let name = file.name().unwrap().to_string();
        let mut content = Vec::new();
        file.read_to_end(&mut content).unwrap();
        if name == "manifest.json" {
            let mut json: serde_json::Value = serde_json::from_slice(&content).unwrap();
            json["format_version"] = serde_json::json!(CURRENT_FORMAT_VERSION + 1);
            content = serde_json::to_vec(&json).unwrap();
        }
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&content).unwrap();
    }
    let newer = writer.finish().unwrap().into_inner();
    assert!(matches!(
        unpack(2, &newer),
        Err(OpenError::FormatTooNew { .. })
    ));
}

#[test]
fn a_file_of_64_without_part_keys_opens_unchanged_and_writes_nothing() {
    let bytes = std::fs::read(fixture("generated_v54.curvyo")).unwrap();
    let d = unpack(2, &bytes).unwrap();
    let id = d.read_objects().objects.first().unwrap().id();
    assert_eq!(
        d.generated(id).unwrap().record.part_keys,
        Vec::<String>::new()
    );
    assert_eq!(d.detached_parts(id), Vec::<String>::new());
    assert_eq!(d.own_step_count(), 0);
}
