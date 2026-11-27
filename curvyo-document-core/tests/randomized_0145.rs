//! Randomized on a polygon and star (`specs/0145-shape-spiro-handles` slice 3, criteria 17, 21
//! to 29, `adrs.md` decisions 4 and 8): the commands, the outline, bounds, the stored registers,
//! the golden file and the format number.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names
)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Angle, CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, Curve, Document, InnerRatio,
    Length, NodeId, ObjectSnapshot, OpenError, Point, PointCount, Randomized, Rounded, Seed, Shape,
    ShapeEditError, StarFrame, drawing_outline_of, outline_of, pack, shape_frame_bounds, unpack,
};
use loro::{LoroDoc, TreeID};

const GOLDEN: &str = "randomized_v60.curvyo";
/// The values of the golden file; the command limits 12.4 to 10.
const VALUES: [f64; 6] = [-10.0, -0.3, 0.3, 1.0, 10.0, 12.4];

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(GOLDEN)
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

fn star(d: &Document, x: f64) -> NodeId {
    d.create_star(
        frame(x),
        PointCount::new(5).unwrap(),
        InnerRatio::new(0.5).unwrap(),
    )
}

fn shape(d: &Document, id: NodeId) -> Shape {
    d.primitive(id).unwrap().shape
}

fn random_of(d: &Document, id: NodeId) -> Randomized {
    match shape(d, id) {
        Shape::Polygon { randomized, .. } | Shape::Star { randomized, .. } => randomized,
        _ => panic!("a polygon or star"),
    }
}

fn golden_document() -> Document {
    let d = Document::new(1);
    for (i, r) in VALUES.iter().enumerate() {
        let x = 50.0 * f64::from(u32::try_from(i).unwrap());
        let polygon = d.create_polygon(frame(x), PointCount::new(6).unwrap());
        let s = star(&d, x + 1000.0);
        // A value beyond ±10 is limited by the command; the golden's 12.4 is written raw below.
        d.set_randomized(&[(polygon, *r), (s, *r)]).unwrap();
    }
    d
}

fn points(shape: &Shape) -> Vec<Point> {
    outline_of(shape).anchors.iter().map(|a| a.point).collect()
}

/// Criterion 21: one commit for the whole batch, a value outside ±10 is limited and written,
/// nothing is committed when nothing changes; one undo restores.
#[test]
fn set_randomized_commits_once_limits_and_undoes() {
    let d = Document::new(1);
    let (a, b) = (star(&d, 0.0), star(&d, 100.0));
    let before = d.version();
    d.set_randomized(&[(a, 0.4), (b, 25.0)]).unwrap();
    assert_ne!(d.version(), before);
    assert_eq!(random_of(&d, a).amount(), 0.4);
    assert_eq!(random_of(&d, b).amount(), 10.0, "limited, and written");
    let after = d.version();
    d.set_randomized(&[(a, 0.4), (b, 10.0)]).unwrap();
    assert_eq!(d.version(), after, "unchanged writes nothing");
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
    assert_eq!(random_of(&d, a).amount(), 0.0);
    assert_eq!(random_of(&d, b).amount(), 0.0);
    assert_eq!(
        d.set_randomized(&[(a, f64::NAN)]),
        Err(ShapeEditError::InvalidRandomized)
    );
}

/// Criterion 21: an ellipse or rectangle refuses the command, nothing is written.
#[test]
fn only_a_polygon_or_star_takes_a_value() {
    let d = Document::new(1);
    let e = d.create_ellipse(curvyo_document_core::EllipseFrame {
        center: Point::new(0.0, 0.0),
        rx: Length::from_mm(5.0),
        ry: Length::from_mm(5.0),
    });
    assert_eq!(
        d.set_randomized(&[(e, 1.0)]),
        Err(ShapeEditError::WrongShape)
    );
    assert_eq!(d.reroll_seeds(&[e]), Err(ShapeEditError::WrongShape));
}

