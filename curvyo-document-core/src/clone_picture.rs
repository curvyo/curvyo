//! What a clone of a picture root draws (ADR 0045 §1 to §4): the drawn subtree of a group, or a
//! text or image itself, mapped once per clone, with the clones on a containment cycle read as
//! orphans.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::affine::Affine;
use crate::path_model::{AnchorId, NewAnchor, NodeId, PathSnapshot};
use crate::primitive_model::{ObjectSnapshot, PrimitiveSnapshot};
use crate::style_model::Opacity;
use crate::units::Angle;

/// The containers and registers the read of a picture root needs, aligned with the read's leaves
/// (index i is leaf i of the resolver's input).
#[derive(Debug, Clone, Default)]
pub(crate) struct Structure {
    /// Each leaf's container (a layer or a group), `None` at the top level.
    parent: Vec<Option<NodeId>>,
    /// Each leaf's own `visible` register.
    visible: Vec<bool>,
    /// Each leaf's own `opacity` register.
    opacity: Vec<f64>,
    /// Each leaf's own `guide` register.
    guide: Vec<bool>,
    /// Every container by id.
    containers: HashMap<NodeId, Container>,
}

/// One layer or group.
#[derive(Debug, Clone, Copy)]
struct Container {
    parent: Option<NodeId>,
    visible: bool,
    opacity: f64,
    layer: bool,
}

impl Structure {
    /// Notes a container met by the walk.
    pub(crate) fn container(
        &mut self,
        id: NodeId,
        parent: Option<NodeId>,
        registers: &crate::node_registers::NodeRegisters,
        layer: bool,
    ) {
        self.containers.insert(
            id,
            Container {
                parent,
                visible: registers.visible,
                opacity: registers.opacity.get(),
                layer,
            },
        );
    }

    /// Notes the next leaf of the resolver's input.
    pub(crate) fn leaf(
        &mut self,
        parent: Option<NodeId>,
        registers: &crate::node_registers::NodeRegisters,
    ) {
        self.parent.push(parent);
        self.visible.push(registers.visible);
        self.opacity.push(registers.opacity.get());
        self.guide.push(registers.guide);
    }

    /// Whether `id` is a group (not a layer): a legal origin.
    pub(crate) fn is_group(&self, id: NodeId) -> bool {
        self.containers.get(&id).is_some_and(|c| !c.layer)
    }

    /// Each leaf's own opacity register.
    fn own_opacity(&self, leaf: usize) -> f64 {
        self.opacity.get(leaf).copied().unwrap_or(1.0)
    }

    /// The leaf ranges of every container: depth first order makes them contiguous.
    fn ranges(&self) -> HashMap<NodeId, (usize, usize)> {
        let mut ranges: HashMap<NodeId, (usize, usize)> = HashMap::new();
        for (leaf, parent) in self.parent.iter().enumerate() {
            let mut current = *parent;
            let mut guard = 0;
            while let Some(container) = current {
                let range = ranges.entry(container).or_insert((leaf, leaf + 1));
                range.0 = range.0.min(leaf);
                range.1 = range.1.max(leaf + 1);
                current = self.containers.get(&container).and_then(|c| c.parent);
                guard += 1;
                if guard > self.containers.len() {
                    break;
                }
            }
        }
        ranges
    }

    /// Whether leaf `leaf` is shown inside group `group` and the opacity of the containers from
    /// its parent up to and including `group` (ADR 0045 §2: `group`'s own flag does not hide).
    fn through(&self, leaf: usize, group: NodeId) -> (bool, f64) {
        let mut shown = self.visible.get(leaf).copied().unwrap_or(true);
        let mut opacity = 1.0;
        let mut current = self.parent.get(leaf).copied().flatten();
        let mut guard = 0;
        while let Some(id) = current {
            let Some(container) = self.containers.get(&id) else {
                break;
            };
            opacity *= container.opacity;
            if id == group {
                break;
            }
            shown &= container.visible;
            current = container.parent;
            guard += 1;
            if guard > self.containers.len() {
                break;
            }
        }
        (shown, opacity)
    }
}

