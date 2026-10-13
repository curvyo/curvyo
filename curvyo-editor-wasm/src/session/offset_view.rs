//! What the Offset tool shows (`specs/0038-path-offset` criteria 4, 5, 9, 12, 14, 15, 18): the
//! view the bar and the readout read, and the canvas layer of preview, box and handles.

use curvyo_document_core::{ObjectSnapshot, Point};
use curvyo_ui_core::{
    OffsetDirection, OffsetRefusal, OffsetRequest, OffsetResultChoice, OrientedBox,
    handle_positions, offset_operands,
};

use super::offset::DragStop;
use super::{Session, Tool};

/// Above this many operand nodes the preview is the grown box (criterion 18).
const PREVIEW_NODE_LIMIT: usize = 2000;

/// The nodes a primitive counts for the preview limit.
const PRIMITIVE_NODES: usize = 8;

/// What the bar, the readout and the canvas show (criteria 4, 5, 12, 15, 18).
// Flags the host reads one by one, not a state machine.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq)]
pub struct OffsetView {
    /// Operands in the selection (groups not counted).
    pub operands: usize,
    /// The selection holds a group.
    pub has_group: bool,
    /// The handles are drawn.
    pub handles: bool,
    /// A drag is in flight.
    pub dragging: bool,
    /// The signed distance shown (the drag's, or the bar's with its Direction).
    pub distance_mm: f64,
    /// Both sides (Shift during a drag, or Direction Both).
    pub both: bool,
    /// The release or Apply changes the original.
    pub change: bool,
    /// Where the handle stopped, if it did.
    pub stop: Option<DragStop>,
    /// Operands whose inset vanishes in the preview.
    pub vanished: usize,
    /// How many operands the preview covers.
    pub of: usize,
    /// The preview is replaced by the grown box.
    pub large: bool,
    /// Why the preview shows nothing, if it is refused.
    pub refusal: Option<OffsetRefusal>,
}

impl Session {
    /// Whether the operands have more nodes than the preview draws (criterion 18).
    pub(super) fn offset_selection_is_large(&self) -> bool {
        let objects = self.objects();
        let selection = self.offset_selection();
        let nodes: usize = offset_operands(&objects, &selection)
            .iter()
            .map(|object| match object {
                ObjectSnapshot::Path(path) => path.all_anchors().count(),
                ObjectSnapshot::Primitive(_) => PRIMITIVE_NODES,
            })
            .sum();
        nodes > PREVIEW_NODE_LIMIT
    }

    /// The request the canvas previews now: the drag's, or the bar's while focus is in the bar.
    fn offset_preview_request(&self) -> Option<OffsetRequest> {
        if self.tool != Tool::Offset {
            return None;
        }
        if let Some(drag) = self.offset.drag {
            return (drag.d != 0.0).then(|| self.offset_request(Some((drag.d, self.held))));
        }
        None
    }

    /// What the bar, readout and canvas show.
    #[must_use]
    pub fn offset_view(&self) -> OffsetView {
        let objects = self.objects();
        let selection = self.offset_selection();
        let operands = offset_operands(&objects, &selection);
        let drag = self.offset.drag;
        let values = self.offset.values;
        let (distance_mm, both, change) = match drag {
            Some(drag) => (
                drag.d,
                self.held.shift,
                values.changes_original(self.held.ctrl),
            ),
            None => (
                if values.direction() == OffsetDirection::In {
                    -values.distance_mm()
                } else {
                    values.distance_mm()
                },
                values.direction() == OffsetDirection::Both,
                values.result() == OffsetResultChoice::ChangeOriginal,
            ),
        };
        let large = self.offset_selection_is_large();
        let mut view = OffsetView {
            operands: operands.len(),
            has_group: !self.selected_group_leaves().is_empty(),
            handles: self.offset_handles_shown(),
            dragging: drag.is_some(),
            distance_mm,
            both,
            change,
            stop: drag.and_then(|drag| drag.stop),
            vanished: 0,
            of: operands.len(),
            large,
            refusal: None,
        };
        if large {
            return view;
        }
        if let Some(request) = self.offset_preview_request() {
            match self.offset_plan_for(request) {
                Ok(plan) => {
                    view.vanished = plan.vanished;
                }
                Err(refusal) => view.refusal = Some(refusal),
            }
        }
        view
    }

    /// The Offset tool's canvas layer: the preview outlines (or the grown box), the selection box
    /// and the handles.
    pub(super) fn offset_draw_list(
        &self,
        view: curvyo_document_core::ViewTransform,
    ) -> curvyo_render_core::DrawList {
        let mut list = curvyo_render_core::DrawList::default();
        if self.tool != Tool::Offset {
            return list;
        }
        let Some(bounds) = self.offset_box_now() else {
            return list;
        };
        if let Some(request) = self.offset_preview_request() {
            if self.offset_selection_is_large() {
                let signed = if request.sides.out.is_some() {
                    request.distance_mm
                } else {
                    -request.distance_mm
                };
                list.extend(curvyo_render_core::build_offset_grown_box(
                    grown_box(&bounds, signed).document_corners(),
                    view,
                ));
            } else if let Ok(plan) = self.offset_plan_for(request) {
                list.extend(curvyo_render_core::build_offset_preview(
                    &plan.preview_outlines(),
                    view,
                ));
            }
        }
        if self.offset_handles_shown() {
            let placed = self
                .offset
                .drag
                .map_or(bounds, |drag| grown_box(&bounds, drag.d));
            list.extend(curvyo_render_core::build_offset_handles(
                bounds.document_corners(),
                handle_positions(&placed, view.scale()),
                curvyo_render_core::OffsetHandleState {
                    hovered: self.offset.hovered,
                    dragged: self.offset.drag.map(|drag| drag.corner),
                },
                view,
            ));
        }
        list
    }
}

/// `bounds` grown by `d` on every side (shrunk at most to its middle), for the handles during a
/// drag and the grown box.
fn grown_box(bounds: &OrientedBox, d: f64) -> OrientedBox {
    let grow = d.max(-bounds.width().min(bounds.height()) / 2.0);
    OrientedBox {
        min: Point::new(bounds.min.x - grow, bounds.min.y - grow),
        max: Point::new(bounds.max.x + grow, bounds.max.y + grow),
        angle: bounds.angle,
        pivot: bounds.pivot,
    }
}
