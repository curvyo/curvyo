//! The only place that checks the document out to an earlier version (ADR 0014 §12).
//!
//! A checkout detaches the document: every writer panics until it is attached again. [`Detached`]
//! is the guard that attaches it again when it is dropped, also while a panic unwinds, and it hands
//! out reads only, as plain data. Nothing that holds a [`Detached`] may read
//! [`crate::Document::version`] or a cache keyed by it.

use std::collections::BTreeMap;

use loro::{FractionalIndex, Frontiers, LoroDoc, TreeID, TreeParentId};

use super::error::HistoryError;
use super::node_value::{Node, read_map};
use crate::document::{Document, OBJECTS_TREE, ROOT_MAP};
use crate::document_background::{DocumentBackground, background_of_root};
use crate::path_model::NodeId;
use crate::primitive_model::ObjectSnapshot;
use crate::shapes::object_in;
use crate::units::DocumentSize;

/// One tree node as it was at a version.
#[derive(Debug, Clone)]
pub(crate) struct NodeAt {
    /// Its meta map; empty when the node does not exist.
    pub meta: BTreeMap<String, Node>,
    /// Its position among its siblings.
    pub index: Option<FractionalIndex>,
    /// The group that holds it; `None` at the document root and for a node that does not exist.
    pub parent: Option<TreeID>,
}

/// The document checked out to a version. Dropping it checks the document out to the latest
/// version again.
pub(crate) struct Detached<'a> {
    doc: &'a LoroDoc,
}

impl<'a> Detached<'a> {
    /// Commits what is pending and checks the document out to `version`.
    pub(crate) fn at(doc: &'a LoroDoc, version: &Frontiers) -> Result<Self, HistoryError> {
        doc.commit();
        let guard = Self { doc };
        doc.checkout(version)
            .map_err(|_| HistoryError::Unreadable)?;
        Ok(guard)
    }

    /// Checks the document out to another version.
    pub(crate) fn goto(&self, version: &Frontiers) -> Result<(), HistoryError> {
        self.doc
            .checkout(version)
            .map_err(|_| HistoryError::Unreadable)
    }

    /// The node `id` at this version.
    pub(crate) fn node(&self, id: TreeID) -> NodeAt {
        let tree = self.doc.get_tree(OBJECTS_TREE);
        let alive = tree.contains(id) && matches!(tree.is_node_deleted(&id), Ok(false));
        if !alive {
            return NodeAt {
                meta: BTreeMap::new(),
                index: None,
                parent: None,
            };
        }
        NodeAt {
            meta: tree
                .get_meta(id)
                .map(|meta| read_map(&meta))
                .unwrap_or_default(),
            index: tree
                .fractional_index(id)
                .map(FractionalIndex::from_hex_string),
            parent: match tree.parent(id) {
                Some(TreeParentId::Node(parent)) => Some(parent),
                _ => None,
            },
        }
    }

    /// The root map at this version.
    pub(crate) fn root(&self) -> BTreeMap<String, Node> {
        read_map(&self.doc.get_map(ROOT_MAP))
    }

    /// Every object at this version, in paint order, as plain copies
    /// (`specs/0042-history-branches` criterion 19).
    pub(crate) fn objects(&self) -> Vec<ObjectSnapshot> {
        let tree = self.doc.get_tree(OBJECTS_TREE);
        crate::group_tree::leaves_under(&tree, None)
            .into_iter()
            .filter_map(|id| object_in(&tree, NodeId::from_parts(id.peer, id.counter)))
            .collect()
    }

    /// The object `id` at this version, as a plain copy; `None` when it does not exist.
    pub(crate) fn object(&self, id: NodeId) -> Option<ObjectSnapshot> {
        object_in(&self.doc.get_tree(OBJECTS_TREE), id)
    }

    /// The page at this version: its size and background.
    pub(crate) fn page(&self) -> (DocumentSize, DocumentBackground) {
        let root = self.doc.get_map(ROOT_MAP);
        (Document::size_of_root(&root), background_of_root(&root))
    }

    /// The children of `parent` (`None` is the document root) in z-order at this version.
    pub(crate) fn children(&self, parent: Option<TreeID>) -> Vec<TreeID> {
        self.doc
            .get_tree(OBJECTS_TREE)
            .children(parent.map_or(TreeParentId::Root, TreeParentId::Node))
            .unwrap_or_default()
    }
}

impl Drop for Detached<'_> {
    fn drop(&mut self) {
        self.doc.checkout_to_latest();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_document_is_attached_again_when_a_read_panics() {
        let doc = LoroDoc::new();
        doc.get_map("m").insert("a", 1).unwrap();
        doc.commit();
        let first = doc.oplog_frontiers();
        doc.get_map("m").insert("a", 2).unwrap();
        doc.commit();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = Detached::at(&doc, &first).unwrap();
            assert!(doc.is_detached());
            panic!("a read failed");
        }));
        assert!(outcome.is_err());
        assert!(!doc.is_detached(), "the guard attached the document again");
        assert_eq!(
            doc.get_map("m").get("a").unwrap().get_deep_value(),
            2.into()
        );
    }
}
