//! How closed outlines nest inside each other (`specs/0035-combine-and-break-apart`, "Depth";
//! `specs/0054-combine-open-paths`, "Contains" and "Depth"): an outline of even depth is a shape,
//! one of odd depth a hole in its parent. Open outlines have no depth and contain nothing.
//!
//! For two closed outlines that do not touch, one test point of the inner outline decides, by the
//! test of the canvas, the nonzero rule on the exact curves ([`contains_point_in_outlines`]): its
//! first node, or the next one when that node lies within [`TOUCH_DISTANCE_MM`] of the other
//! outline (a result of the boolean kernel may touch at a vertex); an outline all of whose nodes
//! touch the other counts as not enclosed by it.
//!
//! For two closed outlines that touch or cross, B is in A when every test point of B farther than
//! the touch distance from A lies inside A (the nonzero winding of A's polyline), at least one
//! such point exists, and no vertex of A farther than the touch distance from B lies inside B.
//! B's test points are its vertices and the midpoints of its chords cut where they cross A's
//! chords, so a concave A whose notch runs across B between B's vertices is caught. Coincident
//! outlines have no test point and do not contain each other. Only `+ - * /` and `sqrt` are used,
//! so the result is the same on every target. Bounding boxes reject nearly every pair of
//! side-by-side outlines.

use curvyo_document_core::Point;

use crate::outline_touch::{
    Flat, TOUCH_DISTANCE_MM, chords_near, point_segment_distance, sweep, touching_pairs,
};
use crate::{Outline, contains_point_in_outlines, signed_area_mm2};

/// Where one outline sits among the others.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nesting {
    /// How many other outlines enclose it: 0 is outermost, even depths are shapes, odd are holes.
    pub depth: usize,
    /// The innermost outline enclosing it, if any.
    pub parent: Option<usize>,
}

