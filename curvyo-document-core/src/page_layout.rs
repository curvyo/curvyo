//! Where the other pages go when one page changes size, is dropped or is deleted (ADR 0051 §2,
//! §3, §3a): pure functions over the page frames in list order.
//!
//! Every test runs on millimetre rectangles within [`EPS`]; the result depends only on the frames
//! and their order, so every device computes the same moves (`specs/0109-page-autolayout`
//! criterion 12). Nothing here reads or writes the document.

use std::collections::VecDeque;

use crate::page_frame::Frame;
use crate::pages::Page;
use crate::{DocumentSize, Point, Vec2};

/// The tolerance of every layout test, in millimetres (ADR 0051 §2).
pub(crate) const EPS: f64 = 1e-9;

/// The pages a layout moves, in list order, each with its translation, and whether the exact
/// round trip of ADR 0051 §2 may be lost.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Layout {
    /// Each moved page and its translation; a page that does not move is not listed.
    pub(crate) moves: Vec<(String, Vec2)>,
    /// Whether a pull stopped short of its edge's change (criterion 10).
    pub(crate) capped: bool,
    /// Whether a page went to the new row below (criterion 11).
    pub(crate) fell_back: bool,
}

/// The distance Add page leaves after a page of `size`: `max(20 mm, 10 % of its larger side)`
/// (`specs/0059-multiple-pages` Frage 2 A).
pub(crate) fn page_gap(size: DocumentSize) -> f64 {
    Rect::of(Point::new(0.0, 0.0), size).gap()
}

/// A page frame as its four edges, in millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Rect {
    pub(crate) x0: f64,
    pub(crate) y0: f64,
    pub(crate) x1: f64,
    pub(crate) y1: f64,
}

impl Rect {
    pub(crate) fn of(origin: Point, size: DocumentSize) -> Self {
        Self {
            x0: origin.x,
            y0: origin.y,
            x1: origin.x + size.width.as_mm(),
            y1: origin.y + size.height.as_mm(),
        }
    }

    pub(crate) fn of_page(page: &Page) -> Self {
        Self::of(page.origin, page.size)
    }

    fn of_frame(frame: Frame) -> Self {
        Self::of(frame.origin, frame.size)
    }

    fn shifted(self, by: Vec2) -> Self {
        Self {
            x0: self.x0 + by.x,
            y0: self.y0 + by.y,
            x1: self.x1 + by.x,
            y1: self.y1 + by.y,
        }
    }

    /// Whether the two rectangles share area; touching does not count.
    pub(crate) fn shares_area(self, other: Self) -> bool {
        self.x0 < other.x1 - EPS
            && other.x0 < self.x1 - EPS
            && self.y0 < other.y1 - EPS
            && other.y0 < self.y1 - EPS
    }

    /// The distance between the two rectangles, 0 when they touch or overlap.
    pub(crate) fn distance(self, other: Self) -> f64 {
        let dx = (self.x0 - other.x1).max(other.x0 - self.x1).max(0.0);
        let dy = (self.y0 - other.y1).max(other.y0 - self.y1).max(0.0);
        dx.hypot(dy)
    }

    /// The gap Add page leaves after this frame.
    pub(crate) fn gap(self) -> f64 {
        20.0_f64.max(0.1 * (self.x1 - self.x0).max(self.y1 - self.y0))
    }
}

/// One edge of a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Right,
    Left,
    Bottom,
    Top,
}

impl Side {
    /// The order chains are shifted and capped in (ADR 0051 §2).
    const ALL: [Self; 4] = [Self::Right, Self::Left, Self::Bottom, Self::Top];

    /// The edge's coordinate.
    fn edge(self, r: Rect) -> f64 {
        match self {
            Self::Right => r.x1,
            Self::Left => r.x0,
            Self::Bottom => r.y1,
            Self::Top => r.y0,
        }
    }

    /// +1 for an edge facing the positive axis, -1 otherwise.
    fn outward(self) -> f64 {
        match self {
            Self::Right | Self::Bottom => 1.0,
            Self::Left | Self::Top => -1.0,
        }
    }

