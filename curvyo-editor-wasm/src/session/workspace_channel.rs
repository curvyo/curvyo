//! The workspace part of a sync link (`specs/0095-workspace` criteria 28 to 33, ADR 0037
//! decisions 5, 7 and 8): it routes the opened records of the workspace room by their inner
//! stream.
//!
//! The list stream goes into the list replica, the open document's stream to the editing
//! session, every other document's stream into the background queue that the host writes into
//! the files. A snapshot's parts are reassembled here; on a join its streams become file writes.
//! The marker cursor ([`WorkspaceChannel::persisted_cursor`]) never passes a record that is not
//! in a file yet.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use curvyo_document_core::{DocumentVersion, EntryKind, ListNodeId, WorkspaceList};
use curvyo_sync_core::{RecordKind, SnapshotAssembler, StreamEntry, decode_inner_record};

/// What a background record carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundKind {
    /// Loro updates to import.
    Update,
    /// A picture's bytes for the asset store.
    Asset,
    /// The document's container bytes from a workspace snapshot, to merge.
    Container,
}

/// One record of a document that is not open, waiting to be written into its file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundItem {
    /// The room sequence it came with.
    pub seq: u64,
    /// The document id.
    pub document: [u8; 16],
    /// What it carries.
    pub kind: BackgroundKind,
    /// The payload, without the inner header.
    pub bytes: Vec<u8>,
}

/// Where a document of the workspace keeps its file on this computer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentPlace {
    /// At this path in the folder (the last component ends in `.curvyo`).
    File(Vec<String>),
    /// Deleted from the list: its kept copy in the marker directory (criterion 40).
    Deleted,
}

/// A file the host writes into the workspace folder, in order: folders before what lies in
/// them (a join, criterion 28).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceWrite {
    /// Create this folder.
    Folder {
        /// Its path, one component per entry.
        path: Vec<String>,
    },
    /// Create this document file (the last component ends in `.curvyo`).
    Document {
        /// Its path.
        path: Vec<String>,
        /// The document id.
        document: [u8; 16],
        /// The sequence the bytes hold the room up to.
        seq: u64,
        /// The container bytes.
        bytes: Vec<u8>,
    },
    /// Keep this deleted document's state as its kept copy (a joined snapshot carries it, so
    /// the joiner can restore it too; criterion 40).
    Deleted {
        /// The document id.
        document: [u8; 16],
        /// The container bytes.
        bytes: Vec<u8>,
    },
}

/// Where an opened record goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Routed {
    /// To the open document, as a `0092` record of its own.
    Open(RecordKind, Vec<u8>),
    /// Nowhere further: the list took it, it was queued, or it was refused (`false`).
    Done(bool),
}

/// The workspace state of one sync link.
pub(super) struct WorkspaceChannel {
    pub(super) workspace: [u8; 16],
    /// The workspace's name (its folder's), for the texts.
    pub(super) name: String,
    pub(super) list: WorkspaceList,
    /// The list's Loro peer, the inner header of its records.
    pub(super) list_peer: u64,
    /// The list version whose changes went out.
    pub(super) list_exported: DocumentVersion,
    /// The list version each queued list push brings the relay to.
    pub(super) list_pushed: BTreeMap<u64, DocumentVersion>,
    /// The list version the relay acknowledged last.
    pub(super) list_synced: DocumentVersion,
    /// The open document's id, when it is a document of this workspace.
    pub(super) stream: Option<[u8; 16]>,
    /// Records applied to the open document since its file was last saved; they go back to the
    /// background queue when it is left unsaved.
    pub(super) unsaved: Vec<BackgroundItem>,
    /// Records of other documents not yet handed to the host.
    pub(super) background: VecDeque<BackgroundItem>,
    /// Records handed to the host and not yet reported written, by document.
    pub(super) handed: BTreeMap<[u8; 16], Vec<BackgroundItem>>,
    /// File writes not yet taken by the host, and those taken but not reported written.
    pub(super) writes: VecDeque<WorkspaceWrite>,
    pub(super) writes_pending: BTreeMap<[u8; 16], u64>,
    /// The snapshot being reassembled and its sequence.
    assembling: Option<(u64, SnapshotAssembler)>,
    /// Joining: the first snapshot gives the list and every document as a new file.
    pub(super) joining: bool,
    /// Documents of the joined snapshot written so far and how many the list names.
    pub(super) received: Option<(usize, usize)>,
    /// The parts of the first snapshot, prepared by Share before the room exists.
    pub(super) prepared: Option<Vec<Vec<u8>>>,
    /// A snapshot the room asked for that the host has to build (decision 8), with the sequence
    /// it covers.
    pub(super) snapshot_due: Option<u64>,
    /// Whether the marker's files changed since the host last took them.
    pub(super) marker_changed: bool,
    /// The marker cursor the host last took.
    pub(super) marker_cursor: u64,
    /// The documents' file bytes the host gave for the snapshot being built.
    pub(super) snapshot_documents: Vec<StreamEntry>,
    /// This device's label: its own deletions show no notice.
    pub(super) me: String,
    /// Notices of other members' list changes not yet shown.
    pub(super) notices: Vec<String>,
    /// The labels of other members by the Loro peer of their list replica, learnt from their
    /// presence: a rename or move notice names its author ("Someone" when unknown).
    pub(super) authors: HashMap<u64, String>,
}

