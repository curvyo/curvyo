//! Pure geometric snapping values.
use std::collections::HashSet;

use curvyo_document_core::{
    AnchorId, DocumentSize, DocumentSnapping, NodeId, NodeRead, ObjectSnapshot, Point, Vec2,
};

use crate::object_bounds::object_outline_bounds as object_bounds;

/// The six page fractions on each axis (criterion 5, `adrs.md` decision 8).
///
/// The customer's "33.3 %" and "66.6 %" are exact thirds, not `0.333` and `0.666`: the line must
/// coincide with the thirds `0061-align-and-distribute` uses, and a third of a 297 mm page is
/// 99 mm while 33.3 % is 98.901 mm.
pub const PAGE_FRACTIONS: [f64; 6] = [0.25, 1.0 / 3.0, 0.5, 2.0 / 3.0, 0.75, 1.0];

/// A target kind, in the tie order of criterion 6: an exact tie goes to the earlier variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TargetKind {
    /// An anchor point of a visible path.
    Node,
    /// A guide line or one of its anchor points.
    Guide,
    /// An edge, centre line or reference point of another object's box.
    Box,
    /// One of the page's six fractions.
    Page,
}

impl TargetKind {
    /// The settings flag that enables this kind.
    #[must_use]
    pub const fn is_enabled(self, settings: &SnapSettings) -> bool {
        match self {
            Self::Node => settings.nodes,
            Self::Guide => settings.guides,
            Self::Box => settings.objects,
            Self::Page => settings.page,
        }
    }
}

/// Which axis scans a target may win.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetAxis {
    /// A vertical line: only the x axis (`point.x` is the line).
    Vertical,
    /// A horizontal line: only the y axis (`point.y` is the line).
    Horizontal,
    /// A point that may win both axes (a node, a box reference point).
    Both,
}

/// One gathered target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapTarget {
    /// The axes it may win.
    pub axis: TargetAxis,
    /// For a line, the coordinate on its own axis is the line and the other coordinate is not
    /// read; for [`TargetAxis::Both`], the point itself.
    pub point: Point,
    /// The tie order and the settings flag.
    pub kind: TargetKind,
    /// The extent the marker line runs along: the target's own box (a full page side for a
    /// fraction, a zero-height point for a node).
    pub extent: (Point, Point),
    /// Exact guide geometry; absent for alignment lines and point targets.
    pub curve: Option<SnapCurve>,
}

impl SnapTarget {
    /// The vertical line `x`, spanning `extent` on the other axis.
    #[must_use]
    pub const fn vertical(x: f64, extent: (Point, Point), kind: TargetKind) -> Self {
        Self {
            axis: TargetAxis::Vertical,
            point: Point::new(x, 0.0),
            kind,
            extent,
            curve: None,
        }
    }

    /// The horizontal line `y`, spanning `extent` on the other axis.
    #[must_use]
    pub const fn horizontal(y: f64, extent: (Point, Point), kind: TargetKind) -> Self {
        Self {
            axis: TargetAxis::Horizontal,
            point: Point::new(0.0, y),
            kind,
            extent,
            curve: None,
        }
    }

    /// A point target that may win both axes.
    #[must_use]
    pub const fn point(point: Point, extent: (Point, Point), kind: TargetKind) -> Self {
        Self {
            axis: TargetAxis::Both,
            point,
            kind,
            extent,
            curve: None,
        }
    }
}

/// A guide segment in global document coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapCurve {
    /// Start anchor.
    pub start: Point,
    /// Outgoing relative handle.
    pub handle_out: Vec2,
    /// Incoming relative handle.
    pub handle_in: Vec2,
    /// End anchor.
    pub end: Point,
}
impl SnapCurve {
    fn project(self, point: Point) -> Point {
        curvyo_geometry_core::nearest_point_on_segment(
            self.start,
            self.handle_out,
            self.handle_in,
            self.end,
            point,
            curvyo_document_core::Tolerance::from_mm(1e-9),
        )
        .2
    }
}

