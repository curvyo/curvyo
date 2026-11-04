//! One pairing attempt as the page drives it (`specs/0096-own-devices-sync` criteria 1 to 5, ADR
//! 0040 decision 4): the opener's or the joiner's machine of `curvyo-sync-core` behind the same
//! transport port as a room session, and the view the pairing block shows.
//!
//! The page opens the pairing WebSocket when asked, reports it open, feeds what arrives and
//! carries the sends and the close out; the view is plain data with the words formatted in
//! `curvyo-ui-core`.

use std::collections::VecDeque;

use curvyo_crypto_core::{DocumentId, Identity, IdentityKeys};
use curvyo_sync_core::{
    Millis, PAIR_WRONG_TRIES, PairEvent, PairFailure, PairJoiner, PairOpener, PairOutput,
    PairingError, RefusalReason,
};
use curvyo_ui_core::pairing_text::{self as texts, PairProblem};

use crate::session::TransportAction;

/// Which side this device is.
enum Machine {
    Opener(Box<PairOpener>),
    Joiner(Box<PairJoiner>),
}

/// Where the pairing block stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairPhase {
    /// Reaching the server.
    Connecting,
    /// The opener shows the code until `expires_at` (ms).
    Code {
        /// The code as shown.
        code: String,
        /// When the relay lets it expire, by the host's clock.
        expires_at: u64,
    },
    /// The joiner sent its share and waits for the opener's.
    Joining,
    /// Both checked the code: the other device and the four check words.
    Compare {
        /// The other device.
        peer: Identity,
        /// Its name.
        label: String,
        /// The check words.
        words: [&'static str; 4],
    },
    /// The opener pressed Pair and waits for the group room.
    Pairing {
        /// The other device.
        peer: Identity,
        /// Its name.
        label: String,
    },
    /// Done.
    Paired {
        /// The other device.
        peer: Identity,
        /// Its name.
        label: String,
        /// The group room.
        group_room: DocumentId,
    },
    /// Ended without a pairing.
    Failed(PairProblem),
}

/// One pairing attempt.
pub struct PairingGlue {
    machine: Machine,
    server: String,
    phase: PairPhase,
    /// Wrong tries the opener saw, or the joiner's tries left after a wrong code.
    wrong: u8,
    /// The other device's name, once known (for the joiner's messages).
    other: String,
    /// The opener's code and its expiry, to show again when a joiner leaves.
    shown_code: Option<(String, u64)>,
    actions: VecDeque<TransportAction>,
    now: u64,
    revision: u32,
}

impl PairingGlue {
    /// The opener's side (Add a device) on `server` with this device's identity entropy and
    /// name; `entropy` is 32 fresh bytes.
    ///
    /// # Errors
    /// `ServerAddress`, `Label`.
    pub fn opener(
        server: &str,
        identity: &[u8; 64],
        label: &str,
        entropy: [u8; 32],
        now: Millis,
    ) -> Result<Self, PairingError> {
        let machine =
            PairOpener::new(server, IdentityKeys::from_entropy(identity), label, entropy)?;
        Ok(Self::new(Machine::Opener(Box::new(machine)), server, now))
    }

    /// The joiner's side (Join my devices) for the typed `code`.
    ///
    /// # Errors
    /// `ServerAddress`, `Label`, `Crypto` for a text that is not a code.
    pub fn joiner(
        server: &str,
        code: &str,
        identity: &[u8; 64],
        label: &str,
        entropy: [u8; 32],
        now: Millis,
    ) -> Result<Self, PairingError> {
        let machine = PairJoiner::new(
            server,
            code,
            IdentityKeys::from_entropy(identity),
            label,
            entropy,
        )?;
        Ok(Self::new(Machine::Joiner(Box::new(machine)), server, now))
    }

    fn new(machine: Machine, server: &str, now: Millis) -> Self {
        let mut actions = VecDeque::new();
        actions.push_back(TransportAction::Connect(server.to_owned()));
        Self {
            machine,
            server: server.to_owned(),
            phase: PairPhase::Connecting,
            wrong: 0,
            other: String::new(),
            shown_code: None,
            actions,
            now: now.0,
            revision: 1,
        }
    }

    /// Whether this is the opener's side.
    #[must_use]
    pub fn is_opener(&self) -> bool {
        matches!(self.machine, Machine::Opener(_))
    }

    /// The phase.
    #[must_use]
    pub fn phase(&self) -> &PairPhase {
        &self.phase
    }

