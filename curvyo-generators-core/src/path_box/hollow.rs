//! The finger hollow of a seat lid on a curved stretch: cut into the longest convex run of the lid's
//! outline that holds it (ADR 0083 §13.10).

use curvyo_document_core::Point;

use crate::contour::{Contour, line_circle};
use crate::footprint::{Footprint, Shape};
use crate::shape_ops::{Chain, direction, length};
use crate::wall_plates::Hollow;

/// Whether a sweep bends the convex way; a rounding-sized sweep counts as convex.
fn convex(sweep: f64) -> bool {
    sweep > -1e-9
}

/// Two directions closer than this (radians) are tangent.
const TANGENT: f64 = 1e-6;

/// Stretches of bends that run on tangent and bend one way (a convex stretch of a run ends where
/// the run turns concave, as at a kink's fillet in the notch), as lists of span indices in walk
/// order; a stretch that wraps past the last span is rotated to start where it begins.
fn runs(footprint: &Footprint) -> Vec<Vec<usize>> {
    let spans = &footprint.spans;
    let count = spans.len();
    let sweep = |i: usize| match spans[i % count].shape {
        Shape::Bend { sweep, .. } => Some(sweep),
        Shape::Straight { .. } => None,
    };
    // Whether span `i` runs on into span `i + 1`.
    let joined = |i: usize| {
        let (a, b) = (
            direction(&spans[i % count].shape, true),
            direction(&spans[(i + 1) % count].shape, false),
        );
        let tangent = libm::atan2(a.0 * b.1 - a.1 * b.0, a.0 * b.0 + a.1 * b.1).abs() < TANGENT;
        matches!((sweep(i), sweep(i + 1)), (Some(x), Some(y)) if convex(x) == convex(y)) && tangent
    };
    // Start where a stretch cannot continue from the span before; a closed loop starts at 0.
    let first = (0..count).find(|&i| !joined(i + count - 1)).unwrap_or(0);
    let mut out: Vec<Vec<usize>> = Vec::new();
    for step in 0..count {
        let i = (first + step) % count;
        if sweep(i).is_none() {
            continue;
        }
        let continues = step > 0 && joined(i + count - 1);
        match out.last_mut() {
            Some(run) if continues => run.push(i),
            _ => out.push(vec![i]),
        }
    }
    out
}

/// The distance along `shape` of the point `p` on its circle, `None` when it lies off the arc.
fn along(shape: &Shape, p: Point) -> Option<f64> {
    let Shape::Bend {
        centre,
        radius,
        from,
        sweep,
    } = *shape
    else {
        return None;
    };
    let angle = libm::atan2(p.y - centre.y, p.x - centre.x);
    let mut a = ((angle - from) * sweep.signum()).rem_euclid(std::f64::consts::TAU);
    if a > std::f64::consts::TAU - 1e-9 {
        a = 0.0;
    }
    (a <= sweep.abs() + 1e-9).then_some(a * radius)
}

/// Where the line through `at` along `along_dir` first meets the run (the meeting nearest the
/// boundary, the least coordinate along `along_dir`): the distance along the run and the point.
fn meet(chain: &Chain, at: Point, along_dir: (f64, f64)) -> Option<(f64, Point)> {
    let reach = chain.total.max(1.0) * 2.0;
    let (a, b) = (
        Point::new(at.x - along_dir.0 * reach, at.y - along_dir.1 * reach),
        Point::new(at.x + along_dir.0 * reach, at.y + along_dir.1 * reach),
    );
    let mut best: Option<(f64, f64, Point)> = None;
    let mut done = 0.0;
    for shape in &chain.shapes {
        if let Shape::Bend { centre, radius, .. } = *shape {
            for p in line_circle(a, b, centre, radius) {
                if let Some(d) = along(shape, p) {
                    let depth = (p.x - at.x) * along_dir.0 + (p.y - at.y) * along_dir.1;
                    if best.is_none_or(|(_, least, _)| depth < least) {
                        best = Some((done + d, depth, p));
                    }
                }
            }
        }
        done += length(shape);
    }
    best.map(|(d, _, p)| (d, p))
}

