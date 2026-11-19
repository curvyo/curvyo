//! How the job machine takes `Home` and the controller's homing events (`specs/0118` M3, M5).

use super::{BUSY_WORDS, ControllerCommand, Event, Machine, Output, Waiting};
use crate::message::{ErrorCode, State};

impl Machine {
    /// `Home` from the app: homes from `Idle`, `Ready`, `Done` or `Alarm` (a `Ready` job is kept),
    /// waits for the local press with `require_local_start`, and is refused with its cause in
    /// every other state; in `Homing` it changes nothing.
    pub(super) fn home(&mut self, out: &mut Output) {
        let busy_word = self
            .word
            .as_deref()
            .filter(|word| BUSY_WORDS.contains(word))
            .map(str::to_owned);
        let refusal = match self.state {
            State::Idle | State::Ready | State::Done { .. } => {
                busy_word.map(|word| format!("Controller state: {word}."))
            }
            State::Alarm => None,
            State::Homing => return,
            State::Running | State::Paused => Some("A job is running.".to_owned()),
            State::Receiving => Some("A job is arriving.".to_owned()),
            State::AwaitingLocalStart => Some("A start waits at the machine.".to_owned()),
            State::NoController => Some("No controller.".to_owned()),
        };
        if let Some(message) = refusal {
            Self::error(out, ErrorCode::WrongState, &message);
            return;
        }
        self.homed_from = match self.state {
            State::Ready => State::Ready,
            State::Alarm => State::Alarm,
            _ => State::Idle,
        };
        if self.homed_from == State::Idle {
            // A done job's result is over: homing from `Done` returns to `Idle`.
            self.discard();
        }
        if self.options.require_local_start {
            self.waiting = Waiting::Home;
            self.set(State::AwaitingLocalStart, out);
        } else {
            out.commands.push(ControllerCommand::Home);
            self.set(State::Homing, out);
        }
    }

    /// Stop while homing: the controller is told at once; without confirmation the machine is
    /// back where the homing started, with it the stop waits for `Stopped`.
    pub(super) fn stop_homing(&mut self, out: &mut Output) {
        if self.stop_pending {
            return;
        }
        out.commands.push(ControllerCommand::Stop);
        if self.confirm {
            self.stop_pending = true;
            out.messages.push(self.status());
        } else {
            self.end_homing(out);
        }
    }

    /// Back to the state the homing started from: `Ready` with the kept job, `Alarm` with its
    /// text, else `Idle`.
    pub(super) fn end_homing(&mut self, out: &mut Output) {
        let back = match self.homed_from {
            State::Alarm => State::Alarm,
            _ if self.held.is_some() => State::Ready,
            _ => State::Idle,
        };
        self.set(back, out);
    }

    /// `Homed`, `HomingFailed` or a confirmed `Stopped` while `Homing`.
    pub(super) fn on_homing(&mut self, event: Event, out: &mut Output) {
        match event {
            Event::Homed => {
                // GRBL leaves the alarm by homing.
                self.alarm = None;
                self.homed_from = State::Idle;
                self.end_homing(out);
            }
            Event::HomingFailed { detail } => {
                let back = match self.homed_from {
                    State::Alarm => State::Alarm,
                    _ if self.held.is_some() => State::Ready,
                    _ => State::Idle,
                };
                self.set_with(back, detail, false, out);
            }
            Event::Stopped if self.stop_pending => self.end_homing(out),
            _ => {}
        }
    }
}
