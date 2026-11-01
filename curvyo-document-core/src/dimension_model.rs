//! The linked dimension object as read from the document (ADR 0027 §2): its kind, the slots that
//! name what it measures, where its line and text sit, and the last value it measured.

use serde::Serialize;

use crate::path_model::{AnchorId, NodeId};
use crate::style_model::Style;
use crate::units::{Angle, Length, Point};

/// The label of the commit that leaves a dimension; its history name is "Dimension".
pub const CREATE_DIMENSION_COMMIT_LABEL: &str = "create_dimension";
/// The label of a text or offset drag of a dimension; its history name is "Move dimension".
pub const MOVE_DIMENSION_COMMIT_LABEL: &str = "move_dimension";
/// The label of a Node-tool move of an unlinked dimension's kept point; its history name is
/// "Edit nodes" (`specs/0070-measure-and-dimension` criterion 13).
pub const EDIT_DIMENSION_NODES_COMMIT_LABEL: &str = "edit_dimension_nodes";
/// The label of the commit that converts dimensions to paths and texts; its history name is
/// "Convert to path".
pub const CONVERT_DIMENSION_COMMIT_LABEL: &str = "convert_dimension";

/// What a dimension measures (ADR 0027 §2, the kind vocabulary of `0066`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DimensionKind {
    /// The distance between two points, along the line through them.
    Aligned,
    /// The distance along the document's x axis.
    Horizontal,
    /// The distance along the document's y axis.
    Vertical,
    /// The angle between two rays from a vertex.
    Angle,
    /// The radius of a circle, an arc or a rounded corner.
    Radius,
    /// The diameter of a circle, an arc or a rounded corner.
    Diameter,
}

impl DimensionKind {
    /// Every kind, in the order of the specification.
    pub const ALL: [Self; 6] = [
        Self::Aligned,
        Self::Horizontal,
        Self::Vertical,
        Self::Angle,
        Self::Radius,
        Self::Diameter,
    ];

    /// The stored code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Aligned => "aligned",
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
            Self::Angle => "angle",
            Self::Radius => "radius",
            Self::Diameter => "diameter",
        }
    }

    /// The kind of a stored code, `None` for an unknown one.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.code() == code)
    }

    /// The name the mode label and the Layers row show ("Aligned", "Radius").
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Aligned => "Aligned",
            Self::Horizontal => "Horizontal",
            Self::Vertical => "Vertical",
            Self::Angle => "Angle",
            Self::Radius => "Radius",
            Self::Diameter => "Diameter",
        }
    }

    /// Whether the kind measures one curve (radius, diameter) instead of points.
    #[must_use]
    pub const fn is_curve(self) -> bool {
        matches!(self, Self::Radius | Self::Diameter)
    }

    /// How many point slots the kind takes: two for a distance, three for an angle, none for a
    /// curve kind (which takes one curve slot).
    #[must_use]
    pub const fn point_slots(self) -> usize {
        match self {
            Self::Aligned | Self::Horizontal | Self::Vertical => 2,
            Self::Angle => 3,
            Self::Radius | Self::Diameter => 0,
        }
    }
}

/// A point of a primitive a slot can name (ADR 0027 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case", tag = "feature", content = "index")]
pub enum ShapeFeature {
    /// The centre of the shape's frame.
    Center,
    /// Corner `i` of a rectangle's box, clockwise from the top-left (rotated with the shape).
    Corner(u32),
    /// Anchor `i` of the shape's outline (a polygon or star vertex).
    Vertex(u32),
    /// The middle of outline segment `i` (from anchor `i` to the next).
    EdgeMid(u32),
}