/// `leaf` mapped by `map` (a leaf of a picture is a path, a text or an image; any other kind is
/// mapped as it is drawn: a primitive or a sketch as its outline path).
#[must_use]
pub fn map_leaf(leaf: &ObjectSnapshot, map: Affine) -> ObjectSnapshot {
    match leaf {
        ObjectSnapshot::Path(path) => ObjectSnapshot::Path(path.mapped(map)),
        ObjectSnapshot::Text(text) => ObjectSnapshot::Text(Box::new(text.mapped_by(map))),
        ObjectSnapshot::Image(image) => ObjectSnapshot::Image(image.mapped_by(map)),
        ObjectSnapshot::Primitive(primitive) => {
            ObjectSnapshot::Path(primitive_path(primitive).mapped(map))
        }
        ObjectSnapshot::Sketch(sketch) => ObjectSnapshot::Path(sketch.resolved.mapped(map)),
        ObjectSnapshot::Clone(clone) => ObjectSnapshot::Clone(clone.mapped_by(map)),
        ObjectSnapshot::Dimension(dimension) => {
            ObjectSnapshot::Dimension(Box::new(dimension.mapped_by(map)))
        }
    }
}

/// A primitive's drawn outline as a path with its id and style (ADR 0045 §3: a primitive is never
/// written by a clone, only drawn as its outline). The anchor ids are positions, not stored ids.
#[must_use]
pub(crate) fn primitive_path(primitive: &PrimitiveSnapshot) -> PathSnapshot {
    let outline =
        crate::primitive_outline::drawing_outline_of_rotated(&primitive.shape, primitive.rotation);
    PathSnapshot {
        id: primitive.id,
        closed: outline.closed,
        style: primitive.style.clone(),
        anchors: outline
            .anchors
            .iter()
            .zip(0..)
            .map(|(a, k)| NewAnchor {
                id: AnchorId::new(0, k),
                point: a.point,
                handle_in: a.handle_in,
                handle_out: a.handle_out,
                kind: a.kind,
            })
            .collect(),
        extra_subpaths: Vec::new(),
        rotation: Angle::from_radians(0.0),
    }
}

/// `object` with each paint's opacity multiplied by `opacity`.
fn faded(mut object: ObjectSnapshot, opacity: f64) -> ObjectSnapshot {
    if (opacity - 1.0).abs() < f64::EPSILON {
        return object;
    }
    // invariant: a product of numbers in 0..=1 lies in 0..=1.
    let times = |paint: Opacity| Opacity::new(paint.get() * opacity).unwrap_or(paint);
    if let ObjectSnapshot::Image(image) = &mut object {
        image.shown_opacity = times(image.shown_opacity);
        return object;
    }
    let style = object.style_mut();
    style.fill.opacity = times(style.fill.opacity);
    style.stroke.opacity = times(style.stroke.opacity);
    object
}

/// What a clone's chain ends at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RootRef {
    /// A leaf of the read: a path, sketch, primitive (outline roots), text or image.
    Leaf(usize),
    /// A group (a picture root).
    Group(NodeId),
}

/// A clone whose chain ends at a picture root, resolved once every other leaf is read.
#[derive(Debug, Clone)]
pub(crate) struct Pending {
    /// Its leaf index.
    pub(crate) at: usize,
    /// Its id.
    pub(crate) id: NodeId,
    /// Its stored link.
    pub(crate) link: crate::clone_codec::CloneLink,
    /// Its own-placement flag.
    pub(crate) own_placement: bool,
    /// Where its chain ends.
    pub(crate) root: RootRef,
    /// Its depth.
    pub(crate) depth: usize,
    /// Its composed matrix.
    pub(crate) composed: Affine,
}

