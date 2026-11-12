//! The Trace tool's pointer and keys on the mask (`specs/0114-image-trace-mask` Behaviour 2,
//! criteria 2, 3 and 17): with Rectangle pressed a drag draws a rectangle; with no region tool a
//! press on a rectangle of the active part selects it and a drag moves it or, on a handle,
//! resizes it; the arrows move the selected region, Delete removes it, Escape cancels a drag,
//! then drops the selection, then releases the tool. Each finished gesture is one step.

use curvyo_document_core::{
    Affine, ImageSnapshot, NewTraceRegion, Point, TraceRegionEdit, TraceRegionKind,
};
use curvyo_ui_core::{
    PxRect, drawn_trace_rect, moved_trace_rect, resized_trace_rect, trace_rect_at,
    trace_rect_handle_at,
};

use super::keys::{EscapeStep, KeyInput, KeyOutcome};
use super::trace_mask_tool::{MaskDrag, MaskDragKind, TraceRegionTool};
use super::{Session, Tool};

/// How far past a rectangle's outline a press still selects it, screen pixels.
const REGION_SLOP_PX: f64 = 4.0;

/// The picture frame a gesture works in: the image, its working image's size, the map from
/// document millimetres to its pixels, and screen pixels per picture pixel.
struct MaskFrame {
    image: ImageSnapshot,
    width: u32,
    height: u32,
    to_px: Affine,
    screen_per_px: f64,
}

fn kurbo_point(p: Point) -> kurbo::Point {
    kurbo::Point::new(p.x, p.y)
}

/// The rectangle a drag leaves with the modifiers `shift` (from the centre) and `ctrl` (a
/// square, or proportions at a corner); `None` for a draw under 2 px.
pub(super) fn drag_rect(drag: &MaskDrag, shift: bool, ctrl: bool) -> Option<PxRect> {
    match &drag.kind {
        MaskDragKind::Draw => drawn_trace_rect(drag.from, drag.to, shift, ctrl),
        MaskDragKind::Move { rect, .. } => {
            let (dx, dy) = (drag.to.x - drag.from.x, drag.to.y - drag.from.y);
            Some(moved_trace_rect(*rect, whole(dx), whole(dy)))
        }
        MaskDragKind::Resize { rect, handle, .. } => {
            Some(resized_trace_rect(*rect, *handle, drag.to, shift, ctrl))
        }
    }
}

/// `v` as whole pixels, held to a range no sum overflows.
fn whole(v: f64) -> i32 {
    // invariant: rounded and clamped into i32's range first; a NaN becomes 0.
    #[allow(clippy::cast_possible_truncation)]
    let r = v.round().clamp(-1.0e9, 1.0e9) as i32;
    r
}

impl Session {
    /// The frame of the one traced image, when its picture decodes.
    fn trace_mask_frame(&mut self) -> Option<MaskFrame> {
        let image = self.trace_mask_image()?;
        let working = self.working_image(&image.asset)?;
        let (width, height) = (working.width, working.height);
        let pixel_map = image.pixel_map(width, height);
        let to_px = pixel_map.inverse()?;
        let screen_per_px = self.view().scale() * pixel_map.determinant().abs().sqrt();
        Some(MaskFrame {
            image,
            width,
            height,
            to_px,
            screen_per_px,
        })
    }

