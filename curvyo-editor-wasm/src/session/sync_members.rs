//! `Session`'s participant commands of the Share tab (`specs/0092` criteria 32 to 34,
//! `specs/0097` criteria 2 to 5, 10 to 15, 17 to 19): roles, removal with key rotation, leave,
//! Change keys now, Add back, and the device's cache of the labels other participants announced.

use std::collections::BTreeMap;

use curvyo_crypto_core::MemberId;
use curvyo_crypto_core::keyring::{Payload, Role};
use curvyo_document_core::{EndReason, EndedRecord};
use curvyo_sync_core::{History, SessionError};
use curvyo_ui_core::share_text::{LEFT_NOTICE, REMOVED_NOTICE};

use super::Session;
use super::sync::{SyncEnd, SyncLink, hex, unhex};

fn member_of(member_hex: &str) -> Result<MemberId, SessionError> {
    unhex::<32>(member_hex)
        .map(MemberId)
        .ok_or(SessionError::NotAMember)
}

impl Session {
    /// Removes the participant `member_hex` (its member id as hex) with a key rotation: the
    /// client writes `RemoveMember` and `NewEpoch` and the session uploads a snapshot sealed
    /// under the new epoch (criterion 32, ADR 0031 decision 6a). Admin only, while connected.
    /// The participant's label stays in the cache: the Removed or left row shows it (ADR 0041
    /// decision 5).
    ///
    /// # Errors
    /// `NotAMember` for an unknown id or this device; `NotAdmin`, `Offline`, `Ended`, `Crypto`
    /// as [`curvyo_sync_core::ClientSession::remove`].
    pub fn remove_participant(&mut self, member_hex: &str) -> Result<(), SessionError> {
        let member = member_of(member_hex)?;
        let link = self.sync.as_mut().ok_or(SessionError::Ended)?;
        link.client.remove(member)?;
        link.revision = link.revision.wrapping_add(1);
        self.sync_pump();
        Ok(())
    }

    /// Make admin (`admin`) or Make member of the participant `member_hex`; Step down is this
    /// device's own id with `admin` false (`specs/0097` criteria 2 to 5). Admin only.
    ///
    /// # Errors
    /// `LastAdmin` when no other admin would remain; `NotAMember`, `NotAdmin`, `Offline`,
    /// `Ended`, `Crypto` as [`curvyo_sync_core::ClientSession::set_role`].
    pub fn set_participant_role(
        &mut self,
        member_hex: &str,
        admin: bool,
    ) -> Result<(), SessionError> {
        let member = member_of(member_hex)?;
        let role = if admin { Role::Admin } else { Role::Member };
        let link = self.sync.as_mut().ok_or(SessionError::Ended)?;
        link.client.set_role(member, role)?;
        link.revision = link.revision.wrapping_add(1);
        self.sync_pump();
        Ok(())
    }

    /// Change keys now (criterion 12): a `NewEpoch` for the remaining participants and a fresh
    /// snapshot under it. Admin only.
    ///
    /// # Errors
    /// As [`curvyo_sync_core::ClientSession::rotate`].
    pub fn rotate_share_keys(&mut self) -> Result<(), SessionError> {
        let link = self.sync.as_mut().ok_or(SessionError::Ended)?;
        link.client.rotate()?;
        link.revision = link.revision.wrapping_add(1);
        self.sync_pump();
        Ok(())
    }

    /// Adds the former participant `member_hex` back as a member (criteria 17 to 19, 23): a
    /// document with every epoch key this device holds (`full_history`) or the current one and
    /// a fresh snapshot; a workspace always with every key (WS-HISTORY B). Admin only.
    ///
    /// # Errors
    /// As [`curvyo_sync_core::ClientSession::add_back`].
    pub fn add_back_participant(
        &mut self,
        member_hex: &str,
        full_history: bool,
    ) -> Result<(), SessionError> {
        let member = member_of(member_hex)?;
        let link = self.sync.as_mut().ok_or(SessionError::Ended)?;
        let history = if full_history || link.workspace.is_some() {
            History::Full
        } else {
            History::FromNow
        };
        link.client.add_back(member, history)?;
        link.adding_back.insert(member);
        link.revision = link.revision.wrapping_add(1);
        self.sync_pump();
        Ok(())
    }

    /// Leave (criteria 10, 13 and 15): this device writes its own `RemoveMember` and keeps
    /// syncing, shown as "Leaving...", until the relay confirms it or refuses this device as
    /// removed (ADR 0041 decisions 2 and 11); then a document keeps its ended record "left" in
    /// place of its share record, a workspace hands the host the marker's
    /// ([`Session::workspace_ended_record`]). The room's keys stay in the credential store.
    /// Refused while this device is the only admin. `name` is the document's as the leaver's
    /// notice says it ("" for "this document").
    ///
    /// # Errors
    /// `LastAdmin`, `NotAMember`, `Offline`, `Ended`, `Crypto` as
    /// [`curvyo_sync_core::ClientSession::leave`].
    pub fn leave_share(&mut self, name: &str) -> Result<(), SessionError> {
        let link = self
            .sync
            .as_mut()
            .filter(|l| l.ended.is_none())
            .ok_or(SessionError::Ended)?;
        link.client.leave()?;
        link.leaving = true;
        name.trim().clone_into(&mut link.leave_name);
        link.revision = link.revision.wrapping_add(1);
        self.sync_pump();
        Ok(())
    }

