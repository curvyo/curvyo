//! The Trace tool's mask state and the parts of the traced image as the document stores them
//! (`specs/0114-image-trace-mask` slice 1, ADR 0061 §9): the pressed region tool, Subtract, the
//! selected region, a drag in progress and the bar's settings are tool state; the parts, their
//! modes and their regions are read from the document.

use std::cell::RefCell;

use curvyo_document_core::{
    ImageSnapshot, ImplicitTracePart, NodeId, TraceMaskRegion, TracePartRef, TracePartSettings,
};
use curvyo_ui_core::{PxRect, TraceRegionHandle};

use super::{Session, Tool};

/// The region tool pressed in the Mask group.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TraceRegionTool {
    /// None: a press selects, moves and resizes regions.
    #[default]
    None,
    /// Rectangle: a drag draws a rectangle.
    Rect,
}

/// A drag of the mask in picture pixels of the working image `width` x `height`.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct MaskDrag {
    pub(super) kind: MaskDragKind,
    pub(super) from: kurbo::Point,
    pub(super) to: kurbo::Point,
    pub(super) width: u32,
    pub(super) height: u32,
}

/// What a mask drag does.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum MaskDragKind {
    /// Draws a rectangle.
    Draw,
    /// Moves the rectangle `region`.
    Move { region: String, rect: PxRect },
    /// Drags a handle of the rectangle `region`.
    Resize {
        region: String,
        rect: PxRect,
        handle: TraceRegionHandle,
    },
}

/// The mask state of the Trace tool; leaving the tool drops it.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct TraceMaskTool {
    pub(super) region_tool: TraceRegionTool,
    pub(super) subtract: bool,
    /// The selected region of the active part, by id.
    pub(super) selected: Option<String>,
    pub(super) drag: Option<MaskDrag>,
    /// The bar's settings of the active part: what the first gesture and Add part store.
    pub(super) settings: TracePartSettings,
    /// The chip ids of the stored parts of `chips_for`.
    chips: RefCell<ChipIds>,
}

/// Small stable numbers for the stored part ids of one image: part `0` and the implicit part
/// are chip 0, so the bar's settings stay with the part when the first gesture stores it.
#[derive(Debug, Clone, Default, PartialEq)]
struct ChipIds {
    image: Option<NodeId>,
    ids: Vec<(String, u32)>,
    next: u32,
}

impl ChipIds {
    fn chip(&mut self, image: NodeId, part: &str) -> u32 {
        if self.image != Some(image) {
            *self = Self {
                image: Some(image),
                ids: Vec::new(),
                next: 1,
            };
        }
        if part == curvyo_document_core::FIRST_TRACE_PART {
            return 0;
        }
        if let Some((_, chip)) = self.ids.iter().find(|(id, _)| id == part) {
            return *chip;
        }
        let chip = self.next;
        self.next += 1;
        self.ids.push((part.to_owned(), chip));
        chip
    }
}

/// One part of the traced image as the tool reads it.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct MaskPart {
    /// The stored part's id; `None` for the implicit part of an untouched image.
    pub(super) stored: Option<String>,
    /// Its chip id for the host.
    pub(super) chip: u32,
    pub(super) outside: bool,
    /// Its regions in order.
    pub(super) regions: Vec<TraceMaskRegion>,
}

impl Session {
    /// The one traced image, when the selection is one image (with mask paths).
    pub(super) fn trace_mask_image(&self) -> Option<ImageSnapshot> {
        let (_, images) = self.trace_images();
        match images.as_slice() {
            [image] => Some(image.clone()),
            _ => None,
        }
    }

    /// The parts of the traced image in order: the stored ones, or the implicit first part.
    pub(super) fn trace_mask_parts(&self) -> Vec<MaskPart> {
        let implicit = || {
            vec![MaskPart {
                stored: None,
                chip: 0,
                outside: self.trace.mask_outside,
                regions: Vec::new(),
            }]
        };
        let Some(image) = self.trace_mask_image() else {
            return implicit();
        };
        let mask = self.document.trace_mask(image.id).unwrap_or_default();
        if mask.is_empty() {
            return implicit();
        }
        let mut chips = self.trace.mask.chips.borrow_mut();
        mask.parts
            .into_iter()
            .map(|part| MaskPart {
                chip: chips.chip(image.id, &part.id),
                stored: Some(part.id),
                outside: part.outside,
                regions: part.regions,
            })
            .collect()
    }

    /// The active part, as read.
    pub(super) fn trace_active_mask_part(&self) -> Option<MaskPart> {
        self.trace_mask_parts()
            .into_iter()
            .nth(self.trace_active_part())
    }

    /// The implicit part as the tool holds it: what a first gesture stores as part `0`.
    pub(super) fn trace_implicit_part(&self) -> ImplicitTracePart {
        ImplicitTracePart {
            outside: self.trace.mask_outside,
            settings: self.trace.mask.settings.clone(),
        }
    }

    /// Runs `write` with the active part as a command's target.
    pub(super) fn with_active_part_ref<R>(
        &self,
        write: impl FnOnce(TracePartRef<'_>) -> R,
    ) -> Option<R> {
        let part = self.trace_active_mask_part()?;
        let implicit = self.trace_implicit_part();
        Some(match &part.stored {
            Some(id) => write(TracePartRef::Stored(id)),
            None => write(TracePartRef::Implicit(&implicit)),
        })
    }

    /// Presses a region tool (`none` releases it); disarms Pick mask paths and drops the region
    /// selection.
    pub fn set_trace_region_tool(&mut self, tool: TraceRegionTool) {
        if self.tool != Tool::Trace {
            return;
        }
        if tool != TraceRegionTool::None {
            self.set_trace_mask_pick(false);
            self.trace.mask.selected = None;
        }
        self.trace.mask.region_tool = tool;
        self.trace.mask.drag = None;
    }

    /// The pressed region tool.
    #[must_use]
    pub const fn trace_region_tool(&self) -> TraceRegionTool {
        self.trace.mask.region_tool
    }

    /// Presses or releases Subtract.
    pub fn set_trace_subtract(&mut self, subtract: bool) {
        self.trace.mask.subtract = subtract && self.tool == Tool::Trace;
    }

    /// Whether Subtract is pressed.
    #[must_use]
    pub const fn trace_subtract(&self) -> bool {
        self.trace.mask.subtract
    }

    /// The bar's settings of the active part, tool state (no step): what the first gesture on an
    /// untouched image and Add part store. `json` is an object of ADR 0061 §5's keys; anything
    /// else is ignored.
    pub fn set_trace_tool_settings(&mut self, json: &str) {
        if let Ok(serde_json::Value::Object(map)) = serde_json::from_str(json) {
            self.trace.mask.settings =
                curvyo_ui_core::written_part_settings(&TracePartSettings::new(), &map);
        }
    }

    /// The selected region's id, if any.
    #[must_use]
    pub fn trace_selected_region(&self) -> Option<String> {
        self.trace.mask.selected.clone()
    }
}
