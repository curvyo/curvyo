//! The Trace tool's pointer and keys on the mask (`specs/0114-image-trace-mask` Behaviour 2 to 4,
//! criteria 2 to 7 and 17, UX notes §5 to §7): with Rectangle pressed a drag draws a rectangle,
//! with Lasso pressed a click sets a corner; with no region tool a press on a region of the active
//! part selects it, a drag moves it, a rectangle's handle resizes it and a lasso's corner moves
//! alone; the arrows move the selected region and Delete removes it. Each finished gesture is
//! one step.

use curvyo_document_core::{
    Affine, ImageSnapshot, NewTraceRegion, Point, TraceRegionEdit, TraceRegionKind,
};
use curvyo_ui_core::{
    PxRect, drawn_trace_rect, moved_trace_lasso, moved_trace_rect, resized_trace_rect,
    trace_lasso_corner_at, trace_lasso_from_unit, trace_lasso_hit, trace_lasso_to_unit,
    trace_rect_at, trace_rect_handle_at, with_trace_lasso_corner,
};

use super::keys::{EscapeStep, KeyInput, KeyOutcome};
use super::trace_mask_tool::{MaskDrag, MaskDragKind, RegionPx, TraceRegionTool};
use super::{Session, Tool};

/// How far past a region's outline a press still selects it and the hover box shows, screen
/// pixels (UX notes §5).
const REGION_SLOP_PX: f64 = 8.0;
/// How near a selected lasso's corner a press drags that corner, screen pixels.
const CORNER_HIT_PX: f64 = 16.0;

/// The picture frame a gesture works in: the image, its working image's size, the map from
/// document millimetres to its pixels, and screen pixels per picture pixel.
pub(super) struct MaskFrame {
    pub(super) image: ImageSnapshot,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) to_px: Affine,
    pub(super) screen_per_px: f64,
}

fn kurbo_point(p: Point) -> kurbo::Point {
    kurbo::Point::new(p.x, p.y)
}

/// `v` as whole pixels, held to a range no sum overflows.
fn whole(v: f64) -> i32 {
    // invariant: rounded and clamped into i32's range first; a NaN becomes 0.
    #[allow(clippy::cast_possible_truncation)]
    let r = v.round().clamp(-1.0e9, 1.0e9) as i32;
    r
}

/// The shape a drag leaves with the modifiers `shift` (from the centre) and `ctrl` (a square,
/// or proportions at a corner); `None` for a draw under 2 px.
pub(super) fn drag_shape(drag: &MaskDrag, shift: bool, ctrl: bool) -> Option<RegionPx> {
    let (dx, dy) = (
        whole(drag.to.x - drag.from.x),
        whole(drag.to.y - drag.from.y),
    );
    match &drag.kind {
        MaskDragKind::Draw => drawn_trace_rect(drag.from, drag.to, shift, ctrl).map(RegionPx::Rect),
        MaskDragKind::Move {
            shape: RegionPx::Rect(rect),
            ..
        } => Some(RegionPx::Rect(moved_trace_rect(*rect, dx, dy))),
        MaskDragKind::Move {
            shape: RegionPx::Lasso(corners),
            ..
        } => Some(RegionPx::Lasso(moved_trace_lasso(corners, dx, dy))),
        MaskDragKind::Resize { rect, handle, .. } => Some(RegionPx::Rect(resized_trace_rect(
            *rect, *handle, drag.to, shift, ctrl,
        ))),
        MaskDragKind::Corner { corners, index, .. } => Some(RegionPx::Lasso(
            with_trace_lasso_corner(corners, *index, drag.to),
        )),
    }
}

impl RegionPx {
    /// The region's points in the unit square of a `width` x `height` working image.
    pub(super) fn to_unit(&self, width: u32, height: u32) -> Vec<curvyo_document_core::UnitPoint> {
        match self {
            Self::Rect(rect) => rect.to_unit(width, height).to_vec(),
            Self::Lasso(corners) => trace_lasso_to_unit(corners, width, height),
        }
    }

    /// Whether `p` (picture pixels) hits it, `slop` pixels past its outline included.
    fn hit(&self, p: kurbo::Point, slop: f64) -> bool {
        match self {
            Self::Rect(rect) => trace_rect_at(&[*rect], p, slop).is_some(),
            Self::Lasso(corners) => trace_lasso_hit(corners, p, slop),
        }
    }
}

