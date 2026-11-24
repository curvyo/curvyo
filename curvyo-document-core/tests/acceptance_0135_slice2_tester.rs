//! Tester acceptance cases for `specs/0135-image-optimisation` slice 2 in `curvyo-document-core`,
//! written from the specification (criteria 5, 8, 18, 59) and ADR 0072 §3 and §8 before the
//! implementation was read: a Crop narrows the drawn frame to the cropped part at the same scale
//! while `image_placement` and the asset bytes stay as they are; removing or switching off the
//! Crop gives the whole frame back exactly (lossless); two crops compose; format 52 and the crop
//! golden.
#![allow(
    clippy::unwrap_used,
    clippy::many_single_char_names,
    clippy::assert_is_empty,
    clippy::float_cmp,
    clippy::too_many_lines,
    missing_docs
)]

use std::io::{Cursor, Read};
use std::path::PathBuf;
use std::sync::Arc;

use curvyo_document_core::{
    AdjustEntry, AdjustStep, AdjustmentsWrite, Affine, Document, ImageSnapshot, NewImage, NodeId,
    ObjectSnapshot, Point, pack, unpack,
};
use serde_json::Value;

const EPS: f64 = 1e-9;

fn png() -> Arc<[u8]> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-image-core/tests/fixtures/images/png_rgb_8.png");
    Arc::from(std::fs::read(path).unwrap())
}

/// A picture 40 x 20 mm at (100, 50), rotated by 30 degrees.
fn placement() -> Affine {
    let (s, c) = 30f64.to_radians().sin_cos();
    Affine::new(40.0 * c, 40.0 * s, -20.0 * s, 20.0 * c, 100.0, 50.0)
}

fn image(d: &Document) -> NodeId {
    d.add_images(&[NewImage {
        bytes: png(),
        placement: placement(),
        name: "P".to_owned(),
    }])
    .unwrap()[0]
}

fn crop(l: i64, t: i64, w: i64, h: i64, enabled: bool) -> AdjustEntry {
    AdjustEntry::Step(AdjustStep {
        kind: "crop".to_owned(),
        enabled,
        fields: [
            ("left_pct", l),
            ("top_pct", t),
            ("width_pct", w),
            ("height_pct", h),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v))
        .collect(),
    })
}

fn set_steps(d: &Document, id: NodeId, steps: Vec<AdjustEntry>) {
    d.edit_image_adjustments(
        id,
        &AdjustmentsWrite {
            preset: None,
            strength: None,
            steps: Some(steps),
            dither: None,
        },
    )
    .unwrap();
}

fn snap(d: &Document, id: NodeId) -> ImageSnapshot {
    d.read_objects()
        .objects
        .iter()
        .find_map(|o| match o {
            ObjectSnapshot::Image(i) if i.id == id => Some(i.clone()),
            _ => None,
        })
        .unwrap()
}

fn close(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() < EPS && (a.y - b.y).abs() < EPS
}

fn unit(u: f64, v: f64) -> Point {
    placement().apply_point(Point::new(u, v))
}

fn zip_member(bytes: &[u8], pred: impl Fn(&str) -> bool) -> Vec<(String, Vec<u8>)> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let names: Vec<String> = (0..zip.len())
        .map(|i| zip.by_index(i).unwrap().name().unwrap().into_owned())
        .collect();
    let mut out = Vec::new();
    for n in names.into_iter().filter(|n| pred(n)) {
        let mut v = Vec::new();
        zip.by_name(&n).unwrap().read_to_end(&mut v).unwrap();
        out.push((n, v));
    }
    out
}

fn format_of(bytes: &[u8]) -> u64 {
    let m = zip_member(bytes, |n| n == "manifest.json");
    serde_json::from_slice::<Value>(&m[0].1).unwrap()["format_version"]
        .as_u64()
        .unwrap()
}

// ---- criterion 5: the visible frame -----------------------------------------------------------

#[test]
fn c5_a_crop_narrows_the_frame_to_the_cropped_part_at_the_same_scale() {
    let d = Document::new(1);
    let id = image(&d);
    let whole = snap(&d, id);
    set_steps(&d, id, vec![crop(20, 10, 50, 60, true)]);
    let s = snap(&d, id);
    let want = [
        unit(0.2, 0.1),
        unit(0.7, 0.1),
        unit(0.7, 0.7),
        unit(0.2, 0.7),
    ];
    for (got, w) in s.corners().iter().zip(want) {
        assert!(close(*got, w), "{got:?} vs {w:?}");
    }
    // The stored placement and the whole frame stay; the visible part keeps its place and size.
    assert_eq!(
        s.placement, whole.placement,
        "image_placement kept (ADR 0072 §3)"
    );
    for (a, b) in s.full_corners().iter().zip(whole.corners()) {
        assert!(close(*a, b), "the whole frame is unchanged");
    }
    // Width and height of the object shrink to 50 % x 60 % of 40 x 20 mm.
    let c = s.corners();
    let w = ((c[1].x - c[0].x).powi(2) + (c[1].y - c[0].y).powi(2)).sqrt();
    let h = ((c[3].x - c[0].x).powi(2) + (c[3].y - c[0].y).powi(2)).sqrt();
    assert!(
        (w - 20.0).abs() < 1e-9 && (h - 12.0).abs() < 1e-9,
        "{w} x {h}"
    );
}

