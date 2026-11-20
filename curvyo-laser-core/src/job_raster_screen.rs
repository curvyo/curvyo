//! The threshold of a dot in a screen dither (ADR 0074, `specs/0135` criteria 42, 43): Halftone
//! dots and Line screen as one threshold table over a rotated cell grid anchored at the item.

use crate::job_input::PointUm;

/// `round(65536 · cos(d°))` for whole degrees 0 to 90; `sin(d°)` is `COS[90 − d]`.
const COS: [i64; 91] = [
    65536, 65526, 65496, 65446, 65376, 65287, 65177, 65048, 64898, 64729, 64540, 64332, 64104,
    63856, 63589, 63303, 62997, 62672, 62328, 61966, 61584, 61183, 60764, 60326, 59870, 59396,
    58903, 58393, 57865, 57319, 56756, 56175, 55578, 54963, 54332, 53684, 53020, 52339, 51643,
    50931, 50203, 49461, 48703, 47930, 47143, 46341, 45525, 44695, 43852, 42995, 42126, 41243,
    40348, 39441, 38521, 37590, 36647, 35693, 34729, 33754, 32768, 31772, 30767, 29753, 28729,
    27697, 26656, 25607, 24550, 23486, 22415, 21336, 20252, 19161, 18064, 16962, 15855, 14742,
    13626, 12505, 11380, 10252, 9121, 7987, 6850, 5712, 4572, 3430, 2287, 1144, 0,
];

/// The fixed point of the rotation.
const ONE: i64 = 65_536;

/// The smallest cell side in dots (ADR 0074 §4).
const MIN_CELL: i64 = 4;

/// The shape a screen burns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenShape {
    /// A round dot per cell that grows with the darkness.
    Dots,
    /// A band per cell row that thickens with the darkness.
    Lines,
}

/// The threshold table of one item's screen and its placement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Screen {
    /// Dots per cell side.
    n: i64,
    /// `n x n` thresholds, row `t` then column `s`.
    table: Vec<u8>,
    cos: i64,
    sin: i64,
    anchor: PointUm,
    interval_um: i64,
}

impl Screen {
    /// The screen of `shape` with cells of `size_um` turned by `angle_deg` (0 to 89) from the
    /// page's X axis, for dots of `interval_um`, anchored at `anchor`.
    #[must_use]
    pub fn new(
        shape: ScreenShape,
        size_um: i64,
        angle_deg: u8,
        interval_um: i64,
        anchor: PointUm,
    ) -> Self {
        let interval_um = interval_um.max(1);
        let n = div_round(size_um, interval_um).max(MIN_CELL);
        let angle = usize::from(angle_deg.min(89));
        Self {
            n,
            table: table(shape, n),
            cos: COS[angle],
            sin: COS[90 - angle],
            anchor,
            interval_um,
        }
    }

    /// Dots per cell side.
    #[cfg(test)]
    #[must_use]
    pub const fn cell(&self) -> i64 {
        self.n
    }

    /// Whether the dot centred at `p` with grey `g` burns: `255 − g > T` for the threshold of
    /// its position in the rotated cell.
    #[must_use]
    pub fn burns(&self, p: PointUm, g: u8) -> bool {
        let (dx, dy) = (p.x - self.anchor.x, p.y - self.anchor.y);
        let along = dx * self.cos + dy * self.sin;
        let across = dy * self.cos - dx * self.sin;
        let step = self.interval_um * ONE;
        let column = along.div_euclid(step).rem_euclid(self.n);
        let row = across.div_euclid(step).rem_euclid(self.n);
        let at = usize::try_from(row * self.n + column).unwrap_or(0);
        255 - u16::from(g) > u16::from(self.table[at])
    }
}

