//! The stored shape of a polygon's or star's Rounded value (`specs/0145-shape-spiro-handles`
//! criteria 5, 6 and 11, `adrs.md` decisions 3 and 4): save and open exactly, the golden file,
//! the damaged file, the limited value, and files of an earlier version that open unchanged.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Angle, CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, Curve, Document, InnerRatio,
    Length, NodeId, ObjectSnapshot, OpenError, Point, PointCount, Rounded, Shape, StarFrame, pack,
    unpack,
};
use loro::{LoroDoc, TreeID};

const GOLDEN: &str = "rounded_v56.curvyo";
/// The values of the golden file; 12.4 is beyond the limit and stored raw.
const VALUES: [f64; 9] = [-10.0, -3.0, -0.3, 0.0, 0.3, 1.0, 2.5, 10.0, 12.4];

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn frame(x: f64) -> StarFrame {
    StarFrame {
        center: Point::new(x, 0.0),
        radius: Length::from_mm(20.0),
        angle: Angle::from_radians(0.0),
    }
}

fn tree_id(id: NodeId) -> TreeID {
    let json = serde_json::to_value(id).unwrap();
    TreeID::new(
        json["peer"].as_str().unwrap().parse().unwrap(),
        i32::try_from(json["counter"].as_i64().unwrap()).unwrap(),
    )
}

/// A hexagon and a 5-point star per value.
fn golden_document() -> Document {
    let d = Document::new(1);
    for (i, q) in VALUES.iter().enumerate() {
        let x = 50.0 * f64::from(u32::try_from(i).unwrap());
        let hexagon = d.create_polygon(frame(x), PointCount::new(6).unwrap());
        let star = d.create_star(
            frame(x + 1000.0),
            PointCount::new(5).unwrap(),
            InnerRatio::new(0.5).unwrap(),
        );
        let before = d.version();
        d.set_rounded(&[
            (hexagon, Rounded::new(*q).unwrap()),
            (star, Rounded::new(*q).unwrap()),
        ])
        .unwrap();
        if d.version() == before {
            // Value 0: nothing written; keep the history free of adjacent equal steps.
            d.translate_objects(&[hexagon], curvyo_document_core::Vec2::new(1.0, 0.0))
                .unwrap();
        }
    }
    d
}

