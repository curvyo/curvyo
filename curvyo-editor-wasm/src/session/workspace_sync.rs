//! `Session`'s link to a shared workspace (`specs/0095-workspace` criteria 25 to 33, ADR 0037
//! decisions 5, 7 to 9): sharing, joining and resuming the workspace room, and the link that
//! moves from one open document's session to the next (its files: `workspace_files`).
//!
//! One `ClientSession` serves the whole workspace; it rides in the open session's `SyncLink`
//! with a [`WorkspaceChannel`]. Records of the open document are applied as in `0092`; the
//! others wait for the host's background writer.

use std::collections::BTreeSet;

use curvyo_crypto_core::keyring::decode_log;
use curvyo_crypto_core::{EpochKey, IdentityKeys};
use curvyo_document_core::{
    DocumentVersion, WorkspaceList, WorkspaceRoomRecord, WorkspaceShareRecord,
};
use curvyo_sync_core::{
    ClientSession, InviteKind, InviteLink, Millis, RecordKind, RoomId, SessionConfig, SessionError,
    SessionStart, encode_inner_record,
};
use curvyo_ui_core::share_text::JoinRefusal;

use super::Session;
use super::sync::{JoinPhase, SyncLink, SyncSeeds};
pub use super::workspace_channel::{BackgroundItem, BackgroundKind, DocumentPlace, WorkspaceWrite};
use super::workspace_channel::{Routed, WorkspaceChannel};
pub use super::workspace_files::MarkerFiles;

/// The link of a shared workspace while it moves from one open document's session to the next.
pub struct WorkspaceLink(Box<SyncLink>);

/// What Share workspace prepared before the room exists (`WorkspaceShareBuilder`): the
/// workspace id, the list and the first snapshot's parts.
pub struct PreparedShare {
    /// The fresh workspace id, the room.
    pub workspace: [u8; 16],
    /// The list built from the scan.
    pub list: WorkspaceList,
    /// The plaintexts of the first snapshot's parts.
    pub parts: Vec<Vec<u8>>,
}

impl Session {
    /// Shares the workspace on `server` (criterion 25): the session writes the genesis,
    /// connects, and uploads the prepared snapshot once the relay has admitted it.
    ///
    /// # Errors
    /// `Crypto` if the genesis cannot be made.
    pub fn share_workspace(
        &mut self,
        server: &str,
        label: &str,
        name: &str,
        share: PreparedShare,
        seeds: &SyncSeeds,
        now: Millis,
    ) -> Result<(), SessionError> {
        let config = workspace_config(RoomId(share.workspace), 0, SessionStart::Share, seeds);
        let mut client = ClientSession::new(config, now)?;
        client.use_workspace_snapshots();
        let mut channel = WorkspaceChannel::new(share.workspace, name, share.list, self.peer);
        channel.prepared = Some(share.parts);
        self.start_workspace_link(client, channel, server, label, now);
        Ok(())
    }

    /// Joins the workspace of a `2.` invite link (criterion 27): the workspace's documents
    /// arrive as file writes; this session's own document is left as it is.
    ///
    /// # Errors
    /// `Invalid` for a text that is not a workspace invite link.
    pub fn join_workspace(
        &mut self,
        link: &str,
        label: &str,
        name: &str,
        seeds: &SyncSeeds,
        now: Millis,
    ) -> Result<(), JoinRefusal> {
        let link = InviteLink::parse(link).map_err(|_| JoinRefusal::Invalid)?;
        if link.kind != InviteKind::Workspace {
            return Err(JoinRefusal::Invalid);
        }
        let server =
            curvyo_sync_core::normalize_server_address(&link.server).ok_or(JoinRefusal::Invalid)?;
        let workspace = link.secret.room.0;
        let config = workspace_config(link.secret.room, 0, SessionStart::Join(link.secret), seeds);
        let mut client = ClientSession::new(config, now).map_err(|_| JoinRefusal::Invalid)?;
        client.use_workspace_snapshots();
        let mut channel =
            WorkspaceChannel::new(workspace, name, WorkspaceList::new(self.peer), self.peer);
        channel.joining = true;
        self.start_workspace_link(client, channel, &server, label, now);
        if let Some(link) = self.sync.as_mut() {
            link.join = Some(JoinPhase::Connecting);
        }
        Ok(())
    }

