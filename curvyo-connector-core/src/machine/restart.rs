//! How the job machine takes a restart from a line: its causes, the move to the line, an end
//! before it (`specs/0118` 2e, adrs R7, R8; `specs/0132` criterion 12).

use uuid::Uuid;

use super::{
    BUSY_WORDS, ControllerCommand, Event, Held, ListLog, Machine, Output, Peer, People, StartRule,
    Waiting, count,
};
use crate::admin::Role;
use crate::assembly::Job;
use crate::job_list::Item;
use crate::message::{ErrorCode, JobKind, ListEntry, Message, State};
use crate::millimetres::Millimetres;
use crate::modal::{self, Modal};
use crate::restart::{self, Now};

/// The restart an interrupted entry offers, computed once from its text.
#[derive(Debug, Clone)]
pub(super) struct Plan {
    job_id: Uuid,
    /// The line it begins at.
    line: u32,
    /// The state before that line, or why it cannot be rebuilt.
    modal: Result<Modal, String>,
}

impl Plan {
    fn of(item: &Item, blocks: u32) -> Option<Self> {
        let acked = item.entry.interrupted_at?;
        let run_from = item.entry.stop.as_ref().map_or(1, |s| s.run_from);
        let line = restart::restart_line(&item.text, acked, run_from, blocks);
        (line <= item.entry.lines).then(|| Self {
            job_id: item.entry.job_id,
            line,
            modal: modal::before(&item.text, line).map_err(|e| e.text(line)),
        })
    }
}

impl Machine {
    /// Computes the plan of every interrupted entry again (an entry came, or the planner size).
    pub(super) fn plan_all(&mut self) {
        let blocks = self.planner;
        let kept = self
            .back
            .as_ref()
            .and_then(|b| self.plan(b.entry.job_id))
            .cloned();
        self.plans = self
            .list
            .items()
            .iter()
            .filter_map(|item| Plan::of(item, blocks))
            .chain(kept)
            .collect();
    }

    fn plan(&self, job_id: Uuid) -> Option<&Plan> {
        self.plans.iter().find(|p| p.job_id == job_id)
    }

    /// Why the machine cannot take a restart now, in the order of `specs/0118` ux-notes W68 after
    /// the job text: the state, the controller's word, the laser mode, the position.
    fn machine_cause(&self, item: &Item) -> Option<String> {
        let state = match self.state {
            State::Idle | State::Done { .. } => None,
            State::Ready => Some("Another job is ready".to_owned()),
            State::Running | State::Paused => Some("A job is running".to_owned()),
            State::Receiving => Some("Sending".to_owned()),
            State::Homing => Some("Homing".to_owned()),
            State::Jogging => Some("Jogging".to_owned()),
            State::Framing => Some("Framing".to_owned()),
            State::AwaitingLocalStart => Some(match (self.shown_requester(), self.waiting) {
                (Some(r), Waiting::Home) => format!("Home requested by {r}"),
                (Some(r), Waiting::Frame) => format!("Frame requested by {r}"),
                (Some(r), _) => format!("{r} waits for an owner to confirm"),
                (None, _) => "Waiting for the start at the machine".to_owned(),
            }),
            State::Alarm => Some("Alarm".to_owned()),
            State::NoController => Some("No controller".to_owned()),
        };
        let word = || {
            self.word
                .as_deref()
                .filter(|w| BUSY_WORDS.contains(w))
                .map(|w| format!("Controller state: {w}"))
        };
        let laser = || {
            if self.laser_off {
                Some("Laser mode is off ($32=0)".to_owned())
            } else if self.laser_unknown {
                Some("Laser mode unknown ($32 not read)".to_owned())
            } else {
                None
            }
        };
        let now = Now {
            homed: self.homed,
            machine: self.mpos,
            offset: self.wco,
            unbroken: self.unbroken(item.entry.job_id),
        };
        let position = || {
            restart::position(item.entry.stop.as_ref(), &now)
                .err()
                .map(str::to_owned)
        };
        state.or_else(word).or_else(laser).or_else(position)
    }

    /// The first cause that refuses a restart of `item` now; `None` when it would run.
    fn restart_cause(&self, plan: &Plan, item: &Item) -> Option<String> {
        match &plan.modal {
            Err(cause) => Some(cause.clone()),
            Ok(_) => self.machine_cause(item),
        }
    }

    /// Adds the restart line, its cause and the crash flag to the interrupted entries.
    pub(super) fn decorate(&self, entries: &mut [ListEntry]) {
        for shown in entries.iter_mut().filter(|e| e.interrupted_at.is_some()) {
            let Some(item) = self.list.get(shown.job_id) else {
                continue;
            };
            shown.line_approximate = item.entry.stop.as_ref().is_none_or(|s| !s.exact);
            if let Some(plan) = self.plan(shown.job_id) {
                shown.restart_line = Some(plan.line);
                shown.restart_cause = self.restart_cause(plan, item);
            }
        }
    }

