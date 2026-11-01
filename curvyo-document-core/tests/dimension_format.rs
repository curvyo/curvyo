//! The stored shape of dimension objects (`specs/0070-measure-and-dimension` criterion 16, ADR
//! 0027 §2 and §8): the golden `dimension_v31.curvyo` with every kind and every slot form, and one
//! damaged file per damage rule. `CURVYO_WRITE_FIXTURES=1` rewrites the fixtures.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::io::{Cursor, Read};
use std::path::PathBuf;

use curvyo_document_core::{
    Angle, CURRENT_FORMAT_VERSION, CurveSlot, DimensionKind, DimensionPlace, DimensionSlots,
    Document, EllipseFrame, Length, NewAnchor, NodeId, ObjectSnapshot, OpenError, Point, PointSlot,
    RectBounds, Shape, ShapeFeature, pack, unpack,
};
use loro::{LoroDoc, LoroValue};

/// The version this slice takes; a rebase onto another bump renumbers it and the fixtures.
const THIS_SLICE_VERSION: u32 = 31;
const GOLDEN: &str = "dimension_v31.curvyo";
const DAMAGED: [&str; 4] = [
    "dimension_kind_unknown_v31.curvyo",
    "dimension_slots_count_v31.curvyo",
    "dimension_slot_malformed_v31.curvyo",
    "dimension_value_nan_v31.curvyo",
];

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn aid(n: u64) -> curvyo_document_core::AnchorId {
    curvyo_document_core::AnchorId::new(31, n)
}

fn place() -> DimensionPlace {
    DimensionPlace::at_offset(Length::from_mm(6.0))
}

fn anchor(node: NodeId, n: u64) -> PointSlot {
    PointSlot::Anchor {
        node,
        anchor: aid(n),
    }
}

/// A short path of its own: a step between two dimensions.
fn separator(document: &Document, first: u64) {
    let y = 150.0 + f64::from(u32::try_from(first).unwrap());
    let _ = document.create_path(
        &[
            NewAnchor::corner(aid(first), Point::new(0.0, y)),
            NewAnchor::corner(aid(first + 1), Point::new(5.0, y)),
        ],
        false,
    );
}

/// Every kind and every slot form the codec writes: a path anchor, a segment midpoint, each
/// primitive feature, a clone, a circle (diameter and radius), an arc, a rounded corner, a free
/// point and a free circle; one dimension unlinked by a deleted node.
fn golden_document() -> Document {
    let document = Document::new(1);
    let add = |kind, slots| document.add_dimension(kind, slots, place()).unwrap();
    // Each dimension follows another step, so no two history rows in a row carry one label.
    let path = document.create_path(
        &[
            NewAnchor::corner(aid(1), Point::new(10.0, 10.0)),
            NewAnchor::corner(aid(2), Point::new(60.0, 10.0)),
            NewAnchor::corner(aid(3), Point::new(60.0, 40.0)),
        ],
        false,
    );
    add(
        DimensionKind::Aligned,
        DimensionSlots::Points(vec![anchor(path, 1), anchor(path, 2)]),
    );
    let rect = document.create_primitive(
        Shape::Rect {
            bounds: RectBounds {
                origin: Point::new(80.0, 10.0),
                width: Length::from_mm(40.0),
                height: Length::from_mm(20.0),
            },
            corner_radii: curvyo_document_core::CornerRadii::uniform(Length::from_mm(5.0)),
        },
        Angle::from_radians(0.0),
    );
    add(
        DimensionKind::Horizontal,
        DimensionSlots::Points(vec![
            PointSlot::SegmentMid {
                node: path,
                anchor: aid(2),
            },
            PointSlot::ShapePoint {
                node: rect,
                feature: ShapeFeature::Center,
            },
        ]),
    );
    let clone = document.linked_clone(&[path], &[]).unwrap()[0];
    add(
        DimensionKind::Vertical,
        DimensionSlots::Points(vec![
            anchor(clone, 3),
            PointSlot::Free {
                point: Point::new(70.0, 90.0),
            },
        ]),
    );
    let circle = document.create_primitive(
        Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(40.0, 80.0),
                rx: Length::from_mm(12.5),
                ry: Length::from_mm(12.5),
            },
            arc: None,
            curve: curvyo_document_core::Curve::ELLIPSE_DEFAULT,
        },
        Angle::from_radians(0.0),
    );
    add(
        DimensionKind::Diameter,
        DimensionSlots::Curve(CurveSlot::Circle { node: circle }),
    );
    let gone = document.create_path(
        &[
            NewAnchor::corner(aid(20), Point::new(0.0, 120.0)),
            NewAnchor::corner(aid(21), Point::new(30.0, 120.0)),
        ],
        false,
    );
    add(
        DimensionKind::Angle,
        DimensionSlots::Points(vec![anchor(path, 2), anchor(path, 1), anchor(path, 3)]),
    );
    separator(&document, 40);
    add(
        DimensionKind::Radius,
        DimensionSlots::Curve(CurveSlot::RoundCorner {
            node: rect,
            corner: 1,
        }),
    );
    separator(&document, 42);
    add(
        DimensionKind::Aligned,
        DimensionSlots::Points(vec![anchor(gone, 20), anchor(gone, 21)]),
    );
    document.delete_objects(&[gone]).unwrap();
    every_other_slot_form(&document, rect, circle);
    document
}

