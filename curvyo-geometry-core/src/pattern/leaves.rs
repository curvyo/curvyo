//! Leaf forest (`specs/0071-pattern-fill` criterion 9, `adrs.md` decision 2): seeded dart throwing
//! of small leaves over the region's box. Each leaf is two cubic arcs inside a circle of half its
//! length; a candidate whose circle comes within [`GAP_MM`] of an accepted one is dropped, so no
//! two leaves overlap. The throwing ends when the leaves aimed for are placed, after
//! [`MAX_MISSES`] drops in a row (the box is as full as throwing gets it) or after
//! [`MAX_THROWS`] candidates, so its work is bounded (`review-architecture.md` change 4).

use curvyo_document_core::{Angle, Length, PatternFraction, PatternSeed, Point, Vec2};

use super::seed::Stream;
use super::{Bounds, Raw};

/// The least distance between two leaves' circles, mm: well above the kernel tolerance.
const GAP_MM: f64 = 0.05;
/// Candidates thrown per leaf aimed for before the forest is left as it is.
const TRIES_PER_LEAF: usize = 30;
/// Candidates dropped in a row after which the forest is left as it is: the box is as full as
/// random throwing gets it. Measured in `plan.md`: the defaults are not reached by it.
const MAX_MISSES: usize = 512;
/// The most candidates thrown for one forest; [`leaf_work`] counts with it.
pub(crate) const MAX_THROWS: usize = 2_000_000;
/// A leaf's half width over its length, and the handle offset that gives it (a cubic's bulge is
/// three quarters of its handles' offset).
const HALF_WIDTH: f64 = 0.15;
const BULGE: f64 = HALF_WIDTH / 0.75;

/// The leaves aimed for: `density` of the box's area covered by nominal circles.
pub(crate) fn leaf_count(bounds: Bounds, leaf_size: Length, density: PatternFraction) -> usize {
    let (lo, hi) = bounds;
    let r = leaf_size.as_mm() / 2.0;
    let n = density.get() * (hi.x - lo.x) * (hi.y - lo.y) / (std::f64::consts::PI * r * r);
    if n.is_finite() && n >= 0.0 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let count = n.min(1e15).ceil() as usize;
        count
    } else {
        usize::MAX
    }
}

/// One leaf of `length` around `centre`, pointing along `direction` (a unit vector).
fn leaf(centre: Point, length: f64, direction: Vec2) -> Raw {
    let normal = Vec2::new(-direction.y, direction.x);
    let tip = direction.scaled(length / 2.0);
    let a = Point::new(centre.x - tip.x, centre.y - tip.y);
    let b = Point::new(centre.x + tip.x, centre.y + tip.y);
    let reach = direction.scaled(length / 3.0);
    let bulge = normal.scaled(BULGE * length);
    // A to B along one side, B back to A along the other.
    Raw::closed(vec![
        (a, reach - bulge, reach + bulge),
        (b, reach.negated() + bulge, reach.negated() - bulge),
    ])
}

/// The candidates the forest throws at most.
pub(crate) fn leaf_throws(bounds: Bounds, leaf_size: Length, density: PatternFraction) -> usize {
    leaf_count(bounds, leaf_size, density)
        .saturating_mul(TRIES_PER_LEAF)
        .min(MAX_THROWS)
}

/// The accepted leaves by the cell of their centre: a cell is at least as wide as the largest
/// leaf plus the gap, so a candidate can only meet leaves in its own and the eight cells around
/// it; the cells are widened so there are about two per leaf aimed for at most. Each leaf's circle
/// is kept with its link, so a test reads one place per leaf (`plan.md`, "Measurements").
struct Cells {
    lo: Point,
    size: f64,
    columns: usize,
    rows: usize,
    /// The last leaf in each cell.
    head: Vec<u32>,
    /// Per leaf: its circle and the leaf before it in the same cell.
    leaves: Vec<Circle>,
}

/// An accepted leaf's circle and the leaf added before it to the same cell.
struct Circle {
    centre: Point,
    radius: f64,
    before: u32,
}

