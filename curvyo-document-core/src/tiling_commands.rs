//! The tiling commands (ADR 0028 §3, §4, §6 and §7): Apply (create and re-apply), Release, the
//! read of a stored tiling, and the strip and remap that Ungroup and the copy writers call.

use std::collections::{HashMap, HashSet};

use loro::{LoroTree, TreeID, TreeParentId};

use crate::clone_model::MAX_CLONE_DEPTH;
use crate::clone_read::ObjectRead;
use crate::document::{Document, OBJECTS_TREE};
use crate::group_tree::{
    KEY_GROUP, MAX_GROUP_DEPTH, children_of, group_parent, groups_above, groups_in, is_group_node,
    leaves_under, node_id,
};
use crate::path_codec::node_exists;
use crate::path_model::{AnchorId, NodeId, PathSnapshot};
use crate::paths::tree_id_of;
use crate::primitive_model::ObjectSnapshot;
use crate::tiling_codec::{read_recipe, read_sources};
use crate::tiling_model::{
    CellKey, EDIT_TILING_COMMIT_LABEL, RELEASE_TILING_COMMIT_LABEL, TILE_COMMIT_LABEL,
    TILING_GROUP_NAME, TilingError, TilingRecipe, TilingRequest, TilingResult,
};
use crate::tiling_write::{Leaf, LeafSource};

/// A stored tiling as the bar reopens it (criterion 26).
#[derive(Debug, Clone, PartialEq)]
pub struct StoredTiling {
    /// The recipe.
    pub recipe: TilingRecipe,
    /// The source nodes, as selected.
    pub sources: Vec<NodeId>,
    /// Every source still exists; otherwise only Release is offered (criterion 30).
    pub sources_live: bool,
}

/// What an Apply did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TilingOutcome {
    /// The tiling group.
    pub tiling: NodeId,
    /// Whether anything was written (an unchanged re-apply writes nothing, criterion 27).
    pub written: bool,
}

/// Everything an Apply checked before its first write.
pub(crate) struct Plan {
    pub(crate) tiling: Option<TreeID>,
    pub(crate) sources: Vec<NodeId>,
    pub(crate) leaves: Vec<Leaf>,
    pub(crate) stored: Option<TilingRecipe>,
    pub(crate) read: std::sync::Arc<ObjectRead>,
}

fn is_locked(tree: &LoroTree, id: TreeID) -> bool {
    let mut current = Some(id);
    let mut steps = 0;
    while let Some(node) = current {
        if tree
            .get_meta(node)
            .is_ok_and(|meta| crate::node_registers::read_registers(&meta).locked)
        {
            return true;
        }
        current = group_parent(tree, node);
        steps += 1;
        if steps > MAX_GROUP_DEPTH * 4 {
            break;
        }
    }
    false
}

fn holds_tiling(tree: &LoroTree, id: TreeID) -> bool {
    let mut stack = vec![id];
    while let Some(node) = stack.pop() {
        if tree
            .get_meta(node)
            .is_ok_and(|meta| crate::tiling_codec::has_recipe(&meta))
        {
            return true;
        }
        if is_group_node(tree, node) {
            stack.extend(children_of(tree, Some(node)));
        }
    }
    false
}

/// Every node of the tree in paint order (pre-order), groups included.
fn paint_order(tree: &LoroTree) -> HashMap<TreeID, usize> {
    let mut order = HashMap::new();
    let mut stack = vec![children_of(tree, None).into_iter()];
    while let Some(level) = stack.last_mut() {
        let Some(node) = level.next() else {
            stack.pop();
            continue;
        };
        order.insert(node, order.len());
        if is_group_node(tree, node) {
            stack.push(children_of(tree, Some(node)).into_iter());
        }
    }
    order
}

