//! The library stream of the device group room as the glue drives it (ADR 0062 decisions 1, 4
//! and 5, `specs/0116-library-sync`): the list replica, the per-item toggles, the entries that
//! changed and wait for the page's session, and the room's framed two-stream snapshot.

use std::collections::{BTreeMap, BTreeSet};

use curvyo_document_core::{LibraryKind, LibraryList, LibraryListError};
use curvyo_sync_core::{
    DEVICES_STREAM_ID, GroupRecord, RecordKind, StreamEntry, encode_group_snapshot,
    encode_library_record, route_group_record, split_snapshot,
};
use curvyo_ui_core::library_text as texts;

use super::{ArrivalChange, DeviceGroupGlue, LibraryArrival};
use crate::session::LibraryEdit;

/// Every entry of the list as `(sync, text)`, keyed by kind and id.
type LibraryState = BTreeMap<(LibraryKind, String), (bool, Option<String>)>;

fn state_of(list: &LibraryList) -> LibraryState {
    let mut state = LibraryState::new();
    for kind in [LibraryKind::Machine, LibraryKind::Material] {
        for entry in list.entries(kind) {
            state.insert((kind, entry.id), (entry.sync, entry.text));
        }
    }
    state
}

impl DeviceGroupGlue {
    /// The library list.
    #[must_use]
    pub const fn library(&self) -> &LibraryList {
        &self.library
    }

    /// The per-item toggles and the library stream's state as
    /// `{"machines": [{"id", "sync"}], "materials": [...], "online", "acked", "state"}` (the
    /// rows of task 7 read this): `online` is the group's connection, `acked` whether every
    /// library change went out and the relay acknowledged it, `state` the toggle tooltip's
    /// second line, formatted in `curvyo-ui-core`.
    #[must_use]
    pub fn library_view_json(&self) -> String {
        let rows = |kind| {
            self.library
                .entries(kind)
                .into_iter()
                .map(|entry| serde_json::json!({"id": entry.id, "sync": entry.sync}))
        };
        let others = self
            .device_ids()
            .iter()
            .filter(|member| **member != self.client.me())
            .count();
        let state = if !self.online {
            texts::OFFLINE_LINE.to_owned()
        } else if !self.library_acked() {
            texts::SYNCING_LINE.to_owned()
        } else {
            texts::in_sync_line(others)
        };
        serde_json::json!({
            "machines": rows(LibraryKind::Machine).collect::<Vec<_>>(),
            "materials": rows(LibraryKind::Material).collect::<Vec<_>>(),
            "online": self.online,
            "acked": self.library_acked(),
            "state": state,
        })
        .to_string()
    }

    /// Whether every library change went out and the relay acknowledged it: the stream's
    /// "this item transfers" state of the toggle (UX notes section 1).
    #[must_use]
    pub fn library_acked(&self) -> bool {
        self.library_pushed.is_empty() && self.library_exported == self.library.version()
    }

    /// Switches the item `id` of `kind` on with `text`, the exact record text (criterion 3);
    /// also the winner write-back of a merge this device's record won (ADR 0062 decision 5).
    pub fn library_set(&mut self, kind: LibraryKind, id: &str, text: &str) {
        if self
            .library
            .entry(kind, id)
            .is_some_and(|entry| entry.text.as_deref() == Some(text))
        {
            return;
        }
        if self.library.set(kind, id, text).is_ok() {
            self.files_changed = true;
            self.bump();
        }
        self.export_library();
        self.pump();
    }

    /// Switches the item off: the entry stays as a marker without text; an item that never
    /// synced writes nothing (criterion 4, P4).
    pub fn library_switch_off(&mut self, kind: LibraryKind, id: &str) {
        if self.library.entry(kind, id).is_some() && self.library.switch_off(kind, id).is_ok() {
            self.snapshot_after_ack = true;
            self.files_changed = true;
            self.bump();
        }
        self.export_library();
        self.pump();
    }

    /// The first sync (criterion 3): writes every local item on behind nothing; the confirm is
    /// the page's. How many entries were written.
    pub fn library_first_sync(&mut self, items: &[(LibraryKind, String, String)]) -> usize {
        let mut written = 0;
        for (kind, id, text) in items {
            if self.library.set(*kind, id, text).is_ok() {
                written += 1;
            }
        }
        if written > 0 {
            self.files_changed = true;
            self.bump();
        }
        self.export_library();
        self.pump();
        written
    }

    /// The local library edits of this device's session (the write list taps): an on item's
    /// entry takes the text, a new item starts on (criterion 4), an off item publishes
    /// nothing (criterion 10), a delete removes the entry of an on item (criterion 11).
    pub fn library_local_edits(&mut self, edits: Vec<LibraryEdit>) {
        let mut changed = false;
        for edit in edits {
            let changed_now = match edit {
                LibraryEdit::Write { kind, id, text } => match self.library.entry(kind, &id) {
                    Some(entry) if !entry.sync => false,
                    Some(entry) if entry.text.as_deref() == Some(text.as_str()) => false,
                    _ => self.library.set(kind, &id, &text).is_ok(),
                },
                LibraryEdit::Remove { kind, id } => {
                    let on = self
                        .library
                        .entry(kind, &id)
                        .is_some_and(|entry| entry.sync);
                    on && {
                        let _ = self.library.remove(kind, &id);
                        self.snapshot_after_ack = true;
                        true
                    }
                }
            };
            changed |= changed_now;
        }
        if changed {
            self.files_changed = true;
            self.bump();
        }
        self.export_library();
        self.pump();
    }

