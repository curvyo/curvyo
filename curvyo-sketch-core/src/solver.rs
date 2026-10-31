//! Solving produces validated candidates without replacing the caller's last valid sketch.
use crate::{ConstraintId, EntityId, Position, Sketch, SketchError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The operation determines whether full diagnosis is needed on this solve.
#[derive(Debug, Clone, PartialEq)]
pub enum SolveMode {
    /// A committed edit requests full rank and conflict analysis.
    Edit,
    /// A curve control targets an actual radius or axis endpoint.
    DragHandle {
        /// Analytic curve identity.
        entity: EntityId,
        /// Visible geometric control.
        handle: crate::SketchHandle,
        /// Requested local handle position.
        target: Position,
    },
    /// A body preview applies simultaneous goals to deduplicated defining points.
    DragPoints {
        /// Local point targets; hard constraints remain authoritative.
        goals: std::collections::BTreeMap<EntityId, Position>,
    },
    /// A pointer preview adds a soft point goal with a bounded iteration budget.
    Drag {
        /// Point being dragged.
        point: EntityId,
        /// Requested local pointer position.
        target: Position,
    },
}
/// Numerical status never conflates zero DOF with a valid solution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolveStatus {
    /// Valid geometry with this many remaining degrees of freedom.
    UnderConstrained(usize),
    /// Valid geometry without any remaining degrees of freedom.
    FullyConstrained,
    /// Valid pointer preview; rank was deliberately not recomputed.
    Preview,
    /// The current hard constraints contradict each other.
    Conflicting,
    /// Iteration or residual validation failed without proof of contradiction.
    SolverFailure,
}
/// A solve result retains original user IDs for actionable diagnostics.
#[derive(Debug, Clone)]
pub struct SolveResult {
    /// Current numerical classification.
    pub status: SolveStatus,
    /// Present only when independently validated geometry satisfies the input.
    pub candidate: Option<Sketch>,
    /// Geometry proven fixed by the current rank analysis; absent during previews.
    pub fully_constrained: Option<BTreeSet<EntityId>>,
    /// Original IDs implicated in a contradictory equation set.
    pub conflicting: BTreeSet<ConstraintId>,
    /// Original IDs whose removal preserves a solved configuration.
    pub redundant: BTreeSet<ConstraintId>,
    /// Reason for failure when convergence alone is insufficient.
    pub message: Option<String>,
}

/// Solves a validated graph from its explicit current geometry seed.
///
/// This function never mutates the input or commits history. Failed candidates
/// stay absent so callers can keep their last valid checkpoint.
///
/// # Errors
/// Rejects malformed graphs or invalid backend response data.
pub fn solve(sketch: &Sketch, mode: &SolveMode) -> Result<SolveResult, SketchError> {
    sketch.validate()?;
    if let Some(candidate) = crate::drag_reach::candidate(sketch, mode) {
        return Ok(SolveResult {
            status: SolveStatus::Preview,
            candidate: Some(candidate),
            fully_constrained: None,
            conflicting: BTreeSet::new(),
            redundant: BTreeSet::new(),
            message: None,
        });
    }
    let first = solve_once(sketch, mode, false)?;
    if *mode == SolveMode::Edit {
        return Ok(first);
    }
    let initial = goal_cost(sketch, mode);
    if initial <= 1e-18 {
        return Ok(first);
    }
    let achieved = first
        .candidate
        .as_ref()
        .map(|candidate| goal_cost(candidate, mode));
    if achieved.is_some_and(|cost| cost < initial - 1e-12 * initial.max(1.0)) {
        return Ok(first);
    }
    // ACS can stop at a zero pseudoinverse step despite a nonzero soft-goal
    // gradient. Retry only this stalled case; ordinary drags start feasible.
    let Ok(retry) = solve_once(sketch, mode, true) else {
        return Ok(first);
    };
    if retry
        .candidate
        .as_ref()
        .is_some_and(|candidate| goal_cost(candidate, mode) < achieved.unwrap_or(initial))
    {
        return Ok(retry);
    }
    Ok(first)
}

fn goal_cost(sketch: &Sketch, mode: &SolveMode) -> f64 {
    match mode {
        SolveMode::Edit => 0.0,
        SolveMode::Drag { point, target } => sketch
            .point(point)
            .map_or(f64::INFINITY, |p| p.distance(*target).0.powi(2)),
        SolveMode::DragPoints { goals } => goals
            .iter()
            .map(|(point, target)| {
                sketch
                    .point(point)
                    .map_or(f64::INFINITY, |p| p.distance(*target).0.powi(2))
            })
            .sum(),
        SolveMode::DragHandle {
            entity,
            handle,
            target,
        } => crate::handle_positions(sketch, entity)
            .ok()
            .and_then(|p| p.get(handle).copied())
            .map_or(f64::INFINITY, |p| p.distance(*target).0.powi(2)),
    }
}

fn solve_once(
    sketch: &Sketch,
    mode: &SolveMode,
    seed_goals: bool,
) -> Result<SolveResult, SketchError> {
    // invariant: the sole caller `solve` validated this immutable input.
    let adapter = crate::solver_input::Input::new(sketch, mode, seed_goals)?;
    let raw = acs::sketch_solve::solve_sketch_json(&adapter.request.to_string())
        .map_err(SketchError::Solver)?;
    let response: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| SketchError::Solver(e.to_string()))?;
    let conflicting = adapter.map_diagnostics(&response, "conflicting");
    let redundant = adapter.map_diagnostics(&response, "redundant");
    if response["status"] != "converged" {
        return Ok(SolveResult {
            status: if conflicting.is_empty() {
                SolveStatus::SolverFailure
            } else {
                SolveStatus::Conflicting
            },
            candidate: None,
            fully_constrained: None,
            conflicting,
            redundant,
            message: Some("hard constraints did not converge".into()),
        });
    }
    let mut candidate = adapter.read_candidate(sketch, &response)?;
    candidate.contact_seeds = adapter.read_contacts(&response)?;
    if let Err(error) = crate::residuals::validate(sketch, &candidate, &candidate.contact_seeds) {
        return Ok(SolveResult {
            status: SolveStatus::SolverFailure,
            candidate: None,
            fully_constrained: None,
            conflicting,
            redundant,
            message: Some(error.to_string()),
        });
    }
    let status = if *mode == SolveMode::Edit {
        let dof = response["dof"]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| SketchError::Solver("missing DOF analysis".into()))?;
        if dof == 0 {
            SolveStatus::FullyConstrained
        } else {
            SolveStatus::UnderConstrained(dof)
        }
    } else {
        SolveStatus::Preview
    };
    Ok(SolveResult {
        status,
        candidate: Some(candidate),
        fully_constrained: if *mode == SolveMode::Edit {
            Some(adapter.map_fully_constrained(sketch, &response))
        } else {
            None
        },
        conflicting,
        redundant,
        message: None,
    })
}