/// The leaves of `sources` in paint order, each once, dimensions skipped (criterion 5).
fn flattened(tree: &LoroTree, sources: &[NodeId]) -> Vec<TreeID> {
    let order = paint_order(tree);
    let mut seen = HashSet::new();
    let mut leaves: Vec<TreeID> = Vec::new();
    for &source in sources {
        let id = tree_id_of(source);
        let under = if is_group_node(tree, id) {
            leaves_under(tree, Some(id))
        } else {
            vec![id]
        };
        leaves.extend(under.into_iter().filter(|leaf| seen.insert(*leaf)));
    }
    leaves.retain(|leaf| {
        tree.get_meta(*leaf)
            .is_ok_and(|meta| !crate::dimension_codec::is_dimension_meta(&meta))
    });
    leaves.sort_by_key(|leaf| order.get(leaf).copied().unwrap_or(usize::MAX));
    leaves
}

fn leaf_of(
    read: &ObjectRead,
    tree: &LoroTree,
    id: NodeId,
    request: &TilingRequest,
) -> Result<Leaf, TilingError> {
    let opacity = tree.get_meta(tree_id_of(id)).map_or(1.0, |meta| {
        crate::node_registers::read_registers(&meta).opacity.get()
    });
    let clones = request.recipe.result == TilingResult::Clones;
    let source = match read.get(id).ok_or(TilingError::CloneTooDeep)? {
        ObjectSnapshot::Clone(clone) if clones && clone.chain.depth >= MAX_CLONE_DEPTH => {
            return Err(TilingError::CloneTooDeep);
        }
        ObjectSnapshot::Primitive(primitive) => {
            let anchors = request
                .conversions
                .iter()
                .find(|(of, _)| *of == id)
                .map(|(_, anchors)| anchors.clone())
                .ok_or(TilingError::Conversions)?;
            LeafSource::Outline(PathSnapshot {
                id,
                closed: true,
                style: primitive.style.clone(),
                anchors,
                extra_subpaths: Vec::new(),
                rotation: crate::units::Angle::from_radians(0.0),
            })
        }
        ObjectSnapshot::Text(text) => LeafSource::Text {
            root: text.source.map_or(id, |s| s.node),
            text: text.clone(),
        },
        ObjectSnapshot::Image(image) => LeafSource::Image(image.clone()),
        other => LeafSource::Outline(other.drawn_path().ok_or(TilingError::EmptySource)?.clone()),
    };
    Ok(Leaf {
        id,
        source,
        opacity,
    })
}

