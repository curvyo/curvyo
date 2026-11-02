//! The children of a tiling group brought in line with the cells of an Apply (ADR 0030 §3 and
//! §6): kept, rewritten where they differ, created, replaced or deleted by cell and member key.

use std::collections::HashMap;
use std::hash::Hash;

use loro::{LoroTree, TreeID, TreeParentId};

use crate::group_tree::{KEY_GROUP, children_of, is_group_node, node_id};
use crate::path_model::AnchorId;
use crate::tiling_codec::{read_cell, read_member, write_cell, write_member};
use crate::tiling_commands::Plan;
use crate::tiling_model::{CellKey, TilingCell, TilingRequest};
use crate::tiling_write::{Leaf, MemberKind, is_member_of, update, write_new};

/// What the Apply knows about every member it writes.
struct Context<'a> {
    tree: &'a LoroTree,
    plan: &'a Plan,
    kinds: Vec<MemberKind>,
    old_kinds: Vec<MemberKind>,
    vary: (bool, bool),
}

/// How a keyed child relates to the cell it stands for.
enum Fit {
    /// A generated instance of the expected shape: rewrite where it differs.
    Fits,
    /// A generated instance of another shape (clones ↔ copies, one leaf ↔ several): replace.
    Replace,
    /// Something the maker made of it (unlinked, converted): leave alone, the cell is taken.
    Foreign,
}

fn lowest_first(ids: &mut [TreeID]) {
    ids.sort_unstable_by_key(|id| (id.peer, id.counter));
}

fn create(tree: &LoroTree, parent: TreeID, at: Option<usize>) -> (TreeID, loro::LoroMap) {
    let parent_id = TreeParentId::Node(parent);
    // invariant: the parent is live and `at` is the index of a child just deleted (at most the
    // child count); the meta map of a node just created can be read.
    #[allow(clippy::unwrap_used)]
    let node = match at {
        Some(index) => tree.create_at(parent_id, index).unwrap(),
        None => tree.create(parent_id).unwrap(),
    };
    #[allow(clippy::unwrap_used)]
    let meta = tree.get_meta(node).unwrap();
    (node, meta)
}

/// Groups the keyed children of `parent` by key with `read_key`, keeps the lowest id per key and
/// deletes the other copies (merge residue, ADR 0030 §6). Returns the survivors.
fn keyed<K: Hash + Eq + Copy>(
    tree: &LoroTree,
    parent: TreeID,
    read_key: impl Fn(&loro::LoroMap) -> Option<K>,
    doomed: &mut Vec<TreeID>,
) -> HashMap<K, TreeID> {
    let mut by_key: HashMap<K, Vec<TreeID>> = HashMap::new();
    for child in children_of(tree, Some(parent)) {
        if let Some(key) = tree.get_meta(child).ok().and_then(|meta| read_key(&meta)) {
            by_key.entry(key).or_default().push(child);
        }
    }
    by_key
        .into_iter()
        .map(|(key, mut ids)| {
            lowest_first(&mut ids);
            doomed.extend(ids.drain(1..));
            (key, ids[0])
        })
        .collect()
}

fn index_in(tree: &LoroTree, parent: TreeID, child: TreeID) -> Option<usize> {
    children_of(tree, Some(parent))
        .iter()
        .position(|c| *c == child)
}

