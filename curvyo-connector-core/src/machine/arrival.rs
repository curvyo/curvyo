//! How the job machine takes a submitted job and its chunks: the job becomes current or waits.

use uuid::Uuid;

use super::{BUSY_WORDS, Held, Machine, Output, Waiting};
use crate::assembly::{Assembly, ChunkError};
use crate::machine::People;
use crate::message::{ErrorCode, JobKind, Message, RejectReason, State};

impl Machine {
    pub(super) fn submit(
        &mut self,
        job_id: Uuid,
        name: String,
        kind: JobKind,
        (bytes, sha256): (u64, String),
        sender: &str,
        out: &mut Output,
    ) {
        let refusal = match self.state {
            State::NoController => Some(RejectReason::NoController),
            State::Alarm
            | State::Homing
            | State::Jogging
            | State::Framing
            | State::Focusing
            | State::Pulsing => Some(RejectReason::Busy),
            State::AwaitingLocalStart
                if matches!(
                    self.waiting,
                    Waiting::Home | Waiting::Frame | Waiting::Point
                ) =>
            {
                Some(RejectReason::Busy)
            }
            _ if self.laser_off => Some(RejectReason::LaserModeOff),
            _ if self.laser_unknown => Some(RejectReason::LaserModeUnknown),
            _ => None,
        };
        if let Some(reason) = refusal {
            Self::reject(out, job_id, reason);
        } else if self.knows(job_id) {
            // The same id twice is no second job.
            Self::reject(out, job_id, RejectReason::Busy);
        } else if !self.becomes_current() {
            self.submit_waiting((job_id, name, kind), (bytes, sha256), sender, out);
        } else if self
            .word
            .as_deref()
            .is_some_and(|w| BUSY_WORDS.contains(&w))
        {
            Self::reject(out, job_id, RejectReason::Busy);
        } else {
            match Assembly::begin(job_id, name, kind, bytes, sha256) {
                Err(reason) => Self::reject(out, job_id, reason),
                Ok(assembly) => {
                    sender.clone_into(&mut self.sender);
                    self.held = None;
                    self.set(State::Receiving, out);
                    if assembly.is_full() {
                        self.complete(assembly, out);
                    } else {
                        self.assembly = Some(assembly);
                    }
                }
            }
        }
    }

    pub(super) fn chunk(&mut self, job_id: Uuid, index: u32, text: &str, out: &mut Output) {
        if self.chunk_frame(job_id, index, text, out) {
            return;
        }
        let current = self.assembly.as_ref().is_some_and(|a| a.job_id() == job_id);
        if !current {
            if !self.chunk_waiting(job_id, index, text, out) {
                Self::error(out, ErrorCode::UnknownJob, "No job with this id is held.");
            }
            return;
        }
        let Some(assembly) = self.assembly.as_mut() else {
            return;
        };
        match assembly.push(job_id, index, text) {
            Ok(()) if assembly.is_full() => {
                if let Some(assembly) = self.assembly.take() {
                    self.complete(assembly, out);
                }
            }
            Ok(()) => {}
            Err(ChunkError::UnknownJob) => {
                Self::error(out, ErrorCode::UnknownJob, "No job with this id is held.");
            }
            Err(ChunkError::Reject(reason)) => {
                let held_id = assembly.job_id();
                self.assembly = None;
                Self::reject(out, held_id, reason);
                self.set(State::Idle, out);
            }
        }
    }

    fn complete(&mut self, assembly: Assembly, out: &mut Output) {
        let job_id = assembly.job_id();
        let sha256 = assembly.sha256().to_owned();
        match assembly.finish() {
            Ok(job) => {
                let lines = u32::try_from(job.text.lines().count()).unwrap_or(u32::MAX);
                self.held = Some(Held {
                    job,
                    sha256,
                    lines,
                    line: 0,
                    elapsed_ms: 0,
                    people: People {
                        sent_by: std::mem::take(&mut self.sender),
                        started_by: None,
                        confirmed_by: None,
                    },
                });
                out.messages.push(Message::JobAccepted {
                    job_id,
                    position: None,
                });
                self.set(State::Ready, out);
            }
            Err(reason) => {
                Self::reject(out, job_id, reason);
                self.set(State::Idle, out);
            }
        }
    }
}
