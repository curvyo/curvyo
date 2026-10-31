//! Turns one closed crack chain into a fitted outline in pixels (`specs/0089` adrs.md §5):
//! corners on the raw chain, the spans between them smoothed with the corners pinned, long
//! straight runs as lines, the rest as cubics within the deviation tolerance, every part held
//! within the room its clearance leaves (`clearance.rs`), plus the coarser contours the guard
//! falls back to.

use kurbo::{BezPath, Point, Vec2};

use crate::corners::{Corner, corners, inset, loop_start, midpoint};
use crate::fit::{Piece, Seg, TANGENT_ARC, arc_length, fit_cubics, point_at};
use crate::line::{Pins, Run, norm, segment_distance, simplify, straight_runs, unit};
use crate::params::Geometry;
use crate::smooth::{passes, smooth_closed, smooth_open};

/// The fall-back levels of one span (`guard.rs`): the fit of the freely smoothed contour
/// within the room the smoothed contours leave each other; the fit of the contour smoothed
/// within the room the walked contours leave; the walked contour simplified within half that
/// room; the contour as walked (which never crosses another chain's).
pub(crate) const SPAN_LEVELS: u8 = 4;

/// Every chain measures its room up to this far, traced px per source px: beyond it nothing
/// limits the smoothing or the fit (a corner reaches at most 4.5 px from its vertex).
pub(crate) const CLEARANCE_REACH: f64 = 10.0;

/// Parts of one chain at most this many edges apart along it (per traced pixel per source
/// pixel) do not limit each other's room: the two sides of a sharp tip meet closer than that,
/// and its corner may reach 4.5 px beyond its vertex.
pub(crate) const APART_EDGES: usize = 16;

/// Two outlines keep this much room between them, traced px (the guard's gap).
const GAP: f64 = 0.1;

/// The share of the room between two smoothed contours a fit may use on its side (the guard
/// catches the rare crossing when both sides use more than half).
const SMOOTH_SHARE: f64 = 1.0;

/// The least room a part of an outline is ever given, traced px.
const ROOM_MIN: f64 = 0.05;

/// The share of a held point's room the smoothing may use (the fit keeps the rest), whatever
/// the Tolerance, so a looser tolerance never smooths less.
const SMOOTH_PART: f64 = 0.75;

/// A corner-free chain is smoothed by at most its length divided by this.
const SMALL_LOOP: f64 = 20.0;

/// Chains of at most this many edges stay their pixel border (a pixel, a 2 x 1 pair).
const TINY_EDGES: usize = 8;

/// A straight run inside a span lies within this of its line, source px (or the tolerance,
/// when smaller), so a circle of radius 2000 px or less never gets one.
const LINE_RESIDUAL: f64 = 0.1;

/// A straight run inside a span is at least this long, source px.
const LINE_MIN: f64 = 60.0;

/// One closed crack chain with its structural corners (stage S3 of adrs.md §6).
#[derive(Debug, Clone)]
pub(crate) struct Chain {
    /// The pixel corners, clockwise for an outer outline and counter-clockwise for a hole.
    vertices: Vec<Point>,
    corners: Vec<Corner>,
    /// Where a corner-free chain starts.
    start: usize,
    /// Traced pixels per source pixel.
    unit: usize,
}

/// One span's contour before simplification: its points as walked and smoothed (from corner
/// to corner, or around a closed chain with the first point repeated), the tolerance of every
/// smoothed segment, the room of every walked point and the chain edge every segment stands
/// for (a corner's chord: its vertex's edge).
#[derive(Debug, Clone)]
struct Span {
    raw: Vec<Point>,
    smoothed: Vec<Point>,
    /// Per segment of `smoothed`: the tolerance of the fit there.
    tols: Vec<f64>,
    /// Per point of `raw`: how far a simplified contour may stray from it.
    room: Vec<f64>,
    /// Per segment of `smoothed`: the chain edge it stands for.
    slots: Vec<usize>,
    /// The straight sides' directions into the span at its ends, when the sides are straight.
    tangents: (Option<Vec2>, Option<Vec2>),
}

impl Span {
    /// A span that was not made (not wanted).
    fn empty() -> Self {
        Self {
            raw: Vec::new(),
            smoothed: Vec::new(),
            tols: Vec::new(),
            room: Vec::new(),
            slots: Vec::new(),
            tangents: (None, None),
        }
    }
}

