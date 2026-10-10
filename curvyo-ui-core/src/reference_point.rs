//! The nine reference points of the typed move
//! (`specs/0047-transform-polish/` Part C, criteria 37 to 40, 43 and 50): the four corners,
//! the four edge midpoints and the centre of the true bounding box, the screen-pixel rule
//! that decides which of their marks a press is on, and when the marks are shown. The
//! choice of a point is view state of the open move chip: nothing here is stored.

use curvyo_document_core::Point;

/// One of the nine points of a box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReferencePoint {
    /// The top left corner.
    TopLeft,
    /// The middle of the top edge.
    TopCentre,
    /// The top right corner.
    TopRight,
    /// The middle of the left edge.
    LeftMiddle,
    /// The centre; what plain Enter uses (criterion 38).
    #[default]
    Centre,
    /// The middle of the right edge.
    RightMiddle,
    /// The bottom left corner.
    BottomLeft,
    /// The middle of the bottom edge.
    BottomCentre,
    /// The bottom right corner.
    BottomRight,
}

impl ReferencePoint {
    /// The nine points row by row, top row first, left to right: the order of the radios of
    /// the Reference control and of their accessible names (criterion 41).
    pub const ALL: [Self; 9] = [
        Self::TopLeft,
        Self::TopCentre,
        Self::TopRight,
        Self::LeftMiddle,
        Self::Centre,
        Self::RightMiddle,
        Self::BottomLeft,
        Self::BottomCentre,
        Self::BottomRight,
    ];

    /// The position in [`ReferencePoint::ALL`].
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::TopLeft => 0,
            Self::TopCentre => 1,
            Self::TopRight => 2,
            Self::LeftMiddle => 3,
            Self::Centre => 4,
            Self::RightMiddle => 5,
            Self::BottomLeft => 6,
            Self::BottomCentre => 7,
            Self::BottomRight => 8,
        }
    }

    /// The point at `index` of [`ReferencePoint::ALL`], `None` beyond it.
    #[must_use]
    pub fn from_index(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
    }

    /// Where this point lies on the box with corners `bounds` (`(min, max)`, document space,
    /// Y down).
    #[must_use]
    pub fn point(self, bounds: (Point, Point)) -> Point {
        let (min, max) = bounds;
        let column = self.index() % 3;
        let row = self.index() / 3;
        let pick = |low: f64, high: f64, step: usize| match step {
            0 => low,
            1 => f64::midpoint(low, high),
            _ => high,
        };
        Point::new(pick(min.x, max.x, column), pick(min.y, max.y, row))
    }

    /// Whether this is the centre (a cross mark; the others are squares).
    #[must_use]
    pub const fn is_centre(self) -> bool {
        matches!(self, Self::Centre)
    }

    /// The accessible name of its radio (criterion 41).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::TopLeft => "Top left",
            Self::TopCentre => "Top centre",
            Self::TopRight => "Top right",
            Self::LeftMiddle => "Left middle",
            Self::Centre => "Centre",
            Self::RightMiddle => "Right middle",
            Self::BottomLeft => "Bottom left",
            Self::BottomCentre => "Bottom centre",
            Self::BottomRight => "Bottom right",
        }
    }

    /// The first line of the hint chip of its mark (criterion 43).
    #[must_use]
    pub const fn hint_name(self) -> &'static str {
        match self {
            Self::TopLeft => "Top left corner",
            Self::TopRight => "Top right corner",
            Self::BottomLeft => "Bottom left corner",
            Self::BottomRight => "Bottom right corner",
            Self::TopCentre => "Top centre",
            Self::LeftMiddle => "Left middle",
            Self::RightMiddle => "Right middle",
            Self::BottomCentre => "Bottom centre",
            Self::Centre => "Centre",
        }
    }
}

/// The marks are drawn and clickable from a shorter side of this many screen pixels
/// (criterion 39); below it the Reference control is the way.
pub const MARKS_MIN_SIDE_PX: f64 = 32.0;
/// The radius of a mark's hit target, screen pixels (criterion 40).
const HIT_RADIUS_PX: f64 = 12.0;
/// The same for a touch or a pen.
const TOUCH_HIT_RADIUS_PX: f64 = 22.0;
/// The hit radius never exceeds this fraction of the shorter side, so that the nine
/// targets, which lie `s / 2` apart, never overlap.
const HIT_RADIUS_SIDE_FRACTION: f64 = 0.24;

/// The shorter side of `bounds` on a screen that shows `px_per_mm` pixels to a millimetre.
#[must_use]
pub fn shorter_side_px(bounds: (Point, Point), px_per_mm: f64) -> f64 {
    let (min, max) = bounds;
    (max.x - min.x).min(max.y - min.y) * px_per_mm
}

/// Whether the marks are shown for a box whose shorter side is `side_px` screen pixels.
#[must_use]
pub fn marks_are_shown(side_px: f64) -> bool {
    side_px >= MARKS_MIN_SIDE_PX
}

