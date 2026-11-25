//! A controller that streams a job's lines at a steady pace and touches nothing (no laser, no
//! serial port); one of the two [`Controller`]s. Pure: the caller passes the time since the
//! server's start with every call.

use std::time::Duration;

use curvyo_connector_core::{
    ControllerCommand, ControllerInfo, Event, Feed, Job, Millimetres, check_frame,
};

use crate::controller::{Advance, Controller};
use crate::fake_jog::FakeHead;

/// How long the fake controller's homing cycle takes (`specs/0118` M6).
pub const FAKE_HOMING: Duration = Duration::from_secs(2);

/// The feed the fake traces a frame at, in mm/min (the frame text's own, `specs/0118` 25).
const FRAME_FEED: f64 = 3_000.0;

/// The text of the alarm the fake controller raises.
#[must_use]
pub fn alarm_text(line: u32) -> String {
    format!("Fake alarm at line {line}")
}

#[derive(Debug, Clone, Copy)]
struct Run {
    total: u32,
    /// Milliseconds the job has run, pauses not counted.
    ran_ms: u64,
    last: Duration,
    paused: bool,
    reported: u32,
    /// A frame: it moves the head and never raises the fake alarm.
    frame: bool,
    /// A restart from a line: the lines before it, counted as done.
    from: u32,
    /// The restart's travel to its line, in milliseconds of the run.
    travel_ms: u64,
    /// The travel is over (`EntryReached` was given).
    reached: bool,
}

/// The fake controller.
#[derive(Debug, Clone)]
pub struct FakeController {
    lines_per_second: u32,
    alarm_at_line: Option<u32>,
    run: Option<Run>,
    /// When the homing cycle ends; `None` while not homing.
    homing_until: Option<Duration>,
    /// The head, moved by jogs.
    head: FakeHead,
    /// When the go to point's pulse ends; `None` while none fires.
    pulse_until: Option<Duration>,
}

impl FakeController {
    /// A controller that runs `lines_per_second` lines a second and, if asked, raises an alarm
    /// when the job reaches `alarm_at_line`.
    #[must_use]
    pub fn new(lines_per_second: u32, alarm_at_line: Option<u32>) -> Self {
        Self {
            lines_per_second,
            alarm_at_line,
            run: None,
            homing_until: None,
            head: FakeHead::default(),
            pulse_until: None,
        }
    }

    /// What the controller reports about itself.
    #[must_use]
    pub fn info() -> ControllerInfo {
        ControllerInfo {
            name: "Fake controller".to_owned(),
            firmware: "curvyo-fake 0".to_owned(),
            port: None,
            baud: None,
            settings: None,
        }
    }

    fn catch_up(run: &mut Run, at: Duration) {
        if !run.paused {
            let ran = at.saturating_sub(run.last);
            run.ran_ms += u64::try_from(ran.as_millis()).unwrap_or(u64::MAX);
        }
        run.last = at;
    }

    /// Takes a command of the job machine; `job_lines` is the number of lines of the held job.
    pub fn take(&mut self, command: &ControllerCommand, job_lines: u32, at: Duration) {
        match command {
            ControllerCommand::Start { .. } => {
                self.run = Some(Run {
                    total: job_lines,
                    ran_ms: 0,
                    last: at,
                    paused: false,
                    reported: 0,
                    frame: false,
                    from: 0,
                    travel_ms: 0,
                    reached: true,
                });
            }
            ControllerCommand::StartAt {
                from_line, target, ..
            } => {
                let head = self.head.position();
                let to = [target[0].0, target[1].0];
                let distance = (to[0] - head[0]).hypot(to[1] - head[1]);
                self.head.trace(&[to], FRAME_FEED, at);
                let from = from_line.saturating_sub(1);
                self.run = Some(Run {
                    total: job_lines,
                    ran_ms: 0,
                    last: at,
                    paused: false,
                    reported: from,
                    frame: false,
                    from,
                    travel_ms: u64::try_from(
                        Duration::from_secs_f64(distance / FRAME_FEED * 60.0).as_millis(),
                    )
                    .unwrap_or(u64::MAX),
                    reached: false,
                });
            }
            ControllerCommand::Pause => {
                if let Some(run) = self.run.as_mut() {
                    Self::catch_up(run, at);
                    run.paused = true;
                }
            }
            ControllerCommand::Resume => {
                if let Some(run) = self.run.as_mut() {
                    Self::catch_up(run, at);
                    run.paused = false;
                }
            }
            ControllerCommand::Stop => {
                self.run = None;
                self.homing_until = None;
                self.pulse_until = None;
                self.head.cancel(at);
            }
            // A frame needs its text for the moves (`take_frame`), a go to point answers with its
            // target (`go_to`).
            ControllerCommand::Unlock
            | ControllerCommand::Frame { .. }
            | ControllerCommand::GoTo { .. } => {}
            ControllerCommand::Pulse { length, .. } => {
                self.pulse_until = Some(at + length.duration());
            }
            ControllerCommand::Home => self.homing_until = Some(at + FAKE_HOMING),
            ControllerCommand::Jog {
                axis,
                direction,
                distance,
                feed,
                ..
            } => self.head.step((*axis, *direction), distance.0, feed.0, at),
            ControllerCommand::JogHold {
                axis,
                direction,
                feed,
                ..
            } => self.head.hold((*axis, *direction), feed.0, at),
            ControllerCommand::JogCancel => self.head.cancel(at),
        }
    }

