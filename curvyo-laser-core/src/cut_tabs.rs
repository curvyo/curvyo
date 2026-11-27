//! Where the tabs of one outline lie (`specs/0139` criteria 2, 3, 5, 6, adrs L8, L12): the
//! automatic placement, the spans of own tabs and the edit rules of a tab handle.

use crate::cut_item_records::{OwnTab, PathFraction};
use crate::job_input::PointUm;
use crate::operation_cut_options::{CutLength, TabCount};

/// The shortest burnt piece a tab may leave, 1 mm in µm (criterion 5, Proposal).
pub const MIN_PIECE_UM: f64 = 1_000.0;

/// The chip text of a width that leaves a piece under 1 mm (criterion 6, C6).
pub const NO_ROOM_TEXT: &str = "No room for this width.";

/// A flattened outline in µm with the arc length at each vertex.
#[derive(Debug, Clone, PartialEq)]
pub struct Measured {
    points: Vec<PointUm>,
    /// The arc length at each vertex, and for a closed outline at its return to the first.
    at: Vec<f64>,
    closed: bool,
}

/// One tab as a stretch of an outline: its centre and its width, in µm of arc length from the
/// outline's first point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TabSpan {
    /// The centre.
    pub centre: f64,
    /// The width.
    pub width: f64,
}

impl TabSpan {
    /// Where it starts.
    #[must_use]
    pub fn start(self) -> f64 {
        self.centre - self.width / 2.0
    }

    /// Where it ends.
    #[must_use]
    pub fn end(self) -> f64 {
        self.centre + self.width / 2.0
    }
}

fn as_f64(um: i64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    // exact: page coordinates in µm are far below 2^53
    let value = um as f64;
    value
}

fn segment(a: PointUm, b: PointUm) -> f64 {
    let (dx, dy) = (as_f64(b.x - a.x), as_f64(b.y - a.y));
    // `sqrt`, not `hypot`: correctly rounded on every target (ADR 0064).
    (dx * dx + dy * dy).sqrt()
}

impl Measured {
    /// The outline `points` (a closed one does not repeat its first point).
    #[must_use]
    pub fn new(points: Vec<PointUm>, closed: bool) -> Self {
        let mut at = Vec::with_capacity(points.len() + 1);
        let mut sum = 0.0;
        at.push(0.0);
        for pair in points.windows(2) {
            sum += segment(pair[0], pair[1]);
            at.push(sum);
        }
        if closed && let (Some(&last), Some(&first)) = (points.last(), points.first()) {
            sum += segment(last, first);
            at.push(sum);
        }
        Self { points, at, closed }
    }

    /// The points.
    #[must_use]
    pub fn points(&self) -> &[PointUm] {
        &self.points
    }

    /// Whether it is closed.
    #[must_use]
    pub const fn closed(&self) -> bool {
        self.closed
    }

    /// Its length in µm.
    #[must_use]
    pub fn length(&self) -> f64 {
        self.at.last().copied().unwrap_or(0.0)
    }

    /// The arc length at each vertex (the nodes a handle snaps to).
    #[must_use]
    pub fn vertex_arcs(&self) -> &[f64] {
        &self.at[..self.points.len()]
    }

    /// `arc` on the outline: wrapped on a closed one, clamped on an open one.
    #[must_use]
    pub fn wrap(&self, arc: f64) -> f64 {
        let length = self.length();
        if length <= 0.0 {
            return 0.0;
        }
        if self.closed {
            arc.rem_euclid(length)
        } else {
            arc.clamp(0.0, length)
        }
    }

    /// The vertex `i` with the closing return to the first point as index `len`.
    fn vertex(&self, i: usize) -> PointUm {
        self.points[i % self.points.len()]
    }

    /// The segment that holds `arc` (wrapped) and the arc at its start.
    fn segment_at(&self, arc: f64) -> usize {
        let arc = self.wrap(arc);
        let last = self.at.len().saturating_sub(2);
        match self.at.binary_search_by(|v| v.total_cmp(&arc)) {
            Ok(i) => i.min(last),
            Err(i) => i.saturating_sub(1).min(last),
        }
    }