/// The stored settings resolved to document millimetres (`adrs.md` decision 3).
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // Independent target-kind switches.
pub struct SnapSettings {
    /// The document's on/off switch. Off means nothing pulls and no marker shows.
    pub enabled: bool,
    /// Snap to the boxes of other objects.
    pub objects: bool,
    /// Snap to guides.
    pub guides: bool,
    /// Snap to path nodes.
    pub nodes: bool,
    /// Snap to the six page fractions on each axis.
    pub page: bool,
    /// Allow correction of the horizontal coordinate.
    pub x: bool,
    /// Allow correction of the vertical coordinate.
    pub y: bool,
    /// How close a feature must come before it captures, in document millimetres.
    pub capture: f64,
    /// How far it must move away before it lets go, in document millimetres. Never smaller
    /// than `capture`.
    pub release: f64,
}

impl SnapSettings {
    /// Resolves the document's stored settings with `mm_per_px` (`1 / view scale`), raising a
    /// release zone below the capture zone to it. A non-finite or negative zone is treated as
    /// zero, so a caller can never create a NaN comparison.
    #[must_use]
    pub fn from_document(value: DocumentSnapping, mm_per_px: f64) -> Self {
        let zone = |px: f64| {
            let mm = px * mm_per_px;
            if mm.is_finite() && mm > 0.0 { mm } else { 0.0 }
        };
        let capture = zone(value.capture_px);
        Self {
            enabled: value.enabled,
            objects: value.objects,
            guides: value.guides,
            nodes: value.nodes,
            page: value.page,
            x: value.x,
            y: value.y,
            capture,
            release: zone(value.release_px).max(capture),
        }
    }

    /// The settings with snapping switched off, for a temporary suspension (Ctrl+Alt).
    #[must_use]
    pub const fn suspended(self) -> Self {
        Self {
            enabled: false,
            ..self
        }
    }

    /// Whether any target kind is enabled. With none, a drag never scans.
    #[must_use]
    pub const fn any_target(self) -> bool {
        self.objects || self.guides || self.nodes || self.page
    }
}

/// One moving feature on one axis: the raw coordinate that may be pulled and the other
/// coordinate of the same feature, where the marker cross sits when only this axis snaps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisProbe {
    /// The coordinate on this axis, raw (unsnapped).
    pub value: f64,
    /// The other coordinate of the same feature, for the cross.
    pub cross_other: f64,
}

/// The moving features of one frame, per axis (`adrs.md` decision 3).
///
/// A move passes the box's three x coordinates (left, centre, right) and three y coordinates
/// (top, centre, bottom); a single point passes one of each. The cross's other coordinate is
/// the edge midpoint for a box and the point itself for a single point.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SnapProbe {
    /// The x coordinates that may be pulled onto a vertical line.
    pub x: Vec<AxisProbe>,
    /// The y coordinates that may be pulled onto a horizontal line.
    pub y: Vec<AxisProbe>,
    /// Pairs of axis features that represent actual moving points.
    pub points: Vec<(usize, usize)>,
}

impl SnapProbe {
    /// A single moving point: the same point on both axes.
    #[must_use]
    pub fn point(point: Point) -> Self {
        Self {
            points: vec![(0, 0)],
            x: vec![AxisProbe {
                value: point.x,
                cross_other: point.y,
            }],
            y: vec![AxisProbe {
                value: point.y,
                cross_other: point.x,
            }],
        }
    }

    /// Multiple anchors with only their real coordinate pairs eligible for point capture.
    #[must_use]
    pub fn points(points: &[Point]) -> Self {
        Self {
            x: points
                .iter()
                .map(|p| AxisProbe {
                    value: p.x,
                    cross_other: p.y,
                })
                .collect(),
            y: points
                .iter()
                .map(|p| AxisProbe {
                    value: p.y,
                    cross_other: p.x,
                })
                .collect(),
            points: (0..points.len()).map(|i| (i, i)).collect(),
        }
    }

