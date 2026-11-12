//! Clips traced outlines to the mask region of a masked trace (`specs/0094-masked-composite-trace`
//! slice 1, `adrs.md` decision 1).
//!
//! The region is the kernel union of the mask paths in document millimetres. A traced outline
//! whose control-point box meets no edge of the region lies wholly inside or outside it; one point
//! test decides whether it stays, curves intact. The outlines that do meet an edge go through the
//! boolean kernel together as one operand (Intersection for Inside, Difference for Outside) and
//! come back as polylines. Only `+ - * /` are used outside the kernel, so the result is the same
//! on every target.

use curvyo_document_core::{Point, Tolerance, Vec2};
use curvyo_geometry_core::signed_area_mm2;
use curvyo_geometry_core::{BooleanError, BooleanOp, Outline, OutlineTriple, boolean};
use kurbo::{BezPath, PathEl, Shape};

use vectrace::mask::{Clipped, Mask};

/// The kernel tolerance of the region and of the clip (`adrs.md` decision 1): the grid keeps
/// 0.002 mm, curves are flattened at 0.003 mm, half of criterion 2's 0.01 mm is left.
pub(crate) const TRACE_REGION_TOLERANCE: Tolerance = Tolerance::from_mm(0.005);

/// The area a masked trace keeps: the mask paths' union, or everything outside it.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceRegion {
    /// The union's closed polygons in document millimetres (outer outlines and holes).
    polygons: Vec<Vec<Point>>,
    /// Outside: the trace keeps what lies outside the polygons.
    outside: bool,
    /// The polygons' box.
    min: Point,
    max: Point,
}

/// Why the mask paths make no region.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TraceRegionError {
    /// A mask contains an editable curve that cannot be evaluated.
    #[error("a mask curve cannot be evaluated")]
    UnsupportedCurve,
    /// No mask path encloses an area: the trace has nothing to keep.
    #[error("no mask path encloses an area")]
    NoArea,
    /// The kernel refused the union for another reason.
    #[error("the mask paths could not be combined: {0}")]
    Kernel(BooleanError),
}

/// The operands of a region: per mask path its outlines as the kernel reads them (anchors and
/// whether the outline is closed).
pub type RegionOperand = Vec<(Vec<OutlineTriple>, bool)>;

/// The outlines of a mask path as the region's union reads them: the collector of the Boolean
/// commands (a path's outlines, a clone's resolved ones, a primitive's rotated outline; text,
/// images, sketches and dimensions have none).
/// # Errors
/// Returns an error when an editable mask curve cannot be evaluated.
pub fn mask_operand(
    object: &curvyo_document_core::ObjectSnapshot,
) -> Result<RegionOperand, TraceRegionError> {
    if object
        .drawn_path()
        .is_some_and(|path| !path.curve_kind.is_bezier())
    {
        let evaluated =
            crate::evaluate_object(object).map_err(|_| TraceRegionError::UnsupportedCurve)?;
        Ok(crate::boolean::outlines_of(&evaluated))
    } else {
        Ok(crate::boolean::outlines_of(object))
    }
}

