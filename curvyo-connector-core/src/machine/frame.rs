//! How the job machine runs a frame job: its own slot beside the held job, the checks of its
//! text, a member's wait for an owner, and the controller's events while it runs (`specs/0118`
//! 2b, adrs F4, F5).

use uuid::Uuid;

use super::{BUSY_WORDS, ControllerCommand, Event, Machine, Output, Peer, StartRule, Waiting};
use crate::MAX_CHUNK_BYTES;
use crate::admin::Role;
use crate::assembly::{Assembly, ChunkError, Job};
use crate::frame_text;
use crate::message::{ErrorCode, JobKind, Message, RejectReason, State};

/// A frame that is arriving, waits for a confirmation or runs.
#[derive(Debug, Clone)]
pub(super) struct FrameRun {
    job_id: Uuid,
    /// The sender: who has to be confirmed, and with which role.
    peer: Peer,
    /// The chunk still to come; `None` once assembled.
    assembly: Option<Assembly>,
    /// The checked frame.
    job: Option<Job>,
}

/// A frame's failure without the line it names: "Failed at line 3: No feed rate (error 22)"
/// reads "No feed rate (error 22)", for the frame's lines are nobody's job lines.
fn without_line(detail: &str) -> &str {
    detail
        .strip_prefix("Failed at line ")
        .and_then(|rest| rest.split_once(": "))
        .map_or(detail, |(_, cause)| cause)
}

impl Machine {
    /// The frame that waits or runs, for the controller to stream; never the held job.
    #[must_use]
    pub fn frame_job(&self) -> Option<&Job> {
        self.frame.as_ref().and_then(|f| f.job.as_ref())
    }

    /// The id `Status.frame_id` carries: the frame that runs or waits for a confirmation.
    pub(super) fn shown_frame_id(&self) -> Option<Uuid> {
        let shown = self.state == State::Framing
            || (self.state == State::AwaitingLocalStart && self.waiting == Waiting::Frame);
        self.frame
            .as_ref()
            .filter(|f| shown && f.job.is_some())
            .map(|f| f.job_id)
    }

    /// Whether the controller reports a state in which nothing may move.
    fn word_blocks(&self) -> bool {
        self.word
            .as_deref()
            .is_some_and(|word| BUSY_WORDS.contains(&word))
    }

    fn frame_may_start(&self) -> bool {
        matches!(self.state, State::Idle | State::Ready | State::Done { .. }) && !self.word_blocks()
    }

    /// `SubmitJob` of kind `Frame`: assembled in the frame slot beside the held job, the state
    /// unchanged; a laser mode of the controller does not matter, the frame never fires.
    pub(super) fn submit_frame(
        &mut self,
        (job_id, name): (Uuid, String),
        (bytes, sha256): (u64, String),
        peer: &Peer,
        out: &mut Output,
    ) {
        let refusal = if self.state == State::NoController {
            Some(RejectReason::NoController)
        } else if !self.frame_may_start() || self.frame.is_some() || self.knows(job_id) {
            Some(RejectReason::Busy)
        } else if bytes > MAX_CHUNK_BYTES as u64 {
            Some(RejectReason::TooLarge)
        } else {
            None
        };
        if let Some(reason) = refusal {
            Self::reject(out, job_id, reason);
            return;
        }
        match Assembly::begin(job_id, name, JobKind::Frame, bytes, sha256) {
            Err(reason) => Self::reject(out, job_id, reason),
            Ok(assembly) => {
                let mut run = FrameRun {
                    job_id,
                    peer: peer.clone(),
                    assembly: None,
                    job: None,
                };
                if assembly.is_full() {
                    self.frame = Some(run);
                    self.complete_frame(assembly, out);
                } else {
                    run.assembly = Some(assembly);
                    self.frame = Some(run);
                }
            }
        }
    }

    /// A chunk of the frame; false when no frame is arriving under `job_id`.
    pub(super) fn chunk_frame(
        &mut self,
        job_id: Uuid,
        index: u32,
        text: &str,
        out: &mut Output,
    ) -> bool {
        let Some(assembly) = self
            .frame
            .as_mut()
            .filter(|f| f.job_id == job_id)
            .and_then(|f| f.assembly.as_mut())
        else {
            return false;
        };
        match assembly.push(job_id, index, text) {
            Ok(()) if assembly.is_full() => {
                if let Some(assembly) = self.frame.as_mut().and_then(|f| f.assembly.take()) {
                    self.complete_frame(assembly, out);
                }
            }
            Ok(()) => {}
            Err(ChunkError::UnknownJob) => {
                Self::error(out, ErrorCode::UnknownJob, "No job with this id is held.");
            }
            Err(ChunkError::Reject(reason)) => {
                self.frame = None;
                Self::reject(out, job_id, reason);
            }
        }
        true
    }

