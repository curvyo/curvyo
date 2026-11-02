//! The variance of one cell (`specs/0069-tiling` criteria 16 and 21): random and stepped values
//! per channel, drawn in a fixed order from the cell's own stretch of the seeded stream.

use curvyo_document_core::{Angle, CellKey, Channel, Length, Percent, Variance, Vec2};

use super::pcg32::Pcg32;

/// Draws per cell: rotation, scale X, scale Y, offset X, offset Y, opacity, hue and one spare.
const DRAWS_PER_CELL: u64 = 8;

/// The smallest scale factor a variance can give; a smaller one would flatten or flip the
/// instance (Proposal).
const MIN_SCALE_FACTOR: f64 = 0.01;

/// The varied values of one cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellVariance {
    /// Rotation, counterclockwise on screen.
    pub rotation: Angle,
    /// Scale factor X.
    pub scale_x: f64,
    /// Scale factor Y.
    pub scale_y: f64,
    /// Offset.
    pub offset: Vec2,
    /// Opacity shift in percentage points.
    pub opacity: Percent,
    /// Hue shift.
    pub hue: Angle,
}

impl CellVariance {
    /// No variance at all (the kept source's cell).
    pub const NONE: Self = Self {
        rotation: Angle::from_radians(0.0),
        scale_x: 1.0,
        scale_y: 1.0,
        offset: Vec2::ZERO,
        opacity: Percent::new(0.0),
        hue: Angle::from_radians(0.0),
    };
}

/// `row · per_row + column · per_column + (2u - 1) · random`, the value of criterion 16.
fn value(channel: [f64; 3], key: CellKey, unit: f64) -> f64 {
    let [random, per_row, per_column] = channel;
    f64::from(key.row) * per_row
        + f64::from(key.column) * per_column
        + 2.0f64.mul_add(unit, -1.0) * random
}

fn angles(c: Channel<Angle>) -> [f64; 3] {
    [c.random, c.per_row, c.per_column].map(Angle::as_radians)
}

fn percents(c: Channel<Percent>) -> [f64; 3] {
    [c.random, c.per_row, c.per_column].map(Percent::get)
}

fn lengths(c: Channel<Length>) -> [f64; 3] {
    [c.random, c.per_row, c.per_column].map(Length::as_mm)
}

/// The variance of the cell `key` for `seed`: the stream starts at `seed`, skips
/// `key.index() · 8` draws and draws all seven channels in order, also those at zero, so one
/// channel or the cell count never changes another's values (criterion 21).
#[must_use]
pub fn cell_variance(variance: &Variance, seed: u32, key: CellKey) -> CellVariance {
    let mut rng = Pcg32::new(u64::from(seed));
    rng.advance(key.index().wrapping_mul(DRAWS_PER_CELL));
    let draws: [f64; 7] = std::array::from_fn(|_| rng.next_unit());
    let scale = |percent: f64| (1.0 + percent / 100.0).max(MIN_SCALE_FACTOR);
    let scale_x = scale(value(percents(variance.scale_x), key, draws[1]));
    let scale_y = if variance.uniform_scale {
        scale_x
    } else {
        scale(value(percents(variance.scale_y), key, draws[2]))
    };
    CellVariance {
        rotation: Angle::from_radians(value(angles(variance.rotation), key, draws[0])),
        scale_x,
        scale_y,
        offset: Vec2::new(
            value(lengths(variance.offset_x), key, draws[3]),
            value(lengths(variance.offset_y), key, draws[4]),
        ),
        opacity: Percent::new(value(percents(variance.opacity), key, draws[5])),
        hue: Angle::from_radians(value(angles(variance.hue), key, draws[6])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_rotation_and_offset() -> Variance {
        let mut v = Variance::default();
        v.rotation.random = Angle::from_radians(0.5);
        v.offset_x.random = Length::from_mm(3.0);
        v
    }

    #[test]
    fn zero_variance_gives_no_change() {
        let cell = cell_variance(&Variance::default(), 42, CellKey { row: 3, column: 4 });
        assert_eq!(cell, CellVariance::NONE);
    }

    #[test]
    fn turning_one_channel_on_leaves_the_others_untouched() {
        let key = CellKey { row: 1, column: 2 };
        let before = cell_variance(&with_rotation_and_offset(), 7, key);
        let mut more = with_rotation_and_offset();
        more.hue.random = Angle::from_radians(1.0);
        let after = cell_variance(&more, 7, key);
        assert_eq!(before.rotation, after.rotation);
        assert_eq!(before.offset, after.offset);
        assert!(after.hue.as_radians() != 0.0);
    }

    #[test]
    fn the_draws_come_from_the_cells_own_stretch_of_the_stream() {
        let key = CellKey { row: 0, column: 3 };
        let mut rng = Pcg32::new(42);
        rng.advance(3 * DRAWS_PER_CELL);
        let unit = rng.next_unit();
        let cell = cell_variance(&with_rotation_and_offset(), 42, key);
        let expected = 2.0f64.mul_add(unit, -1.0) * 0.5;
        assert!((cell.rotation.as_radians() - expected).abs() < 1e-15);
    }

    #[test]
    fn steps_add_per_row_and_column_and_draw_nothing() {
        let mut v = Variance::default();
        v.offset_y.per_row = Length::from_mm(2.0);
        v.offset_y.per_column = Length::from_mm(-0.5);
        let cell = cell_variance(&v, 1, CellKey { row: 3, column: 4 });
        assert!((cell.offset.y - 4.0).abs() < 1e-12);
    }

    #[test]
    fn a_scale_never_falls_below_the_minimum_factor() {
        let mut v = Variance::default();
        v.scale_x.per_column = Percent::new(-60.0);
        let cell = cell_variance(&v, 1, CellKey { row: 0, column: 2 });
        assert!((cell.scale_x - MIN_SCALE_FACTOR).abs() < 1e-15);
        assert!((cell.scale_y - MIN_SCALE_FACTOR).abs() < 1e-15);
    }
}