impl TraceRegion {
    /// The union of `operands` (one per mask path, in document z-order), Inside or `outside`.
    /// An operand without area (a straight open path) is dropped and the union is made of the
    /// others.
    ///
    /// # Errors
    /// [`TraceRegionError::NoArea`] when no operand encloses an area; [`TraceRegionError::Kernel`]
    /// when the kernel refuses for another reason.
    pub fn of_operands(
        operands: &[RegionOperand],
        outside: bool,
    ) -> Result<Self, TraceRegionError> {
        let mut kept: Vec<&RegionOperand> = operands.iter().collect();
        while !kept.is_empty() {
            let owned: Vec<Vec<Outline<'_>>> = kept
                .iter()
                .map(|operand| {
                    operand
                        .iter()
                        .map(|(anchors, closed)| Outline::new(anchors, *closed))
                        .collect()
                })
                .collect();
            let refs: Vec<&[Outline<'_>]> = owned.iter().map(Vec::as_slice).collect();
            match boolean(BooleanOp::Union, &refs, TRACE_REGION_TOLERANCE) {
                Ok(result) => {
                    return Self::from_polygons(result.into_outlines(), outside)
                        .ok_or(TraceRegionError::NoArea);
                }
                Err(BooleanError::EmptyOperands(bad) | BooleanError::OutOfRange(bad)) => {
                    kept = kept
                        .into_iter()
                        .enumerate()
                        .filter(|(i, _)| !bad.contains(i))
                        .map(|(_, operand)| operand)
                        .collect();
                }
                Err(BooleanError::EmptyResult) => return Err(TraceRegionError::NoArea),
                Err(error) => return Err(TraceRegionError::Kernel(error)),
            }
        }
        Err(TraceRegionError::NoArea)
    }

    /// The union of `added` minus the union of `subtracted` (one operand per region, in stored
    /// order; ADR 0061 §2), Inside or `outside`. An operand without area is dropped and the call
    /// made of the others, as [`Self::of_operands`] does.
    ///
    /// # Errors
    /// [`TraceRegionError::NoArea`] when nothing is left; [`TraceRegionError::Kernel`] when the
    /// kernel refuses for another reason.
    pub fn of_signed_operands(
        added: &[RegionOperand],
        subtracted: &[RegionOperand],
        outside: bool,
    ) -> Result<Self, TraceRegionError> {
        let union = Self::of_operands(added, false)?;
        if subtracted.is_empty() {
            return Ok(union.with_outside(outside));
        }
        let corner = |p: &Point| -> OutlineTriple { (*p, Vec2::ZERO, Vec2::ZERO) };
        let base: Vec<Vec<OutlineTriple>> = union
            .polygons
            .iter()
            .map(|p| p.iter().map(corner).collect())
            .collect();
        let base: Vec<Outline<'_>> = base.iter().map(|p| Outline::new(p, true)).collect();
        let mut kept: Vec<&RegionOperand> = subtracted.iter().collect();
        loop {
            let owned: Vec<Vec<Outline<'_>>> = kept
                .iter()
                .map(|operand| {
                    operand
                        .iter()
                        .map(|(anchors, closed)| Outline::new(anchors, *closed))
                        .collect()
                })
                .collect();
            let mut refs: Vec<&[Outline<'_>]> = vec![&base];
            refs.extend(owned.iter().map(Vec::as_slice));
            if kept.is_empty() {
                return Ok(union.with_outside(outside));
            }
            match boolean(BooleanOp::Difference, &refs, TRACE_REGION_TOLERANCE) {
                Ok(result) => {
                    return Self::from_polygons(result.into_outlines(), outside)
                        .ok_or(TraceRegionError::NoArea);
                }
                Err(BooleanError::EmptyOperands(bad) | BooleanError::OutOfRange(bad))
                    if !bad.contains(&0) =>
                {
                    kept = kept
                        .into_iter()
                        .enumerate()
                        .filter(|(i, _)| !bad.contains(&(i + 1)))
                        .map(|(_, operand)| operand)
                        .collect();
                }
                Err(BooleanError::EmptyResult) => return Err(TraceRegionError::NoArea),
                Err(error) => return Err(TraceRegionError::Kernel(error)),
            }
        }
    }

    /// A region of closed `polygons` (document millimetres; nonzero rule); `None` when none has
    /// three points or a coordinate is not finite.
    #[must_use]
    pub fn from_polygons(polygons: Vec<Vec<Point>>, outside: bool) -> Option<Self> {
        let polygons: Vec<Vec<Point>> = polygons.into_iter().filter(|p| p.len() >= 3).collect();
        let points = || polygons.iter().flatten();
        if polygons.is_empty() || !points().all(|p| p.x.is_finite() && p.y.is_finite()) {
            return None;
        }
        let min = points().fold(Point::new(f64::INFINITY, f64::INFINITY), |m, p| {
            Point::new(m.x.min(p.x), m.y.min(p.y))
        });
        let max = points().fold(Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY), |m, p| {
            Point::new(m.x.max(p.x), m.y.max(p.y))
        });
        Some(Self {
            polygons,
            outside,
            min,
            max,
        })
    }

    /// The region's polygons in document millimetres.
    #[must_use]
    pub fn polygons(&self) -> &[Vec<Point>] {
        &self.polygons
    }

    /// Whether the trace keeps what lies outside the polygons.
    #[must_use]
    pub const fn outside(&self) -> bool {
        self.outside
    }

    /// The same polygons with the other mode.
    #[must_use]
    pub fn with_outside(mut self, outside: bool) -> Self {
        self.outside = outside;
        self
    }

    /// The polygons as flat numbers for the worker: the polygon count, then per polygon its
    /// point count and the coordinates.
    #[must_use]
    pub fn to_numbers(&self) -> Vec<f64> {
        let mut numbers = Vec::with_capacity(1 + self.polygons.len() + 2 * self.points());
        #[allow(clippy::cast_precision_loss)] // counts far below 2^52
        {
            numbers.push(self.polygons.len() as f64);
            for polygon in &self.polygons {
                numbers.push(polygon.len() as f64);
                numbers.extend(polygon.iter().flat_map(|p| [p.x, p.y]));
            }
        }
        numbers
    }

    /// The region of [`Self::to_numbers`]; `None` unless they are well formed.
    #[must_use]
    pub fn from_numbers(numbers: &[f64], outside: bool) -> Option<Self> {
        let mut at = 0;
        let mut next = || -> Option<f64> {
            let value = *numbers.get(at)?;
            at += 1;
            Some(value)
        };
        let polygons = count(next()?)?;
        let mut read = Vec::with_capacity(polygons.min(numbers.len()));
        for _ in 0..polygons {
            let n = count(next()?)?;
            let mut polygon = Vec::with_capacity(n.min(numbers.len()));
            for _ in 0..n {
                polygon.push(Point::new(next()?, next()?));
            }
            read.push(polygon);
        }
        (at == numbers.len())
            .then(|| Self::from_polygons(read, outside))
            .flatten()
    }

    fn points(&self) -> usize {
        self.polygons.iter().map(Vec::len).sum()
    }

    /// Whether `p` lies in the polygons (nonzero winding).
    #[must_use]
    pub(crate) fn contains(&self, p: Point) -> bool {
        if p.x < self.min.x || p.x > self.max.x || p.y < self.min.y || p.y > self.max.y {
            return false;
        }
        let mut winding = 0_i32;
        for polygon in &self.polygons {
            let mut a = polygon[polygon.len() - 1];
            for &b in polygon {
                if a.y <= p.y {
                    if b.y > p.y && cross(a, b, p) > 0.0 {
                        winding += 1;
                    }
                } else if b.y <= p.y && cross(a, b, p) < 0.0 {
                    winding -= 1;
                }
                a = b;
            }
        }
        winding != 0
    }

    /// Whether the trace keeps the point `p`.
    #[must_use]
    pub fn keeps(&self, p: Point) -> bool {
        self.contains(p) != self.outside
    }

    /// The closed `quad` minus the polygons and the `quad` within them, as polygons (the
    /// Overlay's washes of Inside and Outside, UX notes §2); an empty part has none.
    #[must_use]
    pub fn quad_parts(&self, quad: &[Point; 4]) -> [Vec<Vec<Point>>; 2] {
        let corner = |p: &Point| -> OutlineTriple { (*p, Vec2::ZERO, Vec2::ZERO) };
        let quad: Vec<OutlineTriple> = quad.iter().map(corner).collect();
        let polygons: Vec<Vec<OutlineTriple>> = self
            .polygons
            .iter()
            .map(|p| p.iter().map(corner).collect())
            .collect();
        let quad_operand = [Outline::new(&quad, true)];
        let region: Vec<Outline<'_>> = polygons.iter().map(|p| Outline::new(p, true)).collect();
        [BooleanOp::Difference, BooleanOp::Intersection].map(|op| {
            boolean(op, &[&quad_operand, &region], TRACE_REGION_TOLERANCE)
                .map(curvyo_geometry_core::BooleanResult::into_outlines)
                .unwrap_or_default()
        })
    }

    /// Whether any edge of the polygons meets the box `min`..`max` (closed).
    pub(crate) fn edge_meets(&self, min: Point, max: Point) -> bool {
        if max.x < self.min.x || min.x > self.max.x || max.y < self.min.y || min.y > self.max.y {
            return false;
        }
        self.polygons.iter().any(|polygon| {
            let mut a = polygon[polygon.len() - 1];
            polygon.iter().any(|&b| {
                let meets = segment_meets_box(a, b, min, max);
                a = b;
                meets
            })
        })
    }

    /// The crossing `outlines` as one operand clipped by the kernel, as polylines whose outer
    /// outlines have the kurbo area sign `sign`, and how many were kept whole unclipped after a
    /// kernel refusal.
    fn clip_crossing(&self, outlines: &[&BezPath], sign: f64) -> (Vec<BezPath>, usize) {
        let subject: Vec<Vec<OutlineTriple>> =
            outlines.iter().flat_map(|path| triples_of(path)).collect();
        let subject: Vec<Outline<'_>> = subject.iter().map(|a| Outline::new(a, true)).collect();
        let corners: Vec<Vec<OutlineTriple>> = self
            .polygons
            .iter()
            .map(|p| p.iter().map(|&q| (q, Vec2::ZERO, Vec2::ZERO)).collect())
            .collect();
        let region: Vec<Outline<'_>> = corners.iter().map(|p| Outline::new(p, true)).collect();
        let op = if self.outside {
            BooleanOp::Difference
        } else {
            BooleanOp::Intersection
        };
        let result = match boolean(op, &[&subject, &region], TRACE_REGION_TOLERANCE) {
            Ok(result) => result,
            // Nothing of the crossing outlines is kept, or they enclose no area.
            Err(BooleanError::EmptyResult | BooleanError::EmptyOperands(_)) => {
                return (Vec::new(), 0);
            }
            // invariant: traced outlines are finite and the tolerance is a constant above the
            // grid; should the kernel refuse anyway, the crossing outlines are not lost but kept
            // whole when the region keeps their first point (no exact stencil edge for them).
            Err(_) => {
                let whole: Vec<BezPath> = outlines
                    .iter()
                    .filter(|path| first_point(path).is_some_and(|p| self.keeps(p)))
                    .map(|path| (*path).clone())
                    .collect();
                let count = whole.len();
                return (whole, count);
            }
        };
        (wound_polylines(result.outlines(), sign), 0)
    }
}

/// `outlines` (closed, document millimetres, holes wound against their outer outline) clipped to
/// the kept area. The outlines that stay whole keep their order and curves; the clipped ones come
/// back as polylines in the place of the first outline that crossed, wound as the trace winds its
/// outlines.
impl Mask for TraceRegion {
    fn keeps(&self, p: kurbo::Point) -> bool {
        Self::keeps(self, Point::new(p.x, p.y))
    }

