//! Criterion 11 of `specs/0085-stroke-to-path` and `adrs.md` decision 4: the stroke outline,
//! filled, paints what the canvas paints for the stroke. Each shape is drawn by
//! `curvyo-render-core` before (its `lyon` stroke, dashes and markers) and after Stroke to path
//! (the fill of the outline), both rasterised at 20 px per mm. For the shapes of criterion 11 at
//! most 0.5 % of the stroke's pixels may differ; for every shape, the thin dashed curves with
//! markers and the long dashed path of 29 (d) included, every differing pixel lies on an edge
//! (within 1 px, 0.05 mm: stroke edges, dash ends and marker positions).

#![allow(
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    missing_docs
)]

use std::collections::HashSet;

use curvyo_document_core::{
    AnchorKind, DashPattern, Document, Length, LineCap, LineJoin, MarkerPlace, MarkerShape,
    NewAnchor, NodeId, Point, StyleEdit, Vec2, ViewTransform,
};
use curvyo_render_core::build_artwork;
use curvyo_ui_core::{AnchorIdMinter, plan_stroke_to_path};

/// Pixels per millimetre.
const SCALE: f64 = 20.0;

/// One shape: anchors `(point, handle in, handle out)`, closed, and its style edits.
struct Case {
    name: String,
    anchors: Vec<(Point, Vec2, Vec2)>,
    closed: bool,
    edits: Vec<StyleEdit>,
}

fn corners(points: &[(f64, f64)]) -> Vec<(Point, Vec2, Vec2)> {
    points
        .iter()
        .map(|&(x, y)| (Point::new(x, y), Vec2::new(0.0, 0.0), Vec2::new(0.0, 0.0)))
        .collect()
}

fn circle(r: f64) -> Vec<(Point, Vec2, Vec2)> {
    let k = 0.552_284_749_830_793_4 * r;
    let a = |x: f64, y: f64, hin: (f64, f64), hout: (f64, f64)| {
        (
            Point::new(x, y),
            Vec2::new(hin.0, hin.1),
            Vec2::new(hout.0, hout.1),
        )
    };
    vec![
        a(r, 0.0, (0.0, -k), (0.0, k)),
        a(0.0, r, (k, 0.0), (-k, 0.0)),
        a(-r, 0.0, (0.0, k), (0.0, -k)),
        a(0.0, -r, (-k, 0.0), (k, 0.0)),
    ]
}

/// The 12-node path of criterion 11, as in the kernel's golden test.
fn bezier_path() -> Vec<(Point, Vec2, Vec2)> {
    let a = |x: f64, y: f64, hin: (f64, f64), hout: (f64, f64)| {
        (
            Point::new(x, y),
            Vec2::new(hin.0, hin.1),
            Vec2::new(hout.0, hout.1),
        )
    };
    vec![
        a(0.0, 0.0, (0.0, 0.0), (6.0, -4.0)),
        a(12.0, -2.0, (-3.0, -2.0), (3.0, 2.0)),
        a(20.0, 6.0, (0.0, 0.0), (0.0, 0.0)),
        a(30.0, 0.0, (0.0, 0.0), (4.0, 4.0)),
        a(36.0, 10.0, (0.0, -4.0), (0.0, 4.0)),
        a(30.0, 20.0, (4.0, 0.0), (0.0, 0.0)),
        a(20.0, 16.0, (0.0, 0.0), (0.0, 0.0)),
        a(12.0, 24.0, (4.0, -2.0), (-4.0, 2.0)),
        a(2.0, 22.0, (0.0, 0.0), (0.0, 0.0)),
        a(-4.0, 14.0, (0.0, 3.0), (0.0, -3.0)),
        a(4.0, 8.0, (-3.0, 0.0), (3.0, 0.0)),
        a(14.0, 10.0, (-2.0, -2.0), (0.0, 0.0)),
    ]
}

