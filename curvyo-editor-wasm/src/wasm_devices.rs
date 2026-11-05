//! My devices' `wasm-bindgen` surface (`specs/0096-own-devices-sync`): one pairing attempt and
//! the device group, each behind the transport port the page's `SyncTransport` drives, the open
//! workspace's reconciliation and arrival, and the fixed words. Pass-throughs to
//! `pairing_glue`, `device_group_glue`, the session and `curvyo-ui-core`.

use curvyo_crypto_core::{DocumentId, Identity};
use curvyo_sync_core::Millis;
use curvyo_ui_core::devices_text as texts;
use wasm_bindgen::prelude::*;

use crate::session::{SyncSeeds, TransportAction, hex, unhex};
use crate::wasm_api::WasmSession;
use crate::wasm_sync::SyncAction;
use crate::{DeviceGroupGlue, PairPhase, PairingGlue};

fn millis(now: f64) -> Millis {
    // Date.now() is a whole number of milliseconds well inside u64.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Millis(now.max(0.0) as u64)
}

fn seeds(identity: &[u8], nonce: &[u8], entropy: &[u8]) -> Option<SyncSeeds> {
    Some(SyncSeeds {
        identity: identity.try_into().ok()?,
        nonce: nonce.try_into().ok()?,
        entropy: entropy.try_into().ok()?,
    })
}

fn action(action: TransportAction) -> SyncAction {
    match action {
        TransportAction::Connect(address) => SyncAction {
            kind: "connect".to_owned(),
            address,
            bytes: Vec::new(),
        },
        TransportAction::Send(bytes) => SyncAction {
            kind: "send".to_owned(),
            address: String::new(),
            bytes,
        },
        TransportAction::Close => SyncAction {
            kind: "close".to_owned(),
            address: String::new(),
            bytes: Vec::new(),
        },
    }
}

/// One pairing attempt.
#[wasm_bindgen]
pub struct WasmPairing {
    glue: PairingGlue,
}

#[wasm_bindgen]
impl WasmPairing {
    /// Add a device: this device shows a code. `undefined` for a bad server address or name.
    #[must_use]
    pub fn open(
        server: &str,
        identity: &[u8],
        label: &str,
        entropy: &[u8],
        now: f64,
    ) -> Option<WasmPairing> {
        let identity: [u8; 64] = identity.try_into().ok()?;
        let entropy: [u8; 32] = entropy.try_into().ok()?;
        PairingGlue::opener(server, &identity, label, entropy, millis(now))
            .ok()
            .map(|glue| Self { glue })
    }

    /// Join my devices with the typed `code`. `undefined` for a text that is not a code.
    #[must_use]
    pub fn join(
        server: &str,
        code: &str,
        identity: &[u8],
        label: &str,
        entropy: &[u8],
        now: f64,
    ) -> Option<WasmPairing> {
        let identity: [u8; 64] = identity.try_into().ok()?;
        let entropy: [u8; 32] = entropy.try_into().ok()?;
        PairingGlue::joiner(server, code, &identity, label, entropy, millis(now))
            .ok()
            .map(|glue| Self { glue })
    }

    /// The next transport action.
    pub fn sync_next_action(&mut self) -> Option<SyncAction> {
        self.glue.next_action().map(action)
    }

    /// The socket opened.
    pub fn sync_connected(&mut self, now: f64) {
        self.glue.connected(millis(now));
    }

    /// The socket closed or did not open.
    pub fn sync_disconnected(&mut self, now: f64) {
        self.glue.disconnected(millis(now));
    }

    /// A message arrived.
    pub fn sync_received(&mut self, bytes: &[u8], now: f64) {
        self.glue.received(bytes, millis(now));
    }

    /// The timer.
    pub fn sync_tick(&mut self, now: f64) {
        self.glue.tick(millis(now));
    }

    /// The network is back; a pairing does not reconnect.
    #[allow(clippy::unused_self)] // The transport port's shape.
    pub fn sync_network_back(&mut self, _now: f64) {}