/// The hit radius of a mark of a box with shorter side `side_px`, in screen pixels:
/// `min(12, 0.24 s)`, or `min(22, 0.24 s)` for a touch.
#[must_use]
pub fn mark_hit_radius_px(side_px: f64, touch: bool) -> f64 {
    let base = if touch {
        TOUCH_HIT_RADIUS_PX
    } else {
        HIT_RADIUS_PX
    };
    base.min(HIT_RADIUS_SIDE_FRACTION * side_px)
}

/// The marks of the box `bounds` that are drawn and clickable: none unless the shorter side
/// is at least [`MARKS_MIN_SIDE_PX`], and only those inside the viewport, a screen
/// `viewport_px` wide and high; a mark outside it is not pulled in (criterion 39).
/// `to_screen` maps a document point to screen pixels.
#[must_use]
pub fn visible_marks(
    bounds: (Point, Point),
    px_per_mm: f64,
    viewport_px: (f64, f64),
    to_screen: impl Fn(Point) -> (f64, f64),
) -> Vec<ReferencePoint> {
    if !marks_are_shown(shorter_side_px(bounds, px_per_mm)) {
        return Vec::new();
    }
    ReferencePoint::ALL
        .into_iter()
        .filter(|mark| {
            let (x, y) = to_screen(mark.point(bounds));
            x >= 0.0 && y >= 0.0 && x <= viewport_px.0 && y <= viewport_px.1
        })
        .collect()
}

