//! The geometry of drawing, finding, moving and resizing a rectangle region of a trace mask, in
//! picture pixels of the working image (`specs/0114-image-trace-mask` Behaviour 2, ported from
//! Tracyo's `region_edit`); the stored region is in the image's unit square (ADR 0061 §1).

use curvyo_document_core::UnitPoint;
use kurbo::{Point, Rect};

/// A rectangle on whole picture pixels; never empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PxRect {
    /// Left edge.
    pub x0: i32,
    /// Top edge.
    pub y0: i32,
    /// Right edge, exclusive.
    pub x1: i32,
    /// Bottom edge, exclusive.
    pub y1: i32,
}

impl PxRect {
    /// The rectangle between two corners given in any order; a side that would be 0 px long is
    /// made 1 px long, so a region always has an area.
    #[must_use]
    pub fn new(xa: i32, ya: i32, xb: i32, yb: i32) -> Self {
        let (x0, x1) = ordered(xa, xb);
        let (y0, y1) = ordered(ya, yb);
        Self { x0, y0, x1, y1 }
    }

    /// Width in px.
    #[must_use]
    pub fn width(&self) -> i32 {
        self.x1.saturating_sub(self.x0)
    }

    /// Height in px.
    #[must_use]
    pub fn height(&self) -> i32 {
        self.y1.saturating_sub(self.y0)
    }

    /// The same rectangle as a kurbo rectangle.
    #[must_use]
    pub fn to_kurbo(&self) -> kurbo::Rect {
        kurbo::Rect::new(
            f64::from(self.x0),
            f64::from(self.y0),
            f64::from(self.x1),
            f64::from(self.y1),
        )
    }

    /// The rectangle of a stored region's two corners `points` on a `width` x `height` working
    /// image, snapped to whole pixels.
    #[must_use]
    pub fn from_unit(points: [UnitPoint; 2], width: u32, height: u32) -> Self {
        let (w, h) = (f64::from(width), f64::from(height));
        let [a, b] = points;
        Self::new(px(a.u * w), px(a.v * h), px(b.u * w), px(b.v * h))
    }

    /// The two corners of the rectangle in the unit square of a `width` x `height` working
    /// image, top-left first.
    #[must_use]
    pub fn to_unit(self, width: u32, height: u32) -> [UnitPoint; 2] {
        let (w, h) = (f64::from(width.max(1)), f64::from(height.max(1)));
        [
            UnitPoint::new(f64::from(self.x0) / w, f64::from(self.y0) / h),
            UnitPoint::new(f64::from(self.x1) / w, f64::from(self.y1) / h),
        ]
    }
}

/// `a` and `b` as (low, high), 1 apart when they are equal.
fn ordered(a: i32, b: i32) -> (i32, i32) {
    match a.cmp(&b) {
        std::cmp::Ordering::Less => (a, b),
        std::cmp::Ordering::Greater => (b, a),
        std::cmp::Ordering::Equal if a == i32::MAX => (a - 1, a),
        std::cmp::Ordering::Equal => (a, a + 1),
    }
}

/// A resize handle of a selected rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    /// Top edge.
    N,
    /// Top-right corner.
    NE,
    /// Right edge.
    E,
    /// Bottom-right corner.
    SE,
    /// Bottom edge.
    S,
    /// Bottom-left corner.
    SW,
    /// Left edge.
    W,
    /// Top-left corner.
    NW,
}

impl Handle {
    /// The eight handles, corners and edges, clockwise from the top-left corner.
    pub const ALL: [Self; 8] = [
        Self::NW,
        Self::N,
        Self::NE,
        Self::E,
        Self::SE,
        Self::S,
        Self::SW,
        Self::W,
    ];

    /// The handle's direction: -1, 0 or 1 along x and along y.
    #[must_use]
    pub const fn direction(self) -> (i32, i32) {
        match self {
            Self::N => (0, -1),
            Self::NE => (1, -1),
            Self::E => (1, 0),
            Self::SE => (1, 1),
            Self::S => (0, 1),
            Self::SW => (-1, 1),
            Self::W => (-1, 0),
            Self::NW => (-1, -1),
        }
    }

