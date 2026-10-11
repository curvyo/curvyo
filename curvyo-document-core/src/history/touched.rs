//! Which objects a step touched, derived from its operations and never stored (ADR 0014 §5).
//!
//! Tree operations name their target directly. Every other operation names a container, which maps
//! to an object through the chain of containers up to an object's meta map. The chain comes from
//! the container-creating operations of the log, so it also resolves the nested containers of
//! deleted objects, which `get_path_to_container` cannot.

use std::collections::{BTreeSet, HashMap};

use loro::{
    ContainerID, ContainerType, JsonChange, JsonListOp, JsonMapOp, JsonMovableListOp,
    JsonOpContent, JsonTreeOp, LoroDoc, LoroValue, TreeID, VersionVector,
};

use crate::document::{OBJECTS_TREE, ROOT_MAP};

/// What a container belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Owner {
    /// The document root map.
    Root,
    /// An object's tree node.
    Node(TreeID),
}

/// The objects one step created, deleted, moved or changed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Touched {
    pub(crate) created: Vec<TreeID>,
    pub(crate) deleted: Vec<TreeID>,
    pub(crate) moved: Vec<TreeID>,
    pub(crate) changed: Vec<TreeID>,
    /// The step wrote a root register (size, unit).
    pub(crate) root: bool,
}

impl Touched {
    /// The same step cut to one object: what an object undo of a Change, Create or Delete step
    /// restores.
    pub(crate) fn only(&self, object: TreeID) -> Self {
        let keep = |nodes: &[TreeID]| nodes.iter().copied().filter(|n| *n == object).collect();
        Self {
            created: keep(&self.created),
            deleted: keep(&self.deleted),
            moved: keep(&self.moved),
            changed: keep(&self.changed),
            root: false,
        }
    }

    /// Every touched object, created ones first.
    pub(crate) fn nodes(&self) -> impl Iterator<Item = TreeID> + '_ {
        self.created
            .iter()
            .chain(&self.deleted)
            .chain(&self.moved)
            .chain(&self.changed)
            .copied()
    }
}

/// The child-to-parent chain of every container created in the log so far.
#[derive(Debug, Default)]
pub(crate) struct ContainerIndex {
    parents: HashMap<ContainerID, ContainerID>,
    indexed: VersionVector,
}

impl ContainerIndex {
    /// Reads the changes committed since the last call into the index.
    pub(crate) fn extend(&mut self, doc: &LoroDoc) {
        let end = doc.oplog_vv();
        if end == self.indexed {
            return;
        }
        let updates = doc.export_json_updates_without_peer_compression(&self.indexed, &end);
        for change in &updates.changes {
            for op in &change.ops {
                self.note(&op.container, &op.content);
            }
        }
        self.indexed = end;
    }

    fn note(&mut self, parent: &ContainerID, content: &JsonOpContent) {
        let mut child = |value: &LoroValue| {
            if let LoroValue::Container(id) = value {
                self.parents.insert(id.clone(), parent.clone());
            }
        };
        match content {
            JsonOpContent::Map(JsonMapOp::Insert { value, .. })
            | JsonOpContent::MovableList(JsonMovableListOp::Set { value, .. }) => child(value),
            JsonOpContent::List(JsonListOp::Insert { value, .. })
            | JsonOpContent::MovableList(JsonMovableListOp::Insert { value, .. }) => {
                value.iter().for_each(child);
            }
            _ => {}
        }
    }

    /// The object a container belongs to, following the chain up to a meta map or the root map.
    pub(crate) fn owner(&self, doc: &LoroDoc, container: &ContainerID) -> Option<Owner> {
        let mut current = container.clone();
        for _ in 0..64 {
            match &current {
                ContainerID::Root {
                    name,
                    container_type: ContainerType::Map,
                } if name.as_str() == ROOT_MAP => return Some(Owner::Root),
                ContainerID::Normal {
                    peer,
                    counter,
                    container_type: ContainerType::Map,
                } => {
                    let id = TreeID::new(*peer, *counter);
                    if doc.get_tree(OBJECTS_TREE).contains(id) {
                        return Some(Owner::Node(id));
                    }
                }
                _ => {}
            }
            match self.parents.get(&current) {
                Some(parent) => current = parent.clone(),
                None => break,
            }
        }
        Self::owner_by_path(doc, container)
    }

    /// A container the chain does not know (created before a wipe): ask the live document.
    fn owner_by_path(doc: &LoroDoc, container: &ContainerID) -> Option<Owner> {
        let path = doc.get_path_to_container(container)?;
        path.iter().find_map(|(_, index)| match index {
            loro::Index::Node(id) => Some(Owner::Node(*id)),
            _ => None,
        })
    }

    /// The touched objects of the given changes of one step.
    pub(crate) fn touched(&self, doc: &LoroDoc, changes: &[JsonChange]) -> Touched {
        let mut created_ids = BTreeSet::new();
        let mut deleted_ids = BTreeSet::new();
        let mut moved_ids = BTreeSet::new();
        let mut changed_ids = BTreeSet::new();
        let mut root = false;
        let mut order: Vec<TreeID> = Vec::new();
        let note = |id: TreeID, order: &mut Vec<TreeID>| {
            if !order.contains(&id) {
                order.push(id);
            }
        };
        for op in changes.iter().flat_map(|change| &change.ops) {
            match &op.content {
                JsonOpContent::Tree(tree_op) => match tree_op {
                    JsonTreeOp::Create { target, .. } => {
                        note(*target, &mut order);
                        created_ids.insert(*target);
                    }
                    JsonTreeOp::Move { target, .. } => {
                        note(*target, &mut order);
                        moved_ids.insert(*target);
                    }
                    JsonTreeOp::Delete { target } => {
                        note(*target, &mut order);
                        deleted_ids.insert(*target);
                    }
                },
                _ => match self.owner(doc, &op.container) {
                    Some(Owner::Root) => root = true,
                    Some(Owner::Node(id)) => {
                        note(id, &mut order);
                        changed_ids.insert(id);
                    }
                    None => {}
                },
            }
        }
        // A created object is not also "moved" (its placement) or "changed" (its first values);
        // a deleted one is not "changed".
        let pick = |set: &BTreeSet<TreeID>, skip: &[&BTreeSet<TreeID>]| -> Vec<TreeID> {
            order
                .iter()
                .copied()
                .filter(|id| set.contains(id) && skip.iter().all(|other| !other.contains(id)))
                .collect()
        };
        Touched {
            created: pick(&created_ids, &[&deleted_ids]),
            deleted: pick(&deleted_ids, &[&created_ids]),
            moved: pick(&moved_ids, &[&created_ids, &deleted_ids]),
            changed: pick(&changed_ids, &[&created_ids, &deleted_ids]),
            root,
        }
    }
}
