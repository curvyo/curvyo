//! The `$J=` lines and the jog cancel of GRBL, from the press to Idle (`specs/0118` adrs J6):
//! the laser is guaranteed off before the first move, a step is one clamped line, a hold is a
//! row of short increments that never queue more than a quarter of a second of motion, and the
//! cancel (`0x85`) is repeated until GRBL stands with nothing left in its buffer.

use std::time::Duration;

use curvyo_connector_core::{
    ControllerCommand, EDGE, Event, JogAxis, JogDirection, Millimetres, TravelBox, clamp,
};

use crate::grbl::controller::GrblController;
use crate::grbl::home::Owed;
use crate::grbl::reply::Report;
use crate::grbl::texts;

/// GRBL's real-time jog cancel.
pub const JOG_CANCEL: u8 = 0x85;
/// How long a cancel may take before the stop of G12 ends the jog.
pub const CANCEL_LIMIT: Duration = Duration::from_secs(2);
/// The most motion a hold has queued: a connector that dies stops the head within it.
pub const HOLD_AHEAD: Duration = Duration::from_millis(250);
/// The motion of one hold increment.
pub const HOLD_SLICE: Duration = Duration::from_millis(100);
/// What the laser guarantee writes before the first move.
const LASER_OFF: &[u8] = b"M5\nM9\n";

/// The axis, way and feed of a held jog.
#[derive(Debug, Clone, Copy)]
struct Held {
    axis: JogAxis,
    direction: JogDirection,
    feed: f64,
}

/// The jog GRBL runs for the job machine.
#[derive(Debug, Clone)]
pub struct Jog {
    hold: Option<Held>,
    /// Where the last written line ends, machine X and Y, when the position is known.
    planned: Option<[f64; 2]>,
    /// When the motion written so far ends, by the nominal feed.
    busy_until: Duration,
    /// When the cancel went out.
    cancelling: Option<Duration>,
    /// The stop of G12 was started because the cancel did not settle.
    escalated: bool,
    /// The sentence of a line GRBL refused; reported when the head stands.
    failure: Option<String>,
    /// The report count when GRBL last answered a `$J=` line: the end needs a later report.
    answered_at: Option<u64>,
    /// The controller has been homed since it was identified, as the machine said.
    homed: bool,
}

impl Jog {
    fn new(homed: bool) -> Self {
        Self {
            hold: None,
            planned: None,
            busy_until: Duration::ZERO,
            cancelling: None,
            escalated: false,
            failure: None,
            answered_at: None,
            homed,
        }
    }
}

impl GrblController {
    /// The jog commands of the job machine; events that follow at once are queued.
    pub(super) fn jog_command(&mut self, command: &ControllerCommand, at: Duration) {
        match *command {
            ControllerCommand::Jog {
                axis,
                direction,
                distance,
                feed,
                homed,
            } => self.jog_step(axis, direction, distance.0, (feed.0, homed), at),
            ControllerCommand::JogHold {
                axis,
                direction,
                feed,
                homed,
            } => self.jog_hold(axis, direction, (feed.0, homed), at),
            ControllerCommand::JogCancel => self.jog_cancel(at),
            _ => {}
        }
    }

    /// Whether another motion or a stop owns the link, so no jog may be written.
    fn jog_blocked(&self) -> bool {
        self.stream.is_some()
            || self.stop.is_some()
            || self.home_in_flight()
            || self.alarmed
            || self.pulse.is_some()
    }

    fn capped(&self, axis: JogAxis, feed: f64) -> f64 {
        let cap = self.settings.max_rate(axis).unwrap_or(f64::INFINITY);
        feed.min(cap).round().max(1.0)
    }

    fn jog_step(
        &mut self,
        axis: JogAxis,
        direction: JogDirection,
        distance: f64,
        (feed, homed): (f64, bool),
        at: Duration,
    ) {
        if self.jog_blocked() {
            self.events.push(Event::JogEnded);
            return;
        }
        let feed = self.capped(axis, feed);
        let run = self.jog.get_or_insert_with(|| Jog::new(homed));
        if run.cancelling.is_some() {
            return;
        }
        run.homed = homed;
        let written = self.write_jog(axis, direction, distance, feed, at);
        if !written && self.nothing_to_wait_for() {
            self.jog = None;
            self.events.push(Event::JogEnded);
        }
    }

