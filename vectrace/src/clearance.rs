//! Measures the room every boundary chain has (`specs/0089` adrs.md §5, criterion 10): for each
//! segment of a chain's midpoint polyline, the distance to the nearest other boundary, so the
//! smoothing and the fit can stay on their own side of every wall and gap by construction.
//!
//! The midpoints of all chains are bucketed by pixel row (a counting sort, no hash map) and
//! each midpoint searches the rows within the radius for the nearest polyline segment of another
//! chain, or of its own chain far enough away along it (the two sides of a corner meet closer
//! than that). Basic operations and `sqrt` only.

use kurbo::Point;

use crate::corners::midpoint;
use crate::line::segment_distance;

/// One midpoint in the row index: its x, its chain and its edge.
#[derive(Clone, Copy)]
struct Entry {
    x: f64,
    chain: u32,
    edge: u32,
}

/// The clearance of every segment (from point `k` to point `k + 1`) of every closed polyline of
/// `chains`, in traced pixels, capped at `radius`: the distance to the nearest segment of
/// another boundary, where parts of one chain at most `apart` points apart along it are not
/// another boundary (the two sides of a corner meet closer than that).
pub(crate) fn segment_clearances(chains: &[&[Point]], radius: f64, apart: usize) -> Vec<Vec<f64>> {
    let mut clear: Vec<Vec<f64>> = chains.iter().map(|c| vec![radius; c.len()]).collect();
    let index = Index::new(chains);
    for (owner, chain) in chains.iter().enumerate() {
        let count = chain.len();
        for edge in 0..count {
            let point = midpoint(chain, edge);
            let mut best = (radius, None);
            index.near(point, radius, |entry| {
                let other_index = entry.chain as usize;
                let other_edge = entry.edge as usize;
                let other = chains[other_index];
                let len = other.len();
                if other_index == owner {
                    let along = edge.abs_diff(other_edge);
                    if along.min(count - along) <= apart {
                        return;
                    }
                }
                let here = midpoint(other, other_edge);
                let before = midpoint(other, other_edge + len - 1);
                let after = midpoint(other, other_edge + 1);
                let to_before = segment_distance(point, before, here);
                let to_after = segment_distance(point, here, after);
                if to_before < best.0 {
                    best = (to_before, Some((other_index, (other_edge + len - 1) % len)));
                }
                if to_after < best.0 {
                    best = (to_after, Some((other_index, other_edge)));
                }
            });
            let (distance, nearest) = best;
            // The point's two segments, and the segment it is nearest to (a vertex of one
            // chain projecting onto the inside of another's segment).
            let own = &mut clear[owner];
            own[(edge + count - 1) % count] = own[(edge + count - 1) % count].min(distance);
            own[edge] = own[edge].min(distance);
            if let Some((other_index, segment)) = nearest {
                clear[other_index][segment] = clear[other_index][segment].min(distance);
            }
        }
    }
    clear
}

/// The midpoints of all chains bucketed by pixel row and sorted by x within a row.
struct Index {
    entries: Vec<Entry>,
    first_row: i64,
    /// Where each row starts (one more entry than rows).
    starts: Vec<usize>,
}

impl Index {
    /// Calls `visit` for every entry within `radius` plus one pixel of `point` in x and y.
    fn near(&self, point: Point, radius: f64, mut visit: impl FnMut(&Entry)) {
        #[allow(clippy::cast_possible_truncation)] // a few pixels
        let reach = (radius + 1.0).ceil() as i64;
        let row = row_of(point);
        for r in row - reach..=row + reach {
            let Some(slot) = usize::try_from(r - self.first_row).ok() else {
                continue;
            };
            if slot + 1 >= self.starts.len() {
                continue;
            }
            let entries = &self.entries[self.starts[slot]..self.starts[slot + 1]];
            let from = entries.partition_point(|e| e.x < point.x - radius - 1.0);
            for entry in &entries[from..] {
                if entry.x > point.x + radius + 1.0 {
                    break;
                }
                visit(entry);
            }
        }
    }
}

/// The pixel row of `p`.
fn row_of(p: Point) -> i64 {
    // invariant: picture coordinates fit `i64` easily.
    #[allow(clippy::cast_possible_truncation)]
    let r = p.y.floor() as i64;
    r
}

impl Index {
    /// The midpoints of `chains` bucketed by pixel row and sorted by x within a row.
    fn new(chains: &[&[Point]]) -> Self {
        let (entries, first_row, starts) = index(chains);
        Self {
            entries,
            first_row,
            starts,
        }
    }
}