impl WorkspaceChannel {
    pub(super) fn new(workspace: [u8; 16], name: &str, list: WorkspaceList, peer: u64) -> Self {
        let version = list.version();
        Self {
            workspace,
            name: name.to_owned(),
            list,
            list_peer: peer,
            list_exported: version.clone(),
            list_pushed: BTreeMap::new(),
            list_synced: version,
            stream: None,
            unsaved: Vec::new(),
            background: VecDeque::new(),
            handed: BTreeMap::new(),
            writes: VecDeque::new(),
            writes_pending: BTreeMap::new(),
            assembling: None,
            joining: false,
            received: None,
            prepared: None,
            snapshot_due: None,
            marker_changed: true,
            marker_cursor: 0,
            snapshot_documents: Vec::new(),
            me: String::new(),
            notices: Vec::new(),
            authors: HashMap::new(),
        }
    }

    /// Routes one opened record of the room.
    pub(super) fn route(&mut self, seq: u64, kind: RecordKind, bytes: &[u8]) -> Routed {
        if kind == RecordKind::Snapshot {
            return self.route_part(seq, bytes);
        }
        let Ok((header, payload)) = decode_inner_record(bytes) else {
            return Routed::Done(false);
        };
        if header.stream == self.workspace {
            return Routed::Done(self.import_list(Some(header.peer), payload));
        }
        if self.joining {
            // Before the snapshot gave the documents, a live record has no file to go into:
            // the snapshot's tail is delivered after it, in order.
            return Routed::Done(true);
        }
        let item = BackgroundItem {
            seq,
            document: header.stream,
            kind: if kind == RecordKind::Asset {
                BackgroundKind::Asset
            } else {
                BackgroundKind::Update
            },
            bytes: payload.to_vec(),
        };
        if self.stream == Some(header.stream) {
            self.unsaved.push(item);
            return Routed::Open(kind, payload.to_vec());
        }
        self.background.push_back(item);
        Routed::Done(true)
    }

    /// Imports list changes; `author` is the list peer of the record's sender (none for a
    /// snapshot).
    fn import_list(&mut self, author: Option<u64>, bytes: &[u8]) -> bool {
        let before = self.list.entries();
        let deleted_before: HashSet<ListNodeId> =
            self.list.deleted_entries().iter().map(|e| e.node).collect();
        let Ok(report) = self.list.import_updates(bytes) else {
            return false;
        };
        self.list_exported = self.list.version();
        self.marker_changed = true;
        self.notices.extend(super::workspace_notices::list_notices(
            &before,
            &self.list.entries(),
            &deleted_before,
            &self.list.deleted_entries(),
            !report.dropped_moves.is_empty(),
            &super::workspace_notices::NoticeNames {
                author: author
                    .and_then(|peer| self.authors.get(&peer))
                    .map_or("", String::as_str),
                me: &self.me,
                workspace: &self.name,
            },
        ));
        true
    }

