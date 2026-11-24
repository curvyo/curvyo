//! A fragment: pages or objects taken out of a read-only source document as an owned value, to
//! be inserted as plain copies into another document (ADR 0084 §6, `specs/0137` criteria 4, 5
//! and 8).
//!
//! The fragment is a private copy of the source with everything that was not taken deleted, so
//! every reference into the rest follows the rule of its own removal (a linked clone whose
//! original was not taken becomes an ordinary path, ADR 0049; a text on a carrier that was not
//! taken keeps its outline, ADR 0022). It is never written to disk and never committed into the
//! source.

use std::collections::HashSet;
use std::sync::Arc;

use loro::{LoroTree, LoroValue, TreeID, TreeParentId};

use crate::assets::AssetHash;
use crate::document::{Document, OBJECTS_TREE};
use crate::page_kind::PageKind;
use crate::pages::Page;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;

/// Why a fragment cannot be taken.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FragmentError {
    /// An object or the page does not exist in the source.
    #[error("the source has no such object or page")]
    NotFound,
    /// Nothing was named.
    #[error("nothing to import")]
    Empty,
    /// The source could not be copied.
    #[error("the source cannot be read")]
    Unreadable,
}

/// What a fragment's insert writes; built by [`Document::extract_fragment`].
pub struct Fragment {
    /// The pruned private copy of the source.
    pub(crate) source: Document,
    /// The top-level nodes taken, in paint order.
    pub(crate) roots: Vec<TreeID>,
    /// The source page, for a page import.
    pub(crate) page: Option<Page>,
    /// The asset bytes the roots refer to, with whether each is a derived result.
    pub(crate) assets: Vec<(AssetHash, Arc<[u8]>, bool)>,
    /// How many linked clones became paths because their originals were not taken.
    pub(crate) clones_became_paths: usize,
    /// How many texts on a path left their carrier behind and were released from it.
    pub(crate) texts_released: usize,
}

impl Fragment {
    /// The number of fresh anchor ids an insert takes, one per anchor under the roots.
    #[must_use]
    pub fn anchor_count(&self) -> usize {
        let tree = self.source.loro().get_tree(OBJECTS_TREE);
        self.roots
            .iter()
            .map(|&root| crate::group_tree::anchors_under(&tree, root))
            .sum()
    }

    /// The top-level objects taken, in paint order, as ids of the fragment's own read
    /// ([`Fragment::source`]).
    #[must_use]
    pub fn roots(&self) -> Vec<NodeId> {
        self.roots
            .iter()
            .map(|id| NodeId::from_parts(id.peer, id.counter))
            .collect()
    }

    /// The private copy of the source the fragment holds, read only: the caller measures the
    /// roots in it (the box an element import centres on the drop point).
    #[must_use]
    pub const fn source(&self) -> &Document {
        &self.source
    }

    /// The source page of a page import.
    #[must_use]
    pub const fn page(&self) -> Option<&Page> {
        self.page.as_ref()
    }

    /// How many linked clones became ordinary paths because their originals were not taken (the
    /// notice of `specs/0137` criterion 5).
    #[must_use]
    pub const fn clones_became_paths(&self) -> usize {
        self.clones_became_paths
    }

    /// How many texts on a path were released from it because their carrier was not taken
    /// (they keep their straight placement, the `0080` Release; no reference into the source).
    #[must_use]
    pub const fn texts_released(&self) -> usize {
        self.texts_released
    }
}

impl Document {
    /// Takes the top-level objects `roots` (and, with `page`, that page with its registers and,
    /// for an output page, its manufacturing) out of this document as a [`Fragment`]. This
    /// document is not changed. `roots` are top-level objects (children of a layer or of the
    /// root); for a page import the caller passes the page's members by the membership read.
    ///
    /// # Errors
    /// [`FragmentError::NotFound`] for an object or page that does not exist;
    /// [`FragmentError::Empty`] when neither an object nor a page is named;
    /// [`FragmentError::Unreadable`] if the private copy cannot be made.
    pub fn extract_fragment(
        &self,
        roots: &[NodeId],
        page: Option<&str>,
    ) -> Result<Fragment, FragmentError> {
        self.extract_fragment_placing(roots, page, &[])
    }

