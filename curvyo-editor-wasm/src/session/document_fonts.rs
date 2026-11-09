//! The fonts a document carries (`specs/0107-document-fonts`, ADR 0047): the files its texts use
//! go into the asset store before a save or a room snapshot, and the stored files are listed as
//! the document block of faces so a text finds its face on a device that has not installed it.

use curvyo_document_core::{AssetHash, FontRef};
use curvyo_text_core::FaceKey;
use curvyo_ui_core::FontBlock;

use super::Session;
use super::text_fonts::ListedEntry;

/// The host id of an embedded font file: the prefix and the file's hash.
fn host_id(hash: &AssetHash) -> String {
    format!("doc:{}", hash.as_str())
}

/// The hash a [`host_id`] names.
fn hash_of_host_id(host_id: &str) -> Option<AssetHash> {
    AssetHash::parse(host_id.strip_prefix("doc:")?)
}

/// OS/2 `fsType` value 2: the font's licence forbids embedding (ADR 0047 decision 7).
const FS_TYPE_RESTRICTED: u16 = 0x0002;

impl Session {
    /// Puts the file of every font a text uses into the asset store, once per file. A font whose
    /// licence forbids embedding, one the session never held the bytes of and one that does not
    /// fit the project's asset budget stay out; the text then behaves as without this feature.
    pub(super) fn embed_used_fonts(&self) {
        for font in self.document.used_fonts() {
            let Some(hash) = AssetHash::parse(&font.face_hash) else {
                continue;
            };
            if self.document.asset(&hash).is_some() {
                continue;
            }
            let Some(bytes) = self.fonts.file_bytes(&font.face_hash) else {
                continue;
            };
            if self.embedding_forbidden(&font) {
                continue;
            }
            let _ = self.document.insert_asset(&bytes);
        }
    }

    fn embedding_forbidden(&self, font: &FontRef) -> bool {
        let Some(hash) = curvyo_text_core::FaceHash::from_hex(&font.face_hash) else {
            return false;
        };
        let key = FaceKey {
            hash,
            index: font.face_index,
        };
        self.fonts
            .face(&key)
            .and_then(|face| curvyo_text_core::font_info(face).fs_type)
            .is_some_and(|fs_type| fs_type & 0x000F == FS_TYPE_RESTRICTED)
    }

    /// Embeds what the texts use now and lists it: the Fonts tab calls this when it is shown
    /// (a font used since the last save appears under Document Fonts at once).
    pub fn sync_document_fonts(&mut self) {
        self.embed_used_fonts();
        self.refresh_document_fonts();
    }

    /// Lists the embedded files the document's texts use as the document block of faces and
    /// parses the ones not parsed yet. Runs on open, when a record arrives and from
    /// [`Session::sync_document_fonts`], never per frame.
    pub(super) fn refresh_document_fonts(&mut self) {
        let mut entries: Vec<ListedEntry> = Vec::new();
        for font in self.document.used_fonts() {
            let Some(hash) = AssetHash::parse(&font.face_hash) else {
                continue;
            };
            if self.document.asset(&hash).is_none() {
                continue;
            }
            entries.push((
                host_id(&hash),
                font.family,
                font.style,
                font.face_index,
                font.kind,
            ));
        }
        let ids: Vec<String> = {
            let mut ids: Vec<String> = entries.iter().map(|entry| entry.0.clone()).collect();
            ids.dedup();
            ids
        };
        self.fonts
            .set_block(FontBlock::Document, entries, Vec::new());
        for id in ids {
            if !self.fonts.needs_file(&id) {
                continue;
            }
            if let Some(bytes) = hash_of_host_id(&id).and_then(|hash| self.document.asset(&hash)) {
                self.fonts.add_file(&id, &bytes);
            }
        }
    }

    /// The file of the embedded face listed under `host_id`, for installing it.
    #[must_use]
    pub fn document_font_file(&self, host_id: &str) -> Option<Vec<u8>> {
        let hash = hash_of_host_id(host_id)?;
        self.document.asset(&hash).map(|bytes| bytes.to_vec())
    }
}
