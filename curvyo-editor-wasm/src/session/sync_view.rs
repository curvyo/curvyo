//! `Session`'s Share tab and collaboration pill as the host reads them (`specs/0092` UX notes
//! §2 to §6): the connection state, the join phase and the participant list, as one JSON record.

use std::collections::BTreeSet;

use curvyo_crypto_core::MemberId;
use curvyo_crypto_core::keyring::{Participant, Role};
use curvyo_sync_core::PendingKind;
use curvyo_ui_core::share_text::{
    ConnectionState, cut_label, initials, join_refusal_text, participant_colour, pill_name,
    server_shown, share_subject,
};
use serde_json::{Value, json};

use super::Session;
use super::sync::{JoinPhase, SyncEnd, SyncLink};

impl Session {
    /// The fingerprint of a current participant.
    pub(super) fn participant_fingerprint(&self, member: &MemberId) -> Option<String> {
        let keyring = self.sync.as_ref()?.client.keyring()?;
        keyring
            .participants()
            .into_iter()
            .find(|p| p.identity.member == *member)
            .map(|p| p.identity.fingerprint())
    }

    /// Changes whenever anything the Share tab or the pill shows changes, so the host reads
    /// [`Session::share_view_json`] only then.
    #[must_use]
    pub fn share_revision(&self) -> u32 {
        self.sync.as_ref().map_or(0, |s| s.revision)
    }

    /// The Share tab and the pill: whether the document is shared, the server, the connection,
    /// the join phase and the participants (own row first, then by member id).
    #[must_use]
    pub fn share_view_json(&self) -> String {
        // A device removed from its shared workspace shows a local workspace: "Not shared" with
        // Share workspace available (`specs/0095-workspace` criterion 44, `ux-notes.md` 4).
        let Some(link) = self.sync.as_ref().filter(|_| !self.workspace_removed()) else {
            return json!({
                "shared": false,
                "live": false,
                "subject": share_subject(false, 0, ConnectionState::NotConnected),
                "participants": [],
                "ended": self.sync.as_ref().and_then(|l| l.ended).map(ended_name),
            })
            .to_string();
        };
        let state = link.state();
        let keyring = link.client.keyring();
        let is_admin = keyring.and_then(|k| k.role(&link.client.me())) == Some(Role::Admin);
        let participants = participant_rows(link, is_admin);
        let shared = link.join.is_none_or(|j| j == JoinPhase::Done) && link.ended.is_none();
        let online_people = participants
            .iter()
            .filter(|p| p["online"].as_bool() == Some(true))
            .count();
        let join = join_json(link);
        let workspace = workspace_json(link, participants.len(), state);
        let people = people_json(link, &participants);
        json!({
            "shared": shared,
            "live": link.ended.is_none(),
            "server": link.server,
            "serverShown": server_shown(&link.server),
            "state": state.name(),
            "stateLine": state.tab_line(&link.server),
            "pillText": state.pill_text(),
            "pillName": pill_name(online_people, state),
            "subject": share_subject(shared, participants.len(), state),
            "epoch": keyring.map(|k| k.current_epoch().number),
            "admin": is_admin,
            "online": link.online,
            "participants": participants,
            "people": people,
            "join": join,
            "ended": link.ended.map(ended_name),
            "endedRoom": link.ended.map(|_| super::sync::hex(&ended_room(link))),
            "workspace": workspace,
        })
        .to_string()
    }
}

/// The word the host reads for how a link ended.
fn ended_name(end: SyncEnd) -> &'static str {
    match end {
        SyncEnd::Removed => "removed",
        SyncEnd::Refused => "refused",
        SyncEnd::Left => "left",
    }
}

/// The room an ended link synced: a workspace's id, else the document's room.
fn ended_room(link: &SyncLink) -> [u8; 16] {
    link.workspace
        .as_ref()
        .map_or(link.client.room().0, |channel| channel.workspace)
}

/// The participants an own act of `kind` waits on the relay for.
fn pending_of(link: &SyncLink, kind: PendingKind) -> Vec<MemberId> {
    link.pending
        .iter()
        .filter(|a| a.kind == kind)
        .map(|a| a.member)
        .collect()
}

/// The busy word of a row whose own act waits on the relay (ADR 0041 decision 11), as
/// `membership_word` names it; `None` when nothing waits.
fn pending_word(link: &SyncLink, member: &MemberId) -> Option<&'static str> {
    link.pending
        .iter()
        .rev()
        .find(|a| a.member == *member)
        .and_then(|a| match a.kind {
            PendingKind::MakeAdmin => Some("making-admin"),
            PendingKind::MakeMember => Some("making-member"),
            PendingKind::Remove => Some("removing"),
            PendingKind::Leave => Some("leaving"),
            PendingKind::Add => None,
        })
}