    /// Whether `page` lies wholly beyond this edge of `of`; touching counts.
    fn beyond(self, of: Rect, page: Rect) -> bool {
        match self {
            Self::Right => page.x0 >= of.x1 - EPS,
            Self::Left => page.x1 <= of.x0 + EPS,
            Self::Bottom => page.y0 >= of.y1 - EPS,
            Self::Top => page.y1 <= of.y0 + EPS,
        }
    }

    /// The range of `r` along the edge (across the axis the edge moves on).
    fn across(self, r: Rect) -> (f64, f64) {
        match self {
            Self::Right | Self::Left => (r.y0, r.y1),
            Self::Bottom | Self::Top => (r.x0, r.x1),
        }
    }

    /// Whether `page`'s range along the edge overlaps `range` by more than [`EPS`].
    fn faces(self, page: Rect, (lo, hi): (f64, f64)) -> bool {
        let (a, b) = self.across(page);
        a < hi - EPS && lo < b - EPS
    }

    /// The translation by `amount` along the axis the edge moves on.
    fn vector(self, amount: f64) -> Vec2 {
        match self {
            Self::Right | Self::Left => Vec2::new(amount, 0.0),
            Self::Bottom | Self::Top => Vec2::new(0.0, amount),
        }
    }
}

/// The pages beside `side` of `from` whose range along the edge overlaps `extent`, and the pages
/// behind them, transitively (ADR 0051 §2, direct members and followers), in list order. Pages
/// marked in `skip` are in no chain.
fn chain(rects: &[Rect], skip: &[bool], side: Side, from: Rect, extent: (f64, f64)) -> Vec<usize> {
    let mut member = vec![false; rects.len()];
    let mut work: Vec<usize> = (0..rects.len())
        .filter(|&i| !skip[i] && side.beyond(from, rects[i]) && side.faces(rects[i], extent))
        .collect();
    for &i in &work {
        member[i] = true;
    }
    while let Some(p) = work.pop() {
        for q in 0..rects.len() {
            if !member[q]
                && !skip[q]
                && side.beyond(rects[p], rects[q])
                && side.faces(rects[q], side.across(rects[p]))
            {
                member[q] = true;
                work.push(q);
            }
        }
    }
    (0..rects.len()).filter(|&i| member[i]).collect()
}

/// How far `p` may travel along `side`'s axis in direction `sign` before it shares area with
/// `q`: unbounded when `q` is not in its way.
fn room(p: Rect, q: Rect, side: Side, sign: f64) -> f64 {
    if !side.faces(q, side.across(p)) {
        return f64::INFINITY;
    }
    let (p_lo, p_hi, q_lo, q_hi) = match side {
        Side::Right | Side::Left => (p.x0, p.x1, q.x0, q.x1),
        Side::Bottom | Side::Top => (p.y0, p.y1, q.y0, q.y1),
    };
    if sign < 0.0 {
        if q_hi <= p_lo + EPS {
            (p_lo - q_hi).max(0.0)
        } else if q_lo >= p_hi - EPS {
            f64::INFINITY
        } else {
            0.0
        }
    } else if q_lo >= p_hi - EPS {
        (q_lo - p_hi).max(0.0)
    } else if q_hi <= p_lo + EPS {
        f64::INFINITY
    } else {
        0.0
    }
}

/// The amount a pull of `members` by `delta` along `side`'s axis actually moves: at most
/// `delta`, stopping where a member would first share area with a page outside the chain it did
/// not share area with before (ADR 0051 §2, cap). `gone` takes no part.
fn capped_pull(
    before: &[Rect],
    current: &[Rect],
    members: &[usize],
    gone: Option<usize>,
    side: Side,
    delta: f64,
) -> f64 {
    let mut in_chain = vec![false; current.len()];
    for &m in members {
        in_chain[m] = true;
    }
    let mut allowed = delta.abs();
    for &p in members {
        for q in 0..current.len() {
            if in_chain[q] || Some(q) == gone || before[p].shares_area(before[q]) {
                continue;
            }
            allowed = allowed.min(room(current[p], current[q], side, delta.signum()));
        }
    }
    allowed * delta.signum()
}

