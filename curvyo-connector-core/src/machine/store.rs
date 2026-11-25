//! What the state directory keeps of the job machine: `jobs/list.json` with the job texts
//! (`specs/0132` L6, `specs/0118` adrs R2).

use std::sync::Arc;

use uuid::Uuid;

use super::Machine;
use super::list::entry_of;
use crate::job_list::{STORED_FORMAT, StopPoint, StoredCurrent, StoredList};
use crate::message::State;

impl Machine {
    /// The list as `jobs/list.json` keeps it, the running job's line approximate (written at most
    /// once a second).
    #[must_use]
    pub fn stored(&self) -> StoredList {
        self.stored_at(false)
    }

    /// The list as `jobs/list.json` keeps it; `exact` when it is written at the stop itself. A
    /// restart that has not reached its line keeps its entry as it was.
    #[must_use]
    pub fn stored_at(&self, exact: bool) -> StoredList {
        let current = self
            .held
            .as_ref()
            .filter(|_| self.returning().is_none())
            .filter(|_| {
                matches!(
                    self.state,
                    State::Ready
                        | State::AwaitingLocalStart
                        | State::Homing
                        | State::Jogging
                        | State::Running
                        | State::Paused
                )
            })
            .map(|h| {
                let started =
                    matches!(self.state, State::Running | State::Paused) || self.waits_for_resume();
                let mut entry = entry_of(h);
                entry.stop = started.then(|| StopPoint {
                    machine_position: self.mpos,
                    work_offset: self.wco,
                    run_from: h.from_line.unwrap_or(1),
                    exact,
                });
                StoredCurrent {
                    entry,
                    started,
                    line: h.line,
                }
            });
        let waiting = self.returning().into_iter().chain(self.list.items());
        StoredList {
            format: STORED_FORMAT,
            current,
            waiting: waiting.map(|i| i.entry.clone()).collect(),
        }
    }

    /// The texts `jobs/` keeps: the current job's and every listed job's.
    #[must_use]
    pub fn stored_texts(&self) -> Vec<(Uuid, Arc<str>)> {
        let stored_current = self.stored().current.is_some();
        let current = self
            .held
            .iter()
            .filter(|_| stored_current)
            .map(|h| (h.job.job_id, Arc::clone(&h.job.text)));
        let waiting = self
            .returning()
            .into_iter()
            .chain(self.list.items())
            .map(|i| (i.entry.job_id, Arc::clone(&i.text)));
        current.chain(waiting).collect()
    }
}
