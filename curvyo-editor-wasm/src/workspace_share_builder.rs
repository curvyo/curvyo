//! Share workspace's preparation (`specs/0095-workspace` criterion 25, ADR 0037 decision 9): the
//! scanned documents get a fresh document id and `share.json` version 2, the list is built from
//! the folders, and the first workspace snapshot is cut into parts, all in memory, before the
//! room exists. The host writes the files only once the relay took the snapshot.

use std::collections::BTreeMap;

use curvyo_document_core::{
    Created, ListNodeId, WorkspaceList, WorkspaceShareRecord, document_name, pack, same_entry_name,
    unpack,
};
use curvyo_sync_core::{StreamEntry, encode_snapshot_stream, split_snapshot};

use crate::session::{APP_VERSION, PreparedShare};

/// Why a scanned document stays out of the shared workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotAdded {
    /// Its editable curves or width profile are not supported by older collaboration peers.
    UnsupportedDocument,
    /// It cannot be read.
    Unreadable,
    /// Its name or a folder's breaks a list naming rule, or differs from the name of a
    /// document or folder added before only in case.
    Name,
}

/// The files Share writes once the relay took the snapshot: path and bytes.
pub type ShareFiles = Vec<(Vec<String>, Vec<u8>)>;

/// The workspace being prepared for sharing.
pub struct WorkspaceShareBuilder {
    workspace: [u8; 16],
    peer: u64,
    list: WorkspaceList,
    at: i64,
    seed: [u8; 16],
    counter: u64,
    folders: BTreeMap<Vec<String>, ListNodeId>,
    documents: Vec<StreamEntry>,
    files: ShareFiles,
    not_added: Vec<(Vec<String>, NotAdded)>,
}

impl WorkspaceShareBuilder {
    /// An empty preparation of the workspace `workspace` (the fresh id Share writes, ADR 0037
    /// decision 4) at `at` (unix seconds); `peer` is the list's Loro peer and `seed` 16 random
    /// bytes from which the creation stamps are drawn.
    #[must_use]
    pub fn new(workspace: [u8; 16], peer: u64, at: i64, seed: [u8; 16]) -> Self {
        Self {
            workspace,
            peer,
            list: WorkspaceList::new(peer),
            at,
            seed,
            counter: 0,
            folders: BTreeMap::new(),
            documents: Vec::new(),
            files: Vec::new(),
            not_added: Vec::new(),
        }
    }

    fn created(&mut self) -> Created {
        self.counter += 1;
        let mut id = self.seed;
        for (byte, add) in id[8..].iter_mut().zip(self.counter.to_le_bytes()) {
            *byte ^= add;
        }
        Created { at: self.at, id }
    }

    /// The folder `path` in the list (`None` for the top), made with its parents when absent.
    fn folder(&mut self, path: &[String]) -> Result<Option<ListNodeId>, NotAdded> {
        let Some((name, above)) = path.split_last() else {
            return Ok(None);
        };
        if let Some(node) = self.folders.get(path) {
            return Ok(Some(*node));
        }
        // A folder whose name differs from an added one's only in case (a case-sensitive file
        // system) would be listed under another name than its folder on disk.
        if self.folders.keys().any(|added| same_path(added, path)) {
            return Err(NotAdded::Name);
        }
        let parent = self.folder(above)?;
        let created = self.created();
        let node = self
            .list
            .create_folder(parent, name, created)
            .map_err(|_| NotAdded::Name)?;
        self.folders.insert(path.to_vec(), node);
        Ok(Some(node))
    }

    /// Adds a scanned folder, so that an empty one is shared too. `false` when its name breaks a
    /// naming rule.
    pub fn add_folder(&mut self, path: &[String]) -> bool {
        self.folder(path).is_ok()
    }

