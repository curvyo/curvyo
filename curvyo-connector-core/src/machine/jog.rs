//! How the job machine takes jog messages and jog events, and the dead-man clock
//! (`specs/0118` criteria 8, 10, 18, 19; adrs J3 to J5).

use std::time::Duration;

use super::{BUSY_WORDS, ControllerCommand, Machine, Output};
use crate::jog::{JOG_DEAD_MAN, JogAxis, JogDirection, JogStep};
use crate::message::{ErrorCode, Message, State};
use crate::millimetres::Millimetres;

/// The jog the controller runs, as the machine tracks it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct JogRun {
    /// A continuous jog that lives on keep-alives.
    hold: bool,
    /// When a hold without a keep-alive is cancelled.
    deadline: Option<Duration>,
    /// Steps given to the controller: the one that runs and at most one queued behind it.
    steps: u8,
    /// The cancel was given; the machine waits for the controller to stand.
    cancelling: bool,
}

impl Machine {
    /// `Jog`, `JogKeep` or `JogStop` from the app.
    pub(super) fn jog_message(&mut self, message: &Message, out: &mut Output) {
        match *message {
            Message::Jog {
                axis,
                direction,
                step,
                hold,
            } => self.jog(axis, direction, step, hold, out),
            Message::JogKeep => {
                let now = self.now;
                if let Some(run) = self.jog.as_mut().filter(|r| r.hold && !r.cancelling) {
                    run.deadline = Some(now + JOG_DEAD_MAN);
                }
            }
            // A release that arrives after the jog ended changes nothing (criterion 18).
            Message::JogStop => self.cancel_jog(false, out),
            _ => {}
        }
    }

    fn refusal(&self) -> Option<String> {
        let busy_word = self
            .word
            .as_deref()
            .filter(|word| BUSY_WORDS.contains(word))
            .map(|word| format!("Controller state: {word}."));
        match self.state {
            State::Idle | State::Ready | State::Done { .. } => busy_word,
            State::Alarm => Some("Alarm.".to_owned()),
            State::Homing => Some("Homing.".to_owned()),
            State::Running | State::Paused => Some("A job is running.".to_owned()),
            State::Receiving => Some("A job is arriving.".to_owned()),
            State::AwaitingLocalStart => Some(self.waiting_words().to_owned()),
            State::Framing => Some("Framing.".to_owned()),
            State::Focusing => Some("Focus point.".to_owned()),
            State::Pulsing => Some("Pulse.".to_owned()),
            State::NoController => Some("No controller.".to_owned()),
            State::Jogging => None,
        }
    }

    fn jog(
        &mut self,
        axis: JogAxis,
        direction: JogDirection,
        step: JogStep,
        hold: bool,
        out: &mut Output,
    ) {
        if let Some(message) = self.refusal() {
            Self::error(out, ErrorCode::WrongState, &message);
            return;
        }
        let feed = step.feed();
        if self.state == State::Jogging {
            self.jog_while_jogging(axis, direction, step, hold, out);
            return;
        }
        self.motion_from = self.state;
        if hold {
            out.commands.push(ControllerCommand::JogHold {
                axis,
                direction,
                feed,
                homed: self.homed,
            });
        } else {
            out.commands.push(ControllerCommand::Jog {
                axis,
                direction,
                distance: step.distance(),
                feed,
                homed: self.homed,
            });
        }
        self.jog = Some(JogRun {
            hold,
            deadline: hold.then_some(self.now + JOG_DEAD_MAN),
            steps: 1,
            cancelling: false,
        });
        self.set(State::Jogging, out);
    }

    /// A press during a jog: one more step is taken once, a held press or more steps are dropped
    /// (criterion 19; the hub answers with the status).
    fn jog_while_jogging(
        &mut self,
        axis: JogAxis,
        direction: JogDirection,
        step: JogStep,
        hold: bool,
        out: &mut Output,
    ) {
        let homed = self.homed;
        let Some(run) = self.jog.as_mut() else {
            return;
        };
        if hold || run.hold || run.cancelling || run.steps >= 2 {
            return;
        }
        run.steps += 1;
        out.commands.push(ControllerCommand::Jog {
            axis,
            direction,
            distance: step.distance(),
            feed: step.feed(),
            homed,
        });
    }

    /// Ends the jog now: without confirmation the machine is back at once, with it the machine
    /// waits for the controller to stand (`JogEnded`); `show` makes the wait read "Stopping".
    pub(super) fn cancel_jog(&mut self, show: bool, out: &mut Output) {
        let Some(run) = self.jog.as_mut().filter(|r| !r.cancelling) else {
            return;
        };
        run.cancelling = true;
        run.deadline = None;
        out.commands.push(ControllerCommand::JogCancel);
        if self.confirm {
            if show {
                self.stop_pending = true;
            }
            out.messages.push(self.status());
        } else {
            self.end_jog(None, out);
        }
    }

    /// Back to the state the jog started from (`Idle`, `Ready` with the kept job, or `Done`).
    pub(super) fn end_jog(&mut self, detail: Option<String>, out: &mut Output) {
        self.jog = None;
        let back = match self.motion_from {
            State::Ready | State::Done { .. } if self.held.is_none() => State::Idle,
            state @ (State::Ready | State::Done { .. }) => state,
            _ => State::Idle,
        };
        match detail {
            Some(detail) => self.set_with(back, detail, false, out),
            None => self.set(back, out),
        }
    }

    /// The clock: a hold whose keep-alive is late is cancelled (criterion 18).
    pub(super) fn on_clock(&mut self, now: Duration, out: &mut Output) {
        self.now = now;
        self.pulse_clock(now, out);
        let late = self
            .jog
            .is_some_and(|r| r.deadline.is_some_and(|deadline| now >= deadline));
        if late {
            self.cancel_jog(false, out);
        }
    }

    /// `JogEnded`, `JogFailed`, a confirmed stop or a reset while jogging.
    pub(super) fn on_jogging(&mut self, event: super::Event, out: &mut Output) {
        match event {
            super::Event::JogEnded | super::Event::Stopped | super::Event::ControllerReset => {
                self.end_jog(None, out);
            }
            super::Event::JogFailed { detail } => self.end_jog(Some(detail), out),
            _ => {}
        }
    }

    /// The positions of a report; the status goes out when one changed.
    pub(super) fn on_machine_position(
        &mut self,
        machine: Option<[Millimetres; 3]>,
        offset: Option<[Millimetres; 3]>,
        out: &mut Output,
    ) {
        let before = (self.mpos, self.wco);
        self.mpos = machine.or(self.mpos);
        self.wco = offset.or(self.wco);
        if before != (self.mpos, self.wco) {
            out.messages.push(self.status());
        }
    }
}