impl Context<'_> {
    fn member_fit(&self, node: TreeID, index: usize) -> Fit {
        let Ok(meta) = self.tree.get_meta(node) else {
            return Fit::Foreign;
        };
        let leaf = &self.plan.leaves[index];
        let read = &self.plan.read;
        if is_member_of(read, &meta, node_id(node), leaf, self.kinds[index]) {
            Fit::Fits
        } else if is_member_of(read, &meta, node_id(node), leaf, self.old_kinds[index]) {
            Fit::Replace
        } else {
            Fit::Foreign
        }
    }

    fn instance_fit(&self, node: TreeID) -> Fit {
        let multi = self.plan.leaves.len() > 1;
        let is_group = is_group_node(self.tree, node);
        let holds_members = is_group
            && children_of(self.tree, Some(node)).iter().any(|c| {
                self.tree
                    .get_meta(*c)
                    .is_ok_and(|m| read_member(&m).is_some())
            });
        match (multi, is_group) {
            (true, true) => Fit::Fits,
            (false, false) => self.member_fit(node, 0),
            (false, true) if holds_members => Fit::Replace,
            (true, false)
                if (0..self.plan.leaves.len())
                    .any(|i| !matches!(self.member_fit(node, i), Fit::Foreign)) =>
            {
                Fit::Replace
            }
            _ => Fit::Foreign,
        }
    }

    fn write_member_node(
        &self,
        parent: TreeID,
        at: Option<usize>,
        index: usize,
        cell: &TilingCell,
        keyed: bool,
        mint: &mut dyn FnMut() -> AnchorId,
    ) {
        let (_, meta) = create(self.tree, parent, at);
        let leaf: &Leaf = &self.plan.leaves[index];
        write_new(&meta, leaf, self.kinds[index], cell, self.vary.0, mint);
        if keyed {
            write_member(&meta, leaf.id);
        }
    }

    /// Creates the instance of `cell` at `at` (or last) under `group`.
    fn create_instance(
        &self,
        group: TreeID,
        at: Option<usize>,
        cell: &TilingCell,
        mint: &mut dyn FnMut() -> AnchorId,
    ) {
        if self.plan.leaves.len() == 1 {
            let (_, meta) = create(self.tree, group, at);
            write_new(
                &meta,
                &self.plan.leaves[0],
                self.kinds[0],
                cell,
                self.vary.0,
                mint,
            );
            write_cell(&meta, cell.key);
            return;
        }
        let (node, meta) = create(self.tree, group, at);
        // invariant: inserting a bool under a known key of a fresh meta map cannot fail.
        #[allow(clippy::unwrap_used)]
        meta.insert(KEY_GROUP, true).unwrap();
        write_cell(&meta, cell.key);
        for index in 0..self.plan.leaves.len() {
            self.write_member_node(node, None, index, cell, true, mint);
        }
    }

    fn update_member(
        &self,
        node: TreeID,
        index: usize,
        cell: &TilingCell,
        mint: &mut dyn FnMut() -> AnchorId,
    ) -> bool {
        let Ok(meta) = self.tree.get_meta(node) else {
            return false;
        };
        let leaf = &self.plan.leaves[index];
        update(
            &self.plan.read,
            (&meta, node_id(node)),
            leaf,
            self.kinds[index],
            cell,
            self.vary,
            mint,
        )
    }

    /// Brings the members of a multi-source instance group in line (criterion 24).
    fn update_members(
        &self,
        instance: TreeID,
        cell: &TilingCell,
        doomed: &mut Vec<TreeID>,
        mint: &mut dyn FnMut() -> AnchorId,
    ) -> bool {
        let before = doomed.len();
        let members = keyed(self.tree, instance, read_member, doomed);
        let mut changed = doomed.len() > before;
        let wanted: HashMap<_, usize> = self
            .plan
            .leaves
            .iter()
            .enumerate()
            .map(|(i, leaf)| (leaf.id, i))
            .collect();
        for (leaf, node) in &members {
            if !wanted.contains_key(leaf) {
                doomed.push(*node);
                changed = true;
            }
        }
        for (index, leaf) in self.plan.leaves.iter().enumerate() {
            match members.get(&leaf.id) {
                None => {
                    self.write_member_node(instance, None, index, cell, true, mint);
                    changed = true;
                }
                Some(&node) => match self.member_fit(node, index) {
                    Fit::Fits => {
                        changed |= self.update_member(node, index, cell, mint);
                    }
                    Fit::Replace => {
                        let at = index_in(self.tree, instance, node);
                        doomed.push(node);
                        self.write_member_node(instance, at, index, cell, true, mint);
                        changed = true;
                    }
                    Fit::Foreign => {}
                },
            }
        }
        changed
    }

    /// Brings the instance of `cell` in line: creates it, rewrites it where it differs,
    /// replaces it, or leaves a foreign one alone. Returns whether anything was written.
    fn apply_cell(
        &self,
        group: TreeID,
        existing: Option<TreeID>,
        cell: &TilingCell,
        doomed: &mut Vec<TreeID>,
        mint: &mut dyn FnMut() -> AnchorId,
    ) -> bool {
        let Some(node) = existing else {
            self.create_instance(group, None, cell, mint);
            return true;
        };
        match self.instance_fit(node) {
            Fit::Fits if self.plan.leaves.len() > 1 => {
                self.update_members(node, cell, doomed, mint)
            }
            Fit::Fits => self.update_member(node, 0, cell, mint),
            Fit::Replace => {
                let at = index_in(self.tree, group, node);
                doomed.push(node);
                self.create_instance(group, at, cell, mint);
                true
            }
            Fit::Foreign => false,
        }
    }
}

/// Brings the children of `group` in line with `request.cells`; returns whether anything was
/// written. Uncommitted.
pub(crate) fn regenerate(
    document: &crate::document::Document,
    tree: &LoroTree,
    group: TreeID,
    plan: &Plan,
    request: &TilingRequest,
    mint: &mut dyn FnMut() -> AnchorId,
) -> bool {
    let old_result = plan.stored.map_or(request.recipe.result, |r| r.result);
    let context = Context {
        tree,
        plan,
        kinds: plan
            .leaves
            .iter()
            .map(|l| l.kind(request.recipe.result))
            .collect(),
        old_kinds: plan.leaves.iter().map(|l| l.kind(old_result)).collect(),
        vary: (
            request.recipe.variance.varies_style(),
            plan.stored.is_some_and(|r| r.variance.varies_style()),
        ),
    };
    // The nodes to delete once every write is done: their clones are repaired first.
    let mut doomed = Vec::new();
    let instances = keyed(tree, group, read_cell, &mut doomed);
    let wanted: HashMap<CellKey, &TilingCell> = request.cells.iter().map(|c| (c.key, c)).collect();
    doomed.extend(
        instances
            .iter()
            .filter(|(key, _)| !wanted.contains_key(key))
            .map(|(_, node)| *node),
    );
    let mut changed = !doomed.is_empty();
    for cell in &request.cells {
        let existing = instances.get(&cell.key).copied();
        changed |= context.apply_cell(group, existing, cell, &mut doomed, mint);
    }
    // Clones a maker made of a removed instance are repaired as for any deleted object (`0049`).
    document.repair_clones_of_removed(tree, &doomed);
    for node in doomed {
        // invariant: the node was read as a live child of a live parent by this Apply, and no
        // doomed node lies under another (an instance is doomed whole or its members alone).
        #[allow(clippy::unwrap_used)]
        tree.delete(node).unwrap();
    }
    // A tiling always keeps a child (count 1 or more, or the kept source), so this removes
    // nothing; it is the rule of every command that deletes nodes.
    crate::groups::remove_emptied_groups(tree, &[Some(group)]);
    changed
}
