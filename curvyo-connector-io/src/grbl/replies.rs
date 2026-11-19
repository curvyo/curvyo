//! How `GrblController` reads GRBL's replies once identified: status reports (position, state
//! words, hold, alarm, completion), answers to lines, alarms and banners.

use std::time::Duration;

use curvyo_connector_core::{Event, Millimetres};

use crate::grbl::controller::GrblController;
use crate::grbl::home::Owed;
use crate::grbl::identify::RESET;
use crate::grbl::reply::Report;
use crate::grbl::stream::Hold;
use crate::grbl::texts;

/// GRBL 1.1's real-time Toggle Spindle Stop, accepted only in a completed hold (addendum C7).
pub const SPINDLE_STOP: u8 = 0x9E;
/// The tolerance below which a position counts as unchanged, in millimetres.
const SAME_POSITION_MM: f64 = 1e-6;

/// GRBL's alarm for a reset while moving.
const RESET_WHILE_MOVING: u16 = 3;

/// GRBL 1.1's `Bf:` count may be one low when its ring buffer has wrapped, so one byte more than
/// the connector has in flight is no foreign answer.
const BF_SLACK: usize = 1;

/// The states of GRBL that map to the job machine's own states (G4).
const OWN_STATES: [&str; 4] = ["Idle", "Run", "Hold", "Alarm"];

impl GrblController {
    fn alarm_words(&self) -> String {
        match self.alarm_code {
            Some(code) => texts::alarm_text(code),
            None if self.locked => texts::LOCKED_NOT_HOMED.to_owned(),
            None => texts::LOCKED.to_owned(),
        }
    }

    fn update_position(&mut self, report: &Report) {
        if let Some(wco) = report.wco {
            self.wco = wco.map(|v| self.settings.to_mm(v));
        }
        let work = match (report.wpos, report.mpos) {
            (Some(w), _) => Some(w.map(|v| self.settings.to_mm(v))),
            (None, Some(m)) => {
                let m = m.map(|v| self.settings.to_mm(v));
                Some([m[0] - self.wco[0], m[1] - self.wco[1], m[2] - self.wco[2]])
            }
            (None, None) => None,
        };
        let Some(work) = work else {
            return;
        };
        let changed = self.position.is_none_or(|old| {
            old.iter()
                .zip(work.iter())
                .any(|(a, b)| (a - b).abs() > SAME_POSITION_MM)
        });
        if changed {
            self.position = Some(work);
            self.events.push(Event::Position(work.map(Millimetres)));
        }
    }

    fn update_word(&mut self, report: &Report) {
        let word = (!OWN_STATES.contains(&report.state.as_str())).then(|| report.state.clone());
        if word != self.word {
            if let Some(w) = &word {
                self.notices.push(format!("Controller state: {w}"));
            }
            self.word.clone_from(&word);
            self.events.push(Event::ControllerWord(word));
        }
    }

    /// A status report.
    pub(super) fn on_report(&mut self, report: &Report, at: Duration) {
        self.last_report = at;
        self.reports += 1;
        self.update_position(report);
        self.update_word(report);
        if report.state == "Alarm" && !self.alarmed {
            self.raise_alarm();
        } else if report.state == "Idle" && self.alarmed {
            self.alarmed = false;
            self.alarm_code = None;
            self.locked = false;
            self.events.push(Event::AlarmCleared);
            self.notices.push("Alarm cleared".to_owned());
        }
        if let Some(stop) = self.stop.as_mut() {
            stop.on_report(report, at, &mut self.out);
            return;
        }
        if self.buffer_disagrees(report) {
            // GRBL holds more than the connector sent and was not answered for: an `ok` it
            // did not ask for was taken as a line's answer (G28's second guard).
            self.begin_stop(Some(texts::UNEXPECTED_ANSWER.to_owned()), at);
            return;
        }
        self.on_job_report(report);
    }

    /// Whether `Bf:` shows more bytes in GRBL's receive buffer than the job's lines in flight
    /// plus the owed `M5`, `M9` and `$X`. Every byte GRBL holds was written before the report
    /// was read, and an answer read before it freed its line, so the count can only be lower.
    fn buffer_disagrees(&self, report: &Report) -> bool {
        let (Some(free), Some(stream)) = (report.rx_free, self.stream.as_ref()) else {
            return false;
        };
        let held = self.rx_size.saturating_sub(free);
        held > stream.in_flight() + self.owed_bytes() + BF_SLACK
    }

