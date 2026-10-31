//! Groups shapes into text lines and gives each line its baseline frame.
//!
//! The direction is the principal axis of the shape centres (a 2 x 2 covariance, its eigenvector
//! by `sqrt`, a unit vector, no angle; `adrs.md` decision 3 stage 2). Shapes far taller than the
//! usual letter do not steer it. In that frame shapes are grouped into lines by vertical overlap,
//! the tallest first so a dot joins the line of its letter; a wide gap splits a line.

use kurbo::{Point, Vec2};

use super::blobs::Blob;
use super::sample::f;

/// A line's frame: line space has x along `u` and y along `n` (down on screen for upright text).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Frame {
    /// The document point of the line space origin.
    pub origin: Point,
    /// The unit vector along the line.
    pub u: Vec2,
}

impl Frame {
    /// The unit vector across the line, a quarter turn clockwise on screen from `u`.
    pub(crate) const fn n(self) -> Vec2 {
        Vec2::new(-self.u.y, self.u.x)
    }

    /// A document point in line space.
    pub(crate) fn to_line(self, p: Point) -> Point {
        let d = p - self.origin;
        let n = self.n();
        Point::new(d.x * self.u.x + d.y * self.u.y, d.x * n.x + d.y * n.y)
    }

    /// A line space point in the document.
    pub(crate) fn to_document(self, p: Point) -> Point {
        let n = self.n();
        self.origin + self.u * p.x + n * p.y
    }
}

/// One text line: its blobs in reading order and its frame.
#[derive(Debug, Clone)]
pub(crate) struct Line {
    /// Indices into the blobs, sorted along the line.
    pub blobs: Vec<usize>,
    /// The frame the blobs were grouped in.
    pub frame: Frame,
}

/// A wider gap than this many line heights splits a line (Proposal).
const SPLIT_GAP: f64 = 3.0;
/// A shape taller than this many median heights is not a letter of the band it overlaps.
const TALL: f64 = 2.5;

/// The median of `values` (sorted in place; total order), 0 for none.
pub(crate) fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        values[mid]
    } else {
        f64::midpoint(values[mid - 1], values[mid])
    }
}

/// The principal axis of `points` as a unit vector pointing right (x >= 0), or `(1, 0)` when
/// there are fewer than two points or no direction stands out.
pub(crate) fn principal_axis(points: &[Point]) -> Vec2 {
    if points.len() < 2 {
        return Vec2::new(1.0, 0.0);
    }
    let count = f(points.len());
    let (sx, sy) = points
        .iter()
        .fold((0.0, 0.0), |(x, y), p| (x + p.x, y + p.y));
    let (mx, my) = (sx / count, sy / count);
    let (mut xx, mut xy, mut yy) = (0.0, 0.0, 0.0);
    for p in points {
        let (dx, dy) = (p.x - mx, p.y - my);
        xx += dx * dx;
        xy += dx * dy;
        yy += dy * dy;
    }
    // The larger eigenvalue of [[xx, xy], [xy, yy]] and its eigenvector.
    let half = (xx - yy) / 2.0;
    let root = (half * half + xy * xy).sqrt();
    let lambda = f64::midpoint(xx, yy) + root;
    let v = if xy.abs() > 1e-12 * (xx + yy).max(1e-300) {
        Vec2::new(lambda - yy, xy)
    } else if xx >= yy {
        Vec2::new(1.0, 0.0)
    } else {
        Vec2::new(0.0, 1.0)
    };
    let length = (v.x * v.x + v.y * v.y).sqrt();
    if length.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) || !length.is_finite() {
        return Vec2::new(1.0, 0.0);
    }
    let v = v / length;
    // Reading direction left to right: the axis points right; a vertical axis points down.
    if v.x < 0.0 || (v.x == 0.0 && v.y < 0.0) {
        -v
    } else {
        v
    }
}

