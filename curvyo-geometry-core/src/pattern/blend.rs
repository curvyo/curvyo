//! Interpolation waves (`specs/0071-pattern-fill` criterion 12, customer 2026-11-02: like
//! Inkscape's Interpolate): the region's top and bottom boundary, each resampled by arc length to
//! the same number of points, and the lines blending point by point between them.

use curvyo_document_core::{PatternCount, Point, Vec2};

use super::Raw;
use super::region::Region;
use crate::boolean_grid::{Path64, to_millimetres};

/// Points per line along the longer boundary run, at least every this many mm.
const SAMPLE_MM: f64 = 0.5;
/// The most points per line.
const MAX_SAMPLES: usize = 2000;

/// The two runs of `contour` between its left and right side, both left to right: the top run
/// from the highest leftmost point to the highest rightmost one over the top, the bottom run from
/// the lowest leftmost point to the lowest rightmost one under the bottom (a square gives its top
/// and bottom edges).
fn runs(contour: &Path64) -> (Vec<Point>, Vec<Point>) {
    let n = contour.len();
    let (min_x, max_x) = (
        contour.iter().map(|p| p.x).min().unwrap_or(0),
        contour.iter().map(|p| p.x).max().unwrap_or(0),
    );
    let side = |x: i64, highest: bool| {
        let on_side = (0..n).filter(|&i| contour[i].x == x);
        if highest {
            on_side.min_by_key(|&i| contour[i].y).unwrap_or(0)
        } else {
            on_side.max_by_key(|&i| contour[i].y).unwrap_or(0)
        }
    };
    let walk = |from: usize, to: usize, step: usize| {
        let mut run = vec![to_millimetres(&contour[from])];
        let mut i = from;
        while i != to {
            i = (i + step) % n;
            run.push(to_millimetres(&contour[i]));
        }
        run
    };
    #[allow(clippy::cast_precision_loss)]
    let mean = |run: &[Point]| run.iter().map(|p| p.y).sum::<f64>() / run.len() as f64;
    let pick = |from: usize, to: usize, top: bool| {
        let (a, b) = (walk(from, to, 1), walk(from, to, n - 1));
        if (mean(&a) <= mean(&b)) == top { a } else { b }
    };
    (
        pick(side(min_x, true), side(max_x, true), true),
        pick(side(min_x, false), side(max_x, false), false),
    )
}

/// The distance from `a` to `b` by `sqrt` of a sum of squares (the same bits everywhere).
fn distance(a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    (dx * dx + dy * dy).sqrt()
}

fn length(run: &[Point]) -> f64 {
    run.windows(2).map(|w| distance(w[0], w[1])).sum()
}

/// `run` resampled to `count` points evenly spaced by arc length, ends kept.
fn resample(run: &[Point], count: usize) -> Vec<Point> {
    let total = length(run);
    let mut out = Vec::with_capacity(count);
    let mut segment = 0;
    let mut walked = 0.0;
    for k in 0..count {
        #[allow(clippy::cast_precision_loss)]
        let target = total * k as f64 / (count - 1) as f64;
        while segment + 2 < run.len() {
            let (a, b) = (run[segment], run[segment + 1]);
            let piece = distance(a, b);
            if walked + piece >= target {
                break;
            }
            walked += piece;
            segment += 1;
        }
        let (a, b) = (run[segment], run[(segment + 1).min(run.len() - 1)]);
        let piece = distance(a, b);
        let t = if piece > 0.0 {
            ((target - walked) / piece).clamp(0.0, 1.0)
        } else {
            0.0
        };
        out.push(Point::new(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y)));
    }
    out
}

/// Points per blended line for runs `top` and `bottom`.
fn sample_count(top: &[Point], bottom: &[Point]) -> usize {
    let longest = length(top).max(length(bottom));
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let samples = ((longest / SAMPLE_MM).ceil() as usize + 1).clamp(2, MAX_SAMPLES);
    samples
}

/// Points per blended line over `region` (the cost estimate's count).
pub(crate) fn samples(region: &Region) -> usize {
    region.largest_outer().map_or(0, |contour| {
        let (top, bottom) = runs(contour);
        sample_count(&top, &bottom).max(top.len()).max(bottom.len())
    })
}

/// The blended lines, the first on the top boundary and the last on the bottom one.
pub(crate) fn interpolation_waves(region: &Region, lines: PatternCount) -> Vec<Raw> {
    let Some(contour) = region.largest_outer() else {
        return Vec::new();
    };
    let (top_run, bottom_run) = runs(contour);
    if top_run.len() < 2 || bottom_run.len() < 2 {
        return Vec::new();
    }
    let samples = sample_count(&top_run, &bottom_run);
    let (top, bottom) = (resample(&top_run, samples), resample(&bottom_run, samples));
    let n = lines.get().max(2);
    let along = |run: &[Point]| run.iter().map(|&p| (p, Vec2::ZERO, Vec2::ZERO)).collect();
    // The first and last lines are the boundary runs themselves, vertex for vertex.
    let (first, last) = (
        Raw::boundary(along(&top_run)),
        Raw::boundary(along(&bottom_run)),
    );
    let inner = (1..n - 1).map(|k| {
        let t = f64::from(k) / f64::from(n - 1);
        let anchors = top
            .iter()
            .zip(&bottom)
            .map(|(a, b)| {
                let p = Point::new(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y));
                (p, Vec2::ZERO, Vec2::ZERO)
            })
            .collect();
        Raw::open(anchors)
    });
    std::iter::once(first)
        .chain(inner)
        .chain(std::iter::once(last))
        .collect()
}
