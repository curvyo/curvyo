//! The `$H` the Home command writes, from the write to its answer (`specs/0118` M4).

use curvyo_connector_core::Event;

use crate::grbl::controller::GrblController;
use crate::grbl::texts;

/// The Home command's line: GRBL's homing cycle.
const HOME_LINE: &[u8] = b"$H\n";

/// A line outside a job's stream whose answer GRBL owes, oldest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owed {
    /// `M5`, `M9` or `$X`, with its bytes.
    Line(usize),
    /// The Home command's `$H`.
    Home,
}

impl Owed {
    /// The bytes the line takes in GRBL's receive buffer.
    #[must_use]
    pub fn bytes(self) -> usize {
        match self {
            Self::Line(bytes) => bytes,
            Self::Home => HOME_LINE.len(),
        }
    }
}

impl GrblController {
    /// Writes `$H` once, alone, never while a job streams or a stop runs (G14: only on the
    /// command). No `$X` first: GRBL 1.1 takes `$H` in Idle and in Alarm.
    pub(super) fn home(&mut self) {
        if self.stream.is_some() || self.stop.is_some() || self.home_in_flight() {
            return;
        }
        self.out.extend_from_slice(HOME_LINE);
        self.owed.push_back(Owed::Home);
        self.home_alarmed = false;
    }

    /// Whether the Home command's `$H` waits for its answer.
    pub(super) fn home_in_flight(&self) -> bool {
        self.owed.contains(&Owed::Home)
    }

    /// GRBL's answer to the Home command's `$H`: `ok` ends the homing and the alarm lock with it,
    /// `error:<n>` leaves GRBL where it was; the `ok` GRBL 1.1f sends after a homing alarm is
    /// consumed without `Homed`.
    pub(super) fn on_home_answer(&mut self, code: Option<u16>) {
        if self.word.as_deref() == Some("Home") {
            // The cycle is over: its word must not outlast it until the next report.
            self.word = None;
            self.events.push(Event::ControllerWord(None));
        }
        if std::mem::take(&mut self.home_alarmed) {
            return;
        }
        match code {
            None => {
                self.alarmed = false;
                self.alarm_code = None;
                self.locked = false;
                self.events.push(Event::Homed);
            }
            Some(code) => self.events.push(Event::HomingFailed {
                detail: texts::error_text(code),
            }),
        }
    }
}
