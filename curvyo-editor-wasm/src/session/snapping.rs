//! Snapping state for a drag in flight (`specs/0060-snapping`, `adrs.md`
//! decisions 3 to 5): reads the document's settings, gathers the target list
//! once at the press, holds the ephemeral result for drawing, and drops it on
//! release/Escape/tool change.

use std::collections::HashSet;

use curvyo_document_core::{AnchorId, DocumentSnapping, DocumentVersion, NodeId, Page, Point};
use curvyo_ui_core::{SnapExclusions, SnapProbe, SnapResult, SnapSettings, SnapState, SnapTargets};

use super::object_cache::DocumentRead;

/// The gathered targets and the resolved settings of a drag in flight.
#[derive(Debug, Clone)]
pub struct SnappingDrag {
    /// The settings resolved with the current zoom.
    settings: SnapSettings,
    /// The flat list built once at the press, kept for the drag's whole life.
    targets: SnapTargets,
    /// The one held target per axis, ephemeral.
    state: SnapState,
    /// The last resolved result, for the marker.
    last_result: SnapResult,
    nodes_only: bool,
    boxes: Option<HashSet<NodeId>>,
    nodes: Option<HashSet<NodeId>>,
    anchors: HashSet<(NodeId, AnchorId)>,
}

impl SnappingDrag {
    /// Builds the target list from `read` and `pages` with the given settings,
    /// excluding `excluded` nodes and anchors.
    pub fn new(
        read: &DocumentRead,
        pages: &[Page],
        settings: SnapSettings,
        exclusions: SnapExclusions<'_>,
    ) -> Self {
        let all = SnapSettings {
            objects: true,
            guides: true,
            nodes: true,
            page: true,
            ..settings
        };
        let targets = SnapTargets::gather_pages(&read.nodes, pages, &all, exclusions);
        let mut anchors = exclusions.anchors.cloned().unwrap_or_default();
        anchors.extend(exclusions.anchor);
        Self {
            settings,
            targets,
            state: SnapState::default(),
            last_result: SnapResult::default(),
            nodes_only: false,
            boxes: exclusions.boxes.cloned(),
            nodes: exclusions.nodes.cloned(),
            anchors,
        }
    }

    /// The last resolved result, for drawing the marker.
    pub fn last_result(&self) -> &SnapResult {
        &self.last_result
    }
}

/// The session's snapping state: the stored settings, the drag in flight, and
/// the resolved settings for the panel.
#[derive(Debug)]
pub struct SnappingState {
    /// The document's stored settings, read at open and after every peer
    /// change.
    stored: DocumentSnapping,
    /// The drag in flight, if any.
    drag: Option<SnappingDrag>,
    pub(super) version: Option<DocumentVersion>,
}

impl SnappingState {
    /// A fresh state with the defaults.
    pub fn new(stored: DocumentSnapping) -> Self {
        Self {
            stored,
            drag: None,
            version: None,
        }
    }

    /// Updates the stored settings (a peer wrote them).
    pub fn update_stored(&mut self, stored: DocumentSnapping) {
        self.stored = stored;
    }

    /// Whether a drag is in flight.
    pub fn drag_in_flight(&self) -> bool {
        self.drag.is_some()
    }

    /// Begins a drag with the current zoom (`mm_per_px = 1 / view.scale()`),
    /// gathering targets from `read` and `pages`, excluding `exclusions`.
    pub fn begin_drag(
        &mut self,
        read: &DocumentRead,
        pages: &[Page],
        mm_per_px: f64,
        exclusions: SnapExclusions<'_>,
    ) {
        let settings = SnapSettings::from_document(self.stored, mm_per_px);
        self.drag = Some(SnappingDrag::new(read, pages, settings, exclusions));
    }

    /// The last resolved result, for drawing the marker.
    pub fn last_result(&self) -> Option<&SnapResult> {
        self.drag.as_ref().map(SnappingDrag::last_result)
    }

    pub(super) fn set_nodes_only(&mut self, enabled: bool) {
        if let Some(drag) = &mut self.drag {
            drag.nodes_only = enabled;
        }
    }