impl Session {
    /// The frame of the one traced image, when its picture decodes.
    pub(super) fn trace_mask_frame(&mut self) -> Option<MaskFrame> {
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

    /// The regions of the active part in picture pixels of `frame`, with their ids, in order.
    pub(super) fn trace_mask_shapes(&self, width: u32, height: u32) -> Vec<(String, RegionPx)> {
        self.trace_active_mask_part()
            .map(|part| part.regions)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|region| match (region.kind, region.points.as_slice()) {
                (TraceRegionKind::Rect, &[a, b]) => Some((
                    region.id,
                    RegionPx::Rect(PxRect::from_unit([a, b], width, height)),
                )),
                (TraceRegionKind::Lasso, points) => Some((
                    region.id,
                    RegionPx::Lasso(trace_lasso_from_unit(points, width, height)),
                )),
                (TraceRegionKind::Rect, _) => None,
            })
            .collect()
    }

    /// The topmost region of the active part at `p` (picture pixels of `frame`).
    fn trace_mask_region_at(
        &self,
        frame: &MaskFrame,
        p: kurbo::Point,
    ) -> Option<(String, RegionPx)> {
        let slop = if frame.screen_per_px > 0.0 {
            REGION_SLOP_PX / frame.screen_per_px
        } else {
            0.0
        };
        self.trace_mask_shapes(frame.width, frame.height)
            .into_iter()
            .rev()
            .find(|(_, shape)| shape.hit(p, slop))
    }

    /// A press in the Trace tool that the mask takes; `false` leaves it to the selection.
    pub(super) fn trace_mask_press(&mut self, point: Point) -> bool {
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
        match self.trace.mask.region_tool {
            TraceRegionTool::Rect => {
                self.trace.mask.drag = Some(start(MaskDragKind::Draw));
                return true;
            }
            TraceRegionTool::Lasso => {
                self.trace_lasso_press(&frame, p);
                return true;
            }
            TraceRegionTool::None => {}
        }
        // A locked image shows its regions; a press never hits them (UX notes §8).
        if self.trace_mask_locked() {
            return false;
        }
        let scale = frame.screen_per_px;
        let selected = self.trace.mask.selected.clone().and_then(|id| {
            self.trace_mask_shapes(frame.width, frame.height)
                .into_iter()
                .find(|(r, _)| *r == id)
        });
        if let Some((region, shape)) = selected
            && scale > 0.0
        {
            match shape {
                RegionPx::Rect(rect) => {
                    let r = rect.to_kurbo();
                    let screen =
                        kurbo::Rect::new(r.x0 * scale, r.y0 * scale, r.x1 * scale, r.y1 * scale);
                    if let Some(handle) =
                        trace_rect_handle_at(screen, (p.to_vec2() * scale).to_point())
                    {
                        self.trace.mask.drag = Some(start(MaskDragKind::Resize {
                            region,
                            rect,
                            handle,
                        }));
                        return true;
                    }
                }
                RegionPx::Lasso(corners) => {
                    if let Some(index) = trace_lasso_corner_at(&corners, p, CORNER_HIT_PX / scale) {
                        self.trace.mask.corner = Some(index);
                        self.trace.mask.drag = Some(start(MaskDragKind::Corner {
                            region,
                            corners,
                            index,
                        }));
                        return true;
                    }
                }
            }
        }
        if let Some((region, shape)) = self.trace_mask_region_at(&frame, p) {
            if self.trace.mask.selected.as_deref() != Some(region.as_str()) {
                self.trace.mask.corner = None;
            }
            self.trace.mask.selected = Some(region.clone());
            self.trace.mask.drag = Some(start(MaskDragKind::Move { region, shape }));
            return true;
        }
        self.trace.mask.selected = None;
        self.trace.mask.corner = None;
        false
    }

    /// The pointer moved: a mask drag follows it (`true`), else the hover box follows it.
    pub(super) fn trace_mask_drag_move(&mut self, point: Point) -> bool {
        let Some(image) = self.trace_mask_image() else {
            return false;
        };
        let Some(drag) = &mut self.trace.mask.drag else {
            self.trace_mask_hover(point);
            return false;
        };
        if let Some(to_px) = image.pixel_map(drag.width, drag.height).inverse() {
            drag.to = kurbo_point(to_px.apply_point(point));
        }
        true
    }

