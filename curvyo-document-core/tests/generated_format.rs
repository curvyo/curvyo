//! Format 54 (`specs/0141-parametric-generators` criteria 5 to 7, ADR 0080 §4, §5): the generated
//! leaf, its goldens (`generated_v54`, `generated_unknown_kind_v54`; the damaged one is checked in
//! the crate), open exactly as stored, move writes only the frame, a resize is refused here,
//! node edits are refused, an unknown kind survives a save, Convert to paths. The goldens are
//! rewritten with `CURVYO_WRITE_FIXTURES=1`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use curvyo_document_core::{
    Affine, AnchorId, CURRENT_FORMAT_VERSION, Document, GeneratedRecord, NewAnchor, NodeId,
    ObjectEditError, ObjectSnapshot, Point, Vec2, pack, unpack,
};
use serde_json::json;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn line_square(x: f64, y: f64, side: f64) -> Vec<f64> {
    let (x1, y1) = (x + side, y + side);
    vec![
        x, y, x, y, x1, y, x1, y, x1, y, x1, y1, x1, y1, x1, y1, x, y1, x, y1, x, y1, x, y, x, y,
    ]
}

fn record(kind: &str) -> GeneratedRecord {
    let mut params = serde_json::Map::new();
    params.insert("module".into(), json!({"mm": 2.0}));
    params.insert("teeth".into(), json!({"count": 24}));
    params.insert("keyway".into(), json!({"on": false}));
    params.insert("web".into(), json!({"choice": "solid"}));
    params.insert("pressure_angle".into(), json!({"deg": 20.0}));
    GeneratedRecord {
        kind: kind.into(),
        seed: 4_000_000_001,
        params,
        // One part with a hole (wound the other way), one plain part.
        parts: vec![
            vec![
                line_square(0.0, 0.0, 20.0),
                vec![
                    5.0, 5.0, 5.0, 5.0, 5.0, 15.0, 5.0, 15.0, 5.0, 15.0, 15.0, 15.0, 15.0, 15.0,
                    15.0, 15.0, 15.0, 5.0, 15.0, 5.0, 15.0, 5.0, 5.0, 5.0, 5.0, 5.0,
                ],
            ],
            vec![line_square(30.0, 0.0, 5.0)],
        ],
        part_keys: Vec::new(),
    }
}

fn document(kind: &str) -> (Document, NodeId) {
    let d = Document::new(1);
    let id = d
        .create_generated(&record(kind), Affine::new(0.0, 1.0, -1.0, 0.0, 100.0, 50.0))
        .unwrap();
    (d, id)
}

fn read_or_write(name: &str, make: impl Fn() -> Vec<u8>) -> Vec<u8> {
    let path = fixture(name);
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(&path, make()).unwrap();
    }
    std::fs::read(path).unwrap()
}

fn golden() -> Vec<u8> {
    read_or_write("generated_v54.curvyo", || {
        pack(&document("curvyo.generators/gear").0, "0.1.0").unwrap()
    })
}

fn unknown() -> Vec<u8> {
    read_or_write("generated_unknown_kind_v54.curvyo", || {
        pack(&document("future.generators/spiral").0, "0.1.0").unwrap()
    })
}

fn only_generated(d: &Document) -> NodeId {
    d.read_objects().objects.first().unwrap().id()
}

#[test]
fn the_format_is_54_and_the_golden_opens_exactly_as_stored() {
    // The generated objects came with format 54; `0121` (print and cut) is 55 on top.
    const { assert!(CURRENT_FORMAT_VERSION >= 54) };
    let d = unpack(2, &golden()).unwrap();
    let id = only_generated(&d);
    let stored = d.generated(id).unwrap();
    assert_eq!(stored.record, record("curvyo.generators/gear"));
    assert_eq!(stored.frame, Affine::new(0.0, 1.0, -1.0, 0.0, 100.0, 50.0));
    assert_eq!(d.own_step_count(), 0, "opening writes no step");
    // Saved again, the stored numbers are the same.
    let again = unpack(3, &pack(&d, "0.1.0").unwrap()).unwrap();
    assert_eq!(again.generated(id).unwrap(), stored);
    // The document.json export lists it as a path.
    assert!(d.export_json().is_ok());
}

