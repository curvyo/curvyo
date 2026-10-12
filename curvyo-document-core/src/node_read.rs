//! The one read of every node of the object tree, in one pre-order pass, with the visible, locked
//! and opacity values each node has through its ancestors
//! (`specs/0050-layers-and-objects-panel/adrs.md` decision 3, ADR 0015 §2 and §6).
//!
//! The walk carries the ancestors' values down, so the pass is O(n) and no consumer walks the
//! tree. Hidden leaves are read too: what to leave out is the consumer's decision, by flag.

use loro::{LoroTree, TreeID};

use crate::document::{Document, OBJECTS_TREE};
use crate::group_tree::{children_of, is_group_node, is_layer_node, node_id};
use crate::node_registers::{NodeRegisters, read_registers};
use crate::path_model::NodeId;
use crate::primitive_model::ObjectSnapshot;
use crate::style_model::Opacity;

/// What a node is.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    /// A layer: a group at the top level with the layer marker.
    Layer,
    /// A group (`specs/0023-groups`).
    Group,
    /// A path or a primitive, with its data.
    Leaf(ObjectSnapshot),
}

/// One node as the read pass sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeRead {
    /// The node's id.
    pub id: NodeId,
    /// The node that holds it; `None` at the top level.
    pub parent: Option<NodeId>,
    /// How many nodes lie above it: 0 at the top level.
    pub depth: usize,
    /// Whether it is a layer, a group or a leaf.
    pub kind: NodeKind,
    /// Its own registers.
    pub registers: NodeRegisters,
    /// Whether it is shown: its own flag and every ancestor's are on.
    pub visible: bool,
    /// Whether it is locked: its own flag or any ancestor's is on.
    pub locked: bool,
    /// Its own opacity times every ancestor's.
    pub opacity: Opacity,
    /// The nearest ancestor whose own flag hides it (not the node itself); `None` when no
    /// ancestor hides it.
    pub hidden_by: Option<NodeId>,
    /// The nearest ancestor whose own flag locks it (not the node itself); `None` when no
    /// ancestor locks it.
    pub locked_by: Option<NodeId>,
}

impl NodeRead {
    /// The leaf's data, `None` for a layer or a group.
    #[must_use]
    pub const fn object(&self) -> Option<&ObjectSnapshot> {
        match &self.kind {
            NodeKind::Leaf(object) => Some(object),
            NodeKind::Layer | NodeKind::Group => None,
        }
    }

    /// Whether it is a layer or a group.
    #[must_use]
    pub const fn is_container(&self) -> bool {
        matches!(self.kind, NodeKind::Layer | NodeKind::Group)
    }
}

/// What a node hands down to its children.
#[derive(Clone, Copy)]
struct Inherited {
    parent: Option<NodeId>,
    depth: usize,
    opacity: f64,
    hidden_by: Option<NodeId>,
    locked_by: Option<NodeId>,
}

impl Inherited {
    const TOP: Self = Self {
        parent: None,
        depth: 0,
        opacity: 1.0,
        hidden_by: None,
        locked_by: None,
    };
}

/// The read of one node, or `None` for a leaf whose data does not decode (merge residue only:
/// open refuses such a file), which the read leaves out with everything it would hold.
fn read_node(
    document: &Document,
    tree: &LoroTree,
    id: TreeID,
    from: Inherited,
) -> Option<NodeRead> {
    let node = node_id(id);
    let registers = tree
        .get_meta(id)
        .map(|meta| read_registers(&meta))
        .unwrap_or_default();
    let kind = if is_layer_node(tree, id) {
        NodeKind::Layer
    } else if is_group_node(tree, id) {
        NodeKind::Group
    } else {
        NodeKind::Leaf(document.object(node)?)
    };
    let product = from.opacity * registers.opacity.get();
    // invariant: a product of numbers in 0..=1 lies in 0..=1.
    let opacity = Opacity::new(product).unwrap_or(Opacity::OPAQUE);
    Some(NodeRead {
        id: node,
        parent: from.parent,
        depth: from.depth,
        kind,
        visible: registers.visible && from.hidden_by.is_none(),
        locked: registers.locked || from.locked_by.is_some(),
        opacity,
        hidden_by: from.hidden_by,
        locked_by: from.locked_by,
        registers,
    })
}

impl Document {
    /// Every node of the object tree in one pre-order pass, siblings in stacking order (back to
    /// front), each with its effective visible, locked and opacity values
    /// (`specs/0050-layers-and-objects-panel` criterion 24). Hidden and locked nodes are listed. A
    /// leaf whose data does not decode (merge residue) is left out, as `document.json` leaves it
    /// out, so no consumer meets a node it cannot use.
    #[must_use]
    pub fn read_nodes(&self) -> Vec<NodeRead> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut reads = Vec::new();
        let mut stack: Vec<(TreeID, Inherited)> = children_of(&tree, None)
            .into_iter()
            .rev()
            .map(|id| (id, Inherited::TOP))
            .collect();
        while let Some((id, from)) = stack.pop() {
            let Some(read) = read_node(self, &tree, id, from) else {
                continue;
            };
            if read.is_container() {
                let down = Inherited {
                    parent: Some(read.id),
                    depth: from.depth + 1,
                    opacity: read.opacity.get(),
                    hidden_by: if read.registers.visible {
                        from.hidden_by
                    } else {
                        Some(read.id)
                    },
                    locked_by: if read.registers.locked {
                        Some(read.id)
                    } else {
                        from.locked_by
                    },
                };
                stack.extend(
                    children_of(&tree, Some(id))
                        .into_iter()
                        .rev()
                        .map(|child| (child, down)),
                );
            }
            reads.push(read);
        }
        reads
    }

    /// The own registers of `id`, or `None` when it does not exist.
    #[must_use]
    pub fn node_registers(&self, id: NodeId) -> Option<NodeRegisters> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let tree_id = crate::paths::tree_id_of(id);
        if !crate::path_codec::node_exists(&tree, tree_id) {
            return None;
        }
        tree.get_meta(tree_id)
            .ok()
            .map(|meta| read_registers(&meta))
    }

    /// Whether `id` is a live layer.
    #[must_use]
    pub fn is_layer(&self, id: NodeId) -> bool {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let tree_id = crate::paths::tree_id_of(id);
        crate::path_codec::node_exists(&tree, tree_id) && is_layer_node(&tree, tree_id)
    }
}

/// Whether `id` or any node above it has its own `locked` flag on: the effective lock of one node,
/// for the writes that refuse a locked node.
pub(crate) fn locked_through(tree: &LoroTree, id: TreeID) -> bool {
    // A Loro tree holds no cycle, so the walk to the root ends; it is not capped, so a merged
    // tree deeper than any limit still answers correctly.
    let mut current = Some(id);
    while let Some(node) = current {
        if tree
            .get_meta(node)
            .is_ok_and(|meta| read_registers(&meta).locked)
        {
            return true;
        }
        current = crate::group_tree::group_parent(tree, node);
    }
    false
}