    /// A frame: the lines run at the pace of a job and the head traces their moves in the
    /// fake's own coordinates (machine origin at its homed corner, work offset 0).
    pub fn take_frame(&mut self, job: &Job, at: Duration) {
        let lines = u32::try_from(job.text.lines().count()).unwrap_or(u32::MAX);
        self.run = Some(Run {
            total: lines,
            ran_ms: 0,
            last: at,
            paused: false,
            reported: 0,
            frame: true,
            from: 0,
            travel_ms: 0,
            reached: true,
        });
        if let Ok(moves) = check_frame(&job.text) {
            let head = self.head.position();
            let zero = [Millimetres(0.0); 2];
            let points: Vec<[f64; 2]> = moves
                .points([Millimetres(head[0]), Millimetres(head[1])], zero)
                .iter()
                .map(|p| [p[0].0, p[1].0])
                .collect();
            self.head.trace(&points, FRAME_FEED, at);
        }
    }

    /// A go to point: the head moves straight along X, then Y, to `target` (the fake's own
    /// coordinates, no travel to clamp to); `JogEnded` at once when it is there already.
    pub fn go_to(&mut self, target: [Millimetres; 2], feed: Feed, at: Duration) -> Vec<Event> {
        self.head.trace(&[[target[0].0, target[1].0]], feed.0, at);
        let mut events = vec![Event::GoToTarget { target }];
        if !self.head.moving() {
            events.push(Event::JogEnded);
        }
        events
    }

    /// Lets time pass and returns the events that happened: the end of a homing, progress, the
    /// end of the job, or the alarm, after which the stream stops by itself.
    pub fn run_until(&mut self, at: Duration) -> Vec<Event> {
        if self.pulse_until.is_some_and(|until| at >= until) {
            self.pulse_until = None;
            return vec![Event::PulseEnded];
        }
        if self.homing_until.is_some_and(|until| at >= until) {
            self.homing_until = None;
            let mut events = vec![Event::Homed];
            events.extend(self.head.home());
            return events;
        }
        let mut events = self.head.run_until(at);
        let Some(mut run) = self.run else {
            return events;
        };
        Self::catch_up(&mut run, at);
        if !run.reached && run.ran_ms >= run.travel_ms {
            run.reached = true;
            events.push(Event::EntryReached);
        }
        let moving = run.ran_ms.saturating_sub(run.travel_ms);
        let done = moving.saturating_mul(u64::from(self.lines_per_second)) / 1000;
        let done = u64::from(run.from).saturating_add(done);
        let mut line = u32::try_from(done.min(u64::from(run.total))).unwrap_or(run.total);
        let alarm = self
            .alarm_at_line
            .filter(|n| !run.frame && *n >= 1 && *n <= run.total && line >= *n);
        if let Some(n) = alarm {
            line = n;
        }
        if line != run.reported {
            run.reported = line;
            events.push(Event::Progress {
                line,
                elapsed_ms: run.ran_ms,
            });
        }
        if let Some(n) = alarm {
            events.push(Event::Alarm {
                text: alarm_text(n),
            });
            self.run = None;
        } else if line == run.total {
            events.push(Event::JobFinished {
                elapsed_ms: run.ran_ms,
            });
            self.run = None;
        } else {
            self.run = Some(run);
        }
        events
    }
}

impl Controller for FakeController {
    fn info(&self) -> Option<ControllerInfo> {
        Some(Self::info())
    }

    fn command(
        &mut self,
        command: &ControllerCommand,
        job: Option<&Job>,
        at: Duration,
    ) -> Vec<Event> {
        if let (ControllerCommand::Frame { .. }, Some(job)) = (command, job) {
            self.take_frame(job, at);
            return Vec::new();
        }
        if let ControllerCommand::GoTo { target, feed } = command {
            return self.go_to(*target, *feed, at);
        }
        let lines = job.map_or(0, |j| j.text.lines().count());
        self.take(command, u32::try_from(lines).unwrap_or(u32::MAX), at);
        Vec::new()
    }