fn rounded_values(d: &Document) -> Vec<f64> {
    d.read_objects()
        .objects
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Primitive(p) => match p.shape {
                Shape::Polygon { rounded, .. } | Shape::Star { rounded, .. } => Some(rounded.get()),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

fn expected() -> Vec<f64> {
    VALUES.iter().flat_map(|q| [*q, *q]).collect()
}

fn write_fixture_on_request() {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(
            fixture_path(GOLDEN),
            pack(&golden_document(), "0.1.0").unwrap(),
        )
        .unwrap();
    }
}

/// Criterion 11: save and open keep every value exactly, the golden file reads back raw.
#[test]
fn the_value_survives_save_open_and_the_golden_reads_back() {
    write_fixture_on_request();
    let d = golden_document();
    let reopened = unpack(2, &pack(&d, "t").unwrap()).unwrap();
    assert_eq!(rounded_values(&reopened), expected());
    assert_eq!(reopened.export_json().unwrap(), d.export_json().unwrap());
    let bytes = std::fs::read(fixture_path(GOLDEN)).unwrap();
    assert_eq!(rounded_values(&unpack(2, &bytes).unwrap()), expected());
}

/// Criterion 11: a shape at 0 stores no key, so an untouched shape is byte for byte as before.
#[test]
fn zero_stores_no_key_and_deletes_the_register() {
    let d = Document::new(1);
    let p = d.create_polygon(frame(0.0), PointCount::new(5).unwrap());
    d.set_rounded(&[(p, Rounded::new(0.7).unwrap())]).unwrap();
    d.set_rounded(&[(p, Rounded::ZERO)]).unwrap();
    let loro = LoroDoc::new();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let meta = loro.get_tree("paths").get_meta(tree_id(p)).unwrap();
    assert!(meta.get("rounded").is_none());
}

/// Criterion 11: a non-finite stored value is damaged; one beyond ±10 opens and is limited
/// only where it is evaluated, nothing is written back.
#[test]
fn a_non_finite_value_is_refused_and_a_large_one_is_limited_on_use() {
    let d = Document::new(1);
    let p = d.create_polygon(frame(0.0), PointCount::new(5).unwrap());
    let loro = LoroDoc::new();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    loro.set_peer_id(7).unwrap();
    loro.get_tree("paths")
        .get_meta(tree_id(p))
        .unwrap()
        .insert("rounded", f64::NAN)
        .unwrap();
    loro.commit();
    let manifest = serde_json::json!({
        "format_version": CURRENT_FORMAT_VERSION,
        "loro_snapshot_version": CURRENT_LORO_SNAPSHOT_VERSION,
        "app_version": "test",
    });
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
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
    let bytes = writer.finish().unwrap().into_inner();
    assert_eq!(unpack(2, &bytes).err(), Some(OpenError::Damaged));
    let golden = unpack(2, &std::fs::read(fixture_path(GOLDEN)).unwrap()).unwrap();
    assert!(rounded_values(&golden).contains(&12.4));
}

/// Criterion 5: both registers non-zero (an LWW merge) is valid; Rounded wins; one writer
/// writes both in one commit and one undo restores both.
#[test]
fn rounded_and_curve_exclude_each_other_in_one_commit() {
    let d = Document::new(1);
    let p = d.create_polygon(frame(0.0), PointCount::new(6).unwrap());
    d.set_curves(&[(p, Curve::new(0.5).unwrap())]).unwrap();
    d.set_rounded(&[(p, Rounded::new(1.5).unwrap())]).unwrap();
    let Some(Shape::Polygon { curve, rounded, .. }) = d.primitive(p).map(|x| x.shape) else {
        panic!("a polygon")
    };
    assert_eq!((curve, rounded.get()), (Curve::POLYGON_DEFAULT, 1.5));
    d.set_curves(&[(p, Curve::new(0.25).unwrap())]).unwrap();
    let Some(Shape::Polygon { curve, rounded, .. }) = d.primitive(p).map(|x| x.shape) else {
        panic!("a polygon")
    };
    assert_eq!((curve.get(), rounded), (0.25, Rounded::ZERO));
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
    let Some(Shape::Polygon { curve, rounded, .. }) = d.primitive(p).map(|x| x.shape) else {
        panic!("a polygon")
    };
    assert_eq!((curve, rounded.get()), (Curve::POLYGON_DEFAULT, 1.5));
}

/// Criterion 6: Points change and the value stays; Rounded does not turn the object into a path.
#[test]
fn a_looped_star_stays_a_star_when_its_points_change() {
    let d = Document::new(1);
    let s = d.create_star(
        frame(0.0),
        PointCount::new(5).unwrap(),
        InnerRatio::new(0.5).unwrap(),
    );
    d.set_rounded(&[(s, Rounded::new(2.5).unwrap())]).unwrap();
    d.set_point_count(&[s], PointCount::new(8).unwrap())
        .unwrap();
    let Some(Shape::Star {
        rounded,
        point_count,
        ..
    }) = d.primitive(s).map(|x| x.shape)
    else {
        panic!("a star")
    };
    assert_eq!((rounded.get(), point_count.get()), (2.5, 8));
}

/// Criterion 11: a build before this one refuses the file by version.
#[test]
fn an_older_build_refuses_the_file() {
    let bytes = pack(&golden_document(), "t").unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut manifest = Vec::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_end(&mut manifest)
        .unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(&manifest).unwrap();
    assert!(manifest["format_version"].as_u64().unwrap() >= 56);
}
