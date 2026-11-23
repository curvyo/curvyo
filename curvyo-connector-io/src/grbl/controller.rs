//! `GrblController`: the [`Controller`] for GRBL 1.1 (`specs/0127` G2 to G17, G28, G29): identify,
//! polling, the loss watchdog and the job commands. GRBL's replies are read in `replies`.

use std::collections::VecDeque;
use std::time::Duration;

use curvyo_connector_core::{ControllerCommand, ControllerInfo, Event, Job};

use crate::controller::{Advance, Controller};
use crate::grbl::Timing;
use crate::grbl::home::Owed;
use crate::grbl::identify::{Failure, Identify, Outcome};
use crate::grbl::jog::Jog;
use crate::grbl::prepare::prepare;
use crate::grbl::reply::{Lines, Reply, parse};
use crate::grbl::settings::Settings;
use crate::grbl::stop::{Stop, Tick};
use crate::grbl::stream::{Hold, Stream};
use crate::grbl::texts;

/// What the connector writes when it gives the controller up: reset, spindle off, air off.
pub const SAFETY: &[u8] = b"\x18M5\nM9\n";
/// The terminal line of a lost controller.
const LOST: &str = "Lost the controller: no answer to ? for 3 s";
/// The terminal line of a stop GRBL did not confirm (G12).
const UNCONFIRMED: &str = "Could not confirm the machine stopped (no GRBL banner within 3 s)";
/// GRBL states in which no job starts (G4): the door, check mode, sleep.
const BUSY_WORDS: [&str; 3] = ["Door", "Check", "Sleep"];

/// GRBL 1.1 on one serial port.
#[derive(Debug)]
#[allow(clippy::struct_excessive_bools)] // independent GRBL flags, each read on its own
pub struct GrblController {
    pub(super) port: String,
    baud: u32,
    pub(super) timing: Timing,
    lines: Lines,
    identify: Option<Identify>,
    info: Option<ControllerInfo>,
    pub(super) settings: Settings,
    /// GRBL's receive buffer as `Bf:` reported it at identify (empty then), else 128.
    pub(super) rx_size: usize,
    pub(super) out: Vec<u8>,
    pub(super) events: Vec<Event>,
    pub(super) notices: Vec<String>,
    close: Option<String>,
    closed: bool,
    pub(super) last_report: Duration,
    /// When the last `?` went out; `None` before the first.
    last_poll: Option<Duration>,
    pub(super) reports: u64,
    pub(super) wco: [f64; 3],
    /// A report carried the work offset since the connector identified the controller.
    pub(super) wco_seen: bool,
    pub(super) position: Option<[f64; 3]>,
    /// The head's machine position as the last report showed it, when the report allows it.
    pub(super) mpos: Option<[f64; 3]>,
    /// The machine position and offset the job machine was last told.
    pub(super) told: (Option<[f64; 3]>, Option<[f64; 3]>),
    pub(super) word: Option<String>,
    pub(super) alarmed: bool,
    pub(super) alarm_code: Option<u16>,
    pub(super) locked: bool,
    /// Non-job lines (`M5`, `M9`, `$X`, the Home command's `$H`) whose answers are owed.
    pub(super) owed: VecDeque<Owed>,
    /// An alarm came while the Home command's `$H` was in flight: its `ok` ends nothing.
    pub(super) home_alarmed: bool,
    pub(super) stream: Option<Stream>,
    pub(super) stop: Option<Stop>,
    /// An alarm raised during the stop, shown after `Stopped` (G9).
    pub(super) deferred_alarm: bool,
    /// A reset the connector sent after an alarm; its banner is expected.
    pub(super) expect_banner: bool,
    /// When the unanswered `$H` went out; the homing cap counts from it (`specs/0142`).
    pub(super) homing_since: Option<Duration>,
    /// The jog that runs or is being cancelled (`specs/0118` J6).
    pub(super) jog: Option<Jog>,
    /// The last spindle word the connector wrote was its own `M5`: no job line since.
    pub(super) spindle_off: bool,
    /// Machine X and Y at the first report after a homing ended, for the travel box.
    pub(super) home_mpos: Option<[f64; 2]>,
    /// A homing ended and the next report shows where the head stands.
    pub(super) home_pending: bool,
}