/// The contour before simplification: the spans between corners, or one closed chain.
#[derive(Debug, Clone)]
pub(crate) struct Contour {
    spans: Vec<Span>,
    closed: bool,
}

impl Contour {
    /// The smoothed contour as one closed polyline (its first point not repeated).
    pub(crate) fn polyline(&self) -> Vec<Point> {
        if self.closed {
            let q = &self.spans[0].smoothed;
            return q[..q.len() - 1].to_vec();
        }
        // Every span ends where the next begins.
        self.spans
            .iter()
            .flat_map(|s| s.smoothed[..s.smoothed.len() - 1].iter().copied())
            .collect()
    }

    /// `clearance` (per segment of [`Self::polyline`]: the distance to the nearest other
    /// smoothed contour) as the room per chain edge (`edges` of them; an edge no segment
    /// stands for has all the room).
    pub(crate) fn room_by_edge(&self, clearance: &[f64], edges: usize, reach: f64) -> Vec<f64> {
        let mut room = vec![reach; edges];
        let n = clearance.len().max(1);
        let mut at = 0;
        for span in &self.spans {
            for (j, &slot) in span.slots.iter().enumerate() {
                room[slot] = room[slot].min(clearance[(at + j) % n]);
            }
            at += span.slots.len();
        }
        room
    }
}

/// How a contour's smoothing and segment tolerances are bounded.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Bound<'a> {
    /// Every segment has the tolerance (to measure the smoothed contours' room).
    Free,
    /// Each segment's tolerance is half the room of its edge among the smoothed contours (per
    /// chain edge, [`Contour::room_by_edge`]) less the gap.
    Smoothed(&'a [f64]),
    /// The tolerances come from the walked contour's room less what smoothing used of it: the
    /// fit then never crosses another chain's.
    Walked,
}

/// The room a chain's parts have, from its clearances: per segment of the midpoint polyline
/// (from the midpoint of edge `k` to that of edge `k + 1`) and per midpoint; `inset` is taken
/// off everywhere (half a pixel for a contour on the pixel corners, which lie that far outside
/// the midpoints).
struct Room<'a> {
    clearance: &'a [f64],
    inset: f64,
}

impl Room<'_> {
    /// Half the clearance of segment `k` less the gap: how far the outline may move there.
    fn segment(&self, k: usize) -> f64 {
        let n = self.clearance.len();
        (self.clearance[k % n] / 2.0 - GAP - self.inset).max(ROOM_MIN)
    }

    /// The room at the midpoint of edge `k`: the smaller of its two segments'.
    fn point(&self, k: usize) -> f64 {
        let n = self.clearance.len();
        self.segment((k + n - 1) % n).min(self.segment(k))
    }

    /// The room of a corner at vertex `vertex` with `zone`: the least over the edges of its
    /// zone on both sides.
    fn corner(&self, vertex: usize, zone: f64) -> f64 {
        let n = self.clearance.len();
        // invariant: zones are a few pixels.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let z = (zone.ceil() as usize).max(1).min(n / 2);
        (0..=2 * z)
            .map(|t| self.segment((vertex + 2 * n - 1 - z + t) % n))
            .fold(f64::INFINITY, f64::min)
    }
}

impl Chain {
    /// The chain of `vertices` (pixel corners, unit edges) with the corners for `corner_cos`;
    /// `unit` is the traced pixels per source pixel.
    pub(crate) fn new(vertices: Vec<Point>, corner_cos: f64, unit: usize) -> Self {
        let tiny = vertices.len() <= TINY_EDGES;
        let corners = if tiny {
            Vec::new()
        } else {
            corners(&vertices, corner_cos, unit)
        };
        let start = if corners.is_empty() && !tiny {
            loop_start(&vertices, unit)
        } else {
            0
        };
        Self {
            vertices,
            corners,
            start,
            unit,
        }
    }

