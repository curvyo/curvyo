//! Interactive construction retains presolve intent independently of its solved pose.
use crate::{Entity, EntityId, Length, Position, Sketch, SketchError};

/// A completed analytic construction expressed in local sketch coordinates.
#[derive(Debug, Clone)]
pub enum Construction {
    /// Continues a line/arc chain with an explicitly selected geometric mode.
    Continue {
        /// Previous analytic curve.
        previous: EntityId,
        /// Shared endpoint of the previous curve.
        start: EntityId,
        /// New endpoint identity.
        end: EntityId,
        /// Proposed endpoint in local coordinates.
        position: Position,
        /// Requested persistent direction relationship.
        mode: crate::ContinuationMode,
    },
    /// One independent geometric point.
    Point(Position),
    /// One incremental polyline segment, optionally creating its last point.
    Segment {
        /// Existing shared start point.
        start: EntityId,
        /// Existing or newly allocated endpoint.
        end: EntityId,
        /// New endpoint position; absent when closing onto an existing point.
        end_position: Option<Position>,
    },
    /// Initial connected polyline vertices.
    Polyline(Vec<Position>),
    /// An axis-aligned rectangle.
    Rectangle(Position, Position),
    /// A circular arc through three ordered picks.
    Arc([Position; 3]),
    /// A three-point arc continuing from an existing endpoint identity.
    ArcFrom {
        /// Existing first endpoint.
        start: EntityId,
        /// Interior point on the new arc.
        through: Position,
        /// New final endpoint.
        end: Position,
    },
    /// A centre/start/end-direction arc, projected onto the picked start radius.
    ArcCenter {
        /// Pinned centre.
        center: Position,
        /// Pinned start point defining radius.
        start: Position,
        /// End direction; radial distance is ignored.
        end: Position,
    },
    /// A centre-radius circle.
    Circle {
        /// Centre position.
        center: Position,
        /// Positive radius.
        radius: Length,
    },
    /// A circle through three picks.
    CircleThreePoints([Position; 3]),
    /// A centre-axis ellipse.
    Ellipse {
        /// Centre position.
        center: Position,
        /// First semi-axis endpoint.
        axis_end: Position,
        /// Second semi-axis length.
        second_axis: Length,
    },
    /// An ellipse from axis endpoints and a point on the curve.
    EllipseThreePoints([Position; 3]),
    /// A straight slot between cap centres.
    Slot {
        /// First cap centre.
        start: Position,
        /// Second cap centre.
        end: Position,
        /// Full slot width.
        width: Length,
    },
    /// A rounded joint retaining its original constraint references.
    Fillet {
        /// First segment.
        first: EntityId,
        /// Second segment.
        second: EntityId,
        /// Requested radius.
        radius: Length,
    },
}

/// The operation separates the explicit edit from solver-dependent movement.
#[derive(Debug, Clone)]
pub struct ConstructionResult {
    /// Presolve graph: compare with the exact supplied base to obtain explicit edits.
    pub intent: Sketch,
    /// Validated pose: dependent coordinates belong only to the derived checkpoint.
    pub candidate: Sketch,
}

/// Builds a construction against a compatible resolved pose without committing it.
///
/// The caller retains the supplied base/revision and applies only intent differences
/// to authoritative records. It must rebase or reject after intervening peer edits.
/// # Errors
/// Rejects degenerate picks, missing references, ID collisions or invalid solutions.
pub fn construct(
    base: &Sketch,
    prefix: &str,
    command: &Construction,
) -> Result<ConstructionResult, SketchError> {
    let intent = build(base, prefix, command)?;
    let candidate = crate::construction::finish(&intent)?;
    Ok(ConstructionResult { intent, candidate })
}

/// Builds a transient construction using bounded validation without rank analysis.
/// # Errors
/// Rejects malformed picks or a construction whose hard constraints cannot be met.
pub fn construct_preview(
    base: &Sketch,
    prefix: &str,
    command: &Construction,
) -> Result<ConstructionResult, SketchError> {
    let intent = build(base, prefix, command)?;
    let result = crate::solve(
        &intent,
        &crate::SolveMode::DragPoints {
            goals: std::collections::BTreeMap::new(),
        },
    )?;
    let candidate = result.candidate.ok_or_else(|| {
        SketchError::Invalid(
            result
                .message
                .unwrap_or_else(|| "construction has no valid preview".into()),
        )
    })?;
    Ok(ConstructionResult { intent, candidate })
}

fn build(base: &Sketch, prefix: &str, command: &Construction) -> Result<Sketch, SketchError> {
    use crate::primitives as p;
    match command {
        Construction::Continue {
            previous,
            start,
            end,
            position,
            mode,
        } => crate::continuation::build(base, prefix, previous, start, end, *position, *mode),
        Construction::Point(position) => {
            let mut intent = base.clone();
            crate::construction::insert(
                &mut intent,
                prefix.into(),
                Entity::Point {
                    position: *position,
                    fixed: false,
                },
            )?;
            Ok(intent)
        }
        Construction::Segment {
            start,
            end,
            end_position,
        } => segment(base, prefix, start, end, *end_position),
        Construction::Polyline(points) => p::create_polyline(base, prefix, points, false),
        Construction::Rectangle(a, b) => p::create_rectangle(base, prefix, *a, *b),
        Construction::ArcFrom {
            start,
            through,
            end,
        } => crate::arc_center::from_endpoint(base, prefix, start, *through, *end),
        Construction::ArcCenter { center, start, end } => {
            crate::arc_center::build(base, prefix, *center, *start, *end)
        }
        Construction::Arc(points) => p::create_arc_three_points(base, prefix, *points),
        Construction::Circle { center, radius } => p::create_circle(base, prefix, *center, *radius),
        Construction::CircleThreePoints(points) => {
            p::create_circle_three_points(base, prefix, *points)
        }
        Construction::Ellipse {
            center,
            axis_end,
            second_axis,
        } => p::create_ellipse(base, prefix, *center, *axis_end, *second_axis),
        Construction::EllipseThreePoints([a, b, c]) => {
            p::create_ellipse_three_points(base, prefix, *a, *b, *c)
        }
        Construction::Slot { start, end, width } => {
            crate::construction::create_slot(base, prefix, *start, *end, *width)
        }
        Construction::Fillet {
            first,
            second,
            radius,
        } => crate::fillet::fillet(base, prefix, first, second, *radius),
    }
}

fn segment(
    base: &Sketch,
    prefix: &str,
    start: &EntityId,
    end: &EntityId,
    position: Option<Position>,
) -> Result<Sketch, SketchError> {
    let mut intent = base.clone();
    if let Some(position) = position {
        crate::construction::insert(
            &mut intent,
            end.clone(),
            Entity::Point {
                position,
                fixed: false,
            },
        )?;
    }
    if intent.point(start)?.distance(intent.point(end)?).0 <= 1e-9 {
        return Err(SketchError::Invalid("segment endpoints coincide".into()));
    }
    crate::construction::insert(
        &mut intent,
        format!("{prefix}:line").into(),
        Entity::Line {
            start: start.clone(),
            end: end.clone(),
        },
    )?;
    Ok(intent)
}
