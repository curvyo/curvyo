//! `Session`'s membership notices of the open share (`specs/0097` criteria 7, 8, 11, 27 to 30;
//! ADR 0041 decision 9), from the keyring and the admin set the host last showed for the room.
//!
//! The host keeps that set, the unread mark and the dismissals per data directory; none of it is
//! in the file or on the wire. The words come from `curvyo-ui-core`.

use curvyo_crypto_core::MemberId;
use curvyo_crypto_core::keyring::{HeldBack, HeldBackKind};
use curvyo_sync_core::{MembershipNotice, admin_set, membership_notices};
use serde_json::{Value, json};

use super::Session;
use super::sync::{SyncLink, hex, unhex};

impl Session {
    /// The admin devices of the open share's room as hex, ordered: what the host keeps as the
    /// last seen admin set. Empty without a live link.
    #[must_use]
    pub fn share_admin_set(&self) -> Vec<String> {
        self.sync
            .as_ref()
            .filter(|l| l.ended.is_none())
            .and_then(|l| l.client.confirmed_keyring())
            .map(|k| admin_set(&k).iter().map(|m| hex(&m.0)).collect())
            .unwrap_or_default()
    }

    /// The notices of the open share as a JSON array, measured against `previous` (the admin set
    /// last shown, as [`Session::share_admin_set`] gave it; `None` the first time) and
    /// `own_group` (the member ids of the maker's device group; this device when empty). Each is
    /// `{kind, member?, label?, left?, admins?, entries?, demoted?, author?, subject?, removal?}`.
    #[must_use]
    pub fn membership_notices_json(
        &self,
        previous: Option<&[String]>,
        own_group: &[String],
    ) -> String {
        let Some(link) = self.sync.as_ref().filter(|l| l.ended.is_none()) else {
            return "[]".to_owned();
        };
        // The notices read what the relay confirmed (ADR 0041 decision 11).
        let Some(keyring) = link.client.confirmed_keyring() else {
            return "[]".to_owned();
        };
        let ids = |hexes: &[String]| -> Vec<MemberId> {
            hexes
                .iter()
                .filter_map(|h| unhex::<32>(h).map(MemberId))
                .collect()
        };
        let previous = previous.map(ids);
        let mut group = ids(own_group);
        if group.is_empty() {
            group.push(link.client.me());
        }
        let notices = membership_notices(previous.as_deref(), &keyring, &group);
        let held = keyring.held_back();
        Value::Array(
            notices
                .iter()
                .map(|n| notice_json(link, &held, n))
                .collect(),
        )
        .to_string()
    }
}

fn notice_json(link: &SyncLink, held: &[HeldBack], notice: &MembershipNotice) -> Value {
    let person = |kind: &str, member: &MemberId| json!({"kind": kind, "member": hex(&member.0), "label": label(link, member)});
    match notice {
        MembershipNotice::NowAdmin(m) => person("nowAdmin", m),
        MembershipNotice::NoLongerAdmin(m) => person("noLongerAdmin", m),
        MembershipNotice::AdminLeft(m) => person("adminLeft", m),
        MembershipNotice::AdminRemoved(m) => person("adminRemoved", m),
        MembershipNotice::RotationPending { member, left } => json!({
            "kind": "rotationPending",
            "member": hex(&member.0),
            "label": label(link, member),
            "left": left,
        }),
        MembershipNotice::MutualRemoval { admins, entries } => json!({
            "kind": "mutualRemoval",
            "admins": admins
                .iter()
                .map(|m| json!({"member": hex(&m.0), "label": label(link, m)}))
                .collect::<Vec<_>>(),
            "entries": entries.iter().map(|e| hex(&e.0)).collect::<Vec<_>>(),
            // Criterion 27: a demotion among them (crossed demotions, or a removal that arrived
            // after its author's demotion) reads "demoted each other"; removals alone (keyrings
            // merged outside one relay) "removed each other".
            "demoted": held
                .iter()
                .any(|h| entries.contains(&h.entry) && h.kind == HeldBackKind::Demotion),
        }),
        MembershipNotice::NotApplied(held) => json!({
            "kind": "notApplied",
            "author": hex(&held.author.0),
            "subject": hex(&held.subject.0),
            "label": label(link, &held.subject),
            "removal": held.kind == HeldBackKind::Removal,
            "entries": [hex(&held.entry.0)],
        }),
        MembershipNotice::OneAdmin => json!({"kind": "oneAdmin"}),
        MembershipNotice::OnlyOwnAdmins => json!({"kind": "onlyOwnAdmins"}),
    }
}

/// The label shown for `member`: this device's own, a kept one, or empty.
fn label(link: &SyncLink, member: &MemberId) -> String {
    if *member == link.client.me() {
        return link.label.clone();
    }
    link.labels.get(member).cloned().unwrap_or_default()
}
