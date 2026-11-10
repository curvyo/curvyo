//! Finds the structural corners of a closed crack chain before any smoothing (`specs/0089`
//! adrs.md §5, trial D1 candidate (b)): the multi-scale turn angle picks the corner vertices, the
//! crossing of the two adjacent total-least-squares lines places them.
//!
//! The turn at vertex `k` and scale `s` is the angle between the directions of the `s` edge
//! midpoints on either side, the apex left out (every crack edge is 1 px, so arc distance is an
//! index distance). A vertex is a
//! candidate when it turns more than the Corners angle at two of the three scales, the coarsest
//! among them (at all of them when the chain is too short for two); candidates are kept by the chord angle at their
//! vertex on the finest scale, sharpest first, more than one scale unit apart, and a run of
//! close corners whose outer sides cross sharply becomes one (an acute tip's blunt end). Basic operations and `sqrt` only.

use kurbo::{Point, Vec2};

use crate::corner_place::place;
use crate::line::fit_line;

/// The three scales in source pixels; scaled by the traced pixels per source pixel.
const SCALES: [usize; 3] = [2, 4, 8];

/// The largest scale in traced pixels (8 source px after Upscale).
const MAX_SCALE: usize = 16;

/// At a corner `1 − cos` of the turn at the next finer scale is at least this share of the
/// coarsest scale's.
const STEADY: f64 = 0.5;

/// A structural corner of a chain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Corner {
    /// The index of its vertex in the chain.
    pub vertex: usize,
    /// Its position: the crossing of the side lines, or the vertex.
    pub at: Point,
    /// The arc length on each side where the chain does not follow the sides.
    pub zone: f64,
    /// The unit directions of the straight sides ahead of and behind the corner, each pointing
    /// away from it (`None`: the side is not straight).
    pub ahead: Option<Vec2>,
    pub behind: Option<Vec2>,
}

/// The scales usable on a chain of `n` edges with `unit` traced pixels per source pixel.
fn scales(n: usize, unit: usize) -> Vec<usize> {
    SCALES
        .iter()
        .map(|s| s * unit)
        .filter(|&s| 4 * s <= n)
        .collect()
}

/// The cosine of the turn at `k` on scale `s`: between the total-least-squares directions of
/// the `s` edge midpoints behind and ahead of it, from 1.5 px on (the digitised apex of a
/// corner is left out); 1 when a side has no direction.
fn turn_cos(chain: &[Point], vertex: usize, scale: usize) -> f64 {
    let len = chain.len();
    let mut ahead = [Point::ZERO; MAX_SCALE];
    let mut behind = [Point::ZERO; MAX_SCALE];
    for step in 1..=scale {
        ahead[step - 1] = midpoint(chain, vertex + step);
        behind[step - 1] = midpoint(chain, vertex + 2 * len - 1 - step);
    }
    match (fit_line(&ahead[..scale]), fit_line(&behind[..scale])) {
        (Some(forward), Some(backward)) => -backward.direction.dot(forward.direction),
        _ => 1.0,
    }
}

/// The cosine of the angle between the chords `V[k-s]→V[k]` and `V[k]→V[k+s]`: sharpest at the
/// digitised apex itself, so it picks one vertex of a run of candidates.
fn chord_cos(chain: &[Point], vertex: usize, scale: usize) -> f64 {
    let len = chain.len();
    let into = chain[vertex] - chain[(vertex + len - scale) % len];
    let out = chain[(vertex + scale) % len] - chain[vertex];
    let lengths = (into.dot(into) * out.dot(out)).sqrt();
    if lengths == 0.0 {
        1.0
    } else {
        into.dot(out) / lengths
    }
}

/// Where a corner goes when its crossing is not safe: halfway between the midpoints of its two
/// edges, inside the pixel corner, so two shapes touching at that corner keep apart.
pub(crate) fn inset(chain: &[Point], k: usize) -> Point {
    let n = chain.len();
    midpoint(chain, k + n - 1).midpoint(midpoint(chain, k))
}

/// The midpoint of edge `i` (between vertices `i` and `i + 1`).
pub(crate) fn midpoint(chain: &[Point], i: usize) -> Point {
    let n = chain.len();
    chain[i % n].midpoint(chain[(i + 1) % n])
}

/// The structural corners of the closed chain `chain` (unit edges) for the corner cosine
/// `corner_cos` (a turn sharper than its angle is a corner), in chain order; `unit` is the
/// traced pixels per source pixel (2 after Upscale).
pub(crate) fn corners(chain: &[Point], corner_cos: f64, unit: usize) -> Vec<Corner> {
    let kept = vertices(chain, corner_cos, unit);
    place(chain, &kept, corner_cos, unit)
}

