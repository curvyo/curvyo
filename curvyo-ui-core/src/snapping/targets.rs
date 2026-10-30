//! Immutable snapping targets collected from resolved geometry.
use super::{
    AnchorId, DocumentSize, HashSet, NodeId, NodeRead, ObjectSnapshot, PAGE_FRACTIONS, Point,
    SnapCurve, SnapSettings, SnapTarget, TargetAxis, TargetKind, object_bounds,
};
/// The flat target list, built once at the start of a drag (`adrs.md` decision 5).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SnapTargets {
    pub(super) entries: Vec<SnapTarget>,
    pub(super) index: super::index::Index,
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
    /// All anchors moved by a multi-anchor gesture.
    pub anchors: Option<&'a HashSet<(NodeId, AnchorId)>>,
}

impl SnapTargets {
    /// An empty list.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// A list from explicit entries; the tools and tests build small lists this way.
    #[must_use]
    pub fn new(entries: Vec<SnapTarget>) -> Self {
        Self {
            index: super::index::Index::build(&entries),
            entries,
        }
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
        let moving_boxes = super::collection::moving(nodes, exclusions.boxes);
        let mut moving_nodes = super::collection::moving(nodes, exclusions.nodes);
        if exclusions.anchors.is_some() {
            // Edited paths retain their stationary anchors; geometry following those paths
            // cannot supply a stationary target for the same gesture.
            moving_nodes.extend(
                moving_boxes
                    .iter()
                    .copied()
                    .filter(|id| !exclusions.boxes.is_some_and(|edited| edited.contains(id))),
            );
        }
        let bounds = super::collection::group_bounds(nodes);
        let moving_groups = super::collection::moving_groups(nodes, &moving_boxes);
        let excluded_box = |id: NodeId| moving_boxes.contains(&id);
        let excluded_nodes = |id: NodeId| moving_nodes.contains(&id);
        for read in nodes {
            if read.visible
                && matches!(read.kind, curvyo_document_core::NodeKind::Group)
                && settings.objects
                && !moving_groups.contains(&read.id)
                && let Some(&(min, max)) = bounds.get(&read.id)
            {
                push_box(&mut entries, min, max, TargetKind::Box);
            }
            let Some(object) = read.object() else {
                continue;
            };
            if !read.visible {
                continue;
            }
            if read.guide && settings.guides && !excluded_box(read.id) {
                push_guide_object(&mut entries, object, read.id, exclusions);
            }
            if !read.guide && settings.objects && !excluded_box(read.id) {
                let (min, max) = object_bounds(object);
                push_box(&mut entries, min, max, TargetKind::Box);
            }
            if settings.nodes
                && !excluded_nodes(read.id)
                && let Some(path) = object.drawn_path()
            {
                let is_path = matches!(object, ObjectSnapshot::Path(_) | ObjectSnapshot::Clone(_));
                if is_path {
                    for anchor in path.all_anchors() {
                        if exclusions.anchor == Some((read.id, anchor.id))
                            || exclusions
                                .anchors
                                .is_some_and(|ids| ids.contains(&(read.id, anchor.id)))
                        {
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
        if settings.page {
            let (width, height) = (size.width.as_mm(), size.height.as_mm());
            let page = (Point::new(0.0, 0.0), Point::new(width, height));
            for fraction in PAGE_FRACTIONS {
                entries.push(SnapTarget::vertical(
                    width * fraction,
                    page,
                    TargetKind::Page,
                ));
                entries.push(SnapTarget::horizontal(
                    height * fraction,
                    page,
                    TargetKind::Page,
                ));
            }
        }
        Self {
            index: super::index::Index::build(&entries),
            entries,
        }
    }

    /// Builds targets from every visible spatial page frame in global coordinates.
    #[must_use]
    pub fn gather_pages(
        nodes: &[NodeRead],
        pages: &[curvyo_document_core::Page],
        settings: &SnapSettings,
        exclusions: SnapExclusions<'_>,
    ) -> Self {
        let mut targets = Self::gather(
            nodes,
            DocumentSize::default(),
            &SnapSettings {
                page: false,
                ..*settings
            },
            exclusions,
        );
        if settings.page {
            for page in pages {
                let width = page.size.width.as_mm();
                let height = page.size.height.as_mm();
                let extent = (
                    page.origin,
                    Point::new(page.origin.x + width, page.origin.y + height),
                );
                for fraction in PAGE_FRACTIONS {
                    targets.entries.push(SnapTarget::vertical(
                        page.origin.x + width * fraction,
                        extent,
                        TargetKind::Page,
                    ));
                    targets.entries.push(SnapTarget::horizontal(
                        page.origin.y + height * fraction,
                        extent,
                        TargetKind::Page,
                    ));
                }
            }
        }
        targets.index = super::index::Index::build(&targets.entries);
        targets
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
        self.index = super::index::Index::build(&self.entries);
    }
}

/// Adds the three vertical and three horizontal lines and the nine reference points of a box.
pub(super) fn push_box(entries: &mut Vec<SnapTarget>, min: Point, max: Point, kind: TargetKind) {
    let extent = (min, max);
    let centre = Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y));
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

/// Adds a finite straight guide segment.
#[cfg(test)]
pub(super) fn push_guide(entries: &mut Vec<SnapTarget>, start: Point, end: Point) {
    push_curve(
        entries,
        SnapCurve {
            start,
            end,
            handle_out: super::Vec2::ZERO,
            handle_in: super::Vec2::ZERO,
        },
    );
}
fn push_curve(entries: &mut Vec<SnapTarget>, curve: SnapCurve) {
    let bounds = curvyo_geometry_core::segment_bounds(
        curve.start,
        curve.handle_out,
        curve.handle_in,
        curve.end,
    );
    entries.push(SnapTarget {
        axis: TargetAxis::Both,
        point: curve.start,
        extent: bounds,
        kind: TargetKind::Guide,
        curve: Some(curve),
    });
}

fn push_guide_object(
    entries: &mut Vec<SnapTarget>,
    object: &ObjectSnapshot,
    id: NodeId,
    exclusions: SnapExclusions<'_>,
) {
    if let Some(path) = object.drawn_path() {
        for subpath in path.subpaths() {
            for (a, b) in crate::hit_test::segment_pairs(subpath.anchors.len(), subpath.closed) {
                let start = &subpath.anchors[a];
                let end = &subpath.anchors[b];
                push_curve(
                    entries,
                    SnapCurve {
                        start: start.point,
                        handle_out: start.handle_out,
                        handle_in: end.handle_in,
                        end: end.point,
                    },
                );
            }
        }
    }
    if let ObjectSnapshot::Primitive(primitive) = object {
        let outline =
            curvyo_document_core::outline_of_rotated(&primitive.shape, primitive.rotation);
        for (a, b) in crate::hit_test::segment_pairs(outline.len(), outline.closed) {
            let start = &outline[a];
            let end = &outline[b];
            push_curve(
                entries,
                SnapCurve {
                    start: start.point,
                    handle_out: start.handle_out,
                    handle_in: end.handle_in,
                    end: end.point,
                },
            );
        }
    }
    if let Some(path) = object.drawn_path() {
        for anchor in path.all_anchors() {
            if exclusions.anchor == Some((id, anchor.id))
                || exclusions
                    .anchors
                    .is_some_and(|ids| ids.contains(&(id, anchor.id)))
            {
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
