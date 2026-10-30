//! Host-facing snapshots of document snapping settings.
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
            capture_px: stored.capture_px,
            release_px: stored.release_px,
        }
    }
}
