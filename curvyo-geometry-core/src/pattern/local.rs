//! The region near a small box (`specs/0071-pattern-fill`, acceptance report finding 2): the
//! region's contours cut to one tile of its cell map grown by a margin, so a small piece is cut
//! against the boundary edges near it instead of every edge of the shape.

use std::collections::HashMap;

use crate::boolean_grid::{Path64, Paths64, Point64};

use super::region::Region;
use super::string_clip::round_div;

/// Cells along one side of the smallest tile.
const TILE_CELLS: i64 = 8;

/// A tile: its level (side `TILE_CELLS << level` cells) and its column and row at that level.
type Key = (u32, i64, i64);

/// The region's contours cut to tiles, made as they are needed: a tile is cut from the tile one
/// level up, so making one looks only at the edges near it.
pub(crate) struct LocalRegions<'r> {
    region: &'r Region,
    tiles: HashMap<Key, Paths64>,
    /// The smallest tile's side, grid units.
    base: i64,
    /// The levels below the whole region.
    levels: u32,
}

/// What [`LocalRegions::near`] gives: the contours to cut against and the edges scanned to make
/// the tiles it needed (zero when they were made before).
pub(crate) struct Near<'a> {
    pub(crate) contours: &'a Paths64,
    pub(crate) scanned: usize,
}

impl<'r> LocalRegions<'r> {
    pub(crate) fn new(region: &'r Region) -> Self {
        let (_, cell) = region.rows();
        let base = cell * TILE_CELLS;
        let extent = (region.max.x - region.min.x).max(region.max.y - region.min.y);
        let mut levels = 0;
        while (base << levels) < extent && levels < 40 {
            levels += 1;
        }
        Self {
            region,
            tiles: HashMap::new(),
            base,
            levels,
        }
    }

    /// The box of tile `key` with its margin of half a tile on each side.
    fn tile_box(&self, (level, column, row): Key) -> (Point64, Point64) {
        let side = self.base << level;
        let min = self.region.min;
        (
            Point64::new(
                min.x + column * side - side / 2,
                min.y + row * side - side / 2,
            ),
            Point64::new(
                min.x + (column + 1) * side + side / 2,
                min.y + (row + 1) * side + side / 2,
            ),
        )
    }

    /// The contours to cut a piece in the box `lo hi` against: those of the smallest tile whose
    /// margin holds the box, or the whole region's.
    pub(crate) fn near(&mut self, lo: Point64, hi: Point64) -> Near<'_> {
        let extent = (hi.x - lo.x).max(hi.y - lo.y);
        let mut level = 0;
        while level < self.levels && (self.base << level) / 2 <= extent {
            level += 1;
        }
        if level >= self.levels {
            return Near {
                contours: &self.region.contours,
                scanned: 0,
            };
        }
        let min = self.region.min;
        let side = self.base << level;
        let key = (
            level,
            (lo.x - min.x).div_euclid(side),
            (lo.y - min.y).div_euclid(side),
        );
        let scanned = self.make(key);
        Near {
            contours: &self.tiles[&key],
            scanned,
        }
    }

    /// The region cut to the box `lo hi` grown by [`AROUND`] units: what a piece in that box is
    /// cut against. It is cut from the tile [`LocalRegions::near`] gives, so only the edges near
    /// the box are looked at; the edges scanned come with it.
    pub(crate) fn around(&mut self, lo: Point64, hi: Point64) -> (Paths64, usize) {
        let (lo, hi) = (
            Point64::new(lo.x - AROUND, lo.y - AROUND),
            Point64::new(hi.x + AROUND, hi.y + AROUND),
        );
        let near = self.near(lo, hi);
        let mut scanned = near.scanned;
        let mut cut = Paths64::new();
        for contour in near.contours {
            scanned += contour.len();
            if let Some(clipped) = clip_contour(contour, lo, hi) {
                cut.push(clipped);
            }
        }
        (cut, scanned)
    }

    /// Makes tile `key` and the tiles above it that are missing; the edges scanned.
    fn make(&mut self, key: Key) -> usize {
        if self.tiles.contains_key(&key) {
            return 0;
        }
        let (level, column, row) = key;
        let above = (level + 1, column.div_euclid(2), row.div_euclid(2));
        let mut scanned = 0;
        let parent: &Paths64 = if level + 1 >= self.levels {
            &self.region.contours
        } else {
            scanned += self.make(above);
            &self.tiles[&above]
        };
        let (lo, hi) = self.tile_box(key);
        let mut cut = Paths64::new();
        for contour in parent {
            scanned += contour.len();
            if let Some(clipped) = clip_contour(contour, lo, hi) {
                cut.push(clipped);
            }
        }
        self.tiles.insert(key, cut);
        scanned
    }
}

/// How far the box a piece is cut against reaches past the piece, grid units: the points the
/// cut adds on the box's sides stay off the piece.
const AROUND: i64 = 16;

/// `contour` cut to the box, `None` when nothing of it is left.
fn clip_contour(contour: &Path64, lo: Point64, hi: Point64) -> Option<Path64> {
    let outside = contour.iter().all(|p| p.x < lo.x)
        || contour.iter().all(|p| p.x > hi.x)
        || contour.iter().all(|p| p.y < lo.y)
        || contour.iter().all(|p| p.y > hi.y);
    if outside {
        return None;
    }
    let clipped = clip_to_box(contour, lo, hi);
    (clipped.len() >= 3).then_some(clipped)
}

