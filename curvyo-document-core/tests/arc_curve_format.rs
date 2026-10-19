//! The stored shape of ellipse arcs and shape curves (`specs/0021-ellipse-arcs-and-shaping`
//! criteria 14, 15, 32, 33 and 50, `adrs.md` decisions 1 to 3): save and open exactly, the
//! golden files, the damaged files, and files of an earlier version that open unchanged.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Angle, ArcKind, CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, Curve, Document,
    EllipseArc, EllipseFrame, InnerRatio, Length, NodeId, OpenError, Point, PointCount, Shape,
    StarFrame, Vec2, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeID};

/// The version this slice takes. A rebase onto another bump renumbers this constant, the
/// fixtures and their names (`specs/README.md`, "`format_version` plan").
const ARC_FORMAT_VERSION: u32 = 15;
const GOLDEN_PIE: &str = "arc_pie_v15.curvyo";
const GOLDEN_ARC: &str = "arc_arc_v15.curvyo";
const GOLDEN_CHORD: &str = "arc_chord_v15.curvyo";
const GOLDEN_CURVE_ELLIPSE: &str = "curve_ellipse_v15.curvyo";
const GOLDEN_CURVE_POLYGON_STAR: &str = "curve_polygon_star_v15.curvyo";
const DAMAGED_ARC_WITH_CURVE: &str = "arc_with_curve_damaged_v15.curvyo";
const DAMAGED_CURVE_TWO: &str = "curve_two_damaged_v15.curvyo";
const DAMAGED_UNKNOWN_KIND: &str = "arc_unknown_kind_damaged_v15.curvyo";

/// The curve values of the golden files (decision 1 of `adrs.md`, the fixture list).
const CURVES: [f64; 5] = [1.0, 0.5, 0.0, -0.5, -1.0];

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn read_member(bytes: &[u8], name: &str) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Vec::new();
    archive
        .by_name(name)
        .unwrap()
        .read_to_end(&mut out)
        .unwrap();
    out
}

fn manifest_version(bytes: &[u8]) -> u32 {
    let manifest: serde_json::Value =
        serde_json::from_slice(&read_member(bytes, "manifest.json")).unwrap();
    u32::try_from(manifest["format_version"].as_u64().unwrap()).unwrap()
}

fn tree_id(id: NodeId) -> TreeID {
    let json = serde_json::to_value(id).unwrap();
    TreeID::new(
        json["peer"].as_str().unwrap().parse().unwrap(),
        i32::try_from(json["counter"].as_i64().unwrap()).unwrap(),
    )
}

fn deg(value: f64) -> Angle {
    Angle::from_radians(value.to_radians())
}

fn circle(d: &Document, x: f64, r: f64) -> NodeId {
    d.create_ellipse(EllipseFrame {
        center: Point::new(x, 0.0),
        rx: Length::from_mm(r),
        ry: Length::from_mm(r),
    })
}

fn star_frame(x: f64) -> StarFrame {
    StarFrame {
        center: Point::new(x, 49.0),
        radius: Length::from_mm(20.0),
        angle: Angle::from_radians(0.0),
    }
}

/// One circle of radius 20 with the arc 30° to 150° of `kind`.
fn arc_document(kind: ArcKind) -> (Document, NodeId) {
    let d = Document::new(1);
    let id = circle(&d, 0.0, 20.0);
    d.set_ellipse_arcs(&[(id, EllipseArc::new(deg(30.0), deg(150.0), kind))])
        .unwrap();
    (d, id)
}

/// Sets `k` on `id`, or moves it by 1 mm where `k` is its default and writes nothing: an
/// older file never has two adjacent steps with one label.
fn curve_or_move(d: &Document, id: NodeId, k: f64) {
    let before = d.version();
    d.set_curves(&[(id, Curve::new(k).unwrap())]).unwrap();
    if d.version() == before {
        d.translate_objects(&[id], Vec2::new(0.0, 1.0)).unwrap();
    }
}

/// A circle of radius 20 and an ellipse 30 × 10 at every curve of [`CURVES`].
fn curve_ellipse_document() -> (Document, Vec<NodeId>) {
    let d = Document::new(1);
    let mut ids = Vec::new();
    for (i, k) in CURVES.iter().enumerate() {
        let x = 50.0 * f64::from(u32::try_from(i).unwrap());
        let c = circle(&d, x, 20.0);
        curve_or_move(&d, c, *k);
        let e = d.create_ellipse(EllipseFrame {
            center: Point::new(x, 49.0),
            rx: Length::from_mm(30.0),
            ry: Length::from_mm(10.0),
        });
        curve_or_move(&d, e, *k);
        ids.extend([c, e]);
    }
    (d, ids)
}

