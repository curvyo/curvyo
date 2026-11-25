//! The job list in the state directory (`specs/0132` L6): `jobs/<job_id>.gcode` and
//! `jobs/list.json`, written by one ordered thread so a 64 MiB write never holds the hub lock.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;

use curvyo_connector_core::{Restored, StoredList, restore};
use uuid::Uuid;

use crate::identity::{make_dir, write_private};

/// The folder in the state directory.
pub const JOBS_DIR: &str = "jobs";
/// The list file in it.
pub const LIST_FILE: &str = "list.json";

enum Op {
    Text(Uuid, Arc<str>),
    Remove(Uuid),
    List(String),
    Flush(Sender<()>),
}

/// The ordered writer of the job store.
pub struct JobStore {
    sender: Sender<Op>,
    thread: Option<JoinHandle<()>>,
}

/// What a restart found.
#[derive(Debug, Default)]
pub struct Loaded {
    /// The jobs that came back, and the ones dropped.
    pub restored: Restored,
    /// The stored list could not be read: every job was dropped.
    pub list_dropped: Option<String>,
}

fn dir_of(state_dir: &Path) -> PathBuf {
    state_dir.join(JOBS_DIR)
}

/// Reads `jobs/` and rebuilds the list; texts without an entry are deleted. A missing folder is
/// an empty list.
#[must_use]
pub fn load(state_dir: &Path) -> Loaded {
    let dir = dir_of(state_dir);
    let mut loaded = Loaded::default();
    let Ok(bytes) = fs::read(dir.join(LIST_FILE)) else {
        remove_texts(&dir, &HashMap::new());
        return loaded;
    };
    let stored: StoredList = match serde_json::from_slice(&bytes) {
        Ok(list) => list,
        Err(e) => {
            loaded.list_dropped = Some(e.to_string());
            remove_texts(&dir, &HashMap::new());
            return loaded;
        }
    };
    let mut texts: HashMap<Uuid, Arc<str>> = HashMap::new();
    let ids = stored
        .current
        .iter()
        .map(|c| c.entry.job_id)
        .chain(stored.waiting.iter().map(|e| e.job_id));
    for id in ids {
        if let Ok(text) = fs::read_to_string(dir.join(format!("{id}.gcode"))) {
            texts.insert(id, Arc::from(text));
        }
    }
    loaded.restored = restore(&stored, &texts);
    let kept: HashMap<Uuid, Arc<str>> = loaded
        .restored
        .items
        .iter()
        .map(|i| (i.entry.job_id, Arc::clone(&i.text)))
        .collect();
    remove_texts(&dir, &kept);
    loaded
}

/// Deletes the job texts that no entry names.
fn remove_texts(dir: &Path, keep: &HashMap<Uuid, Arc<str>>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(id) = name
            .strip_suffix(".gcode")
            .and_then(|s| s.parse::<Uuid>().ok())
        else {
            continue;
        };
        if !keep.contains_key(&id) {
            let _ = fs::remove_file(entry.path());
        }
    }
}

fn run(dir: &Path, receiver: &Receiver<Op>) {
    for op in receiver {
        match op {
            Op::Text(id, text) => {
                if make_dir(dir).is_ok() {
                    let _ = write_private(&dir.join(format!("{id}.gcode")), text.as_bytes());
                }
            }
            Op::Remove(id) => {
                let _ = fs::remove_file(dir.join(format!("{id}.gcode")));
            }
            Op::List(json) => {
                if make_dir(dir).is_ok() {
                    let _ = write_private(&dir.join(LIST_FILE), json.as_bytes());
                }
            }
            Op::Flush(done) => {
                let _ = done.send(());
            }
        }
    }
}

impl JobStore {
    /// A store writing into `<state_dir>/jobs/`; its thread ends when the store is dropped.
    #[must_use]
    pub fn new(state_dir: &Path) -> Self {
        let (sender, receiver) = channel();
        let dir = dir_of(state_dir);
        let thread = std::thread::Builder::new()
            .name("job-store".to_owned())
            .spawn(move || run(&dir, &receiver))
            .ok();
        Self { sender, thread }
    }

    /// Writes the text of a job.
    pub fn write_text(&self, job_id: Uuid, text: Arc<str>) {
        let _ = self.sender.send(Op::Text(job_id, text));
    }

    /// Deletes the text of a job.
    pub fn remove_text(&self, job_id: Uuid) {
        let _ = self.sender.send(Op::Remove(job_id));
    }