    /// Ends the drag (release, Escape, tool change).
    pub fn end_drag(&mut self) {
        self.drag = None;
    }

    /// Whether Ctrl+Alt is held: snapping is off while the combination is
    /// held, whatever the stored switch says.
    pub fn suspended(ctrl: bool, alt: bool) -> bool {
        ctrl && alt
    }
}

mod records;
pub use records::{SnappingSettingsRecord, SnappingToolbarRecord};

impl super::Session {
    /// Starts a snapping drag for the current selection, gathering targets from the current
    /// document read. Called when a move or resize drag begins.
    pub(super) fn begin_snapping_drag_for_selection(&mut self) {
        use curvyo_ui_core::SnapExclusions;
        let read = self.object_cache.read(&self.document, true, &self.fonts);
        let pages = self.document.pages();
        let mm_per_px = 1.0 / self.viewport.view().scale();
        let mut excluded_boxes = std::collections::HashSet::new();
        let mut excluded_nodes = std::collections::HashSet::new();
        for id in self.selection.ids() {
            excluded_boxes.insert(*id);
            excluded_nodes.insert(*id);
        }
        let exclusions = SnapExclusions {
            boxes: Some(&excluded_boxes),
            nodes: Some(&excluded_nodes),
            anchor: None,
            anchors: None,
        };
        self.snapping.update_stored(self.document.snapping());
        self.snapping
            .begin_drag(&read, &pages, mm_per_px, exclusions);
        self.snapping.version = Some(self.document.version());
    }

    /// Starts a snapping drag for shape creation, gathering targets from the current
    /// document read. No boxes or nodes are excluded.
    pub(super) fn begin_snapping_drag_for_shape(&mut self) {
        use curvyo_ui_core::SnapExclusions;
        let read = self.object_cache.read(&self.document, true, &self.fonts);
        let pages = self.document.pages();
        let mm_per_px = 1.0 / self.viewport.view().scale();
        let exclusions = SnapExclusions {
            boxes: None,
            nodes: None,
            anchor: None,
            anchors: None,
        };
        self.snapping.update_stored(self.document.snapping());
        self.snapping
            .begin_drag(&read, &pages, mm_per_px, exclusions);
        self.snapping.version = Some(self.document.version());
    }

    /// Resolves snapping for a move drag with the given probe (the raw position and box features).
    /// Returns the snap result, which includes the offset to apply.
    pub(super) fn resolve_snapping(&mut self, probe: &SnapProbe) -> SnapResult {
        self.resolve_snapping_direction(probe, None)
    }

    pub(super) fn resolve_snapping_direction(
        &mut self,
        probe: &curvyo_ui_core::SnapProbe,
        direction: Option<curvyo_document_core::Vec2>,
    ) -> curvyo_ui_core::SnapResult {
        self.snapping.update_stored(self.document.snapping());
        let version = self.document.version();
        if self.snapping.drag.is_some() && self.snapping.version.as_ref() != Some(&version) {
            let read = self.document_read();
            let pages = self.document.pages();
            if let Some(drag) = &mut self.snapping.drag {
                let all = SnapSettings {
                    objects: true,
                    guides: true,
                    nodes: true,
                    page: true,
                    ..drag.settings
                };
                drag.targets = SnapTargets::gather_pages(
                    &read.nodes,
                    &pages,
                    &all,
                    SnapExclusions {
                        boxes: drag.boxes.as_ref(),
                        nodes: drag.nodes.as_ref(),
                        anchor: None,
                        anchors: Some(&drag.anchors),
                    },
                );
                drag.state.clear();
            }
            self.snapping.version = Some(version);
        }
        let mm_per_px = 1.0 / self.view().scale();
        if let Some(drag) = &mut self.snapping.drag {
            drag.settings = SnapSettings::from_document(self.document.snapping(), mm_per_px);
            if drag.nodes_only {
                drag.settings.objects = false;
                drag.settings.guides = false;
                drag.settings.page = false;
            }
            if self.held.ctrl && self.held.alt {
                drag.settings.enabled = false;
            }
        }
        if let Some(drag) = &mut self.snapping.drag {
            let result = if let Some(direction) = direction {
                curvyo_ui_core::resolve_probe_along(
                    &drag.settings,
                    &drag.targets,
                    probe,
                    direction,
                    &mut drag.state,
                )
            } else {
                curvyo_ui_core::resolve(&drag.settings, &drag.targets, probe, &mut drag.state)
            };
            drag.last_result = result.clone();
            result
        } else {
            SnapResult::default()
        }
    }