    /// Adds the scanned document at `path` with its file's `bytes` under the fresh document id
    /// `document`.
    ///
    /// # Errors
    /// The reason it stays out; nothing of it is kept.
    pub fn add_document(
        &mut self,
        path: &[String],
        bytes: &[u8],
        document: [u8; 16],
    ) -> Result<(), NotAdded> {
        let result = self.try_add(path, bytes, document);
        if let Err(reason) = result {
            self.not_added.push((path.to_vec(), reason));
        }
        result
    }

    fn try_add(
        &mut self,
        path: &[String],
        bytes: &[u8],
        document: [u8; 16],
    ) -> Result<(), NotAdded> {
        let (file, folder) = path.split_last().ok_or(NotAdded::Name)?;
        let opened = unpack(self.peer, bytes).map_err(|_| NotAdded::Unreadable)?;
        if opened.has_extended_path_history() {
            return Err(NotAdded::UnsupportedDocument);
        }
        let parent = self.folder(folder)?;
        if self.files.iter().any(|(added, _)| same_path(added, path)) {
            return Err(NotAdded::Name);
        }
        let created = self.created();
        self.list
            .create_document(parent, document_name(file), document, created)
            .map_err(|_| NotAdded::Name)?;
        opened.set_workspace_share(Some(WorkspaceShareRecord {
            workspace: self.workspace,
            document,
            synced_version: opened.version(),
        }));
        let bytes = pack(&opened, APP_VERSION).map_err(|_| NotAdded::Unreadable)?;
        self.documents.push(StreamEntry {
            stream: document,
            bytes: bytes.clone(),
        });
        self.files.push((path.to_vec(), bytes));
        Ok(())
    }

    /// The documents added so far.
    #[must_use]
    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    /// The documents left out, with the reason.
    #[must_use]
    pub fn not_added(&self) -> &[(Vec<String>, NotAdded)] {
        &self.not_added
    }

    /// The prepared share (the list first in the snapshot stream, then every document) and the
    /// files to write once the relay took it. `snapshot_id` is 16 random bytes.
    #[must_use]
    pub fn finish(self, snapshot_id: [u8; 16]) -> (PreparedShare, ShareFiles) {
        let mut entries = Vec::with_capacity(self.documents.len() + 1);
        entries.push(StreamEntry {
            stream: self.workspace,
            bytes: self.list.snapshot().unwrap_or_default(),
        });
        entries.extend(self.documents);
        let parts = split_snapshot(&encode_snapshot_stream(&entries), snapshot_id);
        (
            PreparedShare {
                workspace: self.workspace,
                list: self.list,
                parts,
            },
            self.files,
        )
    }
}

/// The document file `current` (as it is on disk now) given the workspace record of `prepared`,
/// the file Share built from an earlier read (`specs/0105`, a Save during the upload): nothing of
/// `current` is lost, and the record keeps the prepared synced version, so what changed since goes
/// out as edits once the document is open. `None` when either does not open or `prepared` holds
/// no workspace record.
#[must_use]
pub fn restamp_workspace_record(current: &[u8], prepared: &[u8]) -> Option<Vec<u8>> {
    let record = curvyo_document_core::container_share_members(prepared)
        .share_json
        .and_then(|json| WorkspaceShareRecord::from_share_json(&json))?;
    let document = unpack(1, current).ok()?;
    document.set_workspace_share(Some(record));
    pack(&document, APP_VERSION).ok()
}

