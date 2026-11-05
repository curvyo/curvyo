//! The maker's device group as the page drives it (ADR 0040 decisions 1, 5 to 8,
//! `specs/0096-own-devices-sync` criteria 3, 6, 9, 13, 22 to 28): the group room's session, its
//! devices list, who is online, and the files and keys the host keeps.
//!
//! The page owns the socket (the same transport port as a room session), the clock and the
//! entropy. Changes of the devices list go out as updates; the room's snapshot is the list's.

use std::collections::{BTreeMap, VecDeque};

use curvyo_crypto_core::keyring::{Keyring, Role, decode_log, encode_log};
use curvyo_crypto_core::{EpochKey, Identity, IdentityKeys, MemberId};
use curvyo_document_core::{DevicesList, DocumentVersion};
use curvyo_sync_core::{
    ClientSession, ConnectionId, DeviceAction, Millis, RecordKind, RoomId, SessionConfig,
    SessionError, SessionStart, Target, reconcile,
};

use crate::session::{SyncSeeds, TransportAction};

/// How often a freshly paired device retries its first connection when the relay has not yet
/// stored the opener's `AddMember` (the welcome travels on another socket).
const JOIN_RETRIES: u8 = 5;
/// The wait between two such retries (ms).
const JOIN_RETRY_MS: u64 = 1_000;

/// The files of the group the host writes to `devices/`.
pub struct GroupFiles {
    /// `keyring.log`.
    pub keyring_log: Vec<u8>,
    /// `devices.loro`.
    pub list: Vec<u8>,
    /// The relay sequence the list holds.
    pub server_seq: u64,
    /// The list's version the room acknowledged last.
    pub synced_version: String,
}

/// How the group session was started, kept for a retry of a fresh join.
#[derive(Clone, Copy)]
struct Seeds {
    identity: [u8; 64],
    nonce: [u8; 32],
    entropy: [u8; 32],
}

/// The device group of this device.
// Independent facts of one group (online, removed, files and keys changed); one enum would merge
// unrelated states.
#[allow(clippy::struct_excessive_bools)]
pub struct DeviceGroupGlue {
    client: ClientSession,
    server: String,
    room: RoomId,
    seeds: Seeds,
    peer: u64,
    list: DevicesList,
    exported: DocumentVersion,
    synced: DocumentVersion,
    pushed: BTreeMap<u64, DocumentVersion>,
    actions: VecDeque<TransportAction>,
    online: bool,
    removed: bool,
    /// Join retries left, and when the next one is due.
    retries: Option<(u8, Option<Millis>)>,
    peers: BTreeMap<ConnectionId, MemberId>,
    last_seen: BTreeMap<MemberId, u64>,
    /// The group members at the last keyring change, for the added, removed and left notices.
    devices_seen: Vec<MemberId>,
    /// Devices to add once the session is online: identity and name.
    pending_adds: Vec<(Identity, String)>,
    /// This device's name, written to the list once it can be.
    own_label: Option<String>,
    /// This device is the app in a browser: its list entry is marked so once it can be.
    own_browser: bool,
    files_changed: bool,
    keys_changed: bool,
    /// A list update of a newer build was refused (ADR 0040 decision 11): reported once, and no
    /// snapshot is uploaded since this replica lacks that update.
    newer_list: bool,
    notices: VecDeque<String>,
    now: Millis,
    revision: u32,
}

impl DeviceGroupGlue {
    /// A new group with this device as its first admin (Pair on a device without a group): the
    /// session writes the genesis of `room` and connects.
    ///
    /// # Errors
    /// `Crypto` if the genesis cannot be made.
    pub fn create(
        server: &str,
        room: [u8; 16],
        label: &str,
        seeds: &SyncSeeds,
        peer: u64,
        now: Millis,
    ) -> Result<Self, SessionError> {
        let mut glue = Self::start(server, room, SessionStart::Share, 0, seeds, peer, now)?;
        glue.own_label = Some(label.to_owned());
        Ok(glue)
    }

    /// This device after its pairing: a member of `room` by the opener's `AddMember`, without
    /// keys yet; they arrive with the keyring.
    ///
    /// # Errors
    /// `Crypto`.
    pub fn join(
        server: &str,
        room: [u8; 16],
        label: &str,
        seeds: &SyncSeeds,
        peer: u64,
        now: Millis,
    ) -> Result<Self, SessionError> {
        let start = SessionStart::Member {
            keyring: Vec::new(),
            epoch_keys: Vec::new(),
        };
        let mut glue = Self::start(server, room, start, 0, seeds, peer, now)?;
        glue.own_label = Some(label.to_owned());
        glue.retries = Some((JOIN_RETRIES, None));
        Ok(glue)
    }

