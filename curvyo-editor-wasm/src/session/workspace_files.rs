//! `Session`'s files of a shared workspace (`specs/0094-workspace` criteria 28, 32 and 33; ADR
//! 0036 decisions 1, 7 and 8): what it hands the host to write (a join's files, the background
//! records, the marker) and the workspace snapshot the host builds from the files.

use curvyo_crypto_core::keyring::encode_log;
use curvyo_document_core::WorkspaceRoomRecord;
use curvyo_sync_core::{SessionError, StreamEntry, encode_snapshot_stream, split_snapshot};

use super::Session;
use super::workspace_channel::{BackgroundItem, DocumentPlace, WorkspaceChannel, WorkspaceWrite};

/// The marker's files of a shared workspace (ADR 0036 decision 1).
pub struct MarkerFiles {
    /// `share.json`.
    pub share_json: Vec<u8>,
    /// `keyring.log`.
    pub keyring_log: Vec<u8>,
    /// `list.loro`.
    pub list: Vec<u8>,
}

impl Session {
    /// The room asked for a snapshot: Share's prepared one goes out at once, any later one is
    /// built by the host from the files (decision 8).
    pub(super) fn workspace_snapshot_asked(&mut self, covers_seq: u64) {
        let Some(link) = self.sync.as_mut() else {
            return;
        };
        let Some(channel) = link.workspace.as_mut() else {
            return;
        };
        if let Some(parts) = channel.prepared.take() {
            let _ = link.client.upload_snapshot_parts(&parts, covers_seq);
        } else {
            channel.snapshot_due = Some(covers_seq);
        }
    }

    /// The sequence a snapshot the host has to build covers, once every record up to it is in
    /// the files; `None` while none is due or the background writer is behind.
    #[must_use]
    pub fn workspace_snapshot_ready(&self) -> Option<u64> {
        let channel = self.workspace_channel()?;
        let covers = channel.snapshot_due?;
        channel.files_hold(covers).then_some(covers)
    }

    /// The documents whose files go into the snapshot: every one but the open one, with its
    /// place (a deleted document's kept copy too, criterion 40).
    #[must_use]
    pub fn workspace_snapshot_documents(&self) -> Vec<([u8; 16], DocumentPlace)> {
        self.workspace_channel()
            .map(WorkspaceChannel::closed_documents)
            .unwrap_or_default()
    }

    /// One document's file bytes for the snapshot being built.
    pub fn add_workspace_snapshot_document(&mut self, document: [u8; 16], bytes: Vec<u8>) {
        if let Some(channel) = self.workspace_channel_mut() {
            channel.snapshot_documents.push(StreamEntry {
                stream: document,
                bytes,
            });
        }
    }

    /// Uploads the snapshot that is due with the documents added (decision 8).
    ///
    /// # Errors
    /// As [`Session::upload_workspace_snapshot`]; `Ended` when none is due.
    pub fn finish_workspace_snapshot(&mut self, snapshot_id: [u8; 16]) -> Result<(), SessionError> {
        let channel = self.workspace_channel_mut().ok_or(SessionError::Ended)?;
        let covers = channel.snapshot_due.ok_or(SessionError::Ended)?;
        let documents = std::mem::take(&mut channel.snapshot_documents);
        self.upload_workspace_snapshot(covers, documents, snapshot_id)
    }

    /// The file path of the workspace document `document`, as the list places it.
    #[must_use]
    pub fn workspace_document_path(&self, document: [u8; 16]) -> Option<Vec<String>> {
        self.workspace_channel()?.document_path(document)
    }