    fn advance(&mut self, _read: &[u8], at: Duration) -> Advance {
        Advance {
            events: self.run_until(at),
            ..Advance::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn start(c: &mut FakeController, lines: u32, at: Duration) {
        c.take(
            &ControllerCommand::Start {
                job_id: Uuid::nil(),
            },
            lines,
            at,
        );
    }

    fn lines_of(events: &[Event]) -> Option<u32> {
        events.iter().rev().find_map(|e| match e {
            Event::Progress { line, .. } => Some(*line),
            _ => None,
        })
    }

    #[test]
    fn it_streams_at_the_set_pace_and_finishes() {
        let mut c = FakeController::new(20, None);
        start(&mut c, 3, ms(1000));
        assert_eq!(c.run_until(ms(1049)), Vec::new());
        assert_eq!(lines_of(&c.run_until(ms(1050))), Some(1));
        assert_eq!(lines_of(&c.run_until(ms(1100))), Some(2));
        let end = c.run_until(ms(1150));
        assert_eq!(lines_of(&end), Some(3));
        assert_eq!(end.last(), Some(&Event::JobFinished { elapsed_ms: 150 }));
        assert_eq!(c.run_until(ms(5000)), Vec::new());
    }

    #[test]
    fn pause_holds_the_count_and_resume_continues_from_the_same_line() {
        let mut c = FakeController::new(20, None);
        start(&mut c, 100, ms(0));
        assert_eq!(lines_of(&c.run_until(ms(500))), Some(10));
        c.take(&ControllerCommand::Pause, 0, ms(500));
        assert_eq!(c.run_until(ms(9000)), Vec::new());
        c.take(&ControllerCommand::Resume, 0, ms(9000));
        assert_eq!(lines_of(&c.run_until(ms(9050))), Some(11));
    }

    #[test]
    fn stop_ends_the_stream() {
        let mut c = FakeController::new(20, None);
        start(&mut c, 100, ms(0));
        c.run_until(ms(200));
        c.take(&ControllerCommand::Stop, 0, ms(200));
        assert_eq!(c.run_until(ms(2000)), Vec::new());
    }

    #[test]
    fn an_alarm_at_a_line_stops_the_stream_by_itself() {
        let mut c = FakeController::new(20, Some(5));
        start(&mut c, 100, ms(0));
        let events = c.run_until(ms(2000));
        assert_eq!(lines_of(&events), Some(5));
        assert_eq!(
            events.last(),
            Some(&Event::Alarm {
                text: alarm_text(5)
            })
        );
        assert_eq!(c.run_until(ms(4000)), Vec::new());
    }

    #[test]
    fn an_empty_job_finishes_at_once() {
        let mut c = FakeController::new(20, None);
        start(&mut c, 0, ms(0));
        assert_eq!(
            c.run_until(ms(1)),
            vec![Event::JobFinished { elapsed_ms: 1 }]
        );
    }

    #[test]
    fn home_takes_two_seconds_of_the_passed_time() {
        let mut c = FakeController::new(20, None);
        c.take(&ControllerCommand::Home, 0, ms(1000));
        assert_eq!(c.run_until(ms(2999)), Vec::new());
        // The head's position follows every homing (`specs/0118` 2e: the restart needs it).
        assert_eq!(c.run_until(ms(3000))[0], Event::Homed);
        assert_eq!(c.run_until(ms(9000)), Vec::new(), "homed once");
    }

    #[test]
    fn stop_during_home_cancels_it() {
        let mut c = FakeController::new(20, None);
        c.take(&ControllerCommand::Home, 0, ms(0));
        c.run_until(ms(500));
        c.take(&ControllerCommand::Stop, 0, ms(500));
        assert_eq!(c.run_until(ms(5000)), Vec::new());
    }

    #[test]
    fn home_after_its_alarm_ends_in_homed() {
        let mut c = FakeController::new(20, Some(2));
        start(&mut c, 10, ms(0));
        let events = c.run_until(ms(1000));
        assert!(matches!(events.last(), Some(Event::Alarm { .. })));
        c.take(&ControllerCommand::Home, 0, ms(1000));
        assert_eq!(c.run_until(ms(3000))[0], Event::Homed);
    }

    #[test]
    fn it_reports_itself_without_a_position() {
        let info = FakeController::info();
        assert_eq!(info.name, "Fake controller");
        assert_eq!(info.firmware, "curvyo-fake 0");
    }
}