    /// Bumped whenever the view changes.
    #[must_use]
    pub fn revision(&self) -> u32 {
        self.revision
    }

    /// The next transport action.
    pub fn next_action(&mut self) -> Option<TransportAction> {
        self.actions.pop_front()
    }

    /// The pairing socket opened: the machine starts.
    pub fn connected(&mut self, now: Millis) {
        self.now = now.0;
        let out = match &mut self.machine {
            Machine::Opener(m) => m.start(),
            Machine::Joiner(m) => m.start(),
        };
        if let Ok(out) = out {
            if !self.is_opener() {
                self.set_phase(PairPhase::Joining);
            }
            self.carry_out(out);
        }
    }

    /// The socket closed or could not open.
    pub fn disconnected(&mut self, now: Millis) {
        self.now = now.0;
        if self.phase == PairPhase::Connecting {
            self.actions.clear();
            self.set_phase(PairPhase::Failed(PairProblem::Unreachable));
            return;
        }
        let out = match &mut self.machine {
            Machine::Opener(m) => m.closed(),
            Machine::Joiner(m) => m.closed(),
        };
        self.carry_out(out);
    }

    /// One message arrived.
    pub fn received(&mut self, bytes: &[u8], now: Millis) {
        self.now = now.0;
        let out = match &mut self.machine {
            Machine::Opener(m) => m.receive(bytes),
            Machine::Joiner(m) => m.receive(bytes),
        };
        self.carry_out(out);
    }

    /// Time passed: an opener's code past its lifetime shows as expired.
    pub fn tick(&mut self, now: Millis) {
        self.now = now.0;
        if let PairPhase::Code { expires_at, .. } = self.phase
            && now.0 >= expires_at
        {
            let out = self.cancel_machine();
            self.carry_out(out);
            self.set_phase(PairPhase::Failed(PairProblem::Expired));
        }
    }

    /// The opener's Pair, after `AddMember` for the joiner went to `group_room`.
    ///
    /// # Errors
    /// `NotNow` on the joiner's side or before the comparison.
    pub fn pair(&mut self, group_room: DocumentId) -> Result<(), PairingError> {
        let Machine::Opener(machine) = &mut self.machine else {
            return Err(PairingError::NotNow);
        };
        let out = machine.pair(group_room)?;
        self.carry_out(out);
        Ok(())
    }

    /// The opener pressed Pair and its device group is writing `AddMember` first.
    pub fn mark_pairing(&mut self) {
        if let PairPhase::Compare { peer, label, .. } = &self.phase {
            let phase = PairPhase::Pairing {
                peer: *peer,
                label: label.clone(),
            };
            self.set_phase(phase);
        }
    }

    /// Cancel, Escape or Words differ: the attempt ends and the code dies.
    pub fn cancel(&mut self, words_differ: bool) {
        let out = self.cancel_machine();
        self.carry_out(out);
        let problem = if words_differ {
            PairProblem::WordsDiffer
        } else {
            PairProblem::Stopped
        };
        self.set_phase(PairPhase::Failed(problem));
    }

    fn cancel_machine(&mut self) -> Vec<PairOutput> {
        match &mut self.machine {
            Machine::Opener(m) => m.cancel(),
            Machine::Joiner(m) => m.cancel(),
        }
    }

    fn set_phase(&mut self, phase: PairPhase) {
        self.phase = phase;
        self.revision = self.revision.wrapping_add(1);
    }

    fn carry_out(&mut self, out: Vec<PairOutput>) {
        for output in out {
            match output {
                PairOutput::Send(bytes) => self.actions.push_back(TransportAction::Send(bytes)),
                PairOutput::Close => self.actions.push_back(TransportAction::Close),
                PairOutput::Event(event) => self.on_event(event),
            }
        }
    }

    fn on_event(&mut self, event: PairEvent) {
        match event {
            PairEvent::Code { code, expires_in_s } => {
                let expires_at = self.now + u64::from(expires_in_s) * 1000;
                self.shown_code = Some((code.clone(), expires_at));
                self.set_phase(PairPhase::Code { code, expires_at });
            }
            PairEvent::Compare { peer, label, words } => {
                label.clone_into(&mut self.other);
                self.set_phase(PairPhase::Compare { peer, label, words });
            }
            PairEvent::WrongCode { tries_left } => {
                self.wrong = PAIR_WRONG_TRIES - tries_left;
                self.revision = self.revision.wrapping_add(1);
            }
            PairEvent::JoinerLeft => {
                // The joiner went away: the code is shown again for the next one.
                if let Some((code, expires_at)) = self.shown_code.clone() {
                    self.set_phase(PairPhase::Code { code, expires_at });
                }
            }
            PairEvent::Paired {
                peer,
                label,
                group_room,
            } => self.set_phase(PairPhase::Paired {
                peer,
                label,
                group_room,
            }),
            PairEvent::Failed(failure) => {
                if !matches!(self.phase, PairPhase::Failed(_)) {
                    let problem = self.problem(failure);
                    self.set_phase(PairPhase::Failed(problem));
                }
            }
        }
    }