    /// The library entries that changed since the last call, for the page's session to apply.
    pub fn take_library_arrivals(&mut self) -> Vec<LibraryArrival> {
        self.library_arrivals.drain(..).collect()
    }

    /// Pushes the library's changes since the last push, beside the devices list's.
    pub(super) fn export_library(&mut self) {
        let version = self.library.version();
        if version == self.library_exported || !self.online {
            return;
        }
        let Ok(bytes) = self.library.export_updates(&self.library_exported) else {
            return;
        };
        let record = encode_library_record(self.peer, &bytes);
        if let Ok(id) = self.client.push(RecordKind::Update, record) {
            self.library_pushed.insert(id, version.clone());
            self.library_exported = version;
        }
    }

    /// One library update of the room: merged into the list, its changes queued for the page's
    /// session; a newer build's update refused once (ADR 0040 decision 11).
    pub(super) fn import_library(&mut self, payload: &[u8]) {
        self.import_library_bytes(payload, false);
    }

    /// The library part of a framed room snapshot: state-only, so a list with history starts
    /// over from it (`LibraryList::import_snapshot`).
    fn import_library_snapshot(&mut self, payload: &[u8]) {
        self.import_library_bytes(payload, true);
    }

    fn import_library_bytes(&mut self, payload: &[u8], snapshot: bool) {
        // Own changes still waiting go out first: whatever the import adds is the room's, and
        // is not pushed back (a pushed echo would carry the imported history to the others).
        self.export_library();
        let in_step = self.library.version() == self.library_exported;
        let before = state_of(&self.library);
        let result = if snapshot {
            self.library.import_snapshot(self.peer, payload)
        } else {
            self.library.import_updates_seen(payload)
        };
        match result {
            Ok(seen) => {
                self.files_changed = true;
                self.queue_arrivals(&before, seen);
                self.bump();
                if in_step {
                    self.library_exported = self.library.version();
                } else {
                    self.export_library();
                }
            }
            Err(LibraryListError::FormatTooNew) => {
                if !self.newer_library {
                    self.newer_library = true;
                    self.notices.push_back(texts::newer_library_list_notice());
                    self.bump();
                }
            }
            Err(_) => {}
        }
    }

    /// One stream of a framed room snapshot (ADR 0062 decision 1): the devices list's or the
    /// library's, as [`route_group_record`] reads the library's.
    pub(super) fn snapshot_stream(&mut self, entry: &StreamEntry) {
        if entry.stream == DEVICES_STREAM_ID {
            self.import_devices(&entry.bytes);
        } else if let Ok(GroupRecord::Library { payload, .. }) = route_group_record(&entry.bytes) {
            self.import_library_snapshot(payload);
        }
    }

    /// Uploads the room's framed snapshot of both lists, in parts (ADR 0062 decision 1): a
    /// pre-0116 build in the group ignores it and keeps converging through update records.
    pub(super) fn upload_group_snapshot(&mut self, covers_seq: u64) {
        let Some(stream) = self.group_snapshot_stream() else {
            return;
        };
        let id = self.client.fresh_snapshot_id();
        let parts = split_snapshot(&stream, id);
        let _ = self.client.upload_snapshot_parts(&parts, covers_seq);
    }

    /// The room's snapshot stream: the devices list and the library's current state without its
    /// history (a switched-off or deleted item's text is not in it, ADR 0062 decision 4).
    pub(super) fn group_snapshot_stream(&self) -> Option<Vec<u8>> {
        let devices = self.list.snapshot().ok()?;
        let library = self.library.snapshot_state().ok()?;
        Some(encode_group_snapshot(
            &devices,
            &encode_library_record(self.peer, &library),
        ))
    }

    /// The entries that changed between `before` and now, queued for the page's session.
    /// `seen`: the change was made knowing this device's earlier entries, so it supersedes the
    /// entry text they held; otherwise a local record may be a concurrent edit.
    fn queue_arrivals(&mut self, before: &LibraryState, seen: bool) {
        let after = state_of(&self.library);
        let keys: BTreeSet<_> = before.keys().chain(after.keys()).collect();
        for key in keys {
            let change = match (before.get(key), after.get(key)) {
                (None, Some(entry)) => Some(ArrivalChange::Arrived {
                    sync: entry.0,
                    text: entry.1.clone(),
                    previous: None,
                }),
                (Some(_), None) => Some(ArrivalChange::Removed),
                (Some(before_entry), Some(after_entry)) if before_entry != after_entry => {
                    if before_entry.0 && !after_entry.0 {
                        Some(ArrivalChange::SwitchedOff)
                    } else {
                        Some(ArrivalChange::Arrived {
                            sync: after_entry.0,
                            text: after_entry.1.clone(),
                            previous: before_entry.1.clone().filter(|_| seen),
                        })
                    }
                }
                _ => None,
            };
            if let Some(change) = change {
                self.library_arrivals.push_back(LibraryArrival {
                    kind: key.0,
                    id: key.1.clone(),
                    change,
                });
            }
        }
    }
}