    /// One part of a workspace snapshot: its completed streams are routed.
    fn route_part(&mut self, seq: u64, part: &[u8]) -> Routed {
        let starts_new = self.assembling.as_ref().is_none_or(|(at, _)| *at != seq);
        if starts_new {
            self.assembling = Some((seq, SnapshotAssembler::default()));
        }
        let Some((_, assembler)) = self.assembling.as_mut() else {
            return Routed::Done(false);
        };
        let Ok(entries) = assembler.push_part(part) else {
            self.assembling = None;
            return Routed::Done(false);
        };
        let complete = assembler.is_complete();
        let mut open: Option<Vec<u8>> = None;
        let mut fine = true;
        for entry in entries {
            match self.snapshot_entry(seq, entry) {
                Routed::Open(_, bytes) => open = Some(bytes),
                Routed::Done(ok) => fine &= ok,
            }
        }
        if complete {
            self.assembling = None;
            self.joining = false;
        }
        match open {
            Some(bytes) => Routed::Open(RecordKind::Snapshot, bytes),
            None => Routed::Done(fine),
        }
    }

    fn snapshot_entry(&mut self, seq: u64, entry: StreamEntry) -> Routed {
        if entry.stream == self.workspace {
            if self.joining {
                let Ok(list) = WorkspaceList::from_snapshot(self.list_peer, &entry.bytes) else {
                    return Routed::Done(false);
                };
                self.list = list;
                self.list_exported = self.list.version();
                self.list_synced = self.list.version();
                self.marker_changed = true;
                self.queue_folders();
                return Routed::Done(true);
            }
            return Routed::Done(self.import_list(None, &entry.bytes));
        }
        if self.joining {
            let Some(path) = self.document_path(entry.stream) else {
                if self.list.tombstoned_documents().contains(&entry.stream) {
                    self.writes_pending.insert(entry.stream, seq);
                    self.writes.push_back(WorkspaceWrite::Deleted {
                        document: entry.stream,
                        bytes: entry.bytes,
                    });
                }
                return Routed::Done(true);
            };
            if let Some((done, _)) = self.received.as_mut() {
                *done += 1;
            }
            self.writes_pending.insert(entry.stream, seq);
            self.writes.push_back(WorkspaceWrite::Document {
                path,
                document: entry.stream,
                seq,
                bytes: entry.bytes,
            });
            return Routed::Done(true);
        }
        let item = BackgroundItem {
            seq,
            document: entry.stream,
            kind: BackgroundKind::Container,
            bytes: entry.bytes,
        };
        if self.stream == Some(entry.stream) {
            let bytes = item.bytes.clone();
            self.unsaved.push(item);
            return Routed::Open(RecordKind::Snapshot, bytes);
        }
        self.background.push_back(item);
        Routed::Done(true)
    }

    /// Every folder of the list, parents first, as writes; the count of documents to receive.
    fn queue_folders(&mut self) {
        let entries = self.list.entries();
        let documents = entries
            .iter()
            .filter(|e| e.kind == EntryKind::Document)
            .count();
        self.received = Some((0, documents));
        for entry in entries {
            if entry.kind == EntryKind::Folder {
                self.writes
                    .push_back(WorkspaceWrite::Folder { path: entry.path });
            }
        }
    }

    /// The file path of the document `id` as the list places it.
    pub(super) fn document_path(&self, id: [u8; 16]) -> Option<Vec<String>> {
        let entry = self
            .list
            .entries()
            .into_iter()
            .find(|e| e.document == Some(id))?;
        let mut path = entry.path;
        let last = path.last_mut()?;
        last.push_str(".curvyo");
        Some(path)
    }

    /// The host wrote the join's file of `document`.
    pub(super) fn document_written(&mut self, document: [u8; 16]) {
        if self.writes_pending.remove(&document).is_some() {
            self.marker_changed = true;
        }
    }

    /// The background records not yet handed out; they stay counted until reported written.
    pub(super) fn take_background(&mut self) -> Vec<BackgroundItem> {
        let items: Vec<BackgroundItem> = self.background.drain(..).collect();
        for item in &items {
            self.handed
                .entry(item.document)
                .or_default()
                .push(item.clone());
        }
        items
    }