/// One side of the box: which coordinate (`true` for x), the bound, and whether points below it
/// are inside.
type Side = (bool, i64, bool);

/// `contour` cut to the box (Sutherland-Hodgman): inside the box it winds every point as the
/// contour does, so the cut contours paint the region within the box under the nonzero rule. The
/// new points on the box's sides are rounded to the grid.
fn clip_to_box(contour: &Path64, lo: Point64, hi: Point64) -> Path64 {
    let sides: [Side; 4] = [
        (true, lo.x, false),
        (true, hi.x, true),
        (false, lo.y, false),
        (false, hi.y, true),
    ];
    let mut points = contour.clone();
    for (is_x, bound, below) in sides {
        if points.is_empty() {
            break;
        }
        let value = |p: Point64| if is_x { p.x } else { p.y };
        let inside = |p: Point64| {
            if below {
                value(p) <= bound
            } else {
                value(p) >= bound
            }
        };
        let crossing = |a: Point64, b: Point64| {
            let (along, other) = if is_x {
                ((a.x, b.x), (a.y, b.y))
            } else {
                ((a.y, b.y), (a.x, b.x))
            };
            let at = i128::from(other.0)
                + round_div(
                    i128::from(other.1 - other.0) * i128::from(bound - along.0),
                    i128::from(along.1 - along.0),
                );
            // invariant: the point lies between `a` and `b`, which are i64.
            let at = i64::try_from(at).unwrap_or(other.0);
            if is_x {
                Point64::new(bound, at)
            } else {
                Point64::new(at, bound)
            }
        };
        let mut out = Path64::with_capacity(points.len() + 4);
        for i in 0..points.len() {
            let (a, b) = (points[i], points[(i + 1) % points.len()]);
            match (inside(a), inside(b)) {
                (true, true) => out.push(b),
                (true, false) => out.push(crossing(a, b)),
                (false, true) => {
                    out.push(crossing(a, b));
                    out.push(b);
                }
                (false, false) => {}
            }
        }
        out.dedup();
        while out.len() > 1 && out.first() == out.last() {
            out.pop();
        }
        points = out;
    }
    drop_side_runs(&points, lo, hi)
}

/// `path` without the points inside a run along one side of the box: the parts of a contour
/// outside the box come back as back-and-forth runs on its sides, which enclose nothing.
fn drop_side_runs(path: &Path64, lo: Point64, hi: Point64) -> Path64 {
    let side = |p: Point64| [p.x == lo.x, p.x == hi.x, p.y == lo.y, p.y == hi.y];
    let same_side = |a: Point64, b: Point64, c: Point64| {
        let (sa, sb, sc) = (side(a), side(b), side(c));
        (0..4).any(|i| sa[i] && sb[i] && sc[i])
    };
    let mut out: Path64 = Vec::with_capacity(path.len());
    for &p in path {
        while out.len() >= 2 && same_side(out[out.len() - 2], out[out.len() - 1], p) {
            out.pop();
        }
        if out.last() != Some(&p) {
            out.push(p);
        }
    }
    // The run through the first point, which the pass above saw from one end only.
    while out.len() >= 3 && same_side(out[out.len() - 2], out[out.len() - 1], out[0]) {
        out.pop();
    }
    while out.len() >= 3 && same_side(out[out.len() - 1], out[0], out[1]) {
        out.remove(0);
    }
    while out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: i64, y: i64) -> Point64 {
        Point64::new(x, y)
    }

    /// `path` starting at its smallest point, so cuts compare whatever their first point.
    fn from_least(mut path: Path64) -> Path64 {
        if let Some(at) = (0..path.len()).min_by_key(|&i| (path[i].x, path[i].y)) {
            path.rotate_left(at);
        }
        path
    }

    #[test]
    fn a_square_is_cut_to_the_box() {
        let square = vec![p(0, 0), p(10, 0), p(10, 10), p(0, 10)];
        let cut = from_least(clip_to_box(&square, p(5, -5), p(20, 5)));
        assert_eq!(cut, vec![p(5, 0), p(10, 0), p(10, 5), p(5, 5)]);
    }

    #[test]
    fn runs_on_the_sides_are_dropped() {
        // A comb whose teeth and the gaps between them lie below the box: cut, they come back
        // as runs on the box's lower side.
        let comb = vec![
            p(0, 0),
            p(10, 0),
            p(10, -10),
            p(8, -10),
            p(8, -8),
            p(6, -8),
            p(6, -10),
            p(4, -10),
            p(4, -8),
            p(2, -8),
            p(2, -10),
            p(0, -10),
        ];
        let cut = from_least(clip_to_box(&comb, p(-5, -5), p(20, 20)));
        assert_eq!(cut, vec![p(0, -5), p(0, 0), p(10, 0), p(10, -5)]);
    }

    #[test]
    fn a_contour_outside_the_box_is_dropped() {
        let square = vec![p(0, 0), p(10, 0), p(10, 10), p(0, 10)];
        assert!(clip_to_box(&square, p(20, 20), p(30, 30)).len() < 3);
    }

    #[test]
    fn a_diagonal_edge_is_cut_on_the_side() {
        let triangle = vec![p(0, 0), p(10, 0), p(0, 10)];
        let cut = from_least(clip_to_box(&triangle, p(0, 0), p(4, 100)));
        assert_eq!(cut, vec![p(0, 0), p(4, 0), p(4, 6), p(0, 10)]);
    }
}