/// One measured point (ADR 0027 §2, a point slot).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "slot")]
pub enum PointSlot {
    /// An anchor of a path or clone, by its node and anchor id (a clone uses its root's ids).
    Anchor {
        /// The path or clone.
        node: NodeId,
        /// The anchor.
        anchor: AnchorId,
    },
    /// The middle of the segment that starts at `anchor`.
    SegmentMid {
        /// The path or clone.
        node: NodeId,
        /// The segment's start anchor.
        anchor: AnchorId,
    },
    /// A feature of a primitive.
    ShapePoint {
        /// The primitive.
        node: NodeId,
        /// Which point of it.
        feature: ShapeFeature,
    },
    /// A point in document millimetres, linked to nothing.
    Free {
        /// The point.
        point: Point,
    },
}

impl PointSlot {
    /// The node this slot refers to, `None` for a free point.
    #[must_use]
    pub const fn node(&self) -> Option<NodeId> {
        match *self {
            Self::Anchor { node, .. }
            | Self::SegmentMid { node, .. }
            | Self::ShapePoint { node, .. } => Some(node),
            Self::Free { .. } => None,
        }
    }
}

/// One measured curve (ADR 0027 §2, a curve slot).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "slot")]
pub enum CurveSlot {
    /// An ellipse primitive that is a circle.
    Circle {
        /// The ellipse.
        node: NodeId,
    },
    /// The circular segment of a path or clone that starts at `anchor`.
    Arc {
        /// The path or clone.
        node: NodeId,
        /// The segment's start anchor.
        anchor: AnchorId,
    },
    /// The rounded corner `corner` (clockwise from the top-left) of a rectangle.
    RoundCorner {
        /// The rectangle.
        node: NodeId,
        /// Which corner.
        corner: u32,
    },
    /// A circle in document millimetres, linked to nothing: a copied radius or diameter whose
    /// curve was not copied (ADR 0027 §5).
    FreeCircle {
        /// The centre.
        center: Point,
        /// The radius, above zero.
        radius: Length,
    },
}

impl CurveSlot {
    /// The node this slot refers to, `None` for a free circle.
    #[must_use]
    pub const fn node(&self) -> Option<NodeId> {
        match *self {
            Self::Circle { node } | Self::Arc { node, .. } | Self::RoundCorner { node, .. } => {
                Some(node)
            }
            Self::FreeCircle { .. } => None,
        }
    }
}

/// What a dimension refers to: two or three points, or one curve.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DimensionSlots {
    /// Two points (a distance) or three (an angle: vertex, first, second).
    Points(Vec<PointSlot>),
    /// One curve (a radius or diameter).
    Curve(CurveSlot),
}

impl DimensionSlots {
    /// These slots with every free point, and the centre and a rim point of a free circle,
    /// mapped by `map`; the linked slots stay, they follow their geometry on read.
    #[must_use]
    pub fn mapped_free(&self, map: impl Fn(Point) -> Point) -> Self {
        let mut mapped = self.clone();
        match &mut mapped {
            Self::Points(points) => {
                for slot in points.iter_mut() {
                    if let PointSlot::Free { point } = slot {
                        *point = map(*point);
                    }
                }
            }
            Self::Curve(CurveSlot::FreeCircle { center, radius }) => {
                let rim = map(Point::new(center.x + radius.as_mm(), center.y));
                *center = map(*center);
                *radius = Length::from_mm(center.vector_to(rim).length());
            }
            Self::Curve(_) => {}
        }
        mapped
    }

    /// Whether the slots fit `kind`: the right count of point slots, or one curve slot.
    #[must_use]
    pub fn fit(&self, kind: DimensionKind) -> bool {
        match self {
            Self::Points(points) => !kind.is_curve() && points.len() == kind.point_slots(),
            Self::Curve(_) => kind.is_curve(),
        }
    }

    /// The nodes the slots refer to, each once, in slot order.
    #[must_use]
    pub fn nodes(&self) -> Vec<NodeId> {
        let mut nodes: Vec<NodeId> = Vec::new();
        let mut push = |node: NodeId| {
            if !nodes.contains(&node) {
                nodes.push(node);
            }
        };
        match self {
            Self::Points(points) => points
                .iter()
                .filter_map(PointSlot::node)
                .for_each(&mut push),
            Self::Curve(curve) => curve.node().into_iter().for_each(&mut push),
        }
        nodes
    }
}