impl Document {
    /// The stored tiling of `group`, `None` when it is not a tiling (or its recipe is residue).
    #[must_use]
    pub fn tiling(&self, group: NodeId) -> Option<StoredTiling> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let id = tree_id_of(group);
        if !node_exists(&tree, id) || !is_group_node(&tree, id) {
            return None;
        }
        let meta = tree.get_meta(id).ok()?;
        let recipe = read_recipe(&meta)?.ok()?;
        let sources = read_sources(&meta).unwrap_or_default();
        let sources_live =
            !sources.is_empty() && sources.iter().all(|s| node_exists(&tree, tree_id_of(*s)));
        Some(StoredTiling {
            recipe,
            sources,
            sources_live,
        })
    }

    /// The managed instances of the tiling `group`: its children with a cell key, in paint order
    /// (criterion 34). Empty when `group` is not a tiling.
    #[must_use]
    pub fn tiling_instances(&self, group: NodeId) -> Vec<NodeId> {
        if self.tiling(group).is_none() {
            return Vec::new();
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        children_of(&tree, Some(tree_id_of(group)))
            .into_iter()
            .filter(|child| {
                tree.get_meta(*child)
                    .is_ok_and(|meta| crate::tiling_codec::read_cell(&meta).is_some())
            })
            .map(node_id)
            .collect()
    }

    /// The leaves a tiling of `sources` repeats, in paint order: groups flattened, dimensions
    /// skipped (criterion 5). The caller builds the conversions of the primitives among them.
    #[must_use]
    pub fn tiling_leaves(&self, sources: &[NodeId]) -> Vec<NodeId> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let live: Vec<NodeId> = sources
            .iter()
            .copied()
            .filter(|s| node_exists(&tree, tree_id_of(*s)))
            .collect();
        flattened(&tree, &live).into_iter().map(node_id).collect()
    }

    /// Checks everything an Apply needs before its first write (ADR 0028 §4).
    fn plan_tiling(
        &self,
        request: &TilingRequest,
        read: std::sync::Arc<ObjectRead>,
    ) -> Result<Plan, TilingError> {
        request.recipe.check().map_err(TilingError::Recipe)?;
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let (tiling, stored, sources) = if let Some(group) = request.tiling {
            let stored = self.tiling(group).ok_or(TilingError::NotATiling)?;
            if !stored.sources_live {
                return Err(TilingError::SourceMissing);
            }
            (Some(tree_id_of(group)), Some(stored.recipe), stored.sources)
        } else {
            let mut seen = HashSet::new();
            let sources: Vec<NodeId> = request
                .sources
                .iter()
                .copied()
                .filter(|s| seen.insert(*s))
                .collect();
            (None, None, sources)
        };
        check_sources(&tree, &sources)?;
        let leaf_ids = flattened(&tree, &sources);
        if leaf_ids.is_empty() {
            return Err(TilingError::EmptySource);
        }
        check_depth(&tree, tiling, &sources, leaf_ids.len() > 1)?;
        check_cells(request)?;
        let leaves = leaf_ids
            .iter()
            .map(|&leaf| leaf_of(&read, &tree, node_id(leaf), request))
            .collect::<Result<Vec<_>, _>>()?;
        check_conversions(&read, &leaves, request)?;
        Ok(Plan {
            tiling,
            sources,
            leaves,
            stored,
            read,
        })
    }

    /// Creates or re-applies a tiling in **one commit** (ADR 0028 §3): "Tile" for a new one,
    /// "Edit tiling" for a re-apply. The group holds the kept source (lowest) and one instance per
    /// cell of `request.cells`; a re-apply keeps instances whose key stays and rewrites only what
    /// differs, creates missing cells and members, deletes cells that left the layout, leaves
    /// unkeyed and hand-converted children alone and repairs duplicate keys. Nothing is committed
    /// when nothing differs. `mint` gives the anchor ids of path copies.
    ///
    /// # Errors
    /// Every [`TilingError`] is checked before the first write; nothing is written then.
    ///
    /// # Panics
    /// Does not panic in practice: every tree write acts on nodes just confirmed live.
    pub fn apply_tiling(
        &self,
        request: &TilingRequest,
        mint: &mut dyn FnMut() -> AnchorId,
    ) -> Result<TilingOutcome, TilingError> {
        self.apply_tiling_with_read(request, std::sync::Arc::new(self.read_objects()), mint)
    }

    /// [`Self::apply_tiling`] with `read`, a read of this document at its current version that
    /// the caller already holds (the session's cached read), so the Apply does not read the
    /// document again (criterion 40).
    ///
    /// # Errors
    /// As [`Self::apply_tiling`].
    ///
    /// # Panics
    /// As [`Self::apply_tiling`]: does not panic in practice.
    pub fn apply_tiling_with_read(
        &self,
        request: &TilingRequest,
        read: std::sync::Arc<ObjectRead>,
        mint: &mut dyn FnMut() -> AnchorId,
    ) -> Result<TilingOutcome, TilingError> {
        let plan = self.plan_tiling(request, read)?;
        let tree = self.loro().get_tree(OBJECTS_TREE);
        tree.enable_fractional_index(0);
        if request.recipe.result == TilingResult::Clones && !request.conversions.is_empty() {
            self.convert_to_paths_uncommitted(&request.conversions)
                .map_err(|_| TilingError::Conversions)?;
        }
        let created = plan.tiling.is_none();
        let group = match plan.tiling {
            Some(group) => group,
            None => create_group(&tree, &plan.sources),
        };
        let mut written = created;
        written |= place_sources(&tree, group, &plan, request.recipe.keep_source);
        written |= crate::tiling_regenerate::regenerate(self, &tree, group, &plan, request, mint);
        // invariant: the group was confirmed live or was just created.
        #[allow(clippy::unwrap_used)]
        let meta = tree.get_meta(group).unwrap();
        if plan.stored != Some(request.recipe) {
            crate::tiling_codec::write_recipe(&meta, &request.recipe);
            written = true;
        }
        if crate::tiling_codec::read_sources(&meta).as_deref() != Some(plan.sources.as_slice()) {
            crate::tiling_codec::write_sources(&meta, &plan.sources);
            written = true;
        }
        if written {
            self.commit_with_label(if created {
                TILE_COMMIT_LABEL
            } else {
                EDIT_TILING_COMMIT_LABEL
            });
        }
        Ok(TilingOutcome {
            tiling: node_id(group),
            written,
        })
    }

    /// Turns the tiling `group` into a plain group of its instances in **one commit**
    /// (criterion 32): the recipe, the source list and every cell and member key are removed.
    ///
    /// # Errors
    /// [`TilingError::NotATiling`] when `group` carries no recipe; nothing is written then.
    pub fn release_tiling(&self, group: NodeId) -> Result<(), TilingError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let id = tree_id_of(group);
        let is_tiling = node_exists(&tree, id)
            && tree
                .get_meta(id)
                .is_ok_and(|meta| crate::tiling_codec::has_recipe(&meta));
        if !is_tiling {
            return Err(TilingError::NotATiling);
        }
        crate::tiling_copy::strip_tiling(&tree, id);
        self.commit_with_label(RELEASE_TILING_COMMIT_LABEL);
        Ok(())
    }
}

