//! Which changes of the job machine reach `jobs.log`, `admin.log` and the terminal, with the
//! clients behind them (`0064` Q18, `specs/0127` G7, `specs/0128` A14, A15, A18).

use std::time::Duration;

use curvyo_connector_core::{
    Confirmation, Confirms, ListLog, Message, People, RejectReason, State,
};
use time::OffsetDateTime;
use uuid::Uuid;

use super::{Hub, Inner};
use crate::admin_log::AdminEvent;
use crate::hub_log::{logged_outcome, rejection_line, terminal_alarm};
use crate::job_log::{LogPeople, LogResult};

/// The shortest time between two writes of the running job's line.
const LINE_PERIOD: Duration = Duration::from_secs(1);

fn log_people(people: Option<&People>) -> LogPeople {
    people.map_or_else(LogPeople::default, |p| LogPeople {
        sent_by: p.sent_by.clone(),
        started_by: p.started_by.clone().unwrap_or_default(),
        confirmed_by: p.confirmed_by.clone().unwrap_or_default(),
    })
}

impl Hub {
    /// Appends one `jobs.log` line; a failure is a terminal line.
    pub(super) fn append_job_line(
        &self,
        inner: &Inner,
        name: &str,
        result: &LogResult,
        people: &LogPeople,
    ) {
        let at = OffsetDateTime::now_utc();
        // Any line ends the run of one client's jogs (`specs/0118` criterion 12).
        *self
            .jog_run
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
        if let Err(e) = self.job_log.append(at, name, result, people) {
            let _ = inner.notices.send(format!("Cannot write the job log: {e}"));
        }
    }

    /// Appends one `admin.log` line by `by` (empty: the host); a failure is a terminal line.
    pub(super) fn append_admin(&self, inner: &Inner, by: &str, event: &AdminEvent) {
        if let Err(e) = self.admin_log.append(OffsetDateTime::now_utc(), by, event) {
            let _ = inner
                .notices
                .send(format!("Cannot write the admin log: {e}"));
        }
    }

    /// A frame went to the controller: one `jobs.log` line `Frame: <client>` and one terminal
    /// line, never a result line (`specs/0118` 2b, adrs F6).
    pub(super) fn log_frame(&self, inner: &Inner, client: &str) {
        let _ = inner.notices.send(format!("Frame sent by {client}"));
        let result = LogResult::Frame(client.to_owned());
        self.append_job_line(inner, "", &result, &LogPeople::default());
    }

    /// The refusal of a submitted job: its log line and, for the settings refusals, its own
    /// terminal line (UX notes section 10).
    pub(super) fn log_rejection(&self, inner: &Inner, job_id: Uuid, reason: RejectReason) {
        let name = inner
            .submitted
            .iter()
            .rev()
            .find(|(id, _)| *id == job_id)
            .map(|(_, name)| name.clone())
            .unwrap_or_default();
        let result = LogResult::Rejected(format!("{reason:?}"));
        self.append_job_line(inner, &name, &result, &LogPeople::default());
        let line =
            rejection_line(reason).unwrap_or_else(|| format!("Job {name}: {}", result.text()));
        let _ = inner.notices.send(line);
    }

    /// A change of the job list that `admin.log` records (`specs/0132` L6).
    pub(super) fn log_list_change(&self, inner: &Inner, log: &ListLog) {
        match log {
            ListLog::Removed {
                job_id,
                name,
                sent_by,
                by,
            } => {
                let event = AdminEvent::JobRemoved {
                    job_name: name.clone(),
                    job_id: job_id.to_string(),
                    sent_by: sent_by.clone(),
                };
                self.append_admin(inner, by, &event);
            }
            ListLog::RunNext {
                job_id,
                name,
                by,
                on,
            } => {
                let event = AdminEvent::RunNextSet {
                    job_name: name.clone(),
                    job_id: job_id.to_string(),
                    on: *on,
                };
                self.append_admin(inner, by, &event);
            }
        }
    }

    /// What a restart dropped from the stored job list, one `admin.log` line each.
    pub(super) fn log_restore(&self, loaded: &crate::job_store::Loaded) {
        let Ok(inner) = self.inner.lock() else { return };
        if let Some(detail) = &loaded.list_dropped {
            let event = AdminEvent::JobListDropped {
                detail: detail.clone(),
            };
            self.append_admin(&inner, "", &event);
        }
        for dropped in &loaded.restored.dropped {
            let event = AdminEvent::JobDropped {
                job_name: dropped.name.clone(),
                detail: dropped.detail.clone(),
            };
            self.append_admin(&inner, "", &event);
        }
    }