/// A wavy open path of `nodes` smooth nodes about 1 mm apart (the long dashed path of criterion
/// 29 (d), shortened to stay under the canvas's 2,000 dashes per object).
fn wavy(nodes: usize) -> Vec<(Point, Vec2, Vec2)> {
    let points: Vec<Point> = (0..nodes)
        .map(|i| {
            let x = i as f64;
            Point::new(x, 3.0 * (x / 7.0).sin())
        })
        .collect();
    (0..nodes)
        .map(|i| {
            let (prev, next) = (points[i.saturating_sub(1)], points[(i + 1).min(nodes - 1)]);
            let t = Vec2::new((next.x - prev.x) / 6.0, (next.y - prev.y) / 6.0);
            (points[i], Vec2::new(-t.x, -t.y), t)
        })
        .collect()
}

fn width(mm: f64) -> StyleEdit {
    StyleEdit::StrokeWidth(Length::from_mm(mm))
}

fn dash(pattern: &[f64]) -> StyleEdit {
    StyleEdit::StrokeDash(DashPattern::new(pattern.to_vec()).unwrap())
}

#[allow(clippy::too_many_lines)] // a flat table of cases
fn cases() -> Vec<Case> {
    let straight = || corners(&[(0.0, 0.0), (100.0, 0.0)]);
    let mut cases = Vec::new();
    let mut add = |name: String, anchors, closed, edits| {
        cases.push(Case {
            name,
            anchors,
            closed,
            edits,
        });
    };
    for cap in [LineCap::Butt, LineCap::Square, LineCap::Round] {
        add(
            format!("ac4 {cap:?}"),
            straight(),
            false,
            vec![width(2.0), StyleEdit::StrokeCap(cap)],
        );
        add(
            format!("ac8 dash 5 5 {cap:?}"),
            straight(),
            false,
            vec![width(1.0), StyleEdit::StrokeCap(cap), dash(&[5.0, 5.0])],
        );
    }
    for join in [LineJoin::Miter, LineJoin::Round, LineJoin::Bevel] {
        add(
            format!("ac5 square {join:?}"),
            corners(&[(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)]),
            true,
            vec![width(2.0), StyleEdit::StrokeJoin(join)],
        );
        for cap in [LineCap::Butt, LineCap::Square, LineCap::Round] {
            add(
                format!("ac11 bezier {join:?} {cap:?}"),
                bezier_path(),
                false,
                vec![
                    width(2.0),
                    StyleEdit::StrokeJoin(join),
                    StyleEdit::StrokeCap(cap),
                ],
            );
        }
    }
    add(
        "ac6 sharp corner".into(),
        corners(&[(0.0, 0.0), (50.0, 0.0), (0.0, 5.0)]),
        false,
        vec![width(2.0)],
    );
    add("ac7 circle".into(), circle(10.0), true, vec![width(2.0)]);
    add(
        "ac8 dash around a corner".into(),
        corners(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]),
        false,
        vec![width(1.0), dash(&[15.0, 5.0])],
    );
    add(
        "ac9 figure eight".into(),
        corners(&[(0.0, 0.0), (20.0, 20.0), (20.0, 0.0), (0.0, 20.0)]),
        false,
        vec![width(3.0)],
    );
    add(
        "ac10 markers".into(),
        corners(&[(0.0, 0.0), (30.0, 0.0), (30.0, 20.0), (60.0, 25.0)]),
        false,
        vec![
            width(1.0),
            StyleEdit::MarkerStart(MarkerShape::Dot),
            StyleEdit::MarkerEnd(MarkerShape::Arrow),
            StyleEdit::MarkerMid(MarkerShape::Arrow),
            StyleEdit::MarkerPlace(MarkerPlace::AtNodes),
        ],
    );
    add(
        "extra: spaced dots on a dashed curve".into(),
        bezier_path(),
        false,
        vec![
            width(0.5),
            StyleEdit::StrokeCap(LineCap::Round),
            dash(&[4.0, 3.0, 0.0, 3.0]),
            StyleEdit::MarkerMid(MarkerShape::Dot),
            StyleEdit::MarkerPlace(MarkerPlace::Spaced),
            StyleEdit::MarkerCount(curvyo_document_core::MarkerCount::new(5).unwrap()),
        ],
    );
    add(
        "extra: ac29d long dashed path".into(),
        wavy(1200),
        false,
        vec![
            width(0.5),
            StyleEdit::StrokeJoin(LineJoin::Round),
            StyleEdit::StrokeCap(LineCap::Round),
            dash(&[2.0, 1.0]),
        ],
    );
    cases
}

