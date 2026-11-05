//! The connection's liveness and reconnect timing (ADR 0031 decision 9, adrs.md decision 8):
//! ping every 2 s, offline after 4 s without an answer, reconnect after 0.5 s doubling to 10 s
//! with ±20 % jitter from the caller's entropy. A stall of the host between two ticks does not
//! count as silence.

use crate::entropy::Entropy;
use crate::time::Millis;

/// The ping interval.
pub(super) const PING_MS: u64 = 2_000;
/// Silence after which the connection counts as lost.
pub(super) const OFFLINE_MS: u64 = 4_000;
/// A gap between two ticks longer than this is a stall of the host (its timer runs every 50 to
/// 1000 ms), not silence of the link.
const STALL_MS: u64 = 2_000;
const FIRST_RETRY_MS: u64 = 500;
const MAX_RETRY_MS: u64 = 10_000;

/// When to ping, when the link is lost, and when to try again.
pub(super) struct Link {
    last_heard: Millis,
    last_ping: Millis,
    retry_ms: u64,
    /// The host's last tick, to tell a stalled host from a silent link.
    last_tick: Option<Millis>,
    /// The previous tick was a stall's grace: the next one judges again.
    graced: bool,
}

impl Link {
    pub(super) fn new() -> Self {
        Self {
            last_heard: Millis(0),
            last_ping: Millis(0),
            retry_ms: FIRST_RETRY_MS,
            last_tick: None,
            graced: false,
        }
    }

    /// The host ticks at `now`. A gap since its last tick longer than [`STALL_MS`] was a stall
    /// of the host (a long synchronous job on its thread), not silence of the link: it is not
    /// counted, so the socket's open, or the answer to the ping this tick sends, still has the
    /// full offline time. Only one tick in a row gets this credit, so a host that ticks rarely
    /// (a hidden tab) still notices a lost link.
    pub(super) fn ticked(&mut self, now: Millis) {
        let gap = self.last_tick.map_or(0, |t| now.since(t));
        self.last_tick = Some(now);
        let credit = gap > STALL_MS && !self.graced;
        self.graced = credit;
        if credit {
            self.last_heard = Millis(self.last_heard.0.saturating_add(gap).min(now.0));
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

#[cfg(test)]
mod tests {
    use super::*;

    /// PR #281 review: a 4.4 s stall of the host (the first save's thumbnail renderer) is not
    /// taken for a silent link: the socket's open queued behind the tick still counts.
    #[test]
    fn a_stall_of_the_host_is_not_silence() {
        let mut link = Link::new();
        link.heard(Millis(1_000));
        link.ticked(Millis(1_050));
        link.ticked(Millis(5_450));
        assert!(!link.is_lost(Millis(5_450)), "the stall is not counted");
        link.ticked(Millis(5_500));
        assert!(!link.is_lost(Millis(5_500)));
        for t in (5_550..=9_500).step_by(50) {
            link.ticked(Millis(t));
        }
        assert!(
            link.is_lost(Millis(9_500)),
            "silence after the stall still counts"
        );
    }

    /// A host that ticks rarely (a hidden tab) gets the credit at most every other tick, so a
    /// silent link is still noticed.
    #[test]
    fn rare_ticks_still_notice_a_silent_link() {
        let mut link = Link::new();
        link.heard(Millis(0));
        link.ticked(Millis(100));
        link.ticked(Millis(60_100));
        assert!(!link.is_lost(Millis(60_100)));
        link.ticked(Millis(120_100));
        assert!(link.is_lost(Millis(120_100)));
        link.heard(Millis(120_200));
        link.ticked(Millis(121_200));
        link.ticked(Millis(122_200));
        assert!(!link.is_lost(Millis(122_200)), "1 s ticks are no stall");
    }
}
