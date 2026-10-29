//! Snapping state for a drag in flight (`specs/0060-snapping`, `adrs.md`
//! decisions 3 to 5): reads the document's settings, gathers the target list
//! once at the press, holds the ephemeral result for drawing, and drops it on
//! release/Escape/tool change.

use std::collections::HashSet;

use curvyo_document_core::{Document, DocumentSize, DocumentSnapping, NodeId, Point};
use curvyo_ui_core::{
    SnapExclusions, SnapProbe, SnapResult, SnapSettings, SnapState, SnapTargets,
};

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
}

impl SnappingDrag {
    /// Builds the target list from `read` and `size` with the given settings,
    /// excluding `excluded` nodes and anchors.
    pub fn new(
        read: &DocumentRead,
        size: DocumentSize,
        settings: SnapSettings,
        exclusions: SnapExclusions<'_>,
    ) -> Self {
        let targets = SnapTargets::gather(&read.nodes, size, &settings, exclusions);
        Self {
            settings,
            targets,
            state: SnapState::default(),
            last_result: SnapResult::default(),
        }
    }

    /// Whether any target kind is enabled.
    pub fn any_target(&self) -> bool {
        self.settings.any_target()
    }

    /// Resolves one frame with `probe`; updates `state` and returns the result.
    /// The result is stored for drawing.
    pub fn resolve(&mut self, probe: &SnapProbe) -> SnapResult {
        let result = curvyo_ui_core::resolve(
            &self.settings,
            &self.targets,
            probe,
            &mut self.state,
        );
        self.last_result = result.clone();
        result
    }

    /// The last resolved result, for drawing the marker.
    pub fn last_result(&self) -> &SnapResult {
        &self.last_result
    }

    /// Adds the anchors of `path` as node targets, skipping `skip`: the Node
    /// tool's own path.
    pub fn add_path_nodes(
        &mut self,
        path: &curvyo_document_core::PathSnapshot,
        skip: Option<curvyo_document_core::AnchorId>,
    ) {
        self.targets.add_path_nodes(path, skip);
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
}

impl SnappingState {
    /// A fresh state with the defaults.
    pub fn new(stored: DocumentSnapping) -> Self {
        Self {
            stored,
            drag: None,
        }
    }

    /// The stored settings.
    pub fn stored(&self) -> DocumentSnapping {
        self.stored
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
    /// gathering targets from `read` and `size`, excluding `exclusions`.
    pub fn begin_drag(
        &mut self,
        read: &DocumentRead,
        size: DocumentSize,
        mm_per_px: f64,
        exclusions: SnapExclusions<'_>,
    ) {
        let settings = SnapSettings::from_document(self.stored, mm_per_px);
        self.drag = Some(SnappingDrag::new(read, size, settings, exclusions));
    }

    /// Adds the Node tool's own path anchors to the current drag.
    pub fn add_path_nodes(
        &mut self,
        path: &curvyo_document_core::PathSnapshot,
        skip: Option<curvyo_document_core::AnchorId>,
    ) {
        if let Some(drag) = &mut self.drag {
            drag.add_path_nodes(path, skip);
        }
    }

    /// Resolves the current drag with `probe`. Returns the result.
    pub fn resolve(&mut self, probe: &SnapProbe) -> SnapResult {
        match &mut self.drag {
            Some(drag) => drag.resolve(probe),
            None => SnapResult::default(),
        }
    }

    /// The last resolved result, for drawing the marker.
    pub fn last_result(&self) -> Option<&SnapResult> {
        self.drag.as_ref().map(|drag| drag.last_result())
    }

    /// Ends the drag (release, Escape, tool change).
    pub fn end_drag(&mut self) {
        self.drag = None;
    }

    /// Whether Ctrl+Alt is held: snapping is off while the combination is
    /// held, whatever the stored switch says.
    pub fn suspended(&self, ctrl: bool, alt: bool) -> bool {
        ctrl && alt
    }
}

/// Snapping settings as a flat record the host reads.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
#[derive(Debug, Clone, PartialEq)]
pub struct SnappingSettingsRecord {
    /// Whether snapping is enabled.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub enabled: bool,
    /// Whether objects snap to other objects.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub objects: bool,
    /// Whether objects snap to guides.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub guides: bool,
    /// Whether objects snap to nodes (anchors).
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub nodes: bool,
    /// Whether objects snap to page fractions.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub page: bool,
    /// The capture distance in screen pixels.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub capture_px: f64,
    /// The release distance in screen pixels.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub release_px: f64,
}

impl SnappingSettingsRecord {
    /// The record of `snapping`.
    #[must_use]
    pub fn new(snapping: curvyo_document_core::DocumentSnapping) -> Self {
        Self {
            enabled: snapping.enabled,
            objects: snapping.objects,
            guides: snapping.guides,
            nodes: snapping.nodes,
            page: snapping.page,
            capture_px: snapping.capture_px,
            release_px: snapping.release_px,
        }
    }
}

/// Snapping toolbar state for the panel header.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
#[derive(Debug, Clone, PartialEq)]
pub struct SnappingToolbarRecord {
    /// Whether snapping is enabled.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub enabled: bool,
    /// Whether snapping is suspended due to Ctrl+Alt.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub suspended: bool,
    /// Whether a snapping drag is in flight.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub has_drag: bool,
    /// Whether objects snap to other objects.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub objects: bool,
    /// Whether objects snap to guides.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub guides: bool,
    /// Whether objects snap to nodes (anchors).
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub nodes: bool,
    /// Whether objects snap to page fractions.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub page: bool,
    /// The capture distance in screen pixels.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub capture_px: f64,
    /// The release distance in screen pixels.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub release_px: f64,
}

impl SnappingToolbarRecord {
    /// The record for the current session.
    #[must_use]
    pub fn new(
        stored: curvyo_document_core::DocumentSnapping,
        suspended: bool,
        has_drag: bool,
    ) -> Self {
        Self {
            enabled: stored.enabled,
            suspended,
            has_drag,
            objects: stored.objects,
            guides: stored.guides,
            nodes: stored.nodes,
            page: stored.page,
            capture_px: stored.capture_px,
            release_px: stored.release_px,
        }
    }
}

impl super::Session {
    /// Starts a snapping drag for the current selection, gathering targets from the current
    /// document read. Called when a move or resize drag begins.
    pub(super) fn begin_snapping_drag_for_selection(&mut self) {
        use curvyo_ui_core::SnapExclusions;
        let read = self.object_cache.read(&self.document, true, &self.fonts);
        let size = self.document.size();
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
        };
        self.snapping.begin_drag(&read, size, mm_per_px, exclusions);
    }

