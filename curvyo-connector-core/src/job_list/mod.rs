//! The ordered list of jobs a connector holds behind its current job: order, limits and moves.

mod chain;
mod rights;
mod stored;

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use chain::{Chain, End, next};
pub use rights::{Action, may};
pub use stored::{Dropped, Restored, STORED_FORMAT, StoredCurrent, StoredList, restore};

use crate::machine::Peer;
use crate::message::ListEntry;

/// The most jobs the list holds, waiting, interrupted and in flight together.
pub const MAX_ENTRIES: usize = 20;
/// The most bytes the list holds in those jobs together.
pub const MAX_BYTES: u64 = 256 * 1024 * 1024;

/// One job of the list as the connector stores it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// The job.
    pub job_id: Uuid,
    /// The name sent with it.
    pub name: String,
    /// Its lines.
    pub lines: u32,
    /// Its size in bytes.
    pub bytes: u64,
    /// SHA-256 of its text, lower-case hex.
    pub sha256: String,
    /// The client that sent it.
    pub sent_by: String,
    /// Who marked it "Run next automatically".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_next_by: Option<Peer>,
    /// The line it had reached when the connector stopped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interrupted_at: Option<u32>,
}

impl Entry {
    /// The entry as the app sees it at `position` (0 for the current and an interrupted job).
    #[must_use]
    pub fn shown(&self, position: u32) -> ListEntry {
        ListEntry {
            job_id: self.job_id,
            name: self.name.clone(),
            lines: self.lines,
            bytes: self.bytes,
            sent_by: self.sent_by.clone(),
            position,
            run_next: self.run_next_by.is_some(),
            interrupted_at: self.interrupted_at,
        }
    }
}

/// An entry with its text.
#[derive(Debug, Clone)]
pub struct Item {
    /// The entry.
    pub entry: Entry,
    /// The G-code.
    pub text: Arc<str>,
}

/// Why a move was not made: the job is not where the app saw it, or the target is out of range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StalePosition;

/// The jobs behind the current one: an interrupted job first, then the waiting jobs in order.
#[derive(Debug, Clone, Default)]
pub struct JobList {
    items: Vec<Item>,
}

impl JobList {
    /// The items, interrupted first.
    #[must_use]
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// The items, consumed.
    #[must_use]
    pub fn into_items(self) -> Vec<Item> {
        self.items
    }

    /// How many jobs the list holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the list holds no job.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The bytes of every job in the list.
    #[must_use]
    pub fn bytes(&self) -> u64 {
        self.items.iter().map(|i| i.entry.bytes).sum()
    }

    /// Whether a job of `bytes` more fits next to `in_flight` jobs and `in_flight_bytes` that are
    /// still arriving.
    #[must_use]
    pub fn fits(&self, in_flight: usize, in_flight_bytes: u64, bytes: u64) -> bool {
        self.len() + in_flight < MAX_ENTRIES && self.bytes() + in_flight_bytes + bytes <= MAX_BYTES
    }

    /// The first waiting job that is not interrupted.
    #[must_use]
    pub fn first_waiting(&self) -> Option<&Item> {
        self.items.iter().find(|i| i.entry.interrupted_at.is_none())
    }

    /// The item of `job_id`.
    #[must_use]
    pub fn get(&self, job_id: Uuid) -> Option<&Item> {
        self.items.iter().find(|i| i.entry.job_id == job_id)
    }

    /// The 1-based place of `job_id` among the waiting jobs; `None` for an interrupted job.
    #[must_use]
    pub fn position(&self, job_id: Uuid) -> Option<u32> {
        self.items
            .iter()
            .filter(|i| i.entry.interrupted_at.is_none())
            .position(|i| i.entry.job_id == job_id)
            .map(|p| u32::try_from(p + 1).unwrap_or(u32::MAX))
    }

    /// Adds a job at the end of the waiting jobs; its place.
    pub fn push(&mut self, item: Item) -> u32 {
        let interrupted = item.entry.interrupted_at.is_some();
        if interrupted {
            let at = self
                .items
                .iter()
                .take_while(|i| i.entry.interrupted_at.is_some())
                .count();
            self.items.insert(at, item);
            return 0;
        }
        let id = item.entry.job_id;
        self.items.push(item);
        self.position(id).unwrap_or(0)
    }

    /// Takes the job out of the list.
    pub fn remove(&mut self, job_id: Uuid) -> Option<Item> {
        let at = self.items.iter().position(|i| i.entry.job_id == job_id)?;
        Some(self.items.remove(at))
    }

    /// Moves the waiting job `job_id` from place `from` to place `to`.
    ///
    /// # Errors
    /// `StalePosition` when the job is not at `from`, is interrupted, or `to` is out of range.
    pub fn move_to(&mut self, job_id: Uuid, from: u32, to: u32) -> Result<(), StalePosition> {
        let waiting = self
            .items
            .iter()
            .filter(|i| i.entry.interrupted_at.is_none())
            .count();
        let target = usize::try_from(to).unwrap_or(usize::MAX);
        if self.position(job_id) != Some(from) || target == 0 || target > waiting {
            return Err(StalePosition);
        }
        let interrupted = self.items.len() - waiting;
        let Some(item) = self.remove(job_id) else {
            return Err(StalePosition);
        };
        self.items.insert(interrupted + target - 1, item);
        Ok(())
    }