    /// A moving box: its four edges and two centre lines, with the edge midpoint as the cross's
    /// other coordinate.
    #[must_use]
    pub fn box_edges(min: Point, max: Point) -> Self {
        let centre = Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y));
        Self {
            points: (0..3).flat_map(|x| (0..3).map(move |y| (x, y))).collect(),
            x: [min.x, centre.x, max.x]
                .into_iter()
                .map(|value| AxisProbe {
                    value,
                    cross_other: centre.y,
                })
                .collect(),
            y: [min.y, centre.y, max.y]
                .into_iter()
                .map(|value| AxisProbe {
                    value,
                    cross_other: centre.x,
                })
                .collect(),
        }
    }

    /// A moving box whose cross is the dragged corner rather than the edge midpoint: a resize
    /// moves one corner or edge, so its cross sits on the dragged reference point.
    #[must_use]
    pub fn box_corners(min: Point, max: Point, cross: Point) -> Self {
        Self {
            points: (0..2).flat_map(|x| (0..2).map(move |y| (x, y))).collect(),
            x: [min.x, max.x]
                .into_iter()
                .map(|value| AxisProbe {
                    value,
                    cross_other: cross.y,
                })
                .collect(),
            y: [min.y, max.y]
                .into_iter()
                .map(|value| AxisProbe {
                    value,
                    cross_other: cross.x,
                })
                .collect(),
        }
    }

    /// Whether both axes are empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.x.is_empty() && self.y.is_empty()
    }
}

/// The held target of one axis: which entry of [`SnapTargets`], which probe captured it, and the
/// target coordinate. Ephemeral, never stored (`adrs.md` decision 9).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeldSnap {
    /// The index into [`SnapTargets::entries`].
    pub target: usize,
    /// The index into [`SnapProbe::x`] or [`SnapProbe::y`] that captured it.
    pub probe: usize,
    /// The target coordinate on this axis.
    pub coordinate: f64,
}

/// The one held target per axis, carried from one frame to the next.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SnapState {
    pub(crate) curve: Option<(HeldSnap, HeldSnap)>,
    /// The held target on the x axis.
    pub x: Option<HeldSnap>,
    /// The held target on the y axis.
    pub y: Option<HeldSnap>,
}

impl SnapState {
    /// Drops both held targets (release, Escape, tool change, a new press).
    pub fn clear(&mut self) {
        self.x = None;
        self.y = None;
        self.curve = None;
    }

    /// Whether nothing is held.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.x.is_none() && self.y.is_none()
    }
}

/// One line of the snap marker. The renderer extends it by 8 screen pixels at each end and draws
/// a 1 px white casing; the engine only says where the target is (`specs/0060-snapping` §2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MarkerLine {
    /// The actual matched guide geometry.
    Curve(SnapCurve),
    /// A vertical line at `x` from `y0` to `y1`.
    Vertical {
        /// The line's x.
        x: f64,
        /// The extent's top.
        y0: f64,
        /// The extent's bottom.
        y1: f64,
    },
    /// A horizontal line at `y` from `x0` to `x1`.
    Horizontal {
        /// The line's y.
        y: f64,
        /// The extent's left.
        x0: f64,
        /// The extent's right.
        x1: f64,
    },
}

/// The marker geometry: one line per held axis and a cross at the matched moving point.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SnapMarker {
    /// One line for the held x target and one for the held y target, at most two.
    pub lines: Vec<MarkerLine>,
    /// The cross at the matched moving point; absent when nothing holds.
    pub cross: Option<Point>,
}

impl SnapMarker {
    /// Whether no part of the marker is drawn.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty() && self.cross.is_none()
    }
}

/// What a resolve produced: the displacement to add to the raw position, the held targets and
/// the marker. [`Self::is_empty`] means nothing holds.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapResult {
    /// The displacement to add to the raw position, exactly the target coordinates.
    pub offset: Vec2,
    /// The held target on the x axis.
    pub held_x: Option<HeldSnap>,
    /// The held target on the y axis.
    pub held_y: Option<HeldSnap>,
    /// The marker geometry.
    pub marker: SnapMarker,
}

impl Default for SnapResult {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            held_x: None,
            held_y: None,
            marker: SnapMarker::default(),
        }
    }
}

impl SnapResult {
    /// Whether nothing holds: the raw position is the result.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.held_x.is_none() && self.held_y.is_none()
    }
}

mod collection;
mod compose;
mod constrained;
mod index;
mod resolve;
mod targets;
pub use constrained::{resolve_along, resolve_probe_along};
pub use resolve::resolve;
pub use targets::{SnapExclusions, SnapTargets};
#[cfg(test)]
mod tests;