#[test]
fn it_reads_as_one_compound_path_with_the_frame_applied() {
    let d = unpack(2, &golden()).unwrap();
    let ObjectSnapshot::Path(path) = d.read_objects().objects.first().unwrap().clone() else {
        panic!("a generated object reads as a path");
    };
    assert!(path.closed);
    // The outline, the hole and the second part: three outlines.
    assert_eq!(path.extra_subpaths.len(), 2);
    // The frame turns (x, y) to (-y + 100, x + 50): the first corner (0, 0) goes to (100, 50).
    assert_eq!(path.anchors[0].point, Point::new(100.0, 50.0));
    assert_eq!(path.anchors[1].point, Point::new(100.0, 70.0));
    assert!((path.rotation.as_radians() - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
}

#[test]
fn a_move_writes_only_the_frame_and_the_record_stays_equal() {
    let d = unpack(2, &golden()).unwrap();
    let id = only_generated(&d);
    let before = d.generated(id).unwrap();
    d.translate_objects(&[id], Vec2::new(7.0, -3.0)).unwrap();
    let after = d.generated(id).unwrap();
    assert_eq!(after.record, before.record);
    assert_eq!(after.frame.e, before.frame.e + 7.0);
    assert_eq!(after.frame.f, before.frame.f - 3.0);
    // Through the gesture writer, with the result a Select move draws.
    let object = d.read_objects().objects.first().unwrap().clone();
    d.transform_objects(&[object.translated(Vec2::new(1.0, 1.0))], false)
        .unwrap();
    let moved = d.generated(id).unwrap();
    assert_eq!(moved.record, before.record);
    assert!((moved.frame.e - (before.frame.e + 8.0)).abs() < 1e-9);
    // A turn keeps the frame exactly orthonormal.
    let object = d.read_objects().objects.first().unwrap().clone();
    let turned = object.rotated(
        Point::new(0.0, 0.0),
        curvyo_document_core::Angle::from_radians(0.3),
    );
    d.transform_objects(&[turned], false).unwrap();
    let frame = d.generated(id).unwrap().frame;
    assert!((frame.a * frame.a + frame.b * frame.b - 1.0).abs() < 1e-12);
    assert_eq!(d.generated(id).unwrap().record, before.record);
}

#[test]
fn a_resize_is_not_a_transform_of_a_generated_object() {
    let d = unpack(2, &golden()).unwrap();
    let id = only_generated(&d);
    let ObjectSnapshot::Path(path) = d.read_objects().objects.first().unwrap().clone() else {
        panic!("path");
    };
    let scaled = ObjectSnapshot::Path(path.mapped(Affine::scale(2.0, 2.0)));
    assert_eq!(
        d.transform_objects(&[scaled], false),
        Err(ObjectEditError::GeneratedNotRigid)
    );
    assert_eq!(
        d.generated(id).unwrap().record,
        record("curvyo.generators/gear")
    );
}

#[test]
fn node_edits_of_a_generated_object_are_refused() {
    let d = unpack(2, &golden()).unwrap();
    let id = only_generated(&d);
    let ObjectSnapshot::Path(path) = d.read_objects().objects.first().unwrap().clone() else {
        panic!("path");
    };
    let anchor = path.anchors[0].id;
    assert!(
        d.move_anchors(&[(id, anchor, Point::new(1.0, 1.0))])
            .is_err()
    );
    assert!(d.path(id).is_none() || d.generated(id).is_some());
    assert_eq!(
        d.generated(id).unwrap().record,
        record("curvyo.generators/gear")
    );
}

#[test]
fn an_unknown_kind_opens_draws_moves_copies_and_survives_a_save_unchanged() {
    let d = unpack(2, &unknown()).unwrap();
    let id = only_generated(&d);
    let stored = d.generated(id).unwrap();
    assert_eq!(stored.record.kind, "future.generators/spiral");
    assert!(matches!(
        d.read_objects().objects.first().unwrap(),
        ObjectSnapshot::Path(_)
    ));
    d.translate_objects(&[id], Vec2::new(1.0, 0.0)).unwrap();
    let saved = unpack(3, &pack(&d, "0.1.0").unwrap()).unwrap();
    assert_eq!(saved.generated(id).unwrap().record, stored.record);
    assert_eq!(saved.generated(id).unwrap().frame.e, stored.frame.e + 1.0);
    // Copy and delete.
    let copy = d
        .duplicate_objects(
            &[curvyo_document_core::CopySource {
                id,
                anchor_ids: vec![],
            }],
            Vec2::new(5.0, 5.0),
        )
        .unwrap();
    assert_eq!(d.generated(copy[0]).unwrap().record, stored.record);
    d.delete_objects(&[id]).unwrap();
    assert!(d.generated(id).is_none());
    assert!(d.generated(copy[0]).is_some());
}

#[test]
fn the_golden_says_54_and_a_file_of_53_still_opens() {
    let bytes = golden();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();
    let mut manifest = String::new();
    std::io::Read::read_to_string(
        &mut archive.by_name("manifest.json").unwrap(),
        &mut manifest,
    )
    .unwrap();
    assert!(
        manifest.contains("\"format_version\":54") || manifest.contains("\"format_version\": 54")
    );
    // A file of 53 opens unchanged (ADR 0080 §4).
    assert!(unpack(2, include_bytes!("fixtures/image_adjust_ai_v53.curvyo")).is_ok());
}

#[test]
fn convert_makes_one_path_for_one_part_and_a_group_for_several() {
    let anchors = |ids: u64, side: f64| -> Vec<NewAnchor> {
        (0..4)
            .map(|k| {
                NewAnchor::corner(
                    AnchorId::new(9, ids + k),
                    Point::new(f64::from(u32::try_from(k).unwrap()) * side, 0.0),
                )
            })
            .collect()
    };
    // Several parts: a group under the same id.
    let (d, id) = document("curvyo.generators/box");
    d.convert_generated(
        id,
        &[
            vec![(anchors(100, 5.0), true), (anchors(200, 2.0), true)],
            vec![(anchors(300, 3.0), true)],
        ],
    )
    .unwrap();
    assert!(d.generated(id).is_none());
    let objects = d.read_objects();
    assert_eq!(objects.objects.len(), 2);
    assert_eq!(d.own_step_count(), 2, "create and convert");
    // One part: the node itself becomes a compound path, keeping its id.
    let (d, id) = document("curvyo.generators/gear");
    d.convert_generated(
        id,
        &[vec![(anchors(10, 5.0), true), (anchors(20, 2.0), true)]],
    )
    .unwrap();
    let ObjectSnapshot::Path(path) = d.object(id).unwrap() else {
        panic!("a path");
    };
    assert_eq!(path.extra_subpaths.len(), 1);
    // An id used twice is refused and nothing is written.
    let (d, id) = document("curvyo.generators/gear");
    let twice = vec![vec![(anchors(1, 5.0), true), (anchors(1, 2.0), true)]];
    assert!(d.convert_generated(id, &twice).is_err());
    assert!(d.generated(id).is_some());
}