    /// Whether the handle sits on a corner.
    #[must_use]
    pub const fn is_corner(self) -> bool {
        let (x, y) = self.direction();
        x != 0 && y != 0
    }

    /// Where the handle sits on `rect`.
    #[must_use]
    pub fn position(self, rect: Rect) -> Point {
        let (dx, dy) = self.direction();
        let pick = |d: i32, lo: f64, hi: f64| match d {
            -1 => lo,
            1 => hi,
            _ => f64::midpoint(lo, hi),
        };
        Point::new(pick(dx, rect.x0, rect.x1), pick(dy, rect.y0, rect.y1))
    }
}

/// The picture pixel nearest to `v`, held to a range that cannot overflow later sums.
fn px(v: f64) -> i32 {
    const LIMIT: f64 = 1.0e9;
    // invariant: clamped into i32's range and rounded above, a NaN becomes 0.
    #[allow(clippy::cast_possible_truncation)]
    let r = v.round().clamp(-LIMIT, LIMIT) as i32;
    r
}

/// The rectangle drawn by a drag from `from` to `to` (picture pixels): corner to corner, from the
/// centre when `centre`, a square when `square`. A drag shorter than 2 px in both directions draws
/// nothing; corners land on whole pixels and a side is at least 1 px.
#[must_use]
pub fn drawn(from: Point, to: Point, centre: bool, square: bool) -> Option<PxRect> {
    let (mut dx, mut dy) = (to.x - from.x, to.y - from.y);
    if dx.abs().max(dy.abs()) < 2.0 {
        return None;
    }
    if square {
        let side = dx.abs().max(dy.abs());
        dx = side.copysign(if dx < 0.0 { -1.0 } else { 1.0 });
        dy = side.copysign(if dy < 0.0 { -1.0 } else { 1.0 });
    }
    let (ax, ay) = (px(from.x), px(from.y));
    if centre {
        let (hx, hy) = if square {
            let half = px(dx.abs());
            (half, half)
        } else {
            (px(dx.abs()), px(dy.abs()))
        };
        return Some(PxRect::new(
            ax.saturating_sub(hx),
            ay.saturating_sub(hy),
            ax.saturating_add(hx),
            ay.saturating_add(hy),
        ));
    }
    if square {
        let side = px(dx.abs());
        let sx = if dx < 0.0 { -side } else { side };
        let sy = if dy < 0.0 { -side } else { side };
        return Some(PxRect::new(
            ax,
            ay,
            ax.saturating_add(sx),
            ay.saturating_add(sy),
        ));
    }
    Some(PxRect::new(ax, ay, px(to.x), px(to.y)))
}

/// The topmost (last) of `rects` that holds `p`, each grown by `slop_px`.
#[must_use]
pub fn rect_at(rects: &[PxRect], p: Point, slop_px: f64) -> Option<usize> {
    rects
        .iter()
        .rposition(|rect| rect.to_kurbo().inflate(slop_px, slop_px).contains(p))
}

/// The handle of the selected rectangle `rect` (screen pixels) the pointer is on: corners always,
/// edge handles only while the shorter side is 24 px or more; the hit radius of a handle is a third
/// of the shorter side, between 4 and 16 px; the nearest centre wins.
#[must_use]
pub fn handle_at(rect: Rect, pointer: Point) -> Option<Handle> {
    let short = rect.width().min(rect.height());
    let radius = (short / 3.0).clamp(4.0, 16.0);
    Handle::ALL
        .into_iter()
        .filter(|h| h.is_corner() || short >= 24.0)
        .map(|h| (h, h.position(rect).distance(pointer)))
        .filter(|(_, d)| *d <= radius)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(h, _)| h)
}