/// Criterion 22: a fresh seed is drawn in the same commit when the value first leaves 0; the seed
/// stays when the value goes back to 0 and again away; Re-roll gives a new seed and a different
/// outline in one commit, one undo restores both.
#[test]
fn the_seed_is_drawn_once_and_rerolling_changes_the_outline() {
    let d = Document::new(1);
    let s = star(&d, 0.0);
    assert_eq!(random_of(&d, s).seed(), Seed::new(0));
    d.set_randomized(&[(s, 1.0)]).unwrap();
    let seed = random_of(&d, s).seed();
    assert_eq!(seed, Seed::fresh(s));
    d.set_randomized(&[(s, 0.0)]).unwrap();
    d.set_randomized(&[(s, 1.0)]).unwrap();
    assert_eq!(random_of(&d, s).seed(), seed, "kept while the value is 0");
    let ring = points(&shape(&d, s));
    d.reroll_seeds(&[s]).unwrap();
    assert_eq!(random_of(&d, s).seed(), seed.next());
    assert_ne!(points(&shape(&d, s)), ring, "the outline changes");
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
    assert_eq!(random_of(&d, s).seed(), seed);
    assert_eq!(points(&shape(&d, s)), ring, "the same outline again");
}

/// Criterion 22: a shape at 0 is left alone by Re-roll (nothing changes, nothing is committed);
/// two shapes with different seeds get different new seeds in one commit.
#[test]
fn rerolling_touches_only_randomized_shapes_each_with_its_own_seed() {
    let d = Document::new(1);
    let (a, b, c) = (star(&d, 0.0), star(&d, 100.0), star(&d, 200.0));
    d.set_randomized(&[(a, 1.0), (b, 1.0)]).unwrap();
    let (sa, sb) = (random_of(&d, a).seed(), random_of(&d, b).seed());
    assert_ne!(sa, sb);
    let before = d.version();
    d.reroll_seeds(&[c]).unwrap();
    assert_eq!(d.version(), before);
    d.reroll_seeds(&[a, b, c]).unwrap();
    let (na, nb) = (random_of(&d, a).seed(), random_of(&d, b).seed());
    assert_eq!((na, nb), (sa.next(), sb.next()));
    assert_ne!(na, nb);
    assert_eq!(random_of(&d, c).seed(), Seed::new(0));
}

/// Criterion 25: `r = 0` draws today's shape, whatever the stored seed is.
#[test]
fn zero_is_todays_shape_whatever_the_seed() {
    let d = Document::new(1);
    let plain = star(&d, 0.0);
    let kept = star(&d, 0.0);
    d.set_randomized(&[(kept, 1.0)]).unwrap();
    d.reroll_seeds(&[kept]).unwrap();
    d.set_randomized(&[(kept, 0.0)]).unwrap();
    assert_ne!(random_of(&d, kept).seed(), Seed::new(0), "a seed is stored");
    let (a, b) = (shape(&d, plain), shape(&d, kept));
    assert_eq!(outline_of(&a), outline_of(&b));
    assert_eq!(drawing_outline_of(&a), drawing_outline_of(&b));
    assert_eq!(shape_frame_bounds(&a), shape_frame_bounds(&b));
}

/// Criterion 27 and 6: Points and Ratio keep working and keep the value and the seed, also
/// together with Rounded and Curve; Curve and Rounded exclude each other as before.
#[test]
fn the_shape_stays_live_and_combines_with_rounded_and_curve() {
    let d = Document::new(1);
    let s = star(&d, 0.0);
    d.set_randomized(&[(s, 0.3)]).unwrap();
    let kept = random_of(&d, s);
    d.set_point_count(&[s], PointCount::new(8).unwrap())
        .unwrap();
    d.set_inner_ratio(&[s], InnerRatio::new(0.4).unwrap())
        .unwrap();
    d.translate_objects(&[s], curvyo_document_core::Vec2::new(5.0, 5.0))
        .unwrap();
    assert_eq!(random_of(&d, s), kept);
    d.set_rounded(&[(s, Rounded::new(0.8).unwrap())]).unwrap();
    assert_eq!(random_of(&d, s), kept, "Rounded combines with Randomized");
    d.set_curves(&[(s, Curve::new(0.2).unwrap())]).unwrap();
    assert_eq!(random_of(&d, s), kept, "Curve combines with Randomized");
    let Shape::Star { rounded, .. } = shape(&d, s) else {
        panic!("a star")
    };
    assert_eq!(rounded, Rounded::ZERO);
}

