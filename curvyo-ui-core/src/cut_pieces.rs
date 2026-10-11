//! Splits one contour into open pieces at its cut parameters (`specs/0036-cut-at-crossings`
//! criteria 3 to 5). Every cut is made on the exact curve with `subdivide_at_parameter`, so the
//! halves lie on the original curve and a line stays a line.
//!
//! A closed contour with `m` cuts gives `m` pieces (the last runs round the closing segment), an
//! open one `m + 1`. A new end node is a Corner node with a zero handle on its open side; every
//! other node keeps its place, handles and kind.

use curvyo_document_core::{AnchorKind, NewAnchor, Point, Vec2};
use curvyo_geometry_core::{CutParameter, subdivide_at_parameter};

use crate::anchor_id_minter::AnchorIdMinter;

/// One cubic between two points, handles relative to their own end.
#[derive(Clone, Copy)]
struct Curve {
    start: Point,
    start_out: Vec2,
    end_in: Vec2,
    end: Point,
}

/// How many pieces `cuts` make of a contour.
pub(crate) fn piece_count(closed: bool, cuts: usize) -> usize {
    if closed { cuts } else { cuts + 1 }
}

/// The part of the segment from `from` to `to` over the parameters `t0` to `t1`.
fn sub_curve(from: &NewAnchor, to: &NewAnchor, t0: f64, t1: f64) -> Curve {
    let mut curve = Curve {
        start: from.point,
        start_out: from.handle_out,
        end_in: to.handle_in,
        end: to.point,
    };
    let mut t1 = t1;
    if t0 > 0.0 {
        let cut = subdivide_at_parameter(curve.start, curve.start_out, curve.end_in, curve.end, t0);
        curve = Curve {
            start: cut.new_point,
            start_out: cut.new_handle_out,
            end_in: cut.next_in,
            end: curve.end,
        };
        t1 = (t1 - t0) / (1.0 - t0);
    }
    if t1 < 1.0 {
        let cut = subdivide_at_parameter(curve.start, curve.start_out, curve.end_in, curve.end, t1);
        curve = Curve {
            start: curve.start,
            start_out: cut.prev_out,
            end_in: cut.new_handle_in,
            end: cut.new_point,
        };
    }
    curve
}

/// Where a piece starts or ends.
#[derive(Clone, Copy)]
enum Stop {
    /// The first or last node of an open contour: kept as it is.
    Node,
    /// A cut: the new end node is a Corner with a zero handle on the open side.
    Cut,
}

/// The piece from `from` (a position `(segment, t)` in unrolled segment numbers) to `to`.
fn piece(
    anchors: &[NewAnchor],
    from: (usize, f64),
    to: (usize, f64),
    stops: (Stop, Stop),
    minter: &mut AnchorIdMinter,
) -> Vec<NewAnchor> {
    let count = anchors.len();
    let node = |k: usize| anchors[k % count];
    let mut curves: Vec<Curve> = Vec::new();
    for k in from.0..=to.0 {
        let t0 = if k == from.0 { from.1 } else { 0.0 };
        let t1 = if k == to.0 { to.1 } else { 1.0 };
        if t1 > t0 {
            curves.push(sub_curve(&node(k), &node(k + 1), t0, t1));
        }
    }
    let mut out = Vec::with_capacity(curves.len() + 1);
    // invariant: a piece spans at least some length of curve, so `curves` is not empty.
    let first = curves.first().copied();
    let last = curves.last().copied();
    let (Some(first), Some(last)) = (first, last) else {
        return out;
    };
    let start_node = node(from.0);
    out.push(match stops.0 {
        Stop::Node => NewAnchor {
            id: minter.mint(),
            point: first.start,
            handle_out: first.start_out,
            ..start_node
        },
        Stop::Cut => NewAnchor {
            id: minter.mint(),
            point: first.start,
            handle_in: Vec2::ZERO,
            handle_out: first.start_out,
            kind: AnchorKind::Corner,
        },
    });
    for (offset, pair) in curves.windows(2).enumerate() {
        let original = node(from.0 + offset + 1);
        out.push(NewAnchor {
            id: minter.mint(),
            handle_in: pair[0].end_in,
            handle_out: pair[1].start_out,
            ..original
        });
    }
    let end_node = node(to.0);
    out.push(match stops.1 {
        Stop::Node => NewAnchor {
            id: minter.mint(),
            point: last.end,
            handle_in: last.end_in,
            ..end_node
        },
        Stop::Cut => NewAnchor {
            id: minter.mint(),
            point: last.end,
            handle_in: last.end_in,
            handle_out: Vec2::ZERO,
            kind: AnchorKind::Corner,
        },
    });
    out
}

/// The open pieces of the contour `anchors` cut at `cuts` (sorted along it, at least one, in
/// range for the contour, as `cut_parameters` returns them), with fresh anchor ids.
pub(crate) fn split_contour(
    anchors: &[NewAnchor],
    closed: bool,
    cuts: &[CutParameter],
    minter: &mut AnchorIdMinter,
) -> Vec<Vec<NewAnchor>> {
    let count = anchors.len();
    let segments = if closed { count } else { count - 1 };
    let at = |cut: &CutParameter| (cut.segment, cut.t);
    let mut pieces = Vec::with_capacity(piece_count(closed, cuts.len()));
    if !closed {
        let ends: Vec<((usize, f64), Stop)> = std::iter::once(((0, 0.0), Stop::Node))
            .chain(cuts.iter().map(|cut| (at(cut), Stop::Cut)))
            .chain(std::iter::once(((segments, 0.0), Stop::Node)))
            .collect();
        for pair in ends.windows(2) {
            pieces.push(piece(
                anchors,
                pair[0].0,
                pair[1].0,
                (pair[0].1, pair[1].1),
                minter,
            ));
        }
        return pieces;
    }
    for (index, cut) in cuts.iter().enumerate() {
        let from = at(cut);
        let to = match cuts.get(index + 1) {
            Some(next) => at(next),
            // Round the closing segment to the first cut again.
            None => (cuts[0].segment + segments, cuts[0].t),
        };
        pieces.push(piece(anchors, from, to, (Stop::Cut, Stop::Cut), minter));
    }
    pieces
}
