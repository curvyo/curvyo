//! Which leads and overcuts come too near a cut (`specs/0139` criterion 18, adrs L11): each lead
//! or overcut against the other burn paths of its block, boxes first through a grid, then
//! segments within 0.01 mm.

use std::collections::HashMap;

use crate::job_cut_ends::CutEnds;
use crate::job_input::PointUm;
use crate::job_order::OrderedPath;

/// A lead nearer than this to a cut touches it (criterion 18, adrs L10 `LEAD_TOUCH`), in µm.
pub(crate) const LEAD_TOUCH_UM: f64 = 10.0;

type Box2 = (f64, f64, f64, f64);

fn as_f64(um: i64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    // exact: page coordinates in µm are far below 2^53
    let value = um as f64;
    value
}

fn bounds(points: &[PointUm]) -> Box2 {
    let mut b = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for p in points {
        let (x, y) = (as_f64(p.x), as_f64(p.y));
        b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
    }
    (
        b.0 - LEAD_TOUCH_UM,
        b.1 - LEAD_TOUCH_UM,
        b.2 + LEAD_TOUCH_UM,
        b.3 + LEAD_TOUCH_UM,
    )
}

fn overlaps(a: Box2, b: Box2) -> bool {
    a.0 <= b.2 && b.0 <= a.2 && a.1 <= b.3 && b.1 <= a.3
}

#[allow(clippy::many_single_char_names)] // the usual names of a projection onto a segment
fn point_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (x, y) = (a.0 + dx * t - p.0, a.1 + dy * t - p.1);
    (x * x + y * y).sqrt()
}

