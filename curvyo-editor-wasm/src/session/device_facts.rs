//! What one synced workspace tells the My devices block (`specs/0096-own-devices-sync` criteria
//! 22, 26 and 27, `ux-notes.md` 2.1 and 5): this device's sync state there, the workspace's
//! members and who is connected, and what an admin still has to write.

use std::collections::BTreeSet;

use curvyo_crypto_core::MemberId;
use curvyo_crypto_core::keyring::Keyring;
use curvyo_sync_core::{DeviceAction, WorkspaceInput, reconcile};
use serde::{Deserialize, Serialize};

use super::Session;
use super::sync::hex;

/// This device's state in one synced workspace (`ux-notes.md` 5.1).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum OwnState {
    /// Current.
    #[default]
    InSync,
    /// Documents with records not yet in their files: done of total.
    Syncing {
        /// Documents without waiting records.
        done: usize,
        /// Documents of the workspace.
        total: usize,
    },
    /// Not connected; changes not yet acknowledged.
    Offline {
        /// Pushes waiting for the relay.
        waiting: usize,
    },
    /// The first copy arriving: done of total documents.
    Receiving {
        /// Documents written.
        done: usize,
        /// Documents to write.
        total: usize,
    },
    /// A copy is here already and the maker has not chosen yet (criterion 14).
    Answer,
    /// Devices are due here and this device is not an admin (criteria 17, 26).
    NotAdmin,
}

/// One synced workspace's facts; member ids are hex. The page gathers them from each
/// workspace's session (or names a workspace waiting for an answer) and hands them to the group.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceFacts {
    /// The workspace id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// This device's state there.
    pub state: OwnState,
    /// Its members.
    pub members: Vec<String>,
    /// Members connected to its room now.
    pub present: Vec<String>,
    /// Group devices still to add there.
    pub pending_adds: Vec<String>,
    /// Former group devices still to remove there.
    pub pending_removes: Vec<String>,
    /// A key a departed device holds is not renewed yet (criterion 27).
    pub rotate_pending: bool,
}

impl Session {
    /// The facts of the shared workspace this session carries, measured against the device
    /// group's keyring and the workspace's switch `on`; writes nothing, works offline. `None`
    /// without a workspace link or after it ended.
    #[must_use]
    pub fn device_facts(&self, group: &Keyring, on: bool) -> Option<DeviceFacts> {
        let link = self.sync.as_ref().filter(|l| l.ended.is_none())?;
        let channel = link.workspace.as_ref()?;
        let mut facts = DeviceFacts {
            id: hex(&channel.workspace),
            name: channel.name.clone(),
            ..DeviceFacts::default()
        };
        let present: BTreeSet<MemberId> = link.peers.values().filter_map(|p| p.member).collect();
        facts.present = present.iter().map(|m| hex(&m.0)).collect();
        let mut needs_admin = false;
        if let Some(keyring) = link.client.keyring() {
            facts.members = keyring
                .participants()
                .iter()
                .map(|p| hex(&p.identity.member.0))
                .collect();
            for plan in reconcile(group, &[WorkspaceInput { on, keyring }], link.client.me()) {
                needs_admin |= plan.needs_admin && !plan.actions.is_empty();
                for action in plan.actions {
                    match action {
                        DeviceAction::Add { identity, .. } => {
                            facts.pending_adds.push(hex(&identity.member.0));
                        }
                        DeviceAction::Remove { members } => facts
                            .pending_removes
                            .extend(members.iter().map(|m| hex(&m.0))),
                        DeviceAction::Rotate => facts.rotate_pending = true,
                    }
                }
            }
        }
        facts.state = if let Some((done, total)) = channel.receiving() {
            OwnState::Receiving { done, total }
        } else if needs_admin {
            OwnState::NotAdmin
        } else if !link.online {
            OwnState::Offline {
                waiting: link.pushed.len() + channel.list_pushed.len(),
            }
        } else {
            let waiting = channel.waiting_documents();
            if waiting == 0 {
                OwnState::InSync
            } else {
                let total = channel.document_count().max(waiting);
                OwnState::Syncing {
                    done: total - waiting,
                    total,
                }
            }
        };
        Some(facts)
    }
}
