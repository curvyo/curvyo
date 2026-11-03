//! Turns a read line into the fit's reading frame (`specs/0091-trace-text-recognition` slice 3):
//! an upside-down or steep line is matched as if it read left to right, and its texts are turned
//! back.

use curvyo_text_core::RecognisedRun;
use kurbo::{Affine, Vec2};

use crate::trace_fit::{FitLine, TraceFit, unscaled};

/// The turn that makes `line` read left to right in the fit's space (`specs/0091` slice 3): none
/// for a line within 45° of that, which the recogniser lays out along its own axis as before;
/// else the rotation of its direction onto the x axis, so an upside-down or steep line is matched
/// upright.
pub(crate) fn fit_turn(line: &FitLine) -> Affine {
    let d = line.direction;
    if d.x >= d.y.abs() {
        Affine::IDENTITY
    } else {
        Affine::rotate(-d.y.atan2(d.x))
    }
}

impl TraceFit {
    /// Line `source`'s direction in the fit's space.
    pub(crate) fn direction(&self, source: usize) -> Vec2 {
        (self.turns[source] * self.lines[source].direction.to_point()).to_vec2()
    }

    /// `run`, found for line `source` in the fit's space, in document mm.
    pub(crate) fn in_document(&self, source: usize, run: RecognisedRun) -> RecognisedRun {
        let mut run = unscaled(run, self.scales[source]);
        run.placement = self.turns[source].inverse() * run.placement;
        run
    }
}
