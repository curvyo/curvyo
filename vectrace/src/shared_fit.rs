//! Fits every shared edge of a multicolour label map once (`specs/0089` adrs.md §10, criterion
//! 28): each colour loop is a closed chain with its junction vertices pinned, an edge is fitted
//! in the loop of its larger label (its owner) and taken reversed by the other, so neighbours
//! share the same segments and a triple junction is one node; a fitted span that crosses goes
//! one level coarser, down to the walked contour, which never crosses.

use kurbo::Point;

use crate::edges::{Edge, Loop, Vertex};
use crate::fit::Seg;
use crate::guard::crossings;
use crate::outline::{Bound, CLEARANCE_REACH, Chain, Contour, SPAN_LEVELS};
use crate::params::Geometry;
use crate::progress::{Phase, Poller};
use crate::trace::TraceError;

/// Rounds of the guard that go one level coarser; after them a crossing span goes straight to
/// the walked contour.
const FAST_ROUNDS: usize = 1;

/// One loop in the fit: its chain, the part each span belongs to, each span's level and its
/// fitted parts (empty for a tiny loop, whose edges stay their pixel border).
struct Fitted {
    chain: Chain,
    span_part: Vec<usize>,
    /// The spans of the edges this loop owns (the others it takes reversed).
    owned: Vec<bool>,
    levels: Vec<u8>,
    spans: Vec<Vec<Seg>>,
    free: Option<Contour>,
    clamped: Option<Contour>,
}

/// The pixel corners of `vertices` as points.
fn points(vertices: &[Vertex]) -> Vec<Point> {
    vertices
        .iter()
        .map(|&(x, y)| Point::new(f64::from(x), f64::from(y)))
        .collect()
}

/// The vertices of loop `l` in order and where each part starts (`None` for one closed edge).
fn walk(edges: &[Edge], l: &Loop) -> (Vec<Point>, Option<Vec<usize>>) {
    if let [(e, forward)] = l.parts[..]
        && edges[e].closed
    {
        let mut v = points(&edges[e].vertices);
        if !forward {
            v.reverse();
        }
        return (v, None);
    }
    let mut v = Vec::new();
    let mut starts = Vec::with_capacity(l.parts.len());
    for &(e, forward) in &l.parts {
        starts.push(v.len());
        let mut part = points(&edges[e].vertices);
        if !forward {
            part.reverse();
        }
        part.pop();
        v.extend(part);
    }
    (v, Some(starts))
}

/// `segs` run the other way.
pub(crate) fn reversed(segs: &[Seg]) -> Vec<Seg> {
    segs.iter()
        .rev()
        .map(|s| match *s {
            Seg::Line(a, b) => Seg::Line(b, a),
            Seg::Cubic(a, b, c, d) => Seg::Cubic(d, c, b, a),
        })
        .collect()
}

/// The pixel border of `edge` as lines, collinear corners left out.
fn crack_segs(edge: &Edge) -> Vec<Seg> {
    let mut v = points(&edge.vertices);
    if edge.closed {
        v.push(v[0]);
    }
    let mut kept: Vec<Point> = vec![v[0]];
    for k in 1..v.len() - 1 {
        let (a, b, c) = (v[k - 1], v[k], v[k + 1]);
        if (b - a).cross(c - b) != 0.0 {
            kept.push(b);
        }
    }
    kept.push(v[v.len() - 1]);
    kept.windows(2).map(|w| Seg::Line(w[0], w[1])).collect()
}

impl Fitted {
    /// Fits the owned spans at their levels (the contours made when first needed).
    fn refit(&mut self, geometry: &Geometry, tight: &Geometry) {
        if self.chain.is_tiny() {
            return;
        }
        let room = vec![CLEARANCE_REACH * self.unit(); self.chain.edge_count()];
        let free_corners = vec![false; self.chain.span_count()];
        let (chain, owned) = (&self.chain, &self.owned);
        let contour = |g| chain.contour_where(g, &room, Bound::Free, &free_corners, owned);
        let free = self.free.get_or_insert_with(|| contour(geometry));
        if self.clamped.is_none() && self.levels.iter().any(|&l| l > 0) {
            self.clamped = Some(contour(tight));
        }
        let clamped = self.clamped.as_ref().unwrap_or(free);
        self.spans = chain.fitted_where(geometry, &self.levels, free, clamped, &self.owned);
    }

    #[allow(clippy::cast_precision_loss)] // 1 or 2
    fn unit(&self) -> f64 {
        self.chain.unit() as f64
    }
}

/// The fitted segments of every edge, each in its own direction, and how many spans fell back.
pub(crate) fn shared_fit(
    edges: &[Edge],
    loops: &[Loop],
    geometry: &Geometry,
    unit: usize,
    poller: &mut Poller<'_>,
) -> Result<(Vec<Vec<Seg>>, usize), TraceError> {
    let tight = Geometry {
        smoothing_px: geometry.smoothing_px / 2.0,
        tolerance_px: geometry.tolerance_px / 2.0,
        ..*geometry
    };
    let mut fits = Vec::with_capacity(loops.len());
    for (k, l) in loops.iter().enumerate() {
        if k % 64 == 0 {
            poller.check(Phase::Tracing, k, loops.len().max(1))?;
        }
        let mut fit = prepared(edges, l, geometry, unit);
        fit.refit(geometry, &tight);
        fits.push(fit);
    }
    let fallbacks = guard(edges, loops, &mut fits, (geometry, &tight), poller)?;
    let mut out: Vec<Vec<Seg>> = vec![Vec::new(); edges.len()];
    for (l, fit) in loops.iter().zip(&fits) {
        for (p, &(e, forward)) in l.parts.iter().enumerate() {
            if !forward {
                continue;
            }
            out[e] = if fit.chain.is_tiny() {
                crack_segs(&edges[e])
            } else {
                (0..fit.spans.len())
                    .filter(|&s| fit.span_part[s] == p)
                    .flat_map(|s| fit.spans[s].iter().copied())
                    .collect()
            };
        }
    }
    Ok((out, fallbacks))
}

