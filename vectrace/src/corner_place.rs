//! Places the structural corners a chain's turn found (`specs/0089` adrs.md §5): each at the
//! crossing of the total-least-squares lines of its two sides, a blunt run of close corners as
//! one acute tip.

use kurbo::Point;

use crate::corners::{Corner, midpoint};
use crate::line::{Line, fit_line};

/// A side point may lie this far from the side's line (digitisation of the traced raster, px).
const SIDE_RESIDUAL_PX: f64 = 0.6;

/// A side line needs this many points.
const SIDE_MIN_POINTS: usize = 3;

/// The zone around a corner where the digitised chain does not follow its sides, source px.
const ZONE_MIN: f64 = 1.0;
const ZONE_MAX: f64 = 4.0;

/// A crossing of the side lines further than this beyond the zone from every nearby vertex is
/// not trusted, source px.
const REACH: f64 = 0.5;

/// The cosine of the sharpest turn (160 degrees, a 20 degree tip) a run of corners merges into.
const TIP_MIN_COS: f64 = -0.94;

/// How far an acute tip may reach beyond its digitised end, source px.
const TIP_REACH: f64 = 1.5;

/// The corners at the vertices `kept` (with their turn cosines), placed: runs of close corners
/// merged into one tip where their outer sides cross sharply, every other corner at the
/// crossing of its side lines, or on the one side line when the other side is too short, or at
/// its vertex.
pub(crate) fn place(
    chain: &[Point],
    kept: &[(usize, f64)],
    corner_cos: f64,
    unit: usize,
) -> Vec<Corner> {
    let n = chain.len();
    #[allow(clippy::cast_precision_loss)] // 1 or 2
    let scale = unit as f64;
    let list: Vec<Corner> = kept
        .iter()
        .map(|&(k, c)| Corner {
            vertex: k,
            at: chain[k],
            zone: zone(c) * scale,
            ahead: None,
            behind: None,
        })
        .collect();
    let (mut list, merged) = merge_runs(chain, &list, corner_cos, unit, scale);
    for i in 0..list.len() {
        if merged[i] {
            continue;
        }
        let reach = reaches(&list, i, n);
        let (forward, backward) = sides(chain, &list[i], reach, scale);
        let at = match (forward, backward) {
            (Some(_), Some(_)) => cross(chain, &list[i], forward, backward, scale),
            (Some(line), None) | (None, Some(line)) => Some(line.project(chain[list[i].vertex])),
            (None, None) => None,
        };
        if let Some(at) = at {
            list[i].at = at;
        }
        list[i].ahead = forward.map(|l| l.direction);
        list[i].behind = backward.map(|l| l.direction);
    }
    list
}

/// The arc lengths from corner `i` of `list` to where its neighbours' zones begin, ahead and
/// behind.
fn reaches(list: &[Corner], i: usize, n: usize) -> (f64, f64) {
    let m = list.len();
    let (next, previous) = (&list[(i + 1) % m], &list[(i + m - 1) % m]);
    (
        side_length(list[i].vertex, next.vertex, n) - next.zone,
        side_length(previous.vertex, list[i].vertex, n) - previous.zone,
    )
}

/// The side lines ahead of and behind `corner`, each up to its reach.
fn sides(
    chain: &[Point],
    corner: &Corner,
    (ahead, behind): (f64, f64),
    scale: f64,
) -> (Option<Line>, Option<Line>) {
    let n = chain.len();
    let k = corner.vertex;
    // Midpoint `k + t` lies t + 0.5 ahead of the vertex, midpoint `k - 1 - t` as far behind.
    (
        side_line(corner.zone, ahead, scale, |t| midpoint(chain, k + t)),
        side_line(corner.zone, behind, scale, |t| {
            midpoint(chain, k + 2 * n - 1 - t)
        }),
    )
}

/// Where the side lines cross, if they do within the corner's zone plus [`REACH`] of its
/// vertex.
fn cross(
    chain: &[Point],
    corner: &Corner,
    forward: Option<Line>,
    backward: Option<Line>,
    scale: f64,
) -> Option<Point> {
    let at = forward?.intersect(&backward?)?;
    near_chain(chain, corner.vertex, at, corner.zone + REACH * scale).then_some(at)
}

/// Whether `at` lies within `reach` of a vertex of `chain` near vertex `k` (as far along the
/// chain as `reach` plus two): a crossing beyond a round or tapered end is no corner.
fn near_chain(chain: &[Point], k: usize, at: Point, reach: f64) -> bool {
    let n = chain.len();
    // invariant: `reach` is a few pixels, so this is a small whole number.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let window = (reach.ceil() as usize + 2).min(n / 2);
    (0..=2 * window).any(|t| {
        let d = at - chain[(k + n - window + t) % n];
        d.dot(d) <= reach * reach
    })
}

/// Corners at most this far apart along the chain, source px, may be one corner: the
/// digitised apex of an acute tip, or a corner the turn found on either side.
const TIP_GAP: usize = 6;

