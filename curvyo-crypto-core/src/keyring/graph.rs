//! The entry DAG: deduplication by hash, a deterministic topological order and ancestor sets.

use std::collections::{BTreeMap, BTreeSet};

use crate::keyring::{Entry, EntryHash};

/// Entries indexed by position, with the causal relation the replay rules ask about.
pub(crate) struct Graph {
    pub(crate) entries: Vec<Entry>,
    pub(crate) hashes: Vec<EntryHash>,
    pub(crate) parents: Vec<Vec<usize>>,
    /// Topological order (parents first, ties by lowest hash); entries with a missing ancestor
    /// are absent.
    pub(crate) order: Vec<usize>,
    /// Strict ancestors of each ordered entry.
    pub(crate) ancestors: Vec<BTreeSet<usize>>,
    /// Position in `order`, `usize::MAX` for unresolved entries.
    pub(crate) position: Vec<usize>,
}

impl Graph {
    /// Builds the graph; entries that fail to hash are dropped, duplicates are kept once.
    pub(crate) fn new(input: &[Entry]) -> Self {
        let mut by_hash: BTreeMap<EntryHash, Entry> = BTreeMap::new();
        for entry in input {
            if let Ok(hash) = entry.try_hash() {
                by_hash.entry(hash).or_insert_with(|| entry.clone());
            }
        }
        let hashes: Vec<EntryHash> = by_hash.keys().copied().collect();
        let entries: Vec<Entry> = by_hash.into_values().collect();
        let index: BTreeMap<EntryHash, usize> =
            hashes.iter().enumerate().map(|(i, h)| (*h, i)).collect();
        let mut parents = Vec::with_capacity(entries.len());
        let mut complete = Vec::with_capacity(entries.len());
        for entry in &entries {
            let found: Vec<usize> = entry
                .parents
                .iter()
                .filter_map(|p| index.get(p).copied())
                .collect();
            complete.push(found.len() == entry.parents.len());
            parents.push(found);
        }
        let order = topological_order(&parents, &complete);
        let mut ancestors = vec![BTreeSet::new(); entries.len()];
        let mut position = vec![usize::MAX; entries.len()];
        for (pos, &i) in order.iter().enumerate() {
            position[i] = pos;
            let mut set = BTreeSet::new();
            for &p in &parents[i] {
                set.insert(p);
                set.extend(ancestors[p].iter().copied());
            }
            ancestors[i] = set;
        }
        Self {
            entries,
            hashes,
            parents,
            order,
            ancestors,
            position,
        }
    }

    /// Whether `a` is a strict ancestor of `b`.
    pub(crate) fn is_ancestor(&self, a: usize, b: usize) -> bool {
        self.ancestors[b].contains(&a)
    }
}

/// Kahn's algorithm with the lowest index (= lowest hash) first; entries with a missing
/// parent, and everything after them, stay out.
fn topological_order(parents: &[Vec<usize>], complete: &[bool]) -> Vec<usize> {
    let mut children = vec![Vec::new(); parents.len()];
    let mut waiting: Vec<usize> = parents.iter().map(Vec::len).collect();
    for (i, ps) in parents.iter().enumerate() {
        for &p in ps {
            children[p].push(i);
        }
    }
    let mut ready: BTreeSet<usize> = (0..parents.len())
        .filter(|&i| waiting[i] == 0 && complete[i])
        .collect();
    let mut order = Vec::with_capacity(parents.len());
    while let Some(i) = ready.pop_first() {
        order.push(i);
        for &c in &children[i] {
            waiting[c] -= 1;
            if waiting[c] == 0 && complete[c] {
                ready.insert(c);
            }
        }
    }
    order
}
