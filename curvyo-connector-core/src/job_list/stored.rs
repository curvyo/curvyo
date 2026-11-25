//! The job list as the state directory keeps it, and how it comes back after a restart
//! (`specs/0132` criterion 6, ADR notes L6).

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Entry, Item};
use crate::assembly::check_whole;

/// The stored form of the current job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredCurrent {
    /// The job.
    #[serde(flatten)]
    pub entry: Entry,
    /// It was started: it ran, was paused or waited for a resume.
    pub started: bool,
    /// The line it had reached (approximate: written at most once a second).
    pub line: u32,
}

/// `jobs/list.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StoredList {
    /// The format of this file.
    pub format: u32,
    /// The current job, if the connector held one that is not over.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current: Option<StoredCurrent>,
    /// The waiting jobs in order, an interrupted one first.
    #[serde(default)]
    pub waiting: Vec<Entry>,
}

/// The format of `jobs/list.json`.
pub const STORED_FORMAT: u32 = 1;

/// A job that could not come back, for `admin.log`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dropped {
    /// Its name.
    pub name: String,
    /// Its id and the cause (`<id> text missing`).
    pub detail: String,
}

/// The list after a restart.
#[derive(Debug, Default)]
pub struct Restored {
    /// The jobs that came back in order, an interrupted one first.
    pub items: Vec<Item>,
    /// The jobs that did not.
    pub dropped: Vec<Dropped>,
}

/// Rebuilds the list from the stored one and the texts found (by job id): a current job that
/// never started comes back first in the list, a started one as interrupted at its line, a text
/// that is missing or fails the checks is dropped, nothing starts.
#[must_use]
pub fn restore<S: std::hash::BuildHasher>(
    list: &StoredList,
    texts: &HashMap<Uuid, Arc<str>, S>,
) -> Restored {
    let mut out = Restored::default();
    let mut interrupted = Vec::new();
    let mut waiting = Vec::new();
    let current = list.current.iter().map(|c| {
        let mut entry = c.entry.clone();
        if c.started {
            entry.interrupted_at = Some(c.line);
        }
        entry
    });
    for entry in current.chain(list.waiting.iter().cloned()) {
        let Some(text) = texts.get(&entry.job_id) else {
            out.dropped.push(dropped(&entry, "text missing"));
            continue;
        };
        if let Err(reason) =
            check_whole(entry.job_id, &entry.name, entry.bytes, &entry.sha256, text)
        {
            out.dropped.push(dropped(&entry, &format!("{reason:?}")));
            continue;
        }
        let item = Item {
            entry,
            text: Arc::clone(text),
        };
        if item.entry.interrupted_at.is_some() {
            interrupted.push(item);
        } else {
            waiting.push(item);
        }
    }
    out.items = interrupted;
    out.items.extend(waiting);
    out
}

fn dropped(entry: &Entry, cause: &str) -> Dropped {
    Dropped {
        name: entry.name.clone(),
        detail: format!("{} {cause}", entry.job_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine::Peer;
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;

    fn entry(n: u128, text: &str) -> (Entry, Arc<str>) {
        let sha256 = Sha256::digest(text.as_bytes())
            .iter()
            .fold(String::new(), |mut hex, b| {
                let _ = write!(hex, "{b:02x}");
                hex
            });
        (
            Entry {
                job_id: Uuid::from_u128(n),
                name: format!("j{n}.gcode"),
                lines: u32::try_from(text.lines().count()).unwrap(),
                bytes: text.len() as u64,
                sha256,
                sent_by: "Phone".into(),
                run_next_by: None,
                interrupted_at: None,
                stop: None,
            },
            Arc::from(text),
        )
    }

    fn names(restored: &Restored) -> Vec<String> {
        restored
            .items
            .iter()
            .map(|i| match i.entry.interrupted_at {
                Some(line) => format!("{}@{line}", i.entry.name),
                None => i.entry.name.clone(),
            })
            .collect()
    }

    #[test]
    fn restore_table() {
        let (a, ta) = entry(1, "G0 X1\nG0 X2\n");
        let (b, tb) = entry(2, "G0 X3\n");
        let (c, tc) = entry(3, "G0 X4\n");
        let texts: HashMap<_, _> = [(a.job_id, ta), (b.job_id, tb), (c.job_id, tc)]
            .into_iter()
            .collect();
        let current = |entry: &Entry, started, line| StoredCurrent {
            entry: entry.clone(),
            started,
            line,
        };
        let cases = [
            // (stored, expected names)
            (
                StoredList {
                    format: 1,
                    current: None,
                    waiting: vec![b.clone(), c.clone()],
                },
                vec!["j2.gcode", "j3.gcode"],
            ),
            (
                StoredList {
                    format: 1,
                    current: Some(current(&a, false, 0)),
                    waiting: vec![b.clone()],
                },
                vec!["j1.gcode", "j2.gcode"],
            ),
            (
                StoredList {
                    format: 1,
                    current: Some(current(&a, true, 7)),
                    waiting: vec![b.clone(), c.clone()],
                },
                vec!["j1.gcode@7", "j2.gcode", "j3.gcode"],
            ),
        ];
        for (stored, expected) in cases {
            let restored = restore(&stored, &texts);
            assert_eq!(names(&restored), expected);
            assert_eq!(restored.dropped, []);
        }
    }

    #[test]
    fn flags_survive_and_bad_jobs_are_dropped_with_their_cause() {
        let (mut a, ta) = entry(1, "G0 X1\n");
        a.run_next_by = Some(Peer::new("Laptop", crate::admin::Role::Owner));
        let (b, _) = entry(2, "G0 X2\n");
        let (mut c, tc) = entry(3, "G0 X3\n");
        c.sha256 = "0".repeat(64);
        let (d, td) = entry(4, "G0 X4\n");
        let texts: HashMap<_, _> = [(a.job_id, ta), (c.job_id, tc), (d.job_id, td)]
            .into_iter()
            .collect();
        let stored = StoredList {
            format: 1,
            current: None,
            waiting: vec![a, b, c, d],
        };
        let restored = restore(&stored, &texts);
        assert_eq!(names(&restored), ["j1.gcode", "j4.gcode"]);
        assert!(restored.items[0].entry.run_next_by.is_some());
        let causes: Vec<_> = restored.dropped.iter().map(|d| d.detail.clone()).collect();
        assert_eq!(restored.dropped[0].name, "j2.gcode");
        assert!(causes[0].ends_with("text missing"), "{causes:?}");
        assert!(causes[1].ends_with("HashMismatch"), "{causes:?}");
    }
}