/// The notch in the run `chain` at its middle: the run's arcs up to the hollow, its two sides and
/// its end, the arcs after it. `None` when the hollow does not fit or the run is not convex
/// across its width.
fn notch(chain: &Chain, hollow: Hollow) -> Option<Vec<Piece>> {
    let half = hollow.width / 2.0;
    let middle = chain.total / 2.0;
    let m = chain.at(middle);
    let ahead = chain.at(middle + 0.01);
    let behind = chain.at(middle - 0.01);
    let t = {
        let (dx, dy) = (ahead.x - behind.x, ahead.y - behind.y);
        let l = libm::hypot(dx, dy).max(1e-12);
        (dx / l, dy / l)
    };
    let u = (-t.1, t.0);
    let side = |sign: f64| Point::new(m.x + sign * t.0 * half, m.y + sign * t.1 * half);
    let (s1, p1) = meet(chain, side(-1.0), u)?;
    let (s2, p2) = meet(chain, side(1.0), u)?;
    if !(s1 < middle && middle < s2) {
        return None;
    }
    let convex = chain
        .part(s1, s2)
        .iter()
        .all(|s| matches!(s, Shape::Bend { sweep, .. } if convex(*sweep)));
    if !convex {
        return None;
    }
    let depth_of = |p: Point| (p.x - m.x) * u.0 + (p.y - m.y) * u.1;
    let level = (hollow.depth - half).max(depth_of(p1)).max(depth_of(p2));
    let centre = Point::new(m.x + u.0 * level, m.y + u.1 * level);
    let (c1, c2) = (
        Point::new(centre.x - t.0 * half, centre.y - t.1 * half),
        Point::new(centre.x + t.0 * half, centre.y + t.1 * half),
    );
    let mut pieces: Vec<Piece> = chain.part(0.0, s1).into_iter().map(Piece::Shape).collect();
    pieces.push(Piece::Line(p1, c1));
    pieces.push(Piece::Round {
        centre,
        radius: half,
        from: libm::atan2(-t.1, -t.0),
    });
    pieces.push(Piece::Line(c2, p2));
    pieces.extend(chain.part(s2, chain.total).into_iter().map(Piece::Shape));
    Some(pieces)
}

enum Piece {
    Shape(Shape),
    Line(Point, Point),
    Round {
        centre: Point,
        radius: f64,
        from: f64,
    },
}

fn push(contour: &mut Contour, piece: &Piece) {
    match *piece {
        Piece::Shape(Shape::Straight { from, to }) | Piece::Line(from, to) => {
            contour.line_to(from, to);
        }
        Piece::Shape(Shape::Bend {
            centre,
            radius,
            from,
            sweep,
        }) => contour.arc(centre, radius, from, sweep),
        Piece::Round {
            centre,
            radius,
            from,
        } => contour.arc(centre, radius, from, -std::f64::consts::PI),
    }
}

/// The lid outline `footprint` with the hollow cut into its longest convex run that holds it,
/// else the next longest; `None` when no run takes it.
pub(crate) fn lid_with_hollow(footprint: &Footprint, hollow: Hollow, wall: f64) -> Option<Contour> {
    let mut candidates: Vec<(f64, Vec<usize>)> = runs(footprint)
        .into_iter()
        .map(|run| {
            let total = run
                .iter()
                .map(|&i| length(&footprint.spans[i].shape))
                .sum::<f64>();
            (total, run)
        })
        .filter(|(total, _)| *total >= hollow.width + 2.0 * wall)
        .collect();
    candidates.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (_, run) in candidates {
        let convex = run.iter().all(
            |&i| matches!(footprint.spans[i].shape, Shape::Bend { sweep, .. } if convex(sweep)),
        );
        if !convex {
            continue;
        }
        let chain = Chain::new(run.iter().map(|&i| footprint.spans[i].shape).collect());
        let Some(pieces) = notch(&chain, hollow) else {
            continue;
        };
        let count = footprint.spans.len();
        let (first, last) = (run[0], run[run.len() - 1]);
        let mut contour = Contour::new();
        // The run first, then the spans after it round to the span before it.
        for piece in &pieces {
            push(&mut contour, piece);
        }
        let mut i = (last + 1) % count;
        while i != first {
            match footprint.spans[i].shape {
                Shape::Straight { from, to } => contour.line_to(from, to),
                Shape::Bend {
                    centre,
                    radius,
                    from,
                    sweep,
                } => contour.arc(centre, radius, from, sweep),
            }
            i = (i + 1) % count;
        }
        return Some(contour);
    }
    None
}
