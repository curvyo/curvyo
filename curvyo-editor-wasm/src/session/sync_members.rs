//! `Session`'s participant commands of the Share tab (`specs/0092` criteria 32 to 34): removal
//! with key rotation, and the device's cache of the labels other participants announced.

use std::collections::BTreeMap;

use curvyo_crypto_core::MemberId;
use curvyo_sync_core::SessionError;

use super::Session;
use super::sync::{hex, unhex};

impl Session {
    /// Removes the participant `member_hex` (its member id as hex) with a key rotation: the
    /// client writes `RemoveMember` and `NewEpoch` and the session uploads a snapshot sealed
    /// under the new epoch (criterion 32, ADR 0031 decision 6a). Admin only, while connected.
    ///
    /// # Errors
    /// `NotAMember` for an unknown id or this device; `NotAdmin`, `Offline`, `Ended`, `Crypto`
    /// as [`curvyo_sync_core::ClientSession::remove`].
    pub fn remove_participant(&mut self, member_hex: &str) -> Result<(), SessionError> {
        let member = unhex::<32>(member_hex)
            .map(MemberId)
            .ok_or(SessionError::NotAMember)?;
        let link = self.sync.as_mut().ok_or(SessionError::Ended)?;
        link.client.remove(member)?;
        link.labels.remove(&member);
        link.revision = link.revision.wrapping_add(1);
        self.sync_pump();
        Ok(())
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
