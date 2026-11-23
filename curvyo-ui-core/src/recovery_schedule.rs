//! The rule that decides when a recovery copy is written (`specs/0136-autosave-and-restore`
//! criteria 1, 8 and 13, ADR 0079 decision 1), driven by a monotonic clock in ms that the caller
//! passes in.
//!
//! One rule: a committed step is written at once when the last write is [`INTERVAL_MS`] or more
//! back; steps within the interval wait for exactly one trailing write at its end; with no step
//! since the last write nothing is written. A clean project is never written; a write that falls
//! due on a clean project removes the copy instead.

/// The least time between two recovery writes, ms: a constant of the core, not a setting
/// (criterion 13).
pub const INTERVAL_MS: u64 = 300_000;

/// What the caller does now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Due {
    /// Nothing.
    Nothing,
    /// Write the copy: call [`RecoverySchedule::begin`], then [`RecoverySchedule::written`] or
    /// [`RecoverySchedule::failed`].
    Write,
    /// The project is clean again (an undo back to the saved state) and a copy exists: delete it,
    /// then call [`RecoverySchedule::deleted`].
    DeleteCopy,
}

/// The state of one session's recovery copy.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecoverySchedule {
    /// When the last write succeeded.
    last_write: Option<u64>,
    /// The last write failed and no later one succeeded.
    failure: Option<Failure>,
    /// Steps committed that no write holds.
    unwritten: bool,
    /// A write is running.
    writing: bool,
    /// A copy exists on disk or in the store.
    has_copy: bool,
}

/// A failed attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Failure {
    /// When it failed.
    at: u64,
    /// A step was committed since.
    step_since: bool,
}

impl RecoverySchedule {
    /// A schedule for a session with no copy.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A step was committed.
    pub fn step(&mut self) {
        self.unwritten = true;
        if let Some(failure) = self.failure.as_mut() {
            failure.step_since = true;
        }
    }

    /// What to do at `now` (ms, monotonic) for a project that `dirty` says differs from its
    /// saved file. The caller asks at idle moments, never inside a drag.
    #[must_use]
    pub fn due(&self, now: u64, dirty: bool) -> Due {
        if self.writing || !self.unwritten {
            return Due::Nothing;
        }
        if !dirty {
            return if self.has_copy {
                Due::DeleteCopy
            } else {
                Due::Nothing
            };
        }
        let rested = self
            .last_write
            .is_none_or(|at| now.saturating_sub(at) >= INTERVAL_MS);
        let retry = self
            .failure
            .is_none_or(|f| f.step_since || now.saturating_sub(f.at) >= INTERVAL_MS);
        if rested && retry {
            Due::Write
        } else {
            Due::Nothing
        }
    }

    /// A write starts: the steps so far go into it, a later step stays waiting.
    pub fn begin(&mut self) {
        self.writing = true;
        self.unwritten = false;
        if let Some(failure) = self.failure.as_mut() {
            failure.step_since = false;
        }
    }

    /// The write that began succeeded at `now`.
    pub fn written(&mut self, now: u64) {
        self.writing = false;
        self.has_copy = true;
        self.last_write = Some(now);
        self.failure = None;
    }

    /// The write that began failed at `now`; it does not count as a write. Whether the canvas
    /// notice shows: once when the failure first happens, not again on each retry (criterion 13).
    pub fn failed(&mut self, now: u64) -> bool {
        self.writing = false;
        self.unwritten = true;
        let first = self.failure.is_none();
        self.failure = Some(Failure {
            at: now,
            step_since: false,
        });
        first
    }

    /// The copy was deleted because the project is clean.
    pub fn deleted(&mut self) {
        self.has_copy = false;
        self.unwritten = false;
    }

    /// Save, Discard or Close ended the copy: everything starts over, the notice goes.
    pub fn ended(&mut self) {
        *self = Self::default();
    }

    /// Whether the File tab shows the failed-write line (until the next success).
    #[must_use]
    pub fn failure_line(&self) -> bool {
        self.failure.is_some()
    }