impl GrblController {
    /// A controller on `port` opened at `at`: the soft reset is the first byte (G17).
    #[must_use]
    pub fn new(port: &str, baud: u32, timing: Timing, at: Duration) -> Self {
        let mut out = Vec::new();
        let identify = Identify::start(at, timing.identify, &mut out);
        Self {
            port: port.to_owned(),
            baud,
            timing,
            lines: Lines::default(),
            identify: Some(identify),
            info: None,
            settings: Settings::default(),
            rx_size: crate::grbl::flow::RX_BUFFER,
            out,
            events: Vec::new(),
            notices: Vec::new(),
            close: None,
            closed: false,
            last_report: at,
            last_poll: None,
            reports: 0,
            wco: [0.0; 3],
            wco_seen: false,
            position: None,
            mpos: None,
            told: (None, None),
            word: None,
            alarmed: false,
            alarm_code: None,
            locked: false,
            owed: VecDeque::new(),
            home_alarmed: false,
            stream: None,
            stop: None,
            deferred_alarm: false,
            expect_banner: false,
            homing_since: None,
            jog: None,
            spindle_off: false,
            home_mpos: None,
            home_pending: false,
        }
    }

    pub(super) fn give_up(&mut self, notice: String, safety: bool) {
        // Nothing queued goes out ahead of the safety bytes, or at all.
        self.out.clear();
        if safety {
            self.out.extend_from_slice(SAFETY);
        }
        self.close = Some(notice);
        self.closed = true;
        self.stream = None;
        self.stop = None;
        self.jog = None;
    }

    fn on_identify(&mut self, outcome: Outcome, at: Duration) {
        match outcome {
            Outcome::Waiting => {}
            Outcome::Failed(failure) => {
                self.identify = None;
                let (detail, notice) = match failure {
                    Failure::Unsupported => (
                        texts::OLD_GRBL.to_owned(),
                        format!("{} (on {})", texts::OLD_GRBL, self.port),
                    ),
                    Failure::NoAnswer { got } => {
                        let detail = texts::no_answer(&self.port);
                        let notice = match got {
                            Some(got) => format!("{detail} (got: {got})"),
                            None => detail.clone(),
                        };
                        (detail, notice)
                    }
                };
                self.events.push(Event::NoControllerDetail { detail });
                self.give_up(notice, false);
            }
            Outcome::Done(found) => {
                self.identify = None;
                self.settings = found.settings;
                if let Some(free) = found.report.rx_free {
                    self.rx_size = free;
                }
                self.alarm_code = found.alarm;
                self.locked = found.locked;
                self.info = Some(ControllerInfo {
                    name: "GRBL".to_owned(),
                    firmware: found.version.clone(),
                    port: Some(self.port.clone()),
                    baud: Some(self.baud),
                    settings: Some(self.settings.to_wire()),
                });
                self.notices.push(format!(
                    "GRBL {} on {}: {}",
                    found.version,
                    self.port,
                    self.settings.summary()
                ));
                if self.settings.laser_mode() == Some(false) {
                    self.notices.push(texts::LASER_MODE_OFF.to_owned());
                }
                let unknown = self.settings.laser_mode().is_none();
                if unknown {
                    self.notices.push(texts::LASER_MODE_UNKNOWN.to_owned());
                }
                let laser_off = self.settings.laser_mode() == Some(false);
                self.events.push(Event::LaserModeOff(laser_off));
                self.events.push(Event::LaserModeUnknown(unknown));
                self.events.push(Event::ControllerConnected);
                self.last_poll = None;
                self.on_report(&found.report, at);
            }
        }
    }

    fn on_line(&mut self, line: &str, at: Duration) {
        let reply = parse(line);
        if let Some(identify) = self.identify.as_mut() {
            let outcome = identify.on_reply(reply, at, &mut self.out);
            self.on_identify(outcome, at);
            return;
        }
        match reply {
            Reply::Report(report) => self.on_report(&report, at),
            Reply::Ok => self.on_answer(None, at),
            Reply::Error(code) => self.on_answer(Some(code), at),
            Reply::Alarm(code) => self.on_alarm_line(code),
            Reply::Banner(_) => self.on_banner(),
            Reply::Message(text) if text.contains("to unlock") => self.locked = true,
            _ => {}
        }
    }