    /// The frame is whole: hash and ASCII were checked, now the words, then the state again.
    fn complete_frame(&mut self, assembly: Assembly, out: &mut Output) {
        let job_id = assembly.job_id();
        let job = match assembly.finish() {
            Ok(job) => job,
            Err(reason) => {
                self.frame = None;
                Self::reject(out, job_id, reason);
                return;
            }
        };
        let moves = frame_text::check(&job.text).map_or(0, |moves| moves.len());
        let reason = if moves == 0 {
            Some(RejectReason::NotAFrame)
        } else if !self.frame_may_start() {
            Some(RejectReason::Busy)
        } else {
            None
        };
        if let Some(reason) = reason {
            self.frame = None;
            Self::reject(out, job_id, reason);
            return;
        }
        let Some(run) = self.frame.as_mut() else {
            return;
        };
        let peer = run.peer.clone();
        run.job = Some(job);
        out.messages.push(Message::JobAccepted {
            job_id,
            position: None,
        });
        self.motion_from = self.state;
        let waits = self.options.start_rule == StartRule::LocalPress
            || (self.options.start_rule == StartRule::OwnerConfirms && peer.role == Role::Member);
        if waits {
            self.request(Waiting::Frame, &peer, out);
        } else {
            self.run_frame(out);
        }
    }

    /// The frame goes to the controller.
    pub(super) fn run_frame(&mut self, out: &mut Output) {
        let Some(job_id) = self.frame.as_ref().map(|f| f.job_id) else {
            return;
        };
        out.commands.push(ControllerCommand::Frame {
            job_id,
            homed: self.homed,
        });
        self.set(State::Framing, out);
    }

    /// The state a frame returns to: where it started, with the held job kept.
    fn back_from_frame(&self) -> State {
        match self.motion_from {
            State::Ready | State::Done { .. } if self.held.is_none() => State::Idle,
            state @ (State::Ready | State::Done { .. }) => state,
            _ => State::Idle,
        }
    }

    /// The frame is over, or a waiting one is discarded: back to where it started.
    pub(super) fn end_frame(&mut self, detail: Option<String>, out: &mut Output) {
        self.frame = None;
        self.requester = None;
        let back = self.back_from_frame();
        match detail {
            Some(detail) => self.set_with(back, detail, false, out),
            None => self.set(back, out),
        }
    }

    /// Stop while the frame runs: the controller is told at once; without confirmation the
    /// machine is back where the frame started, with it the stop waits for `Stopped`.
    pub(super) fn stop_frame(&mut self, out: &mut Output) {
        if self.stop_pending {
            return;
        }
        out.commands.push(ControllerCommand::Stop);
        if self.confirm {
            self.stop_pending = true;
            out.messages.push(self.status());
        } else {
            self.end_frame(None, out);
        }
    }

    /// The controller's events while `Framing`: the progress of the frame is nobody's.
    pub(super) fn on_framing(&mut self, event: Event, out: &mut Output) {
        match event {
            Event::JobFinished { .. } | Event::JobFailed { .. } | Event::ControllerReset => {
                self.end_frame(None, out);
            }
            Event::JobFailedWith { detail, .. } => {
                self.end_frame(Some(without_line(&detail).to_owned()), out);
            }
            Event::Stopped if self.stop_pending => self.end_frame(None, out),
            _ => {}
        }
    }

    /// The connection that sent a frame still arriving ended: the frame is dropped.
    pub(super) fn abandon_frame(&mut self, job_id: Uuid) {
        if self
            .frame
            .as_ref()
            .is_some_and(|f| f.job_id == job_id && f.assembly.is_some())
        {
            self.frame = None;
        }
    }

    /// The id of the frame in the slot.
    pub(super) fn frame_id(&self) -> Option<Uuid> {
        self.frame.as_ref().map(|f| f.job_id)
    }

    /// The messages that do not fit a running frame, answered with `WrongState`.
    pub(super) fn frame_refuses(&self, message: &Message, out: &mut Output) -> bool {
        if self.state != State::Framing {
            return false;
        }
        match message {
            Message::Start { .. }
            | Message::Home
            | Message::Jog { .. }
            | Message::Pause
            | Message::Resume => {
                Self::error(out, ErrorCode::WrongState, "Framing.");
                true
            }
            // A release or keep-alive of a jog that ended is nobody's.
            Message::JogKeep | Message::JogStop => true,
            _ => false,
        }
    }
}
