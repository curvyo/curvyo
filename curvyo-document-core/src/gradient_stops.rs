//! The edits of a gradient's stop list (`specs/0124-gradient-fill` criteria 3 and 6): add, remove,
//! move, recolour and reverse, each returning a new valid list.

use crate::gradient_eval::PremultipliedRgba8;
use crate::gradient_model::{GradientStop, GradientStops, StopOffset};
use crate::path_model::Color;

/// A premultiplied colour back to a colour and its alpha, rounding half up; a clear colour
/// reads as clear black.
fn unpremultiplied(c: PremultipliedRgba8) -> (Color, u8) {
    if c.a == 0 {
        return (Color::BLACK, 0);
    }
    let a = u32::from(c.a);
    let channel = |v: u8| {
        // invariant: `v <= a`, so the quotient is at most 255.
        #[allow(clippy::cast_possible_truncation)]
        let byte = ((2 * u32::from(v) * 255 + a) / (2 * a)).min(255) as u8;
        byte
    };
    (
        Color {
            r: channel(c.r),
            g: channel(c.g),
            b: channel(c.b),
        },
        c.a,
    )
}

/// Two gaps that differ by no more than this are equal for Add stop.
const EQUAL_GAP: f64 = 1e-9;

impl GradientStops {
    /// A list built from stops that already satisfy the rules.
    fn from_valid(stops: Vec<GradientStop>) -> Self {
        // invariant: every edit below keeps two or more stops, the ends at 0 and 1 and the
        // positions ordered.
        #[allow(clippy::expect_used)]
        Self::new(stops).expect("a stop edit keeps the list valid")
    }

    /// The order and the positions mirrored: the first stop becomes the last (criterion 3).
    #[must_use]
    pub fn reversed(&self) -> Self {
        let stops = self
            .as_slice()
            .iter()
            .rev()
            .map(|stop| GradientStop {
                offset: StopOffset::clamped(
                    1.0 - stop.offset.get(),
                    StopOffset::START,
                    StopOffset::END,
                ),
                ..*stop
            })
            .collect();
        Self::from_valid(stops)
    }

    /// The middle of the widest gap between two neighbouring stops, the first of equal gaps
    /// (the panel's Add stop).
    #[must_use]
    pub fn widest_gap_middle(&self) -> StopOffset {
        let mut best = (0.0, 0.5);
        for pair in self.as_slice().windows(2) {
            let (lo, hi) = (pair[0].offset.get(), pair[1].offset.get());
            // Gaps within rounding of each other are equal, and the first of them wins.
            if hi - lo > best.0 + EQUAL_GAP {
                best = (hi - lo, f64::midpoint(lo, hi));
            }
        }
        StopOffset::clamped(best.1, StopOffset::START, StopOffset::END)
    }

    /// A new inner stop at `offset` in the shade's colour there; returns the list and the new
    /// stop's index. The stop goes after every stop at or before `offset`, but never after the
    /// last one.
    #[must_use]
    pub fn with_stop_added(&self, offset: StopOffset) -> (Self, usize) {
        let (colour, alpha) = unpremultiplied(self.colour_at(offset.get()));
        let stops = self.as_slice();
        let index = stops
            .partition_point(|stop| stop.offset <= offset)
            .clamp(1, stops.len() - 1);
        let mut list = stops.to_vec();
        list.insert(
            index,
            GradientStop {
                colour,
                alpha,
                offset,
            },
        );
        (Self::from_valid(list), index)
    }

    /// The list without inner stop `index`; `None` for an end stop or an index past the list.
    #[must_use]
    pub fn without_stop(&self, index: usize) -> Option<Self> {
        if index == 0 || index + 1 >= self.len() {
            return None;
        }
        let mut list = self.as_slice().to_vec();
        list.remove(index);
        Some(Self::from_valid(list))
    }

    /// Stop `index` in a new colour and alpha; `None` for an index past the list.
    #[must_use]
    pub fn with_stop_colour(&self, index: usize, colour: Color, alpha: u8) -> Option<Self> {
        let mut list = self.as_slice().to_vec();
        let stop = list.get_mut(index)?;
        stop.colour = colour;
        stop.alpha = alpha;
        Some(Self::from_valid(list))
    }

