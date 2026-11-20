//! Which changes of the job machine reach `jobs.log`, `admin.log` and the terminal, with the
//! clients behind them (`0064` Q18, `specs/0127` G7, `specs/0128` A14, A15, A18).

use curvyo_connector_core::{Confirmation, Message, People, RejectReason, State};
use time::OffsetDateTime;
use uuid::Uuid;

use super::{Hub, Inner};
use crate::admin_log::AdminEvent;
use crate::hub_log::{logged_outcome, rejection_line, terminal_alarm};
use crate::job_log::{LogPeople, LogResult};

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

    /// The refusal of a submitted job: its log line and, for the settings refusals, its own
    /// terminal line (UX notes section 10).
    pub(super) fn log_rejection(&self, inner: &Inner, job_id: Uuid, reason: RejectReason) {
        let name = inner
            .submitted
            .as_ref()
            .filter(|(id, _)| *id == job_id)
            .map(|(_, name)| name.clone())
            .unwrap_or_default();
        let result = LogResult::Rejected(format!("{reason:?}"));
        self.append_job_line(inner, &name, &result, &LogPeople::default());
        let line =
            rejection_line(reason).unwrap_or_else(|| format!("Job {name}: {}", result.text()));
        let _ = inner.notices.send(line);
    }

    /// A start or resume that waited ran: `admin.log` and the terminal (A14).
    pub(super) fn log_confirmation(&self, inner: &Inner, confirmation: &Confirmation) {
        let name = inner
            .machine
            .held_job()
            .map(|j| j.name.clone())
            .unwrap_or_default();
        let Some(owner) = &confirmation.confirmed_by else {
            // Enter on the terminal: the host confirmed.
            let event = Self::confirmed_event(confirmation, &name);
            self.append_admin(inner, "", &event);
            return;
        };
        let verb = if confirmation.resume {
            "Resumed"
        } else {
            "Started"
        };
        let _ = inner.notices.send(format!(
            "{verb} {name}, confirmed by {owner} for {}",
            confirmation.requested_by
        ));
        let event = Self::confirmed_event(confirmation, &name);
        self.append_admin(inner, owner, &event);
    }

    fn confirmed_event(confirmation: &Confirmation, name: &str) -> AdminEvent {
        AdminEvent::StartConfirmed {
            requester: confirmation.requested_by.clone(),
            job_name: name.to_owned(),
            job_id: confirmation.job_id.to_string(),
            resume: confirmation.resume,
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
        // A waiting Home carries no job (`specs/0118` M5).
        let job = match status {
            Message::Status { job: None, .. } => "Home",
            _ => inner.job_name.as_str(),
        };
        let line = match &status {
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
