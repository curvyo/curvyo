//! One job streaming to GRBL (`specs/0127` G6, G8 to G11): the prepared lines, what is in flight,
//! the hold and the running time without pauses. Pure: time comes in as `at`.

use std::time::Duration;

use crate::grbl::flow::Flow;
use crate::grbl::prepare::is_system;

/// Where the job stands with GRBL's feed hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hold {
    /// Lines go out.
    Running,
    /// `Pause` arrived while the job's `$H` is in flight: nothing written, the hold is sent when
    /// GRBL answers the `$H` (`specs/0133` P4).
    Deferred,
    /// `!` sent; `Paused` once GRBL reports `Hold:0`, dropped after the time if there is one
    /// (`0127` G29); the `!` sent after `$H`'s answer has none and is never dropped.
    Requested(Option<Duration>),
    /// GRBL holds; `spindle_stopped` when the connector sent Toggle Spindle Stop (`$32=0`).
    Held {
        /// The connector switched the spindle off for the hold.
        spindle_stopped: bool,
    },
    /// `~` sent; `Running` once GRBL reports `Run` or `Idle`.
    Resuming,
}

/// The job.
#[derive(Debug, Clone)]
pub struct Stream {
    lines: Vec<String>,
    next: usize,
    flow: Flow,
    /// The hold state.
    pub hold: Hold,
    /// A `$` line is in flight: nothing else is sent until it is answered.
    system: bool,
    ran: Duration,
    last: Duration,
    /// The report count when the last line was answered: the next `Idle` report completes it.
    pub finished_at_report: Option<u64>,
    /// A restart from a line: the entry lines that come first, owed but no job line.
    entry: usize,
    /// The job line of the first line after the entry (1 for a whole job).
    first_line: u32,
}

impl Stream {
    /// The prepared lines of a job started at `at`.
    #[must_use]
    pub fn new(lines: Vec<String>, at: Duration) -> Self {
        Self {
            lines,
            next: 0,
            flow: Flow::default(),
            hold: Hold::Running,
            system: false,
            ran: Duration::ZERO,
            last: at,
            finished_at_report: None,
            entry: 0,
            first_line: 1,
        }
    }

    /// A restart from `first_line` (`specs/0118` 2e, adrs R9): the `entry` lines, then `lines`,
    /// the job's lines from `first_line` on. The entry lines count no job line.
    #[must_use]
    pub fn new_at(entry: Vec<String>, lines: Vec<String>, first_line: u32, at: Duration) -> Self {
        let count = entry.len();
        let mut all = entry;
        all.extend(lines);
        Self {
            entry: count,
            first_line: first_line.max(1),
            ..Self::new(all, at)
        }
    }

    /// The job line of the line at `index` of the stream; an entry line is its restart line's.
    fn job_line(&self, index: usize) -> u32 {
        let past = index.saturating_sub(self.entry);
        let past = u32::try_from(past).unwrap_or(u32::MAX);
        self.first_line.saturating_add(past)
    }

    /// Whether every entry line was answered (a restart reached its own lines' turn).
    #[must_use]
    pub fn entry_answered(&self) -> bool {
        usize::try_from(self.flow.answered()).unwrap_or(usize::MAX) >= self.entry
    }

    /// Counts the time up to `at`, pauses excluded (`0064` Q14).
    pub fn tick(&mut self, at: Duration) {
        if !matches!(self.hold, Hold::Held { .. }) {
            self.ran += at.saturating_sub(self.last);
        }
        self.last = at;
    }

    /// Milliseconds the job has run.
    #[must_use]
    pub fn elapsed_ms(&self) -> u64 {
        u64::try_from(self.ran.as_millis()).unwrap_or(u64::MAX)
    }

    /// Writes the next lines that fit beside `other` bytes of non-job lines; empty lines count
    /// as they come. Nothing goes out unless the job runs.
    pub fn fill(&mut self, other: usize, out: &mut Vec<u8>) {
        if self.hold != Hold::Running {
            return;
        }
        while let Some(line) = self.lines.get(self.next) {
            if self.system {
                break;
            }
            let job_line = self.job_line(self.next);
            if line.is_empty() {
                self.flow.push(job_line, 0);
                self.next += 1;
                continue;
            }
            let bytes = line.len() + 1;
            let alone = is_system(line);
            if (alone && !self.flow.is_idle()) || !self.flow.fits(bytes, other) {
                break;
            }
            // One write per line, so no real-time byte lands inside it (addendum C4).
            out.extend_from_slice(line.as_bytes());
            out.push(b'\n');
            self.flow.push(job_line, bytes);
            self.next += 1;
            if alone {
                self.system = true;
            }
        }
    }

    /// After GRBL answered `$H` with `ok`: a pause deferred while it homed is requested now, without
    /// a drop limit, so `fill` sends nothing more; `true` means the caller writes `!` at once.
    pub fn send_deferred_hold(&mut self) -> bool {
        if self.hold != Hold::Deferred || self.homing() {
            return false;
        }
        self.hold = Hold::Requested(None);
        true
    }

    /// GRBL answered the oldest line: its job line, `None` when none was in flight.
    pub fn answer(&mut self) -> Option<u32> {
        let line = self.flow.answer();
        if self.flow.is_idle() {
            self.system = false;
        }
        line
    }

    /// Lines answered, counted in the job's text: a restart from line n starts at n - 1 and the
    /// entry lines count none.
    #[must_use]
    pub fn answered(&self) -> u32 {
        let lines = usize::try_from(self.flow.answered()).unwrap_or(usize::MAX);
        let past = u32::try_from(lines.saturating_sub(self.entry)).unwrap_or(u32::MAX);
        (self.first_line - 1).saturating_add(past)
    }

