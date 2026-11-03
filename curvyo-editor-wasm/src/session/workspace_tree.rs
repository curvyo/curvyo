//! `Session`'s side of a shared workspace's live tree (`specs/0095-workspace` criteria 35 to 41,
//! ADR 0037 decision 6): the maker's changes as list edits, the plan that brings the folder to
//! the list, a new document's first records, the Deleted list and Download again.
//!
//! The host runs the file operations; the page does a local change on disk first and then
//! records it in the list, so a refused file operation never reaches the other members.

use std::collections::HashMap;

use curvyo_document_core::{
    Created, DeletedEntry, DiskOp, DiskState, DocumentVersion, EntryKind, ListEntry, ListError,
    ListNodeId, ScannedDocument, WorkspaceShareRecord, document_name, pack, plan_disk, unpack,
};
use curvyo_sync_core::{RecordKind, SessionError, encode_inner_record};

use super::Session;
use super::workspace_channel::DocumentPlace;

/// A change of the shared list the maker made in the File tab. Paths are file paths in the
/// folder (a document's last component ends in `.curvyo`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListEdit {
    /// A new document `document` at `path`.
    CreateDocument {
        /// Its file path.
        path: Vec<String>,
        /// Its fresh document id.
        document: [u8; 16],
    },
    /// A new folder.
    CreateFolder {
        /// Its path.
        path: Vec<String>,
    },
    /// A rename, a move or both.
    Move {
        /// The entry's path before.
        from: Vec<String>,
        /// Its path after.
        to: Vec<String>,
    },
    /// A shared deletion by this device's member at `at` (unix seconds).
    Delete {
        /// The entry's path.
        path: Vec<String>,
        /// When.
        at: i64,
    },
    /// Restore of a deleted entry from the Deleted list.
    Restore {
        /// Its node.
        node: ListNodeId,
    },
}

/// The folder as the host scanned it, with the document ids its files carry and where this
/// computer last saw each document (`index.json`).
#[derive(Debug, Clone, Default)]
pub struct DiskScan {
    /// Every folder.
    pub folders: Vec<Vec<String>>,
    /// Every document file and its id in this workspace.
    pub documents: Vec<ScannedDocument>,
    /// Document id to the file path last seen.
    pub known: HashMap<[u8; 16], Vec<String>>,
}

/// One file operation for the host, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiskStep {
    /// Create a folder.
    CreateFolder {
        /// Where.
        path: Vec<String>,
    },
    /// Move or rename a file.
    Move {
        /// From.
        from: Vec<String>,
        /// To.
        to: Vec<String>,
    },
    /// Write the document at its place: its kept copy when there is one (Restore), else the
    /// background writer makes it from its records.
    Write {
        /// Where.
        path: Vec<String>,
        /// Which document.
        document: [u8; 16],
    },
    /// Keep a copy of a deleted document whose folder goes to the trash next.
    Keep {
        /// Its file.
        path: Vec<String>,
        /// Which document.
        document: [u8; 16],
    },
    /// Keep a copy of a deleted document, then move its file to the OS trash; not done while
    /// it is open with unsaved changes (criterion 40).
    TrashDocument {
        /// Its file.
        path: Vec<String>,
        /// Which document.
        document: [u8; 16],
        /// Who deleted it ("" when unknown).
        by: String,
    },
    /// Move a folder to the OS trash.
    Trash {
        /// Its path.
        path: Vec<String>,
    },
}

/// What the plan answers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiskAnswer {
    /// The file operations, in order.
    pub steps: Vec<DiskStep>,
    /// File paths of documents the list names whose file another program removed: "Missing on
    /// this computer".
    pub missing: Vec<Vec<String>>,
    /// Files that are not part of the shared workspace: "Not in the shared workspace".
    pub outside: Vec<Vec<String>>,
    /// Places the list names that a file outside the workspace holds.
    pub blocked: Vec<Vec<String>>,
    /// Documents whose file lies at the list's place: the next `index.json`.
    pub settled: Vec<([u8; 16], Vec<String>)>,
}

