//! The stitch list writer of the plan (`specs/0160-embroidery` criteria 10, 15, `adrs.md` EM3,
//! EM10): jumps from the hoop centre, trims, jumps and colour changes between blocks, and every
//! step longer than the limit split into equal parts of whole tenths.

use crate::operation::ThreadColour;
use crate::profile::DST_MAX_STITCH;
use crate::stitch::{Stitch, StitchKind, StitchPoint, TenthMm};

/// A travel between two blocks of one colour longer than this is a trim (Proposal: 5 mm).
pub const TRIM_ABOVE: TenthMm = TenthMm::new(50);

/// The points that split the step `from` to `to` into equal parts no longer than `max` in X or
/// Y, each rounded to whole tenths, `to` last (EM3: the parts sum exactly).
fn split(from: StitchPoint, to: StitchPoint, max: TenthMm) -> Vec<StitchPoint> {
    let (dx, dy) = (to.x.get() - from.x.get(), to.y.get() - from.y.get());
    let longest = dx.unsigned_abs().max(dy.unsigned_abs());
    let max = max.get().unsigned_abs().max(1);
    let parts = longest.div_ceil(max).max(1);
    (1..=parts)
        .map(|k| {
            let at = |a: i32, d: i32| {
                let t = f64::from(k) / f64::from(parts);
                TenthMm::of_mm((f64::from(a) + f64::from(d) * t) / 10.0)
            };
            StitchPoint {
                x: at(from.x.get(), dx),
                y: at(from.y.get(), dy),
            }
        })
        .collect()
}

/// Builds the stitch list block by block.
pub(crate) struct Emitter {
    stitches: Vec<Stitch>,
    at: StitchPoint,
    thread: Option<ThreadColour>,
    max_stitch: TenthMm,
}

impl Emitter {
    /// A list that starts at the hoop centre; no stitch longer than `max_stitch` (at most the DST
    /// limit).
    pub(crate) fn new(max_stitch: TenthMm) -> Self {
        Self {
            stitches: Vec::new(),
            at: StitchPoint::ORIGIN,
            thread: None,
            max_stitch: max_stitch.min(DST_MAX_STITCH),
        }
    }

    /// How many stitches are written.
    pub(crate) fn len(&self) -> usize {
        self.stitches.len()
    }

    fn push(&mut self, at: StitchPoint, kind: StitchKind) {
        self.stitches.push(Stitch { at, kind });
        self.at = at;
    }

    fn travel(&mut self, to: StitchPoint) {
        if to == self.at {
            return;
        }
        for point in split(self.at, to, DST_MAX_STITCH) {
            self.push(point, StitchKind::Jump);
        }
    }

    /// Starts a block at `start` in `thread` (criterion 10): a colour change before a new
    /// colour, a trim before a travel over 5 mm in the same colour, the jumps there, and the
    /// first needle point at `start`.
    pub(crate) fn enter(&mut self, start: StitchPoint, thread: ThreadColour) {
        match self.thread {
            Some(current) if current != thread => self.push(self.at, StitchKind::ColourChange),
            Some(_) if self.at.distance_mm(start) > TRIM_ABOVE.mm() => {
                self.push(self.at, StitchKind::Trim);
            }
            _ => {}
        }
        self.thread = Some(thread);
        self.travel(start);
        self.push(start, StitchKind::Normal);
    }

    /// Sews to each of `points` in turn, a repeat of the current point left out.
    pub(crate) fn sew(&mut self, points: &[StitchPoint]) {
        for &to in points {
            if to == self.at {
                continue;
            }
            for point in split(self.at, to, self.max_stitch) {
                self.push(point, StitchKind::Normal);
            }
        }
    }

    /// The list with its end record.
    pub(crate) fn finish(mut self) -> Vec<Stitch> {
        if !self.stitches.is_empty() {
            self.push(self.at, StitchKind::End);
        }
        self.stitches
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(stitches: &[Stitch]) -> Vec<StitchKind> {
        stitches.iter().map(|s| s.kind).collect()
    }

    #[test]
    fn a_30_mm_travel_is_three_jumps_and_a_long_stitch_is_split() {
        use StitchKind::{End, Jump, Normal};
        let mut e = Emitter::new(DST_MAX_STITCH);
        let red = ThreadColour::rgb(200, 0, 0);
        e.enter(StitchPoint::new(300, 0), red);
        e.sew(&[StitchPoint::new(300, 250)]);
        let s = e.finish();
        assert_eq!(
            kinds(&s),
            [Jump, Jump, Jump, Normal, Normal, Normal, Normal, End]
        );
        assert_eq!(s[0].at, StitchPoint::new(100, 0), "criterion 15");
        for pair in s.windows(2) {
            let d = (pair[1].at.x.get() - pair[0].at.x.get()).abs();
            assert!(d <= 121 && (pair[1].at.y.get() - pair[0].at.y.get()).abs() <= 121);
        }
    }

    #[test]
    fn a_long_travel_trims_a_short_one_jumps_and_a_new_colour_changes() {
        use StitchKind::{ColourChange, End, Jump, Normal, Trim};
        let red = ThreadColour::rgb(200, 0, 0);
        let blue = ThreadColour::rgb(0, 0, 200);
        let mut e = Emitter::new(DST_MAX_STITCH);
        e.enter(StitchPoint::new(0, 0), red);
        e.sew(&[StitchPoint::new(10, 0)]);
        e.enter(StitchPoint::new(40, 0), red);
        e.enter(StitchPoint::new(140, 0), red);
        e.enter(StitchPoint::new(150, 0), blue);
        assert_eq!(
            kinds(&e.finish()),
            [
                Normal,
                Normal,
                Jump,
                Normal,
                Trim,
                Jump,
                Normal,
                ColourChange,
                Jump,
                Normal,
                End
            ]
        );
    }
}