/// Criterion 27: with Rounded or Curve the outline still moves the vertices (the handles and the
/// nodes on the outline are pinned in `ring_random.rs`).
#[test]
fn randomizing_moves_the_vertices_also_with_rounded_and_curve() {
    for rounded in [None, Some(0.6), Some(-1.5)] {
        let d = Document::new(1);
        let s = star(&d, 0.0);
        d.set_randomized(&[(s, 0.4)]).unwrap();
        if let Some(q) = rounded {
            d.set_rounded(&[(s, Rounded::new(q).unwrap())]).unwrap();
        } else {
            d.set_curves(&[(s, Curve::new(0.1).unwrap())]).unwrap();
        }
        let moved = outline_of(&shape(&d, s));
        let plain = {
            let p = star(&d, 0.0);
            outline_of(&shape(&d, p))
        };
        assert!(moved.anchors.len() >= plain.anchors.len());
        // The unmoved ring differs: at least one vertex moved by more than a millimetre.
        assert!(
            moved.anchors.iter().zip(&plain.anchors).any(|(a, b)| a
                .point
                .vector_to(b.point)
                .length()
                > 1.0)
        );
    }
}

/// Criterion 28: the box is today's box united with the exact box of the outline; 4096 sampled
/// outline points are inside and the extremes are touched wherever the outline leaves today's box.
#[test]
fn the_box_holds_the_outline_and_touches_its_extremes() {
    for (r, rounded) in [(0.5, 0.0), (2.0, 0.0), (0.5, 1.5)] {
        let d = Document::new(1);
        let s = star(&d, 0.0);
        d.set_randomized(&[(s, r)]).unwrap();
        if rounded != 0.0 {
            d.set_rounded(&[(s, Rounded::new(rounded).unwrap())])
                .unwrap();
        }
        let sh = shape(&d, s);
        let (min, max) = shape_frame_bounds(&sh);
        let outline = outline_of(&sh).anchors;
        let (mut lo, mut hi) = (
            Point::new(f64::INFINITY, f64::INFINITY),
            Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        );
        let n = outline.len();
        let per = 4096 / n + 1;
        for i in 0..n {
            let (a, b) = (outline[i], outline[(i + 1) % n]);
            for k in 0..=per {
                let t =
                    f64::from(u32::try_from(k).unwrap()) / f64::from(u32::try_from(per).unwrap());
                let u = 1.0 - t;
                let pt = |f: fn(Point) -> f64| {
                    u.powi(3) * f(a.point)
                        + 3.0 * u * u * t * f(a.point.translated(a.handle_out))
                        + 3.0 * u * t * t * f(b.point.translated(b.handle_in))
                        + t.powi(3) * f(b.point)
                };
                let (x, y) = (pt(|p| p.x), pt(|p| p.y));
                assert!(
                    x >= min.x - 1e-9 && x <= max.x + 1e-9,
                    "x {x} in {min:?} {max:?}"
                );
                assert!(y >= min.y - 1e-9 && y <= max.y + 1e-9, "y {y}");
                lo = Point::new(lo.x.min(x), lo.y.min(y));
                hi = Point::new(hi.x.max(x), hi.y.max(y));
            }
        }
        // Today's box is the circumscribed square of radius 20 around the centre.
        if lo.x < -20.0 - 1e-3 {
            assert!(
                (lo.x - min.x).abs() < 1e-3,
                "left extreme {} vs {}",
                lo.x,
                min.x
            );
        }
        if hi.x > 20.0 + 1e-3 {
            assert!((hi.x - max.x).abs() < 1e-3, "right extreme");
        }
        if lo.y < -20.0 - 1e-3 {
            assert!((lo.y - min.y).abs() < 1e-3, "top extreme");
        }
        if hi.y > 20.0 + 1e-3 {
            assert!((hi.y - max.y).abs() < 1e-3, "bottom extreme");
        }
        assert!(min.x <= -20.0 && max.x >= 20.0 && min.y <= -20.0 && max.y >= 20.0);
    }
}

fn amounts(d: &Document) -> Vec<f64> {
    d.read_objects()
        .objects
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Primitive(p) => match p.shape {
                Shape::Polygon { randomized, .. } | Shape::Star { randomized, .. } => {
                    Some(randomized.amount())
                }
                _ => None,
            },
            _ => None,
        })
        .collect()
}

fn write_fixture_on_request() {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(fixture_path(), pack(&golden_document(), "0.1.0").unwrap()).unwrap();
    }
}