    /// The share of this link ended by removal or leave: a document keeps the ended record in
    /// place of `share.json` and `keyring.log`, a workspace's link keeps the marker's for the
    /// host (ADR 0041 decision 6). A joiner that never received the document keeps nothing.
    /// `name` names the document in a leave's notice ("" for "this document").
    pub(super) fn end_share(&mut self, reason: EndReason, name: &str) {
        // The cursor counts the records already opened: they go into the document first.
        if self.sync.as_ref().is_some_and(|l| l.workspace.is_none()) {
            self.sync_apply_remote();
        }
        let Some(link) = self.sync.as_mut() else {
            return;
        };
        link.online = false;
        link.peers.clear();
        link.ended = Some(match reason {
            EndReason::Removed => SyncEnd::Removed,
            EndReason::Left => SyncEnd::Left,
        });
        let record = ended_record(link, reason);
        if link.workspace.is_none() {
            let text = match reason {
                EndReason::Removed => REMOVED_NOTICE.to_owned(),
                EndReason::Left if name.trim().is_empty() => LEFT_NOTICE.to_owned(),
                EndReason::Left => curvyo_ui_core::membership_text::left_notice(name.trim(), false),
            };
            link.notice(text, reason == EndReason::Removed);
            let joined = !link.awaiting_document;
            self.document.set_ended_share(joined.then_some(record));
        } else {
            let text = match (reason, link.workspace.as_ref()) {
                // `specs/0097` ux-notes 5.2: a leave says what stays, not "removed".
                (EndReason::Left, Some(channel)) => {
                    curvyo_ui_core::membership_text::left_notice(&channel.name, true)
                }
                _ => curvyo_ui_core::workspace_tree_text::WORKSPACE_REMOVED_TEXT.to_owned(),
            };
            link.notice(text, reason == EndReason::Removed);
            // The open document keeps its `share.json` v2 like every document of the folder
            // (ADR 0041 decision 6): it is local while the marker holds the ended record.
            link.workspace_ended = Some(record);
        }
    }

    /// The ended record of a shared workspace this link carried, for the marker's
    /// `share.ended.json` (ADR 0041 decision 6); `None` while the share goes on.
    #[must_use]
    pub fn workspace_ended_record(&self) -> Option<EndedRecord> {
        self.sync.as_ref()?.workspace_ended.clone()
    }

    /// The member id (hex) that signed this device's removal, read from the replayed keyring;
    /// `None` when the keyring holds none (`specs/0097` criterion 20: the host tells a switch-off
    /// by an own device from a removal by another maker's admin).
    #[must_use]
    pub fn share_removed_by(&self) -> Option<String> {
        let link = self.sync.as_ref()?;
        let me = link.client.me();
        link.client
            .keyring()?
            .accepted_entries()
            .iter()
            .rev()
            .find_map(|entry| match &entry.payload {
                Payload::RemoveMember { member } if *member == me => Some(hex(&entry.author.0)),
                _ => None,
            })
    }

    /// The labels other participants announced, by member id as hex, for the host to keep
    /// between runs (adrs.md decision 5: cached per device, never in the document).
    #[must_use]
    pub fn share_labels(&self) -> BTreeMap<String, String> {
        self.sync.as_ref().map_or_else(BTreeMap::new, |link| {
            link.labels
                .iter()
                .map(|(member, label)| (hex(&member.0), label.clone()))
                .collect()
        })
    }

    /// Labels kept from earlier runs; a label announced in this run wins. Unknown ids and
    /// malformed entries are ignored.
    pub fn remember_share_labels(&mut self, labels: &BTreeMap<String, String>) {
        let Some(link) = self.sync.as_mut() else {
            return;
        };
        for (member_hex, label) in labels {
            let label = label.trim();
            if let Some(bytes) = unhex::<32>(member_hex)
                && !label.is_empty()
            {
                link.labels
                    .entry(MemberId(bytes))
                    .or_insert_with(|| label.to_owned());
            }
        }
        link.revision = link.revision.wrapping_add(1);
    }
}

/// Where this link synced and how far, as its ended record: a document's room and cursor, or a
/// workspace's id and the marker's cursor and list version.
fn ended_record(link: &SyncLink, ended: EndReason) -> EndedRecord {
    // Only entries the relay confirmed: an own entry it never stored is no head (ADR 0041
    // decision 11).
    let heads = link
        .client
        .confirmed_heads()
        .into_iter()
        .map(|h| h.0)
        .collect();
    match link.workspace.as_ref() {
        None => EndedRecord {
            server: link.server.clone(),
            room: link.client.room().0,
            server_seq: link.client.cursor(),
            synced_version: link.synced.clone(),
            ended,
            heads,
        },
        Some(channel) => EndedRecord {
            server: link.server.clone(),
            room: channel.workspace,
            server_seq: channel.marker_cursor,
            synced_version: channel.list_synced.clone(),
            ended,
            heads,
        },
    }
}