    fn clip(&self, outlines: &[BezPath]) -> Clipped {
        let mut kept = Vec::with_capacity(outlines.len());
        let mut crossing: Vec<&BezPath> = Vec::new();
        let mut crossing_at = None;
        for outline in outlines {
            let b = outline.control_box();
            let (min, max) = (Point::new(b.x0, b.y0), Point::new(b.x1, b.y1));
            if self.edge_meets(min, max) {
                crossing_at.get_or_insert(kept.len());
                crossing.push(outline);
                continue;
            }
            if first_point(outline).is_some_and(|p| Self::keeps(self, p)) {
                kept.push(outline.clone());
            }
        }
        let Some(at) = crossing_at else {
            let changed = kept.len() != outlines.len();
            return Clipped {
                outlines: kept,
                changed,
                kept_whole: 0,
            };
        };
        let (pieces, kept_whole) = self.clip_crossing(&crossing, outer_sign(outlines));
        kept.splice(at..at, pieces);
        Clipped {
            outlines: kept,
            changed: true,
            kept_whole,
        }
    }
}

/// The muted notice of outlines kept whole across the stencil edge (`specs/0094`, tester
/// 2026-11-04).
#[must_use]
pub fn unclipped_notice(count: usize) -> String {
    match count {
        0 => String::new(),
        1 => "1 shape crosses the mask edge unclipped.".to_owned(),
        n => format!("{n} shapes cross the mask edge unclipped."),
    }
}

/// The z component of `(b - a) x (p - a)`.
/// A count read from the region's numbers: a whole, non-negative, finite number.
fn count(value: f64) -> Option<usize> {
    // invariant: checked whole and in range before the cast.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    (value.is_finite() && value >= 0.0 && value.fract() == 0.0 && value < 9.0e15)
        .then_some(value as usize)
}

fn cross(a: Point, b: Point, p: Point) -> f64 {
    (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y)
}

/// Whether the segment `a`..`b` meets the closed box `min`..`max` (separating axes: the box's
/// two and the segment's normal).
fn segment_meets_box(a: Point, b: Point, min: Point, max: Point) -> bool {
    if a.x.max(b.x) < min.x || a.x.min(b.x) > max.x || a.y.max(b.y) < min.y || a.y.min(b.y) > max.y
    {
        return false;
    }
    let sides = [
        cross(a, b, min),
        cross(a, b, Point::new(max.x, min.y)),
        cross(a, b, max),
        cross(a, b, Point::new(min.x, max.y)),
    ];
    !(sides.iter().all(|&s| s > 0.0) || sides.iter().all(|&s| s < 0.0))
}

/// The first point of `path`.
fn first_point(path: &BezPath) -> Option<Point> {
    path.elements().iter().find_map(|element| match *element {
        PathEl::MoveTo(p) => Some(Point::new(p.x, p.y)),
        _ => None,
    })
}

/// The kernel's `polygons` as polyline paths whose outer outlines (positive kernel area) have
/// the kurbo area sign `sign` and whose holes have the other one, as the trace winds them.
pub(crate) fn wound_polylines(polygons: &[Vec<Point>], sign: f64) -> Vec<BezPath> {
    polygons
        .iter()
        .map(|polygon| {
            let mut path = polyline(polygon);
            let wanted = if signed_area_mm2(polygon) > 0.0 {
                sign
            } else {
                -sign
            };
            if path.area() * wanted < 0.0 {
                path = path.reverse_subpaths();
            }
            path
        })
        .collect()
}

/// The kurbo area sign of the trace's outer outlines: the sign of the largest outline, which no
/// hole can be (a hole lies inside its outer outline).
pub(crate) fn outer_sign(outlines: &[BezPath]) -> f64 {
    let largest =
        outlines.iter().map(Shape::area).fold(
            0.0_f64,
            |best, a| if a.abs() > best.abs() { a } else { best },
        );
    if largest < 0.0 { -1.0 } else { 1.0 }
}

/// The closed polygon `points` as a polyline path (its closing segment implied).
fn polyline(points: &[Point]) -> BezPath {
    let mut path = BezPath::new();
    for (i, p) in points.iter().enumerate() {
        let q = kurbo::Point::new(p.x, p.y);
        if i == 0 {
            path.move_to(q);
        } else {
            path.line_to(q);
        }
    }
    path.close_path();
    path
}

/// The closed subpaths of `path` as the kernel reads them: anchors with handles relative to
/// their point; a last segment that ends on the first anchor closes the outline.
pub(crate) fn triples_of(path: &BezPath) -> Vec<Vec<OutlineTriple>> {
    let mut outlines: Vec<Vec<OutlineTriple>> = Vec::new();
    let point = |p: kurbo::Point| Point::new(p.x, p.y);
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(p) => outlines.push(vec![(point(p), Vec2::ZERO, Vec2::ZERO)]),
            PathEl::LineTo(p) | PathEl::QuadTo(_, p) => {
                if let Some(anchors) = outlines.last_mut() {
                    anchors.push((point(p), Vec2::ZERO, Vec2::ZERO));
                }
            }
            PathEl::CurveTo(c1, c2, p) => {
                if let Some(anchors) = outlines.last_mut()
                    && let Some(last) = anchors.last_mut()
                {
                    last.2 = Vec2::new(c1.x - last.0.x, c1.y - last.0.y);
                    anchors.push((point(p), Vec2::new(c2.x - p.x, c2.y - p.y), Vec2::ZERO));
                }
            }
            PathEl::ClosePath => {}
        }
    }
    for anchors in &mut outlines {
        if anchors.len() > 2 {
            let (first, last) = (anchors[0], anchors[anchors.len() - 1]);
            if (first.0.x - last.0.x).abs() < 1e-12 && (first.0.y - last.0.y).abs() < 1e-12 {
                anchors[0].1 = last.1;
                anchors.pop();
            }
        }
    }
    outlines
}

