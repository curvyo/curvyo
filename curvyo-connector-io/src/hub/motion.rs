//! How the hub takes the motion commands Home, Jog and Go to point: the dead-man clock before each
//! jog message, the answers, and the log lines (`specs/0118` criteria 12, 18, 35 c; adrs M8, J4,
//! J7, P9).

use std::time::Instant;

use curvyo_connector_core::{Event, Message, Peer, State};

use super::Hub;
use crate::job_log::{LogPeople, LogResult};

impl Hub {
    /// `Home` from the client `peer` (`specs/0118` M8): one `jobs.log` line and one terminal
    /// line when the connector takes it; the answers for that client alone, the status when there
    /// is no other answer (in `Homing` a second Home changes nothing).
    pub fn home(&self, peer: &Peer, now: Instant) -> Vec<Message> {
        let mut inner = self.lock();
        self.turn(&mut inner, &[], now);
        let before = inner.machine.state();
        let mut direct = Vec::new();
        let event = Event::Message(Message::Home, peer.clone());
        self.apply(&mut inner, event, now, &mut direct);
        let after = inner.machine.state();
        // An owner's press that confirms a waiting Home is no second request (`specs/0118` 37).
        let requested = before != State::AwaitingLocalStart;
        if after != before
            && requested
            && matches!(after, State::Homing | State::AwaitingLocalStart)
        {
            let client = peer.name.as_str();
            let _ = inner.notices.send(format!("Home sent by {client}"));
            let result = LogResult::Home(client.to_owned());
            self.append_job_line(&inner, "", &result, &LogPeople::default());
        }
        if direct.is_empty() {
            direct.push(inner.machine.status());
        }
        direct
    }

    /// `Jog`, `JogKeep` or `JogStop` from the client `peer`. The machine's clock is set to now
    /// first, so a hold's 300 ms count from this message. `Jog` answers with the status when
    /// nothing else (a step that was dropped); keep-alives and releases answer only a refusal.
    pub fn jog(&self, message: Message, peer: &Peer, now: Instant) -> Vec<Message> {
        let mut inner = self.lock();
        self.turn(&mut inner, &[], now);
        let at = self.at(now);
        self.apply(&mut inner, Event::Clock(at), now, &mut Vec::new());
        let before = inner.machine.state();
        let answers_status = matches!(message, Message::Jog { .. });
        let mut direct = Vec::new();
        let event = Event::Message(message, peer.clone());
        self.apply(&mut inner, event, now, &mut direct);
        if before != State::Jogging && inner.machine.state() == State::Jogging {
            self.log_jog(&inner, &peer.name);
        }
        if direct.is_empty() && answers_status {
            direct.push(inner.machine.status());
        }
        direct
    }

    /// `GoToPoint` from the client `peer` (`specs/0118` 2d, adrs P9): one `jobs.log` line
    /// `Focus point: <client>` and one terminal line when the connector takes it (the pulse writes
    /// its own line when its block goes out); the answers for that client alone, the status when
    /// there is no other answer.
    pub fn go_to_point(&self, message: Message, peer: &Peer, now: Instant) -> Vec<Message> {
        let mut inner = self.lock();
        self.turn(&mut inner, &[], now);
        let at = self.at(now);
        self.apply(&mut inner, Event::Clock(at), now, &mut Vec::new());
        let before = inner.machine.state();
        let mut direct = Vec::new();
        let event = Event::Message(message, peer.clone());
        self.apply(&mut inner, event, now, &mut direct);
        let after = inner.machine.state();
        if after != before && matches!(after, State::Focusing | State::AwaitingLocalStart) {
            let client = peer.name.as_str();
            let _ = inner.notices.send(format!("Focus point sent by {client}"));
            let result = LogResult::FocusPoint(client.to_owned());
            self.append_job_line(&inner, "", &result, &LogPeople::default());
        }
        if direct.is_empty() {
            direct.push(inner.machine.status());
        }
        direct
    }

    /// The go to point's pulse block went to the controller: one `jobs.log` line `Focus pulse:
    /// <client>` (`specs/0118` 35 c), so the move and the laser emission are logged apart.
    pub(super) fn log_pulse(&self, inner: &super::Inner) {
        let client = inner.machine.focus_client().unwrap_or_default().to_owned();
        let result = LogResult::FocusPulse(client);
        self.append_job_line(inner, "", &result, &LogPeople::default());
    }

    /// One `jobs.log` line and one terminal line per run of one client's jogs; any other
    /// `jobs.log` line or another client ends the run.
    fn log_jog(&self, inner: &super::Inner, client: &str) {
        let mut run = self
            .jog_run
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if run.as_deref() == Some(client) {
            return;
        }
        let _ = inner.notices.send(format!("Jog sent by {client}"));
        let result = LogResult::Jog(client.to_owned());
        drop(run);
        self.append_job_line(inner, "", &result, &LogPeople::default());
        run = self
            .jog_run
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *run = Some(client.to_owned());
    }
}
