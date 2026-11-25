//! How the job machine takes a go to point and its pulse (`specs/0118` 2d, adrs P3, P6): the move
//! laser off, the pulse only after arrival, and every way either ends.

use std::time::Duration;

use super::{BUSY_WORDS, ControllerCommand, Event, Machine, Output, Peer, StartRule, Waiting};
use crate::focus::{PULSE_GRACE, Pulse, arrived, clamp_target};
use crate::jog::JogStep;
use crate::message::{ErrorCode, Message, State};
use crate::millimetres::Millimetres;

/// The sentence of a move that ended away from its target, so no pulse fired.
pub const MISSED: &str = "Did not reach the point";
/// The sentence of a pulse the connector stopped because it did not end in time.
pub const PULSE_LATE: &str = "The pulse did not end";

/// A go to point that waits, moves or pulses.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct FocusRun {
    /// The target in job file coordinates; the clamped one once the controller named it.
    target: [Millimetres; 2],
    /// The pulse asked for.
    pulse: Option<Pulse>,
    /// Who pressed.
    client: String,
    /// The move's jog cancel was given.
    cancelling: bool,
    /// When a running pulse counts as not ended: its start, its length and the grace.
    deadline: Option<Duration>,
    /// The sentence the go to point ends with after its stop.
    failure: Option<String>,
}

impl Machine {
    /// The client whose go to point waits, moves or pulses.
    #[must_use]
    pub fn focus_client(&self) -> Option<&str> {
        self.focus.as_ref().map(|f| f.client.as_str())
    }

    /// The sentence of the stop the connector began itself for the go to point.
    pub(super) fn focus_failure(&self) -> Option<&str> {
        self.focus.as_ref().and_then(|f| f.failure.as_deref())
    }

    /// The target `Status.target` carries: while the go to point moves, pulses or waits.
    pub(super) fn shown_target(&self) -> Option<[Millimetres; 2]> {
        let shown = matches!(self.state, State::Focusing | State::Pulsing)
            || (self.state == State::AwaitingLocalStart && self.waiting == Waiting::Point);
        self.focus.as_ref().filter(|_| shown).map(|f| f.target)
    }

    fn point_refusal(&self) -> Option<String> {
        let busy_word = self
            .word
            .as_deref()
            .filter(|word| BUSY_WORDS.contains(word))
            .map(|word| format!("Controller state: {word}."));
        let words = match self.state {
            State::Idle | State::Ready | State::Done { .. } => {
                return busy_word.or_else(|| (!self.homed).then(|| "Not homed.".to_owned()));
            }
            State::Alarm => "Alarm.",
            State::Homing => "Homing.",
            State::Jogging => "Jogging.",
            State::Framing => "Framing.",
            State::Focusing => "Focus point.",
            State::Pulsing => "Pulse.",
            State::Running | State::Paused => "A job is running.",
            State::Receiving => "A job is arriving.",
            State::AwaitingLocalStart => return Some(self.waiting_words().to_owned()),
            State::NoController => "No controller.",
        };
        Some(words.to_owned())
    }

    /// `GoToPoint` from `peer`: taken in `Idle`, `Ready` or `Done` of a homed controller with no
    /// busy word (a `Ready` job is kept) with its target clamped into the travel the controller
    /// named, waits for Enter under `StartRule::LocalPress`, and is refused with its cause in
    /// every other state.
    pub(super) fn go_to_point(
        &mut self,
        target: [Millimetres; 2],
        pulse: Option<Pulse>,
        peer: &Peer,
        out: &mut Output,
    ) {
        if let Some(message) = self.point_refusal() {
            Self::error(out, ErrorCode::WrongState, &message);
            return;
        }
        self.motion_from = self.state;
        let wco = self.wco.map_or([Millimetres(0.0); 2], |w| [w[0], w[1]]);
        let target = clamp_target(target, wco, self.travel.as_ref());
        self.focus = Some(FocusRun {
            target,
            pulse,
            client: peer.name.clone(),
            cancelling: false,
            deadline: None,
            failure: None,
        });
        if self.options.start_rule == StartRule::LocalPress {
            self.waiting = Waiting::Point;
            self.set(State::AwaitingLocalStart, out);
        } else {
            self.run_point(out);
        }
    }

    /// The go to point's move goes to the controller.
    pub(super) fn run_point(&mut self, out: &mut Output) {
        let Some(target) = self.focus.as_ref().map(|f| f.target) else {
            self.set(State::Idle, out);
            return;
        };
        out.commands.push(ControllerCommand::GoTo {
            target,
            feed: JogStep::Ten.feed(),
        });
        self.set(State::Focusing, out);
    }

