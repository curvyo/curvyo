//! The pure snapping engine (`specs/0060-snapping`, `adrs.md` decisions 3 to 5 and 8):
//! the stored settings resolved to document millimetres, the target list built once at the
//! start of a drag, the moving features of one frame, the one held target per axis and the
//! marker geometry.
//!
//! No document access and no rendering: the caller converts the stored screen-pixel zones with
//! the current zoom, gathers [`SnapTargets`] from one read, and draws [`SnapResult::marker`].
//! Everything here is a pure function of the raw pointer-derived position, so a fast and a slow
//! drag behave the same and a target cannot keep itself alive by its own output (criterion 4).

use std::collections::HashSet;

use curvyo_document_core::{
    AnchorId, DocumentSize, DocumentSnapping, NodeId, NodeRead, ObjectSnapshot, Point, Vec2,
};

use crate::object_bounds::{object_bounds, object_outline_bounds};

/// The six page fractions on each axis (criterion 5, `adrs.md` decision 8).
///
/// The customer's "33.3 %" and "66.6 %" are exact thirds, not `0.333` and `0.666`: the line must
/// coincide with the thirds `0061-align-and-distribute` uses, and a third of a 297 mm page is
/// 99 mm while 33.3 % is 98.901 mm.
pub const PAGE_FRACTIONS: [f64; 6] = [0.25, 1.0 / 3.0, 0.5, 2.0 / 3.0, 0.75, 1.0];

/// Two coordinates closer than this are the same page side or box edge (millimetres). A guide
/// drawn as a straight horizontal or vertical line is one line, not a four-edge box.
const DEGENERATE_MM: f64 = 1e-9;

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
        }
    }
}

/// The stored settings resolved to document millimetres (`adrs.md` decision 3).
#[derive(Debug, Clone, Copy, PartialEq)]
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
}