    /// The group as this device saved it (`devices/` and the credential store).
    ///
    /// # Errors
    /// `Crypto` for a damaged keyring; `Ended` for a damaged list.
    #[allow(clippy::too_many_arguments)] // The saved files, the keys and the seeds.
    pub fn resume(
        server: &str,
        room: [u8; 16],
        keyring_log: &[u8],
        list: &[u8],
        server_seq: u64,
        epoch_keys: Vec<(u32, [u8; 32])>,
        seeds: &SyncSeeds,
        peer: u64,
        now: Millis,
    ) -> Result<Self, SessionError> {
        let keyring = decode_log(keyring_log)?;
        let epoch_keys = epoch_keys
            .into_iter()
            .map(|(n, k)| (n, EpochKey::from_bytes(k)))
            .collect();
        let start = SessionStart::Member {
            keyring,
            epoch_keys,
        };
        let mut glue = Self::start(server, room, start, server_seq, seeds, peer, now)?;
        glue.list = DevicesList::from_snapshot(peer, list).map_err(|_| SessionError::Ended)?;
        glue.exported = glue.list.version();
        glue.synced = glue.exported.clone();
        glue.keys_changed = false;
        Ok(glue)
    }

    fn start(
        server: &str,
        room: [u8; 16],
        start: SessionStart,
        since_seq: u64,
        seeds: &SyncSeeds,
        peer: u64,
        now: Millis,
    ) -> Result<Self, SessionError> {
        let saved = Seeds {
            identity: seeds.identity,
            nonce: seeds.nonce,
            entropy: seeds.entropy,
        };
        let client = new_client(room, start, since_seq, saved, peer, now)?;
        let list = DevicesList::new(peer);
        let mut glue = Self {
            client,
            server: server.to_owned(),
            room: RoomId(room),
            seeds: saved,
            peer,
            // A fresh list's own first change (its `format_version`) goes out with the first
            // push: a replica that never saw it could not apply anything this one writes.
            exported: DocumentVersion::never(),
            synced: DocumentVersion::never(),
            list,
            pushed: BTreeMap::new(),
            actions: VecDeque::new(),
            online: false,
            removed: false,
            retries: None,
            peers: BTreeMap::new(),
            last_seen: BTreeMap::new(),
            devices_seen: Vec::new(),
            pending_adds: Vec::new(),
            own_label: None,
            own_browser: false,
            files_changed: true,
            keys_changed: true,
            newer_list: false,
            notices: VecDeque::new(),
            now,
            revision: 1,
        };
        glue.pump();
        Ok(glue)
    }

    /// The group room.
    #[must_use]
    pub fn room(&self) -> [u8; 16] {
        self.room.0
    }

    /// This device's member id.
    #[must_use]
    pub fn me(&self) -> MemberId {
        self.client.me()
    }

    /// The replayed group keyring, once known.
    #[must_use]
    pub fn keyring(&self) -> Option<&Keyring> {
        self.client.keyring()
    }

    /// The devices list.
    #[must_use]
    pub fn list(&self) -> &DevicesList {
        &self.list
    }

    /// Whether this device was removed from the group (criterion 24) or left it.
    #[must_use]
    pub fn removed(&self) -> bool {
        self.removed
    }

    /// Whether the group session is connected and holds the current key.
    #[must_use]
    pub fn is_online(&self) -> bool {
        self.online
    }

    /// Bumped whenever the view changes.
    #[must_use]
    pub fn revision(&self) -> u32 {
        self.revision
    }

    /// The next transport action.
    pub fn next_action(&mut self) -> Option<TransportAction> {
        self.actions.pop_front()
    }

    /// The socket opened.
    pub fn connected(&mut self, now: Millis) {
        self.now = now;
        self.client.connected(now);
        self.pump();
    }

    /// The socket closed or could not open.
    pub fn disconnected(&mut self, now: Millis) {
        self.now = now;
        self.client.disconnected(now);
        self.pump();
    }

    /// One message arrived.
    pub fn received(&mut self, bytes: &[u8], now: Millis) {
        self.now = now;
        self.client.received(bytes, now);
        self.pump();
    }

    /// The platform reports the network is back.
    pub fn network_back(&mut self, now: Millis) {
        self.now = now;
        self.client.network_back(now);
        self.pump();
    }