    /// Reconnects a shared workspace opened from its folder (criterion 33): the marker's
    /// `share.json`, `keyring.log` and `list.loro`, and the epoch keys from the credential
    /// store. `false` when the marker's files cannot be read.
    #[allow(clippy::too_many_arguments)] // The marker's three files, the keys and the seeds.
    pub fn resume_workspace(
        &mut self,
        share_json: &[u8],
        keyring_log: &[u8],
        list: &[u8],
        name: &str,
        label: &str,
        epoch_keys: Vec<(u32, [u8; 32])>,
        seeds: &SyncSeeds,
        now: Millis,
    ) -> bool {
        let (Ok(record), Ok(keyring)) = (
            WorkspaceRoomRecord::from_bytes(share_json),
            decode_log(keyring_log),
        ) else {
            return false;
        };
        let Ok(list) = WorkspaceList::from_snapshot(self.peer, list) else {
            return false;
        };
        let start = SessionStart::Member {
            keyring,
            epoch_keys: epoch_keys
                .into_iter()
                .map(|(n, k)| (n, EpochKey::from_bytes(k)))
                .collect(),
        };
        let config = workspace_config(RoomId(record.workspace), record.server_seq, start, seeds);
        let Ok(mut client) = ClientSession::new(config, now) else {
            return false;
        };
        client.use_workspace_snapshots();
        let mut channel = WorkspaceChannel::new(record.workspace, name, list, self.peer);
        // Changes of the list after the acknowledged version go out on connect.
        channel.list_exported = record.synced_version.clone();
        channel.list_synced = record.synced_version;
        channel.marker_cursor = record.server_seq;
        self.start_workspace_link(client, channel, &record.server, label, now);
        if let Some(link) = self.sync.as_mut() {
            // Keys came from the store: nothing new to write until the keyring changes.
            link.keys_changed = false;
        }
        true
    }

    pub(super) fn start_workspace_link(
        &mut self,
        client: ClientSession,
        channel: WorkspaceChannel,
        server: &str,
        label: &str,
        now: Millis,
    ) {
        let mut link = SyncLink::new(client, server.to_owned(), label.to_owned(), now);
        let mut channel = channel;
        label.clone_into(&mut channel.me);
        link.workspace = Some(Box::new(channel));
        self.install_link(Box::new(link));
        self.bind_workspace_stream();
        self.sync_pump();
    }

    /// Whether this session carries a shared workspace's link that has not ended.
    #[must_use]
    pub fn has_workspace_link(&self) -> bool {
        self.sync
            .as_ref()
            .is_some_and(|s| s.workspace.is_some() && s.ended.is_none())
    }

    /// Whether this device was removed from its shared workspace or left it (criteria 44,
    /// 45): the link keeps its ended record ([`Session::workspace_ended_record`]) and the
    /// workspace is a local one.
    #[must_use]
    pub fn workspace_removed(&self) -> bool {
        self.sync.as_ref().is_some_and(|l| {
            l.workspace.is_some()
                && matches!(
                    l.ended,
                    Some(super::sync::SyncEnd::Removed | super::sync::SyncEnd::Left)
                )
        })
    }

    /// Whether the session is connected and caught up.
    #[must_use]
    pub fn sync_online(&self) -> bool {
        self.sync.as_ref().is_some_and(|l| l.online)
    }

    /// Takes the workspace's link out of this session (the maker switches documents); `None`
    /// when it carries none or the link has ended (left, removed, refused). Records applied to
    /// this document but not in its file go back to the background queue.
    pub fn take_workspace_link(&mut self) -> Option<WorkspaceLink> {
        if !self.has_workspace_link() {
            return None;
        }
        self.sync_export();
        let mut link = self.sync.take()?;
        if let Some(channel) = link.workspace.as_mut() {
            channel.switch_stream(None);
        }
        link.remote.clear();
        link.pushed.clear();
        link.packed_through = None;
        Some(WorkspaceLink(link))
    }

    /// Gives this session the workspace's link: when its document belongs to the workspace it
    /// becomes the open stream, and its records not yet in its file are applied.
    pub fn attach_workspace_link(&mut self, link: WorkspaceLink) {
        self.install_link(link.0);
        self.bind_workspace_stream();
        self.sync_pump();
    }

    /// Share workspace's commit for the open document (`specs/0105` criterion 20): `file`, the
    /// container Share wrote for it, gives the open document its workspace record in place, and
    /// the document becomes the link's stream; the session, its view and its tab stay. `false`
    /// when `file` holds no workspace record.
    pub fn adopt_workspace_document(&mut self, file: &[u8]) -> bool {
        let Some(record) = curvyo_document_core::container_share_members(file)
            .share_json
            .and_then(|json| WorkspaceShareRecord::from_share_json(&json))
        else {
            return false;
        };
        self.document.set_workspace_share(Some(record));
        self.bind_workspace_stream();
        self.sync_pump();
        true
    }

    /// Makes the open document the link's stream when it is a document of the workspace.
    fn bind_workspace_stream(&mut self) {
        let record = self.document.workspace_share();
        let assets = self.referenced_assets();
        let asset_total = self.document.asset_bytes();
        let version = self.document.version();
        let Some(link) = self.sync.as_mut() else {
            return;
        };
        let Some(channel) = link.workspace.as_mut() else {
            return;
        };
        let stream = record
            .as_ref()
            .filter(|r| r.workspace == channel.workspace)
            .map(|r| r.document);
        let waiting = channel.switch_stream(stream);
        let synced = record.map_or_else(DocumentVersion::never, |r| r.synced_version);
        // Pictures placed since the acknowledged version (offline, before a reopen) go out with
        // the edits: the file does not say which the room holds, so every referenced picture is
        // sent again; the room's copies are idempotent (ADR 0031 decision 9, Proposal).
        let unsynced = version != synced;
        link.exported = synced.clone();
        link.synced = synced;
        link.assets = if unsynced { BTreeSet::new() } else { assets };
        link.asset_total = if unsynced { 0 } else { asset_total };
        for item in waiting {
            let kind = match item.kind {
                BackgroundKind::Update => RecordKind::Update,
                BackgroundKind::Asset => RecordKind::Asset,
                BackgroundKind::Container => RecordKind::Snapshot,
            };
            link.remote.push_back((item.seq, kind, item.bytes));
        }
        link.revision = link.revision.wrapping_add(1);
    }