#[test]
fn c5_removing_or_switching_off_the_crop_is_lossless() {
    let d = Document::new(1);
    let id = image(&d);
    let whole = snap(&d, id);
    set_steps(&d, id, vec![crop(33, 41, 7, 9, true)]);
    set_steps(&d, id, vec![crop(33, 41, 7, 9, false)]);
    assert_eq!(
        snap(&d, id).corners(),
        whole.corners(),
        "a crop that is off"
    );
    set_steps(&d, id, vec![crop(33, 41, 7, 9, true)]);
    set_steps(&d, id, vec![]);
    assert_eq!(snap(&d, id).corners(), whole.corners(), "the crop removed");
    assert_eq!(snap(&d, id).placement, whole.placement);
    assert_eq!(snap(&d, id).asset, whole.asset);
}

#[test]
fn c5_two_crops_compose() {
    let d = Document::new(1);
    let id = image(&d);
    set_steps(
        &d,
        id,
        vec![crop(50, 0, 50, 100, true), crop(0, 50, 50, 50, true)],
    );
    // The second keeps the bottom-left quarter of the right half: u 0.5..0.75, v 0.5..1.0.
    let want = [
        unit(0.5, 0.5),
        unit(0.75, 0.5),
        unit(0.75, 1.0),
        unit(0.5, 1.0),
    ];
    for (got, w) in snap(&d, id).corners().iter().zip(want) {
        assert!(close(*got, w), "{got:?} vs {w:?}");
    }
}

#[test]
fn c5_a_damaged_crop_does_not_narrow_the_frame() {
    let d = Document::new(1);
    let id = image(&d);
    let whole = snap(&d, id).corners();
    for bad in [
        crop(60, 0, 50, 50, true),
        crop(0, 0, 0, 50, true),
        crop(-5, 0, 50, 50, true),
    ] {
        set_steps(&d, id, vec![bad]);
        assert_eq!(
            snap(&d, id).corners(),
            whole,
            "a crop that cannot run keeps the frame"
        );
    }
}

// ---- criterion 18: the asset is never modified ------------------------------------------------

#[test]
fn c18_the_asset_bytes_are_identical_after_a_crop_and_reopen() {
    let d = Document::new(1);
    let id = image(&d);
    let before = pack(&d, "0.1.0").unwrap();
    set_steps(&d, id, vec![crop(10, 10, 20, 20, true)]);
    let after = pack(&d, "0.1.0").unwrap();
    let assets = |b: &[u8]| zip_member(b, |n| n.starts_with("assets/"));
    let (a0, a1) = (assets(&before), assets(&after));
    assert!(!a0.is_empty());
    assert_eq!(a0, a1, "same asset paths and bytes");
    assert!(
        a1.iter().any(|(_, v)| v.as_slice() == &*png()),
        "the imported bytes themselves"
    );
    let back = unpack(2, &after).unwrap();
    assert_eq!(snap(&back, id).asset, snap(&d, id).asset);
    assert_eq!(snap(&back, id).corners(), snap(&d, id).corners());
}

// ---- criterion 8: format 52 and the golden ----------------------------------------------------

#[test]
fn c8_slice2_saves_format_52_and_the_crop_golden_opens_and_saves_back() {
    let d = Document::new(1);
    let id = image(&d);
    set_steps(&d, id, vec![crop(10, 10, 20, 20, true)]);
    assert_eq!(format_of(&pack(&d, "0.1.0").unwrap()), 59);

    let golden = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/image_adjust_crop_v52.curvyo"),
    )
    .unwrap();
    assert_eq!(format_of(&golden), 52);
    let g = unpack(3, &golden).unwrap();
    let kinds: Vec<String> = g
        .read_objects()
        .objects
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Image(i) => Some(i.adjustments.steps.clone()),
            _ => None,
        })
        .flatten()
        .filter_map(|e| e.step().map(|s| s.kind.clone()))
        .collect();
    for k in ["crop", "resize", "channel_mix", "curves"] {
        assert!(kinds.iter().any(|x| x == k), "golden holds {k}: {kinds:?}");
    }
    let again = unpack(4, &pack(&g, "0.1.0").unwrap()).unwrap();
    let corners = |d: &Document| -> Vec<[Point; 4]> {
        d.read_objects()
            .objects
            .iter()
            .filter_map(|o| match o {
                ObjectSnapshot::Image(i) => Some(i.corners()),
                _ => None,
            })
            .collect()
    };
    assert_eq!(corners(&g), corners(&again), "the frames survive a save");
}
