//! Sketch records retain intent separately from derived solution checkpoints.
use crate::{Affine, NodeId, PathSnapshot, Style};
use curvyo_sketch_core::{
    Constraint, ConstraintId, Entity, EntityId, Parameter, ParameterId, Sketch, SolveStatus,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// An indivisible entity record, including its output participation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SketchEntityRecord {
    /// Analytic definition and explicit seed.
    pub entity: Entity,
    /// Whether the entity is excluded from production outlines.
    pub construction: bool,
}

/// An explicit edit changes only the listed authoritative records.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SketchDelta {
    /// Insert/replace an entity or delete it with `None`.
    pub entities: BTreeMap<EntityId, Option<SketchEntityRecord>>,
    /// Insert/replace a constraint or delete it with `None`.
    pub constraints: BTreeMap<ConstraintId, Option<Constraint>>,
    /// Insert/replace a parameter or delete it with `None`.
    pub parameters: BTreeMap<ParameterId, Option<Parameter>>,
}

/// A derived solution is never a second source of editable geometry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SketchCheckpoint {
    /// SHA-256 of the canonical authoritative graph at solve time.
    pub fingerprint: String,
    /// Original inputs permit independent validation even after a conflicting merge.
    pub intent: Sketch,
    /// Solved geometry and tangent branch seeds.
    pub candidate: Sketch,
    /// Diagnostic result when the checkpoint was produced.
    pub status: SolveStatus,
    /// Entities fixed by rank analysis.
    pub fully_constrained: Option<BTreeSet<EntityId>>,
    /// Conflicting user constraints.
    pub conflicting: BTreeSet<ConstraintId>,
    /// Redundant user constraints.
    pub redundant: BTreeSet<ConstraintId>,
}

/// A parametric object exposes a derived path only for read-only consumers.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SketchSnapshot {
    /// Persistent object identity.
    pub id: NodeId,
    /// Current authoritative graph, including repairable dangling references.
    pub graph: Sketch,
    /// Local-to-document placement.
    pub placement: Affine,
    /// Object style.
    pub style: Style,
    /// Validated output, possibly the last valid solution after a failed edit/merge.
    pub resolved: PathSnapshot,
    /// Derived local analytic solution behind the visible outline; when stale, its
    /// topology is historical and must not be treated as current editable intent.
    pub solution: Sketch,
    /// Last persisted validated checkpoint.
    pub checkpoint: Option<SketchCheckpoint>,
    /// Current solve classification; absent when references are invalid.
    pub status: Option<SolveStatus>,
    /// Entities fixed by current rank analysis.
    pub fully_constrained: Option<BTreeSet<EntityId>>,
    /// Conflicting user constraints.
    pub conflicting: BTreeSet<ConstraintId>,
    /// Redundant user constraints.
    pub redundant: BTreeSet<ConstraintId>,
    /// Whether the output belongs to an earlier graph.
    pub stale: bool,
    /// Repairable graph or numerical failure.
    pub diagnosis: Option<String>,
}

impl SketchSnapshot {
    /// Maps the read view without altering local parametric intent.
    #[must_use]
    pub fn mapped_by(&self, map: Affine) -> Self {
        let mut snapshot = self.clone();
        snapshot.placement = map.compose(snapshot.placement);
        snapshot.resolved = snapshot.resolved.mapped(map);
        snapshot.resolved.rotation =
            crate::Angle::from_radians(snapshot.placement.b.atan2(snapshot.placement.a));
        snapshot
    }
}