/// Every run of corners spanning at most [`TIP_GAP`] becomes one corner when the side before the
/// run and the side after it cross sharper than the Corners angle near the run's middle (two
/// parallel sides, as at the square end of a thin line, keep the run). Returns `list` and which
/// corners are placed already.
fn merge_runs(
    chain: &[Point],
    list: &[Corner],
    corner_cos: f64,
    unit: usize,
    scale: f64,
) -> (Vec<Corner>, Vec<bool>) {
    let n = chain.len();
    let m = list.len();
    let gap = |i: usize| (list[(i + 1) % m].vertex + n - list[i].vertex) % n;
    let close = |i: usize| gap(i) <= TIP_GAP * unit;
    // A run starts after a wide gap; without one the corners are left alone.
    let Some(first) = (0..m).find(|&i| !close((i + m - 1) % m)) else {
        return (list.to_vec(), vec![false; m]);
    };
    let mut out: Vec<(Corner, bool)> = Vec::with_capacity(m);
    let mut i = 0;
    while i < m {
        let start = (first + i) % m;
        // A run spans at most `TIP_GAP` from its first corner to its last.
        let mut len = 1;
        while len < m
            && close((start + len - 1) % m)
            && (list[(start + len) % m].vertex + n - list[start].vertex) % n <= TIP_GAP * unit
        {
            len += 1;
        }
        let run: Vec<usize> = (0..len).map(|t| (start + t) % m).collect();
        i += len;
        let merged = (len > 1)
            .then(|| merge(chain, list, &run, corner_cos, scale))
            .flatten();
        match merged {
            Some(corner) => out.push((corner, true)),
            None => out.extend(run.iter().map(|&r| (list[r], false))),
        }
    }
    out.sort_by_key(|(c, _)| c.vertex);
    out.into_iter().unzip()
}

/// The one corner the run `run` of `list` stands for, if its outer sides cross sharply near it.
fn merge(
    chain: &[Point],
    list: &[Corner],
    run: &[usize],
    corner_cos: f64,
    scale: f64,
) -> Option<Corner> {
    let n = chain.len();
    let (a, b) = (run[0], run[run.len() - 1]);
    let behind = reaches(list, a, n).1;
    let ahead = reaches(list, b, n).0;
    let backward = sides(chain, &list[a], (0.0, behind), scale).1?;
    let forward = sides(chain, &list[b], (ahead, 0.0), scale).0?;
    let turn = -backward.direction.dot(forward.direction);
    // Sharper than the Corners angle, but not nearly parallel: sides closer than 20 degrees
    // are the two sides of a thin stroke with a square end, not one tip.
    if turn >= corner_cos || turn < TIP_MIN_COS {
        return None;
    }
    let span = (list[b].vertex + n - list[a].vertex) % n;
    #[allow(clippy::cast_precision_loss)] // a few pixels
    let half = span as f64 / 2.0;
    let mid = (list[a].vertex + span / 2) % n;
    let tip = Corner {
        vertex: mid,
        at: chain[mid],
        zone: (zone(turn) * scale).max(half + scale),
        ahead: Some(forward.direction),
        behind: Some(backward.direction),
    };
    let at = forward.intersect(&backward)?;
    // Near the run: a tip's digitised end falls short by about 1.5 px (36 degrees); a taper
    // whose sides meet further out keeps its corners.
    let reach = (TIP_REACH + REACH) * scale;
    near_chain(chain, mid, at, reach + half).then_some(Corner { at, ..tip })
}

/// The edges from vertex `from` forward to vertex `to` (the whole chain when equal).
fn side_length(from: usize, to: usize, n: usize) -> f64 {
    let d = (to + n - from) % n;
    #[allow(clippy::cast_precision_loss)] // chains are far shorter than 2^52
    let edges = if d == 0 { n } else { d } as f64;
    edges
}

/// The zone of a corner turning with cosine `c`, in source pixels: half a pixel plus where the
/// two sides are closer than a pixel, `0.5 + 0.5 cot(α/2)` for the inner angle `α`.
fn zone(c: f64) -> f64 {
    if 1.0 + c < 1e-9 {
        return ZONE_MAX;
    }
    (0.5 + 0.5 * ((1.0 - c) / (1.0 + c)).sqrt()).clamp(ZONE_MIN, ZONE_MAX)
}

/// The straight line of the side points `point(t)` at arc `t + 0.5` between `zone` and `reach`.
fn side_line(zone: f64, reach: f64, scale: f64, point: impl Fn(usize) -> Point) -> Option<Line> {
    let mut far = reach;
    while far - zone >= 2.0 * scale {
        #[allow(clippy::cast_precision_loss)] // chains are far shorter than 2^52
        let points: Vec<Point> = (0..)
            .map(|t: usize| (t, t as f64 + 0.5))
            .skip_while(|&(_, a)| a < zone)
            .take_while(|&(_, a)| a <= far)
            .map(|(t, _)| point(t))
            .collect();
        if points.len() >= SIDE_MIN_POINTS
            && let Some(line) = fit_line(&points)
            && line.max_offset(&points) <= SIDE_RESIDUAL_PX
        {
            return Some(line);
        }
        far = zone + (far - zone) / 2.0;
    }
    None
}