/// `rect` with `handle` dragged to `to` (picture pixels): the opposite side stays, or with `centre`
/// the opposite side mirrors about the centre; with `keep_ratio` a corner handle keeps the
/// proportions. A side never gets shorter than 1 px.
#[must_use]
pub fn resized(rect: PxRect, handle: Handle, to: Point, centre: bool, keep_ratio: bool) -> PxRect {
    let (hx, hy) = handle.direction();
    let (w, h) = (f64::from(rect.width()), f64::from(rect.height()));
    let (mut tx, mut ty) = (to.x, to.y);
    if keep_ratio && handle.is_corner() {
        let cx = f64::midpoint(f64::from(rect.x0), f64::from(rect.x1));
        let cy = f64::midpoint(f64::from(rect.y0), f64::from(rect.y1));
        let (anchor_x, anchor_y, full_w, full_h) = if centre {
            (cx, cy, w / 2.0, h / 2.0)
        } else {
            (
                f64::from(if hx > 0 { rect.x0 } else { rect.x1 }),
                f64::from(if hy > 0 { rect.y0 } else { rect.y1 }),
                w,
                h,
            )
        };
        let scale = ((to.x - anchor_x).abs() / full_w).max((to.y - anchor_y).abs() / full_h);
        tx = anchor_x + f64::from(hx) * full_w * scale;
        ty = anchor_y + f64::from(hy) * full_h * scale;
    }
    let (x0, x1) = side(rect.x0, rect.x1, hx, px(tx), centre);
    let (y0, y1) = side(rect.y0, rect.y1, hy, px(ty), centre);
    PxRect { x0, y0, x1, y1 }
}

/// One axis of a resize: the `lo`..`hi` extent with the handle's direction `d` (-1 moves `lo`, 1
/// moves `hi`, 0 neither) dragged to `t`.
fn side(lo: i32, hi: i32, d: i32, t: i32, centre: bool) -> (i32, i32) {
    match d {
        0 => (lo, hi),
        _ if centre => {
            let sum = i64::from(lo) + i64::from(hi);
            // The dragged edge, mirrored: at least one pixel of width remains.
            let moved = i64::from(if d > 0 {
                t
            } else {
                sum_to_i32(sum - i64::from(t))
            });
            let high = moved.max((sum + 2).div_euclid(2));
            (sum_to_i32(sum - high), sum_to_i32(high))
        }
        1 => (lo, t.max(lo.saturating_add(1))),
        _ => (t.min(hi.saturating_sub(1)), hi),
    }
}

fn sum_to_i32(v: i64) -> i32 {
    i32::try_from(v).unwrap_or(if v < 0 { i32::MIN } else { i32::MAX })
}