    /// A closed chain whose vertices at `pins` (indices, ascending) are fixed nodes: corners
    /// placed exactly at their vertex, which end the spans beside them (the junctions of a
    /// multicolour loop, `specs/0089` adrs.md §10); a structural corner within one scale unit
    /// of a pin gives way to it.
    pub(crate) fn pinned(
        vertices: Vec<Point>,
        pins: &[usize],
        corner_cos: f64,
        unit: usize,
    ) -> Self {
        let mut chain = Self::new(vertices, corner_cos, unit);
        if pins.is_empty() {
            return chain;
        }
        let n = chain.vertices.len();
        let near = |k: usize, p: usize| {
            let d = k.abs_diff(p);
            d.min(n - d) <= unit
        };
        chain
            .corners
            .retain(|c| pins.iter().all(|&p| !near(c.vertex, p)));
        chain.corners.extend(pins.iter().map(|&p| Corner {
            vertex: p,
            at: chain.vertices[p],
            zone: 0.0,
            ahead: None,
            behind: None,
        }));
        chain.corners.sort_by_key(|c| c.vertex);
        chain.start = 0;
        chain
    }

    #[allow(clippy::cast_precision_loss)] // 1 or 2
    fn scale(&self) -> f64 {
        self.unit as f64
    }

    /// Traced pixels per source pixel.
    pub(crate) fn unit(&self) -> usize {
        self.unit
    }

    /// The unit edges of the chain.
    pub(crate) fn edge_count(&self) -> usize {
        self.vertices.len()
    }

    /// The midpoints of the chain's edges: the contour as walked, as one closed polyline.
    pub(crate) fn midpoints(&self) -> Vec<Point> {
        (0..self.vertices.len())
            .map(|k| midpoint(&self.vertices, k))
            .collect()
    }

    /// Whether the chain is its pixel border whatever the settings.
    pub(crate) fn is_tiny(&self) -> bool {
        self.vertices.len() <= TINY_EDGES
    }

    /// The vertex each span starts at (its corner's), in span order; empty for a corner-free
    /// chain.
    pub(crate) fn span_starts(&self) -> Vec<usize> {
        self.corners.iter().map(|c| c.vertex).collect()
    }

    /// The spans the outline is fitted in: one per corner, one for a corner-free chain.
    pub(crate) fn span_count(&self) -> usize {
        self.corners.len().max(1)
    }

    /// The pixel border, collinear corners left out: the last fall-back of the guard (pixel
    /// borders only touch at corners, never cross).
    pub(crate) fn pixel_border(&self) -> Vec<Seg> {
        polygon(&self.vertices)
    }

    /// The pixel border simplified: a vertex is left out when it stays within half a pixel (the
    /// staircase, whatever the Tolerance), and within its room less the half pixel the border
    /// lies outside the walked contour, of the simplified polygon (the Specks band's form for
    /// a shape whose fit shrank below the Specks area although its pixels are well above it).
    pub(crate) fn border_simplified(&self, clearance: &[f64]) -> Vec<Seg> {
        let n = self.vertices.len();
        let room = Room {
            clearance,
            inset: 0.0,
        };
        let scale = self.scale();
        let tolerance = 0.5 * scale;
        let tols: Vec<f64> = (0..n)
            .map(|k| {
                let r = room.segment((k + n - 1) % n).min(room.segment(k)) - 0.5 * scale;
                tolerance.min(r).max(0.0)
            })
            .collect();
        // Closed: simplified as two halves between the first vertex and the one opposite.
        let half = n / 2;
        let mut points: Vec<Point> = Vec::with_capacity(n);
        let mut first = simplify(&self.vertices[..=half], &tols[..=half]);
        first.pop();
        points.extend(first);
        let mut wrapped: Vec<Point> = self.vertices[half..].to_vec();
        wrapped.push(self.vertices[0]);
        let mut wrapped_tols: Vec<f64> = tols[half..].to_vec();
        wrapped_tols.push(tols[0]);
        let mut second = simplify(&wrapped, &wrapped_tols);
        second.pop();
        points.extend(second);
        polygon(&points)
    }

    /// The contour before simplification: the spans between the corners (or the closed chain)
    /// smoothed, each corner at the crossing of its sides held within its room, or (`at_vertex`,
    /// one flag per corner) at its vertex with the chain followed right up to it. `clearance`
    /// is the room of the walked contour (per segment of [`Self::midpoints`], `clearance.rs`);
    /// `bound` says how the smoothing and the tolerances are limited.
    pub(crate) fn contour(
        &self,
        geometry: &Geometry,
        clearance: &[f64],
        bound: Bound<'_>,
        at_vertex: &[bool],
    ) -> Contour {
        self.contour_on(geometry, clearance, bound, at_vertex, false, &[])
    }

