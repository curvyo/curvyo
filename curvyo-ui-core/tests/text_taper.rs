//! The taper on a text on a circle (`specs/0079-text-layout-controls` criterion 19,
//! `adrs.md` decision 10; `specs/0080-text-on-path`): applied in each glyph's own frame before the
//! glyph is put on the carrier, the circle values make the stems of an "H" point at the centre,
//! outside the ring with t = h / (2(R + h/2)) and inside it with t = −h / (2(R − h/2)).
#![allow(clippy::unwrap_used, missing_docs)]

use std::path::PathBuf;

use curvyo_document_core::{
    Affine, CarrierRef, Document, EllipseFrame, FontKind, FontRef, Length, LineOnPath, NewAnchor,
    OnPathValues, PathSide, Point, TextChars, TextLayoutValues, TextOutline, Vec2,
};
use curvyo_text_core::{Face, FaceStore};
use curvyo_ui_core::{PathPlacement, lay_out, text_layout};

const CENTRE: Point = Point { x: 100.0, y: 100.0 };
const SIZE: f64 = 10.0;

fn sans() -> std::sync::Arc<Face> {
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../frontend/public/fonts/source-sans-3/SourceSans3-Regular.otf"),
    )
    .unwrap();
    let mut store = FaceStore::new();
    let key = store.add(bytes, 0).unwrap();
    store.get(&key).unwrap().clone()
}

fn font() -> FontRef {
    FontRef {
        family: "Source Sans 3".to_owned(),
        style: "Regular".to_owned(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind: FontKind::Outline,
    }
}

/// An "H" on the circle of radius 50 about (100, 100), at the top, with `taper`; whether its top
/// is further from the centre than its baseline, its baseline's radius, and the letter's frame
/// (a point on its baseline and its unit up direction).
fn h_on_circle(
    side: PathSide,
    line: LineOnPath,
    taper: f64,
) -> (TextOutline, bool, f64, (Point, Vec2)) {
    let d = Document::new(1);
    let carrier = d.create_ellipse(EllipseFrame {
        center: CENTRE,
        rx: Length::from_mm(50.0),
        ry: Length::from_mm(50.0),
    });
    let t = d
        .create_text_with_chars(
            "H",
            &font(),
            Length::from_mm(SIZE),
            Affine::translation(Vec2::new(3.0, 4.0)),
            &TextLayoutValues {
                taper,
                ..TextLayoutValues::DEFAULT
            },
            &TextChars::default(),
        )
        .unwrap();
    let values = OnPathValues {
        side,
        line,
        ..OnPathValues::at(Length::from_mm(0.0))
    };
    d.put_text_on_path(
        t,
        CarrierRef {
            node: carrier,
            outline: 0,
        },
        &values,
        true,
    )
    .unwrap();
    let text = d.text(t).unwrap();
    let face = sans();
    let laid = text_layout(&text, &face).unwrap();
    let placement = PathPlacement::new(&laid, text.on_path.as_ref().unwrap()).unwrap();
    let centre_x = laid.lines[0].glyphs[0].centre_x.0;
    let base = placement.point(0, centre_x, 0.0).unwrap();
    let top = placement.point(0, centre_x, -SIZE).unwrap();
    let up = Vec2::new((top.x - base.x) / SIZE, (top.y - base.y) / SIZE);
    let outline = lay_out(&text, &face, false).unwrap();
    (
        outline,
        radius(top) > radius(base),
        radius(base),
        (base, up),
    )
}

fn points(o: &TextOutline) -> Vec<Point> {
    let one = |anchors: &[NewAnchor]| anchors.iter().map(|a| a.point).collect::<Vec<_>>();
    let mut all = one(&o.glyphs.anchors);
    for sub in &o.glyphs.extra_subpaths {
        all.extend(one(&sub.anchors));
    }
    all
}

fn radius(p: Point) -> f64 {
    (p.x - CENTRE.x).hypot(p.y - CENTRE.y)
}

/// The outer stem edges of the H: the corners furthest along and against the line on its
/// lowest and on its highest line (in the letter's frame `base`, `up`), as two lines.
fn stems(o: &TextOutline, (base, up): (Point, Vec2)) -> [(Point, Point); 2] {
    let all = points(o);
    let height = |p: &Point| (p.x - base.x) * up.x + (p.y - base.y) * up.y;
    let along = |p: &Point| (p.x - base.x) * -up.y + (p.y - base.y) * up.x;
    let (low, high) = all
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), p| {
            (l.min(height(p)), h.max(height(p)))
        });
    let ends = |level: f64| {
        let set: Vec<Point> = all
            .iter()
            .copied()
            .filter(|p| (height(p) - level).abs() < 1e-6)
            .collect();
        let first = set
            .iter()
            .copied()
            .min_by(|a, b| along(a).total_cmp(&along(b)))
            .unwrap();
        let last = set
            .iter()
            .copied()
            .max_by(|a, b| along(a).total_cmp(&along(b)))
            .unwrap();
        (first, last)
    };
    let (low_first, low_last) = ends(low);
    let (high_first, high_last) = ends(high);
    [(low_first, high_first), (low_last, high_last)]
}

/// The distance of the circle's centre from the line through `a` and `b`.
fn miss(a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    ((CENTRE.x - a.x) * dy - (CENTRE.y - a.y) * dx).abs() / dx.hypot(dy)
}

#[test]
fn ac19_the_circle_values_point_the_stems_at_the_centre() {
    let mut inside = 0;
    for (side, line) in [
        (PathSide::Above, LineOnPath::Baseline),
        (PathSide::Below, LineOnPath::TopEdge),
        (PathSide::Below, LineOnPath::Baseline),
        (PathSide::Above, LineOnPath::TopEdge),
    ] {
        let (plain, top_out, baseline, frame) = h_on_circle(side, line, 0.0);
        // Outside the ring the letter's top is further out than its baseline; inside, the
        // letters point at the centre.
        inside += usize::from(!top_out);
        let taper = if top_out {
            SIZE / (2.0 * (baseline + SIZE / 2.0))
        } else {
            -SIZE / (2.0 * (baseline - SIZE / 2.0))
        };
        for (a, b) in stems(&plain, frame) {
            assert!(
                miss(a, b) > 1.0,
                "{side:?} {line:?}: untapered stems are parallel"
            );
        }
        let (tapered, _, _, frame) = h_on_circle(side, line, taper);
        for (a, b) in stems(&tapered, frame) {
            let off = miss(a, b);
            assert!(
                off < 0.05,
                "{side:?} {line:?} t = {taper}: {off} mm from the centre"
            );
        }
    }
    assert_eq!(
        inside, 2,
        "two of the four cases point the letters at the centre"
    );
}