/// Where a dimension's line and text sit relative to what it measures (ADR 0027 §2).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct DimensionPlace {
    /// The signed distance of the dimension line from the measured points, across the line (a
    /// distance); the arc radius (an angle); the text's distance beyond the curve (a radius or
    /// diameter).
    pub offset: Length,
    /// The text position along the dimension line or arc, a fraction (0.5 is the middle).
    pub along: f64,
    /// The leader direction of a radius or diameter from the centre.
    pub leader: Angle,
    /// Which of the four angles between two lines an angle dimension shows: bit 0 turns the
    /// first ray round, bit 1 the second.
    pub sector: u8,
}

impl DimensionPlace {
    /// A place `offset` from the measured points with the text in the middle.
    #[must_use]
    pub const fn at_offset(offset: Length) -> Self {
        Self {
            offset,
            along: 0.5,
            leader: Angle::from_radians(-std::f64::consts::FRAC_PI_4),
            sector: 0,
        }
    }

    /// Whether every number is finite and the sector is one of the four.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.offset.as_mm().is_finite()
            && self.along.is_finite()
            && self.leader.as_radians().is_finite()
            && self.sector < 4
    }
}

/// A measured value: a length in millimetres or an angle in radians.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase", tag = "unit", content = "value")]
pub enum DimensionValue {
    /// A distance, radius or diameter.
    Length(Length),
    /// An angle.
    Angle(Angle),
}

impl DimensionValue {
    /// The raw number: millimetres or radians.
    #[must_use]
    pub const fn raw(self) -> f64 {
        match self {
            Self::Length(length) => length.as_mm(),
            Self::Angle(angle) => angle.as_radians(),
        }
    }
}

/// The measured points and value of a dimension: resolved from the geometry, or as kept.
///
/// The points are the two measured points (a distance), the vertex and a point on each ray (an
/// angle), or the centre and a point on the curve (a radius or diameter).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Measured {
    /// The measured points in document millimetres.
    pub points: Vec<Point>,
    /// The value.
    pub value: DimensionValue,
}

impl Measured {
    /// Whether every point and the value are finite.
    #[must_use]
    pub fn is_finite(&self) -> bool {
        self.value.raw().is_finite()
            && self
                .points
                .iter()
                .all(|p| p.x.is_finite() && p.y.is_finite())
    }

    /// Whether `other` differs from this one by more than `tolerance` in a point or the value (the
    /// same number bounds an angle value in radians).
    #[must_use]
    pub fn differs_from(&self, other: &Self, tolerance: crate::units::Tolerance) -> bool {
        let tolerance_mm = tolerance.as_mm();
        self.points.len() != other.points.len()
            || std::mem::discriminant(&self.value) != std::mem::discriminant(&other.value)
            || (self.value.raw() - other.value.raw()).abs() > tolerance_mm
            || self
                .points
                .iter()
                .zip(&other.points)
                .any(|(a, b)| a.vector_to(*b).length() > tolerance_mm)
    }
}

/// One dimension object (ADR 0027): stored intent plus what it measures now.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DimensionSnapshot {
    /// The node.
    pub id: NodeId,
    /// What it measures.
    pub kind: DimensionKind,
    /// What it refers to.
    pub slots: DimensionSlots,
    /// Where its line and text sit.
    pub place: DimensionPlace,
    /// The points and value as last measured (`dimension_kept`).
    pub kept: Measured,
    /// The points and value shown: resolved from the geometry when linked, else the kept ones.
    pub shown: Measured,
    /// Whether every slot resolved against the geometry read with it.
    pub linked: bool,
    /// No paint: a dimension is drawn by its own marks. Present so readers that take any
    /// object's style see one that paints nothing.
    #[serde(skip)]
    pub style: Style,
}