const NONE: u32 = u32::MAX;

impl Cells {
    fn new(bounds: Bounds, least: f64, wanted: usize) -> Self {
        let (lo, hi) = bounds;
        let (w, h) = (hi.x - lo.x, hi.y - lo.y);
        #[allow(clippy::cast_precision_loss)]
        let size = least
            .max((w * h / (2.0 * wanted as f64 + 1.0)).sqrt())
            .max(w / 4096.0)
            .max(h / 4096.0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let count = |extent: f64| (extent / size).floor() as usize + 1;
        let (columns, rows) = (count(w), count(h));
        Self {
            lo,
            size,
            columns,
            rows,
            head: vec![NONE; columns * rows],
            leaves: Vec::new(),
        }
    }

    fn cell(&self, p: Point) -> (usize, usize) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let at = |v: f64, n: usize| (((v / self.size).floor().max(0.0)) as usize).min(n - 1);
        (
            at(p.x - self.lo.x, self.columns),
            at(p.y - self.lo.y, self.rows),
        )
    }

    /// Whether every leaf in cell `at` keeps the gap to a circle of `radius` around `centre`.
    fn clear_in(&self, at: usize, centre: Point, radius: f64) -> bool {
        let mut i = self.head[at];
        while i != NONE {
            let other = &self.leaves[i as usize];
            let (dx, dy) = (other.centre.x - centre.x, other.centre.y - centre.y);
            let least = other.radius + radius + GAP_MM;
            if dx * dx + dy * dy < least * least {
                return false;
            }
            i = other.before;
        }
        true
    }

    /// Whether every leaf near `centre` keeps the gap to a circle of `radius` around it: its own
    /// cell first, where a candidate in a full box usually fails.
    fn clear(&self, centre: Point, radius: f64) -> bool {
        let (cx, cy) = self.cell(centre);
        let own = cy * self.columns + cx;
        if !self.clear_in(own, centre, radius) {
            return false;
        }
        for y in cy.saturating_sub(1)..=(cy + 1).min(self.rows - 1) {
            for x in cx.saturating_sub(1)..=(cx + 1).min(self.columns - 1) {
                let at = y * self.columns + x;
                if at != own && !self.clear_in(at, centre, radius) {
                    return false;
                }
            }
        }
        true
    }

    fn add(&mut self, centre: Point, radius: f64) {
        let (cx, cy) = self.cell(centre);
        let at = cy * self.columns + cx;
        self.leaves.push(Circle {
            centre,
            radius,
            before: self.head[at],
        });
        self.head[at] = u32::try_from(self.leaves.len() - 1).unwrap_or(NONE);
    }
}

/// The forest over the box.
pub(crate) fn leaves(
    bounds: Bounds,
    leaf_size: Length,
    size_jitter: PatternFraction,
    rotation_jitter: Angle,
    density: PatternFraction,
    seed: PatternSeed,
) -> Vec<Raw> {
    let (lo, hi) = bounds;
    let wanted = leaf_count(bounds, leaf_size, density);
    let largest = leaf_size.as_mm() * (1.0 + size_jitter.get());
    let mut stream = Stream::new(seed);
    let mut cells = Cells::new(bounds, largest + GAP_MM, wanted);
    let mut out = Vec::new();
    let mut misses = 0;
    for _ in 0..leaf_throws(bounds, leaf_size, density) {
        if out.len() >= wanted || misses >= MAX_MISSES {
            break;
        }
        let centre = Point::new(
            lo.x + stream.next_unit() * (hi.x - lo.x),
            lo.y + stream.next_unit() * (hi.y - lo.y),
        );
        let length = leaf_size.as_mm() * (1.0 + size_jitter.get() * stream.next_signed());
        let turn = rotation_jitter.as_radians() * stream.next_signed();
        let radius = length / 2.0;
        if !cells.clear(centre, radius) {
            misses += 1;
            continue;
        }
        misses = 0;
        cells.add(centre, radius);
        let direction = Vec2::new(libm::cos(turn), libm::sin(turn));
        out.push(leaf(centre, length, direction));
    }
    out
}