    /// `RestartFromLine { job_id, line }` from `peer`.
    pub(super) fn restart_from_line(
        &mut self,
        job_id: Uuid,
        line: u32,
        peer: &Peer,
        out: &mut Output,
    ) {
        let Some(item) = self
            .list
            .get(job_id)
            .filter(|i| i.entry.interrupted_at.is_some())
        else {
            return Self::error(
                out,
                ErrorCode::UnknownJob,
                "No interrupted job with this id.",
            );
        };
        if self.options.start_rule == StartRule::OwnerConfirms && peer.role == Role::Member {
            return Self::error(out, ErrorCode::OwnerRequired, "Only an owner may restart.");
        }
        let Some(plan) = self.plan(job_id).filter(|p| p.line == line) else {
            // The entry changed under the press: the app gets the list as it is now.
            Self::wrong_state(out);
            return self.refresh_list(out);
        };
        if let Some(cause) = self.restart_cause(plan, item) {
            return Self::error(out, ErrorCode::WrongState, &cause);
        }
        let Some(item) = self.list.remove(job_id) else {
            return;
        };
        if peer.role == Role::Owner {
            self.note = None;
        }
        out.list_log.push(ListLog::Restarted {
            job_id,
            name: item.entry.name.clone(),
            line,
            by: peer.name.clone(),
        });
        self.held = Some(Held {
            job: Job {
                job_id,
                name: item.entry.name.clone(),
                kind: JobKind::Job,
                text: std::sync::Arc::clone(&item.text),
            },
            sha256: item.entry.sha256.clone(),
            lines: item.entry.lines,
            line: line - 1,
            elapsed_ms: 0,
            people: People {
                sent_by: item.entry.sent_by.clone(),
                started_by: Some(peer.name.clone()),
                confirmed_by: None,
            },
            from_line: Some(line),
        });
        self.back = Some(item);
        if self.options.start_rule == StartRule::LocalPress {
            self.waiting = Waiting::Restart;
            self.set(State::AwaitingLocalStart, out);
        } else {
            self.run_restart(out);
        }
    }

    /// The restart goes to the controller: the entry lines, then the job from its line.
    pub(super) fn run_restart(&mut self, out: &mut Output) {
        let Some(held) = self.held.as_ref() else {
            return;
        };
        let job_id = held.job.job_id;
        let Some((line, Ok(modal))) = self.plan(job_id).map(|p| (p.line, &p.modal)) else {
            return self.give_back(None, out);
        };
        let z_now = self
            .mpos
            .zip(self.wco)
            .map(|(machine, offset)| machine[2].0 - offset[2].0);
        let [x, y, _] = modal.position;
        out.commands.push(ControllerCommand::StartAt {
            job_id,
            from_line: line,
            entry: restart::entry(modal, z_now),
            target: [
                Millimetres(x.unwrap_or_default()),
                Millimetres(y.unwrap_or_default()),
            ],
            homed: self.homed,
        });
        self.entering = true;
        self.set(State::Running, out);
    }

    /// The entry goes back to the list as it was, nothing has burned: no job result.
    fn take_back(&mut self, out: &mut Output) -> Option<u32> {
        let item = self.back.take()?;
        let line = self.held.as_ref().and_then(|h| h.from_line).unwrap_or(0);
        self.entering = false;
        self.held = None;
        self.requester = None;
        self.stop_pending = false;
        self.pause_pending = false;
        out.list_log.push(ListLog::RestartReturned {
            job_id: item.entry.job_id,
            name: item.entry.name.clone(),
            line,
        });
        self.list.push(item);
        Some(line)
    }

    /// Back to `Idle` with the entry in the list and `detail`, or "Not started: Restart from
    /// line <n>".
    fn give_back(&mut self, detail: Option<String>, out: &mut Output) {
        let Some(line) = self.take_back(out) else {
            return;
        };
        let detail =
            detail.unwrap_or_else(|| format!("Not started: Restart from line {}", count(line)));
        self.set_with(State::Idle, detail, false, out);
    }

    /// The events that end a restart before its line, or tell that it reached it; true when the
    /// event was taken here.
    pub(super) fn before_line(&mut self, event: &Event, out: &mut Output) -> bool {
        if self.back.is_none() {
            return false;
        }
        match event {
            Event::Message(Message::Stop, _) => match self.state {
                State::AwaitingLocalStart => self.give_back(None, out),
                State::Running | State::Paused => {
                    if !self.stop_pending {
                        out.commands.push(ControllerCommand::Stop);
                        if self.confirm {
                            self.stop_pending = true;
                            out.messages.push(self.status());
                        } else {
                            self.give_back(None, out);
                        }
                    }
                }
                _ => return false,
            },
            Event::Stopped if self.stop_pending => self.give_back(None, out),
            Event::JobFailed { .. } => self.give_back(None, out),
            Event::JobFailedWith { detail, .. } => self.give_back(Some(detail.clone()), out),
            Event::Alarm { .. } => {
                // The alarm is shown as any; the entry waits in the list, the chain is untouched.
                let _line = self.take_back(out);
                self.state = State::Idle;
                return false;
            }
            Event::ControllerLost | Event::ControllerLostWith { .. } | Event::StopUnconfirmed => {
                let text = match event {
                    Event::ControllerLostWith { detail } => detail.clone(),
                    Event::StopUnconfirmed => "Could not confirm the machine stopped".to_owned(),
                    _ => "Lost the controller".to_owned(),
                };
                let _line = self.take_back(out);
                self.state = State::Idle;
                self.lose(Some(&text), out);
            }
            Event::EntryReached if self.entering => {
                self.reached();
                out.messages.push(self.status());
            }
            Event::JobFinished { .. } => {
                self.reached();
                return false;
            }
            _ => return false,
        }
        true
    }

    /// The head stands at the line: from now on the run is a job as any.
    fn reached(&mut self) {
        self.entering = false;
        self.back = None;
        self.plan_all();
    }

    /// The entry a restart that has not reached its line keeps for the list.
    pub(super) fn returning(&self) -> Option<&Item> {
        self.back.as_ref()
    }
}
