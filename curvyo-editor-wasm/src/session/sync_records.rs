//! `Session`'s record traffic with its room: local commits exported as updates (images first),
//! remote records imported at the start of a frame, and the room's snapshots adopted or merged.

use std::collections::BTreeSet;

use curvyo_document_core::{AssetHash, DocumentVersion, ObjectSnapshot};
use curvyo_sync_core::{RecordKind, SessionError};

use super::Session;

impl Session {
    /// Pushes the local commits since the last export as one update, after the bytes of any
    /// image they brought in (so a receiver never meets the reference before its picture).
    pub(super) fn sync_export(&mut self) {
        let Some(link) = self.sync.as_ref() else {
            return;
        };
        if link.ended.is_some() {
            return;
        }
        let version = self.document.version();
        if version == link.exported {
            return;
        }
        if link.workspace.as_ref().is_some_and(|c| c.stream.is_none()) {
            // A document outside the shared workspace sends nothing on its room.
            return;
        }
        if self.document.has_extended_path_history() {
            if let Some(link) = self.sync.as_mut() {
                link.notice(
                    "Documents with editable curves or width profiles cannot be shared yet."
                        .to_owned(),
                    true,
                );
            }
            return;
        }
        let Some(link) = self.sync.as_ref() else {
            return;
        };
        let Ok(bytes) = self.document.export_updates(&link.exported) else {
            return;
        };
        let Some(bytes) = self.workspace_payload(&bytes) else {
            return;
        };
        // Pictures are only scanned for when the store grew: reading every object per commit
        // would cost a 1,000-object document its frame budget (criterion 13).
        self.embed_used_fonts();
        let asset_total = self.document.asset_bytes();
        let new_assets: Vec<AssetHash> = if asset_total == link.asset_total {
            Vec::new()
        } else {
            self.referenced_assets()
                .into_iter()
                .filter(|hash| !link.assets.contains(hash))
                .collect()
        };
        let pictures: Vec<(AssetHash, Vec<u8>)> = new_assets
            .into_iter()
            .filter_map(|hash| {
                let bytes = self.document.asset(&hash)?.to_vec();
                Some((hash, bytes))
            })
            .collect();
        let Some(link) = self.sync.as_mut() else {
            return;
        };
        link.asset_total = asset_total;
        for (hash, picture) in pictures {
            let picture = match link.workspace.as_ref().and_then(|c| c.stream) {
                Some(stream) => curvyo_sync_core::encode_inner_record(stream, self.peer, &picture),
                None => picture,
            };
            if link.client.push(RecordKind::Asset, picture).is_ok() {
                link.assets.insert(hash);
            }
        }
        match link.client.push(RecordKind::Update, bytes) {
            Ok(client_id) => {
                link.pushed.insert(client_id, version.clone());
                link.exported = version;
            }
            Err(SessionError::TooLarge) => {
                link.exported = version;
                link.notice("This change is too large to send.".to_owned(), true);
            }
            Err(_) => {}
        }
        self.sync_pump();
    }

    /// Imports every waiting remote record in order; `true` when the document changed.
    pub(super) fn sync_apply_remote(&mut self) -> bool {
        let Some(link) = self.sync.as_mut() else {
            return false;
        };
        if link.remote.is_empty() {
            return false;
        }
        let records: Vec<(u64, RecordKind, Vec<u8>)> = link.remote.drain(..).collect();
        let mut changed = false;
        let mut rejected: Option<u64> = None;
        for (seq, kind, bytes) in records {
            match self.apply_record(kind, &bytes) {
                Some(applied) => changed |= applied,
                None => rejected = rejected.or(Some(seq)),
            }
        }
        if let Some(seq) = rejected {
            self.record_rejected(seq);
        }
        if changed {
            self.refresh_document_fonts();
        }
        if changed {
            self.drop_gone_from_selection();
        }
        if let Some(link) = self.sync.as_mut() {
            link.exported = self.document.version();
            link.changed |= changed;
            if changed {
                link.revision = link.revision.wrapping_add(1);
            }
        }
        changed
    }

    /// Applies one remote record: `Some(true)` when the document changed, `None` when the record
    /// did not import and is rejected (criterion 25).
    fn apply_record(&mut self, kind: RecordKind, bytes: &[u8]) -> Option<bool> {
        match kind {
            RecordKind::Update => self
                .document
                .import_shared_updates(bytes)
                .ok()
                .map(|()| true),
            RecordKind::Asset => {
                let Ok(hash) = self.document.insert_asset(bytes) else {
                    return Some(false);
                };
                if let Some(link) = self.sync.as_mut() {
                    link.assets.insert(hash);
                }
                Some(true)
            }
            RecordKind::Snapshot => self.merge_snapshot(bytes).then_some(true),
        }
    }

    /// What a remote delete took from the selection leaves it (lazy resolve, ADR 0009 §2).
    fn drop_gone_from_selection(&mut self) {
        let ids = self.selection.ids();
        if ids.iter().all(|id| self.document.object(*id).is_some()) {
            return;
        }
        let kept: Vec<_> = ids
            .iter()
            .copied()
            .filter(|id| self.document.object(*id).is_some())
            .collect();
        self.selection.set(&kept);
    }

    /// A member's snapshot (its cursor lay before the room's snapshot, or it resyncs) merges
    /// into the replica. `false` when it does not open or merge.
    pub(super) fn merge_snapshot(&mut self, bytes: &[u8]) -> bool {
        let Ok(received) = curvyo_document_core::unpack(self.peer.wrapping_add(1), bytes) else {
            return false;
        };
        if received.has_extended_path_history() {
            return false;
        }
        let Ok(all) = received.export_updates(&DocumentVersion::never()) else {
            return false;
        };
        for hash in received_assets(&received) {
            if let Some(picture) = received.asset(&hash) {
                let _ = self.document.insert_asset(&picture);
            }
        }
        self.document.import_updates(&all).is_ok()
    }

    /// The assets the document's images reference.
    pub(super) fn referenced_assets(&self) -> BTreeSet<AssetHash> {
        received_assets(&self.document)
    }
}

fn received_assets(document: &curvyo_document_core::Document) -> BTreeSet<AssetHash> {
    document
        .read_objects()
        .objects
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Image(image) => Some(image.asset.clone()),
            _ => None,
        })
        .chain(document.used_font_hashes())
        .collect()
}
