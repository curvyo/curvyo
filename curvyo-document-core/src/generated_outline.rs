//! The outline a generated node reads as: its stored subpaths as closed anchors, `frame` applied.

use crate::affine::Affine;
use crate::generated_codec::GeneratedSnapshot;
use crate::path_model::{AnchorId, AnchorKind, AnchorSnapshot, PathSnapshot, SubpathSnapshot};
use crate::units::{Angle, Point, Vec2};

/// One subpath of the record as anchors, `frame` applied. The ids are synthesised from `ordinal`
/// and are never written; "Convert to paths" mints real ones.
pub(crate) fn subpath_anchors(numbers: &[f64], frame: Affine, ordinal: u64) -> Vec<AnchorSnapshot> {
    let start = Point::new(numbers[0], numbers[1]);
    let cubics: Vec<[Point; 3]> = numbers[2..]
        .as_chunks::<6>()
        .0
        .iter()
        .map(|c| {
            [
                Point::new(c[0], c[1]),
                Point::new(c[2], c[3]),
                Point::new(c[4], c[5]),
            ]
        })
        .collect();
    // The walk comes back to the start: the last end point is the first anchor.
    let closes = cubics.last().is_some_and(|last| {
        (last[2].x - start.x).abs() < 1e-9 && (last[2].y - start.y).abs() < 1e-9
    });
    let mut points = vec![start];
    points.extend(cubics.iter().map(|cubic| cubic[2]));
    if closes {
        points.pop();
    }
    let count = points.len();
    (0..count)
        .map(|index| {
            let point = points[index];
            let out = cubics
                .get(index)
                .map_or(Vec2::ZERO, |cubic| point.vector_to(cubic[0]));
            let incoming = index
                .checked_sub(1)
                .or_else(|| closes.then(|| cubics.len() - 1))
                .and_then(|from| cubics.get(from));
            let handle_in = incoming.map_or(Vec2::ZERO, |cubic| point_to(cubic[1], point));
            AnchorSnapshot {
                id: AnchorId::new(0, ordinal << 20 | u64::try_from(index).unwrap_or(0)),
                point: frame.apply_point(point),
                handle_in: frame.apply_vector(handle_in),
                handle_out: frame.apply_vector(out),
                kind: AnchorKind::Corner,
            }
        })
        .collect()
}

/// The vector from `to` to `from`'s far end: a handle relative to its own anchor.
fn point_to(control: Point, anchor: Point) -> Vec2 {
    anchor.vector_to(control)
}

/// The outline a generated node reads as: every subpath of every part as one compound path,
/// closed, `frame` applied, with the style and the turn of the node.
pub(crate) fn outline(
    snapshot: &GeneratedSnapshot,
    style: crate::style_model::Style,
) -> PathSnapshot {
    let mut subpaths: Vec<SubpathSnapshot> = Vec::new();
    // One run per painted part, in part order (ADR 0091 P4); the indices count the first outline as 0.
    let mut paint_runs: Vec<crate::PaintRun> = Vec::new();
    let mut seen = 0_u32;
    for (index, part) in snapshot.record.parts.iter().enumerate() {
        let count = u32::try_from(part.len()).unwrap_or(u32::MAX);
        let paint = snapshot
            .record
            .part_keys
            .get(index)
            .and_then(|key| snapshot.record.part_paints.get(key));
        if let Some(paint) = paint {
            paint_runs.push(crate::PaintRun {
                first: seen,
                count,
                paint: *paint,
            });
        }
        seen += count;
    }
    for (ordinal, subpath) in snapshot.record.parts.iter().flatten().enumerate() {
        subpaths.push(SubpathSnapshot {
            closed: true,
            anchors: subpath_anchors(
                subpath,
                snapshot.frame,
                u64::try_from(ordinal).unwrap_or(0) + 1,
            ),
        });
    }
    let mut rest = subpaths.into_iter();
    // invariant: a stored record holds at least one non-empty part (refused at open otherwise).
    #[allow(clippy::expect_used)]
    let first = rest.next().expect("a validated record has a subpath");
    PathSnapshot {
        id: snapshot.id,
        closed: true,
        style,
        anchors: first.anchors,
        extra_subpaths: rest.collect(),
        curve_kind: crate::CurveKind::default(),
        spiro_transform: None,
        width_points: Vec::new(),
        rotation: Angle::from_radians(snapshot.frame.b.atan2(snapshot.frame.a)),
        paint_runs,
    }
}
