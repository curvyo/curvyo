//! How the job machine takes the job controls and the events of the controller side.

use super::{ControllerCommand, Event, Machine, Output, StartRule, Waiting, count};
use crate::job_list::End;
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
                    self.interrupt(true);
                    self.set(
                        State::Done {
                            result: JobResult::Stopped,
                        },
                        out,
                    );
                    self.job_ended(End::Stopped, out);
                }
            }
            (Message::Pause, State::Homing | State::Jogging) => {
                Self::error(out, ErrorCode::WrongState, "No job to pause");
            }
            (Message::Stop, State::Jogging) => self.cancel_jog(true, out),
            (Message::Stop, State::Homing) => self.stop_homing(out),
            (Message::Stop, State::Framing) => self.stop_frame(out),
            (Message::Stop, State::Focusing | State::Pulsing) => self.stop_focus(out),
            (Message::Stop, State::AwaitingLocalStart) if self.waiting == Waiting::Point => {
                // Stop discards a waiting go to point; the held job stays.
                self.end_focus(None, out);
            }
            (Message::Stop, State::AwaitingLocalStart) if self.waiting == Waiting::Frame => {
                // Stop from anyone discards a waiting frame; the held job stays.
                self.end_frame(None, out);
            }
            (Message::Stop, State::AwaitingLocalStart) if self.waiting == Waiting::Home => {
                // Stop from anyone discards a waiting Home.
                self.requester = None;
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
            self.on_no_controller(event, out);
            return;
        }
        let Some(event) = self.on_motion_event(event, out) else {
            return;
        };
        match event {
            Event::ControllerLost => self.lose(None, out),
            Event::ControllerLostWith { detail } => self.lose(Some(&detail), out),
            Event::StopUnconfirmed => self.lose(Some("Could not confirm the machine stopped"), out),
            Event::Alarm { text } => {
                self.homed = false;
                self.breaks = self.breaks.wrapping_add(1);
                let in_flight = self.in_flight();
                if in_flight {
                    self.interrupt(false);
                }
                self.discard();
                if self.state != State::Alarm || self.alarm.is_none() {
                    // The first alarm's text stands until the alarm clears (`specs/0127` G13).
                    self.alarm = Some(text);
                    self.set(State::Alarm, out);
                }
                if in_flight {
                    self.job_ended(End::Alarm, out);
                }
            }
            Event::AlarmCleared if self.state == State::Alarm => {
                self.alarm = None;
                self.set(State::Idle, out);
            }
            Event::MachinePosition { machine, offset } => {
                self.on_machine_position(machine, offset, out);
            }
            Event::Position(position) => {
                self.position = Some(position);
                out.messages.push(self.status());
            }
            Event::LaserModeOff(off) => self.laser_off = off,
            Event::LaserModeUnknown(unknown) => self.laser_unknown = unknown,
            Event::Travel(travel) => self.travel = travel,
            Event::ControllerWord(word) => {
                if word != self.word {
                    self.word = word;
                    out.messages.push(self.status());
                    self.word_during_pulse(out);
                }
            }
            Event::LocalStartPressed if self.options.start_rule != StartRule::LocalPress => {}
            Event::LocalStartPressed
                if self.state == State::AwaitingLocalStart && self.waiting == Waiting::Home =>
            {
                out.commands.push(ControllerCommand::Home);
                self.set(State::Homing, out);
            }
            Event::LocalStartPressed if self.state == State::AwaitingLocalStart => {
                self.run_waiting(None, out);
            }
            event @ (Event::Homed | Event::HomingFailed { .. } | Event::Stopped)
                if self.state == State::Homing =>
            {
                self.on_homing(event, out);
            }
            Event::Homed => {
                // A job's own `$H` ended in `ok`.
                if !self.homed {
                    self.homed = true;
                    out.messages.push(self.status());
                }
            }
            Event::ControllerReset => {
                // A reset outside a motion: the position is not trusted any more.
                let was_homed = std::mem::replace(&mut self.homed, false);
                self.breaks = self.breaks.wrapping_add(1);
                if was_homed {
                    // Only `homed` changes; a stop whose reset came before `Hold:0` sends this
                    // right after its `Stopped`, which must not read as a second end.
                    out.messages.push(self.status());
                }
            }
            other => self.on_job_event(other, out),
        }
    }

    /// The events a running motion (jog, frame, go to point, pulse) takes; the event back when
    /// none does.
    fn on_motion_event(&mut self, event: Event, out: &mut Output) -> Option<Event> {
        match (self.state, event) {
            (
                State::Jogging | State::Framing | State::Focusing | State::Pulsing,
                Event::ControllerReset,
            ) => {
                self.homed = false;
                self.breaks = self.breaks.wrapping_add(1);
                match self.state {
                    State::Jogging => self.on_jogging(Event::ControllerReset, out),
                    State::Framing => self.on_framing(Event::ControllerReset, out),
                    State::Focusing => self.on_focusing(Event::ControllerReset, out),
                    _ => self.on_pulsing(Event::ControllerReset, out),
                }
            }
            (
                State::Jogging,
                event @ (Event::JogEnded | Event::JogFailed { .. } | Event::Stopped),
            ) => self.on_jogging(event, out),
            (
                State::Focusing,
                event @ (Event::GoToTarget { .. }
                | Event::JogEnded
                | Event::JogFailed { .. }
                | Event::Stopped),
            ) => self.on_focusing(event, out),
            (
                State::Pulsing,
                event @ (Event::PulseEnded
                | Event::PulseFailed { .. }
                | Event::PulseSkipped
                | Event::Stopped),
            ) => self.on_pulsing(event, out),
            (
                State::Framing,
                event @ (Event::JobFinished { .. }
                | Event::JobFailed { .. }
                | Event::JobFailedWith { .. }
                | Event::Stopped
                | Event::Progress { .. }
                | Event::Held
                | Event::Resumed
                | Event::PauseDeferred),
            ) => self.on_framing(event, out),
            (_, event) => return Some(event),
        }
        None
    }

    /// The events of a connector without a controller: only its coming and its words matter.
    fn on_no_controller(&mut self, event: Event, out: &mut Output) {
        match event {
            Event::ControllerConnected => {
                self.homed = false;
                self.travel = None;
                self.breaks = self.breaks.wrapping_add(1);
                self.set(State::Idle, out);
            }
            Event::LaserModeOff(off) => self.laser_off = off,
            Event::LaserModeUnknown(unknown) => self.laser_unknown = unknown,
            Event::Travel(travel) => self.travel = travel,
            Event::NoControllerDetail { detail }
                if !self.sticky && self.detail.as_deref() != Some(detail.as_str()) =>
            {
                self.detail = Some(detail);
                out.messages.push(self.status());
            }
            _ => {}
        }
    }

    /// The controller is gone: a job in flight is dropped and named in a detail that stays
    /// through the reconnect (`specs/0127` G15, UX notes V7).
    pub(super) fn lose(&mut self, text: Option<&str>, out: &mut Output) {
        self.homed = false;
        self.breaks = self.breaks.wrapping_add(1);
        if self.planner != crate::restart::PLANNER_BLOCKS {
            self.planner = crate::restart::PLANNER_BLOCKS;
            self.plan_all();
        }
        let in_flight = self.in_flight();
        if in_flight {
            self.interrupt(false);
        }
        self.mpos = None;
        self.wco = None;
        let line = self.held.as_ref().map_or(0, |h| h.line);
        let loss = match self.state {
            _ if self.waits_for_resume() => {
                Some(format!("Lost the controller at line {}", count(line)))
            }
            State::Running | State::Paused => {
                Some(format!("Lost the controller at line {}", count(line)))
            }
            State::Receiving
            | State::Ready
            | State::AwaitingLocalStart
            | State::Homing
            | State::Jogging
            | State::Framing
            | State::Focusing
            | State::Pulsing => Some("Lost the controller".to_owned()),
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
        if in_flight {
            self.job_ended(End::Lost, out);
        }
    }

    /// A job runs, is paused, or waits for its resume.
    fn in_flight(&self) -> bool {
        matches!(self.state, State::Running | State::Paused) || self.waits_for_resume()
    }

    fn on_job_event(&mut self, event: Event, out: &mut Output) {
        let running =
            matches!(self.state, State::Running | State::Paused) || self.waits_for_resume();
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
                self.job_ended(End::Completed, out);
            }
            Event::JobFailed { elapsed_ms } if self.state == State::Running => {
                held.elapsed_ms = elapsed_ms;
                self.set(
                    State::Done {
                        result: JobResult::Failed,
                    },
                    out,
                );
                self.job_ended(End::Failed, out);
            }
            Event::JobFailedWith { elapsed_ms, detail } => {
                held.elapsed_ms = elapsed_ms;
                let done = State::Done {
                    result: JobResult::Failed,
                };
                self.set_with(done, detail, false, out);
                self.job_ended(End::Failed, out);
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
                self.interrupt(true);
                let done = State::Done {
                    result: JobResult::Stopped,
                };
                self.set(done, out);
                self.job_ended(End::Stopped, out);
            }
            _ => {}
        }
    }
}