    fn jog_hold(
        &mut self,
        axis: JogAxis,
        direction: JogDirection,
        (feed, homed): (f64, bool),
        at: Duration,
    ) {
        if self.jog_blocked() || self.jog.is_some() {
            self.events.push(Event::JogEnded);
            return;
        }
        let feed = self.capped(axis, feed);
        let mut run = Jog::new(homed);
        run.hold = Some(Held {
            axis,
            direction,
            feed,
        });
        self.jog = Some(run);
        self.pump_hold(at);
    }

    /// No line is owed and none ran: the jog has nothing left to end.
    fn nothing_to_wait_for(&self) -> bool {
        let owed = self.owed.iter().any(|o| matches!(o, Owed::Jog(_)));
        self.jog
            .as_ref()
            .is_some_and(|j| j.hold.is_none() && j.answered_at.is_none() && !owed)
    }

    fn jog_cancel(&mut self, at: Duration) {
        match self.jog.as_mut() {
            None => self.events.push(Event::JogEnded),
            Some(run) if run.cancelling.is_none() => {
                run.hold = None;
                run.cancelling = Some(at);
                self.out.push(JOG_CANCEL);
            }
            Some(_) => {}
        }
    }

    /// The travel box of the controller, when `$130` and `$131` are known: `[-travel, 0]`, or
    /// `[0, travel]` on an axis `$23` homes towards its positive end and that sat at 0 after
    /// homing (a build that forces the origin).
    pub(super) fn travel_box(&self) -> Option<TravelBox> {
        let travel = self.settings.travel()?;
        let mask = self.settings.homing_dir_mask();
        let positive = [0, 1].map(|i| {
            mask & (1 << i) != 0 && self.home_mpos.is_some_and(|home| home[i].abs() <= EDGE)
        });
        Some(TravelBox::new(travel, positive))
    }

    /// The laser guarantee: `M5` and `M9` go out before the first motion line unless they were
    /// the last spindle words the connector wrote (`specs/0118` criterion 9).
    fn laser_off_first(&mut self) {
        if !self.spindle_off {
            self.out.extend_from_slice(LASER_OFF);
            self.owed.extend([Owed::Line(3), Owed::Line(3)]);
            self.spindle_off = true;
        }
    }

    /// Writes one `$J=` line for `distance` mm along `axis`, clamped to the travel when homed;
    /// false when nothing was written (at the edge, or no room in GRBL's buffer).
    fn write_jog(
        &mut self,
        axis: JogAxis,
        direction: JogDirection,
        distance: f64,
        feed: f64,
        at: Duration,
    ) -> bool {
        let Some(run) = self.jog.as_ref() else {
            return false;
        };
        let base = run.planned.or_else(|| self.mpos.map(|m| [m[0], m[1]]));
        let delta = direction.sign() * distance;
        let allowed = match (run.homed, base, self.travel_box()) {
            (true, Some(base), Some(within)) => {
                clamp(base, axis, Millimetres(delta), &within).map(|m| m.0)
            }
            _ => Some(delta),
        };
        let Some(delta) = allowed else {
            return false;
        };
        let line = format!("$J=G91 G21 {}{delta:.3} F{feed:.0}\n", axis.letter());
        if self.owed_bytes() + LASER_OFF.len() + line.len() > self.rx_size {
            return false;
        }
        self.laser_off_first();
        self.out.extend_from_slice(line.as_bytes());
        self.owed.push_back(Owed::Jog(line.len()));
        let seconds = delta.abs() / (feed / 60.0);
        if let Some(run) = self.jog.as_mut() {
            run.planned = base.map(|mut p| {
                p[axis.index()] += delta;
                p
            });
            run.busy_until = run.busy_until.max(at) + Duration::from_secs_f64(seconds);
        }
        true
    }

