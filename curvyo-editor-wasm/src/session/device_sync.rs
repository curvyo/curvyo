//! The open shared workspace's part in own-devices sync (ADR 0040 decisions 5, 6 and 8,
//! `specs/0096-own-devices-sync` criteria 9 to 11, 13, 16, 17, 23 to 27): its keyring reconciled
//! with the device group, and arriving in a synced workspace as a member the group admitted.

use curvyo_crypto_core::IdentityKeys;
use curvyo_crypto_core::MemberId;
use curvyo_crypto_core::keyring::Keyring;
use curvyo_document_core::EndReason;
use curvyo_document_core::WorkspaceList;
use curvyo_sync_core::{
    ClientSession, DeviceAction, Millis, Plan, RoomId, SessionConfig, SessionStart, WorkspaceInput,
    admin_remains, reconcile, reconcile_with_left,
};
use curvyo_ui_core::membership_text::PERSON_LEAVE_REFUSED;

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
    /// The maker leaves this workspace: this device's own leave went out, or waits for the
    /// snapshot its rotation asked for (`specs/0097` criterion 16).
    pub leaving: bool,
}

impl Session {
    /// Reconciles the open shared workspace with the device group's keyring (`on`: the devices
    /// list's switch for it): group devices missing are added with this device's role, former
    /// ones and, when off, the others are removed with one rotation, and a key a device that
    /// left still holds is renewed. Where this device is not an admin nothing is written.
    /// `None` while no shared workspace's room is online here (nothing could be written).
    pub fn reconcile_workspace(&mut self, group: &Keyring, on: bool) -> Option<ReconcileReport> {
        self.reconcile_open_workspace(group, on, false)
    }

    /// Whether the maker may leave the open synced workspace from all their devices: an admin
    /// must remain outside the device group, counting the devices the group removed (a stolen
    /// one) as the maker's too (`specs/0097` criterion 16, ADR 0041 decision 1).
    #[must_use]
    pub fn person_leave_allowed(&self, group: &Keyring) -> bool {
        let Some(keyring) = self
            .sync
            .as_ref()
            .filter(|l| l.workspace.is_some() && l.ended.is_none())
            .and_then(|l| l.client.keyring())
        else {
            return false;
        };
        let devices: Vec<MemberId> = group
            .participants()
            .iter()
            .map(|p| p.identity)
            .chain(group.former_participants())
            .map(|identity| identity.member)
            .collect();
        admin_remains(keyring, &devices)
    }

    /// Reconciles the open shared workspace whose devices list entry is marked `left` (ADR 0041
    /// decision 3): an admin device removes the other group devices still in it with one
    /// rotation, waits for the snapshot that asks for, then leaves; a member device leaves at
    /// once. The leaving device writes the mark and waits for its acknowledgement first; every
    /// other device calls this when it reads the mark. The ended record says "left" however
    /// the removal arrives. `None` while no shared workspace's room is online here.
    pub fn reconcile_left_workspace(&mut self, group: &Keyring) -> Option<ReconcileReport> {
        self.reconcile_open_workspace(group, false, true)
    }

    fn reconcile_open_workspace(
        &mut self,
        group: &Keyring,
        on: bool,
        left: bool,
    ) -> Option<ReconcileReport> {
        let mut report = ReconcileReport::default();
        let link = self.sync.as_mut().filter(|l| l.workspace.is_some())?;
        if link.ended.is_some() || !link.client.is_online() {
            return None;
        }
        let me = link.client.me();
        let keyring = link.client.keyring()?;
        let inputs = [WorkspaceInput { on, keyring }];
        let plans: Vec<Plan> = if left {
            link.leaving = true;
            reconcile_with_left(group, &inputs, &[keyring.document_id()], me)
        } else {
            reconcile(group, &inputs, me)
        };
        let mut leave = false;
        for plan in plans {
            leave |= plan.leave;
            if plan.needs_admin {
                report.needs_admin |= !plan.actions.is_empty();
                report
                    .pending
                    .extend(plan.actions.iter().flat_map(pending_of));
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
        if leave {
            report.leaving = true;
            self.leave_after_snapshot();
        }
        Some(report)
    }

    /// The person leave's own `RemoveMember`, once no snapshot this device was asked for is
    /// still to go out (the rotation's, ADR 0031 decision 6a).
    pub(super) fn leave_after_snapshot(&mut self) {
        let Some(link) = self.sync.as_mut() else {
            return;
        };
        let due = link
            .workspace
            .as_ref()
            .is_some_and(|c| c.snapshot_due.is_some() || c.prepared.is_some());
        if due {
            link.leave_after_snapshot = true;
            return;
        }
        link.leave_after_snapshot = false;
        if let Err(error) = link.client.leave() {
            // Refused now (an admin it counted on is gone): the workspace stays synced here and
            // the maker is told, never left believing they left.
            link.leaving = false;
            if error == curvyo_sync_core::SessionError::LastAdmin {
                link.notice(PERSON_LEAVE_REFUSED.to_owned(), true);
            }
            return;
        }
        self.sync_pump();
        self.end_share(EndReason::Left, "");
        if let Some(link) = self.sync.as_mut() {
            link.actions.push_back(super::sync::TransportAction::Close);
        }
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

/// The devices an action this device may not write waits for an admin to add (`true`) or
/// remove, as member hex.
fn pending_of(action: &DeviceAction) -> Vec<(String, bool)> {
    match action {
        DeviceAction::Add { identity, .. } => vec![(super::sync::hex(&identity.member.0), true)],
        DeviceAction::Remove { members } => members
            .iter()
            .map(|m| (super::sync::hex(&m.0), false))
            .collect(),
        DeviceAction::Rotate => Vec::new(),
    }
}
