//! The Trace tool's mask state and the parts of the traced image as the document stores them
//! (`specs/0114-image-trace-mask`, ADR 0061 §9): the pressed region tool, Subtract, the selected
//! region, a drag or a lasso in progress and the bar's settings of an untouched image are tool
//! state; the parts, their modes, settings and regions are read from the document.

use std::cell::RefCell;

use curvyo_document_core::{
    ImageSnapshot, ImplicitTracePart, NodeId, TraceMaskRegion, TracePartRef, TracePartSettings,
};
use curvyo_ui_core::{LassoDraft, PxCorner, PxRect, TraceRegionHandle};

use super::{Session, Tool};

/// The region tool pressed in the Mask group.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TraceRegionTool {
    /// None: a press selects, moves and resizes regions.
    #[default]
    None,
    /// Rectangle: a drag draws a rectangle.
    Rect,
    /// Lasso: each click sets a corner of a lasso.
    Lasso,
}

/// A stored region in picture pixels of the working image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RegionPx {
    /// A rectangle.
    Rect(PxRect),
    /// A lasso's corners.
    Lasso(Vec<PxCorner>),
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
    /// Moves the region `region`, which was `shape` at the press.
    Move { region: String, shape: RegionPx },
    /// Drags a handle of the rectangle `region`.
    Resize {
        region: String,
        rect: PxRect,
        handle: TraceRegionHandle,
    },
    /// Drags corner `index` of the lasso `region`, whose corners were `corners` at the press.
    Corner {
        region: String,
        corners: Vec<PxCorner>,
        index: usize,
    },
}

/// A lasso being drawn, with where it is stored when it is finished: the part that was active
/// when its first corner was set, on the image it was drawn on.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct LassoInProgress {
    pub(super) draft: LassoDraft,
    /// The traced picture (an image or an image-root clone).
    pub(super) image: NodeId,
    /// The node that holds the mask (the clone's root image).
    pub(super) owner: NodeId,
    /// The stored part, or `None` for the implicit part of an untouched image.
    pub(super) part: Option<String>,
    pub(super) width: u32,
    pub(super) height: u32,
}

/// The mask state of the Trace tool; leaving the tool drops it.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct TraceMaskTool {
    pub(super) region_tool: TraceRegionTool,
    pub(super) subtract: bool,
    /// The selected region of the active part, by id.
    pub(super) selected: Option<String>,
    /// The selected lasso's last pressed corner.
    pub(super) corner: Option<usize>,
    /// The region of the active part under the pointer (the hover box), by id.
    pub(super) hovered: Option<String>,
    pub(super) drag: Option<MaskDrag>,
    pub(super) lasso: Option<LassoInProgress>,
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
    /// Its stored settings (empty for the implicit part).
    pub(super) settings: TracePartSettings,
    /// Its regions in order.
    pub(super) regions: Vec<TraceMaskRegion>,
}

impl Session {
    /// The traced picture the mask is read for: the one traced image or image-root clone, or the
    /// image a multi-image trace reads now (`TraceParts::for_image`).
    pub(super) fn trace_mask_image(&self) -> Option<ImageSnapshot> {
        let (_, images) = self.trace_images();
        if let Some(id) = self.trace.parts.for_image.get() {
            return images.into_iter().find(|image| image.id == id);
        }
        match images.as_slice() {
            [image] => Some(image.clone()),
            _ => None,
        }
    }

    /// The node that holds the traced picture's mask (an image-root clone's root image).
    pub(super) fn trace_mask_owner_id(&self) -> Option<NodeId> {
        self.trace_mask_image()
            .map(|image| self.trace_mask_owner(image.id))
    }

    /// Whether the traced picture or the image holding its mask is locked, by its own lock or an
    /// ancestor's: the mask is shown and traced but not edited (`0114` adrs.md, UX notes §8).
    #[must_use]
    pub fn trace_mask_locked(&self) -> bool {
        let Some(image) = self.trace_mask_image() else {
            return false;
        };
        let owner = self.trace_mask_owner(image.id);
        let read = self.document_read();
        read.nodes
            .iter()
            .any(|node| (node.id == image.id || node.id == owner) && node.locked)
    }