impl Session {
    /// Records the maker's change in the shared list; it goes out with the next pump.
    ///
    /// # Errors
    /// [`ListError::NotFound`] when the session holds no shared workspace or the list does not
    /// name the entry or its folder; the list's own refusals.
    pub fn workspace_list_edit(&mut self, edit: ListEdit, stamp: Created) -> Result<(), ListError> {
        let link = self.sync.as_mut().ok_or(ListError::NotFound)?;
        let by = link.label.clone();
        let channel = link.workspace.as_mut().ok_or(ListError::NotFound)?;
        if channel.joining {
            return Err(ListError::NotFound);
        }
        let entries = channel.list.entries();
        let list = &channel.list;
        match edit {
            ListEdit::CreateDocument { path, document } => {
                let (parent, name) = place_of(&entries, &path)?;
                list.create_document(parent, &name, document, stamp)?;
            }
            ListEdit::CreateFolder { path } => {
                let (parent, name) = place_of(&entries, &path)?;
                list.create_folder(parent, &name, stamp)?;
            }
            ListEdit::Move { from, to } => {
                let entry = find(&entries, &from).ok_or(ListError::NotFound)?;
                let (parent, name) = place_of(&entries, &to)?;
                if entry.parent != parent {
                    list.move_to(entry.node, parent)?;
                }
                if entry.path.last() != Some(&name) {
                    list.rename(entry.node, &name)?;
                }
            }
            ListEdit::Delete { path, at } => {
                let entry = find(&entries, &path).ok_or(ListError::NotFound)?;
                list.delete(entry.node, &by, at)?;
            }
            ListEdit::Restore { node } => list.restore(node)?,
        }
        channel.marker_changed = true;
        self.export_workspace_list();
        self.bump_share_revision();
        self.sync_pump();
        Ok(())
    }

    /// The plan that brings the folder to the list; moves another program made are written to
    /// the list at once, with folders the list lacks stamped by `stamp`. `None` without a
    /// shared workspace, and while joining.
    pub fn workspace_disk_plan(
        &mut self,
        scan: &DiskScan,
        mut stamp: impl FnMut() -> Created,
    ) -> Option<DiskAnswer> {
        let channel = self.workspace_channel_mut()?;
        if channel.joining {
            return None;
        }
        let entries = channel.list.entries();
        let tombstoned = channel.list.tombstoned_documents();
        let plan = plan_disk(
            &entries,
            &tombstoned,
            &DiskState {
                folders: &scan.folders,
                documents: &scan.documents,
                known: &scan.known,
            },
        );
        let mut list_changed = false;
        for change in &plan.list_changes {
            list_changed |= channel
                .list
                .place(change.node, &change.path, &mut stamp)
                .is_ok();
        }
        let deleted = channel.list.deleted_entries();
        let mut answer = DiskAnswer {
            outside: plan.outside,
            blocked: plan.blocked,
            ..DiskAnswer::default()
        };
        for op in plan.ops {
            disk_step(op, scan, &tombstoned, &deleted, &mut answer.steps);
        }
        for node in plan.missing {
            if let Some(entry) = entries.iter().find(|e| e.node == node) {
                answer.missing.push(file_path(&entry.path));
            }
        }
        for entry in &entries {
            let Some(id) = entry.document else {
                continue;
            };
            let target = file_path(&entry.path);
            if scan
                .documents
                .iter()
                .any(|f| f.document == Some(id) && key(&f.path) == key(&target))
            {
                answer.settled.push((id, target));
            }
        }
        if list_changed {
            channel.marker_changed = true;
            self.export_workspace_list();
            self.bump_share_revision();
            self.sync_pump();
        }
        Some(answer)
    }

    /// A document joins the shared workspace as `document` (New, Duplicate, Add to workspace):
    /// the file's bytes with `share.json` v2, and its whole state pushed as the first records of
    /// its stream, so every member can build its file (criterion 35). Record it in the list
    /// first ([`Session::workspace_list_edit`]): the list goes out before the document.
    ///
    /// # Errors
    /// `Ended` without a shared workspace or for bytes that do not open.
    pub fn workspace_adopt_document(
        &mut self,
        document: [u8; 16],
        bytes: &[u8],
    ) -> Result<Vec<u8>, SessionError> {
        let workspace = self
            .workspace_channel()
            .ok_or(SessionError::Ended)?
            .workspace;
        let adopted = unpack(self.peer, bytes).map_err(|_| SessionError::Ended)?;
        adopted.set_share(None);
        adopted.set_workspace_share(Some(WorkspaceShareRecord {
            workspace,
            document,
            synced_version: adopted.version(),
        }));
        let updates = adopted
            .export_updates(&DocumentVersion::never())
            .map_err(|_| SessionError::Ended)?;
        let pictures: Vec<Vec<u8>> = crate::background_writer::image_assets(&adopted)
            .into_iter()
            .filter_map(|hash| adopted.asset(&hash).map(|bytes| bytes.to_vec()))
            .collect();
        let out = pack(&adopted, super::APP_VERSION).map_err(|_| SessionError::Ended)?;
        self.export_workspace_list();
        let peer = self.peer;
        let link = self.sync.as_mut().ok_or(SessionError::Ended)?;
        for picture in pictures {
            link.client.push(
                RecordKind::Asset,
                encode_inner_record(document, peer, &picture),
            )?;
        }
        link.client.push(
            RecordKind::Update,
            encode_inner_record(document, peer, &updates),
        )?;
        self.sync_pump();
        Ok(out)
    }

