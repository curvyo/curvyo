//! Movement exclusions and aggregate group bounds over one resolved read.
use super::{HashSet, NodeId, NodeRead, ObjectSnapshot, Point, object_bounds};
use std::collections::{HashMap, VecDeque};

pub(super) fn moving(nodes: &[NodeRead], selected: Option<&HashSet<NodeId>>) -> HashSet<NodeId> {
    let mut result = selected.cloned().unwrap_or_default();
    let mut dependents: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for node in nodes {
        if let Some(parent) = node.parent {
            dependents.entry(parent).or_default().push(node.id);
        }
        if let Some(ObjectSnapshot::Clone(clone)) = node.object()
            && !clone.chain.own_placement
        {
            dependents
                .entry(clone.chain.origin)
                .or_default()
                .push(node.id);
        }
    }
    let mut queue: VecDeque<_> = result.iter().copied().collect();
    while let Some(id) = queue.pop_front() {
        for &child in dependents.get(&id).into_iter().flatten() {
            if result.insert(child) {
                queue.push_back(child);
            }
        }
    }
    result
}

pub(super) fn group_bounds(nodes: &[NodeRead]) -> HashMap<NodeId, (Point, Point)> {
    let mut bounds = HashMap::new();
    for node in nodes.iter().rev().filter(|n| n.visible) {
        if !node.guide
            && let Some(object) = node.object()
        {
            bounds.insert(node.id, object_bounds(object));
        }
        if let (Some(parent), Some(&(lo, hi))) = (node.parent, bounds.get(&node.id)) {
            bounds
                .entry(parent)
                .and_modify(|(a, b)| {
                    *a = Point::new(a.x.min(lo.x), a.y.min(lo.y));
                    *b = Point::new(b.x.max(hi.x), b.y.max(hi.y));
                })
                .or_insert((lo, hi));
        }
    }
    bounds
}

/// Aggregate containers change whenever one of their descendants moves.
pub(super) fn moving_groups(nodes: &[NodeRead], moving: &HashSet<NodeId>) -> HashSet<NodeId> {
    let mut ancestors = moving.clone();
    for node in nodes.iter().rev() {
        if ancestors.contains(&node.id)
            && let Some(parent) = node.parent
        {
            ancestors.insert(parent);
        }
    }
    ancestors
}