/// The participant rows of the Share tab: own row first, then by member id. A participant this
/// device removed, or this device after its own leave, stays until the relay confirms it, and
/// one it added back appears only then (ADR 0041 decision 11).
fn participant_rows(link: &SyncLink, is_admin: bool) -> Vec<Value> {
    let me = link.client.me();
    let online: BTreeSet<MemberId> = link.peers.values().filter_map(|p| p.member).collect();
    let adding = pending_of(link, PendingKind::Add);
    let mut rows: Vec<Participant> = link
        .client
        .keyring()
        .map(curvyo_crypto_core::keyring::Keyring::participants)
        .unwrap_or_default()
        .into_iter()
        .filter(|p| !adding.contains(&p.identity.member))
        .collect();
    let mut leaving = pending_of(link, PendingKind::Remove);
    leaving.extend(pending_of(link, PendingKind::Leave));
    if let Some(confirmed) = link.client.confirmed_keyring() {
        for participant in confirmed.participants() {
            let member = participant.identity.member;
            if leaving.contains(&member) && !rows.iter().any(|p| p.identity.member == member) {
                rows.push(participant);
            }
        }
    }
    rows.sort_by_key(|p| (p.identity.member != me, p.identity.member.0));
    let alone = rows.len() < 2;
    rows.iter()
        .map(|participant| {
            let member = participant.identity.member;
            let own = member == me;
            let label = if own {
                link.label.clone()
            } else {
                link.labels.get(&member).cloned().unwrap_or_default()
            };
            let fingerprint = participant.identity.fingerprint();
            let shown = if label.is_empty() {
                curvyo_ui_core::sharing_text::UNNAMED_DEVICE.to_owned()
            } else {
                cut_label(&label, 24)
            };
            let pending = pending_word(link, &member);
            // A row whose own act waits offers no further act on it.
            let free = pending.is_none();
            json!({
                "member": super::sync::hex(&member.0),
                "label": shown,
                "initials": initials(&shown),
                "colour": participant_colour(&fingerprint),
                "fingerprint": fingerprint,
                "self": own,
                "online": own && link.online || online.contains(&member),
                "admin": participant.role == Role::Admin,
                "pending": pending,
                // Criterion 32: only an admin sees Remove, never on its own row.
                "removable": free && is_admin && !own,
                // `specs/0097` criteria 2 and 14: roles for admins on other rows; Step down for
                // an admin and Leave for everyone on the own row, no Leave when alone.
                "canMakeAdmin": free && is_admin && !own && participant.role == Role::Member,
                "canMakeMember": free && is_admin && !own && participant.role == Role::Admin,
                "canStepDown": free && own && participant.role == Role::Admin,
                "canLeave": free && own && !alone,
            })
        })
        .collect()
}

/// The participant rows grouped by person for display (`specs/0105` criterion 14): each
/// person's line and its devices' member ids, by the label each device sealed.
fn people_json(link: &SyncLink, rows: &[Value]) -> Value {
    let me = link.client.me();
    let labels: Vec<String> = rows
        .iter()
        .map(|row| {
            let member = row["member"].as_str().unwrap_or_default();
            let own = super::sync::unhex::<32>(member).is_some_and(|m| m == me.0);
            if own {
                link.label.clone()
            } else {
                super::sync::unhex::<32>(member)
                    .and_then(|m| link.labels.get(&MemberId(m)).cloned())
                    .unwrap_or_default()
            }
        })
        .collect();
    let devices: Vec<curvyo_ui_core::sharing_text::PersonDevice<'_>> = rows
        .iter()
        .zip(&labels)
        .map(|(row, label)| curvyo_ui_core::sharing_text::PersonDevice {
            label: label.trim(),
            own: row["self"].as_bool() == Some(true),
            admin: row["admin"].as_bool() == Some(true),
        })
        .collect();
    curvyo_ui_core::sharing_text::group_people(&devices)
        .into_iter()
        .map(|person| {
            json!({
                "line": person.line,
                "you": person.you,
                "members": person
                    .devices
                    .iter()
                    .map(|&i| rows[i]["member"].clone())
                    .collect::<Vec<_>>(),
            })
        })
        .collect()
}

/// The shared workspace as the Share tab's Workspace block and the File tab's header line show
/// it (`specs/0095-workspace` `ux-notes.md` sections 2.1, 4 and 9); `None` for a document
/// shared on its own.
fn workspace_json(
    link: &SyncLink,
    people: usize,
    state: curvyo_ui_core::share_text::ConnectionState,
) -> Option<Value> {
    use curvyo_ui_core::workspace_share_text::{shared_header_line, shared_through_text};
    let channel = link.workspace.as_ref()?;
    let receiving = channel.receiving();
    Some(json!({
        "id": super::sync::hex(&channel.workspace),
        "name": channel.name,
        "headerLine": shared_header_line(people, state, receiving),
        "receiving": receiving.map(|(done, total)| {
            curvyo_ui_core::workspace_share_text::receiving_text(done, total)
        }),
        "openInWorkspace": channel.stream.is_some(),
        "documentLine": shared_through_text(&channel.name),
        "people": curvyo_ui_core::workspace_share_text::people_text(people),
        "joining": channel.joining,
    }))
}

/// The join phase as the Share tab shows it (UX notes §2.5), `None` when this is no join.
fn join_json(link: &SyncLink) -> Option<Value> {
    let phase = link.join?;
    let workspace = link.workspace.as_ref();
    // A join that never reached the server says so (UX notes §2.5).
    if phase == JoinPhase::Connecting && link.failed_first {
        return Some(json!({
            "phase": "refused",
            "text": join_refusal_text(
                curvyo_ui_core::share_text::JoinRefusal::Unreachable,
                &link.server,
            ),
        }));
    }
    Some(match phase {
        JoinPhase::Connecting => json!({"phase": "connecting", "text": "Connecting..."}),
        JoinPhase::Waiting => json!({
            "phase": "waiting",
            "text": curvyo_ui_core::workspace_share_text::WORKSPACE_WAITING_TEXT,
        }),
        JoinPhase::Downloading => json!({
            "phase": "downloading",
            "text": workspace
                .and_then(|c| c.receiving())
                .map_or_else(
                    || "Downloading...".to_owned(),
                    |(done, total)| {
                        curvyo_ui_core::workspace_share_text::receiving_text(done, total)
                    },
                ),
        }),
        JoinPhase::Done => json!({"phase": "done", "text": ""}),
        JoinPhase::Refused(reason) => json!({
            "phase": "refused",
            "text": join_refusal_text(reason, &link.server),
        }),
    })
}
