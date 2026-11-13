//! The geometry of a lasso region of a trace mask in picture pixels of the working image
//! (`specs/0114-image-trace-mask` Behaviour 3 and 4, UX notes §5 and §6): corners set click by
//! click on whole pixels, the finish rule, hit testing by the non-zero winding rule, and moving
//! the lasso or one corner. The stored lasso is in the image's unit square (ADR 0061 §1).

use curvyo_document_core::UnitPoint;
use kurbo::Point;

/// A corner on a whole picture pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PxCorner {
    /// Across the picture.
    pub x: i32,
    /// Down the picture.
    pub y: i32,
}

impl PxCorner {
    /// The corner (`x`, `y`).
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// The pixel nearest to `p`.
    #[must_use]
    pub fn nearest(p: Point) -> Self {
        Self::new(whole(p.x), whole(p.y))
    }

    /// The corner as a kurbo point.
    #[must_use]
    pub fn to_point(self) -> Point {
        Point::new(f64::from(self.x), f64::from(self.y))
    }
}

/// `v` rounded to a whole pixel, held to a range no later sum overflows; a NaN becomes 0.
fn whole(v: f64) -> i32 {
    // invariant: rounded and clamped into i32's range first.
    #[allow(clippy::cast_possible_truncation)]
    let r = v.round().clamp(-1.0e9, 1.0e9) as i32;
    r
}

/// A lasso being drawn: its corners so far.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LassoDraft {
    corners: Vec<PxCorner>,
}

impl LassoDraft {
    /// The corners set so far, in order.
    #[must_use]
    pub fn corners(&self) -> &[PxCorner] {
        &self.corners
    }

    /// Sets a corner at the pixel nearest to `p`; a click on the pixel of the last corner adds
    /// nothing. Whether a corner was added.
    pub fn add(&mut self, p: Point) -> bool {
        let corner = PxCorner::nearest(p);
        if self.corners.last() == Some(&corner) {
            return false;
        }
        self.corners.push(corner);
        true
    }

    /// Backspace: removes the last corner. Whether any corner is left.
    pub fn remove_last(&mut self) -> bool {
        self.corners.pop();
        !self.corners.is_empty()
    }

    /// Whether `p` is on the close target: three corners or more and within `radius` picture
    /// pixels of the first one.
    #[must_use]
    pub fn closes_at(&self, p: Point, radius: f64) -> bool {
        self.corners.len() >= 3
            && self
                .corners
                .first()
                .is_some_and(|first| first.to_point().distance(p) <= radius)
    }

    /// The lasso this draft finishes as, closed from the last corner to the first; `None` with
    /// fewer than three corners or with every corner on one line.
    #[must_use]
    pub fn finished(&self) -> Option<Vec<PxCorner>> {
        finished(&self.corners)
    }
}

/// `corners` as a lasso: `None` with fewer than three or with every one on one line.
#[must_use]
pub fn finished(corners: &[PxCorner]) -> Option<Vec<PxCorner>> {
    let [first, rest @ ..] = corners else {
        return None;
    };
    if corners.len() < 3 {
        return None;
    }
    let cross = |a: PxCorner, b: PxCorner| {
        let (ax, ay) = (i64::from(a.x - first.x), i64::from(a.y - first.y));
        let (bx, by) = (i64::from(b.x - first.x), i64::from(b.y - first.y));
        ax * by - ay * bx
    };
    let spans = rest.iter().any(|&a| rest.iter().any(|&b| cross(a, b) != 0));
    spans.then(|| corners.to_vec())
}

/// The corners of a stored lasso `points` on a `width` x `height` working image, on whole pixels.
#[must_use]
pub fn lasso_from_unit(points: &[UnitPoint], width: u32, height: u32) -> Vec<PxCorner> {
    let (w, h) = (f64::from(width), f64::from(height));
    points
        .iter()
        .map(|p| PxCorner::nearest(Point::new(p.u * w, p.v * h)))
        .collect()
}