    /// A start, resume or home that waited ran: `admin.log` and the terminal (A14,
    /// `specs/0118` 37).
    pub(super) fn log_confirmation(&self, inner: &Inner, confirmation: &Confirmation) {
        let name = match confirmation.what {
            Confirms::Home => "Home".to_owned(),
            Confirms::Frame => "Frame".to_owned(),
            _ => inner
                .machine
                .held_job()
                .map(|j| j.name.clone())
                .unwrap_or_default(),
        };
        let Some(owner) = &confirmation.confirmed_by else {
            // Enter on the terminal: the host confirmed.
            let event = Self::confirmed_event(confirmation, &name);
            self.append_admin(inner, "", &event);
            return;
        };
        let verb = match confirmation.what {
            Confirms::Start => "Started",
            Confirms::Resume => "Resumed",
            Confirms::Home => "Homing",
            Confirms::Frame => "Framing",
        };
        let line = if matches!(confirmation.what, Confirms::Home | Confirms::Frame) {
            format!(
                "{verb}, confirmed by {owner} for {}",
                confirmation.requested_by
            )
        } else {
            format!(
                "{verb} {name}, confirmed by {owner} for {}",
                confirmation.requested_by
            )
        };
        let _ = inner.notices.send(line);
        let event = Self::confirmed_event(confirmation, &name);
        self.append_admin(inner, owner, &event);
    }

    fn confirmed_event(confirmation: &Confirmation, name: &str) -> AdminEvent {
        match confirmation.job_id {
            Some(job_id) => AdminEvent::StartConfirmed {
                requester: confirmation.requested_by.clone(),
                job_name: name.to_owned(),
                job_id: job_id.to_string(),
                resume: confirmation.what == Confirms::Resume,
            },
            None => AdminEvent::HomeConfirmed {
                requester: confirmation.requested_by.clone(),
            },
        }
    }

    /// The lines of a change from `before`; `people` are the held job's before the change.
    pub(super) fn log_outcome(&self, inner: &mut Inner, before: State, people: Option<People>) {
        let after = inner.machine.state();
        if after == before {
            return;
        }
        let status = inner.machine.status();
        let current = inner.machine.people().cloned().or(people);
        let people = log_people(current.as_ref());
        let name = inner.job_name.clone();
        if let Some((result, shown)) = logged_outcome(before, after, &status) {
            let text = shown.unwrap_or_else(|| result.text());
            self.append_job_line(inner, &name, &result, &people);
            let _ = inner.notices.send(format!("Job {name}: {text}"));
        } else if let Some(line) = terminal_alarm(before, after, &status) {
            let _ = inner.notices.send(line);
        }
        let withdrawn = before == State::AwaitingLocalStart
            && after == State::Idle
            && current.is_some()
            && people.started_by.is_empty();
        if withdrawn {
            // A start that waited and was withdrawn or discarded (A14, UX notes V16).
            self.append_job_line(inner, &name, &LogResult::Discarded, &people);
            let _ = inner.notices.send(format!("Job {name}: Discarded"));
        }
        if after != State::AwaitingLocalStart {
            return;
        }
        // A waiting Home or frame carries no job (`specs/0118` M5, 2b).
        let job = match status {
            Message::Status {
                frame_id: Some(_), ..
            } => "Frame",
            Message::Status { job: None, .. } => "Home",
            _ => inner.job_name.as_str(),
        };
        let line = match &status {
            Message::Status {
                requested_by: Some(member),
                frame_id: Some(_),
                ..
            } => format!("Frame requested by {member}"),
            Message::Status {
                requested_by: Some(member),
                job: None,
                ..
            } => format!("Home requested by {member}"),
            Message::Status {
                requested_by: Some(member),
                ..
            } => {
                let what = if inner.machine.waits_for_resume() {
                    "Resume"
                } else {
                    "Start"
                };
                format!("{what} of {job} requested by {member}")
            }
            _ => format!("Press Enter to start {job}"),
        };
        let _ = inner.notices.send(line);
    }
}

impl Inner {
    /// Writes the job list to the state directory when it changed: new texts, then the list,
    /// then deleted texts. The line of the running job is written at most once a second, or at
    /// once when `force_line`.
    pub(super) fn sync_store(&mut self, at: Duration, force_line: bool) {
        let inner = self;
        if inner.frozen {
            return;
        }
        let mut stored = inner.machine.stored();
        let texts = inner.machine.stored_texts();
        let ids: std::collections::HashSet<_> = texts.iter().map(|(id, _)| *id).collect();
        let line = stored.current.as_ref().map_or(0, |c| c.line);
        let write = match &inner.last_stored {
            None => true,
            Some((last, written_at)) => {
                let mut same = last.clone();
                let last_line = same.current.as_ref().map_or(0, |c| c.line);
                if let Some(c) = same.current.as_mut() {
                    c.line = line;
                }
                let due = at.saturating_sub(*written_at) >= LINE_PERIOD;
                same != stored || (line != last_line && (due || force_line))
            }
        };
        if !write {
            return;
        }
        for (id, text) in texts {
            if inner.stored_ids.insert(id) {
                inner.store.write_text(id, text);
            }
        }
        inner.store.write_list(&stored);
        let gone: Vec<_> = inner
            .stored_ids
            .iter()
            .filter(|id| !ids.contains(id))
            .copied()
            .collect();
        for id in gone {
            inner.stored_ids.remove(&id);
            inner.store.remove_text(id);
        }
        if let Some(c) = stored.current.as_mut() {
            c.line = line;
        }
        inner.last_stored = Some((stored, at));
    }
}