    /// A hold writes increments of a tenth of a second while less than `HOLD_AHEAD` is queued.
    pub(super) fn pump_hold(&mut self, at: Duration) {
        loop {
            let Some(run) = self.jog.as_ref() else {
                return;
            };
            let (Some(held), None) = (run.hold, run.cancelling) else {
                return;
            };
            if run.busy_until > at + HOLD_AHEAD.saturating_sub(HOLD_SLICE) {
                return;
            }
            let increment = held.feed * HOLD_SLICE.as_secs_f64() / 60.0;
            if !self.write_jog(held.axis, held.direction, increment, held.feed, at) {
                return;
            }
        }
    }

    /// A status report while jogging: the cancel is repeated while GRBL could still run a line,
    /// and the jog ends at the first `Idle` after its last answer with nothing owed.
    pub(super) fn jog_report(&mut self, report: &Report) {
        let reports = self.reports;
        let owed = self.owed.iter().any(|o| matches!(o, Owed::Jog(_)));
        let Some(run) = self.jog.as_ref() else {
            return;
        };
        if report.state == "Alarm" {
            return;
        }
        let later = run.answered_at.is_none_or(|n| reports > n);
        let stands = report.state == "Idle" && !owed;
        let (cancelling, holding, answered) = (
            run.cancelling.is_some(),
            run.hold.is_some(),
            run.answered_at.is_some(),
        );
        if cancelling {
            if owed || report.state == "Jog" {
                // GRBL ignores `0x85` outside a jog, so a `$J=` still in its buffer would run.
                self.out.push(JOG_CANCEL);
            } else if stands && later {
                self.finish_jog();
            }
        } else if stands && later && !holding && answered {
            self.finish_jog();
        }
    }

    fn finish_jog(&mut self) {
        let Some(run) = self.jog.take() else {
            return;
        };
        self.events.push(match run.failure {
            Some(detail) => Event::JogFailed { detail },
            None => Event::JogEnded,
        });
    }

    /// The cancel that does not settle in `CANCEL_LIMIT` is the stop of G12 (hold, reset, banner).
    pub(super) fn jog_time(&mut self, at: Duration) {
        let late = self.jog.as_mut().is_some_and(|run| {
            let late = !run.escalated
                && run
                    .cancelling
                    .is_some_and(|since| at >= since + CANCEL_LIMIT);
            run.escalated |= late;
            late
        });
        if late {
            self.begin_stop(None, at);
        }
        self.pump_hold(at);
    }

    /// GRBL's answer to a `$J=` line; an error starts the cancel and names itself when it ends.
    pub(super) fn on_jog_answer(&mut self, code: Option<u16>, at: Duration) {
        let reports = self.reports;
        let Some(run) = self.jog.as_mut() else {
            return;
        };
        run.answered_at = Some(reports);
        if let Some(code) = code {
            run.failure.get_or_insert_with(|| texts::error_text(code));
            run.hold = None;
            if run.cancelling.is_none() {
                run.cancelling = Some(at);
                self.out.push(JOG_CANCEL);
            }
        }
    }

    /// Whether a jog runs or is being cancelled.
    pub(super) fn jogging(&self) -> bool {
        self.jog.is_some()
    }

    /// Writes the go to point's one `line` (an absolute `$J=`) that takes `seconds`, as a jog
    /// GRBL ends at Idle or cancels with `0x85`; false when another motion owns the link or the
    /// line does not fit GRBL's buffer.
    pub(super) fn jog_absolute(&mut self, line: &str, seconds: f64, at: Duration) -> bool {
        if self.jog_blocked() || self.jog.is_some() {
            return false;
        }
        if self.owed_bytes() + LASER_OFF.len() + line.len() > self.rx_size {
            return false;
        }
        self.laser_off_first();
        self.out.extend_from_slice(line.as_bytes());
        self.owed.push_back(Owed::Jog(line.len()));
        let mut run = Jog::new(true);
        run.busy_until = at + Duration::from_secs_f64(seconds.max(0.0));
        self.jog = Some(run);
        true
    }

    /// The jog feed `feed` capped at the slower of `$110` and `$111`, whole, at least 1.
    pub(super) fn capped_both(&self, feed: f64) -> f64 {
        self.capped(JogAxis::X, feed)
            .min(self.capped(JogAxis::Y, feed))
    }
}