/// The direction of the line through `points` (x along, y across) as a unit vector, by the median
/// of the slopes between points at least `min_dx` apart along it (Theil-Sen): a point off the
/// line does not steer it as it does a least-squares axis. `None` without such a pair.
pub(crate) fn median_slope(points: &[Point], min_dx: f64) -> Option<Vec2> {
    let mut slopes = Vec::new();
    for (k, a) in points.iter().enumerate() {
        for b in &points[k + 1..] {
            let dx = b.x - a.x;
            if dx.abs() >= min_dx {
                slopes.push((b.y - a.y) / dx);
            }
        }
    }
    if slopes.is_empty() {
        return None;
    }
    let slope = median(&mut slopes);
    let length = (1.0 + slope * slope).sqrt();
    Some(Vec2::new(1.0 / length, slope / length))
}

/// The extent of a blob across the line, in line space y: (top, bottom).
fn across(blob: &Blob, frame: &Frame) -> (f64, f64) {
    corners_range(blob, frame, false)
}

/// The extent of a blob along the line, in line space x: (left, right).
pub(crate) fn along(blob: &Blob, frame: &Frame) -> (f64, f64) {
    corners_range(blob, frame, true)
}

fn corners_range(blob: &Blob, frame: &Frame, x: bool) -> (f64, f64) {
    let b = blob.bbox;
    let corners = [
        Point::new(b.x0, b.y0),
        Point::new(b.x1, b.y0),
        Point::new(b.x0, b.y1),
        Point::new(b.x1, b.y1),
    ];
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for c in corners {
        let p = frame.to_line(c);
        let v = if x { p.x } else { p.y };
        lo = lo.min(v);
        hi = hi.max(v);
    }
    (lo, hi)
}

struct Band {
    top: f64,
    bottom: f64,
    left: f64,
    right: f64,
    blobs: Vec<usize>,
}

impl Band {
    fn height(&self) -> f64 {
        self.bottom - self.top
    }
}

/// The share of the lower height two extents must overlap by to be one line (a comma reaches
/// below the baseline, a `g` too).
const OVERLAP: f64 = 0.35;
/// A shape below this share of the typical height is a mark (dot, accent, comma, quote): it
/// joins the nearest line it sits next to.
const MARK: f64 = 0.45;

/// The band a shape of extent `(top, bottom)` joins: the one it overlaps most, by at least
/// [`OVERLAP`] of the lower of the two heights; with `along`, only a band whose letters reach it.
fn overlapping(bands: &[Band], top: f64, bottom: f64, along: Option<(f64, f64)>) -> Option<usize> {
    let height = bottom - top;
    let mut best: Option<(usize, f64)> = None;
    for (index, band) in bands.iter().enumerate() {
        if height > TALL * band.height() {
            continue;
        }
        if let Some((left, right)) = along {
            let reach = 0.3 * band.height();
            if right < band.left - reach || left > band.right + reach {
                continue;
            }
        }
        let overlap = bottom.min(band.bottom) - top.max(band.top);
        let needed = OVERLAP * height.min(band.height());
        if overlap >= needed && best.is_none_or(|(_, o)| overlap > o) {
            best = Some((index, overlap));
        }
    }
    best.map(|(index, _)| index)
}

/// The band a mark of extent `across` x `along` joins: the nearest one above or below it within
/// 0.7 of that band's height, whose letters reach it along the line.
fn nearest_for_mark(bands: &[Band], across: (f64, f64), along: (f64, f64)) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for (index, band) in bands.iter().enumerate() {
        let reach = 0.3 * band.height();
        if across.1 - across.0 > MARK * band.height() {
            continue;
        }
        if along.1 < band.left - reach || along.0 > band.right + reach {
            continue;
        }
        let gap = (band.top - across.1).max(across.0 - band.bottom).max(0.0);
        if gap <= 0.7 * band.height() && best.is_none_or(|(_, g)| gap < g) {
            best = Some((index, gap));
        }
    }
    best.map(|(index, _)| index)
}

