//! Format 55 (`specs/0121-print-and-cut` criterion 19, ADR 0081 §9): the goldens
//! `print_and_cut_v55.curvyo` and `print_and_cut_damaged_v55.curvyo` read back as stored, save
//! back unchanged, and a build one version older refuses them. In-crate, because the damaged
//! golden writes registers no command writes. `CURVYO_WRITE_FIXTURES=1` rewrites both.
//!
//! Test-only: declared under `#[cfg(test)]` in `print_and_cut_commands.rs`.

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use crate::document::{CURRENT_FORMAT_VERSION, OBJECTS_TREE};
use crate::manufacturing::NewOutputPage;
use crate::paths::tree_id_of;
use crate::print_and_cut_codec::KEY_PRINT_AND_CUT;
use crate::units::{Angle, Point, Vec2};
use crate::{
    Affine, Document, DocumentSize, OpenError, PrintAndCutMeasures, PrintAndCutRegister, pack,
    unpack,
};

const GOLDEN: &str = "print_and_cut_v55.curvyo";
const DAMAGED: &str = "print_and_cut_damaged_v55.curvyo";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
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

fn read_or_write(name: &str, make: impl Fn() -> Vec<u8>) -> Vec<u8> {
    let path = fixture_path(name);
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(&path, make()).unwrap();
    }
    std::fs::read(path).unwrap()
}

/// The fit the golden is placed by: a turn of 1.8 degrees, a scale of 100.1 % and a move.
fn fit() -> Affine {
    Affine::translation(Vec2::new(83.4, 41.2))
        .compose(Affine::rotation_about(
            Point::new(0.0, 0.0),
            Angle::from_radians(1.8_f64.to_radians()),
        ))
        .compose(Affine::scale(1.001, 1.001))
}

/// A sheet with a 4-mark object, a laser page placing it fitted, 3 measures, tolerance 0.4 mm
/// and a Reset target.
fn golden_document() -> (Document, crate::NodeId) {
    let d = Document::new(1);
    let drawing = d.pages()[0].clone();
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let origin = drawing.origin;
    let page = (
        origin,
        Point::new(
            origin.x + drawing.size.width.as_mm(),
            origin.y + drawing.size.height.as_mm(),
        ),
    );
    d.insert_registration_marks(crate::registration_mark_corners(page, None))
        .unwrap();
    let placement = d
        .place_page(
            &laser.id,
            &drawing.id,
            Affine::translation(Vec2::new(20.0, 0.0)),
        )
        .unwrap();
    // A tolerance step between the measures, so no two adjacent steps share a label (`0020`
    // c36); the last one is 0.4 mm.
    for (n, xy, tolerance) in [
        (1, [93_400, 51_200], 450),
        (2, [283_300, 57_100], 400),
        (3, [274_200, 344_100], 400),
    ] {
        d.set_print_and_cut_measure(placement, n, Some(xy)).unwrap();
        if n < 3 {
            d.set_print_and_cut_tolerance(placement, tolerance).unwrap();
        }
    }
    d.apply_print_and_cut(placement, fit()).unwrap();
    (d, placement)
}

fn golden() -> Vec<u8> {
    read_or_write(GOLDEN, || pack(&golden_document().0, "0.1.0").unwrap())
}

/// Two marks numbered 3, a 1-mark object, a register that is text, a measure that is not an
/// integer.
fn damaged_document() -> Document {
    let (d, placement) = golden_document();
    let tree = d.loro().get_tree(OBJECTS_TREE);
    let marks = d.marks_objects_on(&d.pages()[0]).remove(0);
    let meta = tree.get_meta(tree_id_of(marks.marks[3].id)).unwrap();
    meta.insert("mark_number", 3_i64).unwrap();
    let group = tree.create(loro::TreeParentId::Root).unwrap();
    let group_meta = tree.get_meta(group).unwrap();
    group_meta
        .insert(crate::group_tree::KEY_GROUP, true)
        .unwrap();
    crate::registration_mark_codec::write_marks_tag(&group_meta);
    let lonely = tree.create(loro::TreeParentId::Node(group)).unwrap();
    crate::registration_mark_codec::write_mark(
        &tree.get_meta(lonely).unwrap(),
        &crate::RegistrationMarkSnapshot::new(
            crate::group_tree::node_id(lonely),
            1,
            Point::new(50.0, 50.0),
            crate::Length::from_mm(6.0),
            crate::Length::from_mm(0.25),
        ),
    );
    let output = d.placed_page(placement).unwrap();
    let drawing = output.source.clone().unwrap();
    let text = d
        .place_page(
            &output.output.id,
            &drawing.id,
            Affine::translation(Vec2::new(30.0, 0.0)),
        )
        .unwrap();
    tree.get_meta(tree_id_of(text))
        .unwrap()
        .insert(KEY_PRINT_AND_CUT, "measured")
        .unwrap();
    // A step between the two placements, so no two adjacent steps share a label (`0020` c36).
    d.commit_with_label("damage");
    let fraction = d
        .place_page(
            &output.output.id,
            &drawing.id,
            Affine::translation(Vec2::new(40.0, 0.0)),
        )
        .unwrap();
    let marks: std::collections::HashMap<String, loro::LoroValue> =
        std::collections::HashMap::from([(
            "1".to_owned(),
            loro::LoroValue::from(vec![loro::LoroValue::Double(1.5), loro::LoroValue::I64(2)]),
        )]);
    let value: std::collections::HashMap<String, loro::LoroValue> =
        std::collections::HashMap::from([("marks".to_owned(), loro::LoroValue::from(marks))]);
    tree.get_meta(tree_id_of(fraction))
        .unwrap()
        .insert(KEY_PRINT_AND_CUT, loro::LoroValue::from(value))
        .unwrap();
    d.commit_with_label("damage");
    d
}