/// Where the start of a new row below the lowest of `rects` lies: the leftmost left edge, below
/// every frame by at least that frame's gap (ADR 0051 §2, fallback).
pub(crate) fn new_row_origin(rects: impl Iterator<Item = Rect>) -> Point {
    let (mut left, mut top) = (f64::INFINITY, f64::NEG_INFINITY);
    for r in rects {
        left = left.min(r.x0);
        top = top.max(r.y1 + r.gap());
    }
    if left.is_finite() && top.is_finite() {
        Point::new(left, top)
    } else {
        Point::new(0.0, 0.0)
    }
}

/// Sends one page of every pair that shares area in `current` and did not in `before` to the
/// start of a new row below the lowest page, in list order: the page that is not `keep`, else
/// the later of the two. `gone` takes no part. Whether any page went.
fn settle(before: &[Rect], current: &mut [Rect], keep: Option<usize>, gone: Option<usize>) -> bool {
    let n = current.len();
    let mut sent = vec![false; n];
    for i in 0..n {
        for j in i + 1..n {
            if Some(i) == gone || Some(j) == gone || sent[i] || sent[j] {
                continue;
            }
            if current[i].shares_area(current[j]) && !before[i].shares_area(before[j]) {
                sent[if Some(j) == keep { i } else { j }] = true;
            }
        }
    }
    let mut pending = sent.clone();
    for i in 0..n {
        if !sent[i] {
            continue;
        }
        let origin = new_row_origin(
            (0..n)
                .filter(|&k| !pending[k] && Some(k) != gone)
                .map(|k| current[k]),
        );
        let r = current[i];
        current[i] = r.shifted(Vec2::new(origin.x - r.x0, origin.y - r.y0));
        pending[i] = false;
    }
    sent.contains(&true)
}

/// The moves from `before` to `current`, in list order, leaving out `fixed` and pages that do
/// not move.
fn moves_of(
    pages: &[Page],
    before: &[Rect],
    current: &[Rect],
    fixed: usize,
) -> Vec<(String, Vec2)> {
    (0..pages.len())
        .filter(|&i| i != fixed)
        .filter_map(|i| {
            let by = Vec2::new(current[i].x0 - before[i].x0, current[i].y0 - before[i].y0);
            (by.x.abs() > EPS || by.y.abs() > EPS).then(|| (pages[i].id.clone(), by))
        })
        .collect()
}

/// The reflow of a size change (ADR 0051 §2): page `changed` of `pages` (frames before the step,
/// in list order) gets the frame `new`; the pages beside each moving edge, and the pages behind
/// them, move by that edge's change. The changed page itself is not listed.
pub(crate) fn reflow(pages: &[Page], changed: &str, new: Frame) -> Layout {
    let before: Vec<Rect> = pages.iter().map(Rect::of_page).collect();
    let Some(c) = pages.iter().position(|p| p.id == changed) else {
        return Layout::default();
    };
    let (old, new) = (before[c], Rect::of_frame(new));
    let skip: Vec<bool> = (0..pages.len())
        .map(|i| i == c || before[i].shares_area(old))
        .collect();
    let mut current = before.clone();
    current[c] = new;
    let mut capped = false;
    for side in Side::ALL {
        let delta = side.edge(new) - side.edge(old);
        if delta.abs() <= EPS {
            continue;
        }
        let farther = if delta * side.outward() > 0.0 {
            new
        } else {
            old
        };
        let members = chain(&before, &skip, side, old, side.across(farther));
        if members.is_empty() {
            continue;
        }
        let amount = if delta * side.outward() < 0.0 {
            let amount = capped_pull(&before, &current, &members, None, side, delta);
            capped |= amount.abs() < delta.abs() - EPS;
            amount
        } else {
            delta
        };
        for m in members {
            current[m] = current[m].shifted(side.vector(amount));
        }
    }
    let fell_back = settle(&before, &mut current, Some(c), None);
    Layout {
        moves: moves_of(pages, &before, &current, c),
        capped,
        fell_back,
    }
}