/// Resolves every pending picture clone into `leaves` (input order), adding the orphans to
/// `orphans` and the direct clones to `clones_of`. A clone on a cycle of the edges clone →
/// origin and group → child (a non-trivial strongly connected component or a self loop) is an
/// orphan, and so is every clone whose origin is one (ADR 0045 §4); the result depends only on
/// the leaves.
pub(crate) fn resolve_pictures(
    pending: &[Pending],
    leaves: &mut [Option<ObjectSnapshot>],
    structure: &Structure,
    index: &HashMap<NodeId, usize>,
    orphans: &mut Vec<NodeId>,
    clones_of: &mut HashMap<NodeId, Vec<NodeId>>,
) {
    if pending.is_empty() {
        return;
    }
    let graph = Graph::new(pending, structure, index);
    let ranges = structure.ranges();
    let mut lists: HashMap<NodeId, Arc<[ObjectSnapshot]>> = HashMap::new();
    let mut orphaned: HashSet<usize> = HashSet::new();
    for component in graph.components() {
        let cyclic = component.len() > 1 || graph.loops(component[0]);
        for &node in &component {
            if let Node::Clone(p) = graph.nodes[node]
                && cyclic
            {
                orphaned.insert(pending[p].at);
            }
        }
        for &node in &component {
            match graph.nodes[node] {
                Node::Group(group) => {
                    let list = group_list(group, &ranges, leaves, structure, index);
                    lists.insert(group, list);
                }
                Node::Clone(p) => {
                    let clone = &pending[p];
                    if orphaned.contains(&clone.at) {
                        continue;
                    }
                    let origin_lost = index.get(&clone.link.origin).is_some_and(|&o| {
                        graph.pending_at.contains_key(&o) && orphaned.contains(&o)
                    });
                    let resolved = (!origin_lost)
                        .then(|| picture_clone(clone, leaves, &lists))
                        .flatten();
                    match resolved {
                        Some(snapshot) => leaves[clone.at] = Some(ObjectSnapshot::Clone(snapshot)),
                        None => {
                            orphaned.insert(clone.at);
                        }
                    }
                }
            }
        }
    }
    for clone in pending {
        if orphaned.contains(&clone.at) {
            orphans.push(clone.id);
        } else {
            clones_of
                .entry(clone.link.origin)
                .or_default()
                .push(clone.id);
        }
    }
    orphans.sort_by_key(|id| index.get(id).copied().unwrap_or(usize::MAX));
}

/// The clone `clone` with its root's drawn leaves mapped by its composed matrix, or `None` when
/// its root draws nothing readable or a mapped leaf is not finite.
fn picture_clone(
    clone: &Pending,
    leaves: &[Option<ObjectSnapshot>],
    lists: &HashMap<NodeId, Arc<[ObjectSnapshot]>>,
) -> Option<crate::clone_model::CloneSnapshot> {
    let (root_id, source): (NodeId, Vec<ObjectSnapshot>) = match clone.root {
        RootRef::Group(group) => (group, lists.get(&group)?.to_vec()),
        RootRef::Leaf(at) => {
            let leaf = leaves.get(at)?.as_ref()?;
            match leaf {
                ObjectSnapshot::Text(_) | ObjectSnapshot::Image(_) => {
                    (leaf.id(), vec![leaf.clone()])
                }
                _ => return None,
            }
        }
    };
    let picture: Arc<[ObjectSnapshot]> = source
        .iter()
        .map(|leaf| map_leaf(leaf, clone.composed))
        .collect();
    if !picture.iter().all(leaf_is_finite) {
        return None;
    }
    let empty = PathSnapshot {
        id: root_id,
        closed: false,
        style: crate::style_model::Style::default(),
        anchors: Vec::new(),
        extra_subpaths: Vec::new(),
        rotation: Angle::from_radians(0.0),
    };
    Some(crate::clone_model::CloneSnapshot::new(
        clone.id,
        crate::clone_model::CloneChain {
            origin: clone.link.origin,
            root: root_id,
            depth: clone.depth,
            matrix: clone.link.matrix,
            composed: clone.composed,
            own_placement: clone.own_placement,
            own_style: false,
            style_source: root_id,
            root_outline: Arc::new(empty),
            picture: Some(picture),
        },
    ))
}

