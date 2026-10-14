//! The one read of every object out of the document that a frame and the events between two
//! frames share, kept for as long as the document version is the one it was read at
//! (`specs/0044-editing-quick-wins/adrs.md`, decision 5).

use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::Arc;

use curvyo_document_core::{Document, DocumentVersion, NodeId, NodeRead, ObjectSnapshot};

/// One read of the document: every node with its effective flags
/// (`specs/0050-layers-and-objects-panel/adrs.md` decision 3) and the lists the session derives
/// from it once, so no consumer filters or walks on its own (decision 4).
#[derive(Debug)]
pub(super) struct DocumentRead {
    /// Every node, pre-order, back to front: what the Layers list shows.
    pub(super) nodes: Arc<[NodeRead]>,
    /// Every leaf in stacking order, hidden and locked ones included: what the tools act on.
    pub(super) leaves: Arc<[ObjectSnapshot]>,
    /// The effectively visible leaves, unfaded: what the eyedropper samples and Fit to content
    /// measures (criterion 19).
    pub(super) visible: Arc<[ObjectSnapshot]>,
    /// The effectively visible leaves with their effective opacity applied to each paint: what
    /// the canvas draws (criteria 24 and 27).
    pub(super) drawn: Arc<[ObjectSnapshot]>,
    /// The nodes, leaves and containers, that are effectively hidden or locked.
    pub(super) inert: Arc<HashSet<NodeId>>,
    /// The front-most layer, if the document has one.
    pub(super) front_layer: Option<NodeId>,
    /// Every leaf with an index by id and the direct clones of each: what the panel of linked
    /// clones reads (`specs/0049-linked-clone` decision 3).
    pub(super) clones: Arc<curvyo_document_core::ObjectRead>,
}

impl DocumentRead {
    fn of(document: &Document) -> Self {
        let nodes: Arc<[NodeRead]> = document.read_nodes().into();
        let leaves: Arc<[ObjectSnapshot]> = nodes
            .iter()
            .filter_map(|read| read.object().cloned())
            .collect();
        let plain = nodes
            .iter()
            .all(|read| read.visible && read.opacity == curvyo_document_core::Opacity::OPAQUE);
        let (visible, drawn) = if plain {
            (Arc::clone(&leaves), Arc::clone(&leaves))
        } else {
            let visible: Arc<[ObjectSnapshot]> = nodes
                .iter()
                .filter(|read| read.visible)
                .filter_map(|read| read.object().cloned())
                .collect();
            (visible, curvyo_render_core::drawn_leaves(&nodes).into())
        };
        let inert: HashSet<NodeId> = nodes
            .iter()
            .filter(|read| !read.visible || read.locked)
            .map(|read| read.id)
            .collect();
        let front_layer = nodes
            .iter()
            .rev()
            .find(|read| read.kind == curvyo_document_core::NodeKind::Layer)
            .map(|read| read.id);
        let clones = Arc::new(curvyo_document_core::ObjectRead::from_objects(Arc::clone(
            &leaves,
        )));
        Self {
            clones,
            front_layer,
            nodes,
            leaves,
            visible,
            drawn,
            inert: Arc::new(inert),
        }
    }
}

/// The cached read and whether a drag holds it.
#[derive(Debug, Default)]
struct Held {
    read: Option<(DocumentVersion, Arc<DocumentRead>)>,
    /// A Select or Node-tool drag keeps its first read for its whole life, whatever the document
    /// does: it writes nothing until its release, so only a merge could change the document
    /// under it, and that is seen by the first read after the drag (`0031` criterion 17).
    pinned: bool,
}

/// The document read once per document version.
#[derive(Debug, Default)]
pub(super) struct ObjectCache {
    held: RefCell<Held>,
}