/// The slot forms the first part leaves out: a rectangle corner, a polygon vertex and edge
/// middle, a circle under `radius`, an arc segment of a path and a free circle.
fn every_other_slot_form(document: &Document, rect: NodeId, circle: NodeId) {
    let add = |kind, slots| document.add_dimension(kind, slots, place()).unwrap();
    let square = document.create_primitive(
        Shape::Polygon {
            frame: curvyo_document_core::StarFrame {
                center: Point::new(160.0, 40.0),
                radius: Length::from_mm(20.0),
                angle: Angle::from_radians(0.0),
            },
            point_count: curvyo_document_core::PointCount::new(4).unwrap(),
            curve: curvyo_document_core::Curve::POLYGON_DEFAULT,
        },
        Angle::from_radians(0.0),
    );
    add(
        DimensionKind::Vertical,
        DimensionSlots::Points(vec![
            PointSlot::ShapePoint {
                node: rect,
                feature: ShapeFeature::Corner(0),
            },
            PointSlot::ShapePoint {
                node: square,
                feature: ShapeFeature::Vertex(0),
            },
        ]),
    );
    separator(document, 44);
    add(
        DimensionKind::Aligned,
        DimensionSlots::Points(vec![
            PointSlot::ShapePoint {
                node: square,
                feature: ShapeFeature::EdgeMid(0),
            },
            PointSlot::ShapePoint {
                node: square,
                feature: ShapeFeature::Center,
            },
        ]),
    );
    separator(document, 46);
    add(
        DimensionKind::Radius,
        DimensionSlots::Curve(CurveSlot::Circle { node: circle }),
    );
    // A quarter circle of radius 20 about (110, 80) as one cubic.
    let k = 0.552_284_749_8 * 20.0;
    let arc = document.create_path(
        &[
            NewAnchor {
                handle_out: curvyo_document_core::Vec2::new(0.0, k),
                ..NewAnchor::corner(aid(60), Point::new(130.0, 80.0))
            },
            NewAnchor {
                handle_in: curvyo_document_core::Vec2::new(k, 0.0),
                ..NewAnchor::corner(aid(61), Point::new(110.0, 100.0))
            },
        ],
        false,
    );
    add(
        DimensionKind::Diameter,
        DimensionSlots::Curve(CurveSlot::Arc {
            node: arc,
            anchor: aid(60),
        }),
    );
    separator(document, 48);
    add(
        DimensionKind::Radius,
        DimensionSlots::Curve(CurveSlot::FreeCircle {
            center: Point::new(200.0, 100.0),
            radius: Length::from_mm(7.5),
        }),
    );
}