fn damaged() -> Vec<u8> {
    read_or_write(DAMAGED, || pack(&damaged_document(), "0.1.0").unwrap())
}

fn format_of(bytes: &[u8]) -> serde_json::Value {
    let manifest: serde_json::Value =
        serde_json::from_slice(&members(bytes)["manifest.json"]).unwrap();
    manifest["format_version"].clone()
}

/// The `document.json` of a save, under the golden's own version: a later bump changes only that
/// number.
fn saved_as_golden(d: &Document) -> String {
    String::from_utf8(members(&pack(d, "0.1.0").unwrap())["document.json"].clone())
        .unwrap()
        .replacen(
            &format!("\"format_version\": {CURRENT_FORMAT_VERSION},"),
            "\"format_version\": 55,",
            1,
        )
}

#[test]
fn the_golden_is_format_55_and_reads_back_marks_measures_and_placement() {
    // `0145` (rounded polygons and stars) is 56 on top.
    const { assert!(CURRENT_FORMAT_VERSION >= 55) };
    let bytes = golden();
    assert_eq!(format_of(&bytes), 55);
    let d = unpack(2, &bytes).unwrap();
    assert_eq!(d.own_step_count(), 0, "opening writes no step");
    let marks = d.marks_objects_on(&d.pages()[0]);
    assert_eq!(marks.len(), 1);
    let numbers: Vec<u8> = marks[0].marks.iter().map(|m| m.number).collect();
    assert_eq!(numbers, [1, 2, 3, 4]);
    let placed = d.placed_pages();
    assert_eq!(placed.len(), 1);
    assert!(placed[0].matrix.approx_eq(fit(), 1e-12));
    let Some(PrintAndCutRegister::Measures(PrintAndCutMeasures {
        marks,
        tolerance_um,
        reset,
    })) = d.print_and_cut(placed[0].id)
    else {
        panic!("measures");
    };
    assert_eq!(marks.len(), 3);
    assert_eq!(marks[&2], [283_300, 57_100]);
    assert_eq!(tolerance_um, 400);
    assert_eq!(reset, Some(Affine::translation(Vec2::new(20.0, 0.0))));
    assert_eq!(
        saved_as_golden(&d).as_bytes(),
        members(&bytes)["document.json"].as_slice(),
        "saves back unchanged"
    );
}

#[test]
fn the_damaged_golden_opens_reads_what_it_can_and_saves_back_unchanged() {
    let bytes = damaged();
    let d = unpack(2, &bytes).unwrap();
    let objects = d.marks_objects_on(&d.pages()[0]);
    assert_eq!(objects.len(), 2);
    let numbers: Vec<u8> = objects[0].marks.iter().map(|m| m.number).collect();
    assert_eq!(
        numbers,
        [1, 2, 3, 3],
        "two marks numbered 3 are read as they are"
    );
    assert_eq!(
        objects[1].marks.len(),
        1,
        "a 1-mark object is read as it is"
    );
    let registers: Vec<PrintAndCutRegister> = d
        .placed_pages()
        .iter()
        .map(|p| d.print_and_cut(p.id).unwrap())
        .collect();
    assert!(matches!(registers[0], PrintAndCutRegister::Measures(_)));
    assert_eq!(registers[1], PrintAndCutRegister::Unreadable, "text");
    assert_eq!(
        registers[2],
        PrintAndCutRegister::Unreadable,
        "not an integer"
    );
    let saved = saved_as_golden(&d);
    assert_eq!(
        saved.as_bytes(),
        members(&bytes)["document.json"].as_slice(),
        "never repaired"
    );
}

#[test]
fn a_build_of_format_54_refuses_the_golden() {
    let bytes = golden();
    let mut files = members(&bytes);
    let mut json: serde_json::Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    json["format_version"] = serde_json::json!(CURRENT_FORMAT_VERSION + 1);
    files.insert("manifest.json".into(), serde_json::to_vec(&json).unwrap());
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, content) in &files {
        writer.start_file(name.as_str(), options).unwrap();
        writer.write_all(content).unwrap();
    }
    let newer = writer.finish().unwrap().into_inner();
    assert!(matches!(
        unpack(3, &newer),
        Err(OpenError::FormatTooNew {
            found: 63,
            supported: 62
        })
    ));
}

#[test]
fn a_damaged_mark_register_refuses_the_file() {
    let (d, _) = golden_document();
    let tree = d.loro().get_tree(OBJECTS_TREE);
    let marks = d.marks_objects_on(&d.pages()[0]).remove(0);
    tree.get_meta(tree_id_of(marks.marks[0].id))
        .unwrap()
        .insert("mark_number", 7_i64)
        .unwrap();
    d.commit_with_label("damage");
    let bytes = pack(&d, "0.1.0").unwrap();
    assert!(
        unpack(3, &bytes).is_err(),
        "a number outside 1 to 4 is refused"
    );
}
