//! How the job machine keeps the job list: arrival while a job is held, the list messages, the
//! chain and what the state directory stores (`specs/0132`).

use std::sync::Arc;

use uuid::Uuid;

use super::{BUSY_WORDS, Event, Held, Machine, Output, Peer, People};
use crate::admin::Role;
use crate::assembly::{Assembly, ChunkError, Job};
use crate::job_list::{
    Action, Chain, End, Entry, Item, STORED_FORMAT, StoredCurrent, StoredList, may, next,
};
use crate::message::{ErrorCode, JobKind, Message, RejectReason, State};

/// A job whose chunks arrive while another job is held.
#[derive(Debug, Clone)]
pub(super) struct Transfer {
    pub(super) assembly: Assembly,
    pub(super) sender: String,
}

/// What a list change leaves for `admin.log` (`specs/0132` L6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListLog {
    /// An owner removed a job another client sent.
    Removed {
        /// The job.
        job_id: Uuid,
        /// Its name.
        name: String,
        /// Who sent it.
        sent_by: String,
        /// The owner.
        by: String,
    },
    /// A job's "Run next automatically" changed.
    RunNext {
        /// The job.
        job_id: Uuid,
        /// Its name.
        name: String,
        /// Who changed it.
        by: String,
        /// The new value.
        on: bool,
    },
}

impl Machine {
    /// The `JobList` message for the current state.
    #[must_use]
    pub fn job_list(&self) -> Message {
        let current = self
            .held
            .as_ref()
            .filter(|_| !matches!(self.state, State::Receiving))
            .map(|h| entry_of(h).shown(0));
        Message::JobList {
            current,
            waiting: self.list.shown(),
            note: self.note.clone(),
        }
    }

    /// Emits the `JobList` when it changed; a list that holds nothing waiting and says nothing
    /// is not pushed for a change of the current job alone (the `Status` shows that).
    pub(super) fn announce_list(&mut self, out: &mut Output) {
        let message = self.job_list();
        if self.last_list.as_ref() == Some(&message) {
            return;
        }
        let trivial = |m: &Message| matches!(m, Message::JobList { waiting, note, .. } if waiting.is_empty() && note.is_none());
        if self.last_list.as_ref().is_none_or(trivial) && trivial(&message) {
            return;
        }
        self.last_list = Some(message.clone());
        out.messages.push(message);
    }

    pub(super) fn refresh_list(&mut self, out: &mut Output) {
        let message = self.job_list();
        self.last_list = Some(message.clone());
        out.messages.push(message);
    }

    /// Whether this connector has no owners: every paired client acts as one.
    fn foreground(&self) -> bool {
        self.options.start_rule != super::StartRule::OwnerConfirms
    }

    /// A `SubmitJob` while a job is held or arriving: a transfer for the list.
    pub(super) fn submit_waiting(
        &mut self,
        (job_id, name, kind): (Uuid, String, JobKind),
        (bytes, sha256): (u64, String),
        sender: &str,
        out: &mut Output,
    ) {
        if kind == JobKind::Frame {
            Self::reject(out, job_id, RejectReason::Busy);
            return;
        }
        let in_flight_bytes = self.transfers.iter().map(|t| t.assembly.bytes()).sum();
        if !self.list.fits(self.transfers.len(), in_flight_bytes, bytes) {
            Self::reject(out, job_id, RejectReason::ListFull);
            return;
        }
        match Assembly::begin(job_id, name, kind, bytes, sha256) {
            Err(reason) => Self::reject(out, job_id, reason),
            Ok(assembly) => {
                let transfer = Transfer {
                    assembly,
                    sender: sender.to_owned(),
                };
                if transfer.assembly.is_full() {
                    self.complete_waiting(transfer, out);
                } else {
                    self.transfers.push(transfer);
                }
            }
        }
    }

    /// A chunk of a transfer for the list; false when no such transfer exists.
    pub(super) fn chunk_waiting(
        &mut self,
        job_id: Uuid,
        index: u32,
        text: &str,
        out: &mut Output,
    ) -> bool {
        let Some(at) = self
            .transfers
            .iter()
            .position(|t| t.assembly.job_id() == job_id)
        else {
            return false;
        };
        match self.transfers[at].assembly.push(job_id, index, text) {
            Ok(()) if self.transfers[at].assembly.is_full() => {
                let transfer = self.transfers.remove(at);
                self.complete_waiting(transfer, out);
            }
            Ok(()) => {}
            Err(ChunkError::UnknownJob) => {
                Self::error(out, ErrorCode::UnknownJob, "No job with this id is held.");
            }
            Err(ChunkError::Reject(reason)) => {
                self.transfers.remove(at);
                Self::reject(out, job_id, reason);
            }
        }
        true
    }

