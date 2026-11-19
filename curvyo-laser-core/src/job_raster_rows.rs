//! The sweeps of one raster item (ADR 0069 §1, `specs/0123` criteria 14 to 16): each row's
//! levels cut at gaps over 5 mm into sweeps that every writer, the timeline and the preview read
//! as runs.

use crate::job_input::PointUm;
use crate::job_raster_grid::{DotGrid, GridRow};

/// An unburned gap longer than this, in µm, ends a sweep (criterion 15, confirmed).
pub const GAP_UM: i64 = 5_000;

/// The level of each dot of a sweep in run order: 0 off, 1 to 255 burning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Levels {
    /// One bit per dot (the binary modes, level 0 or 255).
    Binary {
        /// The bits, dot `i` at bit `i % 8` of byte `i / 8`.
        bits: Vec<u8>,
        /// The number of dots.
        len: usize,
    },
    /// One level per dot (Grey).
    Grey(Vec<u8>),
}

impl Levels {
    /// The levels of `levels`, packed to bits when `grey` is off.
    #[must_use]
    pub fn new(levels: &[u8], grey: bool) -> Self {
        if grey {
            return Self::Grey(levels.to_vec());
        }
        let mut bits = vec![0_u8; levels.len().div_ceil(8)];
        for (i, &level) in levels.iter().enumerate() {
            if level > 0 {
                bits[i / 8] |= 1 << (i % 8);
            }
        }
        Self::Binary {
            bits,
            len: levels.len(),
        }
    }

    /// The number of dots.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Binary { len, .. } => *len,
            Self::Grey(levels) => levels.len(),
        }
    }

    /// Whether there is no dot.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The level of dot `i` (0 beyond the end).
    #[must_use]
    pub fn get(&self, i: usize) -> u8 {
        match self {
            Self::Binary { bits, len } if i < *len => {
                if bits[i / 8] >> (i % 8) & 1 == 1 {
                    255
                } else {
                    0
                }
            }
            Self::Binary { .. } => 0,
            Self::Grey(levels) => levels.get(i).copied().unwrap_or(0),
        }
    }
}

/// One stretch of a sweep with one level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Run {
    /// Where it starts, in page µm.
    pub from: PointUm,
    /// Where it ends.
    pub to: PointUm,
    /// Its level, 0 for laser off.
    pub level: u8,
    /// Its number of dots.
    pub dots: usize,
}

/// One sweep: from the leading edge of its first burned dot to the trailing edge of its last,
/// the levels in run order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sweep {
    /// The first dot's leading edge.
    pub start: PointUm,
    /// The last dot's trailing edge.
    pub end: PointUm,
    /// The levels in run order.
    pub levels: Levels,
}

impl Sweep {
    /// The page point of the edge before dot `i` (`i = len` is the end), rounded to µm.
    #[must_use]
    pub fn point(&self, i: usize) -> PointUm {
        let n = i128::try_from(self.levels.len().max(1)).unwrap_or(1);
        let i = i128::try_from(i).unwrap_or(0).min(n);
        let lerp = |a: i64, b: i64| {
            let num = i128::from(b - a) * i;
            let half = if num >= 0 { n / 2 } else { -(n / 2) };
            a + i64::try_from((num + half) / n).unwrap_or(0)
        };
        PointUm::new(
            lerp(self.start.x, self.end.x),
            lerp(self.start.y, self.end.y),
        )
    }

    /// The runs of neighbouring dots of one level, in run order.
    pub fn runs(&self) -> impl Iterator<Item = Run> + '_ {
        let len = self.levels.len();
        let mut i = 0;
        std::iter::from_fn(move || {
            if i >= len {
                return None;
            }
            let level = self.levels.get(i);
            let first = i;
            while i < len && self.levels.get(i) == level {
                i += 1;
            }
            Some(Run {
                from: self.point(first),
                to: self.point(i),
                level,
                dots: i - first,
            })
        })
    }

    /// The number of burned dots.
    #[must_use]
    pub fn burned_dots(&self) -> usize {
        (0..self.levels.len())
            .filter(|&i| self.levels.get(i) > 0)
            .count()
    }
}

/// The power of a run of `level`, in tenths of a percent: off at level 0, else
/// `lowest + (set − lowest) · level / 255` rounded half up with `lowest` capped at `set`
/// (Proposal), so a binary level 255 burns at the set's power.
#[must_use]
pub fn run_power(level: u8, set_tenths: u16, lowest_tenths: u16) -> u16 {
    if level == 0 {
        return 0;
    }
    let set = u32::from(set_tenths);
    let lowest = u32::from(lowest_tenths).min(set);
    let power = lowest + ((set - lowest) * u32::from(level) * 2 + 255) / 510;
    u16::try_from(power).unwrap_or(set_tenths)
}

/// Appends the sweeps of `row` (with `levels`, one per inside dot in stretch order) to `out`, in
/// run order: reversed when `reverse` is on, split at gaps over [`GAP_UM`].
pub fn row_sweeps(
    grid: &DotGrid,
    row: &GridRow,
    levels: &[u8],
    reverse: bool,
    grey: bool,
    out: &mut Vec<Sweep>,
) {
    let max_gap = GAP_UM / grid.interval_um.max(1);
    let mut burned = Vec::new();
    let mut at = 0;
    for &(j0, j1) in &row.stretches {
        for j in j0..j1 {
            if levels.get(at).copied().unwrap_or(0) > 0 {
                burned.push((j, levels[at]));
            }
            at += 1;
        }
    }
    let first_new = out.len();
    let mut from = 0;
    while from < burned.len() {
        let mut to = from + 1;
        while to < burned.len() && burned[to].0 - burned[to - 1].0 - 1 <= max_gap {
            to += 1;
        }
        out.push(sweep_of(grid, row, &burned[from..to], reverse, grey));
        from = to;
    }
    if reverse {
        out[first_new..].reverse();
    }
}