    /// Uploads a workspace snapshot covering `covers_seq`: the list and the open document from
    /// this session, every other document's file bytes from the host (`documents`). The host
    /// wrote every record up to `covers_seq` into those files first.
    ///
    /// # Errors
    /// `NoKey`, `TooLarge`, `Ended` as [`curvyo_sync_core::ClientSession::upload_snapshot_parts`].
    pub fn upload_workspace_snapshot(
        &mut self,
        covers_seq: u64,
        documents: Vec<StreamEntry>,
        snapshot_id: [u8; 16],
    ) -> Result<(), SessionError> {
        let open = self.workspace_channel().and_then(|c| c.stream);
        let open_bytes = match open {
            Some(_) => Some(
                curvyo_document_core::pack(&self.document, super::APP_VERSION)
                    .map_err(|_| SessionError::Ended)?,
            ),
            None => None,
        };
        let link = self.sync.as_mut().ok_or(SessionError::Ended)?;
        let channel = link.workspace.as_mut().ok_or(SessionError::Ended)?;
        let mut entries = vec![StreamEntry {
            stream: channel.workspace,
            bytes: channel.list.snapshot().map_err(|_| SessionError::Ended)?,
        }];
        if let (Some(stream), Some(bytes)) = (open, open_bytes) {
            entries.push(StreamEntry { stream, bytes });
        }
        entries.extend(documents.into_iter().filter(|e| Some(e.stream) != open));
        let parts = split_snapshot(&encode_snapshot_stream(&entries), snapshot_id);
        link.client.upload_snapshot_parts(&parts, covers_seq)?;
        channel.snapshot_due = None;
        self.sync_pump();
        Ok(())
    }

    /// The files of a join to write, folders first, in order.
    pub fn take_workspace_writes(&mut self) -> Vec<WorkspaceWrite> {
        self.workspace_channel_mut()
            .map(|c| c.writes.drain(..).collect())
            .unwrap_or_default()
    }

    /// The next file of a join to write, `None` when there is none.
    pub fn next_workspace_write(&mut self) -> Option<WorkspaceWrite> {
        self.workspace_channel_mut()?.writes.pop_front()
    }

    /// The host wrote the joined document `document`'s file.
    pub fn workspace_document_written(&mut self, document: [u8; 16]) {
        if let Some(channel) = self.workspace_channel_mut() {
            channel.document_written(document);
        }
        self.bump_share_revision();
    }

    /// The records of documents that are not open, for the background writer; they keep the
    /// marker cursor back until [`Session::background_written`].
    pub fn take_background(&mut self) -> Vec<BackgroundItem> {
        self.workspace_channel_mut()
            .map(WorkspaceChannel::take_background)
            .unwrap_or_default()
    }

    /// The next record for the background writer; it keeps the marker cursor back until
    /// [`Session::background_written`].
    pub fn take_background_one(&mut self) -> Option<BackgroundItem> {
        let channel = self.workspace_channel_mut()?;
        let item = channel.background.pop_front()?;
        channel
            .handed
            .entry(item.document)
            .or_default()
            .push(item.clone());
        Some(item)
    }

    /// Whether the host may write `document`'s file in the background: never the open one.
    #[must_use]
    pub fn workspace_may_write(&self, document: [u8; 16]) -> bool {
        self.workspace_channel()
            .is_some_and(|c| c.stream != Some(document))
    }

    /// The background writer wrote `document`'s file with every record up to `through`.
    pub fn background_written(&mut self, document: [u8; 16], through: u64) {
        if let Some(channel) = self.workspace_channel_mut() {
            channel.background_written(document, through);
        }
    }

    /// The marker's files when they changed since the last call (the host writes them).
    pub fn take_workspace_marker(&mut self) -> Option<MarkerFiles> {
        let link = self.sync.as_mut()?;
        if link.ended.is_some() {
            return None;
        }
        let cursor = link.client.cursor();
        let channel = link.workspace.as_mut()?;
        if channel.joining {
            return None;
        }
        let persisted = channel.persisted_cursor(cursor);
        if !channel.marker_changed && persisted == channel.marker_cursor {
            return None;
        }
        let record = WorkspaceRoomRecord {
            server: link.server.clone(),
            workspace: channel.workspace,
            server_seq: persisted,
            synced_version: channel.list_synced.clone(),
        };
        let files = MarkerFiles {
            share_json: record.to_bytes().ok()?,
            keyring_log: encode_log(&link.client.keyring_entries()).ok()?,
            list: channel.list.snapshot().ok()?,
        };
        channel.marker_changed = false;
        channel.marker_cursor = persisted;
        Some(files)
    }
}