    /// Sets or clears the flag of `job_id`; false when the job is not listed.
    pub fn set_run_next(&mut self, job_id: Uuid, by: Option<Peer>) -> bool {
        match self.items.iter_mut().find(|i| i.entry.job_id == job_id) {
            Some(item) => {
                item.entry.run_next_by = by;
                true
            }
            None => false,
        }
    }

    /// Clears every flag; true when one was set.
    pub fn clear_flags(&mut self) -> bool {
        let mut any = false;
        for item in &mut self.items {
            any |= item.entry.run_next_by.take().is_some();
        }
        any
    }

    /// The entries as the app sees them: an interrupted job at place 0, then the waiting jobs.
    #[must_use]
    pub fn shown(&self) -> Vec<ListEntry> {
        self.items
            .iter()
            .map(|i| {
                let place = self.position(i.entry.job_id).unwrap_or(0);
                i.entry.shown(place)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(n: u128, interrupted: Option<u32>, bytes: u64) -> Item {
        Item {
            entry: Entry {
                job_id: Uuid::from_u128(n),
                name: format!("j{n}"),
                lines: 1,
                bytes,
                sha256: String::new(),
                sent_by: "Phone".into(),
                run_next_by: None,
                interrupted_at: interrupted,
            },
            text: Arc::from("G0\n"),
        }
    }

    fn order(list: &JobList) -> Vec<u128> {
        list.items()
            .iter()
            .map(|i| i.entry.job_id.as_u128())
            .collect()
    }

    fn filled() -> JobList {
        let mut list = JobList::default();
        for n in 1..=4 {
            list.push(item(n, None, 10));
        }
        list
    }

    #[test]
    fn jobs_wait_in_order_of_arrival_with_one_based_places() {
        let list = filled();
        assert_eq!(order(&list), [1, 2, 3, 4]);
        assert_eq!(list.position(Uuid::from_u128(3)), Some(3));
        assert_eq!(list.position(Uuid::from_u128(9)), None);
    }

    #[test]
    fn move_table() {
        // (job, from, to) -> order after, or None for a stale answer.
        let cases: [(u128, u32, u32, Option<[u128; 4]>); 9] = [
            (3, 3, 1, Some([3, 1, 2, 4])),
            (1, 1, 4, Some([2, 3, 4, 1])),
            (2, 2, 3, Some([1, 3, 2, 4])),
            (2, 2, 2, Some([1, 2, 3, 4])),
            (2, 3, 1, None),
            (2, 2, 0, None),
            (2, 2, 5, None),
            (9, 1, 2, None),
            (4, 4, 3, Some([1, 2, 4, 3])),
        ];
        for (job, from, to, expected) in cases {
            let mut list = filled();
            let result = list.move_to(Uuid::from_u128(job), from, to);
            if let Some(order_after) = expected {
                assert_eq!(result, Ok(()), "{job} {from}->{to}");
                assert_eq!(order(&list), order_after, "{job} {from}->{to}");
            } else {
                assert_eq!(result, Err(StalePosition), "{job} {from}->{to}");
                assert_eq!(order(&list), [1, 2, 3, 4], "{job} {from}->{to}");
            }
        }
    }

    #[test]
    fn an_interrupted_job_stays_first_and_cannot_be_moved_or_passed() {
        let mut list = filled();
        list.push(item(7, Some(12), 10));
        assert_eq!(order(&list), [7, 1, 2, 3, 4]);
        assert_eq!(list.position(Uuid::from_u128(7)), None);
        assert_eq!(list.position(Uuid::from_u128(1)), Some(1));
        assert_eq!(
            list.first_waiting().map(|i| i.entry.job_id.as_u128()),
            Some(1)
        );
        assert_eq!(list.move_to(Uuid::from_u128(7), 0, 1), Err(StalePosition));
        assert_eq!(list.move_to(Uuid::from_u128(4), 4, 1), Ok(()));
        assert_eq!(order(&list), [7, 4, 1, 2, 3]);
        let shown = list.shown();
        assert_eq!(shown[0].position, 0);
        assert_eq!(shown[0].interrupted_at, Some(12));
        assert_eq!(shown[1].position, 1);
    }

    #[test]
    fn limits_count_entries_in_flight_and_bytes() {
        let mut list = JobList::default();
        for n in 0..19 {
            assert!(list.fits(0, 0, 1), "{n}");
            list.push(item(n, None, 1));
        }
        assert!(list.fits(0, 0, 1));
        assert!(!list.fits(1, 0, 1), "the 20th is an arriving transfer");
        list.push(item(99, None, 1));
        assert!(!list.fits(0, 0, 1));

        let mut big = JobList::default();
        big.push(item(1, None, MAX_BYTES - 10));
        assert!(big.fits(0, 0, 10));
        assert!(!big.fits(0, 0, 11));
        assert!(!big.fits(0, 1, 10), "bytes of a transfer in flight count");
    }

    #[test]
    fn remove_and_flags() {
        let mut list = filled();
        assert!(list.remove(Uuid::from_u128(2)).is_some());
        assert!(list.remove(Uuid::from_u128(2)).is_none());
        assert_eq!(order(&list), [1, 3, 4]);
        let owner = Peer::new("Laptop", crate::admin::Role::Owner);
        assert!(list.set_run_next(Uuid::from_u128(3), Some(owner)));
        assert!(!list.set_run_next(Uuid::from_u128(9), None));
        assert!(list.shown()[1].run_next);
        assert!(list.clear_flags());
        assert!(!list.clear_flags());
    }
}