    /// The view as JSON (`phase` and its fields).
    #[must_use]
    pub fn view_json(&self) -> String {
        self.glue.view_json()
    }

    /// Bumped whenever the view changes.
    #[must_use]
    pub fn revision(&self) -> u32 {
        self.glue.revision()
    }

    /// The other device's identity as 64 bytes once compared (the opener's Pair adds it).
    #[must_use]
    pub fn peer_identity(&self) -> Vec<u8> {
        match self.glue.phase() {
            PairPhase::Compare { peer, .. }
            | PairPhase::Pairing { peer, .. }
            | PairPhase::Paired { peer, .. } => peer.to_bytes().to_vec(),
            _ => Vec::new(),
        }
    }

    /// The other device's name once compared.
    #[must_use]
    pub fn peer_label(&self) -> String {
        match self.glue.phase() {
            PairPhase::Compare { label, .. }
            | PairPhase::Pairing { label, .. }
            | PairPhase::Paired { label, .. } => label.clone(),
            _ => String::new(),
        }
    }

    /// The group room (hex) once paired; empty before.
    #[must_use]
    pub fn group_room(&self) -> String {
        match self.glue.phase() {
            PairPhase::Paired { group_room, .. } => hex(&group_room.0),
            _ => String::new(),
        }
    }

    /// The opener pressed Pair; the device group writes `AddMember` first.
    pub fn mark_pairing(&mut self) {
        self.glue.mark_pairing();
    }

    /// The opener's Pair once the group holds the joiner: sends the group room (hex).
    pub fn pair(&mut self, group_room: &str) -> bool {
        unhex::<16>(group_room).is_some_and(|room| self.glue.pair(DocumentId(room)).is_ok())
    }

    /// Cancel or Words differ.
    pub fn cancel(&mut self, words_differ: bool) {
        self.glue.cancel(words_differ);
    }
}

/// The maker's device group.
#[wasm_bindgen]
pub struct WasmDeviceGroup {
    pub(crate) glue: DeviceGroupGlue,
}

#[wasm_bindgen]
impl WasmDeviceGroup {
    /// A new group (Pair on a device without one); `room` is 16 random bytes.
    #[must_use]
    #[allow(clippy::too_many_arguments)] // The host's bytes cross as flat arguments (ADR 0001 §5).
    pub fn create(
        server: &str,
        room: &[u8],
        label: &str,
        identity: &[u8],
        nonce: &[u8],
        entropy: &[u8],
        peer: u64,
        now: f64,
    ) -> Option<WasmDeviceGroup> {
        let seeds = seeds(identity, nonce, entropy)?;
        DeviceGroupGlue::create(
            server,
            room.try_into().ok()?,
            label,
            &seeds,
            peer,
            millis(now),
        )
        .ok()
        .map(|glue| Self { glue })
    }

    /// This device after its pairing, in the group room `room` (hex).
    #[must_use]
    #[allow(clippy::too_many_arguments)] // The host's bytes cross as flat arguments (ADR 0001 §5).
    pub fn join(
        server: &str,
        room: &str,
        label: &str,
        identity: &[u8],
        nonce: &[u8],
        entropy: &[u8],
        peer: u64,
        now: f64,
    ) -> Option<WasmDeviceGroup> {
        let seeds = seeds(identity, nonce, entropy)?;
        DeviceGroupGlue::join(server, unhex(room)?, label, &seeds, peer, millis(now))
            .ok()
            .map(|glue| Self { glue })
    }

