//! The return check of a removed or left device (ADR 0041 decision 6): the answer from its own
//! replay of the fetched keyring, and the one leave entry of "Keep as it is" (Frage 162 A).

use curvyo_crypto_core::keyring::Payload;

use super::{CheckOutcome, ClientSession, SessionEvent, Start, State};
use crate::frame::{ClientFrame, RefusalReason};

impl ClientSession {
    /// The relay's pong after the keyring: decides from the own replay. The answer counts only
    /// when the fetched keyring lists this device, wraps to it a key it held before the check,
    /// and admits it by an entry that builds on the heads the ended record kept: a relay cannot
    /// sign an add of an admin, wrap a key it never saw, nor pass off the keyring from before
    /// the removal (ADR 0041 decision 6).
    pub(super) fn check_caught_up(&mut self) {
        let Start::Check {
            then_leave, left, ..
        } = self.start
        else {
            return;
        };
        if left || !matches!(self.state, State::Joined { .. }) {
            return;
        }
        let me = self.keys.me();
        let listed = self.keys.keyring().is_some_and(|k| k.role(&me).is_some());
        if !listed {
            self.finish_check(CheckOutcome::NotYet);
            return;
        }
        let after = match &self.start {
            Start::Check { heads, .. } => self
                .keys
                .keyring()
                .is_some_and(|k| k.admitted_after(&me, heads)),
            _ => false,
        };
        if !self.keys.anchored() || !after {
            self.finish_check(CheckOutcome::Unverified);
            return;
        }
        let may_leave = self
            .keys
            .keyring()
            .is_some_and(|k| crate::admin_remains(k, &[me]));
        if !then_leave || !may_leave {
            self.finish_check(CheckOutcome::AddedBack);
            return;
        }
        let Ok(entry) = self.keys.write(Payload::RemoveMember { member: me }) else {
            self.finish_check(CheckOutcome::Failed);
            return;
        };
        let Ok(bytes) = entry.encode() else {
            self.finish_check(CheckOutcome::Failed);
            return;
        };
        self.send(&ClientFrame::Keyring { entry: bytes });
        // The relay accepts the leave and refuses this connection as removed.
        if let Start::Check { left, .. } = &mut self.start {
            *left = true;
        }
    }

    /// The relay refused the check's connection.
    pub(super) fn check_refused(&mut self, reason: RefusalReason) {
        let left = matches!(self.start, Start::Check { left: true, .. });
        let outcome = match reason {
            RefusalReason::Removed if left => CheckOutcome::LeftAgain,
            RefusalReason::Removed => CheckOutcome::NotYet,
            // The relay keeps a removal in the room's keyring: "not a member" means it holds no
            // such room, or one that never listed this device.
            RefusalReason::NotAMember => CheckOutcome::NotFound,
            _ => CheckOutcome::Failed,
        };
        self.finish_check(outcome);
    }

    /// Reports the answer and ends the session.
    pub(super) fn finish_check(&mut self, outcome: CheckOutcome) {
        self.event(SessionEvent::CheckResult(outcome));
        self.end();
    }
}
