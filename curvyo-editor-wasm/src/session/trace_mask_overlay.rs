//! What the mask of the Trace tool draws over the picture (`specs/0114-image-trace-mask`
//! Behaviour 2 to 4, UX notes §5): every part's regions (the dragged one at its new geometry with
//! its old outline as the ghost), the rectangle being drawn, the hover box, the selected region's
//! dashed box with a rectangle's handles or a lasso's corners, and the lasso in progress. Session
//! overlay, never a document node.

use curvyo_document_core::{Affine, Point, TraceMaskRegion, UnitPoint, ViewTransform};
use curvyo_render_core::{
    DrawList, TraceLassoDraft, TraceMaskOutline, TraceMaskOverlay, TraceMaskSelection,
    build_trace_mask_overlay,
};
use curvyo_ui_core::{TraceRegionHandle, region_outline_mm, unit_to_mm};

use super::trace_mask_input::drag_shape;
use super::trace_mask_lasso::CLOSE_TARGET_PX;
use super::trace_mask_tool::{MaskDragKind, RegionPx};
use super::{Session, Tool};

/// The corners of the box of unit-square `points` mapped by `map`: the picture frame's
/// parallelogram, top-left first.
fn unit_box(points: &[UnitPoint], map: Affine) -> Option<[Point; 4]> {
    let (mut u0, mut v0, mut u1, mut v1) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for p in points {
        u0 = u0.min(p.u);
        v0 = v0.min(p.v);
        u1 = u1.max(p.u);
        v1 = v1.max(p.v);
    }
    u0.is_finite().then(|| {
        [
            unit_to_mm(map, UnitPoint::new(u0, v0)),
            unit_to_mm(map, UnitPoint::new(u1, v0)),
            unit_to_mm(map, UnitPoint::new(u1, v1)),
            unit_to_mm(map, UnitPoint::new(u0, v1)),
        ]
    })
}

/// The eight handles of a rectangle whose corners run top-left, top-right, bottom-right,
/// bottom-left, in [`TraceRegionHandle::ALL`]'s order.
fn rect_handles(corners: &[Point]) -> Vec<Point> {
    let mid = |i: usize, j: usize| {
        Point::new(
            f64::midpoint(corners[i].x, corners[j].x),
            f64::midpoint(corners[i].y, corners[j].y),
        )
    };
    TraceRegionHandle::ALL
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
        .collect()
}

impl Session {
    /// The live readout of a rectangle being drawn (UX notes §5): its size along the picture's
    /// sides, "42.3 × 37.0 mm", by the pointer.
    pub(super) fn trace_mask_readout(&self) -> Option<super::shapes::LiveReadout> {
        let drag = self.trace.mask.drag.as_ref()?;
        if !matches!(drag.kind, MaskDragKind::Draw) {
            return None;
        }
        let image = self.trace_mask_image()?;
        let shape = drag_shape(drag, self.held.shift, self.held.ctrl)?;
        let corners = unit_box(&shape.to_unit(drag.width, drag.height), image.placement)?;
        let length = |a: Point, b: Point| a.vector_to(b).length();
        Some(super::shapes::LiveReadout {
            text: format!(
                "{:.1} × {:.1} mm",
                length(corners[0], corners[1]),
                length(corners[0], corners[3])
            ),
            anchor: self.pointer_position?,
        })
    }

