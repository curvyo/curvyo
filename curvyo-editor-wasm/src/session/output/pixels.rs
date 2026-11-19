//! The working images the job input reads (`specs/0123` criteria 7, 36; ADR 0068 §1): taken from
//! the canvas's cache when it holds them, else decoded once, kept while a leaf of the page names
//! them, `None` for bytes that cannot be read.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use curvyo_document_core::{AssetHash, Document};
use curvyo_image_core::WorkingImage;

/// The working images of the output's image leaves, by asset.
#[derive(Debug, Default)]
pub(super) struct PixelCache(RefCell<HashMap<AssetHash, Option<Arc<WorkingImage>>>>);

impl PixelCache {
    /// The working image of `hash`: the cached one, the canvas's (`decoded`), or decoded from
    /// `document`'s asset; `None` when its bytes are missing, damaged or do not hash to its name.
    pub(super) fn working(
        &self,
        document: &Document,
        hash: &AssetHash,
        decoded: impl FnOnce() -> Option<Arc<WorkingImage>>,
    ) -> Option<Arc<WorkingImage>> {
        if let Some(held) = self.0.borrow().get(hash) {
            return held.clone();
        }
        let working = decoded().or_else(|| {
            document
                .asset(hash)
                .filter(|bytes| AssetHash::of(bytes) == *hash)
                .and_then(|bytes| curvyo_image_core::decode_working(&bytes).ok())
                .map(Arc::new)
        });
        self.0.borrow_mut().insert(hash.clone(), working.clone());
        working
    }

    /// The working image of `hash` when it is decoded already; no decode.
    pub(super) fn held(&self, hash: &AssetHash) -> Option<Arc<WorkingImage>> {
        self.0.borrow().get(hash).cloned().flatten()
    }

    /// Drops the images no asset of `alive` names; an unreadable one stays held as such.
    pub(super) fn retain(&self, alive: &HashSet<&AssetHash>) {
        self.0.borrow_mut().retain(|hash, _| alive.contains(hash));
    }

    /// How many images are held, for the tests.
    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.0.borrow().len()
    }
}
