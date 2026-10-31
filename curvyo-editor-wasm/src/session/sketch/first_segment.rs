//! Polyline steps take their mode direction from the previous curve or the sketch X axis.
use super::{Session, SketchTool};
use curvyo_sketch_core::{
    Constraint, ConstraintId, Construction, ContinuationMode, EntityId, Position, Relation,
};

/// Proposal: a free start has no previous curve, so the local X axis is its start direction
/// (the axis AC28 measures angles from).
pub(super) const START_DIRECTION: (f64, f64) = (1.0, 0.0);

/// Twice the cross product (mm²) below which an arc end lies on its start direction; the
/// value `curvyo_sketch_core` continuation uses to refuse such an arc.
const DEGENERATE_ARC: f64 = 1e-9;

/// The construction of a first segment and the identities the chain continues from.
pub(super) struct FirstSegment {
    pub command: Construction,
    pub first: EntityId,
    pub last: EntityId,
    pub curve: EntityId,
    pub relation: Option<(ConstraintId, Constraint)>,
}

/// Whether the mode draws an arc.
pub(super) const fn is_arc(mode: ContinuationMode) -> bool {
    matches!(
        mode,
        ContinuationMode::TangentArc
            | ContinuationMode::PerpendicularArcLeft
            | ContinuationMode::PerpendicularArcRight
    )
}

/// The straight mode drawn in place of an arc whose end lies on its start direction.
pub(super) const fn straight(mode: ContinuationMode) -> ContinuationMode {
    match mode {
        ContinuationMode::TangentArc => ContinuationMode::TangentLine,
        ContinuationMode::PerpendicularArcLeft | ContinuationMode::PerpendicularArcRight => {
            ContinuationMode::PerpendicularLine
        }
        other => other,
    }
}

/// The start direction a mode gives a segment leaving along the unit `tangent`.
pub(super) fn mode_direction(mode: ContinuationMode, tangent: (f64, f64)) -> (f64, f64) {
    match mode {
        ContinuationMode::PerpendicularLine | ContinuationMode::PerpendicularArcLeft => {
            (-tangent.1, tangent.0)
        }
        ContinuationMode::PerpendicularArcRight => (tangent.1, -tangent.0),
        _ => tangent,
    }
}

/// Whether an arc from `start` leaving along unit `direction` cannot reach `end`.
pub(super) fn arc_degenerate(start: Position, direction: (f64, f64), end: Position) -> bool {
    let dx = end.x.0 - start.x.0;
    let dy = end.y.0 - start.y.0;
    (2.0 * (-dx * direction.1 + dy * direction.0)).abs() <= DEGENERATE_ARC
}

/// The first segment of a chain from a free `start` towards `position` in `mode`.
/// Lines along the start direction persist Horizontal (tangent) or Vertical (perpendicular);
/// arcs start along it without a persisted relation (there is no axis entity to relate to).
/// Returns `None` for a degenerate arc or coincident points.
pub(super) fn first_segment(
    prefix: &str,
    start: Position,
    position: Position,
    mode: ContinuationMode,
) -> Option<FirstSegment> {
    let id = |suffix: &str| EntityId::from(format!("{prefix}:{suffix}"));
    let direction = mode_direction(mode, START_DIRECTION);
    if !is_arc(mode) {
        return Some(first_line(prefix, start, position, mode, direction));
    }
    if arc_degenerate(start, direction, position) {
        return None;
    }
    let dx = position.x.0 - start.x.0;
    let dy = position.y.0 - start.y.0;
    let signed = (dx * dx + dy * dy) / (2.0 * (-dx * direction.1 + dy * direction.0));
    let center = Position::mm(
        start.x.0 - direction.1 * signed,
        start.y.0 + direction.0 * signed,
    );
    // The analytic arc runs counter-clockwise; a clockwise turn starts at the far end.
    let (command, first, last) = if signed > 0.0 {
        (
            Construction::ArcCenter {
                center,
                start,
                end: position,
            },
            id("start"),
            id("end"),
        )
    } else {
        (
            Construction::ArcCenter {
                center,
                start: position,
                end: start,
            },
            id("end"),
            id("start"),
        )
    };
    Some(FirstSegment {
        command,
        first,
        last,
        curve: id("curve"),
        relation: None,
    })
}