    /// The mask overlay of the Trace tool; empty in any other tool or without one traced image.
    pub(super) fn trace_mask_draw_list(&self, view: ViewTransform) -> DrawList {
        if self.tool != Tool::Trace {
            return DrawList::default();
        }
        let Some(image) = self.trace_mask_image() else {
            return DrawList::default();
        };
        let map = image.placement;
        let mask = &self.trace.mask;
        let px = 1.0 / view.scale();
        // The dragged region's id and its points now.
        let moved = mask.drag.as_ref().and_then(|drag| {
            let shape = drag_shape(drag, self.held.shift, self.held.ctrl)?;
            let region = match &drag.kind {
                MaskDragKind::Draw => None,
                MaskDragKind::Move { region, .. }
                | MaskDragKind::Resize { region, .. }
                | MaskDragKind::Corner { region, .. } => Some(region.clone()),
            };
            Some((
                region,
                shape.to_unit(drag.width, drag.height),
                matches!(shape, RegionPx::Rect(_)),
            ))
        });
        let mut overlay = TraceMaskOverlay::default();
        let active = self.trace_active_part();
        let mut selected: Option<TraceMaskRegion> = None;
        for (index, part) in self.trace_mask_parts().into_iter().enumerate() {
            for region in part.regions {
                let now = match &moved {
                    Some((Some(id), points, _)) if *id == region.id => {
                        overlay.ghost = Some(region_outline_mm(&region, map));
                        TraceMaskRegion {
                            points: points.clone(),
                            ..region.clone()
                        }
                    }
                    _ => region.clone(),
                };
                if index == active {
                    if mask.selected.as_deref() == Some(now.id.as_str()) {
                        selected = Some(now.clone());
                    } else if mask.drag.is_none()
                        && mask.hovered.as_deref() == Some(now.id.as_str())
                    {
                        overlay.hover = unit_box(&now.corners(), map);
                    }
                }
                overlay.outlines.push(TraceMaskOutline {
                    corners: region_outline_mm(&now, map),
                    subtract: now.subtract,
                });
            }
        }
        if let Some((None, points, true)) = &moved {
            overlay.drawing = Some(TraceMaskOutline {
                corners: unit_box(points, map)
                    .map(|b| b.to_vec())
                    .unwrap_or_default(),
                subtract: mask.subtract,
            });
        }
        if let Some(region) = selected {
            overlay.selection = Some(self.trace_selection_look(&region, map, view));
        }
        if let Some(lasso) = &mask.lasso {
            let pixel_map = image.pixel_map(lasso.width, lasso.height);
            let corners: Vec<Point> = lasso
                .draft
                .corners()
                .iter()
                .map(|c| pixel_map.apply_point(Point::new(f64::from(c.x), f64::from(c.y))))
                .collect();
            let pointer = self.pointer_position;
            let closing = corners.len() >= 3
                && pointer
                    .is_some_and(|p| corners[0].vector_to(p).length() <= CLOSE_TARGET_PX * px);
            overlay.lasso = Some(TraceLassoDraft {
                corners,
                subtract: mask.subtract,
                pointer,
                closing,
            });
        }
        build_trace_mask_overlay(view, &overlay)
    }

    /// The selected region's decoration: its dashed box, a rectangle's handles (edge handles only
    /// while the shorter side on screen is 24 px or more), a lasso's corners.
    fn trace_selection_look(
        &self,
        region: &TraceMaskRegion,
        map: Affine,
        view: ViewTransform,
    ) -> TraceMaskSelection {
        let outline = region_outline_mm(region, map);
        let frame = unit_box(&region.corners(), map).unwrap_or([Point::new(0.0, 0.0); 4]);
        let mut look = TraceMaskSelection {
            frame,
            handles: Vec::new(),
            dragged_handle: None,
            corners: Vec::new(),
            hovered_corner: None,
            selected_corner: None,
        };
        if region.kind == curvyo_document_core::TraceRegionKind::Rect && outline.len() == 4 {
            let side = |a: Point, b: Point| a.vector_to(b).length() * view.scale();
            let short = side(outline[0], outline[1]).min(side(outline[1], outline[2]));
            let dragged = match &self.trace.mask.drag {
                Some(drag) => match &drag.kind {
                    MaskDragKind::Resize { handle, .. } => Some(*handle),
                    _ => None,
                },
                None => None,
            };
            for (handle, at) in TraceRegionHandle::ALL.iter().zip(rect_handles(&outline)) {
                if handle.is_corner() || short >= 24.0 {
                    if dragged == Some(*handle) {
                        look.dragged_handle = Some(look.handles.len());
                    }
                    look.handles.push(at);
                }
            }
        } else {
            let radius = 16.0 / view.scale();
            look.hovered_corner = self.pointer_position.and_then(|p| {
                outline
                    .iter()
                    .enumerate()
                    .map(|(i, c)| (i, c.vector_to(p).length()))
                    .filter(|(_, d)| *d <= radius)
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|(i, _)| i)
            });
            look.selected_corner = self.trace.mask.corner;
            look.corners = outline;
        }
        look
    }
}