    /// Whether a write is running.
    #[must_use]
    pub fn writing(&self) -> bool {
        self.writing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u64 = 60_000;

    fn write_at(schedule: &mut RecoverySchedule, now: u64) {
        assert_eq!(schedule.due(now, true), Due::Write);
        schedule.begin();
        schedule.written(now);
    }

    #[test]
    fn the_first_step_is_written_at_once() {
        let mut s = RecoverySchedule::new();
        s.step();
        assert_eq!(s.due(0, true), Due::Write);
    }

    #[test]
    fn a_step_after_a_gap_of_five_minutes_is_written_at_once() {
        let mut s = RecoverySchedule::new();
        s.step();
        write_at(&mut s, 1_000);
        s.step();
        assert_eq!(s.due(1_000 + INTERVAL_MS, true), Due::Write);
    }

    #[test]
    fn steps_within_the_interval_give_one_trailing_write_when_it_expires() {
        let mut s = RecoverySchedule::new();
        s.step();
        write_at(&mut s, 0);
        for at in [MIN, 2 * MIN, 4 * MIN] {
            s.step();
            assert_eq!(s.due(at, true), Due::Nothing, "not written one by one");
        }
        assert_eq!(s.due(INTERVAL_MS - 1, true), Due::Nothing);
        assert_eq!(s.due(INTERVAL_MS, true), Due::Write);
        s.begin();
        s.written(INTERVAL_MS);
        assert_eq!(s.due(INTERVAL_MS + 1, true), Due::Nothing, "exactly one");
    }

    #[test]
    fn no_step_since_the_last_write_writes_nothing() {
        let mut s = RecoverySchedule::new();
        s.step();
        write_at(&mut s, 0);
        assert_eq!(s.due(10 * INTERVAL_MS, true), Due::Nothing);
        assert_eq!(RecoverySchedule::new().due(0, true), Due::Nothing);
    }

    #[test]
    fn a_clean_project_is_never_written() {
        let mut s = RecoverySchedule::new();
        s.step();
        assert_eq!(s.due(0, false), Due::Nothing, "no copy to delete either");
    }

    #[test]
    fn a_due_write_on_a_clean_project_deletes_the_copy() {
        let mut s = RecoverySchedule::new();
        s.step();
        write_at(&mut s, 0);
        s.step(); // an undo back to the saved state
        assert_eq!(s.due(1, false), Due::DeleteCopy);
        s.deleted();
        assert_eq!(s.due(2, false), Due::Nothing);
    }

    #[test]
    fn a_step_during_a_write_stays_pending() {
        let mut s = RecoverySchedule::new();
        s.step();
        assert_eq!(s.due(0, true), Due::Write);
        s.begin();
        s.step();
        assert_eq!(s.due(1, true), Due::Nothing, "a write is running");
        s.written(2);
        assert_eq!(s.due(3, true), Due::Nothing, "inside the interval");
        assert_eq!(s.due(2 + INTERVAL_MS, true), Due::Write);
    }

    #[test]
    fn a_failure_is_no_write_and_retries_on_the_next_step() {
        let mut s = RecoverySchedule::new();
        s.step();
        s.begin();
        assert!(s.failed(10), "the first failure shows the notice");
        assert_eq!(s.due(11, true), Due::Nothing, "no retry every second");
        s.step();
        assert_eq!(s.due(12, true), Due::Write, "the next step retries");
    }

    #[test]
    fn a_failure_retries_five_minutes_after_the_attempt_without_a_step() {
        let mut s = RecoverySchedule::new();
        s.step();
        s.begin();
        let _ = s.failed(10);
        assert_eq!(s.due(10 + INTERVAL_MS - 1, true), Due::Nothing);
        assert_eq!(s.due(10 + INTERVAL_MS, true), Due::Write);
    }

    #[test]
    fn the_failure_notice_shows_once_and_clears_on_the_next_success() {
        let mut s = RecoverySchedule::new();
        s.step();
        s.begin();
        assert!(s.failed(1));
        assert!(s.failure_line());
        s.step();
        s.begin();
        assert!(!s.failed(2), "a retry does not show it again");
        assert!(s.failure_line());
        s.step();
        s.begin();
        s.written(3);
        assert!(!s.failure_line());
        s.step();
        s.begin();
        assert!(
            s.failed(INTERVAL_MS + 10),
            "a new failure after a success shows again"
        );
    }

    #[test]
    fn ended_starts_everything_over() {
        let mut s = RecoverySchedule::new();
        s.step();
        s.begin();
        let _ = s.failed(1);
        s.ended();
        assert!(!s.failure_line());
        assert_eq!(s.due(2, true), Due::Nothing);
        s.step();
        assert_eq!(
            s.due(3, true),
            Due::Write,
            "the first step after Save is written at once"
        );
    }
}