    /// The group as saved: `devices/` files and the keys (`[[epoch, hex]]` JSON).
    #[must_use]
    #[allow(clippy::too_many_arguments)] // The host's bytes cross as flat arguments (ADR 0001 §5).
    pub fn resume(
        server: &str,
        room: &str,
        keyring_log: &[u8],
        list: &[u8],
        server_seq: f64,
        keys_json: &str,
        identity: &[u8],
        nonce: &[u8],
        entropy: &[u8],
        peer: u64,
        now: f64,
    ) -> Option<WasmDeviceGroup> {
        let seeds = seeds(identity, nonce, entropy)?;
        let keys: Vec<(u32, String)> = serde_json::from_str(keys_json).ok()?;
        let keys = keys
            .into_iter()
            .filter_map(|(n, k)| Some((n, unhex::<32>(&k)?)))
            .collect();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let server_seq = server_seq.max(0.0) as u64;
        DeviceGroupGlue::resume(
            server,
            unhex(room)?,
            keyring_log,
            list,
            server_seq,
            keys,
            &seeds,
            peer,
            millis(now),
        )
        .ok()
        .map(|glue| Self { glue })
    }

    /// The group room as hex.
    #[must_use]
    pub fn room(&self) -> String {
        hex(&self.glue.room())
    }

    /// The next transport action.
    pub fn sync_next_action(&mut self) -> Option<SyncAction> {
        self.glue.next_action().map(action)
    }

    /// The socket opened.
    pub fn sync_connected(&mut self, now: f64) {
        self.glue.connected(millis(now));
    }

    /// The socket closed or did not open.
    pub fn sync_disconnected(&mut self, now: f64) {
        self.glue.disconnected(millis(now));
    }

    /// A message arrived.
    pub fn sync_received(&mut self, bytes: &[u8], now: f64) {
        self.glue.received(bytes, millis(now));
    }

    /// The timer.
    pub fn sync_tick(&mut self, now: f64) {
        self.glue.tick(millis(now));
    }

    /// The network is back.
    pub fn sync_network_back(&mut self, now: f64) {
        self.glue.network_back(millis(now));
    }

    /// The My devices block as JSON, with the synced workspaces' facts (`facts_json`: an array
    /// of what `WasmSession::device_facts_json` returns, or `[]`).
    #[must_use]
    pub fn view_json(&self, offset_minutes: i32, facts_json: &str) -> String {
        let facts: Vec<crate::session::DeviceFacts> =
            serde_json::from_str(facts_json).unwrap_or_default();
        self.glue.view_json(offset_minutes, &facts)
    }

    /// Bumped whenever the view changes.
    #[must_use]
    pub fn revision(&self) -> u32 {
        self.glue.revision()
    }

    /// Whether the group session is online.
    #[must_use]
    pub fn is_online(&self) -> bool {
        self.glue.is_online()
    }

    /// Whether this device was removed or left.
    #[must_use]
    pub fn removed(&self) -> bool {
        self.glue.removed()
    }

    /// Adds the paired device `identity` (64 bytes) named `label`.
    pub fn add_device(&mut self, identity: &[u8], label: &str) -> bool {
        let Ok(bytes) = <[u8; 64]>::try_from(identity) else {
            return false;
        };
        self.glue.add_device(Identity::from_bytes(&bytes), label);
        true
    }

    /// Whether the device `identity` (64 bytes) is in the group and its `AddMember` went out.
    #[must_use]
    pub fn has_device(&self, identity: &[u8]) -> bool {
        <[u8; 64]>::try_from(identity)
            .is_ok_and(|bytes| self.glue.has_member(&Identity::from_bytes(&bytes).member))
    }

    /// Marks this device as the app in a browser in the devices list.
    pub fn set_browser(&mut self) {
        self.glue.set_browser();
    }

    /// Renames this device.
    pub fn set_own_label(&mut self, label: &str) {
        self.glue.set_own_label(label);
    }

    /// The switch of workspace `id` (hex) named `name`.
    pub fn switch_workspace(&mut self, id: &str, name: &str, on: bool) {
        if let Some(id) = unhex::<16>(id) {
            self.glue.switch_workspace(id, name, on);
        }
    }

    /// Whether the devices list marks workspace `id` (hex) on.
    #[must_use]
    pub fn is_switched_on(&self, id: &str) -> bool {
        unhex::<16>(id).is_some_and(|id| self.glue.list().switched_on().contains(&id))
    }