    /// The host wrote `document`'s file with every record up to `through`.
    pub(super) fn background_written(&mut self, document: [u8; 16], through: u64) {
        if let Some(items) = self.handed.get_mut(&document) {
            items.retain(|item| item.seq > through);
            if items.is_empty() {
                self.handed.remove(&document);
            }
            self.marker_changed = true;
        }
    }

    /// The open document's file was saved with every record applied up to `through`.
    pub(super) fn open_saved(&mut self, through: u64) {
        let before = self.unsaved.len();
        self.unsaved.retain(|item| item.seq > through);
        if self.unsaved.len() != before {
            self.marker_changed = true;
        }
    }

    /// The open document becomes `stream` (or none): records of the one left unsaved go back to
    /// the background queue; records of the new one not yet in its file are returned, to be
    /// applied to it.
    pub(super) fn switch_stream(&mut self, stream: Option<[u8; 16]>) -> Vec<BackgroundItem> {
        let left: Vec<BackgroundItem> = std::mem::take(&mut self.unsaved);
        let mut queue: Vec<BackgroundItem> = self.background.drain(..).chain(left).collect();
        self.stream = stream;
        let mut opened = Vec::new();
        if let Some(id) = stream {
            opened.extend(self.handed.remove(&id).unwrap_or_default());
            let (mine, rest): (Vec<_>, Vec<_>) = queue.into_iter().partition(|i| i.document == id);
            opened.extend(mine);
            queue = rest;
        }
        queue.sort_by_key(|item| item.seq);
        opened.sort_by_key(|item| item.seq);
        self.background = queue.into();
        self.unsaved.clone_from(&opened);
        opened
    }

    /// The highest sequence the marker may name: every record up to it is in a file (ADR 0037
    /// decision 8). `cursor` is the session's.
    pub(super) fn persisted_cursor(&self, cursor: u64) -> u64 {
        let waiting = self
            .unsaved
            .iter()
            .chain(&self.background)
            .chain(self.handed.values().flatten())
            .map(|item| item.seq)
            .chain(self.writes_pending.values().copied())
            .min();
        let cursor = waiting.map_or(cursor, |seq| cursor.min(seq.saturating_sub(1)));
        if self.joining { 0 } else { cursor }
    }

    /// Whether every record of a document that is not open up to `covers` is in its file, so
    /// the files can go into a snapshot covering it.
    pub(super) fn files_hold(&self, covers: u64) -> bool {
        self.background
            .iter()
            .chain(self.handed.values().flatten())
            .all(|item| item.seq > covers)
            && self.writes_pending.values().all(|seq| *seq > covers)
    }

    /// Where the document `id` keeps its file: at the list's place, as a kept copy when it is
    /// deleted, `None` when the list does not know it (yet).
    pub(super) fn document_place(&self, id: [u8; 16]) -> Option<DocumentPlace> {
        if let Some(path) = self.document_path(id) {
            return Some(DocumentPlace::File(path));
        }
        self.list
            .tombstoned_documents()
            .contains(&id)
            .then_some(DocumentPlace::Deleted)
    }

    /// Every document of the workspace but the open one, with its place: the listed ones and
    /// the deleted ones, whose state stays in every snapshot (criterion 40).
    pub(super) fn closed_documents(&self) -> Vec<([u8; 16], DocumentPlace)> {
        let mut out: Vec<([u8; 16], DocumentPlace)> = self
            .list
            .entries()
            .into_iter()
            .filter_map(|entry| {
                let id = entry.document?;
                let mut path = entry.path;
                path.last_mut()?.push_str(".curvyo");
                Some((id, DocumentPlace::File(path)))
            })
            .collect();
        let mut deleted: Vec<[u8; 16]> = self.list.tombstoned_documents().into_iter().collect();
        deleted.sort_unstable();
        out.extend(deleted.into_iter().map(|id| (id, DocumentPlace::Deleted)));
        out.retain(|(id, _)| Some(*id) != self.stream);
        out
    }

    /// "Receiving 7 of 20" while a join's documents arrive.
    pub(super) fn receiving(&self) -> Option<(usize, usize)> {
        let (done, total) = self.received?;
        let written = done.saturating_sub(self.writes_pending.len());
        (self.joining || written < total).then_some((written, total))
    }
}

#[cfg(test)]
#[path = "workspace_channel_tests.rs"]
mod tests;