    fn poll_period(&self) -> Duration {
        if self.stream.is_some()
            || self.stop.is_some()
            || self.alarmed
            || self.home_in_flight()
            || self.jogging()
        {
            self.timing.poll_fast
        } else {
            self.timing.poll_slow
        }
    }

    fn on_time(&mut self, at: Duration) {
        if let Some(identify) = &self.identify {
            let outcome = identify.on_time(at);
            self.on_identify(outcome, at);
            return;
        }
        if let Some(stop) = self.stop.as_mut()
            && stop.on_time(at, &mut self.out) == Tick::Unconfirmed
        {
            self.unconfirmed();
            return;
        }
        if let Some(stream) = self.stream.as_mut()
            && let Hold::Requested(Some(until)) = stream.hold
            && at >= until
        {
            // GRBL did not confirm the pause: dropped (G29). Never one taken while `$H` ran: that
            // one waits for the `$H` to end and has no limit (`specs/0133` P4).
            stream.hold = Hold::Running;
        }
        self.jog_time(at);
        if self
            .last_poll
            .is_none_or(|last| at >= last + self.poll_period())
        {
            self.out.push(b'?');
            self.last_poll = Some(at);
        }
    }

    /// The stop's reset got no banner in time, or the link went stale while it waited (G12).
    fn unconfirmed(&mut self) {
        self.events.push(Event::StopUnconfirmed);
        self.give_up(UNCONFIRMED.to_owned(), true);
    }

    /// Whether no answer to `?` came for the loss limit (G15), in every phase after identify,
    /// the stop's wait for its banner included.
    fn stale(&self, at: Duration) -> bool {
        self.identify.is_none() && at.saturating_sub(self.last_report) >= self.timing.loss
    }

    fn take(&mut self) -> Advance {
        Advance {
            events: std::mem::take(&mut self.events),
            write: std::mem::take(&mut self.out),
            notices: std::mem::take(&mut self.notices),
            close: self.close.take(),
        }
    }

    pub(super) fn owed_bytes(&self) -> usize {
        self.owed.iter().map(|owed| owed.bytes()).sum()
    }

    fn start(&mut self, job: Option<&Job>, at: Duration) {
        let Some(job) = job else {
            return;
        };
        if self.stop.is_some() {
            // A stop is still being confirmed; its end decides, never a new stream.
            return;
        }
        let refusal = match (self.settings.laser_mode(), self.word.as_deref()) {
            (None, _) => Some(texts::LASER_MODE_UNKNOWN.to_owned()),
            (_, Some(word)) if BUSY_WORDS.contains(&word) => {
                Some(format!("Controller state: {word}"))
            }
            _ => None,
        };
        if let Some(detail) = refusal {
            // Second guard behind the job machine's refusal of `SubmitJob` (G34, G4).
            self.events.push(Event::JobFailedWith {
                elapsed_ms: 0,
                detail,
            });
            return;
        }
        if self.settings.laser_mode() == Some(false) {
            // Lasering without GRBL's laser mode is refused before the first byte (Frage 240 B).
            self.events.push(Event::JobFailedWith {
                elapsed_ms: 0,
                detail: texts::LASER_MODE_OFF.to_owned(),
            });
            return;
        }
        match prepare(&job.text) {
            Err(refused) => self.events.push(Event::JobFailedWith {
                elapsed_ms: 0,
                detail: texts::failed_at(refused.line, refused.cause.text()),
            }),
            Ok(lines) => {
                self.spindle_off = false;
                let mut stream = Stream::new(lines, at);
                stream.fill(self.owed_bytes(), &mut self.out);
                if stream.all_answered() {
                    stream.finished_at_report = Some(self.reports);
                }
                if stream.answered() > 0 {
                    self.events.push(Event::Progress {
                        line: stream.answered(),
                        elapsed_ms: 0,
                    });
                }
                self.stream = Some(stream);
            }
        }
    }

