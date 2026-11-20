//! How long a `$H` in flight may leave GRBL silent (`specs/0129`, `specs/0142`): the loss clock
//! is paused, the homing cap bounds the pause.

use std::time::Duration;

use curvyo_connector_core::Event;

use crate::grbl::controller::GrblController;
use crate::grbl::stream::Stream;
use crate::grbl::texts;

impl GrblController {
    /// While `$H` is in flight, a job's or the Home command's: GRBL 1.1's homing loop answers no
    /// `?` until the cycle ends (`specs/0129`, `specs/0118` M4), so the silence is no loss and the
    /// loss clock restarts when homing ends; a port error still is one, the stop of G12 keeps
    /// its own banner limit, and the homing cap ends a silence that lasts too long. In a job the
    /// state word reads `Home` meanwhile, as the reports of a GRBL that does answer show it; the
    /// Home command has a state of its own.
    pub(super) fn during_homing(&mut self, at: Duration) {
        let in_job = self.stream.as_ref().is_some_and(Stream::homing);
        if !in_job && !self.home_in_flight() {
            return;
        }
        self.last_report = at;
        if !in_job {
            return;
        }
        if self.word.as_deref() != Some("Home") {
            self.word = Some("Home".to_owned());
            self.notices.push("Controller state: Home".to_owned());
            self.events.push(Event::ControllerWord(self.word.clone()));
        }
    }

    /// Whether a `$H` waits for GRBL's answer under the cap: the job's, or the Home command's
    /// before an alarm answered it; never during a stop, which has its own limits (HC6).
    fn homing_unanswered(&self) -> bool {
        let in_job = self.stream.as_ref().is_some_and(Stream::homing);
        let home = self.home_in_flight() && !self.home_alarmed;
        self.stop.is_none() && (in_job || home)
    }

    /// At the end of a step at `at`: the cap's clock starts with the step that writes `$H` and is
    /// cleared once nothing waits for its answer (HC1).
    pub(super) fn note_homing(&mut self, at: Duration) {
        if !self.homing_unanswered() {
            self.homing_since = None;
        } else if self.homing_since.is_none() {
            self.homing_since = Some(at);
        }
    }

    /// Whether the cap ran out at `at`; then the link is given up with the safety bytes and the
    /// cap's sentence, as every loss is (HC3).
    pub(super) fn homing_capped(&mut self, at: Duration) -> bool {
        let cap = self.timing.homing;
        let expired =
            self.homing_unanswered() && self.homing_since.is_some_and(|since| at >= since + cap);
        if expired {
            let detail = texts::homing_cap(cap);
            self.events.push(Event::ControllerLostWith {
                detail: detail.clone(),
            });
            self.give_up(detail, true);
        }
        expired
    }
}