impl DimensionSnapshot {
    /// A dimension as stored, shown with its kept values until it is resolved.
    #[must_use]
    pub fn new(
        id: NodeId,
        kind: DimensionKind,
        slots: DimensionSlots,
        place: DimensionPlace,
        kept: Measured,
    ) -> Self {
        Self {
            id,
            kind,
            slots,
            place,
            shown: kept.clone(),
            kept,
            linked: false,
            style: crate::image_model::no_paint(),
        }
    }
}

impl DimensionSnapshot {
    /// Whether a move or transform of the nodes that `moved` accepts leaves this dimension as it
    /// is: it is linked and some node it refers to stays put, so only its place could move (ADR
    /// 0027 §5). An unlinked one, or one moved with all its referents, moves its own points.
    #[must_use]
    pub fn stays_when_moved(&self, moved: impl Fn(NodeId) -> bool) -> bool {
        self.linked && !self.slots.nodes().into_iter().all(moved)
    }

    /// This dimension with its free points, kept points and shown points mapped by `map` (a
    /// move or transform of the dimension itself). Linked slots are not moved: they follow their
    /// geometry when read again.
    #[must_use]
    pub fn mapped_by(&self, map: crate::affine::Affine) -> Self {
        let mut mapped = self.clone();
        mapped.slots = self.slots.mapped_free(|point| map.apply_point(point));
        for point in mapped
            .kept
            .points
            .iter_mut()
            .chain(mapped.shown.points.iter_mut())
        {
            *point = map.apply_point(*point);
        }
        mapped
    }
}

/// The dimensions among `moved` that a move or transform of exactly the nodes `moved` leaves as
/// they are ([`DimensionSnapshot::stays_when_moved`]), with `start` giving each dimension as it is
/// before the edit. The one decision the live preview and every writer of a move or transform
/// share (ADR 0027 §5).
#[must_use]
pub fn staying_dimensions<S: std::hash::BuildHasher>(
    moved: &std::collections::HashSet<NodeId, S>,
    start: impl Fn(NodeId) -> Option<DimensionSnapshot>,
) -> std::collections::HashSet<NodeId> {
    moved
        .iter()
        .copied()
        .filter(|&id| {
            start(id).is_some_and(|dimension| dimension.stays_when_moved(|n| moved.contains(&n)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip_and_unknown_codes_are_refused() {
        for kind in DimensionKind::ALL {
            assert_eq!(DimensionKind::from_code(kind.code()), Some(kind));
        }
        assert_eq!(DimensionKind::from_code("ordinate"), None);
    }

    #[test]
    fn slots_fit_their_kind() {
        let free = PointSlot::Free {
            point: Point::new(0.0, 0.0),
        };
        let two = DimensionSlots::Points(vec![free, free]);
        assert!(two.fit(DimensionKind::Aligned));
        assert!(!two.fit(DimensionKind::Angle));
        assert!(!two.fit(DimensionKind::Radius));
        let curve = DimensionSlots::Curve(CurveSlot::Circle {
            node: NodeId::from_parts(1, 1),
        });
        assert!(curve.fit(DimensionKind::Diameter));
        assert!(!curve.fit(DimensionKind::Vertical));
    }

    #[test]
    fn a_change_below_the_tolerance_is_no_change() {
        let a = Measured {
            points: vec![Point::new(0.0, 0.0), Point::new(10.0, 0.0)],
            value: DimensionValue::Length(Length::from_mm(10.0)),
        };
        let mut b = a.clone();
        b.points[1].x += 1e-12;
        assert!(!a.differs_from(&b, crate::units::Tolerance::from_mm(1e-9)));
        b.points[1].x += 1e-6;
        assert!(a.differs_from(&b, crate::units::Tolerance::from_mm(1e-9)));
    }
}
