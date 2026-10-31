//! Compares a placed glyph's shape with its traced candidate beyond their distance.
//!
//! A wide tolerance lets a neighbouring family's glyphs through, wrong letters among them; a sure
//! letter also needs the candidate's pieces and holes and a small shape residual (tester finding
//! 2026-10-31). The notice's deviation is measured here without the search cap.

use kurbo::Point;

use super::candidates::{Candidate, Prepared};
use super::distance::{Side, two_sided};
use super::reference::ReferenceGlyph;
use super::sample::{Ring, f};
use super::verify::{FaceFit, Scaled};

/// A sure letter's shape residual ([`shape_residual`]) is at most this share of the line's median
/// shape height, whatever the tolerance (Proposal, tester finding 2026-10-31: at 0.8 mm and more a
/// neighbouring family's glyphs fit within the tolerance, wrong letters among them; their
/// residual is 1.2 % of the height and more, the right letter in its own face or in a bolder or
/// lighter cut 0.5 % and less).
pub(crate) const SHAPE_SHARE: f64 = 0.008;
/// The residual a trace's own roughness gives a right letter at any size, mm (Proposal).
pub(crate) const SHAPE_FLOOR: f64 = 0.02;
/// A sure letter's residual is at most this share of the next character's: the right letter
/// stands out from the others by its shape, a wrong family's nearest glyph does not (Proposal).
pub(crate) const RESIDUAL_MARGIN: f64 = 0.35;
/// Whether `p` lies inside `rings` by the even-odd rule.
fn inside(rings: &[Ring], p: Point) -> bool {
    let mut odd = false;
    for ring in rings {
        for (i, &a) in ring.iter().enumerate() {
            let b = ring[(i + 1) % ring.len()];
            if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y) {
                odd = !odd;
            }
        }
    }
    odd
}

/// The outer rings and the holes of `rings` (a ring inside an odd number of others is a hole).
pub(crate) fn topology(rings: &[Ring]) -> (usize, usize) {
    let mut holes = 0;
    for (i, ring) in rings.iter().enumerate() {
        let Some(&p) = ring.first() else { continue };
        let around = rings
            .iter()
            .enumerate()
            .filter(|&(j, other)| j != i && inside(std::slice::from_ref(other), p))
            .count();
        holes += around % 2;
    }
    (rings.len() - holes, holes)
}

/// How far a placed glyph differs from its candidate beyond a change of weight: every point of
/// either side is far on the glyph's larger side (a glyph point outside the shape, a shape point
/// inside the glyph) or on its smaller side; a bolder or lighter cut of the right letter is far
/// on one side only, a wrong letter on both. The mean over all points of the smaller of the two
/// sums.
pub(crate) fn shape_residual(candidate: &Candidate, glyph: &Scaled, origin: Point) -> f64 {
    let shift = origin.to_vec2();
    let (mut larger, mut smaller) = (0.0, 0.0);
    for &p in &glyph.coarse {
        let at = p + shift;
        let d = candidate.coarse_grid.nearest(at).1.sqrt();
        if inside(&candidate.coarse_rings, at) {
            smaller += d;
        } else {
            larger += d;
        }
    }
    for &q in &candidate.coarse {
        let at = q - shift;
        let d = glyph.grid.nearest(at).1.sqrt();
        if inside(&glyph.coarse_rings, at) {
            larger += d;
        } else {
            smaller += d;
        }
    }
    let count = f(glyph.coarse.len() + candidate.coarse.len()).max(1.0);
    f64::min(larger, smaller) / count
}

/// The largest distance of any of `fit`'s letters at its place, measured without the cap that
/// makes the search fast (`CAP_TOLERANCES`): the deviation the notice names.
pub(crate) fn largest_uncapped(
    fit: &FaceFit,
    prepared: &Prepared,
    glyphs: &[ReferenceGlyph],
) -> f64 {
    let mut largest = 0.0_f64;
    for letter in &fit.letters {
        let candidate = &prepared.candidates[letter.candidate];
        let glyph = Scaled::new(&glyphs[letter.best.glyph], fit.scale, 1, 1e6);
        if glyph.points.is_empty() || candidate.points.is_empty() {
            continue;
        }
        let grid = candidate.grid.uncapped();
        let fixed = Side {
            points: &candidate.points,
            grid: &grid,
        };
        let moved = Side {
            points: &glyph.points,
            grid: &glyph.grid,
        };
        largest = largest.max(
            two_sided(&fixed, &moved, letter.best.origin.to_vec2())
                .0
                .max,
        );
    }
    largest
}

/// The shape residual of `glyph` placed at `origin` on `candidate` when both have the same
/// pieces and holes (a '6' is no 'ó', a 'u' no '¤'); `None` otherwise.
pub(crate) fn residual(candidate: &Candidate, glyph: &Scaled, origin: Point) -> Option<f64> {
    (topology(&candidate.coarse_rings) == topology(&glyph.coarse_rings))
        .then(|| shape_residual(candidate, glyph, origin))
}

/// The largest residual a letter with the traced shape has on a line of median shape `height`.
pub(crate) fn residual_limit(height: f64) -> f64 {
    (SHAPE_SHARE * height).max(SHAPE_FLOOR)
}
