//! The frame's start for GRBL (`specs/0118` 2b, adrs F6): the words are checked again, a homed
//! machine's travel is checked before the first byte, then the lines stream as a job's do.

use std::time::Duration;

use curvyo_connector_core::{Event, Job, Millimetres, check_frame};

use crate::grbl::controller::{BUSY_WORDS, GrblController};
use crate::grbl::prepare::prepare;
use crate::grbl::texts;

/// The sentence of a frame the travel does not hold; `over` is the largest miss in millimetres.
#[must_use]
pub fn outside_travel(over: f64) -> String {
    format!("Outside the travel by {} mm", over.ceil().max(1.0))
}

impl GrblController {
    /// `ControllerCommand::Frame`: streams `job` (the frame, never the held job) with the laser
    /// off. A laser mode of `$32=0` or unknown does not refuse it: the frame holds no `M3`, `M4`
    /// or `S` above 0. Failures end the frame through `JobFailedWith`; no byte is written then.
    pub(super) fn frame(&mut self, job: Option<&Job>, homed: bool, at: Duration) {
        let Some(job) = job else {
            return;
        };
        if self.stop.is_some() {
            return;
        }
        let fail = |this: &mut Self, detail: String| {
            this.events.push(Event::JobFailedWith {
                elapsed_ms: 0,
                detail,
            });
        };
        if let Some(word) = self
            .word
            .as_deref()
            .filter(|word| BUSY_WORDS.contains(word))
        {
            let detail = format!("Controller state: {word}");
            return fail(self, detail);
        }
        // Second guard behind the job machine: a frame may hold nothing that burns.
        let Ok(moves) = check_frame(&job.text) else {
            return fail(self, "Not a frame".to_owned());
        };
        if let (true, Some(within), Some(mpos), true) =
            (homed, self.travel_box(), self.mpos, self.wco_seen)
            && let Err(over) = moves.fits(
                &within,
                [Millimetres(mpos[0]), Millimetres(mpos[1])],
                [Millimetres(self.wco[0]), Millimetres(self.wco[1])],
            )
        {
            return fail(self, outside_travel(over.0));
        }
        match prepare(&job.text) {
            Err(refused) => {
                let detail = texts::failed_at(refused.line, refused.cause.text());
                fail(self, detail);
            }
            Ok(lines) => self.begin_stream(lines, at),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_miss_is_named_in_whole_millimetres_rounded_up() {
        assert_eq!(outside_travel(0.2), "Outside the travel by 1 mm");
        assert_eq!(outside_travel(11.1), "Outside the travel by 12 mm");
        assert_eq!(outside_travel(12.0), "Outside the travel by 12 mm");
    }
}
