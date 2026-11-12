//! What the mask of the Trace tool draws over the picture (`specs/0114-image-trace-mask`
//! Behaviour 2 and 4): the active part's regions, the one being drawn or dragged in its place,
//! and the selected rectangle's eight handles. Session overlay, never a document node.

use curvyo_document_core::{TraceMaskRegion, TraceRegionKind, ViewTransform};
use curvyo_render_core::{DrawList, TraceMaskOutline, TraceMaskOverlay, build_trace_mask_overlay};
use curvyo_ui_core::{TraceRegionHandle, region_outline_mm};

use super::trace_mask_input::drag_rect;
use super::trace_mask_tool::MaskDragKind;
use super::{Session, Tool};

impl Session {
    /// The mask overlay of the Trace tool; empty in any other tool or without one traced image.
    pub(super) fn trace_mask_draw_list(&self, view: ViewTransform) -> DrawList {
        if self.tool != Tool::Trace {
            return DrawList::default();
        }
        let (Some(image), Some(part)) = (self.trace_mask_image(), self.trace_active_mask_part())
        else {
            return DrawList::default();
        };
        let map = image.placement;
        let mask = &self.trace.mask;
        let ghost = mask.drag.as_ref().and_then(|drag| {
            let rect = drag_rect(drag, self.held.shift, self.held.ctrl)?;
            let region = match &drag.kind {
                MaskDragKind::Draw => None,
                MaskDragKind::Move { region, .. } | MaskDragKind::Resize { region, .. } => {
                    Some(region.clone())
                }
            };
            Some((region, rect.to_unit(drag.width, drag.height)))
        });
        let mut overlay = TraceMaskOverlay::default();
        let mut selected_corners = None;
        for region in &part.regions {
            let corners = match &ghost {
                Some((Some(id), [a, b])) if *id == region.id => {
                    let moved = TraceMaskRegion {
                        points: vec![*a, *b],
                        ..region.clone()
                    };
                    region_outline_mm(&moved, map)
                }
                _ => region_outline_mm(region, map),
            };
            let selected = mask.selected.as_deref() == Some(region.id.as_str());
            if selected && region.kind == TraceRegionKind::Rect {
                selected_corners = Some(corners.clone());
            }
            overlay.outlines.push(TraceMaskOutline {
                corners,
                subtract: region.subtract,
                selected,
            });
        }
        if let Some((None, [a, b])) = &ghost {
            let drawn = TraceMaskRegion {
                id: String::new(),
                order: 0,
                kind: TraceRegionKind::Rect,
                subtract: mask.subtract,
                points: vec![*a, *b],
            };
            overlay.outlines.push(TraceMaskOutline {
                corners: region_outline_mm(&drawn, map),
                subtract: mask.subtract,
                selected: false,
            });
        }
        if let Some(corners) = selected_corners {
            // The corners run top-left, top-right, bottom-right, bottom-left; the edge handles sit
            // between them.
            let mid = |i: usize, j: usize| {
                curvyo_document_core::Point::new(
                    f64::midpoint(corners[i].x, corners[j].x),
                    f64::midpoint(corners[i].y, corners[j].y),
                )
            };
            overlay.handles = TraceRegionHandle::ALL
                .iter()
                .map(|handle| match handle {
                    TraceRegionHandle::NW => corners[0],
                    TraceRegionHandle::N => mid(0, 1),
                    TraceRegionHandle::NE => corners[1],
                    TraceRegionHandle::E => mid(1, 2),
                    TraceRegionHandle::SE => corners[2],
                    TraceRegionHandle::S => mid(2, 3),
                    TraceRegionHandle::SW => corners[3],
                    TraceRegionHandle::W => mid(3, 0),
                })
                .collect();
        }
        build_trace_mask_overlay(view, &overlay)
    }
}
