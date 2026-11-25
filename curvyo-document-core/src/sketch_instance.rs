//! Solves a sketch with one value set and memoises the result (`specs/0099-sketch-pages/adrs.md`
//! A4): the sketch's own solved candidate is the seed, so a preview, the committed read, a reopen
//! and every peer give the same geometry; the cache is keyed by content, never by document version.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use curvyo_sketch_core::{Sketch, SolveMode, SolveStatus};

use crate::affine::Affine;
use crate::instance_codec::ExposedEntry;
use crate::instance_value::{ExposedKey, ExposedParam, ExposedTarget, InstanceKind, ValueSet};
use crate::path_model::{NodeId, PathSnapshot};
use crate::sketch_model::SketchSnapshot;
use crate::style_model::Style;

/// The exposed values of a sketch that its graph still has, with label, kind, limits and the
/// sketch's own value: an entry whose parameter or dimension is gone, is a reference dimension or
/// has no length or angle unit is left out; a limit of the wrong unit is unlimited.
pub(crate) fn exposed_of(graph: &Sketch, entries: &[ExposedEntry]) -> Vec<ExposedParam> {
    let Ok((parameters, dimensions)) = graph.evaluated_values() else {
        return Vec::new();
    };
    let mut exposed: Vec<ExposedParam> = entries
        .iter()
        .filter_map(|entry| {
            let (label, dimension, default) = match entry.key.target() {
                ExposedTarget::Parameter(id) => {
                    let parameter = graph.parameters.0.get(&id)?;
                    (
                        parameter.name.clone(),
                        parameter.dimension,
                        parameters.get(&id)?.value(),
                    )
                }
                ExposedTarget::Dimension(id) => (
                    entry.name.clone().unwrap_or_else(|| id.0.clone()),
                    graph.exposable_dimension(&id)?,
                    dimensions.get(&id)?.value(),
                ),
            };
            let kind = match dimension {
                curvyo_sketch_core::Dimension::Length => InstanceKind::Length,
                curvyo_sketch_core::Dimension::Angle => InstanceKind::Angle,
                curvyo_sketch_core::Dimension::Scalar => return None,
            };
            let default = match kind {
                InstanceKind::Length => default,
                InstanceKind::Angle => default.to_degrees(),
            };
            let limit = |value: Option<crate::instance_value::InstanceValue>| {
                value
                    .filter(|value| value.kind() == kind)
                    .map(crate::instance_value::InstanceValue::number)
            };
            let (mut min, mut max) = (limit(entry.min), limit(entry.max));
            if matches!((min, max), (Some(low), Some(high)) if low > high) {
                min = None;
                max = None;
            }
            Some(ExposedParam {
                key: entry.key.clone(),
                label,
                kind,
                min,
                max,
                default,
                link: entry.link.clone(),
                group: None,
            })
        })
        .collect();
    resolve_groups(&mut exposed);
    exposed
}

/// Gives each value its link group (`adrs.md` A13): entries with the same link string form one;
/// its kind is its first member's, a member of another kind is in none, a group of one is none;
/// groups are numbered in the order of their first member.
fn resolve_groups(exposed: &mut [ExposedParam]) {
    let mut kinds: Vec<(&str, InstanceKind, usize)> = Vec::new();
    for param in exposed.iter() {
        let Some(link) = param.link.as_deref() else {
            continue;
        };
        match kinds.iter_mut().find(|(name, _, _)| *name == link) {
            Some((_, kind, count)) if *kind == param.kind => *count += 1,
            Some(_) => {}
            None => kinds.push((link, param.kind, 1)),
        }
    }
    let groups: Vec<(String, InstanceKind)> = kinds
        .into_iter()
        .filter(|(_, _, count)| *count >= 2)
        .map(|(name, kind, _)| (name.to_owned(), kind))
        .collect();
    for param in exposed.iter_mut() {
        param.group = param.link.as_deref().and_then(|link| {
            groups
                .iter()
                .position(|(name, kind)| name == link && *kind == param.kind)
        });
    }
}

/// A value set a sketch cannot be solved with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NoSolution;