/// The nesting of `outlines`, one entry per outline in order. An open outline gets depth 0 and no
/// parent and encloses no other outline.
#[must_use]
pub fn outline_nesting(outlines: &[Outline<'_>]) -> Vec<Nesting> {
    let flats: Vec<Flat> = outlines.iter().map(Flat::of).collect();
    let touching: Vec<(usize, usize)> = touching_pairs(&flats)
        .into_iter()
        .filter(|(a, b)| a != b)
        .collect();
    let enclosers: Vec<Vec<usize>> = (0..outlines.len())
        .map(|inner| {
            (0..outlines.len())
                .filter(|&outer| {
                    outer != inner && encloses(outlines, &flats, &touching, outer, inner)
                })
                .collect()
        })
        .collect();
    let depths: Vec<usize> = enclosers.iter().map(Vec::len).collect();
    enclosers
        .iter()
        .map(|list| Nesting {
            depth: list.len(),
            parent: list.iter().copied().max_by_key(|&outer| depths[outer]),
        })
        .collect()
}

/// Whether `outer` encloses `inner`. `touching` holds the sorted pairs of different outlines that
/// touch or cross.
fn encloses(
    outlines: &[Outline<'_>],
    flats: &[Flat],
    touching: &[(usize, usize)],
    outer: usize,
    inner: usize,
) -> bool {
    let (outer_flat, inner_flat) = (&flats[outer], &flats[inner]);
    if !outer_flat.closed || !inner_flat.closed {
        return false;
    }
    // The inner box must lie within the outer one, up to the touch distance.
    if inner_flat.min.x < outer_flat.min.x - TOUCH_DISTANCE_MM
        || inner_flat.max.x > outer_flat.max.x + TOUCH_DISTANCE_MM
        || inner_flat.min.y < outer_flat.min.y - TOUCH_DISTANCE_MM
        || inner_flat.max.y > outer_flat.max.y + TOUCH_DISTANCE_MM
    {
        return false;
    }
    if touching
        .binary_search(&(outer.min(inner), outer.max(inner)))
        .is_ok()
    {
        return contains_touching(outer_flat, inner_flat);
    }
    for anchor in outlines[inner].anchors {
        let point = anchor.0;
        if distance_to_polyline(point, outer_flat) <= TOUCH_DISTANCE_MM {
            continue;
        }
        return contains_point_in_outlines(&[outlines[outer]], point);
    }
    false
}

/// "Contains" for two closed outlines that touch or cross (module documentation). The crossings
/// are found by the chord sweep of the touch test and every test point is judged in one scanline
/// pass, so the cost grows with `n log n` in the node counts, not `n · m`.
fn contains_touching(outer: &Flat, inner: &Flat) -> bool {
    let mut strictly_inside = false;
    let inner_inside = every_point(test_points(inner, outer), outer, |near, winding| {
        if near {
            return true;
        }
        strictly_inside = true;
        winding != 0
    });
    inner_inside
        && strictly_inside
        && every_point(outer.points.clone(), inner, |near, winding| {
            near || winding == 0
        })
}

/// The vertices of `flat` and the midpoints of its chords cut where they cross a chord of `other`.
fn test_points(flat: &Flat, other: &Flat) -> Vec<Point> {
    let mut cuts: Vec<(usize, f64)> = Vec::new();
    sweep(
        &chords_near(flat, other),
        &chords_near(other, flat),
        |own, foreign| {
            if let Some(t) = crossing_parameter(own.4, own.5, foreign.4, foreign.5) {
                cuts.push((own.6, t));
            }
            false
        },
    );
    cuts.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let mut points = flat.points.clone();
    let mut next = 0;
    for index in 0..flat.chord_count() {
        let (p, q) = flat.chord(index);
        let mut previous = 0.0;
        while next < cuts.len() && cuts[next].0 == index {
            let t = cuts[next].1;
            points.push(along_chord(p, q, f64::midpoint(previous, t)));
            previous = t;
            next += 1;
        }
        if previous > 0.0 {
            points.push(along_chord(p, q, f64::midpoint(previous, 1.0)));
        }
    }
    points
}

fn along_chord(p: Point, q: Point, t: f64) -> Point {
    Point::new(p.x + t * (q.x - p.x), p.y + t * (q.y - p.y))
}

/// Where the segment `p q` crosses the segment `a b`, as a parameter strictly between 0 and 1 on
/// `p q`; `None` for parallel segments or no crossing.
fn crossing_parameter(p: Point, q: Point, a: Point, b: Point) -> Option<f64> {
    let along = (q.x - p.x, q.y - p.y);
    let other = (b.x - a.x, b.y - a.y);
    let denominator = along.0 * other.1 - along.1 * other.0;
    if denominator == 0.0 {
        return None;
    }
    let offset = (a.x - p.x, a.y - p.y);
    let on_self = (offset.0 * other.1 - offset.1 * other.0) / denominator;
    let on_other = (offset.0 * along.1 - offset.1 * along.0) / denominator;
    (on_self > 0.0 && on_self < 1.0 && (0.0..=1.0).contains(&on_other)).then_some(on_self)
}

/// Whether `accept(near, winding)` holds for every point of `points`, stopping at the first that
/// fails: `near` is whether the point lies within [`TOUCH_DISTANCE_MM`] of the closed polyline of
/// `flat`, `winding` its winding number about the point. One scanline pass: the points sorted by
/// y, the chords by the bottom of their y range widened by the touch distance; a chord is active
/// while its widened range holds the point's y, which is every chord the point can be near and
/// every chord a horizontal ray from it can cross.
fn every_point(
    mut points: Vec<Point>,
    flat: &Flat,
    mut accept: impl FnMut(bool, i32) -> bool,
) -> bool {
    points.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
    let mut chords: Vec<(Point, Point, f64, f64)> = (0..flat.chord_count())
        .map(|index| {
            let (a, b) = flat.chord(index);
            (
                a,
                b,
                a.y.min(b.y) - TOUCH_DISTANCE_MM,
                a.y.max(b.y) + TOUCH_DISTANCE_MM,
            )
        })
        .collect();
    chords.sort_by(|a, b| a.2.total_cmp(&b.2));
    let mut active: Vec<usize> = Vec::new();
    let mut next = 0;
    for point in points {
        while next < chords.len() && chords[next].2 <= point.y {
            active.push(next);
            next += 1;
        }
        active.retain(|&index| chords[index].3 >= point.y);
        let mut near = flat.chord_count() == 0
            && flat.points.first().is_some_and(|&only| {
                point_segment_distance(point, only, only) <= TOUCH_DISTANCE_MM
            });
        let mut winding = 0;
        for &index in &active {
            let (a, b, _, _) = chords[index];
            near = near || point_segment_distance(point, a, b) <= TOUCH_DISTANCE_MM;
            winding += crossing(a, b, point);
        }
        if !accept(near, winding) {
            return false;
        }
    }
    true
}

/// What the chord `a b` adds to the winding number about `point` (a ray to the right).
fn crossing(a: Point, b: Point, point: Point) -> i32 {
    let side = (b.x - a.x) * (point.y - a.y) - (point.x - a.x) * (b.y - a.y);
    if a.y <= point.y {
        i32::from(b.y > point.y && side > 0.0)
    } else {
        -i32::from(b.y <= point.y && side < 0.0)
    }
}

/// The distance from `point` to the polyline of `flat`.
fn distance_to_polyline(point: Point, flat: &Flat) -> f64 {
    if flat.chord_count() == 0 {
        return flat.points.first().map_or(f64::INFINITY, |&only| {
            point_segment_distance(point, only, only)
        });
    }
    (0..flat.chord_count())
        .map(|index| {
            let (a, b) = flat.chord(index);
            point_segment_distance(point, a, b)
        })
        .fold(f64::INFINITY, f64::min)
}

/// The signed area of `outline`, positive for the winding of a shape in the canonical form of the
/// boolean kernel and negative for a hole, computed on the flattened outline (an open outline
/// closed by its straight closing line). An outline whose
/// absolute area is not above 1e-6 mm² (one grid cell) encloses no area.
#[must_use]
pub fn outline_area_mm2(outline: &Outline<'_>) -> f64 {
    signed_area_mm2(&Flat::of(outline).points)
}

/// Whether `outline` encloses any area at all: the areas of the triangles of a fan from its first
/// point, taken without sign, add up to more than 1e-6 mm² (one grid cell). An open outline is
/// read closed by its straight closing line, so a straight or zero-length one has no area. Unlike
/// [`outline_area_mm2`] this is not fooled by a figure eight whose two lobes cancel.
#[must_use]
pub fn outline_has_area(outline: &Outline<'_>) -> bool {
    let points = Flat::of(outline).points;
    let Some(&first) = points.first() else {
        return false;
    };
    let doubled: f64 = points
        .windows(2)
        .map(|pair| {
            let (a, b) = (pair[0], pair[1]);
            ((a.x - first.x) * (b.y - first.y) - (b.x - first.x) * (a.y - first.y)).abs()
        })
        .sum();
    doubled / 2.0 > 1e-6
}