fn check_depth(
    tree: &LoroTree,
    tiling: Option<TreeID>,
    sources: &[NodeId],
    multi: bool,
) -> Result<(), TilingError> {
    let parent = match tiling {
        Some(group) => group_parent(tree, group),
        None => sources
            .first()
            .and_then(|s| group_parent(tree, tree_id_of(*s))),
    };
    let above = parent.map_or(0, |p| groups_above(tree, p) + 1);
    let inside = sources
        .iter()
        .map(|s| groups_in(tree, tree_id_of(*s)))
        .max()
        .unwrap_or(0)
        .max(usize::from(multi));
    if above + 1 + inside > MAX_GROUP_DEPTH {
        Err(TilingError::TooDeep)
    } else {
        Ok(())
    }
}

/// Every source is live, neither a layer nor holding a tiling, and unlocked (criterion 4).
fn check_sources(tree: &LoroTree, sources: &[NodeId]) -> Result<(), TilingError> {
    for source in sources {
        let id = tree_id_of(*source);
        if !node_exists(tree, id) {
            return Err(TilingError::SourceMissing);
        }
        if crate::group_tree::is_layer_node(tree, id) {
            return Err(TilingError::IsALayer);
        }
        if holds_tiling(tree, id) {
            return Err(TilingError::ContainsTiling);
        }
        if is_locked(tree, id) {
            return Err(TilingError::Locked);
        }
    }
    Ok(())
}

/// The conversions name exactly the primitive leaves, each once.
fn check_conversions(
    read: &ObjectRead,
    leaves: &[Leaf],
    request: &TilingRequest,
) -> Result<(), TilingError> {
    let primitives: HashSet<NodeId> = leaves
        .iter()
        .filter(|leaf| matches!(read.get(leaf.id), Some(ObjectSnapshot::Primitive(_))))
        .map(|leaf| leaf.id)
        .collect();
    let converted: HashSet<NodeId> = request.conversions.iter().map(|(id, _)| *id).collect();
    if converted != primitives || converted.len() != request.conversions.len() {
        return Err(TilingError::Conversions);
    }
    Ok(())
}