fn leaf_is_finite(leaf: &ObjectSnapshot) -> bool {
    match leaf {
        ObjectSnapshot::Text(text) => text.placement.is_finite(),
        ObjectSnapshot::Image(image) => image.placement.is_finite(),
        _ => leaf.drawn_path().is_none_or(|path| {
            path.all_anchors().all(|a| {
                a.point.x.is_finite()
                    && a.point.y.is_finite()
                    && a.handle_in.x.is_finite()
                    && a.handle_in.y.is_finite()
                    && a.handle_out.x.is_finite()
                    && a.handle_out.y.is_finite()
            })
        }),
    }
}

/// The drawn leaves of `group` in stacking order (ADR 0045 §2): a leaf hidden by its own flag or
/// a container strictly below `group` is left out, and so are guides, dimensions and orphans;
/// each keeps its style with its opacity through `group` baked in; a clone inside contributes
/// what it draws.
fn group_list(
    group: NodeId,
    ranges: &HashMap<NodeId, (usize, usize)>,
    leaves: &[Option<ObjectSnapshot>],
    structure: &Structure,
    index: &HashMap<NodeId, usize>,
) -> Arc<[ObjectSnapshot]> {
    let Some(&(start, end)) = ranges.get(&group) else {
        return Arc::from(Vec::new());
    };
    let mut list = Vec::new();
    for at in start..end {
        let Some(object) = leaves.get(at).and_then(Option::as_ref) else {
            continue;
        };
        let (shown, through) = structure.through(at, group);
        if !shown {
            continue;
        }
        let own_guide = structure.guide.get(at).copied().unwrap_or(false);
        let leaf_opacity = |id: NodeId| {
            index
                .get(&id)
                .map_or(1.0, |&leaf| structure.own_opacity(leaf))
        };
        match object {
            ObjectSnapshot::Dimension(_) => {}
            ObjectSnapshot::Clone(clone) if clone.is_picture() => {
                let own = leaf_opacity(clone.chain.style_source);
                for leaf in clone.chain.picture.iter().flat_map(|p| p.iter()) {
                    list.push(faded(leaf.clone(), through * own));
                }
            }
            ObjectSnapshot::Clone(clone) => {
                let root_guide = index
                    .get(&clone.chain.root)
                    .is_some_and(|&root| structure.guide.get(root).copied().unwrap_or(false));
                if own_guide || root_guide {
                    continue;
                }
                let own = leaf_opacity(clone.chain.style_source);
                list.push(faded(
                    ObjectSnapshot::Path(clone.resolved.clone()),
                    through * own,
                ));
            }
            _ if own_guide => {}
            ObjectSnapshot::Primitive(primitive) => {
                let own = structure.own_opacity(at);
                list.push(faded(
                    ObjectSnapshot::Path(primitive_path(primitive)),
                    through * own,
                ));
            }
            ObjectSnapshot::Sketch(sketch) => {
                let own = structure.own_opacity(at);
                list.push(faded(
                    ObjectSnapshot::Path(sketch.resolved.clone()),
                    through * own,
                ));
            }
            ObjectSnapshot::Path(_) | ObjectSnapshot::Text(_) | ObjectSnapshot::Image(_) => {
                let own = structure.own_opacity(at);
                list.push(faded(object.clone(), through * own));
            }
        }
    }
    list.into()
}

/// A node of the dependency graph.
#[derive(Debug, Clone, Copy)]
enum Node {
    /// A pending clone, by its place in the pending list.
    Clone(usize),
    /// A group.
    Group(NodeId),
}