fn cross(o: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

/// The distance of two segments (0 when they cross).
fn segments(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> f64 {
    let (d1, d2) = (cross(c, d, a), cross(c, d, b));
    let (d3, d4) = (cross(a, b, c), cross(a, b, d));
    if ((d1 > 0.0) != (d2 > 0.0)) && ((d3 > 0.0) != (d4 > 0.0)) && d1 != 0.0 && d3 != 0.0 {
        return 0.0;
    }
    point_segment(a, c, d)
        .min(point_segment(b, c, d))
        .min(point_segment(c, a, b))
        .min(point_segment(d, a, b))
}

fn xy(p: PointUm) -> (f64, f64) {
    (as_f64(p.x), as_f64(p.y))
}

/// Whether the polylines `a` and `b` come within [`LEAD_TOUCH_UM`].
fn near(a: &[PointUm], b: &[PointUm]) -> bool {
    a.windows(2).any(|s| {
        let sb = bounds(s);
        b.windows(2).any(|t| {
            overlaps(sb, bounds(t))
                && segments(xy(s[0]), xy(s[1]), xy(t[0]), xy(t[1])) <= LEAD_TOUCH_UM
        })
    })
}

/// The burn paths of a block on a grid of boxes, for lookups only (nothing iterates the map
/// into output).
struct Grid<'a> {
    paths: &'a [OrderedPath],
    boxes: Vec<Box2>,
    cell: f64,
    cells: HashMap<(i64, i64), Vec<usize>>,
}

fn cell_of(v: f64, cell: f64) -> i64 {
    #[allow(clippy::cast_possible_truncation)]
    // invariant: page coordinates over a cell of at least 1 mm stay far inside i64
    let c = (v / cell).floor() as i64;
    c
}

impl<'a> Grid<'a> {
    fn new(paths: &'a [OrderedPath]) -> Self {
        let boxes: Vec<Box2> = paths.iter().map(|p| bounds(&p.points)).collect();
        let mut sizes: Vec<f64> = boxes.iter().map(|b| (b.2 - b.0).max(b.3 - b.1)).collect();
        sizes.sort_by(f64::total_cmp);
        let cell = sizes
            .get(sizes.len() / 2)
            .copied()
            .unwrap_or(0.0)
            .max(1_000.0);
        let mut cells: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
        for (index, b) in boxes.iter().enumerate() {
            for x in cell_of(b.0, cell)..=cell_of(b.2, cell) {
                for y in cell_of(b.1, cell)..=cell_of(b.3, cell) {
                    cells.entry((x, y)).or_default().push(index);
                }
            }
        }
        Self {
            paths,
            boxes,
            cell,
            cells,
        }
    }

    /// Whether `lead` of unit `unit` comes near a burn path of another unit.
    fn touches(&self, unit: usize, lead: &[PointUm]) -> bool {
        let lb = bounds(lead);
        let mut seen: Vec<usize> = Vec::new();
        for x in cell_of(lb.0, self.cell)..=cell_of(lb.2, self.cell) {
            for y in cell_of(lb.1, self.cell)..=cell_of(lb.3, self.cell) {
                let Some(list) = self.cells.get(&(x, y)) else {
                    continue;
                };
                for &index in list {
                    if seen.contains(&index) {
                        continue;
                    }
                    seen.push(index);
                    let path = &self.paths[index];
                    if path.unit != unit
                        && overlaps(lb, self.boxes[index])
                        && near(lead, &path.points)
                    {
                        return true;
                    }
                }
            }
        }
        false
    }
}

/// One unit index per lead-in, overcut or lead-out of `ends` that comes within 0.01 mm of a
/// burn path of another unit in `paths`, or of its own outline away from its junction.
pub(crate) fn touching(ends: &[(usize, CutEnds)], paths: &[OrderedPath]) -> Vec<usize> {
    if ends
        .iter()
        .all(|(_, e)| e.lead_in.is_empty() && e.lead_out.is_empty() && e.overcut.is_empty())
    {
        return Vec::new();
    }
    let grid = Grid::new(paths);
    let mut out = Vec::new();
    for (unit, end) in ends {
        let leads = [
            (&end.lead_in, end.own_touch[0]),
            (&end.overcut, false),
            (&end.lead_out, end.own_touch[1]),
        ];
        for (lead, own) in leads {
            if lead.len() >= 2 && (own || grid.touches(*unit, lead)) {
                out.push(*unit);
            }
        }
    }
    out
}

/// Whether `lead`, read from its junction with `outline` (a closed ring) on, comes near the
/// outline again once it has left the 0.01 mm band around the junction (adrs L11).
pub(crate) fn touches_own(lead_from_junction: &[PointUm], outline: &[PointUm]) -> bool {
    let mut ring = outline.to_vec();
    if let Some(&first) = outline.first() {
        ring.push(first);
    }
    let away = lead_from_junction.iter().position(|&p| {
        ring.windows(2)
            .all(|s| point_segment(xy(p), xy(s[0]), xy(s[1])) > LEAD_TOUCH_UM)
    });
    let Some(away) = away else {
        return false;
    };
    near(&lead_from_junction[away..], &ring)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job_input::PointUm;

    fn path(unit: usize, points: &[(i64, i64)]) -> OrderedPath {
        OrderedPath {
            points: points.iter().map(|&(x, y)| PointUm::new(x, y)).collect(),
            unit,
            hatch: false,
        }
    }

    #[test]
    fn a_lead_that_crosses_or_comes_within_a_hundredth_of_another_cut_counts() {
        let cut = path(1, &[(0, 0), (10_000, 0)]);
        let crossing = CutEnds {
            lead_in: vec![PointUm::new(5_000, -1_000), PointUm::new(5_000, 1_000)],
            ..CutEnds::default()
        };
        let near_by = CutEnds {
            lead_out: vec![PointUm::new(0, 9), PointUm::new(-3_000, 9)],
            ..CutEnds::default()
        };
        let clear = CutEnds {
            overcut: vec![PointUm::new(0, 11), PointUm::new(3_000, 11)],
            ..CutEnds::default()
        };
        let ends = vec![(0, crossing), (2, near_by), (3, clear)];
        assert_eq!(touching(&ends, std::slice::from_ref(&cut)), [0, 2]);
        // Its own unit's paths do not count.
        let own = vec![(1, ends[0].1.clone())];
        assert_eq!(touching(&own, &[cut]).len(), 0);
    }

    #[test]
    fn a_lead_that_turns_back_onto_its_own_outline_touches_it() {
        let ring = [
            PointUm::new(0, 0),
            PointUm::new(10_000, 0),
            PointUm::new(10_000, 10_000),
            PointUm::new(0, 10_000),
        ];
        let away = [PointUm::new(5_000, 0), PointUm::new(5_000, -2_000)];
        assert!(!touches_own(&away, &ring));
        let back = [
            PointUm::new(5_000, 0),
            PointUm::new(5_000, -2_000),
            PointUm::new(6_000, 5),
        ];
        assert!(touches_own(&back, &ring));
    }
}