    /// [`Self::contour`] with only the spans flagged in `wanted` smoothed (the others empty,
    /// for [`Self::fitted_where`] with the same flags; a corner-free chain is always made).
    pub(crate) fn contour_where(
        &self,
        geometry: &Geometry,
        clearance: &[f64],
        bound: Bound<'_>,
        at_vertex: &[bool],
        wanted: &[bool],
    ) -> Contour {
        self.contour_on(geometry, clearance, bound, at_vertex, false, wanted)
    }

    /// [`Self::contour`], on the pixel corners when `on_corners`, the spans not in `wanted` (when
    /// given) left empty.
    fn contour_on(
        &self,
        geometry: &Geometry,
        clearance: &[f64],
        bound: Bound<'_>,
        at_vertex: &[bool],
        on_corners: bool,
        wanted: &[bool],
    ) -> Contour {
        let room = Room {
            clearance,
            inset: if on_corners { 0.5 * self.scale() } else { 0.0 },
        };
        let tolerance = geometry.tolerance_px * self.scale();
        let limits = Limits {
            room: &room,
            bound,
            tolerance,
            on_corners,
        };
        if self.corners.is_empty() {
            return Contour {
                spans: vec![self.closed_span(geometry, &limits)],
                closed: true,
            };
        }
        let passes = passes(geometry.smoothing_px);
        let count = self.corners.len();
        let spans = (0..count)
            .map(|i| {
                if !wanted.get(i).copied().unwrap_or(true) {
                    return Span::empty();
                }
                let (mut from, mut to) = (self.corners[i], self.corners[(i + 1) % count]);
                for (corner, k) in [(&mut from, i), (&mut to, (i + 1) % count)] {
                    if at_vertex.get(k).copied().unwrap_or(false) {
                        corner.at = inset(&self.vertices, corner.vertex);
                        corner.zone = 0.0;
                        corner.ahead = None;
                        corner.behind = None;
                    } else {
                        corner.at = held(&self.vertices, *corner, &room);
                    }
                }
                self.open_span(from, to, passes, &limits)
            })
            .collect();
        Contour {
            spans,
            closed: false,
        }
    }

    /// The whole corner-free chain as one span, smoothed from its start (the last point
    /// repeats the first).
    fn closed_span(&self, geometry: &Geometry, limits: &Limits<'_>) -> Span {
        let chain = &self.vertices;
        let len = chain.len();
        // A small loop would shrink: at most a twentieth of its length. After Upscale the
        // bilinear copy is smooth already: the same sigma in its own pixels.
        #[allow(clippy::cast_precision_loss)] // chains are far shorter than 2^52
        let passes = passes(geometry.smoothing_px.min(len as f64 / SMALL_LOOP));
        let edges: Vec<usize> = (0..len).map(|step| (self.start + step) % len).collect();
        let mut raw: Vec<Point> = edges.iter().map(|&e| limits.point(chain, e)).collect();
        let mut smoothed = raw.clone();
        smooth_closed(&mut smoothed, passes);
        let point_room: Vec<f64> = edges.iter().map(|&e| limits.room.point(e)).collect();
        let segment_room: Vec<f64> = edges.iter().map(|&e| limits.room.segment(e)).collect();
        let mut room_pts: Vec<f64> = point_room
            .iter()
            .map(|r| limits.tolerance.min(*r))
            .collect();
        let tols = limits.budgets(
            &raw,
            &mut smoothed,
            &point_room,
            &segment_room,
            &edges,
            true,
        );
        smoothed.push(smoothed[0]);
        raw.push(raw[0]);
        room_pts.push(room_pts[0]);
        Span {
            raw,
            smoothed,
            tols,
            room: room_pts,
            slots: edges,
            tangents: (None, None),
        }
    }

