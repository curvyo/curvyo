//! How `GrblController` takes the feed hold, its resume and the alarm unlock.

use std::time::Duration;

use curvyo_connector_core::Event;

use crate::grbl::controller::GrblController;
use crate::grbl::home::Owed;
use crate::grbl::stream::Hold;

impl GrblController {
    /// `!` at once, or, while the job's `$H` runs (GRBL ignores `!` then), nothing yet: the hold
    /// is deferred to `$H`'s answer and core is told (`specs/0133` P4).
    pub(super) fn pause(&mut self, at: Duration) {
        if self.stop.is_some() {
            return;
        }
        let Some(stream) = self.stream.as_mut().filter(|s| s.hold == Hold::Running) else {
            return;
        };
        if stream.homing() {
            stream.hold = Hold::Deferred;
            self.events.push(Event::PauseDeferred);
        } else {
            self.out.push(crate::grbl::stop::HOLD);
            stream.hold = Hold::Requested(Some(at + self.timing.hold));
        }
    }

    /// `~` from a hold, sent or complete; a deferred pause is only cancelled.
    pub(super) fn resume(&mut self) {
        if self.stop.is_some() {
            return;
        }
        let Some(stream) = self.stream.as_mut() else {
            return;
        };
        match stream.hold {
            Hold::Deferred => stream.hold = Hold::Running,
            Hold::Held { .. } | Hold::Requested(None) => {
                self.out.push(b'~');
                stream.hold = Hold::Resuming;
            }
            Hold::Running | Hold::Requested(Some(_)) | Hold::Resuming => {}
        }
    }

    pub(super) fn unlock(&mut self) {
        if self.alarmed {
            self.out.extend_from_slice(b"$X\n");
            // The hub prints "Unlock sent by <client>"; G33 asks for no second line.
            self.owed.push_back(Owed::Line(3));
        }
    }
}