    /// One opened record of the workspace room: the open document's go to the frame's queue.
    pub(super) fn route_workspace_record(&mut self, seq: u64, kind: RecordKind, bytes: &[u8]) {
        let Some(link) = self.sync.as_mut() else {
            return;
        };
        let Some(channel) = link.workspace.as_mut() else {
            return;
        };
        for presence in link.peers.values().filter_map(|p| p.presence.as_ref()) {
            if let Some(peer) = presence.list_peer_id() {
                channel.authors.insert(peer, presence.label.clone());
            }
        }
        let routed = channel.route(seq, kind, bytes);
        let notices = std::mem::take(&mut channel.notices);
        let joined = !channel.joining;
        if joined && link.join == Some(JoinPhase::Downloading) {
            link.join = Some(JoinPhase::Done);
        }
        for notice in notices {
            link.notice(notice, false);
        }
        match routed {
            Routed::Open(kind, payload) => link.remote.push_back((seq, kind, payload)),
            Routed::Done(true) => {}
            Routed::Done(false) => {
                link.client.record_rejected(seq);
            }
        }
        link.revision = link.revision.wrapping_add(1);
    }

    /// The open document's and the list's changes as inner records (decision 5).
    pub(super) fn workspace_payload(&self, payload: &[u8]) -> Option<Vec<u8>> {
        let link = self.sync.as_ref()?;
        match link.workspace.as_ref() {
            None => Some(payload.to_vec()),
            Some(channel) => channel
                .stream
                .map(|stream| encode_inner_record(stream, self.peer, payload)),
        }
    }

    /// Pushes the list's local changes since the last export.
    pub(super) fn export_workspace_list(&mut self) {
        let Some(link) = self.sync.as_mut() else {
            return;
        };
        let Some(channel) = link.workspace.as_mut() else {
            return;
        };
        if channel.joining {
            return;
        }
        let version = channel.list.version();
        if version == channel.list_exported {
            return;
        }
        let Ok(bytes) = channel.list.export_updates(&channel.list_exported) else {
            return;
        };
        let record = encode_inner_record(channel.workspace, channel.list_peer, &bytes);
        if let Ok(client_id) = link.client.push(RecordKind::Update, record) {
            channel.list_pushed.insert(client_id, version.clone());
            channel.list_exported = version;
            channel.marker_changed = true;
        }
    }

    /// A push of the list was acknowledged: `true` when `client_id` was one.
    pub(super) fn workspace_list_acked(&mut self, client_id: u64) -> bool {
        let Some(channel) = self.sync.as_mut().and_then(|s| s.workspace.as_mut()) else {
            return false;
        };
        let Some(version) = channel.list_pushed.remove(&client_id) else {
            return false;
        };
        channel.list_synced = version;
        channel.marker_changed = true;
        true
    }

    /// The workspace record a save writes into the open document's file (`share.json` v2).
    pub(super) fn workspace_share_record(&self) -> Option<WorkspaceShareRecord> {
        let link = self.sync.as_ref()?;
        let channel = link.workspace.as_ref()?;
        Some(WorkspaceShareRecord {
            workspace: channel.workspace,
            document: channel.stream?,
            synced_version: link.synced.clone(),
        })
    }

    /// The workspace's name and its join progress for the Share tab.
    pub(super) fn workspace_channel(&self) -> Option<&WorkspaceChannel> {
        self.sync.as_ref()?.workspace.as_deref()
    }

    pub(super) fn workspace_channel_mut(&mut self) -> Option<&mut WorkspaceChannel> {
        self.sync.as_mut()?.workspace.as_deref_mut()
    }

    pub(super) fn bump_share_revision(&mut self) {
        if let Some(link) = self.sync.as_mut() {
            link.revision = link.revision.wrapping_add(1);
        }
    }
}

/// A workspace room's session config: every envelope names peer 0 in the clear (decision 5);
/// the Loro peers travel inside the seal.
fn workspace_config(
    room: RoomId,
    since_seq: u64,
    start: SessionStart,
    seeds: &SyncSeeds,
) -> SessionConfig {
    SessionConfig {
        room,
        identity: IdentityKeys::from_entropy(&seeds.identity),
        peer_id: 0,
        nonce_seed: seeds.nonce,
        entropy_seed: seeds.entropy,
        since_seq,
        start,
    }
}