    /// Ends the snapping drag (release, Escape, tool change).
    pub(super) fn end_snapping_drag(&mut self) {
        self.snapping.end_drag();
    }

    /// Whether snapping is suspended due to Ctrl+Alt.
    pub(super) fn snapping_suspended(&self) -> bool {
        SnappingState::suspended(self.held.ctrl, self.held.alt)
    }

    /// The last snap result, for drawing the marker.
    pub(super) fn snap_result(&self) -> Option<&curvyo_ui_core::SnapResult> {
        self.snapping.last_result()
    }

    /// The snapping settings as a record.
    #[must_use]
    pub fn snapping_settings(&self) -> SnappingSettingsRecord {
        SnappingSettingsRecord::new(self.document.snapping())
    }

    /// The snapping toolbar state for the panel header.
    #[must_use]
    pub fn snapping_toolbar(&self) -> SnappingToolbarRecord {
        let stored = self.document.snapping();
        let suspended = self.snapping_suspended();
        let has_drag = self.button_down || self.operation_in_flight();
        SnappingToolbarRecord::new(stored, suspended, has_drag)
    }

    /// Toggles snapping enabled state. Returns the new state.
    pub fn toggle_snapping(&mut self) -> bool {
        let mut settings = self.document.snapping();
        settings.enabled = !settings.enabled;
        self.set_snapping(settings);
        self.document.snapping().enabled
    }

    /// Sets snapping target kinds. Returns whether anything changed.
    #[allow(clippy::fn_params_excessive_bools)] // Host API exposes independent target switches.
    pub fn set_snapping_targets(
        &mut self,
        objects: bool,
        guides: bool,
        nodes: bool,
        page: bool,
    ) -> bool {
        let mut settings = self.document.snapping();
        if settings.objects == objects
            && settings.guides == guides
            && settings.nodes == nodes
            && settings.page == page
        {
            return false;
        }
        settings.objects = objects;
        settings.guides = guides;
        settings.nodes = nodes;
        settings.page = page;
        self.set_snapping(settings)
    }
}
impl super::Session {
    pub(super) fn snap_marker_list(&self) -> curvyo_render_core::DrawList {
        use curvyo_document_core::Vec2;
        use curvyo_render_core::SnapMarkerSegment;
        use curvyo_ui_core::MarkerLine;
        if !self.document.snapping().enabled || self.snapping_suspended() {
            return curvyo_render_core::DrawList::default();
        }
        let Some(result) = self.snap_result() else {
            return curvyo_render_core::DrawList::default();
        };
        let extra = 8.0 / self.view().scale();
        let segments: Vec<_> = result
            .marker
            .lines
            .iter()
            .map(|line| match *line {
                MarkerLine::Vertical { x, y0, y1 } => SnapMarkerSegment {
                    start: Point::new(x, y0 - extra),
                    end: Point::new(x, y1 + extra),
                    handle_out: Vec2::ZERO,
                    handle_in: Vec2::ZERO,
                },
                MarkerLine::Horizontal { y, x0, x1 } => SnapMarkerSegment {
                    start: Point::new(x0 - extra, y),
                    end: Point::new(x1 + extra, y),
                    handle_out: Vec2::ZERO,
                    handle_in: Vec2::ZERO,
                },
                MarkerLine::Curve(c) => SnapMarkerSegment {
                    start: c.start,
                    end: c.end,
                    handle_out: c.handle_out,
                    handle_in: c.handle_in,
                },
            })
            .collect();
        curvyo_render_core::build_snap_marker(&segments, result.marker.cross, self.view())
    }
}

#[cfg(test)]
mod tests;
