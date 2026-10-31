//! Ephemeral sketch interaction state keeps navigation out of document history.
use curvyo_document_core::{DocumentVersion, NodeId};
use curvyo_sketch_core::{ConstraintId, EntityId, Position};
use std::collections::BTreeSet;

/// The two built-in editing contexts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Workbench {
    /// Existing vector object editing.
    #[default]
    Vector,
    /// Analytic parametric sketch editing.
    Sketch,
}

/// Sketch tools name concrete construction variants.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SketchTool {
    /// Select analytic entities or their point handles.
    #[default]
    Select,
    /// Inspect relationships without moving geometry.
    Constraints,
    /// Place an independent point.
    Point,
    /// Add connected segments sharing endpoints.
    Polyline,
    /// Circular arc through three picks.
    Arc,
    /// Centre, start and end direction circular arc.
    ArcCenter,
    /// Centre and radius circle.
    Circle,
    /// Circle through three picks.
    CircleThreePoints,
    /// Centre, first semi-axis and second semi-axis ellipse.
    Ellipse,
    /// Axis endpoints plus a point on the ellipse.
    EllipseThreePoints,
    /// Opposite-corner rectangle.
    Rectangle,
    /// Two cap centres plus width.
    Slot,
    /// Round a selected segment joint.
    Fillet,
}

#[derive(Default)]
pub(in crate::session) struct SketchState {
    pub workbench: Workbench,
    pub active: Option<NodeId>,
    pub page: Option<String>,
    pub empty: Option<curvyo_sketch_core::Sketch>,
    pub tool: SketchTool,
    pub constraint_tool: Option<String>,
    pub mode: curvyo_sketch_core::ContinuationMode,
    pub chain_curve: Option<EntityId>,
    pub entities: BTreeSet<EntityId>,
    pub constraints: BTreeSet<ConstraintId>,
    pub picks: Vec<Position>,
    pub press: Option<curvyo_document_core::Point>,
    pub press_anchor: Option<Position>,
    pub hover: Option<Position>,
    pub chain: Option<(EntityId, EntityId)>,
    pub revision: Option<DocumentVersion>,
    pub error: Option<String>,
    pub pending_prefix: Option<String>,
    pub preview: Option<curvyo_sketch_core::ConstructionResult>,
    pub preview_base: Option<(DocumentVersion, curvyo_sketch_core::Sketch)>,
    pub edit_constraint: Option<ConstraintId>,
    pub affected: BTreeSet<ConstraintId>,
    pub hit_cycle: Option<(curvyo_document_core::Point, Vec<EntityId>, usize)>,
    pub preselection: Option<EntityId>,
    pub lasso: Option<Vec<curvyo_document_core::Point>>,
    pub marquee: Option<(curvyo_document_core::Point, curvyo_document_core::Point)>,
    pub draft_epoch: u64,
    pub arc_radius: Option<curvyo_sketch_core::Length>,
    pub circle_dimension_radius: bool,
    pub dimension_position: Option<curvyo_sketch_core::Position>,
    pub drag: Option<super::drag::SketchDrag>,
}
impl SketchState {
    pub fn cancel(&mut self) {
        self.draft_epoch = self.draft_epoch.wrapping_add(1);
        self.drag = None;
        self.dimension_position = None;
        self.arc_radius = None;
        self.hit_cycle = None;
        self.preselection = None;
        self.marquee = None;
        self.lasso = None;
        self.preview = None;
        self.preview_base = None;
        self.pending_prefix = None;
        self.affected.clear();
        self.edit_constraint = None;
        self.picks.clear();
        self.press = None;
        self.press_anchor = None;
        self.hover = None;
        self.chain = None;
        self.chain_curve = None;
        self.error = None;
    }
}

impl SketchTool {
    /// Stable tool/variant name used by the thin host bridge.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Select => "select",
            Self::Constraints => "constraints",
            Self::Point => "point",
            Self::Polyline => "polyline",
            Self::Arc => "arc",
            Self::ArcCenter => "arc-center",
            Self::Circle => "circle",
            Self::CircleThreePoints => "circle-three-points",
            Self::Ellipse => "ellipse",
            Self::EllipseThreePoints => "ellipse-three-points",
            Self::Rectangle => "rectangle",
            Self::Slot => "slot",
            Self::Fillet => "fillet",
        }
    }
    /// Looks up a concrete tool or named construction variant.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        [
            Self::Select,
            Self::Constraints,
            Self::Point,
            Self::Polyline,
            Self::Arc,
            Self::ArcCenter,
            Self::Circle,
            Self::CircleThreePoints,
            Self::Ellipse,
            Self::EllipseThreePoints,
            Self::Rectangle,
            Self::Slot,
            Self::Fillet,
        ]
        .into_iter()
        .find(|tool| tool.name() == name)
    }
}