    /// Time passed: list changes go out, a fresh join retries, a stale group key is renewed.
    pub fn tick(&mut self, now: Millis) {
        self.now = now;
        if let Some((left, Some(at))) = self.retries
            && now >= at
        {
            self.retry(left, now);
        }
        self.client.tick(now);
        self.export_list();
        self.reconcile_group();
        self.pump();
    }

    /// Adds a paired device to the group (the opener's Pair): `AddMember` with every group key
    /// and its name in the list, as soon as the session is online.
    pub fn add_device(&mut self, identity: Identity, label: &str) {
        self.pending_adds.push((identity, label.to_owned()));
        self.write_pending();
        self.pump();
    }

    /// Whether `member` is a member of the group by this device's keyring, and the `AddMember`
    /// left this device (then the opener sends its welcome).
    #[must_use]
    pub fn has_member(&self, member: &MemberId) -> bool {
        self.pending_adds.iter().all(|(i, _)| i.member != *member)
            && self
                .client
                .keyring()
                .is_some_and(|k| k.role(member).is_some())
    }

    /// This device is a browser: its row shows as one on every device (ADR 0040 decision 1).
    pub fn set_browser(&mut self) {
        self.own_browser = true;
        self.write_own_label();
        self.pump();
    }

    /// Renames this device (criterion 16); the other devices see it with the next update.
    pub fn set_own_label(&mut self, label: &str) {
        self.own_label = Some(label.trim().to_owned());
        self.write_own_label();
        self.pump();
    }

    /// Writes the switch of workspace `id` named `name` (ADR 0040 decision 6). Call it for `on`
    /// after the workspace's devices were admitted, for off before they are removed.
    pub fn switch_workspace(&mut self, id: [u8; 16], name: &str, on: bool) {
        if self.list.switch_workspace(id, name, on).is_ok() {
            self.files_changed = true;
            self.bump();
        }
        self.export_list();
        self.pump();
    }

    /// Marks workspace `id` named `name` as left by the maker (`specs/0097` criterion 16, ADR
    /// 0041 decision 3): switched off with `left`. The leaving device runs the leave once
    /// [`list_acked`](Self::list_acked) says the relay stored the mark.
    pub fn leave_workspace(&mut self, id: [u8; 16], name: &str) {
        if self.list.leave_workspace(id, name).is_ok() {
            self.files_changed = true;
            self.bump();
        }
        self.export_list();
        self.pump();
    }

    /// Whether every change of the devices list went out and the relay acknowledged it.
    #[must_use]
    pub fn list_acked(&self) -> bool {
        self.pushed.is_empty() && self.exported == self.list.version()
    }

    /// Removes another device from the group with a key rotation (criterion 23). Its name
    /// stays in the list, so every device can still say who was removed; rows come from the
    /// keyring.
    ///
    /// # Errors
    /// `Offline` (nothing written), `NotAdmin`, `NotAMember`.
    pub fn remove_device(&mut self, member: MemberId) -> Result<(), SessionError> {
        self.client.remove_members(&[member])?;
        self.files_changed = true;
        self.pump();
        Ok(())
    }

    /// This device leaves the group (criterion 27): its own `RemoveMember`, then the session
    /// ends; the host forgets `devices/` and the group's keys.
    ///
    /// # Errors
    /// `Offline` (nothing written), `NotAMember`.
    pub fn leave(&mut self) -> Result<(), SessionError> {
        self.client.leave()?;
        self.pump();
        self.removed = true;
        self.actions.push_back(TransportAction::Close);
        self.bump();
        Ok(())
    }

    /// The files to write when they changed since the last call.
    pub fn take_files(&mut self) -> Option<GroupFiles> {
        self.take_files_when(false)
    }

    /// The files as [`Self::take_files`], also when only the host's part of `group.json` (the
    /// synced workspaces' folders) changed.
    pub fn take_files_when(&mut self, host_changed: bool) -> Option<GroupFiles> {
        if host_changed {
            self.files_changed = true;
        }
        if !std::mem::take(&mut self.files_changed) || self.removed {
            return None;
        }
        Some(GroupFiles {
            keyring_log: encode_log(&self.client.keyring_entries()).ok()?,
            list: self.list.snapshot().ok()?,
            server_seq: self.client.cursor(),
            synced_version: self.synced.to_text(),
        })
    }