fn expected() -> Vec<(DimensionKind, bool, f64)> {
    vec![
        (DimensionKind::Aligned, true, 50.0),
        (DimensionKind::Horizontal, true, 40.0),
        (DimensionKind::Vertical, true, 50.0),
        (DimensionKind::Diameter, true, 25.0),
        (DimensionKind::Angle, true, std::f64::consts::FRAC_PI_2),
        (DimensionKind::Radius, true, 5.0),
        (DimensionKind::Aligned, false, 30.0),
        (DimensionKind::Vertical, true, 30.0),
        (DimensionKind::Aligned, true, 200f64.sqrt()),
        (DimensionKind::Radius, true, 12.5),
        (DimensionKind::Diameter, true, 40.0),
        (DimensionKind::Radius, true, 7.5),
    ]
}

fn dimensions(document: &Document) -> Vec<(DimensionKind, bool, f64)> {
    document
        .read_objects()
        .objects
        .iter()
        .filter_map(|object| match object {
            ObjectSnapshot::Dimension(d) => Some((d.kind, d.linked, d.shown.value.raw())),
            _ => None,
        })
        .collect()
}

fn close(a: &[(DimensionKind, bool, f64)], b: &[(DimensionKind, bool, f64)]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| x.0 == y.0 && x.1 == y.1 && (x.2 - y.2).abs() < 1e-6)
}

fn loro_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut stored = Vec::new();
    archive
        .by_name("document.loro")
        .unwrap()
        .read_to_end(&mut stored)
        .unwrap();
    stored
}

fn manifest_version(bytes: &[u8]) -> u32 {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut manifest = Vec::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_end(&mut manifest)
        .unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(&manifest).unwrap();
    u32::try_from(manifest["format_version"].as_u64().unwrap()).unwrap()
}

/// A saved document with one aligned dimension whose meta map `edit` damages.
fn damaged(edit: impl Fn(&loro::LoroMap)) -> Vec<u8> {
    let document = Document::new(1);
    let path = document.create_path(
        &[
            NewAnchor::corner(aid(1), Point::new(0.0, 0.0)),
            NewAnchor::corner(aid(2), Point::new(10.0, 0.0)),
        ],
        false,
    );
    document
        .add_dimension(
            DimensionKind::Aligned,
            DimensionSlots::Points(vec![anchor(path, 1), anchor(path, 2)]),
            place(),
        )
        .unwrap();
    let loro = LoroDoc::new();
    loro.import(&document.export_loro_snapshot().unwrap())
        .unwrap();
    loro.set_peer_id(9).unwrap();
    let tree = loro.get_tree("paths");
    let meta = tree
        .get_nodes(false)
        .into_iter()
        .filter_map(|n| tree.get_meta(n.id).ok())
        .find(|meta| {
            meta.get("shape")
                .is_some_and(|v| v.get_deep_value() == LoroValue::from("dimension"))
        })
        .unwrap();
    edit(&meta);
    loro.commit();
    let bytes = pack(&document, "t").unwrap();
    replace_loro(&bytes, &loro.export(loro::ExportMode::Snapshot).unwrap())
}

fn replace_loro(container: &[u8], loro: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut archive = zip::ZipArchive::new(Cursor::new(container)).unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let name = file.name().unwrap().to_string();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        writer.start_file(&name, options).unwrap();
        if name == "document.loro" {
            writer.write_all(loro).unwrap();
        } else {
            writer.write_all(&bytes).unwrap();
        }
    }
    writer.finish().unwrap().into_inner()
}

