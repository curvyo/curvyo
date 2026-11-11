//! `Session`'s binding of the relay session (ADR 0031 decisions 3, 9, 10): it feeds the
//! transport's bytes and the host's clock into `ClientSession` and applies what comes out to the
//! document.
//!
//! The socket lives in the host (ADR 0031 decision 2): it asks for [`TransportAction`]s, opens,
//! sends and closes, and reports back. Local commits are exported as updates; remote records wait
//! for the start of the next frame and are imported there, so one redraw shows any number of
//! them (decision 3). Entropy and time always come from the host.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use curvyo_crypto_core::MemberId;
use curvyo_document_core::{AssetHash, DocumentVersion};
use curvyo_sync_core::{ClientSession, ConnectionId, InviteLink, Millis, RecordKind, SessionError};
use curvyo_ui_core::presence::PresencePayload;
use curvyo_ui_core::share_text::{ConnectionState, JoinRefusal};

use super::Session;

/// A loss shorter than this shows "Reconnecting..."; longer, "Offline" (criterion 17's 5 s).
pub(super) const OFFLINE_AFTER_MS: u64 = 4_000;
/// An offline period longer than this ends with the notice "Back online." (UX notes §6).
pub(super) const BACK_ONLINE_AFTER_MS: u64 = 5_000;

/// A document's epoch keys to store: its room as hex and each key with its epoch number.
pub type KeyUpdate = (String, Vec<(u32, [u8; 32])>);

/// What the host's transport does next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportAction {
    /// Open a WebSocket to this address, then report `connected` or `disconnected`.
    Connect(String),
    /// Send one binary message.
    Send(Vec<u8>),
    /// Close the socket.
    Close,
}

/// The fresh bytes a session needs from the host: the device's identity entropy (from the
/// credential store) and two seeds of 32 bytes from the platform's random source.
pub struct SyncSeeds {
    /// The device identity's 64 bytes.
    pub identity: [u8; 64],
    /// Seeds the envelope nonces.
    pub nonce: [u8; 32],
    /// Seeds keys, invite secrets and jitter.
    pub entropy: [u8; 32],
}

/// Where a join stands (UX notes §2.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinPhase {
    /// Reaching the server.
    Connecting,
    /// Admitted to the room, waiting for a member to hand over the key.
    Waiting,
    /// The key arrived; the document is on its way.
    Downloading,
    /// The document arrived.
    Done,
    /// Refused or expired; the session has ended.
    Refused(JoinRefusal),
}

/// Why a shared session ended for good.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncEnd {
    /// This device was removed (criterion 33).
    Removed,
    /// The relay refused the session.
    Refused,
    /// This device left by its own `RemoveMember` (`specs/0097` criteria 10 and 15).
    Left,
}

/// One other participant's connection, as the relay reported it and its presence says.
#[derive(Debug, Clone, Default)]
pub(super) struct PeerState {
    pub(super) member: Option<MemberId>,
    pub(super) presence: Option<PresencePayload>,
}

/// The shared document's link to its room.
// Independent facts of one link (online, ever online, a failed first attempt, waiting for the
// document, the rejection shown, keys or document changed); one enum would merge unrelated states.
#[allow(clippy::struct_excessive_bools)]
pub(super) struct SyncLink {
    pub(super) client: ClientSession,
    pub(super) server: String,
    pub(super) label: String,
    /// The version whose changes have gone out or came in; later local commits are exported.
    pub(super) exported: DocumentVersion,
    /// The version each queued push brings the relay to, by its client id.
    pub(super) pushed: BTreeMap<u64, DocumentVersion>,
    /// The version the relay acknowledged last (`share.json` `synced_version`).
    pub(super) synced: DocumentVersion,
    /// Assets the room already holds or that went out.
    pub(super) assets: BTreeSet<AssetHash>,
    /// The asset store's size when the assets were last compared.
    pub(super) asset_total: u64,
    /// Opened records waiting for the next frame, with their sequence.
    pub(super) remote: VecDeque<(u64, RecordKind, Vec<u8>)>,
    pub(super) peers: BTreeMap<ConnectionId, PeerState>,
    /// The labels seen for each member this session (adrs.md decision 5: cached outside the
    /// document).
    pub(super) labels: BTreeMap<MemberId, String>,
    pub(super) join: Option<JoinPhase>,
    pub(super) online: bool,
    pub(super) ever_online: bool,
    pub(super) failed_first: bool,
    pub(super) lost_at: Option<Millis>,
    /// When the last message arrived.
    pub(super) last_heard: Millis,
    pub(super) now: Millis,
    pub(super) ended: Option<SyncEnd>,
    pub(super) rejected_shown: bool,
    /// Reconnecting from the server's snapshot after a rejected update (criterion 25).
    pub(super) resyncing: bool,
    /// The epoch keys changed: the host stores them.
    pub(super) keys_changed: bool,
    /// The last presence sent, to send only changes.
    pub(super) last_presence: Vec<u8>,
    /// The local selection the "is editing this" notice was last given for.
    pub(super) contested_for: Vec<curvyo_document_core::NodeId>,
    pub(super) actions: VecDeque<TransportAction>,
    pub(super) notices: VecDeque<(String, bool)>,
    /// Remote records changed the document since the host last asked.
    pub(super) changed: bool,
    /// Bumped whenever anything the Share tab or the pill shows changes.
    pub(super) revision: u32,
    /// The workspace part of a shared workspace's link (`specs/0095-workspace`), `None` for a
    /// document shared on its own.
    pub(super) workspace: Option<Box<super::workspace_channel::WorkspaceChannel>>,
    /// The highest sequence applied to the open document when a save packed it.
    pub(super) packed_through: Option<u64>,
    /// A shared workspace's link that ended by removal or leave: the marker's ended record for
    /// the host to write (ADR 0041 decision 6).
    pub(super) workspace_ended: Option<curvyo_document_core::EndedRecord>,
    /// A person leave waits for the workspace snapshot its rotation asked for (ADR 0041
    /// decision 3).
    pub(super) leave_after_snapshot: bool,
    /// The maker leaves this workspace (its devices list entry is marked `left`), or this
    /// device's own leave waits for the relay (ADR 0041 decisions 2, 3 and 11): however the
    /// removal arrives, the ended record says "left".
    pub(super) leaving: bool,
    /// The own keyring acts the relay had not confirmed at the last pump (ADR 0041 decision 11).
    pub(super) pending: Vec<curvyo_sync_core::PendingAct>,
    /// The participants this device added back whose add the relay has not confirmed yet: their
    /// "was added back" notice waits for it.
    pub(super) adding_back: BTreeSet<MemberId>,
}