    fn problem(&mut self, failure: PairFailure) -> PairProblem {
        match failure {
            PairFailure::Refused(reason) => match reason {
                RefusalReason::Expired => PairProblem::Expired,
                RefusalReason::Used => PairProblem::Used,
                RefusalReason::Locked => PairProblem::Locked,
                RefusalReason::Busy => PairProblem::Busy,
                RefusalReason::Unknown => PairProblem::Unknown,
                _ => PairProblem::Lost,
            },
            PairFailure::WrongCode | PairFailure::Unauthentic => {
                self.wrong = self.wrong.saturating_add(1);
                PairProblem::WrongCode
            }
            PairFailure::Locked => PairProblem::Locked,
            PairFailure::Cancelled => PairProblem::Stopped,
            PairFailure::Gone | PairFailure::Protocol => PairProblem::Lost,
        }
    }

    /// The view as JSON: `phase` and its fields, the texts formatted.
    #[must_use]
    pub fn view_json(&self) -> String {
        let opener = self.is_opener();
        let view = match &self.phase {
            PairPhase::Connecting => serde_json::json!({"phase": "connecting"}),
            PairPhase::Code { code, expires_at } => {
                let left = expires_at.saturating_sub(self.now) / 1000;
                serde_json::json!({
                    "phase": "code",
                    "code": code,
                    "spelled": texts::code_spelled(code),
                    "valid": texts::valid_for_text(left),
                    "announce": texts::code_left_announcement(left),
                    "seconds_left": left,
                    "wrong": if self.wrong > 0 { texts::wrong_tries_seen_text(self.wrong) } else { String::new() },
                })
            }
            PairPhase::Joining => serde_json::json!({"phase": "joining"}),
            PairPhase::Compare { peer, label, words } => serde_json::json!({
                "phase": "compare",
                "label": label,
                "fingerprint": peer.fingerprint(),
                "words": words,
                "heading": if opener { texts::wants_to_join_text(label) } else { texts::waiting_to_confirm_text(label) },
                "must_match": texts::words_must_match_text(label),
            }),
            PairPhase::Pairing { label, .. } => serde_json::json!({
                "phase": "pairing",
                "text": texts::pairing_text(label),
            }),
            PairPhase::Paired { peer, label, .. } => serde_json::json!({
                "phase": "paired",
                "label": label,
                "fingerprint": peer.fingerprint(),
            }),
            PairPhase::Failed(problem) => {
                let text = if opener {
                    texts::opener_problem_text(*problem, &self.other)
                } else {
                    let tries_left = PAIR_WRONG_TRIES.saturating_sub(self.wrong);
                    texts::joiner_problem_text(*problem, tries_left, &self.other, &self.server)
                };
                serde_json::json!({
                    "phase": "failed",
                    "problem": problem_code(*problem),
                    "text": text,
                })
            }
        };
        view.to_string()
    }
}

fn problem_code(problem: PairProblem) -> &'static str {
    match problem {
        PairProblem::WrongCode => "wrong",
        PairProblem::Locked => "locked",
        PairProblem::Expired => "expired",
        PairProblem::Used => "used",
        PairProblem::Unknown => "unknown",
        PairProblem::Busy => "busy",
        PairProblem::Stopped => "stopped",
        PairProblem::Lost => "lost",
        PairProblem::Unreachable => "unreachable",
        PairProblem::WordsDiffer => "words-differ",
    }
}

#[cfg(test)]
mod tests {
    use curvyo_sync_core::{ConnectionId, PairSlots, RelayOutput};

    use super::*;

    const SERVER: &str = "ws://192.168.1.20:8787";

    fn identity(n: u8) -> [u8; 64] {
        [n; 64]
    }