    fn on_job_report(&mut self, report: &Report) {
        let laser_off = self.settings.laser_mode() == Some(false);
        let reports = self.reports;
        let Some(stream) = self.stream.as_mut() else {
            return;
        };
        match stream.hold {
            Hold::Running | Hold::Requested(_) if report.hold_complete() => {
                // Asked for or not (a hold button on the machine): the laser must be off. With
                // `$32=0` GRBL leaves the spindle on in a hold, and a build without
                // `DISABLE_LASER_DURING_HOLD` does with `$32=1`, which `A:` shows: the connector
                // stops it. Never when GRBL turned it off itself: the toggle would turn it on.
                let stop = laser_off || report.spindle_on();
                if stop {
                    self.out.push(SPINDLE_STOP);
                }
                stream.hold = Hold::Held {
                    spindle_stopped: stop,
                };
                self.events.push(Event::Held);
            }
            Hold::Held {
                spindle_stopped: false,
            } if report.spindle_on() => {
                // `A:` comes now and then, not in every report: the first that shows it on.
                self.out.push(SPINDLE_STOP);
                stream.hold = Hold::Held {
                    spindle_stopped: true,
                };
            }
            Hold::Resuming if report.state == "Run" || report.state == "Idle" => {
                stream.hold = Hold::Running;
                self.events.push(Event::Resumed);
            }
            _ => {}
        }
        let complete = stream.hold == Hold::Running
            && stream.all_answered()
            && report.state == "Idle"
            && stream.finished_at_report.is_some_and(|r| reports > r);
        if complete {
            let elapsed_ms = stream.elapsed_ms();
            self.stream = None;
            self.events.push(Event::JobFinished { elapsed_ms });
        }
    }

    /// `ok` (`code` `None`) or `error:<code>`.
    pub(super) fn on_answer(&mut self, code: Option<u16>, at: Duration) {
        if let Some(owed) = self.owed.pop_front() {
            if owed == Owed::Home {
                self.on_home_answer(code);
            }
            return;
        }
        let stopping = self.stop.is_some();
        let reports = self.reports;
        let Some(stream) = self.stream.as_mut() else {
            return;
        };
        let Some(job_line) = stream.answer() else {
            if !stopping {
                // A foreign answer would let the count overflow GRBL's buffer (G28).
                self.begin_stop(Some(texts::UNEXPECTED_ANSWER.to_owned()), at);
            }
            return;
        };
        let (line, elapsed_ms) = (stream.answered(), stream.elapsed_ms());
        if stream.all_answered() {
            stream.finished_at_report = Some(reports);
        }
        self.events.push(Event::Progress { line, elapsed_ms });
        if let Some(code) = code.filter(|_| !stopping) {
            let detail = texts::failed_at(job_line, &texts::error_text(code));
            self.begin_stop(Some(detail), at);
        }
    }

    /// `ALARM:<n>`: the first alarm's number stands, except that a homing alarm while the Home
    /// command's `$H` is in flight supersedes the power-up lock (`specs/0118` M4).
    pub(super) fn on_alarm_line(&mut self, code: u16) {
        let homing = self.home_in_flight();
        if homing {
            self.home_alarmed = true;
        }
        if self.alarmed && !homing {
            return;
        }
        self.alarm_code = Some(code);
        if code == RESET_WHILE_MOVING
            && self.stop.is_none()
            && let Some(stream) = self.stream.take()
        {
            // Alarm 3 comes only from a reset while moving: one nobody here asked for (the
            // machine's reset button, G29). Its banner follows.
            self.events.push(Event::JobFailedWith {
                elapsed_ms: stream.elapsed_ms(),
                detail: texts::reset_at(stream.answered()),
            });
            self.expect_banner = true;
        }
        self.raise_alarm();
    }

    fn raise_alarm(&mut self) {
        self.alarmed = true;
        if self.stop.is_some() {
            self.deferred_alarm = true;
            return;
        }
        if self.stream.take().is_some() {
            // Every unsent line is discarded; the reset switches the spindle off (G13).
            self.out.push(RESET);
            self.expect_banner = true;
        }
        let text = self.alarm_words();
        self.events.push(Event::Alarm { text });
    }

    /// A banner: the stop's reset, the reset after an alarm, or a reset nobody here asked for.
    pub(super) fn on_banner(&mut self) {
        self.owed.clear();
        self.locked = false;
        if self.stop.is_some() {
            // The stop's reset, or any reset while a stop waits (even in its hold step): the
            // controller restarted, so the stop is done.
            self.finish_stop();
        } else if self.expect_banner {
            self.expect_banner = false;
        } else if let Some(stream) = self.stream.take() {
            let detail = texts::reset_at(stream.answered());
            self.events.push(Event::JobFailedWith {
                elapsed_ms: stream.elapsed_ms(),
                detail,
            });
        }
    }

    fn finish_stop(&mut self) {
        let Some(stop) = self.stop.take() else {
            return;
        };
        self.out.extend_from_slice(b"M5\nM9\n");
        self.owed.extend([Owed::Line(3), Owed::Line(3)]);
        let elapsed_ms = self.stream.take().map_or(0, |s| s.elapsed_ms());
        self.events.push(match stop.failure {
            Some(detail) => Event::JobFailedWith { elapsed_ms, detail },
            None => Event::Stopped,
        });
        if std::mem::take(&mut self.deferred_alarm) {
            let text = self.alarm_words();
            self.events.push(Event::Alarm { text });
        }
    }
}