    /// The point at `arc` (wrapped) in µm, unrounded.
    #[must_use]
    pub fn point_at(&self, arc: f64) -> (f64, f64) {
        if self.points.len() < 2 {
            return self
                .points
                .first()
                .map_or((0.0, 0.0), |p| (as_f64(p.x), as_f64(p.y)));
        }
        let arc = self.wrap(arc);
        let i = self.segment_at(arc);
        let (a, b) = (self.vertex(i), self.vertex(i + 1));
        let span = self.at[i + 1] - self.at[i];
        let t = if span > 0.0 {
            ((arc - self.at[i]) / span).clamp(0.0, 1.0)
        } else {
            0.0
        };
        (
            as_f64(a.x) + (as_f64(b.x) - as_f64(a.x)) * t,
            as_f64(a.y) + (as_f64(b.y) - as_f64(a.y)) * t,
        )
    }

    /// The point at `arc` rounded to µm.
    #[must_use]
    pub fn point_um(&self, arc: f64) -> PointUm {
        let (x, y) = self.point_at(arc);
        PointUm::new(round(x), round(y))
    }

    /// The unit direction of the outline at `arc` (the segment leaving it).
    #[must_use]
    pub fn tangent_at(&self, arc: f64) -> (f64, f64) {
        if self.points.len() < 2 {
            return (1.0, 0.0);
        }
        let i = self.segment_at(arc);
        let (a, b) = (self.vertex(i), self.vertex(i + 1));
        let (dx, dy) = (as_f64(b.x - a.x), as_f64(b.y - a.y));
        let norm = (dx * dx + dy * dy).sqrt();
        if norm > 0.0 {
            (dx / norm, dy / norm)
        } else {
            (1.0, 0.0)
        }
    }

    /// The unit direction of the outline arriving at `arc` (the segment that ends there; at a
    /// closed outline's first point, its closing segment), for a lead-out (criterion 14).
    #[must_use]
    pub fn tangent_before(&self, arc: f64) -> (f64, f64) {
        if self.points.len() < 2 {
            return (1.0, 0.0);
        }
        let mut arc = self.wrap(arc);
        if self.closed && arc <= 0.0 {
            arc = self.length();
        }
        let last = self.at.len().saturating_sub(2);
        let i = self
            .at
            .partition_point(|&v| v < arc)
            .saturating_sub(1)
            .min(last);
        let (a, b) = (self.vertex(i), self.vertex(i + 1));
        let (dx, dy) = (as_f64(b.x - a.x), as_f64(b.y - a.y));
        let norm = (dx * dx + dy * dy).sqrt();
        if norm > 0.0 {
            (dx / norm, dy / norm)
        } else {
            (1.0, 0.0)
        }
    }