    /// Delivers every action of both sides through `slots` until nothing moves.
    fn settle(slots: &mut PairSlots, sides: &mut [(&mut PairingGlue, ConnectionId)], now: Millis) {
        loop {
            let mut moved = false;
            for index in 0..sides.len() {
                while let Some(action) = sides[index].0.next_action() {
                    moved = true;
                    let from = sides[index].1;
                    let out = match action {
                        TransportAction::Connect(_) => {
                            sides[index].0.connected(now);
                            continue;
                        }
                        TransportAction::Send(bytes) => slots.receive(from, &bytes, now, [7; 32]),
                        TransportAction::Close => slots.disconnect(from),
                    };
                    for output in out {
                        if let RelayOutput::Send { connection, frame } = output
                            && let Some(side) = sides.iter_mut().find(|(_, c)| *c == connection)
                        {
                            side.0.received(&frame.encode(), now);
                        }
                    }
                }
            }
            if !moved {
                return;
            }
        }
    }

    fn json(glue: &PairingGlue) -> serde_json::Value {
        serde_json::from_str(&glue.view_json()).unwrap()
    }

    #[test]
    fn two_devices_compare_and_pair_through_the_glue() {
        let now = Millis(1_000);
        let mut slots = PairSlots::new();
        let mut a = PairingGlue::opener(SERVER, &identity(1), "Desktop", [1; 32], now).unwrap();
        settle(&mut slots, &mut [(&mut a, ConnectionId(1))], now);
        let view = json(&a);
        assert_eq!(view["phase"], "code");
        assert_eq!(view["valid"], "Valid for 5:00");
        let code = view["code"].as_str().unwrap().to_owned();

        let mut b =
            PairingGlue::joiner(SERVER, &code, &identity(2), "Laptop", [2; 32], now).unwrap();
        settle(
            &mut slots,
            &mut [(&mut a, ConnectionId(1)), (&mut b, ConnectionId(2))],
            now,
        );
        let (va, vb) = (json(&a), json(&b));
        assert_eq!(va["phase"], "compare");
        assert_eq!(vb["phase"], "compare");
        assert_eq!(va["words"], vb["words"]);
        assert_eq!(va["heading"], "Laptop wants to join");
        assert_eq!(vb["heading"], "Waiting for Desktop to confirm");

        a.mark_pairing();
        assert_eq!(json(&a)["text"], "Pairing Laptop...");
        a.pair(DocumentId([0x6e; 16])).unwrap();
        settle(
            &mut slots,
            &mut [(&mut a, ConnectionId(1)), (&mut b, ConnectionId(2))],
            now,
        );
        assert!(matches!(
            b.phase(),
            PairPhase::Paired { group_room, .. } if *group_room == DocumentId([0x6e; 16])
        ));
        assert_eq!(json(&a)["phase"], "paired");
    }

    #[test]
    fn a_wrong_code_counts_tries_and_an_unreachable_server_is_named() {
        let now = Millis(1_000);
        let mut slots = PairSlots::new();
        let mut a = PairingGlue::opener(SERVER, &identity(1), "Desktop", [1; 32], now).unwrap();
        settle(&mut slots, &mut [(&mut a, ConnectionId(1))], now);
        let code = json(&a)["code"].as_str().unwrap().to_owned();
        let mut wrong = code.clone();
        let last = wrong.pop().unwrap();
        wrong.push(if last == '7' { '8' } else { '7' });
        let mut b =
            PairingGlue::joiner(SERVER, &wrong, &identity(2), "Laptop", [2; 32], now).unwrap();
        settle(
            &mut slots,
            &mut [(&mut a, ConnectionId(1)), (&mut b, ConnectionId(2))],
            now,
        );
        assert_eq!(json(&b)["text"], "That code does not match. 4 tries left.");
        assert_eq!(json(&a)["wrong"], "Someone entered a wrong code 1 time.");

        let mut c = PairingGlue::joiner(SERVER, &code, &identity(3), "Pad", [3; 32], now).unwrap();
        assert!(matches!(c.next_action(), Some(TransportAction::Connect(_))));
        c.disconnected(now);
        assert_eq!(
            json(&c)["text"],
            "Cannot reach the server at 192.168.1.20:8787."
        );
    }

    #[test]
    fn the_code_expires_by_the_clock() {
        let now = Millis(1_000);
        let mut slots = PairSlots::new();
        let mut a = PairingGlue::opener(SERVER, &identity(1), "Desktop", [1; 32], now).unwrap();
        settle(&mut slots, &mut [(&mut a, ConnectionId(1))], now);
        a.tick(Millis(1_000 + 5 * 60 * 1000));
        assert_eq!(json(&a)["text"], "This code has expired.");
    }
}