impl SnapProbe {
    /// A single moving point: the same point on both axes.
    #[must_use]
    pub fn point(point: Point) -> Self {
        Self {
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

    /// A moving box: its four edges and two centre lines, with the edge midpoint as the cross's
    /// other coordinate.
    #[must_use]
    pub fn box_edges(min: Point, max: Point) -> Self {
        let centre = Point::new((min.x + max.x) / 2.0, (min.y + max.y) / 2.0);
        Self {
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

/// The flat target list, built once at the start of a drag (`adrs.md` decision 5).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SnapTargets {
    entries: Vec<SnapTarget>,
}

/// Which nodes and anchors a drag excludes from the target list.
#[derive(Debug, Clone, Copy, Default)]
pub struct SnapExclusions<'a> {
    /// Nodes that contribute no box and no guide line: the selection being dragged.
    pub boxes: Option<&'a HashSet<NodeId>>,
    /// Nodes that contribute no node points: the selection being dragged by a move or resize.
    /// The Node tool leaves its edited path out of this set, so the path's other anchors remain
    /// targets while the dragged one is named by [`Self::anchor`].
    pub nodes: Option<&'a HashSet<NodeId>>,
    /// The one anchor being dragged: it contributes no node point even on an otherwise
    /// contributing path.
    pub anchor: Option<(NodeId, AnchorId)>,
}

impl SnapTargets {
    /// An empty list.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// A list from explicit entries; the tools and tests build small lists this way.
    #[must_use]
    pub const fn new(entries: Vec<SnapTarget>) -> Self {
        Self { entries }
    }

    /// Every target, in document order per kind.
    #[must_use]
    pub fn entries(&self) -> &[SnapTarget] {
        &self.entries
    }

    /// How many targets the list holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether there is no target at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Builds the list from the one read the session already holds (criterion 11): the boxes of
    /// every effectively visible leaf (locked included, hidden excluded), the lines and anchors
    /// of guides, the anchors of visible paths, and the six page fractions on each axis.
    /// `settings` filters by kind; `exclusions` names the drag's own objects.
    #[must_use]
    pub fn gather(
        nodes: &[NodeRead],
        size: DocumentSize,
        settings: &SnapSettings,
        exclusions: SnapExclusions<'_>,
    ) -> Self {
        let mut entries = Vec::new();
        let excluded_box = |id: NodeId| exclusions.boxes.is_some_and(|set| set.contains(&id));
        let excluded_nodes = |id: NodeId| exclusions.nodes.is_some_and(|set| set.contains(&id));
        for read in nodes {
            let Some(object) = read.object() else {
                continue;
            };
            if !read.visible {
                continue;
            }
            if read.guide {
                if settings.guides && !excluded_box(read.id) {
                    let (min, max) = object_outline_bounds(object);
                    push_guide(&mut entries, min, max);
                    if let Some(path) = object.drawn_path() {
                        for anchor in path.all_anchors() {
                            if exclusions.anchor == Some((read.id, anchor.id)) {
                                continue;
                            }
                            entries.push(SnapTarget::point(
                                anchor.point,
                                (anchor.point, anchor.point),
                                TargetKind::Guide,
                            ));
                        }
                    }
                }
                continue;
            }
            if settings.objects && !excluded_box(read.id) {
                let (min, max) = object_bounds(object);
                push_box(&mut entries, min, max, TargetKind::Box);
            }
            if settings.nodes && !excluded_nodes(read.id) {
                if let Some(path) = object.drawn_path() {
                    let is_path = matches!(
                        object,
                        ObjectSnapshot::Path(_) | ObjectSnapshot::Clone(_)
                    );
                    if is_path {
                        for anchor in path.all_anchors() {
                            if exclusions.anchor == Some((read.id, anchor.id)) {
                                continue;
                            }
                            entries.push(SnapTarget::point(
                                anchor.point,
                                (anchor.point, anchor.point),
                                TargetKind::Node,
                            ));
                        }
                    }
                }
            }
        }
        if settings.page {
            let (width, height) = (size.width.as_mm(), size.height.as_mm());
            let page = (Point::new(0.0, 0.0), Point::new(width, height));
            for fraction in PAGE_FRACTIONS {
                entries.push(SnapTarget::vertical(width * fraction, page, TargetKind::Page));
                entries.push(SnapTarget::horizontal(
                    height * fraction,
                    page,
                    TargetKind::Page,
                ));
            }
        }
        Self { entries }
    }

    /// Adds the anchors of `path` as node targets, skipping `skip`: the Node tool's own path
    /// (criterion 5, "and of the path the Node tool edits").
    pub fn add_path_nodes(
        &mut self,
        path: &curvyo_document_core::PathSnapshot,
        skip: Option<AnchorId>,
    ) {
        for anchor in path.all_anchors() {
            if Some(anchor.id) == skip {
                continue;
            }
            self.entries.push(SnapTarget::point(
                anchor.point,
                (anchor.point, anchor.point),
                TargetKind::Node,
            ));
        }
    }
}

/// Adds the three vertical and three horizontal lines and the nine reference points of a box.
fn push_box(entries: &mut Vec<SnapTarget>, min: Point, max: Point, kind: TargetKind) {
    let extent = (min, max);
    let centre = Point::new((min.x + max.x) / 2.0, (min.y + max.y) / 2.0);
    for x in [min.x, centre.x, max.x] {
        entries.push(SnapTarget::vertical(x, extent, kind));
    }
    for y in [min.y, centre.y, max.y] {
        entries.push(SnapTarget::horizontal(y, extent, kind));
    }
    for x in [min.x, centre.x, max.x] {
        for y in [min.y, centre.y, max.y] {
            entries.push(SnapTarget::point(Point::new(x, y), extent, kind));
        }
    }
}

/// Adds a guide's own lines: one line for a straight guide, four edges for a guide with area.
fn push_guide(entries: &mut Vec<SnapTarget>, min: Point, max: Point) {
    let extent = (min, max);
    let flat_y = (max.y - min.y).abs() <= DEGENERATE_MM;
    let flat_x = (max.x - min.x).abs() <= DEGENERATE_MM;
    if flat_y {
        entries.push(SnapTarget::horizontal(min.y, extent, TargetKind::Guide));
        return;
    }
    if flat_x {
        entries.push(SnapTarget::vertical(min.x, extent, TargetKind::Guide));
        return;
    }
    entries.push(SnapTarget::horizontal(min.y, extent, TargetKind::Guide));
    entries.push(SnapTarget::horizontal(max.y, extent, TargetKind::Guide));
    entries.push(SnapTarget::vertical(min.x, extent, TargetKind::Guide));
    entries.push(SnapTarget::vertical(max.x, extent, TargetKind::Guide));
}

/// Resolves one frame (`adrs.md` decision 4): with a held target per axis that is still within
/// the release zone of the same probe, keep it; otherwise the nearest target within the capture
/// zone wins, ties broken by kind and then document order.
///
/// `state` is updated in place with the result's held targets.
pub fn resolve(
    settings: &SnapSettings,
    targets: &SnapTargets,
    probe: &SnapProbe,
    state: &mut SnapState,
) -> SnapResult {
    if !settings.enabled || !settings.any_target() || probe.is_empty() || targets.is_empty() {
        state.clear();
        return SnapResult::default();
    }
    let held_x = resolve_axis(
        settings,
        targets,
        &probe.x,
        TargetAxis::Vertical,
        &mut state.x,
    );
    let held_y = resolve_axis(
        settings,
        targets,
        &probe.y,
        TargetAxis::Horizontal,
        &mut state.y,
    );
    if held_x.is_none() && held_y.is_none() {
        return SnapResult::default();
    }
    let offset = Vec2::new(
        held_x.map_or(0.0, |held| {
            held.coordinate - probe.x[held.probe].value
        }),
        held_y.map_or(0.0, |held| {
            held.coordinate - probe.y[held.probe].value
        }),
    );
    let marker = marker_of(targets, probe, held_x, held_y);
    SnapResult {
        offset,
        held_x,
        held_y,
        marker,
    }
}

/// Resolves the held target for one axis. `axis` names the scan: a vertical line wins the x
/// axis, a horizontal one the y axis, and a point may win either.
fn resolve_axis(
    settings: &SnapSettings,
    targets: &SnapTargets,
    probes: &[AxisProbe],
    axis: TargetAxis,
    held: &mut Option<HeldSnap>,
) -> Option<HeldSnap> {
    if probes.is_empty() {
        *held = None;
        return None;
    }
    if let Some(current) = *held {
        let still_holds = targets.entries.get(current.target).is_some_and(|target| {
            let still_a_target = match axis {
                TargetAxis::Vertical => target.axis != TargetAxis::Horizontal,
                TargetAxis::Horizontal => target.axis != TargetAxis::Vertical,
                TargetAxis::Both => false,
            };
            still_a_target
                && target.kind.is_enabled(settings)
                && probes.get(current.probe).is_some_and(|probe| {
                    (probe.value - current.coordinate).abs() <= settings.release
                })
        });
        if still_holds {
            return Some(current);
        }
        *held = None;
    }
    let mut best: Option<(f64, TargetKind, usize, usize, f64)> = None;
    for (index, target) in targets.entries.iter().enumerate() {
        let coordinate = match (axis, target.axis) {
            (TargetAxis::Vertical, TargetAxis::Horizontal) => continue,
            (TargetAxis::Horizontal, TargetAxis::Vertical) => continue,
            (TargetAxis::Vertical, _) => target.point.x,
            (TargetAxis::Horizontal, _) => target.point.y,
            (TargetAxis::Both, _) => unreachable!("one axis at a time"),
        };
        if !target.kind.is_enabled(settings) {
            continue;
        }
        for (probe_index, probe) in probes.iter().enumerate() {
            let distance = (probe.value - coordinate).abs();
            if distance > settings.capture {
                continue;
            }
            let candidate = (distance, target.kind, index, probe_index, coordinate);
            let better = best.is_none_or(|(best_distance, best_kind, best_index, _, _)| {
                candidate.0 < best_distance
                    || (candidate.0 == best_distance
                        && (candidate.1 < best_kind
                            || (candidate.1 == best_kind && candidate.2 < best_index)))
            });
            if better {
                best = Some(candidate);
            }
        }
    }
    let chosen = best.map(|(_, _, target, probe, coordinate)| HeldSnap {
        target,
        probe,
        coordinate,
    });
    *held = chosen;
    chosen
}

/// The marker for the held targets: one line per axis along the target's extent and a cross at
/// the matched moving point. A point target that wins both axes draws both lines through itself.
fn marker_of(
    targets: &SnapTargets,
    probe: &SnapProbe,
    held_x: Option<HeldSnap>,
    held_y: Option<HeldSnap>,
) -> SnapMarker {
    let mut lines = Vec::with_capacity(2);
    if let Some(held) = held_x {
        if let Some(target) = targets.entries.get(held.target) {
            lines.push(MarkerLine::Vertical {
                x: held.coordinate,
                y0: target.extent.0.y,
                y1: target.extent.1.y,
            });
        }
    }
    if let Some(held) = held_y {
        if let Some(target) = targets.entries.get(held.target) {
            lines.push(MarkerLine::Horizontal {
                y: held.coordinate,
                x0: target.extent.0.x,
                x1: target.extent.1.x,
            });
        }
    }
    let cross = match (held_x, held_y) {
        (Some(x), Some(y)) => Some(Point::new(x.coordinate, y.coordinate)),
        (Some(x), None) => probe
            .x
            .get(x.probe)
            .map(|probe| Point::new(x.coordinate, probe.cross_other)),
        (None, Some(y)) => probe
            .y
            .get(y.probe)
            .map(|probe| Point::new(probe.cross_other, y.coordinate)),
        (None, None) => None,
    };
    SnapMarker { lines, cross }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{Document, Length, RectBounds};

    fn settings(capture: f64, release: f64) -> SnapSettings {
        SnapSettings {
            enabled: true,
            objects: true,
            guides: true,
            nodes: true,
            page: true,
            capture,
            release: release.max(capture),
        }
    }

    fn one_vertical(x: f64, kind: TargetKind) -> SnapTargets {
        SnapTargets::new(vec![SnapTarget::vertical(
            x,
            (Point::new(0.0, 0.0), Point::new(0.0, 100.0)),
            kind,
        )])
    }

    fn x_probe(x: f64) -> SnapProbe {
        SnapProbe {
            x: vec![AxisProbe {
                value: x,
                cross_other: 50.0,
            }],
            y: Vec::new(),
        }
    }

    #[test]
    fn a_new_document_resolves_to_the_default_zones_at_one_to_one() {
        let value = SnapSettings::from_document(DocumentSnapping::DEFAULT, 1.0);
        assert!(!value.enabled);
        assert!(value.objects && value.guides && value.nodes && value.page);
        assert_eq!(value.capture, 8.0);
        assert_eq!(value.release, 14.0);
    }

    #[test]
    fn the_zones_convert_from_pixels_by_the_zoom() {
        let value = SnapSettings::from_document(DocumentSnapping::DEFAULT, 0.5);
        assert_eq!(value.capture, 4.0);
        assert_eq!(value.release, 7.0);
    }

    #[test]
    fn a_release_below_the_capture_is_raised_to_it_on_read() {
        let stored = DocumentSnapping {
            capture_px: 10.0,
            release_px: 3.0,
            ..DocumentSnapping::DEFAULT
        };
        let value = SnapSettings::from_document(stored, 1.0);
        assert_eq!(value.capture, 10.0);
        assert_eq!(value.release, 10.0);
    }

    #[test]
    fn capture_is_exactly_at_the_zone_edge() {
        let targets = one_vertical(100.0, TargetKind::Box);
        let mut state = SnapState::default();
        let mut probe = x_probe(108.0);
        let result = resolve(&settings(8.0, 14.0), &targets, &probe, &mut state);
        assert_eq!(result.held_x.unwrap().coordinate, 100.0);
        assert_eq!(result.offset.x, -8.0);
        probe.x[0].value = 108.000_1;
        let mut fresh = SnapState::default();
        let result = resolve(&settings(8.0, 14.0), &targets, &probe, &mut fresh);
        assert!(result.held_x.is_none(), "beyond capture");
    }

    #[test]
    fn a_held_target_survives_to_the_release_edge_and_lets_go_beyond_it() {
        let targets = one_vertical(100.0, TargetKind::Box);
        let mut state = SnapState::default();
        let mut probe = x_probe(108.0);
        assert!(resolve(&settings(8.0, 14.0), &targets, &probe, &mut state).held_x.is_some());
        probe.x[0].value = 113.0;
        let result = resolve(&settings(8.0, 14.0), &targets, &probe, &mut state);
        assert_eq!(result.held_x.unwrap().coordinate, 100.0, "13 is inside 14");
        probe.x[0].value = 114.0;
        let result = resolve(&settings(8.0, 14.0), &targets, &probe, &mut state);
        assert!(result.held_x.is_some(), "exactly 14 still holds");
        probe.x[0].value = 114.001;
        let result = resolve(&settings(8.0, 14.0), &targets, &probe, &mut state);
        assert!(result.held_x.is_none(), "beyond release and beyond capture");
    }

    /// A sweep across a line must capture once and release once, never flicker, because every
    /// distance is measured from the raw position (criterion 4).
    #[test]
    fn a_slow_sweep_captures_once_and_releases_once() {
        let targets = one_vertical(100.0, TargetKind::Box);
        let mut state = SnapState::default();
        let mut captures = 0;
        let mut releases = 0;
        let mut held = false;
        let mut probe = x_probe(80.0);
        for step in 0..=400 {
            probe.x[0].value = 80.0 + f64::from(step) * 0.1;
            let result = resolve(&settings(8.0, 14.0), &targets, &probe, &mut state);
            let now = result.held_x.is_some();
            if now && !held {
                captures += 1;
            }
            if !now && held {
                releases += 1;
            }
            held = now;
        }
        assert_eq!(captures, 1);
        assert_eq!(releases, 1);
    }

    #[test]
    fn the_nearest_target_wins_even_against_a_higher_kind() {
        let targets = SnapTargets::new(vec![
            SnapTarget::vertical(100.0, (Point::new(0.0, 0.0), Point::new(0.0, 10.0)), TargetKind::Node),
            SnapTarget::vertical(104.0, (Point::new(0.0, 0.0), Point::new(0.0, 10.0)), TargetKind::Page),
        ]);
        let mut state = SnapState::default();
        let result = resolve(&settings(8.0, 14.0), &targets, &x_probe(105.0), &mut state);
        assert_eq!(result.held_x.unwrap().coordinate, 104.0);
        assert_eq!(result.held_x.unwrap().target, 1);
    }

    #[test]
    fn an_exact_tie_goes_nodes_then_guides_then_boxes_then_page() {
        let kinds = [
            TargetKind::Page,
            TargetKind::Box,
            TargetKind::Guide,
            TargetKind::Node,
        ];
        let targets = SnapTargets::new(
            kinds
                .iter()
                .map(|kind| {
                    SnapTarget::vertical(
                        100.0,
                        (Point::new(0.0, 0.0), Point::new(0.0, 10.0)),
                        *kind,
                    )
                })
                .collect(),
        );
        let mut state = SnapState::default();
        let result = resolve(&settings(8.0, 14.0), &targets, &x_probe(103.0), &mut state);
        assert_eq!(result.held_x.unwrap().target, 3, "node entry is last");
    }

    #[test]
    fn a_point_target_can_win_both_axes_at_once() {
        let targets = SnapTargets::new(vec![SnapTarget::point(
            Point::new(100.0, 100.0),
            (Point::new(100.0, 100.0), Point::new(100.0, 100.0)),
            TargetKind::Node,
        )]);
        let mut state = SnapState::default();
        let probe = SnapProbe::point(Point::new(104.0, 96.0));
        let result = resolve(&settings(8.0, 14.0), &targets, &probe, &mut state);
        assert!(result.held_x.is_some() && result.held_y.is_some());
        assert_eq!(result.offset, Vec2::new(-4.0, 4.0));
        assert_eq!(result.marker.cross, Some(Point::new(100.0, 100.0)));
        assert_eq!(result.marker.lines.len(), 2);
    }

    #[test]
    fn an_empty_probe_on_one_axis_leaves_that_axis_free() {
        let targets = SnapTargets::new(vec![
            SnapTarget::vertical(100.0, (Point::new(0.0, 0.0), Point::new(0.0, 10.0)), TargetKind::Box),
            SnapTarget::horizontal(100.0, (Point::new(0.0, 0.0), Point::new(10.0, 0.0)), TargetKind::Box),
        ]);
        let mut state = SnapState::default();
        let probe = SnapProbe {
            x: vec![AxisProbe { value: 103.0, cross_other: 20.0 }],
            y: Vec::new(),
        };
        let result = resolve(&settings(8.0, 14.0), &targets, &probe, &mut state);
        assert!(result.held_x.is_some());
        assert!(result.held_y.is_none());
        assert_eq!(result.offset, Vec2::new(-3.0, 0.0));
        assert_eq!(result.marker.cross, Some(Point::new(100.0, 20.0)));
        assert_eq!(result.marker.lines.len(), 1);
    }

    #[test]
    fn switching_snapping_off_clears_the_held_target_and_returns_nothing() {
        let targets = one_vertical(100.0, TargetKind::Box);
        let mut state = SnapState::default();
        assert!(resolve(&settings(8.0, 14.0), &targets, &x_probe(103.0), &mut state).held_x.is_some());
        let off = settings(8.0, 14.0).suspended();
        let result = resolve(&off, &targets, &x_probe(103.0), &mut state);
        assert!(result.is_empty());
        assert!(state.is_empty());
    }

    #[test]
    fn a_disabled_kind_contributes_nothing() {
        let mut value = settings(8.0, 14.0);
        value.objects = false;
        let targets = one_vertical(100.0, TargetKind::Box);
        let mut state = SnapState::default();
        assert!(resolve(&value, &targets, &x_probe(103.0), &mut state).is_empty());
    }

    #[test]
    fn page_fractions_are_exact_thirds_on_both_axes() {
        let settings = SnapSettings { enabled: true, objects: false, guides: false, nodes: false, page: true, capture: 1.0, release: 1.0 };
        let targets = SnapTargets::gather(&[], DocumentSize::from_mm(210.0, 297.0), &settings, SnapExclusions::default());
        let vertical: Vec<f64> = targets.entries().iter().filter(|t| t.axis == TargetAxis::Vertical).map(|t| t.point.x).collect();
        let horizontal: Vec<f64> = targets.entries().iter().filter(|t| t.axis == TargetAxis::Horizontal).map(|t| t.point.y).collect();
        assert_eq!(vertical, vec![52.5, 70.0, 105.0, 140.0, 157.5, 210.0]);
        assert_eq!(horizontal, vec![74.25, 99.0, 148.5, 198.0, 222.75, 297.0]);
        assert!(!vertical.contains(&0.0));
        assert!(!horizontal.contains(&0.0));
    }

    #[test]
    fn page_fractions_follow_a_resize_with_nothing_stored() {
        let settings = SnapSettings { enabled: true, objects: false, guides: false, nodes: false, page: true, capture: 1.0, release: 1.0 };
        let targets = SnapTargets::gather(&[], DocumentSize::from_mm(300.0, 400.0), &settings, SnapExclusions::default());
        let vertical: Vec<f64> = targets.entries().iter().filter(|t| t.axis == TargetAxis::Vertical).map(|t| t.point.x).collect();
        assert_eq!(vertical, vec![75.0, 100.0, 150.0, 200.0, 225.0, 300.0]);
    }

    #[test]
    fn object_boxes_contribute_three_lines_each_way_and_nine_points() {
        let mut entries = Vec::new();
        push_box(&mut entries, Point::new(0.0, 0.0), Point::new(10.0, 20.0), TargetKind::Box);
        assert_eq!(entries.iter().filter(|t| t.axis == TargetAxis::Vertical).count(), 3);
        assert_eq!(entries.iter().filter(|t| t.axis == TargetAxis::Horizontal).count(), 3);
        assert_eq!(entries.iter().filter(|t| t.axis == TargetAxis::Both).count(), 9);
    }

    #[test]
    fn a_straight_guide_is_one_line_not_a_box() {
        let mut entries = Vec::new();
        push_guide(&mut entries, Point::new(0.0, 50.0), Point::new(100.0, 50.0));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].axis, TargetAxis::Horizontal);
        assert_eq!(entries[0].point.y, 50.0);
    }

    #[test]
    fn gather_excludes_hidden_objects_and_keeps_locked_ones() {
        let document = Document::new(1);
        let hidden = document.create_rect(RectBounds::from_corners(
            Point::new(0.0, 0.0),
            Point::new(10.0, 10.0),
        ));
        let locked = document.create_rect(RectBounds::from_corners(
            Point::new(30.0, 30.0),
            Point::new(40.0, 40.0),
        ));
        document.set_node_visible(&[hidden], false).unwrap();
        document.set_node_locked(&[locked], true).unwrap();
        let reads = document.read_nodes();
        let settings = SnapSettings { enabled: true, objects: true, guides: false, nodes: false, page: false, capture: 8.0, release: 14.0 };
        let targets = SnapTargets::gather(&reads, document.size(), &settings, SnapExclusions::default());
        let xs: Vec<f64> = targets.entries().iter().filter(|t| t.axis == TargetAxis::Vertical).map(|t| t.point.x).collect();
        assert!(!xs.contains(&0.0) && !xs.contains(&10.0), "hidden box is no target");
        assert!(xs.contains(&30.0) && xs.contains(&40.0), "locked box is a target");
    }

    #[test]
    fn gather_skips_the_dragged_object() {
        let document = Document::new(1);
        let dragged = document.create_rect(RectBounds::from_corners(
            Point::new(0.0, 0.0),
            Point::new(10.0, 10.0),
        ));
        let other = document.create_rect(RectBounds::from_corners(
            Point::new(30.0, 30.0),
            Point::new(40.0, 40.0),
        ));
        let mut excluded = HashSet::new();
        excluded.insert(dragged);
        let reads = document.read_nodes();
        let settings = SnapSettings { enabled: true, objects: true, guides: false, nodes: false, page: false, capture: 8.0, release: 14.0 };
        let targets = SnapTargets::gather(&reads, document.size(), &settings, SnapExclusions { boxes: Some(&excluded), nodes: Some(&excluded), anchor: None });
        let xs: Vec<f64> = targets.entries().iter().filter(|t| t.axis == TargetAxis::Vertical).map(|t| t.point.x).collect();
        assert!(!xs.contains(&0.0) && !xs.contains(&10.0));
        assert!(xs.contains(&30.0) && xs.contains(&40.0));
        assert!(document.object(other).is_some());
    }

    #[test]
    fn a_guide_clone_and_its_anchors_are_targets() {
        let document = Document::new(1);
        let guide = document.create_path(
            &[
                curvyo_document_core::NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 70.0)),
                curvyo_document_core::NewAnchor::corner(AnchorId::new(1, 2), Point::new(100.0, 70.0)),
            ],
            false,
        );
        document.set_guide(&[guide], true).unwrap();
        let reads = document.read_nodes();
        let settings = SnapSettings { enabled: true, objects: false, guides: true, nodes: false, page: false, capture: 8.0, release: 14.0 };
        let targets = SnapTargets::gather(&reads, document.size(), &settings, SnapExclusions::default());
        assert!(targets.entries().iter().any(|t| t.axis == TargetAxis::Horizontal && (t.point.y - 70.0).abs() < 1e-9));
        assert_eq!(targets.entries().iter().filter(|t| t.axis == TargetAxis::Both && t.kind == TargetKind::Guide).count(), 2);
    }

    #[test]
    fn the_node_tool_can_add_its_own_path_anchors_but_not_the_dragged_one() {
        let document = Document::new(1);
        let id = document.create_path(
            &[
                curvyo_document_core::NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
                curvyo_document_core::NewAnchor::corner(AnchorId::new(1, 2), Point::new(50.0, 50.0)),
            ],
            false,
        );
        let path = match document.object(id).unwrap() {
            ObjectSnapshot::Path(path) => path,
            _ => panic!("path"),
        };
        let settings = SnapSettings { enabled: true, objects: false, guides: false, nodes: true, page: false, capture: 8.0, release: 14.0 };
        let reads = document.read_nodes();
        let mut excluded = HashSet::new();
        excluded.insert(id);
        let mut targets = SnapTargets::gather(&reads, document.size(), &settings, SnapExclusions { boxes: Some(&excluded), nodes: Some(&excluded), anchor: None });
        assert!(targets.is_empty());
        targets.add_path_nodes(&path, Some(AnchorId::new(1, 1)));
        assert_eq!(targets.len(), 1);
        assert_eq!(targets.entries()[0].point, Point::new(50.0, 50.0));
    }

    #[test]
    fn gather_excludes_one_dragged_anchor_from_an_otherwise_included_path() {
        let document = Document::new(1);
        let id = document.create_path(
            &[
                curvyo_document_core::NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
                curvyo_document_core::NewAnchor::corner(AnchorId::new(1, 2), Point::new(50.0, 50.0)),
            ],
            false,
        );
        let reads = document.read_nodes();
        let settings = SnapSettings { enabled: true, objects: false, guides: false, nodes: true, page: false, capture: 8.0, release: 14.0 };
        let targets = SnapTargets::gather(
            &reads,
            document.size(),
            &settings,
            SnapExclusions { boxes: None, nodes: None, anchor: Some((id, AnchorId::new(1, 1))) },
        );
        assert_eq!(targets.len(), 1);
        assert_eq!(targets.entries()[0].point, Point::new(50.0, 50.0));
    }

    #[test]
    fn a_box_move_snaps_by_its_edges_and_centre_lines() {
        let document = Document::new(1);
        let _ = document.create_rect(RectBounds::from_corners(
            Point::new(100.0, 100.0),
            Point::new(110.0, 120.0),
        ));
        let reads = document.read_nodes();
        let settings = SnapSettings { enabled: true, objects: true, guides: false, nodes: false, page: false, capture: 8.0, release: 14.0 };
        let targets = SnapTargets::gather(&reads, document.size(), &settings, SnapExclusions::default());
        let mut state = SnapState::default();
        let probe = SnapProbe::box_edges(Point::new(92.0, 92.0), Point::new(102.0, 112.0));
        let result = resolve(&settings, &targets, &probe, &mut state);
        assert_eq!(result.offset, Vec2::new(-2.0, -2.0));
        assert_eq!(result.marker.cross, Some(Point::new(100.0, 100.0)));
    }

    #[test]
    fn a_length_of_two_thirds_is_an_exact_third_not_a_decimal() {
        assert!((210.0 * PAGE_FRACTIONS[3] - 140.0).abs() < 1e-9);
        assert!((297.0 * PAGE_FRACTIONS[1] - 99.0).abs() < 1e-9);
        assert!((Length::from_mm(297.0).as_mm() * PAGE_FRACTIONS[4] - 222.75).abs() < 1e-9);
    }
}