impl SyncLink {
    pub(super) fn new(client: ClientSession, server: String, label: String, now: Millis) -> Self {
        // A leave written before the app closed is still pending: it goes out first and the
        // share ends as "left" (ADR 0041 decision 11).
        let leaving = client
            .pending_acts()
            .iter()
            .any(|a| a.kind == curvyo_sync_core::PendingKind::Leave);
        Self {
            client,
            server,
            label,
            exported: DocumentVersion::never(),
            pushed: BTreeMap::new(),
            synced: DocumentVersion::never(),
            assets: BTreeSet::new(),
            asset_total: 0,
            remote: VecDeque::new(),
            peers: BTreeMap::new(),
            labels: BTreeMap::new(),
            join: None,
            online: false,
            ever_online: false,
            failed_first: false,
            lost_at: None,
            last_heard: now,
            now,
            ended: None,
            rejected_shown: false,
            resyncing: false,
            keys_changed: true,
            last_presence: Vec::new(),
            contested_for: Vec::new(),
            actions: VecDeque::new(),
            notices: VecDeque::new(),
            changed: false,
            revision: 1,
            workspace: None,
            packed_through: None,
            workspace_ended: None,
            leave_after_snapshot: false,
            leaving,
            pending: Vec::new(),
            adding_back: BTreeSet::new(),
        }
    }

    /// The connection state the pill and the Share tab show.
    pub(super) fn state(&self) -> ConnectionState {
        if self.join == Some(JoinPhase::Waiting) {
            return ConnectionState::Waiting;
        }
        if self.resyncing {
            return ConnectionState::Resyncing;
        }
        if self.online {
            return ConnectionState::Connected;
        }
        if !self.ever_online {
            return if self.failed_first {
                ConnectionState::NotConnected
            } else {
                ConnectionState::Reconnecting
            };
        }
        match self.lost_at {
            Some(at) if self.now.since(at) < OFFLINE_AFTER_MS => ConnectionState::Reconnecting,
            _ => ConnectionState::Offline,
        }
    }

    pub(super) fn notice(&mut self, text: String, alert: bool) {
        self.notices.push_back((text, alert));
        self.revision = self.revision.wrapping_add(1);
    }
}

impl Session {
    /// Makes `link` this session's link. Its revision continues the one it replaces, so the
    /// host, which reads the Share tab again only when the revision changes, never misses the
    /// new link's state (a second return check ending on the first one's number showed the
    /// first one's answer).
    pub(super) fn install_link(&mut self, mut link: Box<SyncLink>) {
        self.document.set_live_share(true);
        if let Some(old) = self.sync.as_ref() {
            link.revision = old.revision.wrapping_add(link.revision).max(1);
        }
        self.sync = Some(link);
    }

    /// Whether this session carries a live link.
    #[must_use]
    pub fn is_shared(&self) -> bool {
        self.sync.as_ref().is_some_and(|s| s.ended.is_none())
    }

    /// The transport opened its socket.
    pub fn sync_connected(&mut self, now: Millis) {
        if let Some(link) = self.sync.as_mut() {
            link.now = now;
            link.client.connected(now);
        }
        self.sync_pump();
    }