/// `rect` moved by (`dx`, `dy`) pixels.
#[must_use]
pub fn moved(rect: PxRect, dx: i32, dy: i32) -> PxRect {
    PxRect {
        x0: rect.x0.saturating_add(dx),
        y0: rect.y0.saturating_add(dy),
        x1: rect.x1.saturating_add(dx),
        y1: rect.y1.saturating_add(dy),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn a_drag_runs_from_corner_to_corner_in_any_direction() {
        let a = drawn(pt(10.2, 20.7), pt(60.4, 50.5), false, false).unwrap();
        assert_eq!(a, PxRect::new(10, 21, 60, 51));
        let b = drawn(pt(60.4, 50.5), pt(10.2, 20.7), false, false).unwrap();
        assert_eq!(a.width(), b.width());
        assert_eq!(b, PxRect::new(10, 21, 60, 51));
    }

    #[test]
    fn a_drag_under_two_pixels_draws_nothing() {
        assert_eq!(drawn(pt(5.0, 5.0), pt(6.5, 6.9), false, false), None);
        assert_eq!(drawn(pt(5.0, 5.0), pt(5.0, 5.0), true, true), None);
        assert!(drawn(pt(5.0, 5.0), pt(7.0, 5.0), false, false).is_some());
    }

    #[test]
    fn a_drag_that_rounds_flat_still_has_an_area() {
        let r = drawn(pt(5.0, 5.0), pt(9.0, 5.2), false, false).unwrap();
        assert_eq!((r.width(), r.height()), (4, 1));
    }

    #[test]
    fn shift_draws_from_the_centre() {
        let r = drawn(pt(50.0, 40.0), pt(70.0, 55.0), true, false).unwrap();
        assert_eq!(r, PxRect::new(30, 25, 70, 55));
    }

    #[test]
    fn ctrl_draws_a_square_towards_the_pointer() {
        let r = drawn(pt(50.0, 40.0), pt(70.0, 55.0), false, true).unwrap();
        assert_eq!(r, PxRect::new(50, 40, 70, 60));
        let l = drawn(pt(50.0, 40.0), pt(30.0, 45.0), false, true).unwrap();
        assert_eq!(l, PxRect::new(30, 40, 50, 60));
        let both = drawn(pt(50.0, 40.0), pt(70.0, 55.0), true, true).unwrap();
        assert_eq!(both, PxRect::new(30, 20, 70, 60));
        assert_eq!(both.width(), both.height());
    }

    #[test]
    fn the_topmost_rectangle_wins_and_the_slop_reaches_past_the_outline() {
        let rects = [PxRect::new(0, 0, 100, 100), PxRect::new(40, 40, 60, 60)];
        assert_eq!(rect_at(&rects, pt(50.0, 50.0), 0.0), Some(1));
        assert_eq!(rect_at(&rects, pt(10.0, 10.0), 0.0), Some(0));
        assert_eq!(rect_at(&rects, pt(104.0, 50.0), 0.0), None);
        assert_eq!(rect_at(&rects, pt(104.0, 50.0), 8.0), Some(0));
        assert_eq!(rect_at(&rects, pt(-9.0, 50.0), 8.0), None);
        assert_eq!(rect_at(&[], pt(0.0, 0.0), 8.0), None);
    }

    #[test]
    fn a_rectangle_reads_from_and_writes_to_the_unit_square_on_whole_pixels() {
        let r = PxRect::from_unit(
            [UnitPoint::new(0.5, 1.0), UnitPoint::new(0.0, 0.0)],
            1000,
            800,
        );
        assert_eq!(r, PxRect::new(0, 0, 500, 800));
        assert_eq!(
            r.to_unit(1000, 800),
            [UnitPoint::new(0.0, 0.0), UnitPoint::new(0.5, 1.0)]
        );
        let snapped = PxRect::from_unit(
            [UnitPoint::new(0.1004, 0.0), UnitPoint::new(0.2, 0.5)],
            1000,
            800,
        );
        assert_eq!(snapped.x0, 100);
    }

    #[test]
    fn a_handle_is_hit_near_its_centre_and_the_nearest_wins() {
        let rect = Rect::new(100.0, 100.0, 300.0, 200.0);
        assert_eq!(handle_at(rect, pt(101.0, 102.0)), Some(Handle::NW));
        assert_eq!(handle_at(rect, pt(200.0, 98.0)), Some(Handle::N));
        assert_eq!(handle_at(rect, pt(302.0, 150.0)), Some(Handle::E));
        assert_eq!(handle_at(rect, pt(200.0, 150.0)), None);
        // 16 px is the largest radius.
        assert_eq!(handle_at(rect, pt(300.0 + 15.0, 100.0)), Some(Handle::NE));
        assert_eq!(handle_at(rect, pt(300.0 + 17.0, 100.0)), None);
    }

    #[test]
    fn a_small_rectangle_has_only_corner_handles_and_small_radii() {
        let small = Rect::new(0.0, 0.0, 20.0, 20.0);
        assert_eq!(
            handle_at(small, pt(10.0, 0.0)),
            None,
            "no edge handle under 24 px"
        );
        // A third of 20 px is 6.7 px: the corner at 6 px away is hit, 7 px away is not.
        assert_eq!(handle_at(small, pt(6.0, 0.0)), Some(Handle::NW));
        assert_eq!(handle_at(small, pt(7.0, 0.0)), None);
        let tiny = Rect::new(0.0, 0.0, 6.0, 6.0);
        // The radius is never under 4 px; the nearest of the overlapping corners wins.
        assert_eq!(handle_at(tiny, pt(1.0, 1.0)), Some(Handle::NW));
        assert_eq!(handle_at(tiny, pt(5.0, 5.0)), Some(Handle::SE));
        let edge_ok = Rect::new(0.0, 0.0, 100.0, 24.0);
        assert_eq!(handle_at(edge_ok, pt(50.0, 0.0)), Some(Handle::N));
    }

    #[test]
    fn an_edge_handle_moves_one_side() {
        let r = PxRect::new(10, 10, 50, 40);
        assert_eq!(
            resized(r, Handle::E, pt(70.4, 99.0), false, false),
            PxRect::new(10, 10, 70, 40)
        );
        assert_eq!(
            resized(r, Handle::N, pt(0.0, 2.0), false, false),
            PxRect::new(10, 2, 50, 40)
        );
        assert_eq!(
            resized(r, Handle::W, pt(-5.0, 0.0), false, false),
            PxRect::new(-5, 10, 50, 40)
        );
    }

    #[test]
    fn a_corner_handle_moves_two_sides_and_stops_at_one_pixel() {
        let r = PxRect::new(10, 10, 50, 40);
        assert_eq!(
            resized(r, Handle::SE, pt(60.0, 80.0), false, false),
            PxRect::new(10, 10, 60, 80)
        );
        let squeezed = resized(r, Handle::SE, pt(-30.0, -30.0), false, false);
        assert_eq!(squeezed, PxRect::new(10, 10, 11, 11));
        let nw = resized(r, Handle::NW, pt(90.0, 90.0), false, false);
        assert_eq!(nw, PxRect::new(49, 39, 50, 40));
    }

    #[test]
    fn shift_resizes_about_the_centre() {
        let r = PxRect::new(10, 10, 50, 40);
        let wide = resized(r, Handle::E, pt(70.0, 0.0), true, false);
        assert_eq!(wide, PxRect::new(-10, 10, 70, 40));
        let corner = resized(r, Handle::NW, pt(0.0, 0.0), true, false);
        assert_eq!(corner, PxRect::new(0, 0, 60, 50));
        let squeezed = resized(r, Handle::E, pt(-100.0, 0.0), true, false);
        assert_eq!(
            (squeezed.width(), squeezed.height()),
            (2, 30),
            "the centre stays, 2 px is the least with an even width"
        );
        let odd = resized(
            PxRect::new(10, 10, 51, 40),
            Handle::E,
            pt(-100.0, 0.0),
            true,
            false,
        );
        assert_eq!(odd.width(), 1);
    }

    #[test]
    fn ctrl_keeps_the_proportions_at_a_corner() {
        let r = PxRect::new(0, 0, 40, 20);
        let grown = resized(r, Handle::SE, pt(80.0, 30.0), false, true);
        assert_eq!(grown, PxRect::new(0, 0, 80, 40));
        let from_nw = resized(r, Handle::NW, pt(-40.0, -5.0), false, true);
        assert_eq!(from_nw, PxRect::new(-40, -20, 40, 20));
        let about_centre = resized(r, Handle::SE, pt(60.0, 30.0), true, true);
        assert_eq!(about_centre, PxRect::new(-20, -10, 60, 30));
        // An edge handle ignores the ratio.
        assert_eq!(
            resized(r, Handle::E, pt(100.0, 0.0), false, true),
            PxRect::new(0, 0, 100, 20)
        );
    }

    #[test]
    fn a_rectangle_moves_by_whole_pixels() {
        assert_eq!(
            moved(PxRect::new(1, 2, 5, 9), -3, 10),
            PxRect::new(-2, 12, 2, 19)
        );
        assert_eq!(moved(PxRect::new(1, 2, 5, 9), i32::MAX, 0).x1, i32::MAX);
    }
}