/// For every vertex whose sides turn sharper than `corner_cos` at two scales, the coarsest among
/// them (noise turns sharply only at the fine ones), and as much at the next finer scale as at
/// the coarsest (an arc, a small circle, turns about half as much there): its turn cosine at
/// the finest scale.
fn sharp_vertices(chain: &[Point], scales: &[usize], corner_cos: f64) -> Vec<Option<f64>> {
    let needed = scales.len().min(2);
    (0..chain.len())
        .map(|k| {
            let turns: Vec<f64> = scales.iter().map(|&s| turn_cos(chain, k, s)).collect();
            let count = turns.iter().filter(|&&t| t < corner_cos).count();
            let coarse = turns.last().is_some_and(|&t| t < corner_cos);
            let steady = match turns[..] {
                [.., finer, coarsest] => 1.0 - finer >= STEADY * (1.0 - coarsest),
                _ => true,
            };
            (count >= needed && coarse && steady).then_some(turns[0])
        })
        .collect()
}

/// The corner vertices with their turn cosines, in chain order: the sharp vertices whose apex
/// is the sharpest of the sharp vertices within the finest scale, sharpest first, one per scale
/// unit.
fn vertices(chain: &[Point], corner_cos: f64, unit: usize) -> Vec<(usize, f64)> {
    let n = chain.len();
    let scales = scales(n, unit);
    let Some(&finest) = scales.first() else {
        return Vec::new();
    };
    // Rounded to 1e-9, so a chain walked from another start or the image turned by 90 degrees
    // (the same numbers up to rounding) keeps the same order.
    let apex: Vec<f64> = (0..n)
        .map(|k| ((1.0 - chord_cos(chain, k, finest)) * 1e9).round())
        .collect();
    let sharp = sharp_vertices(chain, &scales, corner_cos);
    let mut candidates: Vec<(f64, usize, f64)> = (0..n)
        .filter_map(|k| {
            let c = sharp[k]?;
            let peak = (1..=finest).all(|d| {
                [(k + d) % n, (k + n - d) % n]
                    .iter()
                    .all(|&j| sharp[j].is_none() || apex[k] >= apex[j])
            });
            peak.then_some((apex[k], k, c))
        })
        .collect();
    // Equal turns are taken in chain order from a vertex that is no candidate, so a run of
    // equal candidates is never cut where the walk began.
    let origin = (0..n).find(|&k| sharp[k].is_none()).unwrap_or(0);
    candidates.sort_by(|a, b| {
        b.0.total_cmp(&a.0)
            .then(((a.1 + n - origin) % n).cmp(&((b.1 + n - origin) % n)))
    });
    // Equal neighbouring peaks closer than a scale unit are one corner (after Upscale a pixel
    // corner is two vertices).
    let mut taken = vec![false; n];
    let mut kept: Vec<(usize, f64)> = Vec::new();
    for (_, k, c) in candidates {
        if taken[k] {
            continue;
        }
        kept.push((k, c));
        for d in 0..unit.min(n / 2) {
            taken[(k + d) % n] = true;
            taken[(k + n - d) % n] = true;
        }
    }
    kept.sort_unstable_by_key(|&(k, _)| k);
    kept
}

/// The vertex a corner-free chain starts at: the sharpest turn at the coarsest usable scale,
/// the first of equals (so the start does not depend on where the walk began, up to ties).
pub(crate) fn loop_start(chain: &[Point], unit: usize) -> usize {
    let Some(&coarsest) = scales(chain.len(), unit).last() else {
        return 0;
    };
    let mut best = (f64::NEG_INFINITY, 0);
    for k in 0..chain.len() {
        let score = 1.0 - turn_cos(chain, k, coarsest);
        if score > best.0 {
            best = (score, k);
        }
    }
    best.1
}

#[cfg(test)]
// Test shapes are small, named by their formulas (x, y, w, h).
#[allow(clippy::cast_precision_loss, clippy::many_single_char_names)]
mod tests {
    use super::*;

    /// The crack chain around the pixels whose centres `inside` accepts.
    fn chain_of(w: usize, h: usize, inside: impl Fn(f64, f64) -> bool) -> Vec<Point> {
        let shape: Vec<bool> = (0..w * h)
            .map(|i| inside((i % w) as f64 + 0.5, (i / w) as f64 + 0.5))
            .collect();
        crate::contour::outer_loop(&shape, w, h, true)
            .into_iter()
            .map(|(x, y)| Point::new(f64::from(x), f64::from(y)))
            .collect()
    }

