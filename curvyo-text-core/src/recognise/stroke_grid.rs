//! The grid operations the centre-line match (`stroke.rs`) runs on a line's raster.

use kurbo::{ParamCurve, PathSeg, Point};

/// The value at `share` of `values` sorted ascending (0 for none).
pub(super) fn quantile(values: &mut [f64], share: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    // invariant: a share of a length, rounded into the slice.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let at = ((values.len() - 1) as f64 * share).round() as usize;
    values[at.min(values.len() - 1)]
}

/// The cell ranges `x0..x1` of row centre `cy` inside `ring` (grid units) by the non-zero rule.
pub(super) fn spans(ring: &[Point], cy: f64, columns: usize) -> Vec<(usize, usize)> {
    let mut crossings: Vec<(f64, i32)> = Vec::new();
    for k in 0..ring.len() {
        let (p, q) = (ring[k], ring[(k + 1) % ring.len()]);
        if (p.y <= cy) != (q.y <= cy) {
            let x = p.x + (cy - p.y) / (q.y - p.y) * (q.x - p.x);
            crossings.push((x, if q.y > p.y { 1 } else { -1 }));
        }
    }
    crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out = Vec::new();
    let mut winding = 0;
    let mut start = 0.0;
    for (x, turn) in crossings {
        let was = winding;
        winding += turn;
        if was == 0 && winding != 0 {
            start = x;
        } else if was != 0 && winding == 0 {
            // invariant: clamped into the row before the casts.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let (x0, x1) = (
                (start - 0.5).ceil().max(0.0) as usize,
                ((x - 0.5).floor().max(-1.0) + 1.0) as usize,
            );
            if x0 < x1.min(columns) {
                out.push((x0, x1.min(columns)));
            }
        }
    }
    out
}

/// Zhang-Suen thinning of `grid` (`columns` x `rows`, a one-cell empty border).
pub(super) fn thin(grid: &mut [bool], columns: usize, rows: usize) {
    let mut changed = true;
    let mut gone = Vec::new();
    while changed {
        changed = false;
        for pass in 0..2 {
            gone.clear();
            for y in 1..rows - 1 {
                for x in 1..columns - 1 {
                    let i = y * columns + x;
                    if !grid[i] {
                        continue;
                    }
                    // P2 to P9 clockwise from north.
                    let p = [
                        grid[i - columns],
                        grid[i - columns + 1],
                        grid[i + 1],
                        grid[i + columns + 1],
                        grid[i + columns],
                        grid[i + columns - 1],
                        grid[i - 1],
                        grid[i - columns - 1],
                    ];
                    let neighbours = p.iter().filter(|b| **b).count();
                    let turns = (0..8).filter(|&k| !p[k] && p[(k + 1) % 8]).count();
                    let (a, b) = if pass == 0 {
                        (p[0] && p[2] && p[4], p[2] && p[4] && p[6])
                    } else {
                        (p[0] && p[2] && p[6], p[0] && p[4] && p[6])
                    };
                    if (2..=6).contains(&neighbours) && turns == 1 && !a && !b {
                        gone.push(i);
                    }
                }
            }
            for &i in &gone {
                grid[i] = false;
            }
            changed |= !gone.is_empty();
        }
    }
}

/// Per cell the distance to the nearest set cell of `set`, in cells (chamfer 3-4, divided by 3).
pub(super) fn chamfer(set: &[bool], columns: usize, rows: usize) -> Vec<f64> {
    let far = 3.0 * f64::from(u32::try_from(columns + rows).unwrap_or(u32::MAX));
    let mut d: Vec<f64> = set.iter().map(|&s| if s { 0.0 } else { far }).collect();
    for y in 0..rows {
        for x in 0..columns {
            let i = y * columns + x;
            let mut best = d[i];
            if x > 0 {
                best = best.min(d[i - 1] + 3.0);
            }
            if y > 0 {
                best = best.min(d[i - columns] + 3.0);
                if x > 0 {
                    best = best.min(d[i - columns - 1] + 4.0);
                }
                if x + 1 < columns {
                    best = best.min(d[i - columns + 1] + 4.0);
                }
            }
            d[i] = best;
        }
    }
    for y in (0..rows).rev() {
        for x in (0..columns).rev() {
            let i = y * columns + x;
            let mut best = d[i];
            if x + 1 < columns {
                best = best.min(d[i + 1] + 3.0);
            }
            if y + 1 < rows {
                best = best.min(d[i + columns] + 3.0);
                if x + 1 < columns {
                    best = best.min(d[i + columns + 1] + 4.0);
                }
                if x > 0 {
                    best = best.min(d[i + columns - 1] + 4.0);
                }
            }
            d[i] = best;
        }
    }
    d.iter().map(|v| v / 3.0).collect()
}

/// Points along `path` (open or closed) about `spacing` apart.
pub(super) fn path_points(path: &kurbo::BezPath, spacing: f64) -> Vec<Point> {
    let mut out = Vec::new();
    for seg in path.segments() {
        let length = match seg {
            PathSeg::Line(l) => l.p0.distance(l.p1),
            PathSeg::Quad(q) => q.p0.distance(q.p1) + q.p1.distance(q.p2),
            PathSeg::Cubic(c) => c.p0.distance(c.p1) + c.p1.distance(c.p2) + c.p2.distance(c.p3),
        };
        // invariant: a positive length over a positive spacing, capped.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let steps = ((length / spacing).ceil() as usize).clamp(1, 10_000);
        for k in 0..=steps {
            // invariant: small counts.
            #[allow(clippy::cast_precision_loss)]
            out.push(seg.eval(k as f64 / steps as f64));
        }
    }
    out
}
