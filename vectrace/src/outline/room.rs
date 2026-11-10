//! How much room the parts of an outline have between each other, and the smoothing and
//! tolerance budgets that follow from it (`clearance.rs` measures the clearances).

use kurbo::Point;

use super::Bound;
use crate::corners::{Corner, midpoint};
use crate::line::norm;

/// Two outlines keep this much room between them, traced px (the guard's gap).
pub(super) const GAP: f64 = 0.1;

/// The share of the room between two smoothed contours a fit may use on its side (the guard
/// catches the rare crossing when both sides use more than half).
const SMOOTH_SHARE: f64 = 1.0;

/// The least room a part of an outline is ever given, traced px.
const ROOM_MIN: f64 = 0.05;

/// The share of a held point's room the smoothing may use (the fit keeps the rest), whatever
/// the Tolerance, so a looser tolerance never smooths less.
const SMOOTH_PART: f64 = 0.75;

/// The room a chain's parts have, from its clearances: per segment of the midpoint polyline
/// (from the midpoint of edge `k` to that of edge `k + 1`) and per midpoint; `inset` is taken
/// off everywhere (half a pixel for a contour on the pixel corners, which lie that far outside
/// the midpoints).
pub(super) struct Room<'a> {
    pub(super) clearance: &'a [f64],
    pub(super) inset: f64,
}

impl Room<'_> {
    /// Half the clearance of segment `k` less the gap: how far the outline may move there.
    pub(super) fn segment(&self, k: usize) -> f64 {
        let n = self.clearance.len();
        (self.clearance[k % n] / 2.0 - GAP - self.inset).max(ROOM_MIN)
    }

    /// The room at the midpoint of edge `k`: the smaller of its two segments'.
    pub(super) fn point(&self, k: usize) -> f64 {
        let n = self.clearance.len();
        self.segment((k + n - 1) % n).min(self.segment(k))
    }

    /// The room of a corner at vertex `vertex` with `zone`: the least over the edges of its
    /// zone on both sides.
    pub(super) fn corner(&self, vertex: usize, zone: f64) -> f64 {
        let n = self.clearance.len();
        // invariant: zones are a few pixels.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let z = (zone.ceil() as usize).max(1).min(n / 2);
        (0..=2 * z)
            .map(|t| self.segment((vertex + 2 * n - 1 - z + t) % n))
            .fold(f64::INFINITY, f64::min)
    }
}

/// `values` halved.
pub(super) fn halved(values: &[f64]) -> Vec<f64> {
    values.iter().map(|v| v / 2.0).collect()
}

/// What bounds a contour's smoothing and tolerances: the walked room, the bound, the
/// tolerance.
pub(super) struct Limits<'a> {
    pub(super) room: &'a Room<'a>,
    pub(super) bound: Bound<'a>,
    pub(super) tolerance: f64,
    /// The contour runs on the pixel corners instead of the edge midpoints.
    pub(super) on_corners: bool,
}

impl Limits<'_> {
    /// The walked point of edge `e`: its midpoint, or the pixel corner it starts at.
    pub(super) fn point(&self, chain: &[Point], e: usize) -> Point {
        if self.on_corners {
            chain[e % chain.len()]
        } else {
            midpoint(chain, e)
        }
    }

    /// The tolerance per segment of `smoothed` (one fewer than points, or as many when `cyclic`;
    /// `slots` names each segment's chain edge). A point whose room among the smoothed contours
    /// has collapsed to less than half its walked room (a thin feature smoothing closed), or
    /// every point under [`Bound::Walked`], is first pulled back towards its walked position so
    /// it keeps at least half its room, or the tolerance, for the fit; segments at such points
    /// get their walked room less what smoothing still used, the others the bound's tolerance.
    pub(super) fn budgets(
        &self,
        raw: &[Point],
        smoothed: &mut [Point],
        point_room: &[f64],
        segment_room: &[f64],
        slots: &[usize],
        cyclic: bool,
    ) -> Vec<f64> {
        let n = raw.len();
        let segments = if cyclic { n } else { n - 1 };
        let slot = |k: usize| slots[k.min(slots.len() - 1)];
        let held: Vec<bool> = (0..n)
            .map(|k| match self.bound {
                Bound::Walked => true,
                Bound::Free => false,
                Bound::Smoothed(smooth) => {
                    let walked = 2.0 * (point_room[k] + GAP);
                    smooth[slot(k)] < 0.5 * walked
                }
            })
            .collect();
        let moved = clamp_moves(raw, smoothed, point_room, &held);
        (0..segments)
            .map(|i| {
                let next = (i + 1) % n;
                if held[i] || held[next] {
                    let budget = segment_room[i] - moved[i].max(moved[next]);
                    self.tolerance.min(budget).max(ROOM_MIN)
                } else {
                    match self.bound {
                        Bound::Smoothed(smooth) => self
                            .tolerance
                            .min(smooth[slot(i)] * SMOOTH_SHARE - GAP)
                            .max(ROOM_MIN),
                        _ => self.tolerance,
                    }
                }
            })
            .collect()
    }
}

/// A corner's position held within its room: the crossing of its sides, pulled back towards
/// its vertex when the room around the vertex does not reach it.
pub(super) fn held(chain: &[Point], corner: Corner, room: &Room<'_>) -> Point {
    let vertex = chain[corner.vertex];
    let reach = room.corner(corner.vertex, corner.zone);
    let offset = corner.at - vertex;
    let distance = norm(offset);
    if distance <= reach {
        corner.at
    } else {
        vertex + offset * (reach / distance)
    }
}

/// Pulls every `held` smoothed point back towards its raw point so that it keeps
/// `1 - SMOOTH_PART` of its room for the fit; returns how far each point moved.
pub(super) fn clamp_moves(
    raw: &[Point],
    smoothed: &mut [Point],
    room: &[f64],
    held: &[bool],
) -> Vec<f64> {
    raw.iter()
        .zip(smoothed.iter_mut())
        .zip(room.iter().zip(held))
        .map(|((&p, q), (&r, &held))| {
            let limit = r * SMOOTH_PART;
            let offset = *q - p;
            let moved = norm(offset);
            if held && moved > limit {
                *q = p + offset * (limit / moved);
                limit
            } else {
                moved
            }
        })
        .collect()
}
