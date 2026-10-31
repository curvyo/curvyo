//! Canonical sketch records describe editable geometry independently of a solver.
use crate::{Angle, ConstraintId, EntityId, Expression, Length, Parameters, Position, SketchError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// An immutable local coordinate-frame axis, independent of the origin point entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Axis {
    /// Horizontal axis: the point's y coordinate is zero.
    X,
    /// Vertical axis: the point's x coordinate is zero.
    Y,
}

/// A geometric primitive; references always name stable point entities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Entity {
    /// A local point, optionally fixed against numerical movement.
    Point {
        /// Local position.
        position: Position,
        /// Whether both coordinates are fixed.
        fixed: bool,
    },
    /// A segment sharing its endpoint records with adjacent geometry.
    Line {
        /// First point.
        start: EntityId,
        /// Second point.
        end: EntityId,
    },
    /// A circle whose centre is an independently constrainable point.
    Circle {
        /// Centre point.
        center: EntityId,
        /// Positive radius.
        radius: Length,
        /// Whether the radius is fixed.
        fixed_radius: bool,
    },
    /// A counter-clockwise circular arc with shared endpoint points.
    Arc {
        /// Centre point.
        center: EntityId,
        /// Start point.
        start: EntityId,
        /// End point.
        end: EntityId,
        /// Positive radius.
        radius: Length,
        /// Start angle in radians.
        start_angle: Angle,
        /// End angle in radians.
        end_angle: Angle,
        /// Whether radius and angles are fixed.
        fixed_shape: bool,
    },
    /// An ellipse whose major radius derives from its centre, focus and minor radius.
    Ellipse {
        /// Centre point.
        center: EntityId,
        /// First focus point.
        focus: EntityId,
        /// Positive minor radius.
        minor_radius: Length,
        /// Whether the minor radius is fixed.
        fixed_radius: bool,
    },
}

/// A relation stores intent rather than an implementation-specific equation index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Relation {
    /// Perpendicular endpoint tangents of connected line/arc entities.
    PerpendicularAt {
        /// First line or arc.
        a: EntityId,
        /// Second line or arc, with at least one arc in the pair.
        b: EntityId,
        /// The actual endpoint identity shared by both entities.
        point: EntityId,
    },
    /// A point lies on an immutable local coordinate axis.
    OnAxis {
        /// Constrained point identity.
        point: EntityId,
        /// Local frame axis.
        axis: Axis,
    },
    /// A point lies on a finite segment or analytic curve.
    On {
        /// Point constrained to the curve.
        point: EntityId,
        /// Finite target curve.
        curve: EntityId,
    },
    /// A circular arc has a directed counter-clockwise sweep.
    Sweep {
        /// Arc to dimension.
        arc: EntityId,
        /// Required positive angle below a full turn.
        value: Expression,
    },
    /// Two points share coordinates.
    Coincident(EntityId, EntityId),
    /// A line has equal endpoint Y coordinates.
    Horizontal(EntityId),
    /// A line has equal endpoint X coordinates.
    Vertical(EntityId),
    /// Two segments have parallel directions.
    Parallel(EntityId, EntityId),
    /// Two segments have perpendicular directions.
    Perpendicular(EntityId, EntityId),
    /// Analytic carriers share a tangent; shared endpoint joints retain finite contact.
    Tangent(EntityId, EntityId),
    /// Two segments have equal length or two round curves have equal radius.
    Equal(EntityId, EntityId),
    /// An entity retains the supplied seed geometry.
    Fixed(EntityId),
    /// Two points mirror across an axis segment's infinite supporting line.
    Symmetric {
        /// First point.
        source: EntityId,
        /// Reflected point.
        image: EntityId,
        /// Axis line.
        axis: EntityId,
    },
    /// A point is the midpoint of a segment.
    Midpoint {
        /// Point to constrain.
        point: EntityId,
        /// Segment.
        line: EntityId,
    },
    /// Point-to-point or point-to-segment distance.
    Distance {
        /// First point.
        a: EntityId,
        /// Second point or segment.
        b: EntityId,
        /// Required distance.
        value: Expression,
    },
    /// Segment or circular arc length.
    Length {
        /// Curve to dimension.
        curve: EntityId,
        /// Required length.
        value: Expression,
    },
    /// Circle or circular arc radius.
    Radius {
        /// Curve to dimension.
        curve: EntityId,
        /// Required radius.
        value: Expression,
    },
    /// Circle or circular arc diameter.
    Diameter {
        /// Curve to dimension.
        curve: EntityId,
        /// Required diameter.
        value: Expression,
    },
    /// Directed angle from the first segment to the second.
    Angle {
        /// First segment.
        a: EntityId,
        /// Second segment.
        b: EntityId,
        /// Required angle in radians.
        value: Expression,
    },
}

/// One user-visible relation, including reference-only dimension annotations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Constraint {
    /// Canonical relation with stable references.
    pub relation: Relation,
    /// Reference dimensions report values without moving geometry.
    pub reference: bool,
}
impl Constraint {
    /// Creates an authoritative geometric relation or driving dimension.
    #[must_use]
    pub const fn driving(relation: Relation) -> Self {
        Self {
            relation,
            reference: false,
        }
    }
}

/// A self-contained local sketch graph, independent of document placement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sketch {
    /// Origin point; fixed by default, explicitly releasable.
    pub origin: EntityId,
    /// Stable geometric records in deterministic order.
    pub entities: BTreeMap<EntityId, Entity>,
    /// Construction geometry remains constrainable but is excluded from output.
    pub construction: BTreeSet<EntityId>,
    /// Derived contact coordinates used only to preserve a solved tangent branch.
    /// Document storage places these in the fingerprinted solution checkpoint.
    pub contact_seeds: BTreeMap<ConstraintId, Position>,
    /// Stable user constraint records.
    pub constraints: BTreeMap<ConstraintId, Constraint>,
    /// Named typed parameter records.
    pub parameters: Parameters,
}
impl Sketch {
    /// Creates an empty sketch with one fixed local origin point.
    #[must_use]
    pub fn new(origin: EntityId) -> Self {
        let entities = BTreeMap::from([(
            origin.clone(),
            Entity::Point {
                position: Position::mm(0.0, 0.0),
                fixed: true,
            },
        )]);
        Self {
            origin,
            entities,
            constraints: BTreeMap::new(),
            construction: BTreeSet::new(),
            contact_seeds: BTreeMap::new(),
            parameters: Parameters::default(),
        }
    }
    /// Resolves a geometric point with a typed error on a missing/wrong reference.
    ///
    /// # Errors
    /// Rejects missing references or a non-point entity.
    pub fn point(&self, id: &EntityId) -> Result<Position, SketchError> {
        match self.entities.get(id) {
            Some(Entity::Point { position, .. }) => Ok(*position),
            _ => Err(SketchError::Invalid(format!("{} is not a point", id.0))),
        }
    }
    /// Resolves the endpoints of a segment.
    ///
    /// # Errors
    /// Rejects missing segments or invalid endpoint references.
    pub fn line(&self, id: &EntityId) -> Result<(Position, Position), SketchError> {
        match self.entities.get(id) {
            Some(Entity::Line { start, end }) => Ok((self.point(start)?, self.point(end)?)),
            _ => Err(SketchError::Invalid(format!("{} is not a line", id.0))),
        }
    }
    /// Validates graph references, units and finite geometry before solving.
    ///
    /// # Errors
    /// Rejects non-finite geometry, malformed references or invalid dimensions.
    pub fn validate(&self) -> Result<(), SketchError> {
        crate::validation::validate(self)
    }
}