    /// The hover box: the region of the active part under the pointer, with no region tool and
    /// no drag (UX notes §5).
    fn trace_mask_hover(&mut self, point: Point) {
        self.trace.mask.hovered = None;
        if self.trace.mask.region_tool != TraceRegionTool::None || self.trace_mask_locked() {
            return;
        }
        let Some(frame) = self.trace_mask_frame() else {
            return;
        };
        let p = kurbo_point(frame.to_px.apply_point(point));
        self.trace.mask.hovered = self.trace_mask_region_at(&frame, p).map(|(id, _)| id);
    }

    /// The release of a mask drag: one step "Draw rectangle", "Move region", "Resize region" or
    /// "Move lasso corner", or nothing for a draw under 2 px or an unchanged region. `false`
    /// without a drag.
    pub(super) fn trace_mask_release(&mut self, point: Point, shift: bool, ctrl: bool) -> bool {
        if self.trace.mask.drag.is_none() || !self.trace_mask_drag_move(point) {
            return false;
        }
        let Some(drag) = self.trace.mask.drag.take() else {
            return false;
        };
        let Some(owner) = self.trace_mask_owner_id() else {
            return true;
        };
        let Some(shape) = drag_shape(&drag, shift, ctrl) else {
            return true;
        };
        let points = shape.to_unit(drag.width, drag.height);
        let (region, edit) = match drag.kind {
            MaskDragKind::Draw => {
                let region = NewTraceRegion {
                    kind: TraceRegionKind::Rect,
                    subtract: self.trace.mask.subtract,
                    points,
                };
                let part = self.trace_active_mask_part().and_then(|part| part.stored);
                let document = &self.document;
                let _ = self.with_part_ref(part.as_deref(), |part| {
                    document.draw_trace_region(owner, part, &region)
                });
                return true;
            }
            MaskDragKind::Move { region, .. } => (region, TraceRegionEdit::Move),
            MaskDragKind::Resize { region, .. } => (region, TraceRegionEdit::Resize),
            MaskDragKind::Corner { region, .. } => (region, TraceRegionEdit::MoveCorner),
        };
        let _ = self
            .document
            .set_trace_region_points(owner, &region, &points, edit);
        true
    }