    /// The span from corner `from` to corner `to` (their positions and zones set), smoothed
    /// with its ends pinned.
    fn open_span(&self, from: Corner, to: Corner, passes: usize, limits: &Limits<'_>) -> Span {
        let chain = &self.vertices;
        let len = chain.len();
        let edges = match (to.vertex + len - from.vertex) % len {
            0 => len,
            edges => edges,
        };
        let mut raw = vec![from.at];
        let mut kept = Vec::new();
        for step in 0..edges {
            #[allow(clippy::cast_precision_loss)] // chains are far shorter than 2^52
            let (after, before) = (step as f64 + 0.5, (edges - step) as f64 - 0.5);
            if after >= from.zone && before >= to.zone {
                let e = (from.vertex + step) % len;
                raw.push(limits.point(chain, e));
                kept.push(e);
            }
        }
        raw.push(to.at);
        let n = raw.len();
        // The room of every raw point: the corners', then the midpoints'; per segment: a
        // corner's chord takes the smaller room of its ends, a chord between midpoints the
        // room of its edge. Segment `j` starts at point `j`: the start corner (its vertex's
        // edge) or the midpoint of `kept[j - 1]`.
        let mut point_room = Vec::with_capacity(n);
        point_room.push(limits.room.corner(from.vertex, from.zone));
        point_room.extend(kept.iter().map(|&e| limits.room.point(e)));
        point_room.push(limits.room.corner(to.vertex, to.zone));
        let segment_room: Vec<f64> = (0..n - 1)
            .map(|i| {
                if i == 0 || i + 2 == n {
                    point_room[i].min(point_room[i + 1])
                } else {
                    limits.room.segment(kept[i - 1])
                }
            })
            .collect();
        let slots: Vec<usize> = (0..n - 1)
            .map(|j| if j == 0 { from.vertex } else { kept[j - 1] })
            .collect();
        let mut smoothed = raw.clone();
        smooth_open(&mut smoothed, passes);
        let tols = limits.budgets(
            &raw,
            &mut smoothed,
            &point_room,
            &segment_room,
            &slots,
            false,
        );
        Span {
            raw,
            smoothed,
            tols,
            room: point_room
                .iter()
                .map(|r| limits.tolerance.min(*r))
                .collect(),
            slots,
            tangents: (None, None),
        }
    }

    /// The fitted outline per span, each span at its fall-back level of `levels` (see
    /// [`SPAN_LEVELS`]): `free` is the chain's freely smoothed contour with its room set,
    /// `clamped` the one smoothed within the walked contour's room.
    pub(crate) fn fitted(
        &self,
        geometry: &Geometry,
        levels: &[u8],
        free: &Contour,
        clamped: &Contour,
    ) -> Vec<Vec<Seg>> {
        self.fitted_where(geometry, levels, free, clamped, &[])
    }

    /// [`Self::fitted`] for the spans `wanted` names (empty: all); the others stay empty (the
    /// spans a multicolour loop takes from its neighbours).
    pub(crate) fn fitted_where(
        &self,
        geometry: &Geometry,
        levels: &[u8],
        free: &Contour,
        clamped: &Contour,
        wanted: &[bool],
    ) -> Vec<Vec<Seg>> {
        if self.is_tiny() {
            return vec![self.pixel_border()];
        }
        let scale = self.scale();
        let tolerance = geometry.tolerance_px * scale;
        let rules = Rules {
            tolerance,
            run_tolerance: tolerance.min(LINE_RESIDUAL * scale),
            run_length: LINE_MIN * scale,
            arc: TANGENT_ARC * scale,
        };
        let level = |i: usize| levels.get(i).copied().unwrap_or(0);
        if free.closed {
            let (f, c) = (&free.spans[0], &clamped.spans[0]);
            let mut out = Vec::new();
            match level(0) {
                0 => fit_closed(&f.smoothed, &f.tols, &rules, &mut out),
                1 => fit_closed(&c.smoothed, &c.tols, &rules, &mut out),
                2 => {
                    let mut points = simplify(&c.raw, &halved(&c.room));
                    points.pop();
                    out = polygon(&points);
                }
                _ => out = polygon(&c.raw[..c.raw.len() - 1]),
            }
            return vec![out];
        }
        // Two spans as two lines would be no area: they need a curve.
        let chords = free.spans.len() >= 3;
        free.spans
            .iter()
            .zip(&clamped.spans)
            .enumerate()
            .map(|(i, (f, c))| {
                let mut out = Vec::new();
                if !wanted.get(i).copied().unwrap_or(true) {
                    return out;
                }
                match level(i) {
                    0 => fit_span(f, &f.tols, &rules, chords, &mut out),
                    1 => fit_span(c, &c.tols, &rules, chords, &mut out),
                    2 => out = open_polyline(&simplify(&c.raw, &halved(&c.room))),
                    _ => out = open_polyline(&c.raw),
                }
                out
            })
            .collect()
    }
}