#[cfg(test)]
mod tests {
    use super::*;
    use vectrace::node_count;

    /// The mask's clip as the outlines and whether anything changed.
    fn clip(region: &TraceRegion, outlines: &[BezPath]) -> (Vec<BezPath>, bool) {
        let clipped = Mask::clip(region, outlines);
        (clipped.outlines, clipped.changed)
    }

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<OutlineTriple> {
        [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
            .into_iter()
            .map(|(x, y)| (Point::new(x, y), Vec2::ZERO, Vec2::ZERO))
            .collect()
    }

    fn square_path(x0: f64, y0: f64, x1: f64, y1: f64) -> BezPath {
        let mut p = BezPath::new();
        p.move_to((x0, y0));
        p.line_to((x1, y0));
        p.line_to((x1, y1));
        p.line_to((x0, y1));
        p.close_path();
        p
    }

    #[test]
    fn the_numbers_round_trip_and_bad_ones_are_refused() {
        let region =
            TraceRegion::of_operands(&[vec![(rect(0.0, 0.0, 10.0, 5.0), true)]], false).unwrap();
        let numbers = region.to_numbers();
        assert_eq!(TraceRegion::from_numbers(&numbers, false), Some(region));
        assert_eq!(
            TraceRegion::from_numbers(&numbers[..numbers.len() - 1], false),
            None
        );
        assert_eq!(
            TraceRegion::from_numbers(&[1.0, 2.0, 0.0, 0.0, 1.0, 1.0], false),
            None
        );
        assert_eq!(TraceRegion::from_numbers(&[], false), None);
    }

    #[test]
    fn whole_outlines_stay_or_go_and_crossing_ones_are_cut() {
        let region =
            TraceRegion::of_operands(&[vec![(rect(0.0, 0.0, 10.0, 10.0), true)]], false).unwrap();
        let inside = square_path(1.0, 1.0, 2.0, 2.0);
        let outside = square_path(20.0, 1.0, 22.0, 2.0);
        let crossing = square_path(8.0, 1.0, 12.0, 3.0);
        let (clipped, changed) = clip(&region, &[inside.clone(), outside.clone()]);
        assert!(changed);
        assert_eq!(clipped, std::slice::from_ref(&inside));
        let (clipped, _) = clip(&region, &[inside.clone(), crossing.clone()]);
        assert_eq!(clipped.len(), 2);
        assert_eq!(clipped[0], inside);
        assert!((clipped[1].area().abs() - 4.0).abs() < 1e-6);
        // The trace's outer winding is kept.
        assert!(clipped[1].area() * inside.area() > 0.0);
        let outside_region = region.with_outside(true);
        let (clipped, _) = clip(&outside_region, &[inside, outside.clone(), crossing]);
        assert_eq!(clipped[0], outside);
        assert!((clipped[1].area().abs() - 4.0).abs() < 1e-6);
    }

    /// Architect review 2026-11-04: a kernel refusal does not drop the crossing outlines; they
    /// stay whole when the region keeps their first point.
    #[test]
    fn a_kernel_refusal_keeps_crossing_outlines_by_their_first_point() {
        let region =
            TraceRegion::of_operands(&[vec![(rect(0.0, 0.0, 10.0, 10.0), true)]], false).unwrap();
        // Beyond the kernel's 10^7 mm: refused as out of range.
        let far = square_path(5.0, 1.0, 2.0e7, 3.0);
        let (clipped, changed) = clip(&region, std::slice::from_ref(&far));
        assert!(changed);
        assert_eq!(clipped, std::slice::from_ref(&far));
        let (clipped, _) = clip(&region.with_outside(true), &[far]);
        assert_eq!(clipped, []);
    }

    #[test]
    fn a_region_reports_why_it_is_not_one() {
        let straight = vec![(rect(0.0, 0.0, 10.0, 0.0), false)];
        assert_eq!(
            TraceRegion::of_operands(&[straight], false),
            Err(TraceRegionError::NoArea)
        );
        assert_eq!(
            TraceRegion::of_operands(&[], false),
            Err(TraceRegionError::NoArea)
        );
    }

    #[test]
    fn the_node_count_is_the_tracers() {
        assert_eq!(node_count(&square_path(0.0, 0.0, 1.0, 1.0)), 4);
        let mut closed_back = square_path(0.0, 0.0, 1.0, 1.0);
        closed_back.truncate(4);
        closed_back.line_to((0.0, 0.0));
        closed_back.close_path();
        assert_eq!(node_count(&closed_back), 4);
    }
}
