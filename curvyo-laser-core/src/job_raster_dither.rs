//! The dither of one raster item (ADR 0068 §5, ADR 0074; `specs/0123` criterion 10, `specs/0135`
//! criteria 42, 43): each dot's adjusted grey becomes a level, 0 off and 1 to 255 burning, in
//! integers only.

use crate::job_input::PointUm;
use crate::job_raster_screen::{Screen, ScreenShape};
use crate::operation_image_options::{Dither, ImageSettings};

/// The 8 x 8 Bayer matrix, values 0 to 63.
const BAYER: [[u16; 8]; 8] = [
    [0, 32, 8, 40, 2, 34, 10, 42],
    [48, 16, 56, 24, 50, 18, 58, 26],
    [12, 44, 4, 36, 14, 46, 6, 38],
    [60, 28, 52, 20, 62, 30, 54, 22],
    [3, 35, 11, 43, 1, 33, 9, 41],
    [51, 19, 59, 27, 49, 17, 57, 25],
    [15, 47, 7, 39, 13, 45, 5, 37],
    [63, 31, 55, 23, 61, 29, 53, 21],
];

/// One weight of an error diffusion: columns ahead, rows down, share.
type Weight = (i64, usize, i32);

const FLOYD_STEINBERG: (i32, &[Weight]) = (16, &[(1, 0, 7), (-1, 1, 3), (0, 1, 5), (1, 1, 1)]);
const JARVIS: (i32, &[Weight]) = (
    48,
    &[
        (1, 0, 7),
        (2, 0, 5),
        (-2, 1, 3),
        (-1, 1, 5),
        (0, 1, 7),
        (1, 1, 5),
        (2, 1, 3),
        (-2, 2, 1),
        (-1, 2, 3),
        (0, 2, 5),
        (1, 2, 3),
        (2, 2, 1),
    ],
);
const STUCKI: (i32, &[Weight]) = (
    42,
    &[
        (1, 0, 8),
        (2, 0, 4),
        (-2, 1, 2),
        (-1, 1, 4),
        (0, 1, 8),
        (1, 1, 4),
        (2, 1, 2),
        (-2, 2, 1),
        (-1, 2, 2),
        (0, 2, 4),
        (1, 2, 2),
        (2, 2, 1),
    ],
);
const ATKINSON: (i32, &[Weight]) = (
    8,
    &[
        (1, 0, 1),
        (2, 0, 1),
        (-1, 1, 1),
        (0, 1, 1),
        (1, 1, 1),
        (0, 2, 1),
    ],
);

/// Columns of margin on each side of the error rows, so the weights never leave them.
const MARGIN: i64 = 2;

/// The dither state of one raster item: fed row by row in grid order, each row's inside
/// stretches by increasing column.
#[derive(Debug)]
pub struct Ditherer {
    mode: Dither,
    threshold: u8,
    /// The error weights of a diffusion, `None` for the other modes.
    diffusion: Option<(i32, &'static [Weight])>,
    /// The lowest column of the grid.
    first_column: i64,
    /// Error accumulators of the current row and the two below, in units of `1 / D` grey.
    errors: [Vec<i32>; 3],
    /// The row the first accumulator belongs to.
    row: Option<i64>,
    /// The screen of Halftone and Line screen, `None` for the other modes.
    screen: Option<Screen>,
}

impl Ditherer {
    /// The dither of `settings` over a grid whose columns run from `first_column` for `width`.
    #[must_use]
    pub fn new(settings: &ImageSettings, first_column: i64, width: usize) -> Self {
        let diffusion = match settings.dither {
            Dither::FloydSteinberg => Some(FLOYD_STEINBERG),
            Dither::Jarvis => Some(JARVIS),
            Dither::Atkinson => Some(ATKINSON),
            Dither::Stucki => Some(STUCKI),
            Dither::Threshold
            | Dither::Ordered
            | Dither::Grey
            | Dither::Halftone
            | Dither::LineScreen => None,
        };
        let len = if diffusion.is_some() {
            width + 2 * usize::try_from(MARGIN).unwrap_or(2)
        } else {
            0
        };
        Self {
            mode: settings.dither,
            threshold: settings.threshold_pct,
            diffusion,
            first_column,
            errors: [vec![0; len], vec![0; len], vec![0; len]],
            row: None,
            screen: None,
        }
    }

    /// This dither with the screen of a screen mode for dots of `interval_um`, anchored at the
    /// item's first frame corner `anchor` (ADR 0074 §2); the other modes are unchanged.
    #[must_use]
    pub fn screened(mut self, settings: &ImageSettings, interval_um: i64, anchor: PointUm) -> Self {
        let shape = match settings.dither {
            Dither::Halftone => ScreenShape::Dots,
            Dither::LineScreen => ScreenShape::Lines,
            _ => return self,
        };
        self.screen = Some(Screen::new(
            shape,
            i64::from(settings.screen_size_hundredths) * 10,
            settings.screen_angle_deg,
            interval_um,
            anchor,
        ));
        self
    }

    /// Moves the error rows down to row `k` (rows only increase).
    fn enter_row(&mut self, k: i64) {
        let steps = self.row.map_or(0, |row| k - row);
        for _ in 0..steps.min(3) {
            self.errors.rotate_left(1);
            self.errors[2].fill(0);
        }
        self.row = Some(k);
    }

