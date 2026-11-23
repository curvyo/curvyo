//! The stored AI results of a project as derived assets (ADR 0077 §1, §5, §6; `specs/0135`
//! criteria 48, 59, 66): added within their limits, counted, and pruned at save to what the tree
//! or this session refers to.

use std::collections::HashSet;
use std::sync::Arc;

use crate::assets::{AssetError, AssetHash, MAX_DERIVED_ASSET_BYTES, MAX_DERIVED_PROJECT_BYTES};
use crate::document::{Document, OBJECTS_TREE};

impl Document {
    /// Stores the PNG bytes of an AI step's result as a derived asset, without a history step
    /// (the step's `result` record is the edit); storing bytes already held changes nothing.
    ///
    /// # Errors
    /// [`AssetError::ResultTooLarge`] over 64 MiB; [`AssetError::ResultsFull`] when the stored
    /// results a step refers to would pass 512 MiB with this one. Nothing is stored.
    pub fn insert_derived_asset(&self, bytes: &[u8]) -> Result<AssetHash, AssetError> {
        let size = bytes.len() as u64;
        if size > MAX_DERIVED_ASSET_BYTES {
            return Err(AssetError::ResultTooLarge { bytes: size });
        }
        let hash = AssetHash::of(bytes);
        let referenced = self.referenced_derived_assets();
        let mut store = self.assets.borrow_mut();
        if store.has_derived(&hash) {
            store.mark_derived(&hash);
            return Ok(hash);
        }
        if store.derived_bytes_of(&referenced) + size > MAX_DERIVED_PROJECT_BYTES {
            return Err(AssetError::ResultsFull { adds: size });
        }
        store.insert_derived(hash.clone(), Arc::from(bytes));
        Ok(hash)
    }

    /// The bytes of the stored AI results a step refers to ("Stored results 120 MB of 512 MB",
    /// criterion 66); results kept only for undo do not count.
    #[must_use]
    pub fn derived_asset_bytes(&self) -> u64 {
        let referenced = self.referenced_derived_assets();
        self.assets.borrow().derived_bytes_of(&referenced)
    }

    /// Whether the stored AI result `hash` is held.
    #[must_use]
    pub fn has_derived_asset(&self, hash: &AssetHash) -> bool {
        self.assets.borrow().has_derived(hash)
    }

    /// The derived assets a live node of the tree names (an image's stack or a clone's own).
    #[must_use]
    pub fn referenced_derived_assets(&self) -> HashSet<AssetHash> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        tree.nodes()
            .into_iter()
            .filter(|node| crate::path_codec::node_exists(&tree, *node))
            .filter_map(|node| tree.get_meta(node).ok())
            .flat_map(|meta| crate::image_adjust_codec::derived_assets(&meta))
            .collect()
    }

    /// Marks the derived assets the tree refers to now as this session's (at open).
    pub(crate) fn mark_referenced_derived_assets(&self) {
        let referenced = self.referenced_derived_assets();
        let mut store = self.assets.borrow_mut();
        for hash in &referenced {
            store.mark_derived(hash);
        }
    }

    /// Drops the derived assets neither the tree nor this session refers to (ADR 0077 §5), before
    /// a save writes the store.
    pub(crate) fn prune_derived_assets(&self) {
        let referenced = self.referenced_derived_assets();
        self.assets.borrow_mut().prune_derived(&referenced);
    }
}
