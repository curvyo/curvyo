//! The summary of a stitch list (`specs/0160-embroidery` criterion 12, `adrs.md` EM6): stitches,
//! colours, trims, thread length and time at the machine speed.

use crate::profile::StitchesPerMin;
use crate::stitch::{Stitch, StitchKind, StitchPoint};

/// What the output group's total shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Summary {
    /// Needle points.
    pub stitches: usize,
    /// Thread colours: the colour changes plus one, none without stitches.
    pub colours: usize,
    /// Trims.
    pub trims: usize,
    /// The thread length in mm: the lengths of the stitches summed.
    pub thread_mm: f64,
    /// The time in seconds: the needle points at `speed`.
    pub seconds: f64,
}

/// The summary of `stitches` at `speed` stitches per minute.
#[must_use]
pub fn summarise(stitches: &[Stitch], speed: StitchesPerMin) -> Summary {
    let mut at = StitchPoint::ORIGIN;
    let mut summary = Summary {
        stitches: 0,
        colours: 0,
        trims: 0,
        thread_mm: 0.0,
        seconds: 0.0,
    };
    let mut changes = 0;
    for stitch in stitches {
        match stitch.kind {
            StitchKind::Normal => {
                summary.stitches += 1;
                summary.thread_mm += at.distance_mm(stitch.at);
            }
            StitchKind::Trim => summary.trims += 1,
            StitchKind::ColourChange => changes += 1,
            StitchKind::Jump | StitchKind::End => {}
        }
        at = stitch.at;
    }
    if summary.stitches > 0 {
        summary.colours = changes + 1;
    }
    #[allow(clippy::cast_precision_loss)] // far below 2^52 stitches
    let count = summary.stitches as f64;
    summary.seconds = count * 60.0 / f64::from(speed.get());
    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stitches_colours_trims_thread_and_time_are_counted() {
        let s = |x, y, kind| Stitch {
            at: StitchPoint::new(x, y),
            kind,
        };
        let list = [
            s(0, 0, StitchKind::Normal),
            s(30, 40, StitchKind::Normal),
            s(30, 40, StitchKind::Trim),
            s(100, 40, StitchKind::Jump),
            s(100, 40, StitchKind::ColourChange),
            s(100, 50, StitchKind::Normal),
            s(100, 50, StitchKind::End),
        ];
        let summary = summarise(&list, StitchesPerMin::new(600).unwrap());
        assert_eq!(summary.stitches, 3);
        assert_eq!(summary.colours, 2);
        assert_eq!(summary.trims, 1);
        assert!((summary.thread_mm - 6.0).abs() < 1e-9);
        assert!((summary.seconds - 0.3).abs() < 1e-9);
    }
}