/// `corners` in the unit square of a `width` x `height` working image.
#[must_use]
pub fn lasso_to_unit(corners: &[PxCorner], width: u32, height: u32) -> Vec<UnitPoint> {
    let (w, h) = (f64::from(width.max(1)), f64::from(height.max(1)));
    corners
        .iter()
        .map(|c| UnitPoint::new(f64::from(c.x) / w, f64::from(c.y) / h))
        .collect()
}

/// The winding number of the closed polygon `corners` around `p`.
fn winding(corners: &[PxCorner], p: Point) -> i32 {
    let mut wn = 0;
    for (i, a) in corners.iter().enumerate() {
        let (a, b) = (a.to_point(), corners[(i + 1) % corners.len()].to_point());
        let side = (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y);
        if a.y <= p.y {
            if b.y > p.y && side > 0.0 {
                wn += 1;
            }
        } else if b.y <= p.y && side < 0.0 {
            wn -= 1;
        }
    }
    wn
}

/// The distance from `p` to the segment `a`-`b`.
fn segment_distance(a: Point, b: Point, p: Point) -> f64 {
    let ab = b - a;
    let len2 = ab.hypot2();
    let t = if len2 > 0.0 {
        ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a + ab * t).distance(p)
}

/// Whether `p` hits the lasso `corners`: inside its non-zero winding area, or within `slop`
/// pixels of a side (the closing side included).
#[must_use]
pub fn lasso_hit(corners: &[PxCorner], p: Point, slop: f64) -> bool {
    if corners.len() < 2 {
        return false;
    }
    if corners.len() >= 3 && winding(corners, p) != 0 {
        return true;
    }
    corners.iter().enumerate().any(|(i, a)| {
        let b = corners[(i + 1) % corners.len()];
        segment_distance(a.to_point(), b.to_point(), p) <= slop
    })
}