    /// The rectangles of the active part in picture pixels of `frame`, with their ids.
    fn trace_mask_rects(&self, frame: &MaskFrame) -> Vec<(String, PxRect)> {
        self.trace_active_mask_part()
            .map(|part| part.regions)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|region| match (region.kind, region.points.as_slice()) {
                (TraceRegionKind::Rect, &[a, b]) => Some((
                    region.id,
                    PxRect::from_unit([a, b], frame.width, frame.height),
                )),
                _ => None,
            })
            .collect()
    }

    /// A press in the Trace tool that the mask takes; `false` leaves it to the selection.
    pub(super) fn trace_mask_press(&mut self, point: Point) -> bool {
        if self.trace_mask_pick() {
            return false;
        }
        let Some(frame) = self.trace_mask_frame() else {
            return false;
        };
        let p = kurbo_point(frame.to_px.apply_point(point));
        let start = |kind| MaskDrag {
            kind,
            from: p,
            to: p,
            width: frame.width,
            height: frame.height,
        };
        if self.trace.mask.region_tool == TraceRegionTool::Rect {
            self.trace.mask.drag = Some(start(MaskDragKind::Draw));
            return true;
        }
        let rects = self.trace_mask_rects(&frame);
        let scale = frame.screen_per_px;
        let selected = self
            .trace
            .mask
            .selected
            .as_ref()
            .and_then(|id| rects.iter().find(|(r, _)| r == id))
            .cloned();
        if let Some((region, rect)) = selected
            && scale > 0.0
        {
            let r = rect.to_kurbo();
            let screen = kurbo::Rect::new(r.x0 * scale, r.y0 * scale, r.x1 * scale, r.y1 * scale);
            if let Some(handle) = trace_rect_handle_at(screen, (p.to_vec2() * scale).to_point()) {
                self.trace.mask.drag = Some(start(MaskDragKind::Resize {
                    region,
                    rect,
                    handle,
                }));
                return true;
            }
        }
        let boxes: Vec<PxRect> = rects.iter().map(|(_, rect)| *rect).collect();
        let slop = if scale > 0.0 {
            REGION_SLOP_PX / scale
        } else {
            0.0
        };
        if let Some(index) = trace_rect_at(&boxes, p, slop) {
            let (region, rect) = rects[index].clone();
            self.trace.mask.selected = Some(region.clone());
            self.trace.mask.drag = Some(start(MaskDragKind::Move { region, rect }));
            return true;
        }
        self.trace.mask.selected = None;
        false
    }

    /// The pointer moved during a mask drag; `false` without one.
    pub(super) fn trace_mask_drag_move(&mut self, point: Point) -> bool {
        let Some(image) = self.trace_mask_image() else {
            return false;
        };
        let Some(drag) = &mut self.trace.mask.drag else {
            return false;
        };
        if let Some(to_px) = image.pixel_map(drag.width, drag.height).inverse() {
            drag.to = kurbo_point(to_px.apply_point(point));
        }
        true
    }

    /// The release of a mask drag: one step "Draw rectangle", "Move region" or "Resize region",
    /// or nothing for a draw under 2 px or an unchanged region. `false` without a drag.
    pub(super) fn trace_mask_release(&mut self, point: Point, shift: bool, ctrl: bool) -> bool {
        if !self.trace_mask_drag_move(point) {
            return false;
        }
        let Some(drag) = self.trace.mask.drag.take() else {
            return false;
        };
        let Some(image) = self.trace_mask_image() else {
            return true;
        };
        let Some(rect) = drag_rect(&drag, shift, ctrl) else {
            return true;
        };
        let [a, b] = rect.to_unit(drag.width, drag.height);
        match drag.kind {
            MaskDragKind::Draw => {
                let region = NewTraceRegion {
                    kind: TraceRegionKind::Rect,
                    subtract: self.trace.mask.subtract,
                    points: vec![a, b],
                };
                let document = &self.document;
                let _ = self.with_active_part_ref(|part| {
                    document.draw_trace_region(image.id, part, &region)
                });
            }
            MaskDragKind::Move { region, .. } => {
                let _ = self.document.set_trace_region_points(
                    image.id,
                    &region,
                    &[a, b],
                    TraceRegionEdit::Move,
                );
            }
            MaskDragKind::Resize { region, .. } => {
                let _ = self.document.set_trace_region_points(
                    image.id,
                    &region,
                    &[a, b],
                    TraceRegionEdit::Resize,
                );
            }
        }
        true
    }

    /// A key the mask takes in the Trace tool; `None` leaves it to the key table.
    pub(super) fn trace_mask_key(&mut self, input: &KeyInput<'_>) -> Option<KeyOutcome> {
        if self.tool != Tool::Trace || input.dom_blocked || input.ctrl || input.alt {
            return None;
        }
        if input.key == "Escape" {
            return self.trace_mask_escape().map(KeyOutcome::Escape);
        }
        let selected = self.trace.mask.selected.clone()?;
        let image = self.trace_mask_image()?;
        if matches!(input.key, "Delete" | "Backspace") {
            self.recording(|this| this.document.delete_trace_region(image.id, &selected).ok());
            self.trace.mask.selected = None;
            return Some(KeyOutcome::Deleted);
        }
        let step = if input.shift { 10 } else { 1 };
        let (dx, dy) = match input.key {
            "ArrowLeft" => (-step, 0),
            "ArrowRight" => (step, 0),
            "ArrowUp" => (0, -step),
            "ArrowDown" => (0, step),
            _ => return None,
        };
        let frame = self.trace_mask_frame()?;
        let (_, rect) = self
            .trace_mask_rects(&frame)
            .into_iter()
            .find(|(id, _)| *id == selected)?;
        let [a, b] = moved_trace_rect(rect, dx, dy).to_unit(frame.width, frame.height);
        self.recording(|this| {
            this.document
                .set_trace_region_points(frame.image.id, &selected, &[a, b], TraceRegionEdit::Move)
                .ok()
        });
        Some(KeyOutcome::Nudged { new_run: true })
    }

    /// Escape on the mask: a drag is cancelled, else the region selection dropped, else the
    /// region tool released; `None` when there was nothing of it.
    pub(super) fn trace_mask_escape(&mut self) -> Option<EscapeStep> {
        if self.tool != Tool::Trace {
            return None;
        }
        if self.trace.mask.drag.take().is_some() {
            return Some(EscapeStep::CancelledDrag);
        }
        if self.trace.mask.selected.take().is_some() {
            return Some(EscapeStep::ClearedState);
        }
        if self.trace.mask.region_tool != TraceRegionTool::None {
            self.trace.mask.region_tool = TraceRegionTool::None;
            return Some(EscapeStep::ClearedState);
        }
        None
    }

    /// The cursor of the mask, when it has one: a crosshair while Rectangle is pressed.
    pub(super) fn trace_mask_cursor(&self) -> Option<&'static str> {
        (self.tool == Tool::Trace && self.trace.mask.region_tool == TraceRegionTool::Rect)
            .then_some("crosshair")
    }
}