    /// The corners of a square rotated by `degrees` about (50, 50) with half side 30.
    fn rotated_square(degrees: f64) -> (Vec<Corner>, Vec<Point>) {
        let (s, c) = degrees.to_radians().sin_cos();
        let truth: Vec<Point> = [(-30.0, -30.0), (30.0, -30.0), (30.0, 30.0), (-30.0, 30.0)]
            .iter()
            .map(|&(x, y)| Point::new(50.0 + c * x - s * y, 50.0 + s * x + c * y))
            .collect();
        let chain = chain_of(100, 100, |x, y| {
            let (dx, dy) = (x - 50.0, y - 50.0);
            (c * dx + s * dy).abs() < 30.0 && (-s * dx + c * dy).abs() < 30.0
        });
        (corners(&chain, 0.5, 1), truth)
    }

    #[test]
    fn rotated_squares_have_four_corners_near_the_truth() {
        for degrees in [0.0, 15.0, 30.0, 45.0] {
            let (found, truth) = rotated_square(degrees);
            assert_eq!(found.len(), 4, "{degrees}: {found:?}");
            for t in &truth {
                let error = found
                    .iter()
                    .map(|c| (c.at - *t).hypot())
                    .fold(f64::INFINITY, f64::min);
                assert!(error < 0.5, "{degrees} degrees: {t:?} off by {error}");
            }
        }
    }

    #[test]
    fn a_circle_and_a_rounded_rectangle_have_no_corners() {
        let circle = chain_of(200, 200, |x, y| {
            (x - 100.0).powi(2) + (y - 100.0).powi(2) < 3600.0
        });
        assert_eq!(corners(&circle, 0.5, 1), []);
        let rounded = chain_of(300, 200, |x, y| {
            let (dx, dy) = ((x - 150.0).abs() - 100.0, (y - 100.0).abs() - 55.0);
            dx.max(0.0).powi(2) + dy.max(0.0).powi(2) < 100.0 && dx < 10.0 && dy < 10.0
        });
        assert_eq!(corners(&rounded, 0.5, 1), []);
    }

    #[test]
    fn corners_follow_the_chain_under_rotation_and_shift() {
        // An irregular quadrilateral, its pixels turned by 90 degrees and shifted exactly.
        let (w, h) = (90, 70);
        let quad = [(10.3, 12.1), (75.2, 20.7), (62.9, 61.4), (17.5, 50.2)];
        let inside = |x: f64, y: f64| {
            (0..4).all(|i| {
                let (a, b) = (quad[i], quad[(i + 1) % 4]);
                (b.0 - a.0) * (y - a.1) - (b.1 - a.1) * (x - a.0) > 0.0
            })
        };
        let shape: Vec<bool> = (0..w * h)
            .map(|i| inside((i % w) as f64 + 0.5, (i / w) as f64 + 0.5))
            .collect();
        let points = |s: &[bool], w: usize, h: usize| -> Vec<Point> {
            crate::contour::outer_loop(s, w, h, true)
                .into_iter()
                .map(|(x, y)| Point::new(f64::from(x), f64::from(y)))
                .collect()
        };
        let a = corners(&points(&shape, w, h), 0.5, 1);
        assert_eq!(a.len(), 4, "{a:?}");
        // Turned: the pixel (x, y) moves to (h - 1 - y, x), so a point (x, y) to (h - y, x).
        let mut turned = vec![false; w * h];
        for y in 0..h {
            for x in 0..w {
                turned[x * h + (h - 1 - y)] = shape[y * w + x];
            }
        }
        let b = corners(&points(&turned, h, w), 0.5, 1);
        assert_eq!(a.len(), b.len());
        for ca in &a {
            let mapped = Point::new(h as f64 - ca.at.y, ca.at.x);
            assert!(
                b.iter().any(|cb| (cb.at - mapped).hypot() < 1e-9),
                "{mapped:?} in {b:?}"
            );
        }
        // Shifted by whole pixels inside a larger box.
        let mut shifted = vec![false; (w + 7) * (h + 3)];
        for y in 0..h {
            for x in 0..w {
                shifted[(y + 3) * (w + 7) + x + 7] = shape[y * w + x];
            }
        }
        let c = corners(&points(&shifted, w + 7, h + 3), 0.5, 1);
        for (ca, cc) in a.iter().zip(&c) {
            assert!((cc.at - ca.at - kurbo::Vec2::new(7.0, 3.0)).hypot() < 1e-9);
        }
    }
}