#[derive(Debug)]
struct SketchEntry {
    graph: Sketch,
    checkpoint: Option<crate::SketchCheckpoint>,
    placement: Affine,
    style: Style,
    sets: HashMap<ValueSet, Result<Arc<Sketch>, NoSolution>>,
    outlines: HashMap<(ValueSet, u64), Option<Arc<PathSnapshot>>>,
}

/// The solves of sketch instances, kept between reads.
#[derive(Debug, Default)]
pub struct InstanceSolves {
    entries: RefCell<HashMap<NodeId, SketchEntry>>,
    /// The keys the read in progress has used.
    used: RefCell<HashSet<(NodeId, ValueSet)>>,
    /// Whether a full read is in progress: only its solves count as used.
    reading: Cell<bool>,
    /// The keys the last full read used: what stays cached beside the one preview.
    last_read: RefCell<HashSet<(NodeId, ValueSet)>>,
    preview: RefCell<Option<(NodeId, ValueSet)>>,
    solves: Cell<usize>,
}

impl InstanceSolves {
    /// How many sketch solves this cache has run: a diagnostic for tests and the PR.
    #[must_use]
    pub fn solve_count(&self) -> usize {
        self.solves.get()
    }

    fn entry<'a>(
        entries: &'a mut HashMap<NodeId, SketchEntry>,
        sketch: &SketchSnapshot,
    ) -> &'a mut SketchEntry {
        let stale = entries.get(&sketch.id).is_none_or(|entry| {
            entry.graph != sketch.graph || entry.checkpoint != sketch.checkpoint
        });
        if stale {
            entries.insert(
                sketch.id,
                SketchEntry {
                    graph: sketch.graph.clone(),
                    checkpoint: sketch.checkpoint.clone(),
                    placement: sketch.placement,
                    style: sketch.resolved.style.clone(),
                    sets: HashMap::new(),
                    outlines: HashMap::new(),
                },
            );
        }
        // invariant: inserted above when absent or stale.
        #[allow(clippy::unwrap_used)]
        let entry = entries.get_mut(&sketch.id).unwrap();
        if entry.placement != sketch.placement || entry.style != sketch.resolved.style {
            entry.placement = sketch.placement;
            entry.style = sketch.resolved.style.clone();
            entry.outlines.clear();
        }
        entry
    }

    /// The sketch solved with `set`, `Err` when the set has no solution. A sketch's own geometry is
    /// never asked for through here (an empty set has no solve).
    pub(crate) fn solved(
        &self,
        sketch: &SketchSnapshot,
        set: &ValueSet,
    ) -> Result<Arc<Sketch>, NoSolution> {
        if self.reading.get() {
            self.used.borrow_mut().insert((sketch.id, set.clone()));
        }
        self.solved_untracked(sketch, set)
    }

    fn solved_untracked(
        &self,
        sketch: &SketchSnapshot,
        set: &ValueSet,
    ) -> Result<Arc<Sketch>, NoSolution> {
        let mut entries = self.entries.borrow_mut();
        let entry = Self::entry(&mut entries, sketch);
        if let Some(found) = entry.sets.get(set) {
            return found.clone();
        }
        self.solves.set(self.solves.get() + 1);
        let result = solve_with(sketch, set);
        entry.sets.insert(set.clone(), result.clone());
        result
    }

    /// The outline of the sketch solved with `set`, refined for `stretch`; `None` when the set has
    /// no solution or no drawable outline.
    pub(crate) fn outline(
        &self,
        sketch: &SketchSnapshot,
        set: &ValueSet,
        stretch: f64,
    ) -> Option<Arc<PathSnapshot>> {
        let solved = self.solved(sketch, set).ok()?;
        self.outline_of(sketch, set, &solved, stretch)
    }

    /// The outline a field or knob drag previews: the one preview key is kept, no read used it.
    pub(crate) fn preview_outline(
        &self,
        sketch: &SketchSnapshot,
        set: &ValueSet,
        stretch: f64,
    ) -> Option<Arc<PathSnapshot>> {
        self.set_preview(sketch.id, set);
        let solved = self.solved_untracked(sketch, set).ok()?;
        self.outline_of(sketch, set, &solved, stretch)
    }

    fn outline_of(
        &self,
        sketch: &SketchSnapshot,
        set: &ValueSet,
        solved: &Sketch,
        stretch: f64,
    ) -> Option<Arc<PathSnapshot>> {
        let mut entries = self.entries.borrow_mut();
        let entry = Self::entry(&mut entries, sketch);
        entry
            .outlines
            .entry((set.clone(), stretch.to_bits()))
            .or_insert_with(|| {
                crate::sketch_outline::outline_scaled(
                    sketch.id,
                    solved,
                    sketch.placement,
                    &sketch.resolved.style,
                    stretch,
                )
                .ok()
                .map(Arc::new)
            })
            .clone()
    }

    /// Keeps the one value set a preview asks for; the previous preview's solve is dropped unless
    /// the last full read used it.
    fn set_preview(&self, sketch: NodeId, set: &ValueSet) {
        let previous = self.preview.borrow_mut().replace((sketch, set.clone()));
        if let Some((node, old)) = previous
            && (node, old.clone()) != (sketch, set.clone())
            && !self.last_read.borrow().contains(&(node, old.clone()))
            && let Some(entry) = self.entries.borrow_mut().get_mut(&node)
        {
            entry.sets.remove(&old);
            entry.outlines.retain(|(key, _), _| *key != old);
        }
    }

    /// Starts a full read: its solves are the keys that stay cached when it ends.
    pub(crate) fn begin_read(&self) {
        self.used.borrow_mut().clear();
        self.reading.set(true);
    }

    /// Ends a full read: only the keys it used and the preview's stay cached.
    pub(crate) fn sweep(&self) {
        self.reading.set(false);
        let used = std::mem::take(&mut *self.used.borrow_mut());
        let preview = self.preview.borrow().clone();
        let mut entries = self.entries.borrow_mut();
        entries.retain(|node, _| {
            used.iter().any(|(sketch, _)| sketch == node)
                || preview.as_ref().is_some_and(|(sketch, _)| sketch == node)
        });
        for (node, entry) in entries.iter_mut() {
            let keep = |set: &ValueSet| {
                used.contains(&(*node, set.clone()))
                    || preview
                        .as_ref()
                        .is_some_and(|(sketch, kept)| sketch == node && kept == set)
            };
            entry.sets.retain(|set, _| keep(set));
            entry.outlines.retain(|(set, _), _| keep(set));
        }
        *self.last_read.borrow_mut() = used;
    }

    /// How many value sets are cached over all sketches: a diagnostic for the eviction test.
    #[must_use]
    pub fn cached_sets(&self) -> usize {
        self.entries
            .borrow()
            .values()
            .map(|entry| entry.sets.len())
            .sum()
    }
}

fn solve_with(sketch: &SketchSnapshot, set: &ValueSet) -> Result<Arc<Sketch>, NoSolution> {
    let values = set.to_sketch_values().map_err(|_| NoSolution)?;
    let graph = sketch.graph.with_values(&values).map_err(|_| NoSolution)?;
    let result = curvyo_sketch_core::solve_seeded(&graph, &sketch.solution, &SolveMode::Edit)
        .map_err(|_| NoSolution)?;
    match (result.status, result.candidate) {
        (SolveStatus::Conflicting | SolveStatus::SolverFailure, _) | (_, None) => Err(NoSolution),
        (_, Some(candidate)) => Ok(Arc::new(candidate)),
    }
}

/// The values of `own` that `exposed` still lets a clone set, each clamped into its limits.
pub(crate) fn filtered(exposed: &[ExposedParam], own: &ValueSet) -> ValueSet {
    let mut kept = ValueSet::default();
    for (key, value) in own.iter() {
        if let Some(param) = exposed.iter().find(|param| &param.key == key)
            && param.kind == value.kind()
        {
            let (number, _) = param.clamp(value.number());
            kept.insert(ExposedKey::clone(key), value.with_number(number));
        }
    }
    kept
}