    /// The parts of the traced image in order: the stored ones, or the implicit first part.
    pub(super) fn trace_mask_parts(&self) -> Vec<MaskPart> {
        let implicit = || {
            vec![MaskPart {
                stored: None,
                chip: 0,
                outside: self.trace.mask_outside,
                settings: TracePartSettings::new(),
                regions: Vec::new(),
            }]
        };
        let Some(owner) = self.trace_mask_owner_id() else {
            return implicit();
        };
        let mask = self.document.trace_mask(owner).unwrap_or_default();
        if mask.is_empty() {
            return implicit();
        }
        let mut chips = self.trace.mask.chips.borrow_mut();
        mask.parts
            .into_iter()
            .map(|part| MaskPart {
                chip: chips.chip(owner, &part.id),
                stored: Some(part.id),
                outside: part.outside,
                settings: part.settings,
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

    /// Runs `write` with `part` (a stored part's id, or the implicit part) as a command's target.
    pub(super) fn with_part_ref<R>(
        &self,
        part: Option<&str>,
        write: impl FnOnce(TracePartRef<'_>) -> R,
    ) -> R {
        let implicit = self.trace_implicit_part();
        match part {
            Some(id) => write(TracePartRef::Stored(id)),
            None => write(TracePartRef::Implicit(&implicit)),
        }
    }

    /// Presses a region tool (`none` releases it): a lasso in progress is closed first and an
    /// armed pipette is disarmed; a selected region stays selected (Escape releases the tool
    /// before it deselects, UX notes §7). Nothing on a locked image.
    pub fn set_trace_region_tool(&mut self, tool: TraceRegionTool) {
        if self.tool != Tool::Trace || (tool != TraceRegionTool::None && self.trace_mask_locked()) {
            return;
        }
        self.close_trace_lasso();
        if tool != TraceRegionTool::None {
            self.set_trace_pick(None);
        }
        self.trace.mask.region_tool = tool;
        self.trace.mask.drag = None;
        self.trace.mask.hovered = None;
    }

    /// Releases the region tool, closing a lasso in progress first (the pipette's arming).
    pub(super) fn release_trace_region_tool(&mut self) {
        self.close_trace_lasso();
        self.trace.mask.region_tool = TraceRegionTool::None;
        self.trace.mask.drag = None;
    }

    /// The pressed region tool.
    #[must_use]
    pub const fn trace_region_tool(&self) -> TraceRegionTool {
        self.trace.mask.region_tool
    }

    /// Presses or releases Subtract; a lasso in progress follows it.
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

    /// The stored settings of part `index` of the traced image, every key valid (ADR 0061 §5):
    /// `None` for the implicit part of an untouched image, whose settings are the bar's.
    #[must_use]
    pub fn trace_part_settings(&self, index: usize) -> Option<TracePartSettings> {
        let part = self.trace_mask_parts().into_iter().nth(index)?;
        part.stored?;
        Some(curvyo_ui_core::read_part_settings(&part.settings))
    }

    /// The end of a settings edit of part `index` (a field typed, a slider released, a preset or
    /// method picked): for a stored part one step "Change trace settings" (ADR 0061 §6), the
    /// unknown stored keys kept; for the implicit part tool state. Whether a step was written.
    pub fn set_trace_part_settings(&mut self, index: usize, json: &str) -> bool {
        let Ok(serde_json::Value::Object(map)) = serde_json::from_str(json) else {
            return false;
        };
        let Some(part) = self.trace_mask_parts().into_iter().nth(index) else {
            return false;
        };
        let (Some(id), Some(owner)) = (part.stored, self.trace_mask_owner_id()) else {
            if index == self.trace_active_part() {
                self.trace.mask.settings =
                    curvyo_ui_core::written_part_settings(&TracePartSettings::new(), &map);
            }
            return false;
        };
        if self.tool != Tool::Trace || self.trace_mask_locked() {
            return false;
        }
        let written = curvyo_ui_core::written_part_settings(&part.settings, &map);
        self.recording(|this| {
            this.document
                .set_trace_part_settings(owner, &id, &written)
                .unwrap_or(false)
        })
    }

    /// Clear mask (criterion 20): the active part's regions go in one step "Clear mask", a lasso
    /// in progress with them; the part, its mode and its settings stay.
    pub fn clear_trace_mask(&mut self) {
        if self.tool != Tool::Trace || self.trace_mask_locked() {
            return;
        }
        self.trace.mask.lasso = None;
        let stored = self.trace_active_mask_part().and_then(|part| part.stored);
        if let (Some(part), Some(owner)) = (stored, self.trace_mask_owner_id()) {
            self.recording(|this| {
                this.document
                    .clear_trace_mask(owner, &part)
                    .unwrap_or(false)
            });
        }
        self.trace.mask.selected = None;
        self.trace.mask.corner = None;
    }

    /// The selected region's id, if any.
    #[must_use]
    pub fn trace_selected_region(&self) -> Option<String> {
        self.trace.mask.selected.clone()
    }
}