    fn complete_waiting(&mut self, transfer: Transfer, out: &mut Output) {
        let job_id = transfer.assembly.job_id();
        let bytes = transfer.assembly.bytes();
        let sha256 = transfer.assembly.sha256().to_owned();
        match transfer.assembly.finish() {
            Ok(job) => {
                let entry = Entry {
                    job_id,
                    name: job.name.clone(),
                    lines: count_lines(&job.text),
                    bytes,
                    sha256,
                    sent_by: transfer.sender,
                    run_next_by: None,
                    interrupted_at: None,
                };
                let position = self.list.push(Item {
                    entry,
                    text: job.text,
                });
                out.messages.push(Message::JobAccepted {
                    job_id,
                    position: Some(position),
                });
            }
            Err(reason) => Self::reject(out, job_id, reason),
        }
    }

    /// A client's transfer ended before it was complete: only its own assembly is dropped.
    pub(super) fn abandon(&mut self, job_id: Uuid, out: &mut Output) {
        self.transfers.retain(|t| t.assembly.job_id() != job_id);
        let receiving = self.state == State::Receiving
            && self.assembly.as_ref().is_some_and(|a| a.job_id() == job_id);
        if receiving {
            self.discard();
            self.set(State::Idle, out);
        }
    }

    /// `MoveJob`, `RemoveJob` and `SetRunNext`.
    pub(super) fn list_message(&mut self, message: &Message, peer: &Peer, out: &mut Output) {
        match *message {
            Message::MoveJob { job_id, from, to } => {
                let Some(item) = self.list.get(job_id) else {
                    return self.unknown_or_current(job_id, out);
                };
                if let Err(code) = may(
                    Action::Move,
                    peer,
                    &item.entry.sent_by,
                    item.entry.interrupted_at.is_some(),
                    self.foreground(),
                ) {
                    return Self::error(out, code, "Only an owner may reorder the list.");
                }
                if self.list.move_to(job_id, from, to).is_err() {
                    Self::wrong_state(out);
                    self.refresh_list(out);
                }
            }
            Message::RemoveJob { job_id } => self.remove(job_id, peer, out),
            Message::SetRunNext { job_id, on } => self.set_run_next(job_id, on, peer, out),
            _ => {}
        }
    }

    fn unknown_or_current(&mut self, job_id: Uuid, out: &mut Output) {
        if self.held.as_ref().is_some_and(|h| h.job.job_id == job_id) {
            Self::wrong_state(out);
            self.refresh_list(out);
        } else {
            Self::error(out, ErrorCode::UnknownJob, "No job with this id is listed.");
        }
    }

    fn remove(&mut self, job_id: Uuid, peer: &Peer, out: &mut Output) {
        let Some(item) = self.list.get(job_id) else {
            return self.unknown_or_current(job_id, out);
        };
        let (name, sent_by) = (item.entry.name.clone(), item.entry.sent_by.clone());
        let interrupted = item.entry.interrupted_at.is_some();
        if let Err(code) = may(
            Action::Remove,
            peer,
            &sent_by,
            interrupted,
            self.foreground(),
        ) {
            return Self::error(out, code, "Only an owner may remove this job.");
        }
        self.list.remove(job_id);
        if self
            .list
            .items()
            .iter()
            .all(|i| i.entry.interrupted_at.is_some())
        {
            self.note = None;
        }
        if peer.name != sent_by {
            out.list_log.push(ListLog::Removed {
                job_id,
                name,
                sent_by,
                by: peer.name.clone(),
            });
        }
    }

    fn set_run_next(&mut self, job_id: Uuid, on: bool, peer: &Peer, out: &mut Output) {
        let Some(item) = self.list.get(job_id) else {
            return self.unknown_or_current(job_id, out);
        };
        let name = item.entry.name.clone();
        if item.entry.interrupted_at.is_some() {
            return Self::wrong_state(out);
        }
        if let Err(code) = may(
            Action::SetRunNext,
            peer,
            &item.entry.sent_by,
            false,
            self.foreground(),
        ) {
            return Self::error(out, code, "Only an owner may mark another client's job.");
        }
        self.list.set_run_next(job_id, on.then(|| peer.clone()));
        out.list_log.push(ListLog::RunNext {
            job_id,
            name,
            by: peer.name.clone(),
            on,
        });
    }

    /// The first waiting job becomes the current job, `Ready`; false when `job_id` is not it or
    /// no job may become current now.
    pub(super) fn promote(&mut self, job_id: Uuid, peer: &Peer, out: &mut Output) -> bool {
        let usable = matches!(self.state, State::Idle | State::Done { .. })
            && self.assembly.is_none()
            && self
                .list
                .first_waiting()
                .is_some_and(|i| i.entry.job_id == job_id);
        if !usable {
            return false;
        }
        let Some(item) = self.list.remove(job_id) else {
            return false;
        };
        if peer.role == Role::Owner {
            self.note = None;
        }
        self.held = Some(Held {
            job: Job {
                job_id,
                name: item.entry.name,
                kind: JobKind::Job,
                text: item.text,
            },
            lines: item.entry.lines,
            line: 0,
            elapsed_ms: 0,
            sha256: item.entry.sha256,
            people: People {
                sent_by: item.entry.sent_by,
                started_by: None,
                confirmed_by: None,
            },
        });
        self.set(State::Ready, out);
        true
    }