    /// [`Document::extract_fragment`] where each placed page among what is taken comes in as
    /// ordinary copies of its drawing page's members at its matrix (ADR 0084 decision 6):
    /// `placed` names, per placement, those members by the membership read (`curvyo-ui-core`'s,
    /// the caller's). A placement not named comes in as nothing; its drawing page is not taken.
    ///
    /// # Errors
    /// As [`Document::extract_fragment`].
    pub fn extract_fragment_placing(
        &self,
        roots: &[NodeId],
        page: Option<&str>,
        placed: &[(NodeId, Vec<NodeId>)],
    ) -> Result<Fragment, FragmentError> {
        let page = match page {
            Some(id) => Some(
                self.pages()
                    .into_iter()
                    .find(|p| p.id == id)
                    .ok_or(FragmentError::NotFound)?,
            ),
            None => None,
        };
        if roots.is_empty() && page.is_none() {
            return Err(FragmentError::Empty);
        }
        let mut wanted = self.wanted_nodes(roots, page.as_ref())?;
        let source = self.private_copy()?;
        let before = all_nodes(&source.loro().get_tree(OBJECTS_TREE));
        source.bake_placements(&mut wanted, placed);
        let clones_became_paths = source.prune_to(&wanted);
        let roots = source.live_roots(&wanted, &before);
        let texts_released = source.release_orphan_texts(&roots);
        let assets = source.assets_of(&roots);
        Ok(Fragment {
            source,
            roots,
            page,
            assets,
            clones_became_paths,
            texts_released,
        })
    }

    /// The nodes to take: `roots`, and an output page's manufacturing.
    fn wanted_nodes(
        &self,
        roots: &[NodeId],
        page: Option<&crate::pages::Page>,
    ) -> Result<Vec<TreeID>, FragmentError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut wanted: Vec<TreeID> = Vec::with_capacity(roots.len() + 1);
        for &root in roots {
            let id = tree_id_of(root);
            if !crate::path_codec::node_exists(&tree, id)
                || crate::group_tree::is_layer_node(&tree, id)
            {
                return Err(FragmentError::NotFound);
            }
            if !wanted.contains(&id) {
                wanted.push(id);
            }
        }
        if let Some(PageKind::Output { manufacturing, .. }) = page.map(|p| &p.kind) {
            let id = tree_id_of(*manufacturing);
            if crate::path_codec::node_exists(&tree, id) && !wanted.contains(&id) {
                wanted.push(id);
            }
        }
        Ok(wanted)
    }

    /// A copy of this document that shares nothing with it.
    fn private_copy(&self) -> Result<Self, FragmentError> {
        let bytes = self
            .export_loro_snapshot()
            .map_err(|_| FragmentError::Unreadable)?;
        let copy = Self::from_loro_snapshot(self.loro().peer_id(), &bytes)
            .map_err(|_| FragmentError::Unreadable)?;
        *copy.assets.borrow_mut() = self.assets.borrow().clone();
        Ok(copy)
    }

    /// Deletes every node that is neither under one of `keep` nor an ancestor of one, with the
    /// repair of each removal (ADR 0049, 0022), in one commit of this private copy. Layers stay.
    /// Returns how many linked clones became paths.
    fn prune_to(&self, keep: &[TreeID]) -> usize {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let keep: HashSet<TreeID> = keep.iter().copied().collect();
        let mut ancestors: HashSet<TreeID> = HashSet::new();
        for &id in &keep {
            let mut current = crate::manufacturing_codec::parent_node(&tree, id);
            while let Some(parent) = current {
                if !ancestors.insert(parent) {
                    break;
                }
                current = crate::manufacturing_codec::parent_node(&tree, parent);
            }
        }
        let mut doomed: Vec<TreeID> = Vec::new();
        let mut stack: Vec<TreeParentId> = vec![TreeParentId::Root];
        while let Some(parent) = stack.pop() {
            for child in tree.children(parent).unwrap_or_default() {
                if keep.contains(&child) {
                    continue;
                }
                if ancestors.contains(&child) || crate::group_tree::is_layer_node(&tree, child) {
                    stack.push(TreeParentId::Node(child));
                } else {
                    doomed.push(child);
                }
            }
        }
        if doomed.is_empty() {
            return 0;
        }
        let doomed_set: HashSet<TreeID> = doomed.iter().copied().collect();
        for id in self.object_ids() {
            if is_under(&tree, tree_id_of(id), &doomed_set) {
                self.clear_sketch_claim(id);
            }
        }
        let _ = self.take_clone_repair();
        self.repair_clones_of_removed(&tree, &doomed);
        let parents = crate::group_tree::parents_of(&tree, &doomed);
        for id in doomed {
            // invariant: every doomed node was just read as a child of a live node.
            #[allow(clippy::unwrap_used)]
            tree.delete(id).unwrap();
        }
        crate::groups::remove_emptied_groups(&tree, &parents);
        self.commit_with_label("delete_objects");
        self.take_clone_repair().unlinked
    }

    /// Releases every text under `roots` whose carrier is not in this private copy any more from
    /// its path (every on-path key deleted, the straight placement kept, as `0080`'s Release), so
    /// no copy names a node of the source. Returns how many; committed in this copy.
    fn release_orphan_texts(&self, roots: &[TreeID]) -> usize {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut released = 0;
        for &root in roots {
            let mut nodes = vec![root];
            nodes.extend(crate::group_tree::leaves_under(&tree, Some(root)));
            for node in nodes {
                let Ok(meta) = tree.get_meta(node) else {
                    continue;
                };
                let Some(carrier) = crate::text_path_codec::read_carrier(&meta) else {
                    continue;
                };
                if crate::path_codec::node_exists(&tree, tree_id_of(carrier.node)) {
                    continue;
                }
                if crate::text_path_codec::delete_all(&meta) {
                    released += 1;
                }
            }
        }
        if released > 0 {
            self.commit_with_label(crate::RELEASE_FROM_PATH_LABEL);
        }
        released
    }

    /// The roots after the prune, in paint order: each of `wanted` still alive, and each node the
    /// repair made in place of one (a clone of a group becomes a copy of the group), which is
    /// not in `before`.
    fn live_roots(&self, wanted: &[TreeID], before: &HashSet<TreeID>) -> Vec<TreeID> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut roots = Vec::new();
        walk(&tree, TreeParentId::Root, &mut |node| {
            if wanted.contains(&node) || !before.contains(&node) {
                roots.push(node);
                return false;
            }
            true
        });
        roots
    }

    /// The assets the nodes under `roots` refer to by hash, read from this document's store.
    fn assets_of(&self, roots: &[TreeID]) -> Vec<(AssetHash, Arc<[u8]>, bool)> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut hashes: HashSet<AssetHash> = HashSet::new();
        for &root in roots {
            collect_hashes(&tree, root, &mut hashes);
        }
        let store = self.assets.borrow();
        let mut found: Vec<_> = hashes
            .into_iter()
            .filter_map(|hash| {
                let bytes = store.get(&hash)?;
                let derived = store.has_derived(&hash);
                Some((hash, bytes, derived))
            })
            .collect();
        found.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
        found
    }
}