/// A triangle (limit -50 %), a hexagon and a pentagram star at 100, 50, 0, -50 % and the
/// limit, plus the triangle at 100 % (6 nodes when converted).
fn curve_polygon_star_document() -> (Document, Vec<(NodeId, f64)>) {
    let d = Document::new(1);
    let mut ids = Vec::new();
    let pentagram = InnerRatio::new(0.382).unwrap();
    for (i, k) in [1.0, 0.5, 0.0, -0.5, -1.0].iter().enumerate() {
        let x = 50.0 * f64::from(u32::try_from(i).unwrap());
        let triangle = d.create_polygon(star_frame(x), PointCount::new(3).unwrap());
        curve_or_move(&d, triangle, *k);
        let hexagon = d.create_polygon(star_frame(x + 1000.0), PointCount::new(6).unwrap());
        curve_or_move(&d, hexagon, *k);
        let star = d.create_star(
            star_frame(x + 2000.0),
            PointCount::new(5).unwrap(),
            pentagram,
        );
        curve_or_move(&d, star, *k);
        for id in [triangle, hexagon, star] {
            ids.push((id, *k));
        }
    }
    (d, ids)
}

/// A file whose ellipse's meta map gets `edit` after a valid arc or curve was written.
fn damaged(edit: impl FnOnce(&loro::LoroMap)) -> Vec<u8> {
    let (d, id) = arc_document(ArcKind::Pie);
    let loro = LoroDoc::new();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    loro.set_peer_id(7).unwrap();
    edit(&loro.get_tree("paths").get_meta(tree_id(id)).unwrap());
    loro.commit();
    let manifest = serde_json::json!({
        "format_version": CURRENT_FORMAT_VERSION,
        "loro_snapshot_version": CURRENT_LORO_SNAPSHOT_VERSION,
        "app_version": "test",
    });
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in [
        ("manifest.json", serde_json::to_vec(&manifest).unwrap()),
        (
            "document.loro",
            loro.export(loro::ExportMode::Snapshot).unwrap(),
        ),
        ("document.json", b"{}".to_vec()),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(&bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn damaged_files() -> [(&'static str, Vec<u8>); 3] {
    [
        (
            DAMAGED_ARC_WITH_CURVE,
            damaged(|meta| {
                meta.insert("curve", 0.5).unwrap();
            }),
        ),
        (
            DAMAGED_CURVE_TWO,
            damaged(|meta| {
                meta.delete("ellipse_arc").unwrap();
                meta.insert("curve", 2.0).unwrap();
            }),
        ),
        (
            DAMAGED_UNKNOWN_KIND,
            damaged(|meta| {
                meta.insert(
                    "ellipse_arc",
                    LoroValue::List(
                        vec![
                            LoroValue::Double(30.0),
                            LoroValue::Double(150.0),
                            LoroValue::String("wedge".into()),
                        ]
                        .into(),
                    ),
                )
                .unwrap();
            }),
        ),
    ]
}

fn goldens() -> [(&'static str, Document); 5] {
    [
        (GOLDEN_PIE, arc_document(ArcKind::Pie).0),
        (GOLDEN_ARC, arc_document(ArcKind::Arc).0),
        (GOLDEN_CHORD, arc_document(ArcKind::Chord).0),
        (GOLDEN_CURVE_ELLIPSE, curve_ellipse_document().0),
        (GOLDEN_CURVE_POLYGON_STAR, curve_polygon_star_document().0),
    ]
}

fn write_fixtures_on_request() {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        for (name, document) in goldens() {
            std::fs::write(fixture_path(name), pack(&document, "0.1.0").unwrap()).unwrap();
        }
        for (name, bytes) in damaged_files() {
            std::fs::write(fixture_path(name), bytes).unwrap();
        }
    }
}

#[test]
fn ac15_this_slice_takes_its_format_version() {
    const _: () = assert!(CURRENT_FORMAT_VERSION >= ARC_FORMAT_VERSION);
    let bytes = pack(&Document::new(1), "t").unwrap();
    assert_eq!(manifest_version(&bytes), CURRENT_FORMAT_VERSION);
}

/// Criteria 14 and 32: save and open keep every value exactly.
#[test]
fn ac14_ac32_save_and_open_keep_arc_and_curve_exactly() {
    for (name, document) in goldens() {
        let reopened = unpack(2, &pack(&document, "t").unwrap()).unwrap();
        assert_eq!(reopened.read_objects(), document.read_objects(), "{name}");
        assert_eq!(
            reopened.export_json().unwrap(),
            document.export_json().unwrap(),
            "{name}"
        );
    }
}

/// Criteria 14, 32 and 50: the golden files read back as the primitives they were saved as.
#[test]
fn ac14_ac32_ac50_the_goldens_read_back() {
    write_fixtures_on_request();
    for (name, kind) in [
        (GOLDEN_PIE, ArcKind::Pie),
        (GOLDEN_ARC, ArcKind::Arc),
        (GOLDEN_CHORD, ArcKind::Chord),
    ] {
        let bytes = std::fs::read(fixture_path(name)).unwrap();
        assert_eq!(manifest_version(&bytes), ARC_FORMAT_VERSION);
        let d = unpack(2, &bytes).unwrap();
        let (_, id) = arc_document(kind);
        let Some(Shape::Ellipse { arc: Some(arc), .. }) = d.primitive(id).map(|p| p.shape) else {
            panic!("{name}: an ellipse with an arc");
        };
        assert_eq!(arc.kind, kind);
        assert!((arc.start().as_radians().to_degrees() - 30.0).abs() < 1e-9);
        assert!((arc.end().as_radians().to_degrees() - 150.0).abs() < 1e-9);
    }
    let d = unpack(
        2,
        &std::fs::read(fixture_path(GOLDEN_CURVE_ELLIPSE)).unwrap(),
    )
    .unwrap();
    let (_, ids) = curve_ellipse_document();
    for (id, k) in ids.iter().zip(CURVES.iter().flat_map(|k| [k, k])) {
        let Some(Shape::Ellipse { curve, arc, .. }) = d.primitive(*id).map(|p| p.shape) else {
            panic!("an ellipse");
        };
        assert!(arc.is_none());
        assert_eq!(curve.get(), *k);
    }
    let d = unpack(
        2,
        &std::fs::read(fixture_path(GOLDEN_CURVE_POLYGON_STAR)).unwrap(),
    )
    .unwrap();
    for (id, k) in curve_polygon_star_document().1 {
        match d.primitive(id).map(|p| p.shape) {
            Some(Shape::Polygon { curve, .. } | Shape::Star { curve, .. }) => {
                assert_eq!(curve.get(), k, "stored raw, also below the limit");
            }
            other => panic!("a polygon or star: {other:?}"),
        }
    }
}

/// Criteria 33 and 50: a damaged arc or curve is refused, not clamped.
#[test]
fn ac33_ac50_damaged_files_are_refused() {
    write_fixtures_on_request();
    for (name, bytes) in damaged_files() {
        assert_eq!(unpack(2, &bytes).err(), Some(OpenError::Damaged), "{name}");
        let stored = std::fs::read(fixture_path(name)).unwrap();
        assert_eq!(unpack(2, &stored).err(), Some(OpenError::Damaged), "{name}");
    }
}

/// Criteria 14, 32 and 50: a whole ellipse at 100 % and a straight polygon store no new
/// key, so earlier files are unchanged.
#[test]
fn ac14_ac32_ac50_default_shapes_store_no_new_key() {
    let d = Document::new(1);
    let e = circle(&d, 0.0, 10.0);
    let p = d.create_polygon(star_frame(0.0), PointCount::new(5).unwrap());
    d.set_curves(&[(e, Curve::new(0.3).unwrap()), (p, Curve::new(0.3).unwrap())])
        .unwrap();
    d.set_curves(&[(e, Curve::ELLIPSE_DEFAULT), (p, Curve::POLYGON_DEFAULT)])
        .unwrap();
    let loro = LoroDoc::new();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let tree = loro.get_tree("paths");
    for id in [e, p] {
        let meta = tree.get_meta(tree_id(id)).unwrap();
        assert!(meta.get("curve").is_none() && meta.get("ellipse_arc").is_none());
    }
}

/// Criteria 15 and 33: a file of this build declares a version a build before it refuses.
#[test]
fn ac15_ac33_an_older_build_refuses_the_file() {
    let bytes = pack(&arc_document(ArcKind::Arc).0, "t").unwrap();
    assert!(manifest_version(&bytes) > ARC_FORMAT_VERSION - 1);
}