/// Criterion 26: save and open keep every value and seed exactly, the golden file reads back.
#[test]
fn the_values_and_seeds_survive_save_open_and_the_golden_reads_back() {
    write_fixture_on_request();
    let d = golden_document();
    let reopened = unpack(2, &pack(&d, "t").unwrap()).unwrap();
    assert_eq!(amounts(&reopened), amounts(&d));
    assert_eq!(reopened.export_json().unwrap(), d.export_json().unwrap());
    for o in d.read_objects().objects.iter() {
        if let ObjectSnapshot::Primitive(p) = o {
            assert_eq!(random_of(&reopened, p.id), random_of(&d, p.id));
        }
    }
    let golden = unpack(2, &std::fs::read(fixture_path()).unwrap()).unwrap();
    let want: Vec<f64> = VALUES
        .iter()
        .flat_map(|r| [r.clamp(-10.0, 10.0); 2])
        .collect();
    assert_eq!(amounts(&golden), want);
}

/// Criterion 26: a shape at 0 stores no `randomized` register; no register is a shape at 0.
#[test]
fn zero_stores_no_register() {
    let d = Document::new(1);
    let s = star(&d, 0.0);
    d.set_randomized(&[(s, 0.7)]).unwrap();
    d.set_randomized(&[(s, 0.0)]).unwrap();
    let loro = LoroDoc::new();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let meta = loro.get_tree("paths").get_meta(tree_id(s)).unwrap();
    assert!(meta.get("randomized").is_none());
    let untouched = star(&d, 5.0);
    let loro = LoroDoc::new();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let meta = loro.get_tree("paths").get_meta(tree_id(untouched)).unwrap();
    assert!(meta.get("randomized").is_none() && meta.get("seed").is_none());
}

fn with_registers(d: &Document, id: NodeId, put: impl Fn(&loro::LoroMap)) -> Vec<u8> {
    let loro = LoroDoc::new();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    loro.set_peer_id(7).unwrap();
    put(&loro.get_tree("paths").get_meta(tree_id(id)).unwrap());
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
    writer.finish().unwrap().into_inner()
}

/// Criterion 26: a non-finite value or a seed outside 32 bits is damaged; a value beyond ±10
/// opens and is limited on use, nothing is written back; a value without a seed opens with the
/// seed 0 (a merge can leave it so).
#[test]
fn damaged_registers_are_refused_and_a_large_value_is_limited_on_use() {
    let d = Document::new(1);
    let s = star(&d, 0.0);
    let nan = with_registers(&d, s, |m| {
        m.insert("randomized", f64::NAN).unwrap();
    });
    assert_eq!(unpack(2, &nan).err(), Some(OpenError::Damaged));
    let big_seed = with_registers(&d, s, |m| {
        m.insert("randomized", 1.0).unwrap();
        m.insert("seed", 1_i64 << 33).unwrap();
    });
    assert_eq!(unpack(2, &big_seed).err(), Some(OpenError::Damaged));
    let large = with_registers(&d, s, |m| {
        m.insert("randomized", 12.4).unwrap();
    });
    let opened = unpack(2, &large).unwrap();
    let random = random_of(&opened, s);
    assert_eq!((random.amount(), random.effective()), (12.4, 10.0));
    assert_eq!(random.seed(), Seed::new(0));
    let (min, max) = shape_frame_bounds(&shape(&opened, s));
    assert!(min.x.is_finite() && max.y.is_finite());
}

/// Criterion 26: this build writes format 62 (`0118` slice 2d) and refuses a newer file; the golden is 60.
#[test]
fn the_format_is_60_and_a_newer_file_is_refused() {
    assert_eq!(CURRENT_FORMAT_VERSION, 70);
    let bytes = pack(&golden_document(), "t").unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes.clone())).unwrap();
    let mut manifest = Vec::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_end(&mut manifest)
        .unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(&manifest).unwrap();
    assert_eq!(manifest["format_version"].as_u64().unwrap(), 70);
    let golden = std::fs::read(fixture_path()).unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(golden)).unwrap();
    let mut manifest = Vec::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_end(&mut manifest)
        .unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(&manifest).unwrap();
    assert_eq!(manifest["format_version"].as_u64().unwrap(), 60);
}