impl ObjectCache {
    /// The read of `document`. With `pin` (a drag is in flight) the read already held is
    /// returned unchanged, so a frame of a drag never reads the document; without it the read is
    /// returned only if the document is still at the version it was made at. Everything else
    /// (`New`, `Open`) builds a new `Session` and with it an empty cache.
    pub(super) fn read(&self, document: &Document, pin: bool) -> Arc<DocumentRead> {
        let mut held = self.held.borrow_mut();
        if pin
            && held.pinned
            && let Some((_, read)) = &held.read
        {
            return Arc::clone(read);
        }
        held.pinned = pin;
        let version = document.version();
        let read = match &held.read {
            Some((read_at, read)) if *read_at == version => Arc::clone(read),
            _ => {
                let read = Arc::new(DocumentRead::of(document));
                held.read = Some((version, Arc::clone(&read)));
                read
            }
        };
        #[cfg(test)]
        assert_eq!(
            format!("{:?}", read.leaves),
            format!("{:?}", DocumentRead::of(document).leaves),
            "the cached read differs from the document: a write did not change its version"
        );
        read
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{AnchorId, NewAnchor, Point};

    use super::*;

    fn add_path(document: &Document, n: u64) {
        let _ = document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(1, n), Point::new(0.0, 0.0)),
                NewAnchor::corner(AnchorId::new(1, n + 1), Point::new(10.0, 0.0)),
            ],
            false,
        );
    }

    #[test]
    fn a_second_read_at_the_same_version_is_the_same_allocation() {
        let document = Document::new(1);
        add_path(&document, 1);
        let cache = ObjectCache::default();
        let first = Arc::clone(&cache.read(&document, false).leaves);
        let second = Arc::clone(&cache.read(&document, false).leaves);
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(first.len(), 1);
    }

    #[test]
    fn a_commit_makes_the_next_read_fresh() {
        let document = Document::new(1);
        add_path(&document, 1);
        let cache = ObjectCache::default();
        let before = Arc::clone(&cache.read(&document, false).leaves);
        add_path(&document, 3);
        let after = Arc::clone(&cache.read(&document, false).leaves);
        assert!(!Arc::ptr_eq(&before, &after));
        assert_eq!((before.len(), after.len()), (1, 2));
    }

    /// The held read of a drag survives a commit and is dropped by the first read after it.
    #[test]
    fn a_pinned_read_survives_a_commit_until_the_drag_ends() {
        let document = Document::new(1);
        add_path(&document, 1);
        let cache = ObjectCache::default();
        let start = Arc::clone(&cache.read(&document, true).leaves);
        let during = Arc::clone(&cache.read(&document, true).leaves);
        assert!(Arc::ptr_eq(&start, &during));
        let ids = document.object_ids();
        let _ = document.translate_objects(&ids, curvyo_document_core::Vec2::new(5.0, 0.0));
        assert!(
            Arc::ptr_eq(&start, &cache.read(&document, true).leaves),
            "a drag keeps its read"
        );
        let after = Arc::clone(&cache.read(&document, false).leaves);
        assert!(!Arc::ptr_eq(&start, &after));
        assert_ne!(
            &start[..],
            &after[..],
            "the read after the drag sees the move"
        );
    }

    /// A linked clone is in the read resolved, and follows its original on the next read.
    #[test]
    fn the_read_holds_clones_resolved() {
        let document = Document::new(1);
        add_path(&document, 1);
        let original = document.object_ids()[0];
        let clone = document.linked_clone(&[original], &[]).unwrap()[0];
        let cache = ObjectCache::default();
        let read = cache.read(&document, false).leaves.clone();
        assert!(
            matches!(&read[1], ObjectSnapshot::Clone(c) if c.id() == clone && c.chain.origin == original)
        );
        let _ = document.translate_objects(&[original], curvyo_document_core::Vec2::new(5.0, 0.0));
        let after = cache.read(&document, false).leaves.clone();
        let ObjectSnapshot::Clone(moved) = &after[1] else {
            panic!("a clone")
        };
        assert_eq!(moved.resolved.anchors[0].point, Point::new(5.0, 0.0));
    }

    #[test]
    fn a_session_can_move_to_another_thread() {
        fn assert_send<T: Send>() {}
        assert_send::<super::super::Session>();
    }
}