/// A first line segment: free, or projected onto the mode's start `direction`.
fn first_line(
    prefix: &str,
    start: Position,
    position: Position,
    mode: ContinuationMode,
    direction: (f64, f64),
) -> FirstSegment {
    let id = |suffix: &str| EntityId::from(format!("{prefix}:{suffix}"));
    let end = if mode == ContinuationMode::FreeLine {
        position
    } else {
        let distance =
            (position.x.0 - start.x.0) * direction.0 + (position.y.0 - start.y.0) * direction.1;
        Position::mm(
            start.x.0 + distance * direction.0,
            start.y.0 + distance * direction.1,
        )
    };
    let line = id("line0");
    let relation = match mode {
        ContinuationMode::TangentLine => Some(Relation::Horizontal(line.clone())),
        ContinuationMode::PerpendicularLine => Some(Relation::Vertical(line.clone())),
        _ => None,
    };
    FirstSegment {
        command: Construction::Polyline(vec![start, end]),
        first: id("p0"),
        last: id("p1"),
        curve: line,
        relation: relation.map(|r| (id("direction").0.into(), Constraint::driving(r))),
    }
}

/// One polyline step towards the pointer, as previewed and as committed.
pub(super) struct PolylineStep {
    pub command: Construction,
    /// First, last and curve identity of a mode-directed first segment.
    pub ends: Option<(EntityId, EntityId, EntityId)>,
    pub relation: Option<(ConstraintId, Constraint)>,
    /// An arc mode whose end lies on its start direction, shown straight, never committed.
    pub degenerate: bool,
}

impl PolylineStep {
    /// A command drawn as given, without a mode direction.
    pub(super) const fn plain(command: Construction) -> Self {
        Self {
            command,
            ends: None,
            relation: None,
            degenerate: false,
        }
    }
}

impl Session {
    /// Applies the drawing mode to a polyline `command` that ends at `position`:
    /// a first segment from the free start `picks[0]`, and a degenerate arc as a straight step.
    pub(super) fn sketch_polyline_step(
        &self,
        prefix: &str,
        picks: &[Position],
        command: Construction,
        position: Position,
    ) -> PolylineStep {
        let mut step = PolylineStep::plain(command);
        let mode = self.sketch.mode;
        if self.sketch.tool != SketchTool::Polyline || mode == ContinuationMode::FreeLine {
            return step;
        }
        if self.sketch.chain.is_none() {
            let [start, ..] = picks else {
                return step;
            };
            if let Some(first) = first_segment(prefix, *start, position, mode) {
                step.command = first.command;
                step.ends = Some((first.first, first.last, first.curve));
                step.relation = first.relation;
            } else if let Some(line) = first_segment(prefix, *start, position, straight(mode)) {
                step.command = line.command;
                step.relation = line.relation;
                step.degenerate = true;
            }
            return step;
        }
        if let Construction::Continue {
            previous,
            start,
            end,
            mode,
            ..
        } = &mut step.command
            && is_arc(*mode)
            && let Ok(base) = self.sketch_creation_base()
            && let Ok(tangent) = curvyo_sketch_core::endpoint_tangent(&base, previous, start)
            && let Ok(from) = base.point(start)
        {
            let to = base.point(end).unwrap_or(position);
            if arc_degenerate(from, mode_direction(*mode, tangent), to) {
                *mode = straight(*mode);
                step.degenerate = true;
            }
        }
        step
    }

    /// Marks a straight preview that stands in for a degenerate arc, at the pointer.
    pub(super) fn sketch_degenerate_annotation(
        &self,
    ) -> Option<super::inspection::SketchAnnotation> {
        if !self.sketch.degenerate {
            return None;
        }
        let hover = self.sketch.hover?;
        let placement = self.sketch_placement()?;
        let screen = self.view().document_to_screen(
            placement.apply_point(curvyo_document_core::Point::new(hover.x.0, hover.y.0)),
        );
        Some(super::inspection::SketchAnnotation {
            id: "pending-degenerate-arc".into(),
            x: screen.0,
            y: screen.1 - 18.0,
            text: "No arc: on the start direction".into(),
            dimension: false,
            selected: false,
            conflicting: true,
            preview: true,
            ..Default::default()
        })
    }
}
