//! The presence channel's timing (ADR 0031 decision 7): a changed state goes out at most every
//! 50 ms, an unchanged one every 2 s as a heartbeat, and a peer silent for 5 s disappears.

use std::collections::BTreeMap;

use crate::frame::ConnectionId;
use crate::time::Millis;

const THROTTLE_MS: u64 = 50;
const HEARTBEAT_MS: u64 = 2_000;
const EXPIRY_MS: u64 = 5_000;

/// My presence to send and when the others were last heard.
#[derive(Default)]
pub(super) struct Presence {
    mine: Option<Vec<u8>>,
    dirty: bool,
    last_sent: Option<Millis>,
    heard: BTreeMap<ConnectionId, Millis>,
}

impl Presence {
    /// A new state of mine.
    pub(super) fn set(&mut self, payload: Vec<u8>) {
        self.mine = Some(payload);
        self.dirty = true;
    }

    /// My state, if it is due at `now`; marks it sent.
    pub(super) fn due(&mut self, now: Millis) -> Option<Vec<u8>> {
        let mine = self.mine.as_ref()?;
        let since = self.last_sent.map_or(u64::MAX, |t| now.since(t));
        let due = (self.dirty && since >= THROTTLE_MS) || since >= HEARTBEAT_MS;
        if !due {
            return None;
        }
        self.dirty = false;
        self.last_sent = Some(now);
        Some(mine.clone())
    }

    /// After a reconnect my state goes out at once.
    pub(super) fn resend(&mut self) {
        self.last_sent = None;
    }

    /// A peer's message arrived.
    pub(super) fn heard(&mut self, connection: ConnectionId, now: Millis) {
        self.heard.insert(connection, now);
    }

    /// A peer left; whether it was shown.
    pub(super) fn forget(&mut self, connection: ConnectionId) -> bool {
        self.heard.remove(&connection).is_some()
    }

    /// Every shown peer, all gone at once (my link was lost).
    pub(super) fn forget_all(&mut self) -> Vec<ConnectionId> {
        let all = self.heard.keys().copied().collect();
        self.heard.clear();
        all
    }

    /// The peers silent for 5 s at `now`, forgotten.
    pub(super) fn expire(&mut self, now: Millis) -> Vec<ConnectionId> {
        let gone: Vec<ConnectionId> = self
            .heard
            .iter()
            .filter(|(_, t)| now.since(**t) > EXPIRY_MS)
            .map(|(c, _)| *c)
            .collect();
        for connection in &gone {
            self.heard.remove(connection);
        }
        gone
    }
}
