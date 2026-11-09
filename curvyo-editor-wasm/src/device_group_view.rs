//! The My devices block as the page shows it (`specs/0096-own-devices-sync` criteria 22, 26
//! and 27, `ux-notes.md` 2 and 5): rows with their detail lines from the synced workspaces'
//! facts, removals still to finish, and pending key renewals.

use curvyo_crypto_core::MemberId;
use curvyo_crypto_core::keyring::Keyring;
use curvyo_ui_core::devices_text as texts;

use super::DeviceGroupGlue;
use crate::session::{DeviceFacts, OwnState, hex};

impl DeviceGroupGlue {
    /// The My devices block as JSON: the rows (own first, then devices removed from the group
    /// that are still in a workspace), each with one detail line per synced workspace from
    /// `facts`, the synced workspaces, and the block's states (`ux-notes.md` 2 and 5).
    #[must_use]
    pub fn view_json(&self, offset_minutes: i32, facts: &[DeviceFacts]) -> String {
        let me = self.client.me();
        let now = self.now.unix_seconds();
        let online: Vec<MemberId> = self.peers.values().copied().collect();
        let mut rows = Vec::new();
        let participants = self
            .client
            .keyring()
            .map(Keyring::participants)
            .unwrap_or_default();
        let mut ordered: Vec<_> = participants.iter().collect();
        ordered.sort_by_key(|p| p.identity.member != me);
        for participant in ordered {
            let member = participant.identity.member;
            let own = member == me;
            // Every participant of the group keyring is a row, named or not (crypto review 6).
            let label = self
                .list
                .label(&member.0)
                .or_else(|| own.then(|| self.own_label.clone()).flatten())
                .unwrap_or_else(|| texts::UNNAMED_DEVICE.to_owned());
            let entry = self.list.device(&member.0);
            let browser = entry.as_ref().is_some_and(|e| e.browser);
            let seen = texts::seen_text(
                online.contains(&member),
                self.last_seen.get(&member).copied(),
                now,
                offset_minutes,
                browser,
            );
            let (line, details) = if own {
                own_lines(facts)
            } else {
                (seen.clone(), other_details(facts, &hex(&member.0), &seen))
            };
            rows.push(serde_json::json!({
                "member": hex(&member.0),
                "label": label,
                "fingerprint": participant.identity.fingerprint(),
                "own": own,
                "browser": browser,
                "seen": line,
                "details": details,
                "removing": false,
            }));
        }
        let mut alerts = Vec::new();
        for (member, label) in self.removed_but_present(facts) {
            let still: Vec<&DeviceFacts> = facts
                .iter()
                .filter(|f| f.members.contains(&member))
                .collect();
            let total = facts.len();
            alerts.extend(still.iter().map(|f| texts::still_in_text(&label, &f.name)));
            rows.push(serde_json::json!({
                "member": member,
                "label": label,
                "fingerprint": "",
                "own": false,
                "seen": texts::removal_incomplete_text(total - still.len(), total),
                "details": [],
                "removing": true,
            }));
        }
        let workspaces: Vec<serde_json::Value> = self
            .list
            .workspaces()
            .into_iter()
            .map(|w| serde_json::json!({"id": hex(&w.id), "name": w.name, "on": w.on, "left": w.left}))
            .collect();
        let devices = participants.len().max(1);
        let renewals = facts.iter().filter(|f| f.rotate_pending).count();
        serde_json::json!({
            "room": hex(&self.room.0),
            "online": self.online,
            "removed": self.removed,
            "rows": rows,
            "count": texts::devices_count_text(devices),
            "workspaces": workspaces,
            "renewal": if renewals > 0 { texts::key_renewal_pending_text(renewals) } else { String::new() },
            "alerts": alerts,
        })
        .to_string()
    }

    /// Devices removed from or gone from the group that a synced workspace still lists
    /// (criterion 26): member hex and name.
    fn removed_but_present(&self, facts: &[DeviceFacts]) -> Vec<(String, String)> {
        let Some(keyring) = self.client.keyring() else {
            return Vec::new();
        };
        let current: Vec<String> = keyring
            .participants()
            .iter()
            .map(|p| hex(&p.identity.member.0))
            .collect();
        let mut out: Vec<(String, String)> = Vec::new();
        for member in facts.iter().flat_map(|f| f.members.iter()) {
            if current.contains(member) || out.iter().any(|(m, _)| m == member) {
                continue;
            }
            let Some(id) = unhex32(member) else {
                continue;
            };
            if keyring.was_removed(&MemberId(id)) {
                let label = self
                    .list
                    .label(&id)
                    .unwrap_or_else(|| texts::UNNAMED_DEVICE.to_owned());
                out.push((member.clone(), label));
            }
        }
        out
    }
}

/// Line 3 of the own row (the worst state wins) and its detail lines (`ux-notes.md` 5.2).
fn own_lines(facts: &[DeviceFacts]) -> (String, Vec<serde_json::Value>) {
    let details = facts
        .iter()
        .map(|f| serde_json::json!({"name": f.name, "state": texts::sync_state_text(sync_state(f.state))}))
        .collect();
    let worst = facts
        .iter()
        .map(|f| f.state)
        .max_by_key(|state| match state {
            OwnState::InSync => 0,
            OwnState::NotAdmin => 1,
            OwnState::Answer => 2,
            OwnState::Syncing { .. } => 3,
            OwnState::Receiving { .. } => 4,
            OwnState::Offline { .. } => 5,
        });
    let line = worst.map_or_else(String::new, |state| {
        texts::sync_state_text(sync_state(state))
    });
    (line, details)
}

/// Another device's detail lines: in sync while connected to the workspace's room, waiting for
/// its first copy while not yet a member there, else when it was last seen.
fn other_details(facts: &[DeviceFacts], member: &str, seen: &str) -> Vec<serde_json::Value> {
    facts
        .iter()
        .filter(|f| f.state != OwnState::Answer)
        .map(|f| {
            let state = if f.present.iter().any(|m| m == member) {
                texts::sync_state_text(texts::SyncState::InSync)
            } else if !f.members.iter().any(|m| m == member) {
                texts::sync_state_text(texts::SyncState::WaitingForFirstCopy)
            } else {
                seen.to_owned()
            };
            serde_json::json!({"name": f.name, "state": state})
        })
        .collect()
}

fn sync_state(state: OwnState) -> texts::SyncState {
    match state {
        OwnState::InSync => texts::SyncState::InSync,
        OwnState::Syncing { done, total } => texts::SyncState::Syncing(done, total),
        OwnState::Offline { waiting } => texts::SyncState::Offline(waiting),
        OwnState::Receiving { done, total } => texts::SyncState::Receiving(done, total),
        OwnState::Answer => texts::SyncState::WaitingForAnswer,
        OwnState::NotAdmin => texts::SyncState::NotAdmin,
    }
}

fn unhex32(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64 {
        return None;
    }
    let mut out = [0_u8; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(text.get(index * 2..index * 2 + 2)?, 16).ok()?;
    }
    Some(out)
}