    /// Starts a snapping drag for shape creation, gathering targets from the current
    /// document read. No boxes or nodes are excluded.
    pub(super) fn begin_snapping_drag_for_shape(&mut self) {
        use curvyo_ui_core::SnapExclusions;
        let read = self.object_cache.read(&self.document, true, &self.fonts);
        let size = self.document.size();
        let mm_per_px = 1.0 / self.viewport.view().scale();
        let exclusions = SnapExclusions {
            boxes: None,
            nodes: None,
            anchor: None,
        };
        self.snapping.begin_drag(&read, size, mm_per_px, exclusions);
    }

    /// Adds the Node tool's own path anchors to the current snapping drag.
    pub(super) fn add_snapping_path_nodes(
        &mut self,
        path: &curvyo_document_core::PathSnapshot,
        skip: Option<curvyo_document_core::AnchorId>,
    ) {
        self.snapping.add_path_nodes(path, skip);
    }

    /// Resolves snapping for a move drag with the given probe (the raw position and box features).
    /// Returns the snap result, which includes the offset to apply.
    pub(super) fn resolve_snapping(&mut self, probe: &curvyo_ui_core::SnapProbe) -> curvyo_ui_core::SnapResult {
        self.snapping.resolve(probe)
    }

    /// Ends the snapping drag (release, Escape, tool change).
    pub(super) fn end_snapping_drag(&mut self) {
        self.snapping.end_drag();
    }

    /// Whether snapping is suspended due to Ctrl+Alt.
    pub(super) fn snapping_suspended(&self) -> bool {
        self.snapping.suspended(self.held.ctrl, self.held.alt)
    }

    /// The last snap result, for drawing the marker.
    pub(super) fn snap_result(&self) -> Option<&curvyo_ui_core::SnapResult> {
        self.snapping.last_result()
    }

    /// The snapping settings as a record.
    #[must_use]
    pub fn snapping_settings(&self) -> SnappingSettingsRecord {
        SnappingSettingsRecord::new(self.snapping.stored())
    }

    /// The snapping toolbar state for the panel header.
    #[must_use]
    pub fn snapping_toolbar(&self) -> SnappingToolbarRecord {
        let stored = self.snapping.stored();
        let suspended = self.snapping_suspended();
        let has_drag = self.snapping.drag_in_flight();
        SnappingToolbarRecord::new(stored, suspended, has_drag)
    }

    /// Toggles snapping enabled state. Returns the new state.
    pub fn toggle_snapping(&mut self) -> bool {
        let mut settings = self.snapping.stored();
        settings.enabled = !settings.enabled;
        self.set_snapping(settings);
        settings.enabled
    }

    /// Sets snapping target kinds. Returns whether anything changed.
    pub fn set_snapping_targets(
        &mut self,
        objects: bool,
        guides: bool,
        nodes: bool,
        page: bool,
    ) -> bool {
        let mut settings = self.snapping.stored();
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