/// Whether two paths name the same entry under the list's naming rule (case aside).
fn same_path(a: &[String], b: &[String]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| same_entry_name(x, y))
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{Document, EntryKind};
    use curvyo_sync_core::SnapshotAssembler;

    use super::*;

    fn p(text: &str) -> Vec<String> {
        text.split('/').map(str::to_owned).collect()
    }

    fn file() -> Vec<u8> {
        pack(&Document::new(3), "test").unwrap()
    }

    #[test]
    fn profile_document_is_not_added_to_shared_snapshot() {
        use curvyo_document_core::{AnchorId, NewAnchor, Point};
        let document = Document::new(3);
        let path = document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(3, 1), Point::new(0.0, 0.0)),
                NewAnchor::corner(AnchorId::new(3, 2), Point::new(10.0, 0.0)),
            ],
            false,
        );
        document
            .set_width_point(path, AnchorId::new(3, 3), 0.5, 2.0)
            .unwrap();
        let mut builder = WorkspaceShareBuilder::new([0x57; 16], 9, 1, [3; 16]);
        assert_eq!(
            builder.add_document(
                &p("profile.curvyo"),
                &pack(&document, "test").unwrap(),
                [0x11; 16]
            ),
            Err(NotAdded::UnsupportedDocument)
        );
        assert_eq!(builder.document_count(), 0);
    }

    /// Criterion 25: every readable document gets a fresh id and `share.json` v2, the list
    /// holds the tree, and the snapshot holds the list first and every document.
    #[test]
    fn share_prepares_the_list_the_files_and_the_snapshot() {
        let mut builder = WorkspaceShareBuilder::new([0x57; 16], 9, 1_800_000_000, [3; 16]);
        builder.add_folder(&p("Old"));
        builder
            .add_document(&p("Sides/Left.curvyo"), &file(), [0x11; 16])
            .unwrap();
        builder
            .add_document(&p("Lid.curvyo"), &file(), [0x22; 16])
            .unwrap();
        assert_eq!(
            builder.add_document(&p("Junk.curvyo"), b"junk", [0x33; 16]),
            Err(NotAdded::Unreadable)
        );
        assert_eq!(builder.document_count(), 2);
        assert_eq!(builder.not_added().len(), 1);
        let (share, files) = builder.finish([9; 16]);
        let paths: Vec<Vec<String>> = share.list.entries().into_iter().map(|e| e.path).collect();
        assert!(paths.contains(&p("Old")));
        assert!(paths.contains(&p("Sides/Left")));
        assert!(paths.contains(&p("Lid")));
        assert_eq!(
            share
                .list
                .entries()
                .iter()
                .filter(|e| e.kind == EntryKind::Document)
                .count(),
            2
        );
        assert_eq!(files.len(), 2);
        let record = unpack(5, &files[0].1).unwrap().workspace_share().unwrap();
        assert_eq!(record.workspace, [0x57; 16]);
        assert_eq!(record.document, [0x11; 16]);
        let mut assembler = SnapshotAssembler::default();
        let mut streams = Vec::new();
        for part in &share.parts {
            streams.extend(assembler.push_part(part).unwrap());
        }
        let ids: Vec<[u8; 16]> = streams.iter().map(|s| s.stream).collect();
        assert_eq!(ids, vec![[0x57; 16], [0x11; 16], [0x22; 16]]);
    }

    /// A Save between the read and the commit: the newer file keeps its content and gets the
    /// record with the prepared synced version.
    #[test]
    fn a_newer_file_takes_the_prepared_record() {
        let original = Document::new(3);
        let read = pack(&original, "test").unwrap();
        let mut builder = WorkspaceShareBuilder::new([0x57; 16], 9, 1_800_000_000, [3; 16]);
        builder
            .add_document(&p("Lid.curvyo"), &read, [0x11; 16])
            .unwrap();
        let (_, files) = builder.finish([9; 16]);
        let saved = unpack(4, &read).unwrap();
        let _ = saved.create_rect(curvyo_document_core::RectBounds {
            origin: curvyo_document_core::Point::new(0.0, 0.0),
            width: curvyo_document_core::Length::from_mm(5.0),
            height: curvyo_document_core::Length::from_mm(5.0),
        });
        let current = pack(&saved, "test").unwrap();
        let stamped = unpack(5, &restamp_workspace_record(&current, &files[0].1).unwrap()).unwrap();
        assert_eq!(stamped.read_objects().objects.len(), 1, "the save is kept");
        let record = stamped.workspace_share().unwrap();
        assert_eq!(record.document, [0x11; 16]);
        assert_eq!(record.synced_version, original.version());
        assert_eq!(
            restamp_workspace_record(&current, &read),
            None,
            "no record to give"
        );
    }
}
