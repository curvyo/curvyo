//! A controller that streams a job's lines at a steady pace and touches nothing (no laser, no
//! serial port); one of the two [`Controller`]s. Pure: the caller passes the time since the
//! server's start with every call.

use std::time::Duration;

use curvyo_connector_core::{ControllerCommand, ControllerInfo, Event, Job};

use crate::controller::{Advance, Controller};

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
}

/// The fake controller.
#[derive(Debug, Clone)]
pub struct FakeController {
    lines_per_second: u32,
    alarm_at_line: Option<u32>,
    run: Option<Run>,
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
            ControllerCommand::Stop => self.run = None,
            ControllerCommand::Unlock => {}
        }
    }

    /// Lets time pass and returns the events that happened: progress, the end of the job, or the
    /// alarm, after which the stream stops by itself.
    pub fn run_until(&mut self, at: Duration) -> Vec<Event> {
        let Some(mut run) = self.run else {
            return Vec::new();
        };
        Self::catch_up(&mut run, at);
        let done = run.ran_ms.saturating_mul(u64::from(self.lines_per_second)) / 1000;
        let mut line = u32::try_from(done.min(u64::from(run.total))).unwrap_or(run.total);
        let alarm = self
            .alarm_at_line
            .filter(|n| *n >= 1 && *n <= run.total && line >= *n);
        if let Some(n) = alarm {
            line = n;
        }
        let mut events = Vec::new();
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
    fn it_reports_itself_without_a_position() {
        let info = FakeController::info();
        assert_eq!(info.name, "Fake controller");
        assert_eq!(info.firmware, "curvyo-fake 0");
    }
}
