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
    /// It is shared on its own with people who are not members of the workspace (criterion 49).
    SharedOnItsOwn,
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
    /// The workspace's members at Share: this device alone.
    members: Vec<curvyo_crypto_core::Identity>,
    /// Documents shared on their own that were taken in, with their new id (criterion 42).
    taken_in: Vec<(Vec<String>, [u8; 16])>,
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
            members: Vec::new(),
            taken_in: Vec::new(),
        }
    }

    /// The workspace's first member, this device: a document shared on its own whose
    /// participants are all members is taken in (criterion 42, ADR 0037 decision 10).
    pub fn set_member(&mut self, member: curvyo_crypto_core::Identity) {
        self.members = vec![member];
    }

    /// The documents shared on their own that were taken in: their path and new document id;
    /// each needs its `Moved` record once the workspace is shared.
    #[must_use]
    pub fn taken_in(&self) -> &[(Vec<String>, [u8; 16])] {
        &self.taken_in
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
        let taken_in = opened.share().is_some();
        if taken_in {
            match crate::session::take_in_check(bytes, &self.members) {
                crate::session::TakeIn::Allowed { .. } => opened.set_share(None),
                _ => return Err(NotAdded::SharedOnItsOwn),
            }
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
        if taken_in {
            self.taken_in.push((path.to_vec(), document));
        }
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

/// Whether two paths name the same entry under the list's naming rule (case aside).
fn same_path(a: &[String], b: &[String]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| same_entry_name(x, y))
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{Document, EntryKind, ShareRecord};
    use curvyo_sync_core::SnapshotAssembler;

    use super::*;

    fn p(text: &str) -> Vec<String> {
        text.split('/').map(str::to_owned).collect()
    }

    fn file() -> Vec<u8> {
        pack(&Document::new(3), "test").unwrap()
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
        let shared = Document::new(4);
        shared.set_share(Some(ShareRecord {
            server: "ws://a".to_owned(),
            room: [1; 16],
            server_seq: 0,
            synced_version: shared.version(),
            keyring_log: vec![1],
        }));
        let own = pack(&shared, "test").unwrap();
        assert_eq!(
            builder.add_document(&p("Base.curvyo"), &own, [0x44; 16]),
            Err(NotAdded::SharedOnItsOwn)
        );
        assert_eq!(builder.document_count(), 2);
        assert_eq!(builder.not_added().len(), 2);
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
}