    /// Starts the stop of G12 for the maker's Stop (`failure` `None`) or a failing job; also for
    /// the Home command's homing, which GRBL ends by the reset alone (G9).
    pub(super) fn begin_stop(&mut self, failure: Option<String>, at: Duration) {
        if self.stop.is_some() {
            return;
        }
        let (held, homing, hold_sent) = match self.stream.as_ref() {
            Some(stream) => (
                matches!(stream.hold, Hold::Held { .. }),
                stream.system_in_flight() || self.word.as_deref() == Some("Home"),
                matches!(stream.hold, Hold::Requested(_)),
            ),
            None if self.home_in_flight() => (false, true, false),
            // A jog that did not settle: hold, then reset, as for a stream.
            None if self.jogging() => (false, false, false),
            None => return,
        };
        let limits = (self.timing.hold, self.timing.banner);
        self.stop = Some(Stop::start(
            held,
            homing,
            hold_sent,
            failure,
            limits,
            at,
            &mut self.out,
        ));
    }
}

impl Controller for GrblController {
    fn info(&self) -> Option<ControllerInfo> {
        self.info.clone()
    }

    fn command(
        &mut self,
        command: &ControllerCommand,
        job: Option<&Job>,
        at: Duration,
    ) -> Vec<Event> {
        if self.closed || self.identify.is_some() {
            return Vec::new();
        }
        if let Some(stream) = self.stream.as_mut() {
            stream.tick(at);
        }
        match command {
            ControllerCommand::Start { .. } => self.start(job, at),
            ControllerCommand::Pause => self.pause(at),
            ControllerCommand::Resume => self.resume(),
            ControllerCommand::Stop => self.begin_stop(None, at),
            ControllerCommand::Unlock => self.unlock(),
            ControllerCommand::Home => self.home(),
            ControllerCommand::Jog { .. }
            | ControllerCommand::JogHold { .. }
            | ControllerCommand::JogCancel => self.jog_command(command, at),
        }
        std::mem::take(&mut self.events)
    }

    fn advance(&mut self, read: &[u8], at: Duration) -> Advance {
        if self.closed {
            return Advance::default();
        }
        let lines = self.lines.push(read);
        self.during_homing(at);
        let answered = lines.iter().any(|l| l.starts_with('<'));
        if self.stale(at) && !answered {
            // Judged before anything else is read, so no status of a stale link goes out. During
            // the stop's reset it ends as the unconfirmed stop it is.
            if self.stop.as_ref().is_some_and(Stop::confirms) {
                self.unconfirmed();
            } else {
                self.give_up(LOST.to_owned(), true);
            }
            return self.take();
        }
        if let Some(stream) = self.stream.as_mut() {
            stream.tick(at);
        }
        for line in &lines {
            if self.closed {
                break;
            }
            self.on_line(line, at);
        }
        if !self.closed && self.homing_capped(at) {
            // Judged after the step's input is read: an `ok` in the same step wins (HC5).
            return self.take();
        }
        if !self.closed {
            self.on_time(at);
        }
        let owed = self.owed_bytes();
        if !self.closed
            && self.stop.is_none()
            && let Some(stream) = self.stream.as_mut()
        {
            let before = stream.answered();
            stream.fill(owed, &mut self.out);
            if stream.answered() != before {
                let (line, elapsed_ms) = (stream.answered(), stream.elapsed_ms());
                self.events.push(Event::Progress { line, elapsed_ms });
            }
            if stream.all_answered() && stream.finished_at_report.is_none() {
                stream.finished_at_report = Some(self.reports);
            }
        }
        if !self.closed {
            self.note_homing(at);
        }
        self.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grbl::identify::RESET;

    #[test]
    fn the_first_byte_is_the_soft_reset() {
        let mut c = GrblController::new("/dev/ttyUSB0", 115_200, Timing::default(), Duration::ZERO);
        assert_eq!(c.advance(&[], Duration::ZERO).write, [RESET]);
        assert!(c.info().is_none());
    }
}