    /// Inner stop `index` moved to `offset`, clamped between its neighbours so stops never
    /// reorder; the end stops stay at 0 and 1. `None` for an index past the list.
    #[must_use]
    pub fn with_stop_offset(&self, index: usize, offset: f64) -> Option<Self> {
        let stops = self.as_slice();
        stops.get(index)?;
        if index == 0 || index + 1 == stops.len() {
            return Some(self.clone());
        }
        let mut list = stops.to_vec();
        list[index].offset =
            StopOffset::clamped(offset, stops[index - 1].offset, stops[index + 1].offset);
        Some(Self::from_valid(list))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stop(r: u8, alpha: u8, offset: f64) -> GradientStop {
        GradientStop {
            colour: Color { r, g: 0, b: 0 },
            alpha,
            offset: StopOffset::new(offset).unwrap(),
        }
    }

    fn list(stops: &[(u8, f64)]) -> GradientStops {
        GradientStops::new(stops.iter().map(|&(r, o)| stop(r, 255, o)).collect()).unwrap()
    }

    fn offsets(stops: &GradientStops) -> Vec<f64> {
        stops.as_slice().iter().map(|s| s.offset.get()).collect()
    }

    #[test]
    fn reverse_mirrors_the_order_and_the_positions() {
        let reversed = list(&[(1, 0.0), (2, 0.25), (3, 1.0)]).reversed();
        assert_eq!(offsets(&reversed), [0.0, 0.75, 1.0]);
        let reds: Vec<u8> = reversed.as_slice().iter().map(|s| s.colour.r).collect();
        assert_eq!(reds, [3, 2, 1]);
        assert_eq!(reversed.reversed(), list(&[(1, 0.0), (2, 0.25), (3, 1.0)]));
    }

    #[test]
    fn add_goes_into_the_first_widest_gap_in_the_shade_colour() {
        let stops = list(&[(0, 0.0), (200, 0.2), (100, 0.6), (0, 1.0)]);
        assert!((stops.widest_gap_middle().get() - 0.4).abs() < 1e-12);
        let (added, index) = stops.with_stop_added(stops.widest_gap_middle());
        assert_eq!(index, 2);
        assert_eq!(added.as_slice()[2].colour.r, 150);
        let two = list(&[(0, 0.0), (255, 1.0)]);
        let (added, index) = two.with_stop_added(two.widest_gap_middle());
        assert_eq!((index, added.len()), (1, 3));
    }

    #[test]
    fn a_stop_added_at_an_end_stays_inner() {
        let two = list(&[(0, 0.0), (255, 1.0)]);
        let (at_end, index) = two.with_stop_added(StopOffset::END);
        assert_eq!(index, 1);
        assert_eq!(offsets(&at_end), [0.0, 1.0, 1.0]);
        let (at_start, index) = two.with_stop_added(StopOffset::START);
        assert_eq!(index, 1);
        assert_eq!(offsets(&at_start), [0.0, 0.0, 1.0]);
    }

    #[test]
    fn only_inner_stops_are_removed() {
        let stops = list(&[(0, 0.0), (5, 0.5), (0, 1.0)]);
        assert_eq!(stops.without_stop(0), None);
        assert_eq!(stops.without_stop(2), None);
        assert_eq!(stops.without_stop(9), None);
        assert_eq!(offsets(&stops.without_stop(1).unwrap()), [0.0, 1.0]);
    }

    #[test]
    fn a_position_is_clamped_between_its_neighbours() {
        let stops = list(&[(0, 0.0), (5, 0.3), (6, 0.6), (0, 1.0)]);
        assert_eq!(
            offsets(&stops.with_stop_offset(1, 0.9).unwrap()),
            [0.0, 0.6, 0.6, 1.0]
        );
        assert_eq!(
            offsets(&stops.with_stop_offset(2, -1.0).unwrap()),
            [0.0, 0.3, 0.3, 1.0]
        );
        assert_eq!(
            stops.with_stop_offset(0, 0.5).unwrap(),
            stops,
            "the ends stay"
        );
        assert_eq!(stops.with_stop_offset(4, 0.5), None);
    }

    #[test]
    fn a_stop_takes_a_new_colour_and_alpha() {
        let stops = list(&[(0, 0.0), (0, 1.0)]);
        let red = Color { r: 255, g: 0, b: 0 };
        let edited = stops.with_stop_colour(1, red, 7).unwrap();
        assert_eq!(
            (edited.as_slice()[1].colour, edited.as_slice()[1].alpha),
            (red, 7)
        );
        assert_eq!(stops.with_stop_colour(2, red, 7), None);
    }

    #[test]
    fn unpremultiplying_recovers_the_colour() {
        let c = PremultipliedRgba8 {
            r: 64,
            g: 0,
            b: 128,
            a: 128,
        };
        assert_eq!(
            unpremultiplied(c),
            (
                Color {
                    r: 128,
                    g: 0,
                    b: 255
                },
                128
            )
        );
    }
}
