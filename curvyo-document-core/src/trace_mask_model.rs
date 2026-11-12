//! The trace mask of one image as read from the document (ADR 0061 §1): its parts in order, each
//! with its mode, its settings and its regions in the image's unit square.

use serde::{Serialize, Serializer};

/// How far a stored coordinate may lie from the picture: a thousand picture widths (ADR 0061 §1).
pub const UNIT_LIMIT: f64 = 1000.0;

/// The commit label of a rectangle drawn on the picture; its history name is "Draw rectangle".
pub const DRAW_TRACE_RECTANGLE_LABEL: &str = "draw_trace_rectangle";
/// The commit label of a lasso drawn on the picture; "Draw lasso".
pub const DRAW_TRACE_LASSO_LABEL: &str = "draw_trace_lasso";
/// The commit label of a moved region; "Move region".
pub const MOVE_TRACE_REGION_LABEL: &str = "move_trace_region";
/// The commit label of a resized rectangle; "Resize region".
pub const RESIZE_TRACE_REGION_LABEL: &str = "resize_trace_region";
/// The commit label of a moved lasso corner; "Move lasso corner".
pub const MOVE_TRACE_LASSO_CORNER_LABEL: &str = "move_trace_lasso_corner";
/// The commit label of a deleted region; "Delete region".
pub const DELETE_TRACE_REGION_LABEL: &str = "delete_trace_region";
/// The commit label of Clear mask; "Clear mask".
pub const CLEAR_TRACE_MASK_LABEL: &str = "clear_trace_mask";
/// The commit label of Inside on a stored part; "Mask: Inside".
pub const TRACE_MASK_INSIDE_LABEL: &str = "trace_mask_inside";
/// The commit label of Outside on a stored part; "Mask: Outside".
pub const TRACE_MASK_OUTSIDE_LABEL: &str = "trace_mask_outside";
/// The commit label of Add part; "Add part".
pub const ADD_TRACE_PART_LABEL: &str = "add_trace_part";
/// The commit label of Remove part; "Remove part".
pub const REMOVE_TRACE_PART_LABEL: &str = "remove_trace_part";
/// The commit label of a part's settings edit; "Change trace settings".
pub const EDIT_TRACE_SETTINGS_LABEL: &str = "edit_trace_settings";

/// Every commit label of the trace mask commands (ADR 0061 §6).
pub const TRACE_MASK_LABELS: &[&str] = &[
    DRAW_TRACE_RECTANGLE_LABEL,
    DRAW_TRACE_LASSO_LABEL,
    MOVE_TRACE_REGION_LABEL,
    RESIZE_TRACE_REGION_LABEL,
    MOVE_TRACE_LASSO_CORNER_LABEL,
    DELETE_TRACE_REGION_LABEL,
    CLEAR_TRACE_MASK_LABEL,
    TRACE_MASK_INSIDE_LABEL,
    TRACE_MASK_OUTSIDE_LABEL,
    ADD_TRACE_PART_LABEL,
    REMOVE_TRACE_PART_LABEL,
    EDIT_TRACE_SETTINGS_LABEL,
];

/// The settings of one part: an opaque map of plain values that `curvyo-ui-core` interprets
/// (ADR 0061 §5).
pub type TracePartSettings = serde_json::Map<String, serde_json::Value>;

/// A point of an image's unit square: (0, 0) is the picture's top-left corner, (1, 1) its
/// bottom-right one; the image's placement maps it to document millimetres.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct UnitPoint {
    /// Across the picture, 0 at its left edge and 1 at its right edge.
    pub u: f64,
    /// Down the picture, 0 at its top edge and 1 at its bottom edge.
    pub v: f64,
}

impl UnitPoint {
    /// The point (`u`, `v`).
    #[must_use]
    pub const fn new(u: f64, v: f64) -> Self {
        Self { u, v }
    }

    /// Whether both coordinates are finite and within [`UNIT_LIMIT`].
    #[must_use]
    pub fn is_storable(self) -> bool {
        [self.u, self.v]
            .iter()
            .all(|c| c.is_finite() && c.abs() <= UNIT_LIMIT)
    }
}

/// The shape of a region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TraceRegionKind {
    /// Two opposite corners of an axis-aligned rectangle of the unit square.
    Rect,
    /// A closed polygon of at least three corners, closed from the last to the first.
    Lasso,
}

impl TraceRegionKind {
    /// The stored `kind` text.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Rect => "rect",
            Self::Lasso => "lasso",
        }
    }

    /// The kind of a stored `kind` text.
    #[must_use]
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "rect" => Some(Self::Rect),
            "lasso" => Some(Self::Lasso),
            _ => None,
        }
    }

    /// Whether a region of this kind may hold `count` points.
    #[must_use]
    pub const fn accepts(self, count: usize) -> bool {
        match self {
            Self::Rect => count == 2,
            Self::Lasso => count >= 3,
        }
    }

    /// The commit label of drawing a region of this kind.
    #[must_use]
    pub const fn draw_label(self) -> &'static str {
        match self {
            Self::Rect => DRAW_TRACE_RECTANGLE_LABEL,
            Self::Lasso => DRAW_TRACE_LASSO_LABEL,
        }
    }
}

/// One stored region.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TraceMaskRegion {
    /// Its id, local to its image.
    pub id: String,
    /// Its place among the image's regions (ties by id): the last is the topmost.
    pub order: i64,
    /// Rectangle or lasso.
    pub kind: TraceRegionKind,
    /// Drawn with Subtract: taken out of the part's mask.
    pub subtract: bool,
    /// Its points in the unit square, as stored (`[u0, v0, u1, v1, ...]` in `document.json`).
    #[serde(serialize_with = "flat_points")]
    pub points: Vec<UnitPoint>,
}