    /// Back to the state the go to point started from (`Idle`, `Ready` with the kept job, or
    /// `Done`), with `detail` when it ended short.
    pub(super) fn end_focus(&mut self, detail: Option<String>, out: &mut Output) {
        self.focus = None;
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

    /// Stop while the go to point moves (the jog cancel, position kept, no pulse after it) or
    /// pulses (the stop of `0127` G12); without confirmation the machine is back at once.
    pub(super) fn stop_focus(&mut self, out: &mut Output) {
        if self.state == State::Focusing {
            let Some(run) = self.focus.as_mut().filter(|r| !r.cancelling) else {
                return;
            };
            run.cancelling = true;
            out.commands.push(ControllerCommand::JogCancel);
            if self.confirm {
                self.stop_pending = true;
                out.messages.push(self.status());
            } else {
                self.end_focus(None, out);
            }
            return;
        }
        if self.stop_pending {
            return;
        }
        out.commands.push(ControllerCommand::Stop);
        if self.confirm {
            self.stop_pending = true;
            out.messages.push(self.status());
        } else {
            let failure = self.focus.as_mut().and_then(|f| f.failure.take());
            self.end_focus(failure, out);
        }
    }

    /// Whether the pulse may fire now that the move ended: asked for, not cancelled, at the
    /// target, no stop pending, no busy word, and the controller's laser mode on.
    fn pulse_allowed(&self) -> Option<Pulse> {
        let run = self.focus.as_ref()?;
        let pulse = run.pulse?;
        let here = self.position.is_some_and(|p| arrived(p, run.target));
        let clear = !run.cancelling
            && !self.stop_pending
            && self.word.is_none()
            && !self.laser_off
            && !self.laser_unknown;
        (here && clear).then_some(pulse)
    }

    /// The controller's events while `Focusing`.
    pub(super) fn on_focusing(&mut self, event: Event, out: &mut Output) {
        match event {
            Event::GoToTarget { target } => {
                if let Some(run) = self.focus.as_mut() {
                    run.target = target;
                }
                out.messages.push(self.status());
            }
            Event::JogEnded => {
                if let Some(pulse) = self.pulse_allowed() {
                    out.commands.push(ControllerCommand::Pulse {
                        power: pulse.power_permille,
                        length: pulse.length_ms,
                    });
                    let deadline = self.now + pulse.length_ms.duration() + PULSE_GRACE;
                    if let Some(run) = self.focus.as_mut() {
                        run.deadline = Some(deadline);
                    }
                    self.set(State::Pulsing, out);
                    return;
                }
                let missed = self.focus.as_ref().is_some_and(|run| {
                    run.pulse.is_some()
                        && !run.cancelling
                        && !self.position.is_some_and(|p| arrived(p, run.target))
                });
                self.end_focus(missed.then(|| MISSED.to_owned()), out);
            }
            Event::JogFailed { detail } => self.end_focus(Some(detail), out),
            Event::Stopped | Event::ControllerReset => self.end_focus(None, out),
            _ => {}
        }
    }

    /// The controller's events while `Pulsing`.
    pub(super) fn on_pulsing(&mut self, event: Event, out: &mut Output) {
        let detail = match event {
            Event::PulseEnded | Event::PulseSkipped | Event::Stopped | Event::ControllerReset => {
                None
            }
            Event::PulseFailed { detail } => Some(detail),
            _ => return,
        };
        let failure = self.focus.as_mut().and_then(|f| f.failure.take());
        self.end_focus(failure.or(detail), out);
    }

    /// The clock while `Pulsing`: a pulse not ended within its length and the grace is stopped.
    pub(super) fn pulse_clock(&mut self, now: Duration, out: &mut Output) {
        if self.state != State::Pulsing || self.stop_pending {
            return;
        }
        let Some(run) = self.focus.as_mut() else {
            return;
        };
        if run.deadline.is_some_and(|deadline| now >= deadline) {
            run.failure = Some(PULSE_LATE.to_owned());
            self.stop_focus(out);
        }
    }

    /// A busy word (`Door`, `Check`, `Sleep`) while the pulse runs: the stop of G12.
    pub(super) fn word_during_pulse(&mut self, out: &mut Output) {
        let busy = self
            .word
            .as_deref()
            .is_some_and(|word| BUSY_WORDS.contains(&word));
        if self.state == State::Pulsing && busy {
            self.stop_focus(out);
        }
    }

    /// The messages that do not fit a running go to point, answered with `WrongState`.
    pub(super) fn focus_refuses(&self, message: &Message, out: &mut Output) -> bool {
        let words = match self.state {
            State::Focusing => "Focus point.",
            State::Pulsing => "Pulse.",
            _ => return false,
        };
        match message {
            Message::Start { .. }
            | Message::Home
            | Message::Jog { .. }
            | Message::GoToPoint { .. }
            | Message::Pause
            | Message::Resume => {
                Self::error(out, ErrorCode::WrongState, words);
                true
            }
            // A release or keep-alive of a jog that ended is nobody's.
            Message::JogKeep | Message::JogStop => true,
            _ => false,
        }
    }
}