/// Walks the tree under `parent` depth first in paint order; `visit(node)` returns whether to
/// descend into the node.
fn walk(tree: &LoroTree, parent: TreeParentId, visit: &mut dyn FnMut(TreeID) -> bool) {
    for child in tree.children(parent).unwrap_or_default() {
        if visit(child) {
            walk(tree, TreeParentId::Node(child), visit);
        }
    }
}

/// Whether `node` or one of its ancestors is in `set`.
fn is_under(tree: &LoroTree, node: TreeID, set: &HashSet<TreeID>) -> bool {
    let mut current = Some(node);
    while let Some(id) = current {
        if set.contains(&id) {
            return true;
        }
        current = crate::manufacturing_codec::parent_node(tree, id);
    }
    false
}

/// Every node of the tree.
fn all_nodes(tree: &LoroTree) -> HashSet<TreeID> {
    let mut nodes = HashSet::new();
    walk(tree, TreeParentId::Root, &mut |node| nodes.insert(node));
    nodes
}

/// Adds every asset hash named anywhere in the meta maps under `node` (pictures, fonts, derived
/// results) to `hashes`.
fn collect_hashes(tree: &LoroTree, node: TreeID, hashes: &mut HashSet<AssetHash>) {
    if let Ok(meta) = tree.get_meta(node) {
        scan(&meta.get_deep_value(), hashes);
    }
    for child in tree.children(TreeParentId::Node(node)).unwrap_or_default() {
        collect_hashes(tree, child, hashes);
    }
}

fn scan(value: &LoroValue, hashes: &mut HashSet<AssetHash>) {
    match value {
        LoroValue::String(text) => {
            if let Some(hash) = AssetHash::parse(text) {
                hashes.insert(hash);
            }
        }
        LoroValue::List(items) => items.iter().for_each(|item| scan(item, hashes)),
        LoroValue::Map(map) => map.values().for_each(|item| scan(item, hashes)),
        _ => {}
    }
}