impl TraceMaskRegion {
    /// The corners of the region's outline in the unit square: a rectangle's four corners from
    /// its top-left clockwise (screen y down), a lasso's corners as stored.
    #[must_use]
    pub fn corners(&self) -> Vec<UnitPoint> {
        match (self.kind, self.points.as_slice()) {
            (TraceRegionKind::Rect, [a, b]) => {
                let (u0, u1) = (a.u.min(b.u), a.u.max(b.u));
                let (v0, v1) = (a.v.min(b.v), a.v.max(b.v));
                vec![
                    UnitPoint::new(u0, v0),
                    UnitPoint::new(u1, v0),
                    UnitPoint::new(u1, v1),
                    UnitPoint::new(u0, v1),
                ]
            }
            _ => self.points.clone(),
        }
    }
}

/// One stored part.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TraceMaskPart {
    /// Its id, local to its image: `0` for the part the first gesture made.
    pub id: String,
    /// Its place among the parts (ties by id); later parts stack above earlier ones.
    pub order: i64,
    /// Outside: the trace keeps the picture minus the part's mask.
    pub outside: bool,
    /// Its trace settings, as stored.
    pub settings: TracePartSettings,
    /// Its regions in order.
    pub regions: Vec<TraceMaskRegion>,
}

/// The parts of one image in order; empty for an untouched image.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct TraceMask {
    /// The parts, ordered by (`order`, id).
    pub parts: Vec<TraceMaskPart>,
}

impl TraceMask {
    /// Whether nothing is stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    /// The part `id`.
    #[must_use]
    pub fn part(&self, id: &str) -> Option<&TraceMaskPart> {
        self.parts.iter().find(|part| part.id == id)
    }

    /// The region `id` with the part that holds it.
    #[must_use]
    pub fn region(&self, id: &str) -> Option<(&TraceMaskPart, &TraceMaskRegion)> {
        self.parts.iter().find_map(|part| {
            part.regions
                .iter()
                .find(|region| region.id == id)
                .map(|region| (part, region))
        })
    }
}

/// A region to draw.
#[derive(Debug, Clone, PartialEq)]
pub struct NewTraceRegion {
    /// Rectangle or lasso.
    pub kind: TraceRegionKind,
    /// Drawn with Subtract.
    pub subtract: bool,
    /// Its points in the unit square.
    pub points: Vec<UnitPoint>,
}

/// The one part an untouched image traces with, as the tool holds it: what the first gesture
/// stores as part `0` (ADR 0061 §3).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImplicitTracePart {
    /// Outside is chosen.
    pub outside: bool,
    /// The bar's settings.
    pub settings: TracePartSettings,
}

/// The part a command writes into.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TracePartRef<'a> {
    /// The implicit part of an untouched image: stored as part `0` by the command (or part `0`
    /// itself when a peer stored it meanwhile).
    Implicit(&'a ImplicitTracePart),
    /// A stored part, by id.
    Stored(&'a str),
}

/// What a change of a region's points is to the history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceRegionEdit {
    /// The whole region moved: "Move region".
    Move,
    /// A rectangle's handle moved: "Resize region".
    Resize,
    /// One lasso corner moved: "Move lasso corner".
    MoveCorner,
}

impl TraceRegionEdit {
    /// The commit label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Move => MOVE_TRACE_REGION_LABEL,
            Self::Resize => RESIZE_TRACE_REGION_LABEL,
            Self::MoveCorner => MOVE_TRACE_LASSO_CORNER_LABEL,
        }
    }
}

/// Why a trace mask command wrote nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TraceMaskError {
    /// The node is not a live image.
    #[error("the object is not an image")]
    NotAnImage,
    /// No such stored part.
    #[error("the image has no such trace part")]
    NoSuchPart,
    /// No such stored region.
    #[error("the image has no such trace region")]
    NoSuchRegion,
    /// The points do not fit the region's kind, or a coordinate is not storable.
    #[error("the region's points are not valid")]
    InvalidPoints,
    /// The image's only part cannot be removed.
    #[error("the only part cannot be removed")]
    LastPart,
}

fn flat_points<S: Serializer>(points: &[UnitPoint], serializer: S) -> Result<S::Ok, S::Error> {
    let flat: Vec<f64> = points.iter().flat_map(|p| [p.u, p.v]).collect();
    flat.serialize(serializer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rectangle_reads_its_corners_whatever_order_its_two_points_are_in() {
        let region = TraceMaskRegion {
            id: "1@1".into(),
            order: 0,
            kind: TraceRegionKind::Rect,
            subtract: false,
            points: vec![UnitPoint::new(0.5, 1.0), UnitPoint::new(0.0, 0.0)],
        };
        assert_eq!(
            region.corners(),
            vec![
                UnitPoint::new(0.0, 0.0),
                UnitPoint::new(0.5, 0.0),
                UnitPoint::new(0.5, 1.0),
                UnitPoint::new(0.0, 1.0),
            ]
        );
    }

    #[test]
    fn storable_points_are_finite_and_within_the_limit() {
        assert!(UnitPoint::new(-1000.0, 1000.0).is_storable());
        assert!(!UnitPoint::new(1000.5, 0.0).is_storable());
        assert!(!UnitPoint::new(f64::NAN, 0.0).is_storable());
        assert!(TraceRegionKind::Lasso.accepts(3) && !TraceRegionKind::Lasso.accepts(2));
        assert!(TraceRegionKind::Rect.accepts(2) && !TraceRegionKind::Rect.accepts(3));
    }
}
