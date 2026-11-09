//! `Session`'s participant commands of the Share tab (`specs/0092` criteria 32 to 34,
//! `specs/0097` criteria 2 to 5, 10 to 15, 17 to 19): roles, removal with key rotation, leave,
//! Change keys now, Stop sharing, and the device's cache of the labels other participants
//! announced.

use std::collections::BTreeMap;

use curvyo_crypto_core::MemberId;
use curvyo_crypto_core::keyring::{Payload, Role};
use curvyo_document_core::{EndReason, EndedRecord};
use curvyo_sync_core::SessionError;

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

    /// Leave (criteria 10, 13 and 15): this device writes its own `RemoveMember` and keeps
    /// syncing, shown as "Leaving...", until the relay confirms it or refuses this device as
    /// removed (ADR 0041 decisions 2 and 11); then the workspace hands the host the marker's
    /// ended record "left" ([`Session::workspace_ended_record`]). The room's keys stay in the
    /// credential store. Refused while this device is the only admin.
    ///
    /// # Errors
    /// `LastAdmin`, `NotAMember`, `Offline`, `Ended`, `Crypto` as
    /// [`curvyo_sync_core::ClientSession::leave`].
    pub fn leave_share(&mut self) -> Result<(), SessionError> {
        let link = self
            .sync
            .as_mut()
            .filter(|l| l.ended.is_none())
            .ok_or(SessionError::Ended)?;
        link.client.leave()?;
        link.leaving = true;
        link.revision = link.revision.wrapping_add(1);
        self.sync_pump();
        Ok(())
    }

    /// Stop sharing (customer 2026-11-09, STOP-SHARING by one by one): the only participant
    /// left ends the share here. Nothing is written, as nobody else would read it; the link
    /// closes and the document keeps no share record (the workspace's marker loses its share
    /// files on the host).
    ///
    /// # Errors
    /// `Ended` without a live link; `NotAdmin` while anyone else is still a participant.
    pub fn stop_sharing(&mut self) -> Result<(), SessionError> {
        let link = self
            .sync
            .as_mut()
            .filter(|l| l.ended.is_none())
            .ok_or(SessionError::Ended)?;
        let me = link.client.me();
        let alone = link
            .client
            .keyring()
            .is_some_and(|k| k.participants().iter().all(|p| p.identity.member == me));
        if !alone {
            return Err(SessionError::NotAdmin);
        }
        link.actions.clear();
        link.actions.push_back(super::sync::TransportAction::Close);
        link.ended = Some(SyncEnd::Refused);
        link.online = false;
        link.peers.clear();
        link.revision = link.revision.wrapping_add(1);
        // The open document is no stream of the room any more: its next save writes no record.
        if let Some(channel) = link.workspace.as_mut() {
            channel.switch_stream(None);
        }
        self.document.set_workspace_share(None);
        Ok(())
    }

    /// The share of this link ended by removal or leave: the workspace's link keeps the
    /// marker's ended record for the host (ADR 0041 decision 6); the open document keeps its
    /// `share.json` v2 like every document of the folder and is local while the marker holds it.
    pub(super) fn end_share(&mut self, reason: EndReason) {
        let Some(link) = self.sync.as_mut() else {
            return;
        };
        link.online = false;
        link.peers.clear();
        link.ended = Some(match reason {
            EndReason::Removed => SyncEnd::Removed,
            EndReason::Left => SyncEnd::Left,
        });
        let Some(record) = ended_record(link, reason) else {
            return;
        };
        let text = match (reason, link.workspace.as_ref()) {
            // `specs/0097` ux-notes 5.2: a leave says what stays, not "removed".
            (EndReason::Left, Some(channel)) => {
                curvyo_ui_core::membership_text::left_notice(&channel.name, true)
            }
            _ => curvyo_ui_core::workspace_tree_text::WORKSPACE_REMOVED_TEXT.to_owned(),
        };
        link.notice(text, reason == EndReason::Removed);
        link.workspace_ended = Some(record);
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

/// Where this workspace's link synced and how far, as its ended record: the workspace's id,
/// the marker's cursor and the list's version; `None` for a link of no workspace.
fn ended_record(link: &SyncLink, ended: EndReason) -> Option<EndedRecord> {
    let channel = link.workspace.as_ref()?;
    // Only entries the relay confirmed: an own entry it never stored is no head (ADR 0041
    // decision 11).
    let heads = link
        .client
        .confirmed_heads()
        .into_iter()
        .map(|h| h.0)
        .collect();
    Some(EndedRecord {
        server: link.server.clone(),
        room: channel.workspace,
        server_seq: channel.marker_cursor,
        synced_version: channel.list_synced.clone(),
        ended,
        heads,
    })
}