    /// Writes `list.json`.
    pub fn write_list(&self, list: &StoredList) {
        if let Ok(json) = serde_json::to_string_pretty(list) {
            let _ = self.sender.send(Op::List(json));
        }
    }

    /// Returns once everything sent before is on disk.
    pub fn flush(&self) {
        let (done, wait) = channel();
        if self.sender.send(Op::Flush(done)).is_ok() {
            let _ = wait.recv();
        }
    }
}

impl Drop for JobStore {
    fn drop(&mut self) {
        let (closed, _) = channel();
        self.sender = closed;
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_connector_core::Entry;
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;

    fn entry(n: u128, text: &str) -> Entry {
        let sha256 = Sha256::digest(text.as_bytes())
            .iter()
            .fold(String::new(), |mut hex, b| {
                let _ = write!(hex, "{b:02x}");
                hex
            });
        Entry {
            job_id: Uuid::from_u128(n),
            name: format!("j{n}.gcode"),
            lines: 1,
            bytes: text.len() as u64,
            sha256,
            sent_by: "Phone".into(),
            run_next_by: None,
            interrupted_at: None,
            stop: None,
        }
    }

    #[test]
    fn what_was_written_comes_back_in_order_and_orphans_go() {
        let dir = tempfile::tempdir().unwrap();
        {
            let store = JobStore::new(dir.path());
            for n in 1..=3 {
                store.write_text(Uuid::from_u128(n), Arc::from("G0 X1\n"));
            }
            store.write_text(Uuid::from_u128(9), Arc::from("orphan\n"));
            store.write_list(&StoredList {
                format: 1,
                current: None,
                waiting: vec![
                    entry(2, "G0 X1\n"),
                    entry(1, "G0 X1\n"),
                    entry(3, "G0 X1\n"),
                ],
            });
            store.flush();
        }
        let loaded = load(dir.path());
        let order: Vec<u128> = loaded
            .restored
            .items
            .iter()
            .map(|i| i.entry.job_id.as_u128())
            .collect();
        assert_eq!(order, [2, 1, 3]);
        assert!(loaded.restored.dropped.is_empty());
        assert!(
            !dir.path()
                .join("jobs")
                .join(format!("{}.gcode", Uuid::from_u128(9)))
                .exists()
        );
    }

    #[test]
    fn a_corrupt_list_drops_everything_and_a_missing_folder_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load(dir.path()).restored.items.is_empty());
        fs::create_dir_all(dir.path().join("jobs")).unwrap();
        fs::write(dir.path().join("jobs").join(LIST_FILE), "{ not json").unwrap();
        let loaded = load(dir.path());
        assert!(loaded.restored.items.is_empty());
        assert!(loaded.list_dropped.is_some());
    }

    /// `specs/0118` 2e (adrs R2, R11): a started current job is stored with where it stood; it
    /// comes back interrupted with that point, and a slice-1 file without it reads `stop: None`.
    #[test]
    fn an_interrupted_job_keeps_its_stop_point_and_an_old_file_has_none() {
        use curvyo_connector_core::{Millimetres, StopPoint, StoredCurrent};
        let text = "G21\nG90\nG0 X1 Y1\n";
        let mut current = entry(1, text);
        current.lines = 3;
        current.stop = Some(StopPoint {
            machine_position: Some([-30.0, -40.0, 0.0].map(Millimetres)),
            work_offset: Some([0.0; 3].map(Millimetres)),
            run_from: 1,
            exact: true,
        });
        let list = StoredList {
            format: 1,
            current: Some(StoredCurrent {
                entry: current,
                started: true,
                line: 2,
            }),
            waiting: vec![],
        };
        let golden = include_str!("../tests/fixtures/jobs/list_interrupted.json");
        assert_eq!(
            serde_json::to_string_pretty(&list).unwrap(),
            golden.trim_end()
        );
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("jobs")).unwrap();
        fs::write(dir.path().join("jobs").join(LIST_FILE), golden).unwrap();
        fs::write(
            dir.path()
                .join("jobs")
                .join(format!("{}.gcode", Uuid::from_u128(1))),
            text,
        )
        .unwrap();
        let loaded = load(dir.path());
        let back = &loaded.restored.items[0].entry;
        assert_eq!(back.interrupted_at, Some(2));
        assert!(back.stop.as_ref().unwrap().exact);
        let old = golden.replace(
            &golden[golden.find(",\n    \"stop\"").unwrap()
                ..golden.find(",\n    \"started\"").unwrap()],
            "",
        );
        let parsed: StoredList = serde_json::from_str(&old).unwrap();
        assert_eq!(parsed.current.unwrap().entry.stop, None);
    }
}