fn check_cells(request: &TilingRequest) -> Result<(), TilingError> {
    let mut keys = HashSet::with_capacity(request.cells.len());
    let fits = request.cells.iter().all(|cell| {
        keys.insert(cell.key)
            && request.recipe.has_cell(cell.key)
            && !(request.recipe.keep_source && cell.key == CellKey::ORIGIN)
            && cell.affine.is_finite()
            && cell.affine.is_within_limit()
            && cell.affine.determinant().abs() > crate::affine::SINGULAR_DETERMINANT
            && cell.opacity.get().is_finite()
            && cell.hue.as_radians().is_finite()
    });
    let count_ok =
        u64::try_from(request.cells.len()).is_ok_and(|n| n <= request.recipe.instance_count());
    if fits && count_ok {
        Ok(())
    } else {
        Err(TilingError::Cells)
    }
}

/// Creates the group "Tiling" directly above the topmost source, in its parent. Uncommitted.
fn create_group(tree: &LoroTree, sources: &[NodeId]) -> TreeID {
    let order = paint_order(tree);
    // invariant: the plan holds at least one live source.
    #[allow(clippy::unwrap_used)]
    let topmost = sources
        .iter()
        .map(|s| tree_id_of(*s))
        .max_by_key(|id| order.get(id).copied().unwrap_or(0))
        .unwrap();
    let parent = group_parent(tree, topmost);
    let siblings = children_of(tree, parent);
    let at = siblings
        .iter()
        .position(|s| *s == topmost)
        .map_or(siblings.len(), |p| p + 1);
    let parent_id = parent.map_or(TreeParentId::Root, TreeParentId::Node);
    // invariant: the parent is live and the index is at most the child count; the meta map of
    // a node just created can be read and written.
    #[allow(clippy::unwrap_used)]
    let group = tree.create_at(parent_id, at).unwrap();
    #[allow(clippy::unwrap_used)]
    let meta = tree.get_meta(group).unwrap();
    #[allow(clippy::unwrap_used)]
    meta.insert(KEY_GROUP, true).unwrap();
    #[allow(clippy::unwrap_used)]
    meta.insert(crate::node_registers::KEY_NAME, TILING_GROUP_NAME)
        .unwrap();
    group
}

/// Moves the sources in as the lowest children (keep source on) or out to directly below the
/// group (off). Returns whether anything moved. Uncommitted.
fn place_sources(tree: &LoroTree, group: TreeID, plan: &Plan, keep: bool) -> bool {
    let order = paint_order(tree);
    let mut sources: Vec<TreeID> = plan.sources.iter().map(|s| tree_id_of(*s)).collect();
    sources.sort_by_key(|id| order.get(id).copied().unwrap_or(usize::MAX));
    let inside = |id: &TreeID| group_parent(tree, *id) == Some(group);
    if keep {
        if sources
            .iter()
            .enumerate()
            .all(|(i, id)| inside(id) && children_of(tree, Some(group)).get(i) == Some(id))
        {
            return false;
        }
        for (index, source) in sources.iter().enumerate() {
            // invariant: the source and the group are live and the group is not below the source.
            #[allow(clippy::unwrap_used)]
            tree.mov_to(*source, TreeParentId::Node(group), index)
                .unwrap();
        }
        return true;
    }
    if !sources.iter().any(inside) {
        return false;
    }
    let parent = group_parent(tree, group).map_or(TreeParentId::Root, TreeParentId::Node);
    for source in sources.iter().filter(|id| inside(id)) {
        let siblings = children_of(tree, group_parent(tree, group));
        let at = siblings.iter().position(|s| *s == group).unwrap_or(0);
        // invariant: the source and the group's parent are live.
        #[allow(clippy::unwrap_used)]
        tree.mov_to(*source, parent, at).unwrap();
    }
    true
}