/// The dependency edges between pending clones and groups.
struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Vec<usize>>,
    /// The pending clones by leaf index.
    pending_at: HashMap<usize, usize>,
}

impl Graph {
    fn new(pending: &[Pending], structure: &Structure, index: &HashMap<NodeId, usize>) -> Self {
        let mut nodes: Vec<Node> = (0..pending.len()).map(Node::Clone).collect();
        let pending_at: HashMap<usize, usize> =
            pending.iter().enumerate().map(|(p, c)| (c.at, p)).collect();
        let mut group_node: HashMap<NodeId, usize> = HashMap::new();
        let mut node_of_group = |group: NodeId, nodes: &mut Vec<Node>| {
            *group_node.entry(group).or_insert_with(|| {
                nodes.push(Node::Group(group));
                nodes.len() - 1
            })
        };
        // The children of each group: its child groups and the pending clones directly in it.
        let mut children: HashMap<NodeId, Vec<Node>> = HashMap::new();
        for (&id, container) in &structure.containers {
            if let Some(parent) = container.parent
                && !container.layer
                && structure.is_group(parent)
            {
                children.entry(parent).or_default().push(Node::Group(id));
            }
        }
        for (&at, &p) in &pending_at {
            if let Some(parent) = structure.parent.get(at).copied().flatten()
                && structure.is_group(parent)
            {
                children.entry(parent).or_default().push(Node::Clone(p));
            }
        }
        let mut edges: Vec<Vec<usize>> = vec![Vec::new(); pending.len()];
        let mut todo: Vec<NodeId> = Vec::new();
        for (p, clone) in pending.iter().enumerate() {
            let origin = clone.link.origin;
            if let Some(o) = index.get(&origin).and_then(|o| pending_at.get(o)) {
                edges[p].push(*o);
            } else if structure.is_group(origin) {
                let g = node_of_group(origin, &mut nodes);
                edges[p].push(g);
                todo.push(origin);
            }
        }
        let mut done: HashSet<NodeId> = HashSet::new();
        while let Some(group) = todo.pop() {
            if !done.insert(group) {
                continue;
            }
            let g = node_of_group(group, &mut nodes);
            let mut out = Vec::new();
            for child in children.get(&group).into_iter().flatten() {
                match *child {
                    Node::Clone(p) => out.push(p),
                    Node::Group(c) => {
                        out.push(node_of_group(c, &mut nodes));
                        todo.push(c);
                    }
                }
            }
            if edges.len() < nodes.len() {
                edges.resize(nodes.len(), Vec::new());
            }
            edges[g] = out;
        }
        edges.resize(nodes.len(), Vec::new());
        // A stable order of the edges keeps the walk independent of hash order.
        for out in &mut edges {
            out.sort_unstable();
        }
        Self {
            nodes,
            edges,
            pending_at,
        }
    }

    fn loops(&self, node: usize) -> bool {
        self.edges[node].contains(&node)
    }