    /// The group's epoch keys when they changed since the last call.
    pub fn take_key_update(&mut self) -> Option<Vec<(u32, [u8; 32])>> {
        if !std::mem::take(&mut self.keys_changed) {
            return None;
        }
        let keys: Vec<(u32, [u8; 32])> = self
            .client
            .epoch_keys()
            .into_iter()
            .map(|(n, k)| (n, *k.as_bytes()))
            .collect();
        (!keys.is_empty()).then_some(keys)
    }

    /// The next notice to show (a device was added, removed or left), once.
    pub fn take_notice(&mut self) -> Option<String> {
        self.notices.pop_front()
    }

    fn bump(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    fn retry(&mut self, left: u8, now: Millis) {
        let start = SessionStart::Member {
            keyring: Vec::new(),
            epoch_keys: Vec::new(),
        };
        // The session that is replaced never held a key, so it sealed nothing under its seeds.
        if let Ok(client) = new_client(self.room.0, start, 0, self.seeds, self.peer, now) {
            self.client = client;
            self.retries = Some((left.saturating_sub(1), None));
        }
    }

    fn write_pending(&mut self) {
        if !self.online {
            return;
        }
        let adds = std::mem::take(&mut self.pending_adds);
        for (identity, label) in adds {
            match self.client.add_member(identity, Role::Admin) {
                Ok(()) | Err(SessionError::AlreadyMember) => {
                    let _ = self.list.set_label(identity.member.0, &label);
                    self.files_changed = true;
                }
                Err(_) => self.pending_adds.push((identity, label)),
            }
        }
        self.export_list();
    }

    /// Writes this device's name and browser mark once it can. A device another one added
    /// waits until the list holds the entry the adder wrote: the entry is one register (last
    /// writer wins), and a write made before seeing the adder's would lose to it.
    fn write_own_label(&mut self) {
        if !self.online {
            return;
        }
        let me = self.me().0;
        let alone = self
            .client
            .keyring()
            .is_some_and(|keyring| keyring.participants().len() <= 1);
        if self.list.device(&me).is_none() && !alone {
            return;
        }
        if let Some(label) = self.own_label.take()
            && self.list.label(&me).as_deref() != Some(label.as_str())
        {
            let _ = self.list.set_label(me, &label);
            self.files_changed = true;
            self.bump();
        }
        if self.own_browser && self.list.device(&me).is_some_and(|d| !d.browser) {
            let _ = self.list.mark_browser(me);
            self.files_changed = true;
            self.bump();
        }
        self.export_list();
    }

    /// Pushes the list's changes since the last push.
    fn export_list(&mut self) {
        let version = self.list.version();
        if version == self.exported || !self.online {
            return;
        }
        let Ok(bytes) = self.list.export_updates(&self.exported) else {
            return;
        };
        if let Ok(id) = self.client.push(RecordKind::Update, bytes) {
            self.pushed.insert(id, version.clone());
            self.exported = version;
        }
    }

    /// Renews the group key after a device left by itself (criterion 27).
    fn reconcile_group(&mut self) {
        if !self.online {
            return;
        }
        let Some(group) = self.client.keyring() else {
            return;
        };
        let rotate = reconcile(group, &[], self.client.me()).iter().any(|plan| {
            plan.target == Target::Group
                && !plan.needs_admin
                && plan.actions.contains(&DeviceAction::Rotate)
        });
        if rotate {
            let _ = self.client.rotate();
        }
    }

    fn device_ids(&self) -> Vec<MemberId> {
        self.client.keyring().map_or_else(Vec::new, |k| {
            k.participants().iter().map(|p| p.identity.member).collect()
        })
    }

    fn label_of(&self, member: &MemberId) -> String {
        self.list
            .label(&member.0)
            .unwrap_or_else(|| "A device".to_owned())
    }
}

fn new_client(
    room: [u8; 16],
    start: SessionStart,
    since_seq: u64,
    seeds: Seeds,
    peer: u64,
    now: Millis,
) -> Result<ClientSession, SessionError> {
    ClientSession::new(
        SessionConfig {
            room: RoomId(room),
            identity: IdentityKeys::from_entropy(&seeds.identity),
            peer_id: peer,
            nonce_seed: seeds.nonce,
            entropy_seed: seeds.entropy,
            since_seq,
            start,
        },
        now,
    )
}

#[path = "device_group_events.rs"]
mod events;
#[path = "device_group_view.rs"]
mod view;

#[cfg(test)]
#[path = "device_group_glue_tests.rs"]
mod tests;