/// The midpoints of all chains bucketed by pixel row and sorted by x within a row: the entries,
/// the first row, and where each row starts (one more entry than rows).
fn index(chains: &[&[Point]]) -> (Vec<Entry>, i64, Vec<usize>) {
    let mut total = 0;
    let (mut lo, mut hi) = (i64::MAX, i64::MIN);
    for chain in chains {
        total += chain.len();
        for k in 0..chain.len() {
            let r = row_of(midpoint(chain, k));
            lo = lo.min(r);
            hi = hi.max(r);
        }
    }
    if total == 0 {
        return (Vec::new(), 0, vec![0]);
    }
    let rows = usize::try_from(hi - lo + 1).unwrap_or(0);
    let mut counts = vec![0_usize; rows + 1];
    for chain in chains {
        for k in 0..chain.len() {
            let slot = usize::try_from(row_of(midpoint(chain, k)) - lo).unwrap_or(0);
            counts[slot + 1] += 1;
        }
    }
    for i in 1..counts.len() {
        counts[i] += counts[i - 1];
    }
    let starts = counts.clone();
    let mut fill = counts;
    let mut entries = vec![
        Entry {
            x: 0.0,
            chain: 0,
            edge: 0
        };
        total
    ];
    for (a, chain) in chains.iter().enumerate() {
        for k in 0..chain.len() {
            let m = midpoint(chain, k);
            let slot = usize::try_from(row_of(m) - lo).unwrap_or(0);
            entries[fill[slot]] = Entry {
                x: m.x,
                chain: u32::try_from(a).unwrap_or(u32::MAX),
                edge: u32::try_from(k).unwrap_or(u32::MAX),
            };
            fill[slot] += 1;
        }
    }
    for w in starts.windows(2) {
        entries[w[0]..w[1]].sort_unstable_by(|p, q| {
            p.x.total_cmp(&q.x)
                .then(p.chain.cmp(&q.chain))
                .then(p.edge.cmp(&q.edge))
        });
    }
    (entries, lo, starts)
}

#[cfg(test)]
// Test shapes are small, named by their formulas (x, y, w, h).
#[allow(clippy::cast_precision_loss, clippy::many_single_char_names)]
mod tests {
    use super::*;

    fn chain(rows: &[&str], eight: bool) -> Vec<Point> {
        let w = rows[0].len();
        let bits: Vec<bool> = rows
            .iter()
            .flat_map(|r| r.chars().map(|c| c == '#'))
            .collect();
        crate::contour::outer_loop(&bits, w, rows.len(), eight)
            .into_iter()
            .map(|(x, y)| Point::new(f64::from(x), f64::from(y)))
            .collect()
    }

    #[test]
    fn a_lone_square_has_the_full_radius_everywhere() {
        let square = chain(&["....", ".##.", ".##.", "...."], true);
        let clear = segment_clearances(&[&square], 6.0, 4);
        assert!(
            clear[0].iter().all(|&c| (c - 6.0).abs() < 1e-12),
            "{clear:?}"
        );
    }

    #[test]
    fn two_bars_one_pixel_apart_see_each_other() {
        // Two 6 x 3 bars with a 1 px gap: their facing sides are 1 px apart, their outer
        // sides see their own other side 3 px away.
        let rows = [
            "######", "######", "######", "......", "######", "######", "######",
        ];
        let a = chain(&rows[..4], true);
        let mut b = chain(&rows[3..], true);
        for p in &mut b {
            p.y += 3.0;
        }
        let clear = segment_clearances(&[&a, &b], 6.0, 4);
        let min = |c: &[f64]| c.iter().copied().fold(f64::INFINITY, f64::min);
        let max = |c: &[f64]| c.iter().copied().fold(0.0, f64::max);
        assert!((min(&clear[0]) - 1.0).abs() < 1e-9, "{:?}", clear[0]);
        assert!((min(&clear[1]) - 1.0).abs() < 1e-9, "{:?}", clear[1]);
        // The chords of the midpoint polyline cut the far corners: a little under 3.
        assert!(
            max(&clear[0]) > 2.9 && max(&clear[0]) <= 3.0,
            "{:?}",
            clear[0]
        );
        assert!(
            max(&clear[1]) > 2.9 && max(&clear[1]) <= 3.0,
            "{:?}",
            clear[1]
        );
    }

    #[test]
    fn a_hairline_sees_its_own_other_side_but_not_its_tip() {
        // A 1 x 12 line: the two long sides are 1 px apart (far along the chain), the end
        // segments see only what is beyond the 4 points apart.
        let line = chain(&["############"], true);
        let clear = segment_clearances(&[&line], 6.0, 4);
        let along: Vec<f64> = clear[0].clone();
        assert!(along.iter().any(|&c| (c - 1.0).abs() < 1e-9), "{along:?}");
        assert!(along.iter().all(|&c| c >= 1.0 - 1e-9), "{along:?}");
    }
}