/// The mark a press at screen position `pointer_px` is on, if any: the nearest mark centre
/// within the hit radius of [`mark_hit_radius_px`], the centre winning an exact tie
/// (criterion 40). `marks` are the [`visible_marks`], `to_screen` as there.
#[must_use]
pub fn hit_mark(
    bounds: (Point, Point),
    marks: &[ReferencePoint],
    pointer_px: (f64, f64),
    px_per_mm: f64,
    touch: bool,
    to_screen: impl Fn(Point) -> (f64, f64),
) -> Option<ReferencePoint> {
    let radius = mark_hit_radius_px(shorter_side_px(bounds, px_per_mm), touch);
    let mut best: Option<(ReferencePoint, f64)> = None;
    // The centre is tried first, and only a strictly nearer mark replaces it.
    let centre_first = std::iter::once(ReferencePoint::Centre)
        .chain(ReferencePoint::ALL.into_iter().filter(|m| !m.is_centre()));
    for mark in centre_first.filter(|mark| marks.contains(mark)) {
        let (x, y) = to_screen(mark.point(bounds));
        let distance = (x - pointer_px.0).hypot(y - pointer_px.1);
        if distance <= radius && best.is_none_or(|(_, nearest)| distance < nearest) {
            best = Some((mark, distance));
        }
    }
    best.map(|(mark, _)| mark)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(x0: f64, y0: f64, x1: f64, y1: f64) -> (Point, Point) {
        (Point::new(x0, y0), Point::new(x1, y1))
    }

    /// One screen pixel is one millimetre.
    fn identity(point: Point) -> (f64, f64) {
        (point.x, point.y)
    }

    /// Criterion 37 and the rectangle of criterion 40: a 30 x 40 box at (10, 20) has the
    /// centre (25, 40) and the nine points on its corners, edge midpoints and centre.
    #[test]
    fn the_nine_points_lie_on_the_box() {
        let b = bounds(10.0, 20.0, 40.0, 60.0);
        let want = [
            (10.0, 20.0),
            (25.0, 20.0),
            (40.0, 20.0),
            (10.0, 40.0),
            (25.0, 40.0),
            (40.0, 40.0),
            (10.0, 60.0),
            (25.0, 60.0),
            (40.0, 60.0),
        ];
        for (mark, (x, y)) in ReferencePoint::ALL.iter().zip(want) {
            let at = mark.point(b);
            assert_eq!((at.x, at.y), (x, y), "{mark:?}");
        }
        assert_eq!(ReferencePoint::default(), ReferencePoint::Centre);
    }

    /// The order is the row-by-row order of the radios, with their names (criterion 41).
    #[test]
    fn the_order_and_the_names_are_those_of_the_radios() {
        let names: Vec<&str> = ReferencePoint::ALL.iter().map(|m| m.name()).collect();
        assert_eq!(
            names,
            [
                "Top left",
                "Top centre",
                "Top right",
                "Left middle",
                "Centre",
                "Right middle",
                "Bottom left",
                "Bottom centre",
                "Bottom right"
            ]
        );
        for (index, mark) in ReferencePoint::ALL.iter().enumerate() {
            assert_eq!(mark.index(), index);
            assert_eq!(ReferencePoint::from_index(index), Some(*mark));
        }
        assert_eq!(ReferencePoint::from_index(9), None);
        assert_eq!(ReferencePoint::TopLeft.hint_name(), "Top left corner");
        assert_eq!(ReferencePoint::TopCentre.hint_name(), "Top centre");
        assert_eq!(ReferencePoint::Centre.hint_name(), "Centre");
    }

    /// Criterion 39: shown from 32 px, the radius is `min(12, 0.24 s)` and `min(22, 0.24 s)`
    /// for a touch.
    #[test]
    fn the_marks_show_from_32_px_and_the_radius_shrinks_with_the_box() {
        assert!(!marks_are_shown(31.9));
        assert!(marks_are_shown(32.0));
        assert!((mark_hit_radius_px(32.0, false) - 7.68).abs() < 1e-9);
        assert_eq!(mark_hit_radius_px(50.0, false), 12.0);
        assert_eq!(mark_hit_radius_px(500.0, false), 12.0);
        assert!((mark_hit_radius_px(50.0, true) - 12.0).abs() < 1e-9);
        assert_eq!(mark_hit_radius_px(500.0, true), 22.0);
    }

    /// Criterion 40: the targets never overlap, at any size: two neighbouring marks are
    /// `s / 2` apart and the radius is at most `0.24 s`.
    #[test]
    fn no_two_targets_overlap() {
        for side in [32.0, 40.0, 50.0, 100.0, 5000.0] {
            for touch in [false, true] {
                assert!(2.0 * mark_hit_radius_px(side, touch) < side / 2.0 + 1e-9);
            }
        }
    }

    /// Criterion 50: a 2 mm square at 3.78 px per mm (about 7.6 px) has no marks.
    #[test]
    fn a_small_box_has_no_marks() {
        let b = bounds(0.0, 0.0, 2.0, 2.0);
        assert_eq!(visible_marks(b, 3.78, (1000.0, 800.0), identity), []);
    }

    /// Criterion 39: a mark outside the viewport is not drawn and not pulled in.
    #[test]
    fn marks_outside_the_viewport_are_dropped() {
        let b = bounds(-100.0, 100.0, 200.0, 400.0);
        let shown = visible_marks(b, 1.0, (500.0, 800.0), identity);
        assert!(!shown.contains(&ReferencePoint::TopLeft));
        assert!(!shown.contains(&ReferencePoint::LeftMiddle));
        assert!(!shown.contains(&ReferencePoint::BottomLeft));
        assert!(shown.contains(&ReferencePoint::Centre));
        assert!(shown.contains(&ReferencePoint::TopRight));
    }

    /// Criterion 40: the nearest mark within the radius, the centre cross winning a tie; a
    /// press beyond the radius of every mark is on none.
    #[test]
    fn a_press_is_on_the_nearest_mark_within_the_radius() {
        let b = bounds(0.0, 0.0, 100.0, 100.0);
        let marks = visible_marks(b, 1.0, (1000.0, 1000.0), identity);
        let hit = |x: f64, y: f64| hit_mark(b, &marks, (x, y), 1.0, false, identity);
        assert_eq!(hit(0.0, 0.0), Some(ReferencePoint::TopLeft));
        assert_eq!(hit(11.0, 3.0), Some(ReferencePoint::TopLeft));
        assert_eq!(hit(50.0, 50.0), Some(ReferencePoint::Centre));
        assert_eq!(hit(50.0, 100.0), Some(ReferencePoint::BottomCentre));
        assert_eq!(hit(100.0, 50.0), Some(ReferencePoint::RightMiddle));
        assert_eq!(hit(24.0, 24.0), None, "between the corner and the centre");
        assert_eq!(hit(13.0, 0.0), None, "13 px from the corner is outside 12");
        // A touch reaches 22 px, but never past 0.24 s = 24 px of this box.
        assert_eq!(
            hit_mark(b, &marks, (21.0, 0.0), 1.0, true, identity),
            Some(ReferencePoint::TopLeft)
        );
    }

    /// An exact tie goes to the centre. With the real geometry a tie cannot happen (two
    /// targets are `s / 2` apart and each radius is at most `0.24 s`), so the rule is tested
    /// with a mapping that puts the top centre and the centre on one screen point.
    #[test]
    fn the_centre_wins_an_exact_tie() {
        let b = bounds(0.0, 0.0, 10.0, 10.0);
        let marks = [ReferencePoint::TopCentre, ReferencePoint::Centre];
        let collapsed = |p: Point| (p.x * 10.0, 0.0);
        assert_eq!(
            hit_mark(b, &marks, (50.0, 0.0), 10.0, false, collapsed),
            Some(ReferencePoint::Centre)
        );
        let marks = [ReferencePoint::Centre, ReferencePoint::TopCentre];
        assert_eq!(
            hit_mark(b, &marks, (50.0, 0.0), 10.0, false, collapsed),
            Some(ReferencePoint::Centre),
            "whatever the order of the list"
        );
    }
}