    /// The strongly connected components, each after every component it depends on (Tarjan,
    /// iterative).
    fn components(&self) -> Vec<Vec<usize>> {
        let n = self.nodes.len();
        let mut order = vec![usize::MAX; n];
        let mut low = vec![0; n];
        let mut on_stack = vec![false; n];
        let mut stack: Vec<usize> = Vec::new();
        let mut result = Vec::new();
        let mut counter = 0;
        for start in 0..n {
            if order[start] != usize::MAX {
                continue;
            }
            let mut calls: Vec<(usize, usize)> = vec![(start, 0)];
            order[start] = counter;
            low[start] = counter;
            counter += 1;
            stack.push(start);
            on_stack[start] = true;
            while let Some(&mut (node, ref mut next)) = calls.last_mut() {
                if let Some(&to) = self.edges[node].get(*next) {
                    *next += 1;
                    if order[to] == usize::MAX {
                        order[to] = counter;
                        low[to] = counter;
                        counter += 1;
                        stack.push(to);
                        on_stack[to] = true;
                        calls.push((to, 0));
                    } else if on_stack[to] {
                        low[node] = low[node].min(order[to]);
                    }
                    continue;
                }
                calls.pop();
                if let Some(&(parent, _)) = calls.last() {
                    low[parent] = low[parent].min(low[node]);
                }
                if low[node] == order[node] {
                    let mut component = Vec::new();
                    while let Some(top) = stack.pop() {
                        on_stack[top] = false;
                        component.push(top);
                        if top == node {
                            break;
                        }
                    }
                    result.push(component);
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clone_read::tests_support::{clone, id, square};
    use crate::clone_read::{RawLeaf, resolve_in};
    use crate::node_registers::NodeRegisters;

    /// The leaves and containers of `layout`: (leaf, parent group) in stacking order, groups
    /// with their parent.
    fn read(
        leaves: Vec<(RawLeaf, Option<i32>)>,
        groups: &[(i32, Option<i32>)],
        reverse: bool,
    ) -> crate::clone_read::ObjectRead {
        let mut structure = Structure::default();
        let registers = NodeRegisters::default();
        let mut order: Vec<(i32, Option<i32>)> = groups.to_vec();
        if reverse {
            order.reverse();
        }
        for (group, parent) in order {
            structure.container(id(group), parent.map(id), &registers, false);
        }
        let mut raws = Vec::new();
        for (leaf, parent) in leaves {
            structure.leaf(parent.map(id), &registers);
            raws.push(leaf);
        }
        resolve_in(raws, Some(&structure)).into_read()
    }

    #[test]
    fn a_clone_inside_its_own_original_is_an_orphan_and_the_group_still_draws() {
        // G (100) holds the square 1 and C (2), a clone of G; D (3) outside clones G too.
        let leaves = vec![
            (square(1), Some(100)),
            (clone(2, 100, Affine::IDENTITY), Some(100)),
            (clone(3, 100, Affine::IDENTITY), None),
        ];
        let read = read(leaves, &[(100, None)], false);
        assert_eq!(read.orphans, vec![id(2)]);
        let Some(ObjectSnapshot::Clone(d)) = read.get(id(3)) else {
            panic!("D resolves")
        };
        assert_eq!(d.chain.picture.as_ref().unwrap().len(), 1, "G without C");
    }

    #[test]
    fn two_groups_that_draw_each_other_orphan_both_clones_whatever_the_order() {
        // C1 (2) clones G1 (100) and lies in G2 (200); C2 (4) clones G2 and lies in G1.
        let leaves = || {
            vec![
                (square(1), Some(100)),
                (clone(4, 200, Affine::IDENTITY), Some(100)),
                (square(3), Some(200)),
                (clone(2, 100, Affine::IDENTITY), Some(200)),
                (clone(5, 100, Affine::IDENTITY), None),
            ]
        };
        for reverse in [false, true] {
            let read = read(leaves(), &[(100, None), (200, None)], reverse);
            assert_eq!(read.orphans, vec![id(4), id(2)], "stacking order");
            let Some(ObjectSnapshot::Clone(outside)) = read.get(id(5)) else {
                panic!("the clone outside resolves")
            };
            assert_eq!(outside.chain.picture.as_ref().unwrap().len(), 1);
        }
    }

    #[test]
    fn a_clone_of_a_member_inside_the_group_is_no_cycle() {
        let leaves = vec![
            (square(1), Some(100)),
            (clone(2, 1, Affine::IDENTITY), Some(100)),
            (clone(3, 100, Affine::IDENTITY), None),
        ];
        let read = read(leaves, &[(100, None)], false);
        assert_eq!(read.orphans, Vec::<NodeId>::new());
        let Some(ObjectSnapshot::Clone(c)) = read.get(id(3)) else {
            panic!()
        };
        assert_eq!(c.chain.picture.as_ref().unwrap().len(), 2);
    }
}
