//! How the job machine takes the job controls and the events of the controller side.

use super::{ControllerCommand, Event, Machine, Output, Waiting, count};
use crate::message::{ErrorCode, JobResult, Message, State};

impl Machine {
    /// Pause, Resume or Stop from the app. With `confirm`, the state changes only when the
    /// controller confirms (`Held`, `Resumed`, `Stopped`); a pending stop takes no other control.
    pub(super) fn control(&mut self, message: &Message, out: &mut Output) {
        match (message, self.state) {
            (Message::Pause, State::Running) => {
                if !self.stop_pending && !self.pause_pending {
                    out.commands.push(ControllerCommand::Pause);
                    if !self.confirm {
                        self.set(State::Paused, out);
                    }
                }
            }
            (Message::Resume, State::Paused) => {
                if !self.stop_pending {
                    out.commands.push(ControllerCommand::Resume);
                    if !self.confirm {
                        self.set(State::Running, out);
                    }
                }
            }
            (Message::Resume, State::Running) if self.pause_pending && !self.stop_pending => {
                // Only cancels the pending pause: the job never stood still (`specs/0133` P3).
                self.pause_pending = false;
                out.messages.push(self.status());
                out.commands.push(ControllerCommand::Resume);
            }
            (Message::Stop, State::Running | State::Paused) => {
                if self.stop_pending {
                    return;
                }
                self.pause_pending = false;
                out.commands.push(ControllerCommand::Stop);
                if self.confirm {
                    self.stop_pending = true;
                    out.messages.push(self.status());
                } else {
                    self.set(
                        State::Done {
                            result: JobResult::Stopped,
                        },
                        out,
                    );
                }
            }
            (Message::Pause, State::Homing) => {
                Self::error(out, ErrorCode::WrongState, "No job to pause");
            }
            (Message::Stop, State::Homing) => self.stop_homing(out),
            (Message::Stop, State::AwaitingLocalStart) if self.waiting == Waiting::Home => {
                self.end_homing(out);
            }
            (Message::Stop, State::Receiving | State::Ready | State::AwaitingLocalStart) => {
                self.discard();
                self.set(State::Idle, out);
            }
            (Message::Stop, _)
            | (Message::Pause, State::Paused)
            | (Message::Resume, State::Running) => {}
            _ => Self::wrong_state(out),
        }
    }

    pub(super) fn on_controller(&mut self, event: Event, out: &mut Output) {
        if self.state == State::NoController {
            match event {
                Event::ControllerConnected => self.set(State::Idle, out),
                Event::LaserModeOff(off) => self.laser_off = off,
                Event::LaserModeUnknown(unknown) => self.laser_unknown = unknown,
                Event::NoControllerDetail { detail }
                    if !self.sticky && self.detail.as_deref() != Some(detail.as_str()) =>
                {
                    self.detail = Some(detail);
                    out.messages.push(self.status());
                }
                _ => {}
            }
            return;
        }
        match event {
            Event::ControllerLost => self.lose(None, out),
            Event::StopUnconfirmed => self.lose(Some("Could not confirm the machine stopped"), out),
            Event::Alarm { text } => {
                self.discard();
                if self.state != State::Alarm || self.alarm.is_none() {
                    // The first alarm's text stands until the alarm clears (`specs/0127` G13).
                    self.alarm = Some(text);
                    self.set(State::Alarm, out);
                }
            }
            Event::AlarmCleared if self.state == State::Alarm => {
                self.alarm = None;
                self.set(State::Idle, out);
            }
            Event::Position(position) => {
                self.position = Some(position);
                out.messages.push(self.status());
            }
            Event::LaserModeOff(off) => self.laser_off = off,
            Event::LaserModeUnknown(unknown) => self.laser_unknown = unknown,
            Event::ControllerWord(word) => {
                if word != self.word {
                    self.word = word;
                    out.messages.push(self.status());
                }
            }
            Event::LocalStartPressed
                if self.state == State::AwaitingLocalStart && self.waiting == Waiting::Home =>
            {
                out.commands.push(ControllerCommand::Home);
                self.set(State::Homing, out);
            }
            Event::LocalStartPressed if self.state == State::AwaitingLocalStart => {
                if let Some(h) = &self.held {
                    out.commands.push(ControllerCommand::Start {
                        job_id: h.job.job_id,
                    });
                }
                self.set(State::Running, out);
            }
            event @ (Event::Homed | Event::HomingFailed { .. } | Event::Stopped)
                if self.state == State::Homing =>
            {
                self.on_homing(event, out);
            }
            other => self.on_job_event(other, out),
        }
    }

    /// The controller is gone: a job in flight is dropped and named in a detail that stays
    /// through the reconnect (`specs/0127` G15, UX notes V7).
    fn lose(&mut self, text: Option<&str>, out: &mut Output) {
        let line = self.held.as_ref().map_or(0, |h| h.line);
        let loss = match self.state {
            State::Running | State::Paused => {
                Some(format!("Lost the controller at line {}", count(line)))
            }
            State::Receiving | State::Ready | State::AwaitingLocalStart | State::Homing => {
                Some("Lost the controller".to_owned())
            }
            _ => None,
        };
        let detail = text.map(str::to_owned).or(loss);
        self.discard();
        self.position = None;
        self.alarm = None;
        self.word = None;
        self.laser_off = false;
        self.laser_unknown = false;
        match detail {
            Some(detail) => self.set_with(State::NoController, detail, true, out),
            None => self.set(State::NoController, out),
        }
    }

    fn on_job_event(&mut self, event: Event, out: &mut Output) {
        let running = matches!(self.state, State::Running | State::Paused);
        let Some(held) = self.held.as_mut().filter(|_| running) else {
            return;
        };
        match event {
            Event::Progress { line, elapsed_ms } => {
                held.line = line.min(held.lines);
                held.elapsed_ms = elapsed_ms;
                out.messages.push(self.status());
            }
            Event::JobFinished { elapsed_ms } if self.state == State::Running => {
                held.line = held.lines;
                held.elapsed_ms = elapsed_ms;
                self.set(
                    State::Done {
                        result: JobResult::Completed,
                    },
                    out,
                );
            }
            Event::JobFailed { elapsed_ms } if self.state == State::Running => {
                held.elapsed_ms = elapsed_ms;
                self.set(
                    State::Done {
                        result: JobResult::Failed,
                    },
                    out,
                );
            }
            Event::JobFailedWith { elapsed_ms, detail } => {
                held.elapsed_ms = elapsed_ms;
                let done = State::Done {
                    result: JobResult::Failed,
                };
                self.set_with(done, detail, false, out);
            }
            Event::Held if self.state == State::Running && !self.stop_pending => {
                self.set(State::Paused, out);
            }
            Event::PauseDeferred
                if self.state == State::Running && !self.stop_pending && !self.pause_pending =>
            {
                self.pause_pending = true;
                out.messages.push(self.status());
            }
            Event::Resumed if self.state == State::Paused && !self.stop_pending => {
                self.set(State::Running, out);
            }
            Event::Stopped if self.stop_pending => {
                let done = State::Done {
                    result: JobResult::Stopped,
                };
                self.set(done, out);
            }
            _ => {}
        }
    }
}
