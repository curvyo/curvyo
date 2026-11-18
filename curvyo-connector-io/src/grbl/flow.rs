//! Character counting (`specs/0127` G6): the job lines GRBL has received and not yet answered,
//! and whether the next one still fits its receive buffer.

use std::collections::VecDeque;

/// GRBL's serial receive buffer.
pub const RX_BUFFER: usize = 128;
/// The bytes the connector lets be in flight: the buffer less a safety margin.
pub const RX_BUDGET: usize = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Entry {
    job_line: u32,
    bytes: usize,
}

/// The lines in flight, oldest first. An empty line (nothing left after preparation) takes no
/// bytes and counts as answered once every line before it is answered, so the count stays in
/// job order. The front entry is always a line that was written.
#[derive(Debug, Clone, Default)]
pub struct Flow {
    in_flight: VecDeque<Entry>,
    used: usize,
    answered: u32,
}

impl Flow {
    /// Whether `bytes` more fit beside `other` bytes GRBL holds for non-job lines.
    #[must_use]
    pub fn fits(&self, bytes: usize, other: usize) -> bool {
        self.used + other + bytes <= RX_BUDGET
    }

    /// A job line was written with `bytes` bytes (its newline included), or is empty (`0`).
    pub fn push(&mut self, job_line: u32, bytes: usize) {
        if bytes == 0 && self.in_flight.is_empty() {
            self.answered += 1;
        } else {
            self.in_flight.push_back(Entry { job_line, bytes });
            self.used += bytes;
        }
    }

    /// GRBL answered (`ok` or `error`) the oldest line: its job line, or `None` when no line was
    /// in flight (an answer the connector did not ask for).
    pub fn answer(&mut self) -> Option<u32> {
        let entry = self.in_flight.pop_front()?;
        self.used -= entry.bytes;
        self.answered += 1;
        while self.in_flight.front().is_some_and(|e| e.bytes == 0) {
            self.in_flight.pop_front();
            self.answered += 1;
        }
        Some(entry.job_line)
    }

    /// Job lines answered so far, empty lines included.
    #[must_use]
    pub fn answered(&self) -> u32 {
        self.answered
    }

    /// Bytes in flight.
    #[must_use]
    pub fn used(&self) -> usize {
        self.used
    }

    /// Whether no written line waits for its answer.
    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.in_flight.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_fit_within_the_budget_and_each_answer_frees_the_oldest() {
        let mut flow = Flow::default();
        assert!(flow.fits(120, 0));
        assert!(!flow.fits(121, 0));
        flow.push(1, 60);
        flow.push(2, 50);
        assert!(flow.fits(10, 0));
        assert!(!flow.fits(10, 1));
        assert!(!flow.fits(11, 0));
        assert_eq!(flow.answer(), Some(1));
        assert_eq!(flow.used(), 50);
        assert_eq!(flow.answer(), Some(2));
        assert!(flow.is_idle());
        assert_eq!(flow.answer(), None, "an answer nobody asked for");
        assert_eq!(flow.answered(), 2);
    }

    #[test]
    fn empty_lines_count_in_job_order() {
        let mut flow = Flow::default();
        flow.push(1, 0);
        assert_eq!(flow.answered(), 1, "nothing before it: answered at once");
        flow.push(2, 10);
        flow.push(3, 0);
        flow.push(4, 0);
        assert_eq!(flow.answered(), 1, "waits for line 2");
        flow.push(5, 10);
        assert_eq!(flow.answer(), Some(2));
        assert_eq!(flow.answered(), 4);
        assert_eq!(flow.answer(), Some(5));
        assert_eq!(flow.answered(), 5);
        assert!(flow.is_idle());
    }
}
