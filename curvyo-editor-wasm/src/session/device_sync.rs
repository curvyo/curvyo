//! The open shared workspace's part in own-devices sync (ADR 0040 decisions 5, 6 and 8,
//! `specs/0096-own-devices-sync` criteria 9 to 11, 13, 16, 17, 23 to 27): its keyring reconciled
//! with the device group, and arriving in a synced workspace as a member the group admitted.

use curvyo_crypto_core::IdentityKeys;
use curvyo_crypto_core::keyring::Keyring;
use curvyo_document_core::WorkspaceList;
use curvyo_sync_core::{
    ClientSession, DeviceAction, Millis, RoomId, SessionConfig, SessionStart, WorkspaceInput,
    reconcile,
};

use super::Session;
use super::sync::{JoinPhase, SyncSeeds};
use super::workspace_channel::WorkspaceChannel;

/// What reconciling the open workspace did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReconcileReport {
    /// Keyring changes written.
    pub written: usize,
    /// This device is not an admin of the workspace and changes are due (criteria 17, 26).
    pub needs_admin: bool,
    /// Devices still to add or remove there by an admin: (member hex, add).
    pub pending: Vec<(String, bool)>,
}

impl Session {
    /// Reconciles the open shared workspace with the device group's keyring (`on`: the devices
    /// list's switch for it): group devices missing are added with this device's role, former
    /// ones and, when off, the others are removed with one rotation, and a key a device that
    /// left still holds is renewed. Where this device is not an admin nothing is written.
    /// `None` while no shared workspace's room is online here (nothing could be written).
    pub fn reconcile_workspace(&mut self, group: &Keyring, on: bool) -> Option<ReconcileReport> {
        let mut report = ReconcileReport::default();
        let link = self.sync.as_mut().filter(|l| l.workspace.is_some())?;
        if link.ended.is_some() || !link.client.is_online() {
            return None;
        }
        let me = link.client.me();
        let keyring = link.client.keyring()?;
        let plans = reconcile(group, &[WorkspaceInput { on, keyring }], me);
        for plan in plans {
            if plan.needs_admin {
                report.needs_admin |= !plan.actions.is_empty();
                for action in &plan.actions {
                    match action {
                        DeviceAction::Add { identity, .. } => report
                            .pending
                            .push((super::sync::hex(&identity.member.0), true)),
                        DeviceAction::Remove { members } => report
                            .pending
                            .extend(members.iter().map(|m| (super::sync::hex(&m.0), false))),
                        DeviceAction::Rotate => {}
                    }
                }
                continue;
            }
            for action in plan.actions {
                let done = match action {
                    DeviceAction::Add { identity, role } => {
                        link.client.admit_device(group, identity, role).is_ok()
                    }
                    DeviceAction::Remove { members } => {
                        link.client.remove_members(&members).is_ok()
                    }
                    DeviceAction::Rotate => link.client.rotate().is_ok(),
                };
                if done {
                    report.written += 1;
                }
            }
        }
        if report.written > 0 {
            link.revision = link.revision.wrapping_add(1);
        }
        self.sync_pump();
        Some(report)
    }

    /// Leave my devices in this synced workspace (criterion 27, ADR 0040 decision 8): this
    /// device writes its own `RemoveMember` and the link ends; the files stay. `false` while
    /// offline or without a workspace link (nothing was written).
    pub fn leave_synced_workspace(&mut self) -> bool {
        let Some(link) = self.sync.as_mut().filter(|l| l.workspace.is_some()) else {
            return false;
        };
        if link.client.leave().is_err() {
            return false;
        }
        self.sync_pump();
        if let Some(link) = self.sync.as_mut() {
            // The removal goes out first, then the socket closes.
            link.actions.push_back(super::sync::TransportAction::Close);
            link.ended = Some(super::sync::SyncEnd::Refused);
            link.online = false;
            link.revision = link.revision.wrapping_add(1);
        }
        true
    }

    /// Arrives in the synced workspace `room` on `server` as a member the device group admitted
    /// (criteria 10 and 11): no invite, no one else online needed; its documents come as file
    /// writes like a join's.
    pub fn join_synced_workspace(
        &mut self,
        server: &str,
        room: [u8; 16],
        name: &str,
        label: &str,
        seeds: &SyncSeeds,
        now: Millis,
    ) -> bool {
        let Some(server) = curvyo_sync_core::normalize_server_address(server) else {
            return false;
        };
        let config = SessionConfig {
            room: RoomId(room),
            identity: IdentityKeys::from_entropy(&seeds.identity),
            peer_id: 0,
            nonce_seed: seeds.nonce,
            entropy_seed: seeds.entropy,
            since_seq: 0,
            start: SessionStart::Member {
                keyring: Vec::new(),
                epoch_keys: Vec::new(),
            },
        };
        let Ok(mut client) = ClientSession::new(config, now) else {
            return false;
        };
        client.use_workspace_snapshots();
        let mut channel =
            WorkspaceChannel::new(room, name, WorkspaceList::new(self.peer), self.peer);
        channel.joining = true;
        self.start_workspace_link(client, channel, &server, label, now);
        if let Some(link) = self.sync.as_mut() {
            link.join = Some(JoinPhase::Connecting);
        }
        true
    }
}
