//! The receiving side of the font blobs (ADR 0062 decisions 3 and 6): the parts of the group
//! room's `Asset` records and the blob entries of its snapshots, checked against the entries
//! that name them and the library budget before they are held.

use curvyo_document_core::AssetHash;
use curvyo_sync_core::{BlobAssembler, StreamEntry, decode_blob_entry};
use curvyo_ui_core::library_text as texts;

use super::DeviceGroupGlue;
use super::fonts::{Assembly, stem};

impl DeviceGroupGlue {
    /// One `Asset` record of the group room: a part of a font blob (ADR 0062 decision 3). A
    /// blob over the limit or the budget, out of order or not what its hash says is dropped
    /// with a notice.
    pub(super) fn receive_font_part(&mut self, part: &[u8]) {
        let Ok(first) = curvyo_sync_core::decode_blob_part(part) else {
            self.notices
                .push_back(texts::FONT_SKIPPED_NOTICE.to_owned());
            return;
        };
        let key = first.hash;
        let hash = AssetHash::from_bytes(&key);
        if self.fonts.blobs.contains_key(&hash) {
            // Held already (a second device pushed the same blob): nothing to collect.
            return;
        }
        let (done, assembly) = if first.index == 0 {
            // A restart of the same blob drops the earlier try before it is counted, but not
            // the time its first part came.
            let first_at = self
                .fonts
                .assembling
                .remove(&key)
                .map_or(self.now, |earlier| earlier.first_at);
            if !self.admit_blob(&hash, first.total) {
                return;
            }
            let Ok(mut assembler) = BlobAssembler::start(part) else {
                return;
            };
            let done = assembler.take();
            (
                done,
                Assembly {
                    assembler,
                    last_at: self.now,
                    first_at,
                },
            )
        } else if let Some(mut assembly) = self.fonts.assembling.remove(&key) {
            if first.index < assembly.assembler.next_index() {
                // A part seen already (two devices push the same blob interleaved).
                self.fonts.assembling.insert(key, assembly);
                return;
            }
            let Ok(done) = assembly.assembler.push(part) else {
                self.notices
                    .push_back(texts::FONT_SKIPPED_NOTICE.to_owned());
                return;
            };
            assembly.last_at = self.now;
            (done, assembly)
        } else {
            return;
        };
        if let Some(bytes) = done {
            self.store_blob(hash, bytes);
        } else {
            self.fonts.assembling.insert(key, assembly);
        }
    }

    /// One blob entry of a framed room snapshot.
    pub(super) fn receive_font_entry(&mut self, entry: &StreamEntry) {
        if let Some((hash, bytes)) = decode_blob_entry(entry) {
            self.store_blob(AssetHash::from_bytes(&hash), bytes.to_vec());
        }
    }

    /// Whether a blob announced as `total` bytes may start: the size is what the entry that
    /// names it says, and held plus arriving bytes stay within the library budget (blobs and
    /// half blobs no entry names make room first). A refusal gives a notice.
    fn admit_blob(&mut self, hash: &AssetHash, total: u64) -> bool {
        if self
            .referenced_sizes()
            .get(hash)
            .is_some_and(|size| *size != total)
        {
            self.notices
                .push_back(texts::FONT_SKIPPED_NOTICE.to_owned());
            return false;
        }
        if self.make_room(total) {
            return true;
        }
        let name = self
            .library
            .font_entry(hash)
            .and_then(|entry| entry.details)
            .map(|details| stem(&details.file_name).to_owned());
        let used = self.fonts_in_use();
        self.notices.push_back(name.map_or_else(
            || texts::FONT_SKIPPED_NOTICE.to_owned(),
            |name| texts::font_over_budget_notice(&name, used + total, self.fonts.limit),
        ));
        false
    }

    /// The bytes held and arriving.
    fn fonts_in_use(&self) -> u64 {
        let held: u64 = self.fonts.blobs.values().map(|b| b.len() as u64).sum();
        let arriving: u64 = self
            .fonts
            .assembling
            .values()
            .map(|a| a.assembler.total())
            .sum();
        held + arriving
    }

    /// Whether `incoming` more bytes fit the budget, dropping what no entry names if needed.
    pub(super) fn make_room(&mut self, incoming: u64) -> bool {
        if self.fonts_in_use() + incoming <= self.fonts.limit {
            return true;
        }
        let keep = self.referenced();
        self.fonts.blobs.retain(|held, _| keep.contains(held));
        self.fonts
            .assembling
            .retain(|key, _| keep.contains(&AssetHash::from_bytes(key)));
        self.fonts_in_use() + incoming <= self.fonts.limit
    }

    fn store_blob(&mut self, hash: AssetHash, bytes: Vec<u8>) {
        let size_ok = self
            .referenced_sizes()
            .get(&hash)
            .is_none_or(|size| *size == bytes.len() as u64);
        if AssetHash::of(&bytes) != hash || !size_ok {
            self.notices
                .push_back(texts::FONT_SKIPPED_NOTICE.to_owned());
        } else if self.make_room(bytes.len() as u64) {
            self.fonts.blobs.insert(hash, bytes);
            self.retry_deferred_snapshot();
        } else {
            let name = self
                .library
                .font_entry(&hash)
                .and_then(|entry| entry.details)
                .map(|details| stem(&details.file_name).to_owned());
            let total = self.fonts_in_use() + bytes.len() as u64;
            self.notices.push_back(name.map_or_else(
                || texts::FONT_SKIPPED_NOTICE.to_owned(),
                |name| texts::font_over_budget_notice(&name, total, self.fonts.limit),
            ));
        }
        self.bump();
    }
}