/// `values` halved.
fn halved(values: &[f64]) -> Vec<f64> {
    values.iter().map(|v| v / 2.0).collect()
}

/// What bounds a contour's smoothing and tolerances: the walked room, the bound, the
/// tolerance.
struct Limits<'a> {
    room: &'a Room<'a>,
    bound: Bound<'a>,
    tolerance: f64,
    /// The contour runs on the pixel corners instead of the edge midpoints.
    on_corners: bool,
}

impl Limits<'_> {
    /// The walked point of edge `e`: its midpoint, or the pixel corner it starts at.
    fn point(&self, chain: &[Point], e: usize) -> Point {
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
    fn budgets(
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
fn held(chain: &[Point], corner: Corner, room: &Room<'_>) -> Point {
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
fn clamp_moves(raw: &[Point], smoothed: &mut [Point], room: &[f64], held: &[bool]) -> Vec<f64> {
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

/// The fitting constants of one outline, in traced pixels.
struct Rules {
    tolerance: f64,
    run_tolerance: f64,
    run_length: f64,
    arc: f64,
}

/// The smoothed `span` between two corners: one line when it is straight within the tolerance
/// (and `chords` allows it), else its straight runs as lines and cubics between them, the
/// cubics at the corners along the corners' straight sides; `tols` is the tolerance per
/// segment.
fn fit_span(span: &Span, tols: &[f64], rules: &Rules, chords: bool, out: &mut Vec<Seg>) {
    let q = &span.smoothed;
    let (first, last) = (q[0], q[q.len() - 1]);
    let at = |k: usize| -> f64 {
        let before = tols[k.saturating_sub(1)];
        let after = tols[k.min(tols.len() - 1)];
        before.min(after)
    };
    if q.len() == 2
        || (chords
            && first != last
            && q.iter()
                .enumerate()
                .all(|(k, &p)| segment_distance(p, first, last) <= at(k)))
    {
        out.push(Seg::Line(first, last));
        return;
    }
    let runs = if chords {
        let pins = Pins {
            start: true,
            end: true,
        };
        straight_runs(q, tols, rules.run_tolerance, rules.run_length, pins)
    } else {
        // Two corners only (a thin bar's two ends): a straight side is a line off the chord,
        // joined to the corners by short lines.
        let last = q.len() - 1;
        let pins = Pins {
            start: false,
            end: false,
        };
        let length = arc_length(q);
        straight_runs(
            &q[1..last],
            &tols[1..last - 1],
            rules.tolerance,
            0.6 * length,
            pins,
        )
        .into_iter()
        .map(|r| Run {
            first: r.first + 1,
            last: r.last + 1,
            ..r
        })
        .collect()
    };
    assemble(q, tols, &runs, span.tangents, rules, out);
}

/// The closed corner-free chain `q` (last point = first): its straight runs as lines and
/// cubics between them, tangent-continuous at the start.
fn fit_closed(q: &[Point], tols: &[f64], rules: &Rules, out: &mut Vec<Seg>) {
    let last = q.len() - 1;
    let length = arc_length(q);
    let reach = rules.arc.min(length / 4.0);
    let Some(seam) = unit(point_at(q, reach) - point_at(q, length - reach)) else {
        out.extend(polygon(&q[..last]));
        return;
    };
    let pins = Pins {
        start: false,
        end: false,
    };
    let runs: Vec<Run> = straight_runs(
        &q[1..last],
        &tols[1..last - 1],
        rules.run_tolerance,
        rules.run_length,
        pins,
    )
    .into_iter()
    .map(|r| Run {
        first: r.first + 1,
        last: r.last + 1,
        ..r
    })
    .collect();
    if runs.is_empty() {
        // Two halves meeting tangent-continuously at the middle point.
        let mid = last / 2;
        let at = arc_length(&q[..=mid]);
        let Some(centre) = unit(point_at(q, at + reach) - point_at(q, at - reach)) else {
            out.extend(polygon(&q[..last]));
            return;
        };
        let halves = [
            (0, mid, Some(seam), Some(-centre)),
            (mid, last, Some(centre), Some(-seam)),
        ];
        for (i, j, t0, t1) in halves {
            let piece = Piece {
                q: &q[i..=j],
                tols: &tols[i..j],
                start: q[i],
                end: q[j],
                start_tangent: t0,
                end_tangent: t1,
            };
            fit_cubics(piece, rules.tolerance, rules.arc, out);
        }
        return;
    }
    assemble(q, tols, &runs, (Some(seam), Some(-seam)), rules, out);
}

/// Lines for `runs` of `q` and cubics for the gaps, tangent-continuous at every run end;
/// `(start, end)` are the tangents at `q`'s ends (`None`: free, at a corner).
fn assemble(
    q: &[Point],
    tols: &[f64],
    runs: &[Run],
    (start, end): (Option<Vec2>, Option<Vec2>),
    rules: &Rules,
    out: &mut Vec<Seg>,
) {
    let last = q.len() - 1;
    let (mut cursor, mut node, mut tangent) = (0, q[0], start);
    for run in runs {
        let direction = unit(run.to - run.from);
        if run.first > cursor {
            let piece = Piece {
                q: &q[cursor..=run.first],
                tols: &tols[cursor..run.first],
                start: node,
                end: run.from,
                start_tangent: tangent,
                end_tangent: direction.map(|d| -d),
            };
            fit_cubics(piece, rules.tolerance, rules.arc, out);
        }
        out.push(Seg::Line(run.from, run.to));
        (cursor, node, tangent) = (run.last, run.to, direction);
    }
    if cursor < last {
        let piece = Piece {
            q: &q[cursor..],
            tols: &tols[cursor..last],
            start: node,
            end: q[last],
            start_tangent: tangent,
            end_tangent: end,
        };
        fit_cubics(piece, rules.tolerance, rules.arc, out);
    }
}

/// The closed polygon through `points`, points on a straight line between their neighbours
/// left out.
fn polygon(points: &[Point]) -> Vec<Seg> {
    let n = points.len();
    let kept: Vec<Point> = (0..n)
        .filter(|&i| {
            let (a, b, c) = (points[(i + n - 1) % n], points[i], points[(i + 1) % n]);
            (b - a).cross(c - b) != 0.0 || (b - a).dot(c - b) < 0.0
        })
        .map(|i| points[i])
        .collect();
    let kept = if kept.len() >= 3 {
        kept
    } else {
        points.to_vec()
    };
    let m = kept.len();
    (0..m)
        .map(|i| Seg::Line(kept[i], kept[(i + 1) % m]))
        .collect()
}

/// The lines through `points`, in order (an open polyline).
fn open_polyline(points: &[Point]) -> Vec<Seg> {
    points.windows(2).map(|w| Seg::Line(w[0], w[1])).collect()
}

/// `segments` (a closed outline) as a closed path.
pub(crate) fn to_path(segments: &[Seg]) -> BezPath {
    let mut path = BezPath::new();
    let Some(first) = segments.first() else {
        return path;
    };
    path.move_to(match *first {
        Seg::Line(p, _) | Seg::Cubic(p, _, _, _) => p,
    });
    for seg in segments {
        match *seg {
            // A line of no length is no node.
            Seg::Line(a, b) if a == b => {}
            Seg::Line(_, p) => path.line_to(p),
            Seg::Cubic(_, c1, c2, p) => path.curve_to(c1, c2, p),
        }
    }
    path.close_path();
    path
}

/// The anchors of a closed outline: one per segment, the closing one not counted twice.
pub(crate) fn node_count(path: &BezPath) -> usize {
    use kurbo::PathEl;
    let mut start = None;
    let mut last = None;
    let mut segments = 0;
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(p) => {
                start = Some(p);
                last = Some(p);
            }
            PathEl::LineTo(p) | PathEl::QuadTo(_, p) | PathEl::CurveTo(_, _, p) => {
                segments += 1;
                last = Some(p);
            }
            PathEl::ClosePath => {}
        }
    }
    // The last segment ends on the start: its end is the first anchor again.
    if segments > 0 && start == last {
        segments
    } else {
        segments + 1
    }
}

#[cfg(test)]
// Test shapes are small, named by their formulas (x, y, w, h).
#[allow(clippy::cast_precision_loss, clippy::many_single_char_names)]
mod tests {
    use super::*;
    use crate::params::geometry;

    fn chain(rows: &[&str]) -> Chain {
        let w = rows[0].len();
        let bits: Vec<bool> = rows
            .iter()
            .flat_map(|r| r.chars().map(|c| c == '#'))
            .collect();
        let vertices = crate::contour::outer_loop(&bits, w, rows.len(), true)
            .into_iter()
            .map(|(x, y)| Point::new(f64::from(x), f64::from(y)))
            .collect();
        Chain::new(vertices, crate::params::corner_cos(70), 1)
    }

    /// The chain fitted with `clearance` on every walked segment and `smooth_clearance` on
    /// every smoothed one, every span at `level`.
    fn fit_with(
        c: &Chain,
        g: &Geometry,
        clearance: f64,
        smooth_clearance: f64,
        level: u8,
    ) -> Vec<Seg> {
        let room = vec![clearance; c.vertices.len()];
        let corners = vec![false; c.span_count()];
        let free = c.contour(g, &room, Bound::Free, &corners);
        let n = free.polyline().len();
        let by_edge = free.room_by_edge(
            &vec![smooth_clearance; n],
            c.vertices.len(),
            CLEARANCE_REACH,
        );
        let free = c.contour(g, &room, Bound::Smoothed(&by_edge), &corners);
        let clamped = c.contour(g, &room, Bound::Walked, &corners);
        let levels = vec![level; c.span_count()];
        c.fitted(g, &levels, &free, &clamped).concat()
    }

    fn fit_alone(c: &Chain, g: &Geometry) -> Vec<Seg> {
        fit_with(c, g, CLEARANCE_REACH, CLEARANCE_REACH, 0)
    }

    #[test]
    fn a_single_pixel_is_its_square() {
        let path = to_path(&fit_alone(&chain(&["#"]), &geometry(80, 0.5, 70)));
        assert_eq!(node_count(&path), 4);
        assert!((kurbo::Shape::area(&path).abs() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn a_square_is_four_lines_through_its_corners() {
        let rows = vec!["##########"; 10];
        let c = chain(&rows);
        let segs = fit_alone(&c, &geometry(80, 0.5, 70));
        assert_eq!(segs.len(), 4, "{segs:?}");
        assert!(segs.iter().all(|s| matches!(s, Seg::Line(..))));
        let area = kurbo::Shape::area(&to_path(&segs)).abs();
        assert!((area - 100.0).abs() < 1e-9, "{area}");
    }

    #[test]
    fn a_thin_line_traces() {
        let path = to_path(&fit_alone(&chain(&["##########"]), &geometry(80, 0.5, 70)));
        let area = kurbo::Shape::area(&path).abs();
        assert!(area > 5.0 && area < 15.0, "{area}");
    }

    /// The largest distance of the segments' samples from the walked contour of `c`.
    fn deviation(c: &Chain, segs: &[Seg]) -> f64 {
        let raw = c.midpoints();
        segs.iter()
            .flat_map(|s| match *s {
                Seg::Line(a, b) => vec![a, b],
                Seg::Cubic(a, b, cc, d) => (0..=10)
                    .map(|t| crate::fit::eval(&[a, b, cc, d], f64::from(t) / 10.0))
                    .collect(),
            })
            .map(|p| {
                (0..raw.len())
                    .map(|k| segment_distance(p, raw[k], raw[(k + 1) % raw.len()]))
                    .fold(f64::INFINITY, f64::min)
            })
            .fold(0.0, f64::max)
    }

    #[test]
    fn a_cramped_chain_stays_within_its_room() {
        // A 20 x 10 blob with a toothed top edge, tolerance 1.5 px: with 1 px of room on every
        // walked segment the clamped fit (level 1) stays within half of it (0.5 px, the
        // corner chords included) of the walked contour, the free fit wanders further.
        let rows: Vec<String> = (0..10)
            .map(|y| {
                (0..20)
                    .map(|x| if y > 0 || x % 3 == 0 { '#' } else { '.' })
                    .collect()
            })
            .collect();
        let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
        let c = chain(&refs);
        let g = geometry(80, 1.5, 70);
        let clamped = deviation(&c, &fit_with(&c, &g, 1.0, CLEARANCE_REACH, 1));
        assert!(clamped <= 0.5 + 1e-9, "{clamped}");
        let free = deviation(&c, &fit_with(&c, &g, CLEARANCE_REACH, CLEARANCE_REACH, 0));
        assert!(free > clamped, "{free} > {clamped}");
    }
}