fn damaged_files() -> [(&'static str, Vec<u8>); 4] {
    [
        (
            DAMAGED[0],
            damaged(|meta| {
                meta.insert("dimension_kind", "ordinate").unwrap();
            }),
        ),
        (
            DAMAGED[1],
            damaged(|meta| {
                meta.insert(
                    "dimension_ref_c",
                    LoroValue::List(vec!["free".into(), 1.0.into(), 2.0.into()].into()),
                )
                .unwrap();
            }),
        ),
        (
            DAMAGED[2],
            damaged(|meta| {
                meta.insert(
                    "dimension_ref_b",
                    LoroValue::List(vec!["anchor".into(), "not a node".into()].into()),
                )
                .unwrap();
            }),
        ),
        (
            DAMAGED[3],
            damaged(|meta| {
                meta.insert("dimension_kept", vec![f64::NAN, 0.0, 0.0, 1.0, 0.0])
                    .unwrap();
            }),
        ),
    ]
}

fn write_fixtures_on_request() {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(
            fixture_path(GOLDEN),
            pack(&golden_document(), "0.1.0").unwrap(),
        )
        .unwrap();
        for (name, bytes) in damaged_files() {
            std::fs::write(fixture_path(name), bytes).unwrap();
        }
    }
}

#[test]
fn this_slice_took_its_format_version() {
    let bytes = pack(&Document::new(1), "t").unwrap();
    assert!(manifest_version(&bytes) >= THIS_SLICE_VERSION);
    assert_eq!(manifest_version(&bytes), CURRENT_FORMAT_VERSION);
}

#[test]
fn every_kind_and_slot_form_survives_save_and_open() {
    let document = golden_document();
    assert!(
        close(&dimensions(&document), &expected()),
        "{:?}",
        dimensions(&document)
    );
    let reopened = unpack(2, &pack(&document, "t").unwrap()).unwrap();
    assert!(close(&dimensions(&reopened), &expected()));
}

#[test]
fn each_damage_rule_refuses_the_file() {
    for (name, bytes) in damaged_files() {
        assert!(
            matches!(unpack(2, &bytes), Err(OpenError::Damaged)),
            "{name}"
        );
    }
}

/// A free circle without a radius above zero is damage; checked in memory, without a fixture of
/// its own.
#[test]
fn a_free_circle_without_a_radius_refuses_the_file() {
    let with_radius = |radius: f64| {
        damaged(move |meta| {
            meta.insert("dimension_kind", "radius").unwrap();
            meta.insert(
                "dimension_ref_a",
                LoroValue::List(
                    vec!["free_circle".into(), 1.0.into(), 2.0.into(), radius.into()].into(),
                ),
            )
            .unwrap();
            meta.delete("dimension_ref_b").unwrap();
        })
    };
    assert!(unpack(2, &with_radius(2.0)).is_ok());
    assert!(matches!(
        unpack(2, &with_radius(0.0)),
        Err(OpenError::Damaged)
    ));
}

#[test]
fn the_damaged_fixtures_are_refused_as_damaged() {
    write_fixtures_on_request();
    for name in DAMAGED {
        let bytes = std::fs::read(fixture_path(name)).unwrap();
        assert!(
            matches!(unpack(2, &bytes), Err(OpenError::Damaged)),
            "{name}"
        );
    }
}

#[test]
fn the_golden_reads_back_and_opening_it_writes_nothing() {
    write_fixtures_on_request();
    let bytes = std::fs::read(fixture_path(GOLDEN)).unwrap();
    assert_eq!(manifest_version(&bytes), THIS_SLICE_VERSION);
    let document = unpack(2, &bytes).unwrap();
    assert!(close(&dimensions(&document), &expected()));
    let stored = LoroDoc::new();
    stored.import(&loro_bytes(&bytes)).unwrap();
    let opened = LoroDoc::new();
    opened
        .import(&document.export_loro_snapshot().unwrap())
        .unwrap();
    assert_eq!(opened.oplog_vv(), stored.oplog_vv());
}