/// The pixels whose centres the triangles cover, at [`SCALE`].
fn raster(triangles: &[curvyo_render_core::Vertex]) -> HashSet<(i64, i64)> {
    let mut covered = HashSet::new();
    for tri in triangles.as_chunks::<3>().0 {
        let p: Vec<(f64, f64)> = tri
            .iter()
            .map(|v| (v.position.x * SCALE, v.position.y * SCALE))
            .collect();
        let edge = |a: (f64, f64), b: (f64, f64), c: (f64, f64)| {
            (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
        };
        let area = edge(p[0], p[1], p[2]);
        if area == 0.0 {
            continue;
        }
        let (x0, x1) = (
            p.iter().map(|q| q.0).fold(f64::MAX, f64::min).floor() as i64,
            p.iter().map(|q| q.0).fold(f64::MIN, f64::max).ceil() as i64,
        );
        let (y0, y1) = (
            p.iter().map(|q| q.1).fold(f64::MAX, f64::min).floor() as i64,
            p.iter().map(|q| q.1).fold(f64::MIN, f64::max).ceil() as i64,
        );
        for y in y0..=y1 {
            for x in x0..=x1 {
                let c = (x as f64 + 0.5, y as f64 + 0.5);
                let w = [
                    edge(p[0], p[1], c) * area.signum(),
                    edge(p[1], p[2], c) * area.signum(),
                    edge(p[2], p[0], c) * area.signum(),
                ];
                if w.iter().all(|&v| v >= 0.0) {
                    covered.insert((x, y));
                }
            }
        }
    }
    covered
}

fn artwork(document: &Document) -> Vec<curvyo_render_core::Vertex> {
    let objects: Vec<_> = document
        .object_ids()
        .into_iter()
        .filter_map(|id| document.object(id))
        .collect();
    build_artwork(
        &objects,
        ViewTransform::new(SCALE, Point::new(-200.0, -200.0)),
    )
    .triangles
}

fn make(case: &Case) -> (Document, NodeId) {
    let document = Document::new(1);
    let mut minter = AnchorIdMinter::new(7);
    let anchors: Vec<NewAnchor> = case
        .anchors
        .iter()
        .map(|&(point, handle_in, handle_out)| NewAnchor {
            id: minter.mint(),
            point,
            handle_in,
            handle_out,
            kind: AnchorKind::Corner,
        })
        .collect();
    let id = document.create_path(&anchors, case.closed);
    for edit in &case.edits {
        document.edit_style(&[id], edit).unwrap();
    }
    (document, id)
}

#[test]
fn ac11_the_filled_outline_paints_what_the_stroke_paints() {
    let mut failures = Vec::new();
    for case in cases() {
        let (document, id) = make(&case);
        let before = raster(&artwork(&document));
        let objects: Vec<_> = document.object(id).into_iter().collect();
        let plan = plan_stroke_to_path(
            &objects,
            &[id],
            &|_| false,
            &|_| None,
            &mut AnchorIdMinter::new(8),
        )
        .unwrap();
        document.stroke_to_path(&plan.write).unwrap();
        let after = raster(&artwork(&document));
        let differing: Vec<(i64, i64)> = before.symmetric_difference(&after).copied().collect();
        // On an edge: the pixel lies on the boundary of the stroke or of the filled outline (a
        // neighbour has the other state), so it is within 1 px of an edge.
        let on_edge = |&(x, y): &(i64, i64)| {
            [&before, &after].iter().any(|image| {
                let inside = image.contains(&(x, y));
                (-1..=1).any(|dy| (-1..=1).any(|dx| image.contains(&(x + dx, y + dy)) != inside))
            })
        };
        let off_edge = differing.iter().filter(|p| !on_edge(p)).count();
        let share = differing.len() as f64 / before.len().max(1) as f64;
        println!(
            "{}: {} of {} pixels differ ({:.3} %), {off_edge} off an edge",
            case.name,
            differing.len(),
            before.len(),
            share * 100.0
        );
        // The 0.5 % share is criterion 11's, for its shapes; the thin dashed paths of `adrs.md`
        // decision 4 are held to the edge rule (dash ends and markers within 1 px, 0.05 mm).
        let share_checked = !case.name.starts_with("extra");
        if (share_checked && share > 0.005) || off_edge > 0 {
            failures.push(case.name.clone());
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}