/// The bands of `blobs` in `frame`: letters the tallest first by overlap, then the marks by
/// overlap or nearness.
fn bands_of(blobs: &[Blob], frame: &Frame, typical: f64) -> Vec<Band> {
    let extents: Vec<(f64, f64)> = blobs.iter().map(|b| across(b, frame)).collect();
    let spans: Vec<(f64, f64)> = blobs.iter().map(|b| along(b, frame)).collect();
    let mut order: Vec<usize> = (0..blobs.len()).collect();
    // Tallest first; ties by input order.
    order.sort_by(|&a, &b| {
        let ha = extents[a].1 - extents[a].0;
        let hb = extents[b].1 - extents[b].0;
        hb.total_cmp(&ha).then(a.cmp(&b))
    });
    let mut bands: Vec<Band> = Vec::new();
    let mut marks: Vec<usize> = Vec::new();
    for &i in &order {
        let (top, bottom) = extents[i];
        if bottom - top < MARK * typical {
            marks.push(i);
            continue;
        }
        match overlapping(&bands, top, bottom, None) {
            Some(index) => {
                let band = &mut bands[index];
                band.blobs.push(i);
                band.left = band.left.min(spans[i].0);
                band.right = band.right.max(spans[i].1);
            }
            None => bands.push(Band {
                top,
                bottom,
                left: spans[i].0,
                right: spans[i].1,
                blobs: vec![i],
            }),
        }
    }
    for i in marks {
        let (top, bottom) = extents[i];
        let joined = overlapping(&bands, top, bottom, Some(spans[i]))
            .or_else(|| nearest_for_mark(&bands, extents[i], spans[i]));
        match joined {
            Some(index) => bands[index].blobs.push(i),
            None => bands.push(Band {
                top,
                bottom,
                left: spans[i].0,
                right: spans[i].1,
                blobs: vec![i],
            }),
        }
    }
    bands
}

/// Groups `blobs` into lines (`adrs.md` decision 3 stage 2).
pub(crate) fn group_lines(blobs: &[Blob]) -> Vec<Line> {
    if blobs.is_empty() {
        return Vec::new();
    }
    let mut heights: Vec<f64> = blobs.iter().map(|b| b.bbox.height()).collect();
    let typical = median(&mut heights);
    let centres: Vec<Point> = blobs
        .iter()
        .filter(|b| b.bbox.height() <= TALL * typical && b.bbox.height() >= MARK * typical)
        .map(Blob::centre)
        .collect();
    let frame = Frame {
        origin: Point::ZERO,
        u: principal_axis(&centres),
    };
    let bands = bands_of(blobs, &frame, typical);
    let mut lines = Vec::new();
    for band in bands {
        let band_height = band.height();
        let mut members: Vec<(f64, f64, usize)> = band
            .blobs
            .iter()
            .map(|&i| {
                let (left, right) = along(&blobs[i], &frame);
                (left, right, i)
            })
            .collect();
        members.sort_by(|a, b| {
            let ca = f64::midpoint(a.0, a.1);
            let cb = f64::midpoint(b.0, b.1);
            ca.total_cmp(&cb).then(a.2.cmp(&b.2))
        });
        let mut current: Vec<usize> = Vec::new();
        let mut reach = f64::NEG_INFINITY;
        for (left, right, i) in members {
            if !current.is_empty() && left - reach > SPLIT_GAP * band_height {
                lines.push(std::mem::take(&mut current));
            }
            current.push(i);
            reach = reach.max(right);
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    // Lines in reading order: top to bottom, then left to right.
    let key = |line: &Vec<usize>| {
        let first = &blobs[line[0]];
        let (top, _) = across(first, &frame);
        let (left, _) = along(first, &frame);
        (top, left)
    };
    lines.sort_by(|a, b| {
        let (ta, la) = key(a);
        let (tb, lb) = key(b);
        ta.total_cmp(&tb).then(la.total_cmp(&lb))
    });
    lines
        .into_iter()
        .map(|blobs| Line { blobs, frame })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_axis_of_a_tilted_row_is_found_without_angles() {
        let points: Vec<Point> = (0..10)
            .map(|i| Point::new(f(i) * 4.0, f(i) * 2.0))
            .collect();
        let u = principal_axis(&points);
        let expected = Vec2::new(2.0, 1.0) / 5.0_f64.sqrt();
        assert!((u - expected).hypot() < 1e-9);
    }

    #[test]
    fn frames_round_trip() {
        let frame = Frame {
            origin: Point::new(3.0, 4.0),
            u: Vec2::new(0.6, 0.8),
        };
        let p = Point::new(-2.0, 7.5);
        let back = frame.to_document(frame.to_line(p));
        assert!((back - p).hypot() < 1e-12);
    }
}