    /// A key the mask takes in the Trace tool; `None` leaves it to the key table.
    pub(super) fn trace_mask_key(&mut self, input: &KeyInput<'_>) -> Option<KeyOutcome> {
        if self.tool != Tool::Trace || input.dom_blocked || input.alt {
            return None;
        }
        if input.ctrl {
            return self.trace_mask_history_key(input);
        }
        match input.key {
            "Escape" => return self.trace_mask_escape().map(KeyOutcome::Escape),
            "Enter" if self.trace_lasso_open() => {
                self.close_trace_lasso();
                return Some(KeyOutcome::Consumed);
            }
            "Backspace" if self.trace_lasso_backspace() => return Some(KeyOutcome::Consumed),
            _ => {}
        }
        if self.trace_lasso_open() || self.trace_mask_locked() {
            return None;
        }
        let selected = self.trace.mask.selected.clone()?;
        let owner = self.trace_mask_owner_id()?;
        if matches!(input.key, "Delete" | "Backspace") {
            self.recording(|this| this.document.delete_trace_region(owner, &selected).ok());
            self.trace.mask.selected = None;
            self.trace.mask.corner = None;
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
        let (_, shape) = self
            .trace_mask_shapes(frame.width, frame.height)
            .into_iter()
            .find(|(id, _)| *id == selected)?;
        let moved = match shape {
            RegionPx::Rect(rect) => RegionPx::Rect(moved_trace_rect(rect, dx, dy)),
            RegionPx::Lasso(corners) => RegionPx::Lasso(moved_trace_lasso(&corners, dx, dy)),
        };
        let points = moved.to_unit(frame.width, frame.height);
        self.recording(|this| {
            this.document
                .set_trace_region_points(owner, &selected, &points, TraceRegionEdit::Move)
                .ok()
        });
        Some(KeyOutcome::Nudged { new_run: true })
    }

    /// Ctrl+Z with a lasso in progress discards it and undoes nothing else; a redo discards it
    /// and then runs (`0114` adrs.md, UX notes §6).
    fn trace_mask_history_key(&mut self, input: &KeyInput<'_>) -> Option<KeyOutcome> {
        let key = input.key.to_ascii_lowercase();
        let undo = key == "z" && !input.shift;
        let redo = (key == "z" && input.shift) || key == "y";
        if undo && self.discard_trace_lasso() {
            return Some(KeyOutcome::Consumed);
        }
        if redo {
            self.discard_trace_lasso();
        }
        None
    }

    /// Escape on the mask (UX notes §7): a drag is cancelled, else the lasso in progress is
    /// discarded, else the region tool released, else the region selection dropped; `None` when
    /// there was nothing of it. A running trace and the pipette are the host's steps between the
    /// lasso and the region tool.
    pub(super) fn trace_mask_escape(&mut self) -> Option<EscapeStep> {
        if self.tool != Tool::Trace {
            return None;
        }
        if self.trace.mask.drag.take().is_some() {
            return Some(EscapeStep::CancelledDrag);
        }
        if self.discard_trace_lasso() {
            return Some(EscapeStep::ClearedState);
        }
        if self.trace.mask.region_tool != TraceRegionTool::None {
            self.trace.mask.region_tool = TraceRegionTool::None;
            return Some(EscapeStep::ClearedState);
        }
        if self.trace.mask.selected.take().is_some() {
            self.trace.mask.corner = None;
            return Some(EscapeStep::ClearedState);
        }
        None
    }

    /// Whether the mask holds the first Escape over a running trace: a drag or a lasso in
    /// progress (UX notes §7, steps 1 and 2).
    #[must_use]
    pub fn trace_mask_holds_escape(&self) -> bool {
        self.tool == Tool::Trace
            && (self.trace.mask.drag.is_some() || self.trace.mask.lasso.is_some())
    }

    /// The cursor of the mask, when it has one (UX notes §5): a crosshair while a region tool is
    /// pressed, the rotated double arrow over a selected rectangle's handle.
    pub(super) fn trace_mask_cursor(&self) -> Option<String> {
        if self.tool != Tool::Trace {
            return None;
        }
        if self.trace.mask.region_tool != TraceRegionTool::None {
            return Some("crosshair".to_owned());
        }
        if let Some(MaskDrag {
            kind: MaskDragKind::Resize { handle, .. },
            ..
        }) = &self.trace.mask.drag
        {
            return self.trace_handle_cursor(*handle);
        }
        let handle = self.trace_handle_under_pointer()?;
        self.trace_handle_cursor(handle)
    }

    /// The handle of the selected rectangle under the pointer.
    fn trace_handle_under_pointer(&self) -> Option<curvyo_ui_core::TraceRegionHandle> {
        let pointer = self.pointer_position?;
        let selected = self.trace.mask.selected.as_ref()?;
        let image = self.trace_mask_image()?;
        let working = self.working_image_size(&image.asset)?;
        let pixel_map = image.pixel_map(working.0, working.1);
        let scale = self.view().scale() * pixel_map.determinant().abs().sqrt();
        let p = kurbo_point(pixel_map.inverse()?.apply_point(pointer));
        let (_, shape) = self
            .trace_mask_shapes(working.0, working.1)
            .into_iter()
            .find(|(id, _)| id == selected)?;
        let RegionPx::Rect(rect) = shape else {
            return None;
        };
        let r = rect.to_kurbo();
        let screen = kurbo::Rect::new(r.x0 * scale, r.y0 * scale, r.x1 * scale, r.y1 * scale);
        trace_rect_handle_at(screen, (p.to_vec2() * scale).to_point())
    }

    /// `"resize:<degrees>"` for `handle`: its direction on screen in the image's placement.
    fn trace_handle_cursor(&self, handle: curvyo_ui_core::TraceRegionHandle) -> Option<String> {
        let image = self.trace_mask_image()?;
        let (dx, dy) = handle.direction();
        let m = image.placement;
        let (x, y) = (
            m.a * f64::from(dx) + m.c * f64::from(dy),
            m.b * f64::from(dx) + m.d * f64::from(dy),
        );
        let degrees = y.atan2(x).to_degrees().rem_euclid(180.0);
        Some(format!("resize:{degrees:.1}"))
    }
}