/// The push of a drop (ADR 0051 §3): page `dropped` of `pages` (frames at press, in list order)
/// lands on the frame `new`; every page it comes to share area with moves away in the direction
/// needing the shortest move (right, left, down, up on a tie) to exactly the pusher's gap, in a
/// chain. The dropped page itself is not listed and never moves.
pub(crate) fn push_apart(pages: &[Page], dropped: &str, new: Frame) -> Layout {
    let before: Vec<Rect> = pages.iter().map(Rect::of_page).collect();
    let Some(d) = pages.iter().position(|p| p.id == dropped) else {
        return Layout::default();
    };
    let n = pages.len();
    let mut current = before.clone();
    current[d] = Rect::of_frame(new);
    let mut direction: Vec<Option<usize>> = vec![None; n];
    let mut work = VecDeque::from([d]);
    let mut count = 0;
    'work: while let Some(p) = work.pop_front() {
        let pusher = current[p];
        let gap = pusher.gap();
        for q in 0..n {
            if q == p
                || q == d
                || !pusher.shares_area(current[q])
                || before[p].shares_area(before[q])
            {
                continue;
            }
            let r = current[q];
            let options = [
                Vec2::new(pusher.x1 + gap - r.x0, 0.0),
                Vec2::new(pusher.x0 - gap - r.x1, 0.0),
                Vec2::new(0.0, pusher.y1 + gap - r.y0),
                Vec2::new(0.0, pusher.y0 - gap - r.y1),
            ];
            let mut best = 0;
            for (k, option) in options.iter().enumerate() {
                if option.length() < options[best].length() - EPS {
                    best = k;
                }
            }
            if direction[q].is_some_and(|first| first != best) || count >= n * n {
                break 'work;
            }
            direction[q] = Some(best);
            current[q] = r.shifted(options[best]);
            count += 1;
            work.push_back(q);
        }
    }
    let fell_back = settle(&before, &mut current, Some(d), None);
    Layout {
        moves: moves_of(pages, &before, &current, d),
        capped: false,
        fell_back,
    }
}

/// The close of a delete (ADR 0051 §3a): page `deleted` of `pages` (frames before the delete, in
/// list order) goes; the pages beside its right edge (it is in a row) or else its bottom edge,
/// and the pages behind them, move in by its width (height) plus the nearest one's distance to
/// it. The deleted page is not listed.
pub(crate) fn close_gap(pages: &[Page], deleted: &str) -> Layout {
    let before: Vec<Rect> = pages.iter().map(Rect::of_page).collect();
    let Some(k) = pages.iter().position(|p| p.id == deleted) else {
        return Layout::default();
    };
    let hole = before[k];
    let skip: Vec<bool> = (0..pages.len())
        .map(|i| i == k || before[i].shares_area(hole))
        .collect();
    let in_row = (0..pages.len()).any(|i| {
        !skip[i]
            && (Side::Right.beyond(hole, before[i]) || Side::Left.beyond(hole, before[i]))
            && Side::Right.faces(before[i], Side::Right.across(hole))
    });
    let side = if in_row { Side::Right } else { Side::Bottom };
    let members = chain(&before, &skip, side, hole, side.across(hole));
    if members.is_empty() {
        return Layout::default();
    }
    let near = |r: Rect| match side {
        Side::Right => r.x0,
        _ => r.y0,
    };
    let distance = members
        .iter()
        .map(|&m| near(before[m]) - side.edge(hole))
        .fold(f64::INFINITY, f64::min)
        .max(0.0);
    let length = match side {
        Side::Right => hole.x1 - hole.x0,
        _ => hole.y1 - hole.y0,
    };
    let mut current = before.clone();
    let delta = -(length + distance);
    let amount = capped_pull(&before, &current, &members, Some(k), side, delta);
    for &m in &members {
        current[m] = current[m].shifted(side.vector(amount));
    }
    let fell_back = settle(&before, &mut current, None, Some(k));
    Layout {
        moves: moves_of(pages, &before, &current, k),
        capped: amount.abs() < delta.abs() - EPS,
        fell_back,
    }
}

#[cfg(test)]
#[path = "page_layout_tests.rs"]
mod tests;
