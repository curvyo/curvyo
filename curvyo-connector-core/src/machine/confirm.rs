//! Who has to agree before a start runs (architect addendum D5): the start rule, a member's
//! request, an owner's confirmation and the people of the held job.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ControllerCommand, Machine, Output, Waiting};
use crate::admin::Role;
use crate::message::{ErrorCode, Message, State};

/// Who has to agree before a `Start` (or a member's `Resume`) runs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StartRule {
    /// It runs at once (foreground mode without `--require-local-start`).
    #[default]
    Immediate,
    /// It waits for Enter on the connector's terminal (foreground `--require-local-start`).
    LocalPress,
    /// A member's start or resume waits until an owner sends `Start` (configured mode).
    OwnerConfirms,
}

/// Who sent a message: a client's name and role. The host itself (terminal, signal, reset) is
/// an owner without a name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Peer {
    /// The client's name; empty for the host.
    pub name: String,
    /// Its role.
    pub role: Role,
}

impl Peer {
    /// A client.
    #[must_use]
    pub fn new(name: impl Into<String>, role: Role) -> Self {
        Self {
            name: name.into(),
            role,
        }
    }

    /// The connector's host: an owner without a name.
    #[must_use]
    pub fn host() -> Self {
        Self::new("", Role::Owner)
    }
}

/// What a confirmation let run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confirms {
    /// A job's start.
    Start,
    /// A paused job's resume.
    Resume,
    /// A homing cycle (`specs/0118` 37).
    Home,
    /// A member's frame (`specs/0118` 2b).
    Frame,
}

/// A start, resume or home that waited and ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Confirmation {
    /// The job; the frame for a Frame; `None` for a Home.
    pub job_id: Option<Uuid>,
    /// Who asked for it.
    pub requested_by: String,
    /// The owner who agreed; `None` for Enter on the connector's terminal.
    pub confirmed_by: Option<String>,
    /// What ran.
    pub what: Confirms,
}

/// The clients behind the held job, for its `jobs.log` line (A15).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct People {
    /// Who sent it.
    pub sent_by: String,
    /// Who pressed Start.
    pub started_by: Option<String>,
    /// The owner who confirmed that start or a later resume.
    pub confirmed_by: Option<String>,
}

impl Machine {
    /// The clients behind the held job.
    #[must_use]
    pub fn people(&self) -> Option<&People> {
        self.held.as_ref().map(|h| &h.people)
    }

    /// Whether a paused job waits for an owner to confirm its resume: a job the controller holds.
    #[must_use]
    pub fn waits_for_resume(&self) -> bool {
        self.state == State::AwaitingLocalStart && self.waiting == Waiting::Resume
    }

    /// The member whose request waits, as `Status.requested_by` carries it (owner rule only).
    pub(super) fn shown_requester(&self) -> Option<String> {
        let shown = self.options.start_rule == StartRule::OwnerConfirms
            && self.state == State::AwaitingLocalStart;
        self.requester.clone().filter(|_| shown)
    }

    /// The held job as the owner rule's `Status` names it (configured mode only).
    pub(super) fn shown_job(&self) -> Option<&super::Held> {
        // A waiting Home starts no job, so it names none (`specs/0118` M5).
        let waits_home = self.state == State::AwaitingLocalStart
            && matches!(self.waiting, Waiting::Home | Waiting::Frame);
        let shown = self.options.start_rule == StartRule::OwnerConfirms
            && !waits_home
            && matches!(
                self.state,
                State::Ready
                    | State::AwaitingLocalStart
                    | State::Running
                    | State::Paused
                    | State::Homing
                    | State::Jogging
                    | State::Framing
                    | State::Done { .. }
            );
        self.held.as_ref().filter(|_| shown)
    }

    /// The owner rule's part of `Resume`, `Stop` and the confirming `Home`; true when the message
    /// was taken here.
    pub(super) fn takes_as_request(
        &mut self,
        message: &Message,
        peer: &Peer,
        out: &mut Output,
    ) -> bool {
        if self.options.start_rule != StartRule::OwnerConfirms {
            return false;
        }
        let member = peer.role == Role::Member;
        match message {
            Message::Home
                if self.state == State::AwaitingLocalStart && self.waiting == Waiting::Home =>
            {
                // An owner's Home confirms the waiting one; a second member's is refused.
                if member {
                    Self::wrong_state(out);
                } else {
                    self.run_waiting(Some(peer), out);
                }
            }
            Message::Resume if member && self.state == State::Paused && !self.stop_pending => {
                self.request(Waiting::Resume, peer, out);
            }
            Message::Resume if member && self.state == State::AwaitingLocalStart => {
                Self::wrong_state(out);
            }
            Message::Resume if self.waits_for_resume() => self.run_waiting(Some(peer), out),
            Message::Stop if self.waits_for_resume() => {
                // A waiting resume is a paused job: it stops through the controller (G12).
                self.state = State::Paused;
                self.requester = None;
                self.control(&Message::Stop, out);
            }
            _ => return false,
        }
        true
    }