/// The `n x n` thresholds of `shape`: every position ranked by its key (distance from the cell
/// centre in quarter-dots for Dots, the row's distance for Lines), ties by `(t, s)`, and rank
/// `r` gets `T = round((2r + 1) · 255 / (2n²))`.
fn table(shape: ScreenShape, n: i64) -> Vec<u8> {
    let mut keys: Vec<(i64, i64, i64)> = (0..n)
        .flat_map(|t| (0..n).map(move |s| (t, s)))
        .map(|(t, s)| {
            let (ds, dt) = (2 * s - n + 1, 2 * t - n + 1);
            let key = match shape {
                ScreenShape::Dots => ds * ds + dt * dt,
                ScreenShape::Lines => dt.abs(),
            };
            (key, t, s)
        })
        .collect();
    keys.sort_unstable();
    let cells = n * n;
    let mut out = vec![0_u8; usize::try_from(cells).unwrap_or(0)];
    for (r, &(_, t, s)) in (0_i64..).zip(&keys) {
        let threshold = div_round((2 * r + 1) * 255, 2 * cells);
        let at = usize::try_from(t * n + s).unwrap_or(0);
        out[at] = u8::try_from(threshold.clamp(0, 255)).unwrap_or(u8::MAX);
    }
    out
}

/// `a / b` rounded half up, `b > 0`.
const fn div_round(a: i64, b: i64) -> i64 {
    (2 * a + b).div_euclid(2 * b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn burned(screen: &Screen, g: u8, side: i64) -> usize {
        (0..side)
            .flat_map(|y| (0..side).map(move |x| (x, y)))
            .filter(|&(x, y)| screen.burns(PointUm::new(x * 100 + 50, y * 100 + 50), g))
            .count()
    }

    #[test]
    fn white_burns_nothing_and_black_everything() {
        for shape in [ScreenShape::Dots, ScreenShape::Lines] {
            for angle in [0, 45, 89] {
                let s = Screen::new(shape, 800, angle, 100, PointUm::new(0, 0));
                assert_eq!(burned(&s, 255, 32), 0);
                assert_eq!(burned(&s, 0, 32), 32 * 32);
            }
        }
    }

    #[test]
    fn darker_burns_more_and_the_cell_has_at_least_four_dots() {
        let s = Screen::new(ScreenShape::Dots, 800, 0, 100, PointUm::new(0, 0));
        assert_eq!(s.cell(), 8);
        let counts: Vec<usize> = (0..=255).step_by(15).map(|g| burned(&s, g, 32)).collect();
        assert!(counts.windows(2).all(|w| w[0] >= w[1]), "{counts:?}");
        assert_eq!(
            Screen::new(ScreenShape::Dots, 200, 0, 100, PointUm::new(0, 0)).cell(),
            4
        );
    }

    #[test]
    fn a_dot_grows_from_the_cell_centre_and_a_line_from_its_row() {
        let s = Screen::new(ScreenShape::Dots, 800, 0, 100, PointUm::new(0, 0));
        let dot = |x: i64, y: i64, g| s.burns(PointUm::new(x * 100 + 50, y * 100 + 50), g);
        assert!(
            dot(3, 3, 230) && !dot(0, 0, 230),
            "light grey: the centre only"
        );
        let l = Screen::new(ScreenShape::Lines, 800, 0, 100, PointUm::new(0, 0));
        let line = |x: i64, y: i64, g| l.burns(PointUm::new(x * 100 + 50, y * 100 + 50), g);
        assert!(line(0, 3, 180) && line(7, 4, 180) && !line(0, 0, 180));
    }

    #[test]
    fn the_screen_moves_with_its_anchor() {
        let a = Screen::new(ScreenShape::Dots, 800, 30, 100, PointUm::new(0, 0));
        let b = Screen::new(ScreenShape::Dots, 800, 30, 100, PointUm::new(1_234, -567));
        for (x, y) in [(5, 7), (13, 2), (40, 41)] {
            let p = PointUm::new(x * 100 + 50, y * 100 + 50);
            let q = PointUm::new(p.x + 1_234, p.y - 567);
            assert_eq!(a.burns(p, 128), b.burns(q, 128));
        }
    }
}
