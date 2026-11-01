//! Host-facing snapshots of document snapping settings.

/// The host spelling of a grid snap point.
const fn grid_point_name(point: curvyo_document_core::GridSnapPoint) -> &'static str {
    match point {
        curvyo_document_core::GridSnapPoint::Centre => "centre",
        curvyo_document_core::GridSnapPoint::Box => "box",
        curvyo_document_core::GridSnapPoint::Nodes => "nodes",
    }
}
/// Snapping settings as a flat record the host reads.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // Serialized independent switches.
pub struct SnappingSettingsRecord {
    /// Whether snapping is enabled.
    pub enabled: bool,
    /// Whether objects snap to other objects.
    pub objects: bool,
    /// Whether objects snap to guides.
    pub guides: bool,
    /// Whether objects snap to nodes (anchors).
    pub nodes: bool,
    /// Whether objects snap to page fractions.
    pub page: bool,
    /// Whether shown page grids pull (`specs/0090-grid` criterion 10).
    pub grid: bool,
    /// Which part of a moved selection snaps to a grid: `"centre"`, `"box"` or `"nodes"`.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub grid_point: String,
    /// Whether the horizontal coordinate may snap.
    pub x: bool,
    /// Whether the vertical coordinate may snap.
    pub y: bool,
    /// The capture distance in screen pixels.
    pub capture_px: f64,
    /// The release distance in screen pixels.
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
            grid: snapping.grid,
            grid_point: grid_point_name(snapping.grid_point).to_owned(),
            x: snapping.x,
            y: snapping.y,
            capture_px: snapping.capture_px,
            release_px: snapping.release_px,
        }
    }
}

/// Snapping toolbar state for the panel header.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // Serialized independent switches.
pub struct SnappingToolbarRecord {
    /// Whether snapping is enabled.
    pub enabled: bool,
    /// Whether snapping is suspended due to Ctrl+Alt.
    pub suspended: bool,
    /// Whether a snapping drag is in flight.
    pub has_drag: bool,
    /// Whether objects snap to other objects.
    pub objects: bool,
    /// Whether objects snap to guides.
    pub guides: bool,
    /// Whether objects snap to nodes (anchors).
    pub nodes: bool,
    /// Whether objects snap to page fractions.
    pub page: bool,
    /// Whether shown page grids pull (`specs/0090-grid` criterion 10).
    pub grid: bool,
    /// Which part of a moved selection snaps to a grid: `"centre"`, `"box"` or `"nodes"`.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub grid_point: String,
    /// Whether the horizontal coordinate may snap.
    pub x: bool,
    /// Whether the vertical coordinate may snap.
    pub y: bool,
    /// The capture distance in screen pixels.
    pub capture_px: f64,
    /// The release distance in screen pixels.
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
            grid: stored.grid,
            grid_point: grid_point_name(stored.grid_point).to_owned(),
            x: stored.x,
            y: stored.y,
            capture_px: stored.capture_px,
            release_px: stored.release_px,
        }
    }
}
