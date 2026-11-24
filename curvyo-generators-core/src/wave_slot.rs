//! The outline of a Wave slot: a spine of tangent arcs that swings either side of the column
//! line, with its two sides as exact offsets of the spine (ADR 0082 §6).
//!
//! Each half wave is one arc from inflection to inflection, `period` long along the slot. Its
//! sagitta is `period / 8` and its radius `17 / 16 period`, so the steepest point leans 28 degrees
//! and two neighbouring slots stay at least 0.88 of the spacing apart.

use curvyo_document_core::Point;

use crate::contour::Seg;

/// The radius of a half wave over its period.
const RADIUS: f64 = 17.0 / 16.0;

/// The two sides of a wave slot, each a chain of arcs in the direction of growing `y`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Wave {
    pub(crate) left: Vec<Seg>,
    pub(crate) right: Vec<Seg>,
}

/// The wave of `half_waves` arcs of length `period` along the column `x`, starting at the
/// inflection point `(x, y_start)`, `width` wide. A side is cut where it crosses the horizontal
/// lines `top` and `bottom` when they are given (a slot that runs out through an edge).
pub(crate) fn wave(
    (x, y_start): (f64, f64),
    period: f64,
    width: f64,
    half_waves: u32,
    (top, bottom): (Option<f64>, Option<f64>),
) -> Wave {
    let radius = RADIUS * period;
    let half = (period / 2.0 / radius).asin();
    let lean = radius * libm::cos(half);
    let (mut left, mut right) = (Vec::new(), Vec::new());
    for index in 0..half_waves {
        let top_y = y_start + f64::from(index) * period;
        let (centre, from, sweep, inner_is_left) = if index % 2 == 0 {
            (
                Point::new(x - lean, top_y + radius * libm::sin(half)),
                -half,
                2.0 * half,
                true,
            )
        } else {
            (
                Point::new(x + lean, top_y + radius * libm::sin(half)),
                std::f64::consts::PI + half,
                -2.0 * half,
                false,
            )
        };
        let arc = |radius: f64| Seg::Arc {
            centre,
            radius,
            from,
            sweep,
        };
        let (inner, outer) = (arc(radius - width / 2.0), arc(radius + width / 2.0));
        left.push(if inner_is_left { inner } else { outer });
        right.push(if inner_is_left { outer } else { inner });
    }
    Wave {
        left: clip(left, top, bottom),
        right: clip(right, top, bottom),
    }
}

fn arc_y(seg: &Seg) -> (f64, f64) {
    (seg.start().y, seg.end().y)
}

/// The chain cut at the horizontal lines `top` (everything above goes) and `bottom` (everything
/// below goes).
fn clip(chain: Vec<Seg>, top: Option<f64>, bottom: Option<f64>) -> Vec<Seg> {
    let mut out = Vec::new();
    for seg in chain {
        let (y0, y1) = arc_y(&seg);
        let Seg::Arc {
            centre,
            radius,
            from,
            sweep,
        } = seg
        else {
            continue;
        };
        if top.is_some_and(|edge| y1 <= edge)
            || bottom.is_some_and(|edge| y0 >= bottom.unwrap_or(edge))
        {
            continue;
        }
        let (mut start, mut end) = (from, from + sweep);
        if let Some(angle) = top
            .filter(|edge| y0 < *edge)
            .and_then(|edge| seg.angle_at_y(edge))
        {
            start = angle;
        }
        if let Some(angle) = bottom
            .filter(|edge| y1 > *edge)
            .and_then(|edge| seg.angle_at_y(edge))
        {
            end = angle;
        }
        out.push(Seg::Arc {
            centre,
            radius,
            from: start,
            sweep: end - start,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn radius(seg: &Seg) -> f64 {
        match seg {
            Seg::Arc { radius, .. } => *radius,
            Seg::Line { .. } => panic!("an arc"),
        }
    }

    #[test]
    fn the_sides_are_exact_offsets_of_the_spine_and_the_chain_is_tangent() {
        let wave = wave((10.0, 0.0), 4.0, 0.3, 4, (None, None));
        assert_eq!(wave.left.len(), 4);
        for (l, r) in wave.left.iter().zip(&wave.right) {
            assert!(((radius(l) - radius(r)).abs() - 0.3).abs() < 1e-9);
        }
        // Tangent joins: the end of one arc is the start of the next, on both sides.
        for chain in [&wave.left, &wave.right] {
            for pair in chain.windows(2) {
                let (a, b) = (pair[0].end(), pair[1].start());
                assert!(libm::hypot(a.x - b.x, a.y - b.y) < 1e-9, "{a:?} {b:?}");
            }
        }
    }

    #[test]
    fn the_spine_swings_one_eighth_of_the_period_and_leans_28_degrees() {
        let period = 4.0;
        let wave = wave((0.0, 0.0), period, 0.0001, 2, (None, None));
        // The spine bulges right by period / 8 half way down the first half wave.
        let Seg::Arc { centre, radius, .. } = wave.left[0] else {
            panic!("an arc");
        };
        let bulge = centre.x + radius;
        assert!((bulge - period / 8.0).abs() < 1e-3, "{bulge}");
        let steepest = ((period / 2.0) / radius).asin().to_degrees();
        assert!((steepest - 28.07).abs() < 0.01, "{steepest}");
        // A half wave ends one period further down at the column line.
        let end = wave.left[0].end();
        assert!(
            (end.y - period).abs() < 1e-3 && end.x.abs() < 1e-3,
            "{end:?}"
        );
    }

    #[test]
    fn a_wave_that_runs_out_is_cut_at_the_edge() {
        let wave = wave((10.0, -9.0), 4.0, 0.3, 5, (Some(0.0), None));
        for side in [&wave.left, &wave.right] {
            let first = side[0].start();
            assert!(first.y.abs() < 1e-9, "starts on the edge: {first:?}");
        }
    }
}
