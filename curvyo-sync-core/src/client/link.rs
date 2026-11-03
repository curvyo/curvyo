//! The connection's liveness and reconnect timing (ADR 0031 decision 9, adrs.md decision 8):
//! ping every 2 s, offline after 4 s without an answer, reconnect after 0.5 s doubling to 10 s
//! with ±20 % jitter from the caller's entropy.

use crate::entropy::Entropy;
use crate::time::Millis;

/// The ping interval.
pub(super) const PING_MS: u64 = 2_000;
/// Silence after which the connection counts as lost.
pub(super) const OFFLINE_MS: u64 = 4_000;
const FIRST_RETRY_MS: u64 = 500;
const MAX_RETRY_MS: u64 = 10_000;

/// When to ping, when the link is lost, and when to try again.
pub(super) struct Link {
    last_heard: Millis,
    last_ping: Millis,
    retry_ms: u64,
}

impl Link {
    pub(super) fn new() -> Self {
        Self {
            last_heard: Millis(0),
            last_ping: Millis(0),
            retry_ms: FIRST_RETRY_MS,
        }
    }

    /// A connection attempt starts, or a frame arrived.
    pub(super) fn heard(&mut self, now: Millis) {
        self.last_heard = now;
    }

    /// Whether the link has been silent too long.
    pub(super) fn is_lost(&self, now: Millis) -> bool {
        now.since(self.last_heard) > OFFLINE_MS
    }

    /// Whether a ping is due; marks it sent.
    pub(super) fn ping_due(&mut self, now: Millis) -> bool {
        if now.since(self.last_ping) >= PING_MS {
            self.last_ping = now;
            return true;
        }
        false
    }

    /// Marks a ping sent now.
    pub(super) fn pinged(&mut self, now: Millis) {
        self.last_ping = now;
    }

    /// An admission succeeded: the next loss retries from the first delay.
    pub(super) fn reset_backoff(&mut self) {
        self.retry_ms = FIRST_RETRY_MS;
    }

    /// The time of the next attempt after a loss at `now`, and the delay doubled for the one
    /// after.
    pub(super) fn next_attempt(&mut self, now: Millis, entropy: &mut Entropy) -> Millis {
        let base = self.retry_ms;
        self.retry_ms = (self.retry_ms * 2).min(MAX_RETRY_MS);
        let low = base - base / 5;
        now.plus(low + entropy.below_or_equal(base * 2 / 5))
    }
}