    /// The transport's socket closed or could not open.
    pub fn sync_disconnected(&mut self, now: Millis) {
        if let Some(link) = self.sync.as_mut() {
            link.now = now;
            if !link.ever_online && !link.failed_first {
                link.failed_first = true;
                link.revision = link.revision.wrapping_add(1);
            }
            link.client.disconnected(now);
        }
        self.sync_pump();
    }

    /// One binary message arrived.
    pub fn sync_received(&mut self, bytes: &[u8], now: Millis) {
        if let Some(link) = self.sync.as_mut() {
            link.now = now;
            link.last_heard = now;
            link.client.received(bytes, now);
        }
        self.sync_pump();
    }

    /// The platform reports the network is back: reconnect now (criterion 18).
    pub fn sync_network_back(&mut self, now: Millis) {
        if let Some(link) = self.sync.as_mut() {
            link.now = now;
            link.client.network_back(now);
        }
        self.sync_pump();
    }

    /// Time passed (the host's timer, every 50 ms): local commits go out, presence is sent,
    /// pings, reconnects and expiries run.
    pub fn sync_tick(&mut self, now: Millis) {
        let Some(link) = self.sync.as_mut() else {
            return;
        };
        let state = link.state();
        link.now = now;
        if link.state() != state {
            link.revision = link.revision.wrapping_add(1);
        }
        self.sync_export();
        self.export_workspace_list();
        self.send_presence(now);
        if let Some(link) = self.sync.as_mut() {
            link.client.tick(now);
        }
        self.sync_pump();
    }

    /// The start of a frame: local commits go out first, then every waiting remote record is
    /// imported (decision 3). `true` when the document changed.
    pub fn sync_frame_start(&mut self) -> bool {
        self.sync_export();
        self.sync_apply_remote()
    }

    /// The next thing the transport does, `None` when there is nothing.
    pub fn sync_next_action(&mut self) -> Option<TransportAction> {
        self.sync.as_mut()?.actions.pop_front()
    }

    /// Whether remote records changed the document since the last call: the host refreshes its
    /// panels.
    pub fn take_sync_changed(&mut self) -> bool {
        self.sync
            .as_mut()
            .is_some_and(|s| std::mem::take(&mut s.changed))
    }

    /// The next notice of sharing (text, whether it is an alert), once.
    pub fn take_sync_notice(&mut self) -> Option<(String, bool)> {
        self.sync.as_mut()?.notices.pop_front()
    }

    /// The epoch keys to store, when they changed since the last call: the room as hex and each
    /// key with its epoch number.
    pub fn take_key_update(&mut self) -> Option<KeyUpdate> {
        let link = self.sync.as_mut()?;
        if !std::mem::take(&mut link.keys_changed) {
            return None;
        }
        let keys = link
            .client
            .epoch_keys()
            .into_iter()
            .map(|(n, k)| (n, *k.as_bytes()))
            .collect::<Vec<_>>();
        (!keys.is_empty()).then(|| (hex(&link.client.room().0), keys))
    }

    /// Changes the "Name shown to others"; the room hears it with the next presence message.
    pub fn set_share_label(&mut self, label: &str) {
        if let Some(link) = self.sync.as_mut() {
            label.trim().clone_into(&mut link.label);
        }
    }

    /// Writes a one-time invite valid for `seconds` and returns its link and expiry in seconds
    /// since the Unix epoch (criterion 2).
    ///
    /// # Errors
    /// `Offline`, `NotAdmin` and the other refusals of [`ClientSession::invite`].
    pub fn create_invite(
        &mut self,
        seconds: u64,
        now: Millis,
    ) -> Result<(String, u64), SessionError> {
        let link = self.sync.as_mut().ok_or(SessionError::Ended)?;
        let expires = now.unix_seconds() + seconds;
        let secret = link.client.invite(expires)?;
        let text = InviteLink {
            server: link.server.clone(),
            kind: curvyo_sync_core::InviteKind::Workspace,
            secret,
        }
        .to_text();
        self.sync_pump();
        Ok((text, expires))
    }

    /// Leaves the room without a trace in the document (Cancel while joining): the socket
    /// closes and nothing more is sent.
    pub fn leave_sync(&mut self) {
        if let Some(link) = self.sync.as_mut() {
            link.actions.clear();
            link.actions.push_back(TransportAction::Close);
            link.ended = Some(SyncEnd::Refused);
            link.join = None;
            link.online = false;
            link.revision = link.revision.wrapping_add(1);
        }
        self.document.set_live_share(false);
    }

    /// Drops a link that ended (a Share that failed or was cancelled, after [`Self::leave_sync`]
    /// and the transport took its close), so the Share tab shows the state before the attempt.
    pub fn drop_ended_link(&mut self) {
        if self.sync.as_ref().is_some_and(|link| link.ended.is_some()) {
            self.sync = None;
        }
    }
}

/// Lowercase hex of `bytes`.
pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// The bytes of lowercase or uppercase hex, `None` for anything else.
pub fn unhex<const N: usize>(text: &str) -> Option<[u8; N]> {
    if text.len() != N * 2 {
        return None;
    }
    let mut out = [0; N];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(text.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(out)
}
