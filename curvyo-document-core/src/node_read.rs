//! The one read of every node of the object tree, in one pre-order pass, with the visible, locked
//! and opacity values each node has through its ancestors
//! (`specs/0050-layers-and-objects-panel/adrs.md` decision 3, ADR 0015 §2 and §6).
//!
//! The walk carries the ancestors' values down, so the pass is O(n) and no consumer walks the
//! tree. Hidden leaves are read too: what to leave out is the consumer's decision, by flag.

use loro::{LoroTree, TreeID};

use crate::clone_read::{RawLeaf, resolve};
use crate::document::{Document, OBJECTS_TREE};
use crate::group_tree::{children_of, is_group_node, is_layer_node, node_id};
use crate::node_registers::{NodeRegisters, read_registers};
use crate::object_read::raw_leaf;
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
    /// Whether it is drawn and treated as a guide (ADR 0019 §2 and §3): a path's or shape's own
    /// `guide` register, a linked clone's root's (`specs/0046-document-shapes` criterion 34).
    /// Always `false` for a layer or a group.
    pub guide: bool,
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

    /// Whether it is part of what is produced: a leaf that is shown and is not a guide. The one
    /// rule every machine output and artwork export filters on (`specs/0046-document-shapes`
    /// criteria 25 and 41, `adrs.md` decision 2).
    #[must_use]
    pub const fn is_output_node(&self) -> bool {
        self.object().is_some() && self.visible && !self.guide
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

/// The own registers of every leaf that a linked clone reads from another leaf of its chain: the
/// opacity of its style source and the guide flag of its root.
struct OwnLeafRegisters {
    opacity: std::collections::HashMap<NodeId, f64>,
    guides: std::collections::HashSet<NodeId>,
}

/// One node met by the walk, before the clones among the leaves are resolved.
struct Walked {
    node: NodeId,
    from: Inherited,
    registers: NodeRegisters,
    /// A layer or a group; `None` for a leaf, whose data comes from the resolver.
    container: Option<NodeKind>,
    /// The index of the leaf in the resolver's input.
    leaf: Option<usize>,
}

impl Walked {
    /// The node's read with its data, or `None` for a leaf the resolver left out (an orphan) or
    /// that does not decode. A linked clone's opacity is its style source's own `opacity` register
    /// times the clone's own ancestors' (ADR 0016 §3): its own with own style, else the nearest
    /// own-style clone's on its chain, else the root's; the key on a clone without own style is
    /// residue.
    fn read(
        self,
        leaves: &mut [Option<ObjectSnapshot>],
        own: &OwnLeafRegisters,
    ) -> Option<NodeRead> {
        let kind = match (self.container, self.leaf) {
            (Some(kind), _) => kind,
            (None, Some(at)) => NodeKind::Leaf(leaves[at].take()?),
            (None, None) => return None,
        };
        let own_opacity = match &kind {
            NodeKind::Leaf(ObjectSnapshot::Clone(clone)) => own
                .opacity
                .get(&clone.chain.style_source)
                .copied()
                .unwrap_or(1.0),
            _ => self.registers.opacity.get(),
        };
        let guide = match &kind {
            NodeKind::Leaf(object) => {
                crate::guides::effective_guide(object, |id| own.guides.contains(&id))
            }
            NodeKind::Layer | NodeKind::Group => false,
        };
        // invariant: a product of numbers in 0..=1 lies in 0..=1.
        let opacity = Opacity::new(self.from.opacity * own_opacity).unwrap_or(Opacity::OPAQUE);
        Some(NodeRead {
            id: self.node,
            parent: self.from.parent,
            depth: self.from.depth,
            kind,
            visible: self.registers.visible && self.from.hidden_by.is_none(),
            locked: self.registers.locked || self.from.locked_by.is_some(),
            opacity,
            hidden_by: self.from.hidden_by,
            locked_by: self.from.locked_by,
            guide,
            registers: self.registers,
        })
    }
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
        let mut walked: Vec<Walked> = Vec::new();
        let mut raws: Vec<RawLeaf> = Vec::new();
        let mut stack: Vec<(TreeID, Inherited)> = children_of(&tree, None)
            .into_iter()
            .rev()
            .map(|id| (id, Inherited::TOP))
            .collect();
        while let Some((id, from)) = stack.pop() {
            let node = node_id(id);
            let registers = tree
                .get_meta(id)
                .map(|meta| read_registers(&meta))
                .unwrap_or_default();
            let container = if is_layer_node(&tree, id) {
                Some(NodeKind::Layer)
            } else if is_group_node(&tree, id) {
                Some(NodeKind::Group)
            } else {
                None
            };
            if container.is_some() {
                let opacity = from.opacity * registers.opacity.get();
                let down = Inherited {
                    parent: Some(node),
                    depth: from.depth + 1,
                    opacity,
                    hidden_by: if registers.visible {
                        from.hidden_by
                    } else {
                        Some(node)
                    },
                    locked_by: if registers.locked {
                        Some(node)
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
            let leaf = if container.is_none() {
                raw_leaf(&tree, id).map(|raw| {
                    raws.push(raw);
                    raws.len() - 1
                })
            } else {
                None
            };
            walked.push(Walked {
                node,
                from,
                registers,
                container,
                leaf,
            });
        }
        // One resolution of every clone, in the stacking order of the walk (`specs/0049-linked-
        // clone/adrs.md` decision 3 as amended: result i belongs to leaf i).
        let own = OwnLeafRegisters {
            opacity: walked
                .iter()
                .filter(|w| w.leaf.is_some())
                .map(|w| (w.node, w.registers.opacity.get()))
                .collect(),
            guides: walked
                .iter()
                .filter(|w| w.leaf.is_some() && w.registers.guide)
                .map(|w| w.node)
                .collect(),
        };
        let mut leaves = resolve(raws).leaves;
        walked
            .into_iter()
            .filter_map(|w| w.read(&mut leaves, &own))
            .collect()
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