    /// `Start { job_id }` from `peer`.
    pub(super) fn start(&mut self, job_id: Uuid, peer: &Peer, out: &mut Output) {
        // The first waiting job becomes current, `Ready`, and starts as any `Ready` job does.
        let _promoted = self.promote(job_id, peer, out);
        let held_id = self.held.as_ref().map(|h| h.job.job_id);
        let rule = self.options.start_rule;
        let owner_wait = rule == StartRule::OwnerConfirms
            && self.state == State::AwaitingLocalStart
            && self.waiting != Waiting::Home;
        let waited_id = if self.waiting == Waiting::Frame {
            self.frame_id()
        } else {
            held_id
        };
        if owner_wait {
            if peer.role == Role::Member {
                Self::wrong_state(out);
            } else if waited_id != Some(job_id) {
                self.not_held(job_id, out);
            } else {
                self.run_waiting(Some(peer), out);
            }
            return;
        }
        if self.state != State::Ready {
            Self::wrong_state(out);
            if !self.list.is_empty() {
                self.refresh_list(out);
            }
        } else if held_id != Some(job_id) {
            self.not_held(job_id, out);
        } else if rule == StartRule::LocalPress
            || (rule == StartRule::OwnerConfirms && peer.role == Role::Member)
        {
            self.request(Waiting::Start, peer, out);
        } else {
            if let Some(held) = self.held.as_mut() {
                held.people.started_by = Some(peer.name.clone());
            }
            out.commands.push(ControllerCommand::Start { job_id });
            self.set(State::Running, out);
        }
    }

    /// A `Start` for a job that is not the held one: `WrongState` when the list holds it (it may
    /// not start now), `UnknownJob` when nothing does.
    fn not_held(&mut self, job_id: Uuid, out: &mut Output) {
        if self.list.get(job_id).is_some() {
            Self::wrong_state(out);
            self.refresh_list(out);
        } else {
            Self::error(out, ErrorCode::UnknownJob, "No job with this id is held.");
        }
    }

    pub(super) fn request(&mut self, waiting: Waiting, peer: &Peer, out: &mut Output) {
        self.waiting = waiting;
        self.requester = Some(peer.name.clone());
        self.set(State::AwaitingLocalStart, out);
    }

    /// Runs the start or resume that waits; `owner` is `None` for Enter on the terminal.
    pub(super) fn run_waiting(&mut self, owner: Option<&Peer>, out: &mut Output) {
        let requested_by = self.requester.take().unwrap_or_default();
        let resume = self.waiting == Waiting::Resume;
        let confirmed_by = owner.map(|p| p.name.clone());
        if self.waiting == Waiting::Frame {
            out.confirmed = Some(Confirmation {
                job_id: self.frame_id(),
                requested_by,
                confirmed_by,
                what: Confirms::Frame,
            });
            self.run_frame(out);
            return;
        }
        if self.waiting == Waiting::Home {
            out.confirmed = Some(Confirmation {
                job_id: None,
                requested_by,
                confirmed_by,
                what: Confirms::Home,
            });
            out.commands.push(ControllerCommand::Home);
            self.set(State::Homing, out);
            return;
        }
        let Some(held) = self.held.as_mut() else {
            self.set(State::Idle, out);
            return;
        };
        let job_id = held.job.job_id;
        if !resume {
            held.people.started_by = Some(requested_by.clone());
        }
        held.people.confirmed_by.clone_from(&confirmed_by);
        out.confirmed = Some(Confirmation {
            job_id: Some(job_id),
            requested_by,
            confirmed_by,
            what: if resume {
                Confirms::Resume
            } else {
                Confirms::Start
            },
        });
        if resume {
            self.state = State::Paused;
            self.control(&Message::Resume, out);
            if self.state == State::Paused {
                // A confirming controller resumes on `Resumed`; until then the job is paused.
                out.messages.push(self.status());
            }
        } else {
            out.commands.push(ControllerCommand::Start { job_id });
            self.set(State::Running, out);
        }
    }
}
