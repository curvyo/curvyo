//! The stop sequence of `specs/0127` G12: feed hold, wait for `Hold:0`, soft reset, wait for the
//! banner; the caller then writes `M5` and `M9`. Pure: time comes in as `at`.

use std::time::Duration;

use crate::grbl::identify::RESET;
use crate::grbl::reply::Report;

/// GRBL's feed hold, a real-time byte.
pub const HOLD: u8 = b'!';

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    /// `!` sent; waiting for `Hold:0` until the time.
    Holding(Duration),
    /// The reset sent; waiting for the banner until the time.
    Resetting(Duration),
}

/// What a tick of the sequence means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tick {
    /// Still going.
    Waiting,
    /// No banner within the limit: the stop is not confirmed.
    Unconfirmed,
}

/// One stop in progress, and why it was started: `failure` is the sentence of a job that fails
/// through it (G7, G28), `None` for the maker's Stop.
#[derive(Debug, Clone, PartialEq)]
pub struct Stop {
    step: Step,
    hold: Duration,
    banner: Duration,
    /// The reset went out after `Hold:0` (motion complete): GRBL keeps its position. A reset
    /// while moving loses steps (GRBL raises `ALARM:3`), so the position is not trusted.
    kept: bool,
    /// The sentence the job ends with instead of `Stopped`.
    pub failure: Option<String>,
}

impl Stop {
    /// Starts at `at`. A completed hold or homing (GRBL ignores `!` while homing) goes straight
    /// to the reset; `hold_sent` means a `!` is already on its way.
    pub fn start(
        held: bool,
        homing: bool,
        hold_sent: bool,
        failure: Option<String>,
        (hold_limit, banner_limit): (Duration, Duration),
        at: Duration,
        out: &mut Vec<u8>,
    ) -> Self {
        let step = if held || homing {
            out.push(RESET);
            Step::Resetting(at + banner_limit)
        } else {
            if !hold_sent {
                out.push(HOLD);
            }
            Step::Holding(at + hold_limit)
        };
        Self {
            step,
            hold: hold_limit,
            banner: banner_limit,
            kept: held && !homing,
            failure,
        }
    }

    /// A status report: `Hold:0` while holding sends the reset.
    pub fn on_report(&mut self, report: &Report, at: Duration, out: &mut Vec<u8>) {
        if matches!(self.step, Step::Holding(_)) && report.hold_complete() {
            out.push(RESET);
            self.step = Step::Resetting(at + self.banner);
            self.kept = true;
        }
    }

    /// Whether the reset followed `Hold:0`, so the machine position is still GRBL's count. A
    /// reset after the hold limit, during homing, or any reset before `Hold:0` (the machine's
    /// button) is not.
    #[must_use]
    pub fn kept_position(&self) -> bool {
        self.kept
    }

    /// Whether a banner now confirms the stop.
    #[must_use]
    pub fn confirms(&self) -> bool {
        matches!(self.step, Step::Resetting(_))
    }

    /// Lets time pass: a hold that did not complete is reset anyway; a missing banner means the
    /// stop is not confirmed.
    pub fn on_time(&mut self, at: Duration, out: &mut Vec<u8>) -> Tick {
        match self.step {
            Step::Holding(until) if at >= until => {
                out.push(RESET);
                self.step = Step::Resetting(at + self.banner);
                Tick::Waiting
            }
            Step::Resetting(until) if at >= until => Tick::Unconfirmed,
            _ => Tick::Waiting,
        }
    }

    /// The hold limit this stop runs with.
    #[must_use]
    pub fn hold_limit(&self) -> Duration {
        self.hold
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grbl::reply::{Reply, parse};

    const LIMITS: (Duration, Duration) = (Duration::from_secs(2), Duration::from_secs(3));

    fn report(line: &str) -> Report {
        match parse(line) {
            Reply::Report(r) => r,
            other => panic!("{other:?}"),
        }
    }

    fn s(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    #[test]
    fn running_holds_then_resets_at_hold_zero() {
        let mut out = Vec::new();
        let mut stop = Stop::start(false, false, false, None, LIMITS, s(0), &mut out);
        assert_eq!(out, [HOLD]);
        assert!(!stop.confirms());
        stop.on_report(&report("<Hold:1|MPos:0,0,0>"), s(1), &mut out);
        assert_eq!(out, [HOLD]);
        stop.on_report(&report("<Hold:0|MPos:0,0,0>"), s(1), &mut out);
        assert_eq!(out, [HOLD, RESET]);
        assert!(stop.confirms());
        assert!(stop.kept_position(), "the reset followed Hold:0");
        assert_eq!(stop.on_time(s(3), &mut out), Tick::Waiting);
        assert_eq!(stop.on_time(s(4), &mut out), Tick::Unconfirmed);
    }

    #[test]
    fn paused_or_homing_starts_at_the_reset() {
        for (held, homing) in [(true, false), (false, true)] {
            let mut out = Vec::new();
            let stop = Stop::start(held, homing, false, None, LIMITS, s(0), &mut out);
            assert_eq!(out, [RESET]);
            assert!(stop.confirms());
            assert_eq!(
                stop.kept_position(),
                held,
                "a reset while homing loses the position"
            );
        }
    }

    #[test]
    fn a_hold_that_never_completes_is_reset_after_the_limit() {
        let mut out = Vec::new();
        let mut stop = Stop::start(false, false, true, None, LIMITS, s(0), &mut out);
        assert!(out.is_empty(), "the pending pause already sent `!`");
        stop.on_time(s(1), &mut out);
        assert!(out.is_empty());
        assert!(!stop.kept_position());
        stop.on_time(s(2), &mut out);
        assert_eq!(out, [RESET]);
        assert!(!stop.kept_position(), "the reset came while moving");
        assert_eq!(stop.on_time(s(4), &mut out), Tick::Waiting);
        assert_eq!(stop.on_time(s(5), &mut out), Tick::Unconfirmed);
        assert_eq!(stop.hold_limit(), s(2));
    }
}