    /// The arc of the point of the outline nearest to `p` (µm) and its squared distance.
    #[must_use]
    #[allow(clippy::many_single_char_names)] // the usual names of a projection onto a segment
    pub fn nearest(&self, p: (f64, f64)) -> (f64, f64) {
        let mut best = (0.0, f64::INFINITY);
        let segments = self.at.len().saturating_sub(1);
        for i in 0..segments {
            let (a, b) = (self.vertex(i), self.vertex(i + 1));
            let (ax, ay) = (as_f64(a.x), as_f64(a.y));
            let (dx, dy) = (as_f64(b.x) - ax, as_f64(b.y) - ay);
            let len2 = dx * dx + dy * dy;
            let t = if len2 > 0.0 {
                (((p.0 - ax) * dx + (p.1 - ay) * dy) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let (x, y) = (ax + dx * t, ay + dy * t);
            let d2 = (x - p.0) * (x - p.0) + (y - p.1) * (y - p.1);
            if d2 < best.1 {
                best = (self.at[i] + (self.at[i + 1] - self.at[i]) * t, d2);
            }
        }
        (self.wrap(best.0), best.1)
    }

    /// The arc of `fraction`.
    #[must_use]
    pub fn arc_of(&self, fraction: PathFraction) -> f64 {
        fraction.get() * self.length()
    }

    /// The fraction of `arc`.
    #[must_use]
    pub fn fraction_of(&self, arc: f64) -> PathFraction {
        let length = self.length();
        if length <= 0.0 {
            return PathFraction::of(0.0);
        }
        PathFraction::of(self.wrap(arc) / length)
    }
}

fn round(value: f64) -> i64 {
    #[allow(clippy::cast_possible_truncation)]
    // invariant: `as` saturates; page coordinates are far inside the range
    let um = value.round() as i64;
    um
}

/// How many automatic tabs a path of `length` µm gets (criterion 2, adrs spec issue 5):
/// min(n, floor(L / 4W), floor(L / (W + 1 mm))); an open path at most floor(L / (W + 2 mm)), so
/// its two end pieces keep 1 mm too (lead decision 2026-11-27, Proposal).
#[must_use]
pub fn automatic_count(length: f64, count: TabCount, width: CutLength, closed: bool) -> usize {
    let w = as_f64(width.um());
    if count.get() == 0 || w <= 0.0 || length <= 0.0 {
        return 0;
    }
    let by_width = (length / (4.0 * w)).floor();
    let piece = if closed {
        MIN_PIECE_UM
    } else {
        2.0 * MIN_PIECE_UM
    };
    let by_piece = (length / (w + piece)).floor();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    // invariant: both quotients are finite and at least 0
    let limit = by_width.min(by_piece) as usize;
    usize::from(count.get()).min(limit)
}

/// The automatic tabs of a path of `length` µm: k tabs centred at (i + 0.5) / k of the length.
#[must_use]
pub fn automatic(length: f64, count: TabCount, width: CutLength, closed: bool) -> Vec<TabSpan> {
    let k = automatic_count(length, count, width, closed);
    let w = as_f64(width.um());
    (0..k)
        .map(|i| {
            #[allow(clippy::cast_precision_loss)] // at most 20 tabs
            let centre = (i as f64 + 0.5) / k as f64 * length;
            TabSpan { centre, width: w }
        })
        .collect()
}

/// The spans of own `tabs` on `outline`, each at its own width or `width`.
#[must_use]
pub fn own_spans(outline: &Measured, tabs: &[OwnTab], width: CutLength) -> Vec<TabSpan> {
    tabs.iter()
        .map(|tab| TabSpan {
            centre: outline.arc_of(tab.at),
            width: as_f64(tab.width.unwrap_or(width).um()),
        })
        .collect()
}

/// The burnt pieces `spans` (in centre order) leave on `outline`, as arc stretches; a closed
/// outline's first piece starts at the end of its first tab and the last wraps to the start of
/// it. Every gap must be positive for `overlap` to stay false.
fn gaps(outline: &Measured, spans: &[TabSpan]) -> Vec<f64> {
    let length = outline.length();
    let mut out = Vec::with_capacity(spans.len() + 1);
    if outline.closed() {
        for (i, span) in spans.iter().enumerate() {
            let next = spans[(i + 1) % spans.len()];
            let next_start = if i + 1 == spans.len() {
                next.start() + length
            } else {
                next.start()
            };
            out.push(next_start - span.end());
        }
    } else {
        let mut from = 0.0;
        for span in spans {
            out.push(span.start() - from);
            from = span.end();
        }
        out.push(length - from);
    }
    out
}

/// Whether two of `spans` (in centre order) overlap on `outline` (damage, criterion 11).
#[must_use]
pub fn overlap(outline: &Measured, spans: &[TabSpan]) -> bool {
    if spans.is_empty() {
        return false;
    }
    let found = gaps(outline, spans);
    if outline.closed() {
        spans.len() > 1 && found.iter().any(|&gap| gap < 0.0)
            || spans.iter().map(|s| s.width).sum::<f64>() > outline.length()
    } else {
        found[1..found.len() - 1].iter().any(|&gap| gap < 0.0)
    }
}

/// Whether `spans` (in centre order) leave every burnt piece at least 1 mm (criterion 5).
#[must_use]
pub fn has_room(outline: &Measured, spans: &[TabSpan]) -> bool {
    gaps(outline, spans)
        .iter()
        .all(|&gap| gap >= MIN_PIECE_UM - 1e-6)
}

/// Where a dragged handle lands (criterion 5).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slide {
    /// The new centre in µm of arc.
    pub centre: f64,
    /// Whether it snapped to a node.
    pub snapped: bool,
    /// Whether the 1 mm rule stopped it.
    pub limited: bool,
}

/// Slides tab `index` of `spans` (in centre order) towards arc `target`, snapping to a node
/// within `snap` µm of arc (0 for none) and stopping where a neighbour's piece would fall
/// under 1 mm.
#[must_use]
pub fn slide(outline: &Measured, spans: &[TabSpan], index: usize, target: f64, snap: f64) -> Slide {
    let length = outline.length();
    let span = spans[index];
    let mut target = outline.wrap(target);
    let mut snapped = false;
    if snap > 0.0 {
        let near = outline
            .vertex_arcs()
            .iter()
            .copied()
            .chain(outline.closed().then_some(length))
            .map(|node| (distance(outline, node, target), node))
            .min_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((d, node)) = near
            && d <= snap
        {
            target = outline.wrap(node);
            snapped = true;
        }
    }
    let half = span.width / 2.0;
    let (low, high, delta) = if outline.closed() {
        let mut delta = target - span.centre;
        if delta > length / 2.0 {
            delta -= length;
        } else if delta < -length / 2.0 {
            delta += length;
        }
        if spans.len() == 1 {
            (f64::NEG_INFINITY, f64::INFINITY, delta)
        } else {
            let found = gaps(outline, spans);
            let back = found[(index + spans.len() - 1) % spans.len()];
            let forward = found[index];
            (-(back - MIN_PIECE_UM), forward - MIN_PIECE_UM, delta)
        }
    } else {
        let prev = if index == 0 {
            0.0
        } else {
            spans[index - 1].end()
        };
        let next = spans.get(index + 1).map_or(length, |s| s.start());
        let low = prev + MIN_PIECE_UM + half - span.centre;
        let high = next - MIN_PIECE_UM - half - span.centre;
        (low, high, target - span.centre)
    };
    let clamped = delta.clamp(low.min(0.0), high.max(0.0));
    Slide {
        centre: outline.wrap(span.centre + clamped),
        snapped: snapped && (clamped - delta).abs() < 1e-9,
        limited: (clamped - delta).abs() >= 1e-9,
    }
}

fn distance(outline: &Measured, a: f64, b: f64) -> f64 {
    let d = (a - b).abs();
    if outline.closed() {
        d.min(outline.length() - d)
    } else {
        d
    }
}

/// `spans` with a new tab of `width` centred at `arc`, in centre order, and its index; `None`
/// when the 1 mm rule leaves no room there (criterion 5: nothing changes).
#[must_use]
pub fn add(
    outline: &Measured,
    spans: &[TabSpan],
    arc: f64,
    width: f64,
) -> Option<(Vec<TabSpan>, usize)> {
    let centre = outline.wrap(arc);
    let index = spans.partition_point(|s| s.centre < centre);
    let mut next = spans.to_vec();
    next.insert(index, TabSpan { centre, width });
    has_room(outline, &next).then_some((next, index))
}

/// `spans` with tab `index` at `width`.
///
/// # Errors
/// [`NO_ROOM_TEXT`] when a piece would fall under 1 mm (criterion 6).
pub fn with_width(
    outline: &Measured,
    spans: &[TabSpan],
    index: usize,
    width: f64,
) -> Result<Vec<TabSpan>, &'static str> {
    let mut next = spans.to_vec();
    next[index].width = width;
    if has_room(outline, &next) {
        Ok(next)
    } else {
        Err(NO_ROOM_TEXT)
    }
}

/// The muted line under Count (criterion 2, C1): `None` while every path gets Count tabs (or
/// Count is 0); else "Placed: 3 per path", "Placed: 2 to 4 per path" (the range over every path,
/// when paths differ) or "Placed: none" for the per-path numbers `placed`.
#[must_use]
pub fn placed_text(count: TabCount, placed: &[usize]) -> Option<String> {
    let n = usize::from(count.get());
    if n == 0 || placed.iter().all(|&k| k >= n) {
        return None;
    }
    let low = placed.iter().copied().min().unwrap_or(0);
    let high = placed.iter().copied().max().unwrap_or(0);
    Some(if high == 0 {
        "Placed: none".to_owned()
    } else if low == high {
        format!("Placed: {low} per path")
    } else {
        format!("Placed: {low} to {high} per path")
    })
}

#[cfg(test)]
#[path = "cut_tabs_tests.rs"]
mod tests;