    /// The current job ended as `end`: the chain starts the next job, stops, or does nothing.
    pub(super) fn job_ended(&mut self, end: End, out: &mut Output) {
        let first = self
            .list
            .first_waiting()
            .map(|i| (i.entry.job_id, i.entry.run_next_by.is_some()));
        let flagged = first.is_some_and(|(_, flag)| flag);
        match next(end, flagged, self.chain_ready()) {
            Chain::Nothing => {}
            Chain::Start => {
                out.follow_up = first.map(|(job_id, _)| Event::RunNext { job_id });
            }
            Chain::Stop => {
                if self.list.clear_flags() {
                    let word = match end {
                        End::Completed => self.not_ready_word(),
                        other => other.word().to_owned(),
                    };
                    self.note = Some(format!("Chain stopped: {word}"));
                }
            }
        }
    }

    /// Whether the controller can take a job now.
    fn chain_ready(&self) -> bool {
        self.state != State::NoController
            && !self.laser_off
            && !self.laser_unknown
            && self.busy_word().is_none()
    }

    fn busy_word(&self) -> Option<&str> {
        self.word.as_deref().filter(|w| BUSY_WORDS.contains(w))
    }

    fn not_ready_word(&self) -> String {
        self.busy_word().unwrap_or("Not ready").to_owned()
    }

    /// `RunNext`: the chain's decision when the job ended, carried out.
    pub(super) fn run_next(&mut self, job_id: Uuid, out: &mut Output) {
        let by = self
            .list
            .get(job_id)
            .and_then(|i| i.entry.run_next_by.clone());
        let Some(by) = by else { return };
        if self.state
            != (State::Done {
                result: crate::message::JobResult::Completed,
            })
        {
            return;
        }
        self.start(job_id, &by, out);
    }

    /// The list as `jobs/list.json` keeps it.
    #[must_use]
    pub fn stored(&self) -> StoredList {
        let current = self
            .held
            .as_ref()
            .filter(|_| {
                matches!(
                    self.state,
                    State::Ready
                        | State::AwaitingLocalStart
                        | State::Homing
                        | State::Running
                        | State::Paused
                )
            })
            .map(|h| StoredCurrent {
                entry: entry_of(h),
                started: matches!(self.state, State::Running | State::Paused)
                    || self.waits_for_resume(),
                line: h.line,
            });
        StoredList {
            format: STORED_FORMAT,
            current,
            waiting: self.list.items().iter().map(|i| i.entry.clone()).collect(),
        }
    }

    /// The texts `jobs/` keeps: the current job's and every listed job's.
    #[must_use]
    pub fn stored_texts(&self) -> Vec<(Uuid, Arc<str>)> {
        let stored_current = self.stored().current.is_some();
        let current = self
            .held
            .iter()
            .filter(|_| stored_current)
            .map(|h| (h.job.job_id, Arc::clone(&h.job.text)));
        let waiting = self
            .list
            .items()
            .iter()
            .map(|i| (i.entry.job_id, Arc::clone(&i.text)));
        current.chain(waiting).collect()
    }

    /// Takes every listed job out (a machine that is rebuilt hands them on).
    pub fn take_list(&mut self) -> Vec<Item> {
        std::mem::take(&mut self.list).into_items()
    }

    /// Puts back the jobs a restart found (never the current one): nothing starts.
    pub fn restore(&mut self, items: Vec<Item>) {
        for item in items {
            self.list.push(item);
        }
        self.last_list = Some(self.job_list());
    }

    /// Whether a job of this id is held, arriving or listed.
    pub(super) fn knows(&self, job_id: Uuid) -> bool {
        let over = matches!(self.state, State::Done { .. });
        self.held
            .as_ref()
            .is_some_and(|h| h.job.job_id == job_id && !over)
            || self.assembly.as_ref().is_some_and(|a| a.job_id() == job_id)
            || self.transfers.iter().any(|t| t.assembly.job_id() == job_id)
            || self.list.get(job_id).is_some()
    }

    /// Arrival of a job for the list, or its refusal: the cases of `submit` that do not make the
    /// job current.
    pub(super) fn becomes_current(&self) -> bool {
        matches!(self.state, State::Idle | State::Done { .. })
            && self.list.is_empty()
            && self.transfers.is_empty()
            && self.assembly.is_none()
    }
}

fn count_lines(text: &str) -> u32 {
    u32::try_from(text.lines().count()).unwrap_or(u32::MAX)
}

fn entry_of(held: &Held) -> Entry {
    Entry {
        job_id: held.job.job_id,
        name: held.job.name.clone(),
        lines: held.lines,
        bytes: held.job.text.len() as u64,
        sha256: held.sha256.clone(),
        sent_by: held.people.sent_by.clone(),
        run_next_by: None,
        interrupted_at: None,
    }
}
