//! How a started job that ends early stays listed as interrupted (`specs/0118` adrs R1, Frage
//! 390 C, customer 2026-11-25): a Stop, an alarm or a lost controller while it ran.

use std::sync::Arc;

use uuid::Uuid;

use super::Machine;
use super::list::entry_of;
use crate::job_list::{Item, StopPoint};

impl Machine {
    /// The one place that decides which ends make an interrupted entry: the held job, started
    /// and past its restart line if it is a restart, goes to the list as interrupted at its
    /// acknowledged line with where the machine stood (`exact`). `unbroken` for a Stop: GRBL
    /// keeps its position through the asked reset, so the same machine position proves it until
    /// the next break. `Done { Completed }`, `Failed` and a job that never started never come
    /// here; a connector restart does through `stored::restore`.
    pub(super) fn interrupt(&mut self, unbroken: bool) {
        if self.returning().is_some() {
            // A restart before its line goes back unchanged (R8).
            return;
        }
        let Some(held) = self.held.as_ref() else {
            return;
        };
        let job_id = held.job.job_id;
        let mut entry = entry_of(held);
        entry.interrupted_at = Some(held.line);
        entry.stop = Some(StopPoint {
            machine_position: self.mpos,
            work_offset: self.wco,
            run_from: held.from_line.unwrap_or(1),
            exact: true,
        });
        let item = Item {
            entry,
            text: Arc::clone(&held.job.text),
        };
        let _old = self.list.remove(job_id);
        self.list.push(item);
        self.unbroken.retain(|(id, _)| *id != job_id);
        if unbroken {
            self.unbroken.push((job_id, self.breaks));
        }
        self.plan_all();
    }

    /// Whether nothing broke the position since a Stop interrupted `job_id`.
    pub(super) fn unbroken(&self, job_id: Uuid) -> bool {
        self.unbroken
            .iter()
            .any(|(id, breaks)| *id == job_id && *breaks == self.breaks)
    }
}