    /// Where the workspace document `document` keeps its file, `None` when the list does not
    /// know it (yet).
    #[must_use]
    pub fn workspace_document_place(&self, document: [u8; 16]) -> Option<DocumentPlace> {
        self.workspace_channel()?.document_place(document)
    }

    /// The Deleted list (criterion 40), most recent first.
    #[must_use]
    pub fn workspace_deleted(&self) -> Vec<DeletedEntry> {
        self.workspace_channel()
            .map(|c| c.list.deleted_entries())
            .unwrap_or_default()
    }

    /// The shared workspace's id, `None` without one.
    #[must_use]
    pub fn workspace_id(&self) -> Option<[u8; 16]> {
        self.workspace_channel().map(|c| c.workspace)
    }

    /// This device's label in the workspace ("by you" in the Deleted list).
    #[must_use]
    pub fn workspace_label(&self) -> Option<&str> {
        self.sync.as_ref().map(|s| s.label.as_str())
    }

    /// Download again (criterion 41): the room is read again from its snapshot, so a document
    /// missing on this computer is written anew from it.
    pub fn workspace_download_again(&mut self) {
        if let Some(link) = self.sync.as_mut()
            && link.workspace.is_some()
        {
            link.client.resync();
        }
        self.sync_pump();
    }
}

/// The steps of one planned operation: the trash of a deleted document keeps its copy first, a
/// folder's deleted documents are kept before the folder goes.
fn disk_step(
    op: DiskOp,
    scan: &DiskScan,
    tombstoned: &std::collections::HashSet<[u8; 16]>,
    deleted: &[DeletedEntry],
    steps: &mut Vec<DiskStep>,
) {
    match op {
        DiskOp::CreateFolder { path } => steps.push(DiskStep::CreateFolder { path }),
        DiskOp::Move { from, to } => steps.push(DiskStep::Move { from, to }),
        DiskOp::Write { path, document } => steps.push(DiskStep::Write { path, document }),
        DiskOp::Trash { path } => {
            let id_at = scan
                .documents
                .iter()
                .find(|f| key(&f.path) == key(&path))
                .and_then(|f| f.document);
            if let Some(document) = id_at.filter(|id| tombstoned.contains(id)) {
                let by = deleted
                    .iter()
                    .find(|e| e.document == Some(document))
                    .map(|e| e.by.clone())
                    .unwrap_or_default();
                steps.push(DiskStep::TrashDocument { path, document, by });
                return;
            }
            for file in &scan.documents {
                let below =
                    file.path.len() > path.len() && key(&file.path[..path.len()]) == key(&path);
                if let Some(document) = file.document.filter(|id| below && tombstoned.contains(id))
                {
                    steps.push(DiskStep::Keep {
                        path: file.path.clone(),
                        document,
                    });
                }
            }
            steps.push(DiskStep::Trash { path });
        }
    }
}

/// The folder node and the shown name an entry at the file path `path` takes.
fn place_of(
    entries: &[ListEntry],
    path: &[String],
) -> Result<(Option<ListNodeId>, String), ListError> {
    let (last, folders) = path.split_last().ok_or(ListError::NotFound)?;
    let parent = if folders.is_empty() {
        None
    } else {
        let folder = entries
            .iter()
            .find(|e| e.kind == EntryKind::Folder && key(&e.path) == key(folders))
            .ok_or(ListError::NotFound)?;
        Some(folder.node)
    };
    Ok((parent, document_name(last).to_owned()))
}

/// The entry at the file path `path`.
fn find<'a>(entries: &'a [ListEntry], path: &[String]) -> Option<&'a ListEntry> {
    let (last, folders) = path.split_last()?;
    entries.iter().find(|e| {
        let Some((name, above)) = e.path.split_last() else {
            return false;
        };
        let file_name = match e.kind {
            EntryKind::Document => format!("{name}.curvyo"),
            EntryKind::Folder => name.clone(),
        };
        key(above) == key(folders) && file_name.to_lowercase() == last.to_lowercase()
    })
}

/// The file path of a listed document: its shown path with `.curvyo`.
fn file_path(path: &[String]) -> Vec<String> {
    let mut file = path.to_vec();
    if let Some(last) = file.last_mut() {
        last.push_str(".curvyo");
    }
    file
}

/// Paths compare like names: equal under `to_lowercase` (ADR 0037 decision 6).
fn key(path: &[String]) -> String {
    path.join("/").to_lowercase()
}
