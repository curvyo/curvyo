//! Contour assembly follows explicit coincidence without swallowing attached branches.
use crate::{EntityId, Relation, Sketch};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
type Edges = BTreeMap<EntityId, (EntityId, EntityId)>;
type Step = (EntityId, bool);
type Adjacency = BTreeMap<EntityId, Vec<(EntityId, EntityId, bool)>>;
/// An oriented trail through existing segment records.
pub(crate) struct Contour {
    pub edges: Vec<Step>,
    pub closed: bool,
}

pub(crate) fn contours(s: &Sketch, original: &Edges) -> Vec<Contour> {
    let mut parents: BTreeMap<_, _> = original
        .values()
        .flat_map(|(a, b)| [(a.clone(), a.clone()), (b.clone(), b.clone())])
        .collect();
    for c in s.constraints.values() {
        if !c.reference
            && let Relation::Coincident(a, b) = &c.relation
        {
            parents.entry(a.clone()).or_insert_with(|| a.clone());
            parents.entry(b.clone()).or_insert_with(|| b.clone());
            let a = root(&parents, a);
            let b = root(&parents, b);
            let (small, large) = if a < b { (a, b) } else { (b, a) };
            parents.insert(large, small);
        }
    }
    let edges: Edges = original
        .iter()
        .map(|(id, (a, b))| (id.clone(), (root(&parents, a), root(&parents, b))))
        .collect();
    let mut adjacent = Adjacency::new();
    for (id, (a, b)) in &edges {
        adjacent
            .entry(a.clone())
            .or_default()
            .push((id.clone(), b.clone(), false));
        if a != b {
            adjacent
                .entry(b.clone())
                .or_default()
                .push((id.clone(), a.clone(), true));
        }
    }
    let mut used = BTreeSet::new();
    let mut contours = vec![];
    // Extract cycles first so a spur cannot turn a closed manufacturing loop into an open trail.
    for (id, (start, end)) in &edges {
        if used.contains(id) {
            continue;
        }
        if let Some(path) = return_path(&adjacent, &used, id, end, start) {
            let mut contour = vec![(id.clone(), false)];
            contour.extend(path);
            for (edge, _) in &contour {
                used.insert(edge.clone());
            }
            contours.push(Contour {
                edges: contour,
                closed: true,
            });
        }
    }
    for (id, (start, end)) in &edges {
        if used.contains(id) {
            continue;
        }
        let mut contour = vec![(id.clone(), false)];
        used.insert(id.clone());
        let mut cursor = end.clone();
        while let Some((edge, next, reverse)) = adjacent
            .get(&cursor)
            .into_iter()
            .flatten()
            .find(|(id, _, _)| !used.contains(id))
        {
            contour.push((edge.clone(), *reverse));
            used.insert(edge.clone());
            cursor = next.clone();
        }
        contours.push(Contour {
            edges: contour,
            closed: cursor == *start,
        });
    }
    contours
}
fn root(parents: &BTreeMap<EntityId, EntityId>, id: &EntityId) -> EntityId {
    let mut current = id.clone();
    while let Some(parent) = parents.get(&current) {
        if parent == &current {
            break;
        }
        current = parent.clone();
    }
    current
}
fn return_path(
    adjacent: &Adjacency,
    used: &BTreeSet<EntityId>,
    excluded: &EntityId,
    start: &EntityId,
    end: &EntityId,
) -> Option<Vec<Step>> {
    let mut queue = VecDeque::from([start.clone()]);
    let mut seen = BTreeSet::from([start.clone()]);
    let mut parents: BTreeMap<EntityId, (EntityId, Step)> = BTreeMap::new();
    while let Some(current) = queue.pop_front() {
        if current == *end {
            let mut path = vec![];
            let mut node = end.clone();
            while node != *start {
                let (previous, step) = parents.get(&node)?;
                path.push(step.clone());
                node = previous.clone();
            }
            path.reverse();
            return Some(path);
        }
        for (id, next, reverse) in adjacent.get(&current).into_iter().flatten() {
            if id == excluded || used.contains(id) {
                continue;
            }
            if seen.insert(next.clone()) {
                parents.insert(next.clone(), (current.clone(), (id.clone(), *reverse)));
                queue.push_back(next.clone());
            }
        }
    }
    None
}