    /// The levels of the dots of row `k` from column `first` on, one per grey in `greys`,
    /// appended to `out`; `centre(j)` is the page point of dot `j`'s centre (read by the screens
    /// only).
    pub fn stretch(
        &mut self,
        k: i64,
        first: i64,
        greys: &[u8],
        centre: impl Fn(i64) -> PointUm,
        out: &mut Vec<u8>,
    ) {
        out.reserve(greys.len());
        if let Some(screen) = &self.screen {
            for (j, &g) in (first..).zip(greys) {
                out.push(if screen.burns(centre(j), g) { 255 } else { 0 });
            }
            return;
        }
        let Some((d, weights)) = self.diffusion else {
            for (j, &g) in (first..).zip(greys) {
                out.push(self.level(k, j, g));
            }
            return;
        };
        if self.row != Some(k) {
            self.enter_row(k);
        }
        for (j, &g) in (first..).zip(greys) {
            let at = self.index(j);
            let v = d * i32::from(g) + self.errors[0][at];
            let burn = v < 128 * d;
            let e = v - if burn { 0 } else { 255 * d };
            for &(dx, dy, w) in weights {
                let to = self.index(j + dx);
                if let Some(slot) = self.errors[dy].get_mut(to) {
                    *slot += (e * w).div_euclid(d);
                }
            }
            out.push(if burn { 255 } else { 0 });
        }
    }

    /// The accumulator index of column `j`.
    fn index(&self, j: i64) -> usize {
        usize::try_from(j - self.first_column + MARGIN).unwrap_or(usize::MAX)
    }

    /// The level of a dot of the modes without error.
    fn level(&self, k: i64, j: i64, g: u8) -> u8 {
        let burn = match self.mode {
            Dither::Threshold => 100 * u32::from(g) < u32::from(self.threshold) * 255,
            Dither::Ordered => {
                let m = BAYER[cell(k)][cell(j)];
                128 * u32::from(g) < (2 * u32::from(m) + 1) * 255
            }
            Dither::Grey => return 255 - g,
            Dither::FloydSteinberg
            | Dither::Jarvis
            | Dither::Atkinson
            | Dither::Stucki
            | Dither::Halftone
            | Dither::LineScreen => false,
        };
        if burn { 255 } else { 0 }
    }
}

/// `n mod 8` for the Bayer matrix.
fn cell(n: i64) -> usize {
    usize::try_from(n.rem_euclid(8)).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(dither: Dither) -> ImageSettings {
        ImageSettings {
            dither,
            ..ImageSettings::DEFAULT
        }
    }

    /// The levels of a `width` x `rows` area of one grey per column.
    fn area(dither: Dither, greys: &[u8], rows: i64) -> Vec<Vec<u8>> {
        let s = settings(dither);
        let mut d = Ditherer::new(&s, 0, greys.len()).screened(&s, 100, PointUm::new(0, 0));
        (0..rows)
            .map(|k| {
                let mut out = Vec::new();
                d.stretch(
                    k,
                    0,
                    greys,
                    |j| PointUm::new(j * 100 + 50, k * 100 + 50),
                    &mut out,
                );
                out
            })
            .collect()
    }

    fn burned(levels: &[Vec<u8>]) -> usize {
        levels.iter().flatten().filter(|&&l| l > 0).count()
    }

    const BINARY: [Dither; 8] = [
        Dither::Threshold,
        Dither::Ordered,
        Dither::FloydSteinberg,
        Dither::Jarvis,
        Dither::Atkinson,
        Dither::Stucki,
        Dither::Halftone,
        Dither::LineScreen,
    ];

    #[test]
    fn white_never_burns_and_black_always_burns() {
        for mode in Dither::ALL {
            assert_eq!(burned(&area(mode, &[255; 16], 16)), 0, "{mode:?}");
            assert_eq!(burned(&area(mode, &[0; 16], 16)), 256, "{mode:?}");
        }
    }

    #[test]
    fn a_darker_grey_burns_at_least_as_many_dots() {
        for mode in BINARY {
            let counts: Vec<usize> = (0..16_u8)
                .map(|step| burned(&area(mode, &[255 - step * 17; 32], 32)))
                .collect();
            assert!(
                counts.windows(2).all(|w| w[0] <= w[1]),
                "{mode:?} {counts:?}"
            );
        }
    }

    #[test]
    fn mid_grey_burns_about_half_in_the_diffusions() {
        for mode in [Dither::FloydSteinberg, Dither::Jarvis, Dither::Stucki] {
            let n = burned(&area(mode, &[128; 32], 32));
            assert!((480..=544).contains(&n), "{mode:?} {n}");
        }
        // Atkinson drops a quarter of the error: mid grey stays lighter than half or close to it.
        let n = burned(&area(Dither::Atkinson, &[128; 32], 32));
        assert!(n <= 544, "{n}");
    }

    #[test]
    fn threshold_and_ordered_follow_their_rules() {
        let t = area(Dither::Threshold, &[127, 128], 1);
        assert_eq!(t[0], vec![255, 0], "below 50 % burns");
        let o = area(Dither::Ordered, &[128; 8], 8);
        assert_eq!(burned(&o), 32, "half the Bayer cells");
        assert_eq!(o[0][0], 0, "M = 0 burns only the darkest greys");
        assert_eq!(o[7][0], 255, "M = 63 burns any grey below white");
    }

    #[test]
    fn grey_levels_are_the_darkness() {
        let g = area(Dither::Grey, &[0, 128, 254, 255], 1);
        assert_eq!(g[0], vec![255, 127, 1, 0]);
    }

    #[test]
    fn skipped_rows_drop_their_error() {
        let mut d = Ditherer::new(&settings(Dither::FloydSteinberg), 0, 4);
        let mut out = Vec::new();
        d.stretch(0, 0, &[200; 4], |_| PointUm::new(0, 0), &mut out);
        out.clear();
        d.stretch(5, 0, &[255; 4], |_| PointUm::new(0, 0), &mut out);
        assert_eq!(out, vec![0; 4], "no error reaches row 5 from row 0");
    }
}