    /// Bytes in flight.
    #[must_use]
    pub fn in_flight(&self) -> usize {
        self.flow.used()
    }

    /// Whether every line was written and answered.
    #[must_use]
    pub fn all_answered(&self) -> bool {
        self.next == self.lines.len() && self.flow.is_idle()
    }

    /// Whether a `$` line (homing) is in flight.
    #[must_use]
    pub fn system_in_flight(&self) -> bool {
        self.system
    }

    /// Whether the line in flight is `$H`: a `$` line goes alone, so it is the last one written.
    #[must_use]
    pub fn homing(&self) -> bool {
        self.system
            && self
                .next
                .checked_sub(1)
                .and_then(|i| self.lines.get(i))
                .is_some_and(|l| l.to_ascii_uppercase().starts_with("$H"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &[&str]) -> Vec<String> {
        text.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn lines_go_out_as_they_fit_and_never_while_held() {
        let long = "G1X".to_owned() + &"1".repeat(56); // 60 bytes with the newline
        let mut stream = Stream::new(vec![long.clone(), long.clone(), long], Duration::ZERO);
        let mut out = Vec::new();
        stream.fill(0, &mut out);
        assert_eq!(out.len(), 120);
        assert_eq!(stream.answer(), Some(1));
        stream.hold = Hold::Requested(Some(Duration::from_secs(2)));
        out.clear();
        stream.fill(0, &mut out);
        assert!(out.is_empty());
        stream.hold = Hold::Running;
        stream.fill(0, &mut out);
        assert_eq!(out.len(), 60);
    }

    #[test]
    fn a_system_line_goes_alone() {
        let mut stream = Stream::new(lines(&["$H", "", "G0X0", "G1X1"]), Duration::ZERO);
        let mut out = Vec::new();
        stream.fill(0, &mut out);
        assert_eq!(out, b"$H\n");
        assert!(stream.system_in_flight());
        assert!(stream.homing());
        stream.fill(0, &mut out);
        assert_eq!(out, b"$H\n");
        assert_eq!(stream.answer(), Some(1));
        assert!(!stream.homing());
        stream.fill(0, &mut out);
        assert_eq!(out, b"$H\nG0X0\nG1X1\n");
        assert_eq!(
            stream.answered(),
            2,
            "the empty line counts once line 1 is answered"
        );
        stream.answer();
        stream.answer();
        assert_eq!(stream.answered(), 4);
        assert!(stream.all_answered());
    }

    #[test]
    fn only_dollar_h_in_flight_is_homing() {
        let mut stream = Stream::new(lines(&["$$", "$H"]), Duration::ZERO);
        let mut out = Vec::new();
        stream.fill(0, &mut out);
        assert!(stream.system_in_flight() && !stream.homing());
        stream.answer();
        stream.fill(0, &mut out);
        assert!(stream.homing());
    }

    #[test]
    fn a_deferred_hold_writes_nothing_and_hands_over_once_after_homing() {
        let mut stream = Stream::new(lines(&["$H", "G0X0", "G1X1"]), Duration::ZERO);
        let mut out = Vec::new();
        stream.fill(0, &mut out);
        assert!(stream.homing());
        stream.hold = Hold::Deferred;
        assert!(!stream.send_deferred_hold(), "not while `$H` is in flight");
        stream.fill(0, &mut out);
        assert_eq!(out, b"$H\n");
        assert_eq!(stream.answer(), Some(1));
        stream.fill(0, &mut out);
        assert_eq!(out, b"$H\n", "nothing while deferred");
        assert!(stream.send_deferred_hold());
        assert_eq!(stream.hold, Hold::Requested(None));
        assert!(!stream.send_deferred_hold(), "exactly once");
        stream.fill(0, &mut out);
        assert_eq!(out, b"$H\n", "nothing while the hold is requested");
    }

    #[test]
    fn a_restart_counts_from_its_line_and_its_entry_counts_none() {
        let mut stream = Stream::new_at(
            lines(&["M5", "G1X1F3000S0"]),
            lines(&["G1X2", "G1X3"]),
            11,
            Duration::ZERO,
        );
        let mut out = Vec::new();
        stream.fill(0, &mut out);
        assert_eq!(out, b"M5\nG1X1F3000S0\nG1X2\nG1X3\n");
        assert_eq!(stream.answered(), 10);
        assert_eq!(
            stream.answer(),
            Some(11),
            "an entry line is its restart line's"
        );
        assert!(!stream.entry_answered());
        stream.answer();
        assert!(stream.entry_answered());
        assert_eq!(stream.answered(), 10);
        assert_eq!(stream.answer(), Some(11));
        assert_eq!(stream.answered(), 11);
        assert_eq!(stream.answer(), Some(12));
        assert_eq!(stream.answered(), 12);
        assert!(stream.all_answered());
    }

    #[test]
    fn the_time_excludes_the_hold() {
        let mut stream = Stream::new(lines(&["G0"]), Duration::ZERO);
        stream.tick(Duration::from_millis(500));
        stream.hold = Hold::Held {
            spindle_stopped: false,
        };
        stream.tick(Duration::from_millis(5_500));
        stream.hold = Hold::Running;
        stream.tick(Duration::from_millis(6_000));
        assert_eq!(stream.elapsed_ms(), 1_000);
    }
}
