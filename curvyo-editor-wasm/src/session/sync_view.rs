//! `Session`'s Share tab and collaboration pill as the host reads them (`specs/0092` UX notes
//! §2 to §6): the connection state, the join phase and the participant list, as one JSON record.

use std::collections::BTreeSet;

use curvyo_crypto_core::MemberId;
use curvyo_crypto_core::keyring::Role;
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
        // Share workspace available (`specs/0094-workspace` criterion 44, `ux-notes.md` 4).
        let Some(link) = self.sync.as_ref().filter(|_| !self.workspace_removed()) else {
            let record = self.document.share();
            return json!({
                "shared": record.is_some(),
                "live": false,
                "server": record.as_ref().map(|r| r.server.clone()),
                "subject": share_subject(record.is_some(), 0, ConnectionState::NotConnected),
                "participants": [],
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
            "join": join,
            "ended": link.ended.map(|e| match e {
                SyncEnd::Removed => "removed",
                SyncEnd::Refused => "refused",
                SyncEnd::Moved => "moved",
            }),
            "workspace": workspace,
        })
        .to_string()
    }
}

/// The participant rows of the Share tab: own row first, then by member id.
fn participant_rows(link: &SyncLink, is_admin: bool) -> Vec<Value> {
    let me = link.client.me();
    let online: BTreeSet<MemberId> = link.peers.values().filter_map(|p| p.member).collect();
    let mut rows = link
        .client
        .keyring()
        .map(curvyo_crypto_core::keyring::Keyring::participants)
        .unwrap_or_default();
    rows.sort_by_key(|p| (p.identity.member != me, p.identity.member.0));
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
                "Participant".to_owned()
            } else {
                cut_label(&label, 24)
            };
            json!({
                "member": super::sync::hex(&member.0),
                "label": shown,
                "initials": initials(&shown),
                "colour": participant_colour(&fingerprint),
                "fingerprint": fingerprint,
                "self": own,
                "online": own && link.online || online.contains(&member),
                "admin": participant.role == Role::Admin,
                // Criterion 32: only an admin sees Remove, never on its own row.
                "removable": is_admin && !own,
            })
        })
        .collect()
}

/// The shared workspace as the Share tab's Workspace block and the File tab's header line show
/// it (`specs/0094-workspace` `ux-notes.md` sections 2.1, 4 and 9); `None` for a document
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
            "text": if workspace.is_some() {
                curvyo_ui_core::workspace_share_text::WORKSPACE_WAITING_TEXT
            } else {
                "Waiting for a member of this document to let you in"
            },
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