    /// Removes the device `member` (hex): empty, or the refusal's sentence.
    pub fn remove_device(&mut self, member: &str) -> String {
        let Some(member) = unhex::<32>(member) else {
            return texts::NOT_CONNECTED.to_owned();
        };
        match self
            .glue
            .remove_device(curvyo_crypto_core::MemberId(member))
        {
            Ok(()) => String::new(),
            Err(_) => texts::NOT_CONNECTED.to_owned(),
        }
    }

    /// Leave my devices: empty, or the refusal's sentence.
    pub fn leave(&mut self) -> String {
        match self.glue.leave() {
            Ok(()) => String::new(),
            // ADR 0041 decision 1: the group's only admin device stays (`specs/0097`; a change
            // of `0096` behaviour, where the last device could leave).
            Err(curvyo_sync_core::SessionError::LastAdmin) => {
                curvyo_ui_core::membership_text::GROUP_ONLY_ADMIN.to_owned()
            }
            Err(_) => texts::NOT_CONNECTED.to_owned(),
        }
    }

    /// The files to write when changed, or when `host_changed` (the host's part of `group.json`
    /// changed): `{keyring_log, list, server_seq, synced_version}` (byte arrays as arrays), or
    /// empty.
    pub fn take_files_json(&mut self, host_changed: bool) -> String {
        self.glue
            .take_files_when(host_changed)
            .map_or_else(String::new, |files| {
                serde_json::json!({
                    "keyring_log": files.keyring_log,
                    "list": files.list,
                    "server_seq": files.server_seq,
                    "synced_version": files.synced_version,
                })
                .to_string()
            })
    }

    /// The group's keys when changed, `[[epoch, hex]]`, or empty.
    pub fn take_key_update(&mut self) -> String {
        self.glue
            .take_key_update()
            .map_or_else(String::new, |keys| {
                let keys: Vec<(u32, String)> = keys.iter().map(|(n, k)| (*n, hex(k))).collect();
                serde_json::to_string(&keys).unwrap_or_default()
            })
    }

    /// The next notice, or empty.
    pub fn take_notice(&mut self) -> String {
        self.glue.take_notice().unwrap_or_default()
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// Reconciles the open shared workspace with the group: `{written, needs_admin, pending}`,
    /// or empty while its room is not online here.
    pub fn reconcile_workspace(&mut self, group: &WasmDeviceGroup, on: bool) -> String {
        let Some(keyring) = group.glue.keyring() else {
            return String::new();
        };
        let Some(report) = self.session.reconcile_workspace(keyring, on) else {
            return String::new();
        };
        serde_json::json!({
            "written": report.written,
            "needs_admin": report.needs_admin,
            "pending": report.pending,
        })
        .to_string()
    }

    /// Leave my devices in this synced workspace: its own `RemoveMember`, then the link ends;
    /// `false` while offline.
    pub fn leave_synced_workspace(&mut self) -> bool {
        self.session.leave_synced_workspace()
    }

    /// The facts of this session's synced workspace for the My devices block (`on`: its
    /// switch), as JSON, or empty without a workspace link.
    #[must_use]
    pub fn device_facts_json(&self, group: &WasmDeviceGroup, on: bool) -> String {
        let Some(keyring) = group.glue.keyring() else {
            return String::new();
        };
        self.session
            .device_facts(keyring, on)
            .and_then(|facts| serde_json::to_string(&facts).ok())
            .unwrap_or_default()
    }

    /// Arrives in the synced workspace `room` (hex) as a member the group admitted.
    #[allow(clippy::too_many_arguments)] // The host's bytes cross as flat arguments (ADR 0001 §5).
    pub fn join_synced_workspace(
        &mut self,
        server: &str,
        room: &str,
        name: &str,
        label: &str,
        identity: &[u8],
        nonce: &[u8],
        entropy: &[u8],
        now: f64,
    ) -> bool {
        let (Some(seeds), Some(room)) = (seeds(identity, nonce, entropy), unhex::<16>(room)) else {
            return false;
        };
        self.session
            .join_synced_workspace(server, room, name, label, &seeds, millis(now))
    }
}