/// The sweep of the burned dots `burned` (columns increasing, gaps at most [`GAP_UM`]).
fn sweep_of(
    grid: &DotGrid,
    row: &GridRow,
    burned: &[(i64, u8)],
    reverse: bool,
    grey: bool,
) -> Sweep {
    let (ja, jb) = (burned[0].0, burned[burned.len() - 1].0);
    let half = grid.interval_um / 2;
    let a = grid.point(row.s, grid.centre_t(ja) - half);
    let b = grid.point(row.s, grid.centre_t(jb) + half);
    let mut dense = vec![0_u8; usize::try_from(jb - ja + 1).unwrap_or(0)];
    for &(j, level) in burned {
        dense[usize::try_from(j - ja).unwrap_or(0)] = level;
    }
    let (start, end) = if reverse {
        dense.reverse();
        (b, a)
    } else {
        (a, b)
    };
    Sweep {
        start,
        end,
        levels: Levels::new(&dense, grey),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square_grid(side_um: i64, interval_um: i64) -> DotGrid {
        let ring = vec![
            PointUm::new(0, 0),
            PointUm::new(side_um, 0),
            PointUm::new(side_um, side_um),
            PointUm::new(0, side_um),
        ];
        DotGrid::new(&[ring], interval_um, 0.0).unwrap()
    }

    fn sweeps(grid: &DotGrid, levels: &[u8], reverse: bool) -> Vec<Sweep> {
        let mut out = Vec::new();
        row_sweeps(grid, &grid.rows[0], levels, reverse, false, &mut out);
        out
    }

    #[test]
    fn leading_and_trailing_white_is_not_travelled() {
        let grid = square_grid(1_000, 100);
        let levels = [0, 0, 255, 255, 0, 255, 0, 0, 0, 0];
        let s = sweeps(&grid, &levels, false);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].start, PointUm::new(200, 50));
        assert_eq!(s[0].end, PointUm::new(600, 50));
        let runs: Vec<(u8, usize)> = s[0].runs().map(|r| (r.level, r.dots)).collect();
        assert_eq!(runs, vec![(255, 2), (0, 1), (255, 1)]);
        assert_eq!(s[0].burned_dots(), 3);
    }

    #[test]
    fn a_gap_over_five_mm_splits_the_sweep() {
        let grid = square_grid(20_000, 100);
        let mut levels = vec![0_u8; 200];
        levels[0] = 255;
        levels[51] = 255; // a gap of 50 dots = 5 mm: kept
        levels[103] = 255; // a gap of 51 dots = 5.1 mm: split
        let s = sweeps(&grid, &levels, false);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].levels.len(), 52);
        assert_eq!(s[1].start, PointUm::new(10_300, 50));
    }

    #[test]
    fn a_reversed_row_runs_from_its_end_with_its_levels_reversed() {
        let grid = square_grid(20_000, 100);
        let mut levels = vec![0_u8; 200];
        levels[0] = 255;
        levels[1] = 255;
        levels[150] = 255;
        let s = sweeps(&grid, &levels, true);
        assert_eq!(s.len(), 2);
        assert_eq!(
            s[0].start,
            PointUm::new(15_100, 50),
            "the right sweep first"
        );
        assert_eq!(s[1].start, PointUm::new(200, 50));
        assert_eq!(s[1].end, PointUm::new(0, 50));
        let empty: Vec<u8> = vec![0; 200];
        assert!(
            sweeps(&grid, &empty, false).is_empty(),
            "an empty row writes nothing"
        );
    }

    #[test]
    fn runs_end_on_whole_dots_and_grey_levels_stay_per_dot() {
        let grid = square_grid(1_000, 100);
        let mut out = Vec::new();
        row_sweeps(
            &grid,
            &grid.rows[0],
            &[10, 10, 200, 0, 0, 0, 0, 0, 0, 0],
            false,
            true,
            &mut out,
        );
        let runs: Vec<Run> = out[0].runs().collect();
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].to, PointUm::new(200, 50));
        assert_eq!(runs[1].level, 200);
        assert_eq!(runs[1].to, out[0].end);
    }

    #[test]
    fn the_run_power_ramps_from_lowest_to_the_set_and_never_inverts() {
        assert_eq!(run_power(0, 800, 100), 0);
        assert_eq!(run_power(255, 800, 100), 800);
        assert_eq!(run_power(1, 800, 100), 103, "100 + 700 / 255 = 102.7");
        assert_eq!(run_power(128, 1000, 0), 502);
        assert_eq!(run_power(1, 500, 900), 500, "lowest above the set: the set");
        assert_eq!(run_power(255, 500, 900), 500);
    }

    #[test]
    fn binary_levels_pack_to_bits() {
        let levels = Levels::new(&[255, 0, 0, 255, 0, 0, 0, 0, 255], false);
        assert!(matches!(&levels, Levels::Binary { bits, len: 9 } if bits.len() == 2));
        let read: Vec<u8> = (0..9).map(|i| levels.get(i)).collect();
        assert_eq!(read, vec![255, 0, 0, 255, 0, 0, 0, 0, 255]);
    }
}