/// Loop `l` ready to fit: its chain with the junctions pinned, the part of every span and
/// which spans it owns; a loop that owns no edge gets an empty (tiny) chain.
fn prepared(edges: &[Edge], l: &Loop, geometry: &Geometry, unit: usize) -> Fitted {
    let empty = |chain| Fitted {
        chain,
        span_part: Vec::new(),
        owned: Vec::new(),
        levels: Vec::new(),
        spans: Vec::new(),
        free: None,
        clamped: None,
    };
    if !l.parts.iter().any(|p| p.1) {
        return empty(Chain::new(Vec::new(), geometry.corner_cos, unit));
    }
    let (vertices, starts) = walk(edges, l);
    let pins = starts.clone().unwrap_or_default();
    let chain = Chain::pinned(vertices, &pins, geometry.corner_cos, unit);
    let span_part: Vec<usize> = match &starts {
        None => vec![0; chain.span_count()],
        Some(starts) => chain
            .span_starts()
            .iter()
            .map(|&v| starts.partition_point(|&s| s <= v) - 1)
            .collect(),
    };
    Fitted {
        owned: span_part.iter().map(|&p| l.parts[p].1).collect(),
        levels: vec![0; chain.span_count()],
        span_part,
        ..empty(chain)
    }
}

/// The guard: every owned span that crosses another part (or itself) goes one level coarser,
/// then straight to the walked contour, until none crosses; returns how many spans fell back.
fn guard(
    edges: &[Edge],
    loops: &[Loop],
    fits: &mut [Fitted],
    (geometry, tight): (&Geometry, &Geometry),
    poller: &mut Poller<'_>,
) -> Result<usize, TraceError> {
    let owned = owned_spans(fits);
    let fixed = pixel_borders(edges, loops, fits);
    for round in 0.. {
        let dirty = coarsened(fits, &owned, &fixed, round);
        if !dirty.iter().any(|&d| d) {
            break;
        }
        poller.check(Phase::Tracing, loops.len(), loops.len().max(1))?;
        for (fit, d) in fits.iter_mut().zip(dirty) {
            if d {
                fit.refit(geometry, tight);
            }
        }
    }
    Ok(owned
        .iter()
        .filter(|&&(k, s)| fits[k].levels[s] > 0)
        .count())
}

/// The owned spans the guard checks, as (loop, span); tiny loops' edges stay pixel borders.
fn owned_spans(fits: &[Fitted]) -> Vec<(usize, usize)> {
    fits.iter()
        .enumerate()
        .filter(|(_, f)| !f.chain.is_tiny())
        .flat_map(|(k, f)| {
            (0..f.spans.len())
                .filter(move |&s| f.owned[s])
                .map(move |s| (k, s))
        })
        .collect()
}

/// The pixel borders of the edges tiny loops own: parts the guard checks against but never
/// changes.
fn pixel_borders(edges: &[Edge], loops: &[Loop], fits: &[Fitted]) -> Vec<Vec<Seg>> {
    loops
        .iter()
        .zip(fits)
        .filter(|(_, f)| f.chain.is_tiny())
        .flat_map(|(l, _)| {
            l.parts
                .iter()
                .filter(|p| p.1)
                .map(|p| crack_segs(&edges[p.0]))
        })
        .collect()
}

/// One round of the guard: every `owned` span that crosses another part (or `fixed`) goes one
/// level coarser (after [`FAST_ROUNDS`] rounds straight to the walked contour); returns which
/// loops changed.
fn coarsened(
    fits: &mut [Fitted],
    owned: &[(usize, usize)],
    fixed: &[Vec<Seg>],
    round: usize,
) -> Vec<bool> {
    let last = SPAN_LEVELS - 1;
    let mut parts: Vec<Vec<Seg>> = owned
        .iter()
        .map(|&(k, s)| fits[k].spans[s].clone())
        .collect();
    parts.extend(fixed.iter().cloned());
    let bad = crossings(&parts);
    let mut dirty = vec![false; fits.len()];
    for (u, &(k, s)) in owned.iter().enumerate() {
        if !bad[u].is_empty() && fits[k].levels[s] < last {
            fits[k].levels[s] = if round >= FAST_ROUNDS {
                last
            } else {
                fits[k].levels[s] + 1
            };
            dirty[k] = true;
        }
    }
    dirty
}

/// The closed outline of loop `l` from the edges' fitted segments.
pub(crate) fn loop_segs(edge_segs: &[Vec<Seg>], l: &Loop) -> Vec<Seg> {
    l.parts
        .iter()
        .flat_map(|&(e, forward)| {
            if forward {
                edge_segs[e].clone()
            } else {
                reversed(&edge_segs[e])
            }
        })
        .collect()
}