/// The corner of `corners` within `radius` pixels of `p`, the nearest one winning.
#[must_use]
pub fn lasso_corner_at(corners: &[PxCorner], p: Point, radius: f64) -> Option<usize> {
    corners
        .iter()
        .enumerate()
        .map(|(i, c)| (i, c.to_point().distance(p)))
        .filter(|(_, d)| *d <= radius)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

/// `corners` moved by (`dx`, `dy`) pixels.
#[must_use]
pub fn moved_lasso(corners: &[PxCorner], dx: i32, dy: i32) -> Vec<PxCorner> {
    corners
        .iter()
        .map(|c| PxCorner::new(c.x.saturating_add(dx), c.y.saturating_add(dy)))
        .collect()
}

/// `corners` with corner `index` moved to the pixel nearest to `to` (it may make the lasso cross
/// itself); unchanged for an index out of range.
#[must_use]
pub fn with_lasso_corner(corners: &[PxCorner], index: usize, to: Point) -> Vec<PxCorner> {
    let mut moved = corners.to_vec();
    if let Some(corner) = moved.get_mut(index) {
        *corner = PxCorner::nearest(to);
    }
    moved
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn c(x: i32, y: i32) -> PxCorner {
        PxCorner::new(x, y)
    }

    #[test]
    fn a_click_sets_a_corner_on_the_nearest_pixel_and_a_repeat_adds_nothing() {
        let mut draft = LassoDraft::default();
        assert!(draft.add(pt(10.4, 20.6)));
        assert!(
            !draft.add(pt(9.8, 21.2)),
            "the same pixel as the last corner"
        );
        assert!(draft.add(pt(30.0, 20.0)));
        assert_eq!(draft.corners(), &[c(10, 21), c(30, 20)]);
    }

    #[test]
    fn fewer_than_three_corners_or_one_line_finish_as_nothing() {
        assert_eq!(finished(&[]), None);
        assert_eq!(finished(&[c(0, 0), c(5, 5)]), None);
        assert_eq!(finished(&[c(0, 0), c(5, 5), c(10, 10), c(-3, -3)]), None);
        assert_eq!(
            finished(&[c(0, 0), c(5, 0), c(0, 5)]),
            Some(vec![c(0, 0), c(5, 0), c(0, 5)])
        );
    }

    #[test]
    fn backspace_removes_the_last_corner_and_the_only_one_empties_the_draft() {
        let mut draft = LassoDraft::default();
        draft.add(pt(0.0, 0.0));
        draft.add(pt(5.0, 0.0));
        assert!(draft.remove_last());
        assert_eq!(draft.corners(), &[c(0, 0)]);
        assert!(!draft.remove_last());
        assert_eq!(draft.corners(), &[] as &[PxCorner]);
    }

    #[test]
    fn the_close_target_needs_three_corners_and_the_radius() {
        let mut draft = LassoDraft::default();
        draft.add(pt(0.0, 0.0));
        draft.add(pt(50.0, 0.0));
        assert!(!draft.closes_at(pt(1.0, 1.0), 16.0), "two corners");
        draft.add(pt(0.0, 50.0));
        assert!(draft.closes_at(pt(10.0, 10.0), 16.0));
        assert!(!draft.closes_at(pt(12.0, 12.0), 16.0));
    }

    #[test]
    fn a_lasso_reads_from_and_writes_to_the_unit_square_on_whole_pixels() {
        let corners = vec![c(0, 0), c(600, 0), c(0, 1000)];
        let unit = lasso_to_unit(&corners, 1000, 1000);
        assert_eq!(unit[1], UnitPoint::new(0.6, 0.0));
        assert_eq!(lasso_from_unit(&unit, 1000, 1000), corners);
        let snapped = lasso_from_unit(&[UnitPoint::new(0.10049, 0.5)], 1000, 800);
        assert_eq!(snapped, vec![c(100, 400)]);
    }

    /// UX notes §5 and §6, criterion 23: a figure 8 is hit wherever it covers, by non-zero
    /// winding, and within the slop of its sides.
    #[test]
    fn a_self_crossing_lasso_is_hit_by_non_zero_winding() {
        let eight = [c(0, 0), c(100, 100), c(100, 0), c(0, 100)];
        assert!(lasso_hit(&eight, pt(10.0, 50.0), 0.0), "left lobe");
        assert!(lasso_hit(&eight, pt(90.0, 50.0), 0.0), "right lobe");
        assert!(
            !lasso_hit(&eight, pt(50.0, 10.0), 0.0),
            "above the crossing"
        );
        assert!(lasso_hit(&eight, pt(50.0, 46.0), 8.0), "near a side");
        assert!(!lasso_hit(&eight, pt(50.0, 30.0), 8.0));
        let star = [c(50, 0), c(80, 100), c(0, 35), c(100, 35), c(20, 100)];
        assert!(
            lasso_hit(&star, pt(50.0, 50.0), 0.0),
            "the pentagram's centre winds twice"
        );
    }

    #[test]
    fn the_nearest_corner_within_the_radius_wins() {
        let corners = [c(0, 0), c(10, 0), c(0, 10)];
        assert_eq!(lasso_corner_at(&corners, pt(6.0, 0.0), 16.0), Some(1));
        assert_eq!(lasso_corner_at(&corners, pt(4.0, 0.0), 16.0), Some(0));
        assert_eq!(lasso_corner_at(&corners, pt(40.0, 40.0), 16.0), None);
    }

    #[test]
    fn a_lasso_moves_whole_and_one_corner_moves_alone() {
        let corners = [c(0, 0), c(10, 0), c(0, 10)];
        assert_eq!(
            moved_lasso(&corners, 3, -2),
            vec![c(3, -2), c(13, -2), c(3, 8)]
        );
        assert_eq!(
            with_lasso_corner(&corners, 1, pt(20.4, 5.6)),
            vec![c(0, 0), c(20, 6), c(0, 10)]
        );
        assert_eq!(
            with_lasso_corner(&corners, 7, pt(1.0, 1.0)),
            corners.to_vec()
        );
    }
}
