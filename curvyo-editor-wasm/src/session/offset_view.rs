//! What the Offset tool shows (`specs/0038-path-offset` criteria 5, 9, 12, 14, 15, 18, 52): the
//! view the chip and the readout read, and the canvas layer of preview, box and handles.

use curvyo_document_core::{ObjectSnapshot, Point};
use curvyo_ui_core::{
    OffsetDirection, OffsetJoin, OffsetRefusal, OffsetRequest, OffsetResultChoice, OrientedBox,
    handle_positions, offset_operands,
};

use super::offset::DragStop;
use super::{Session, Tool};

/// Above this many operand nodes the preview is the grown box (criterion 18).
const PREVIEW_NODE_LIMIT: usize = 2000;

/// The nodes a primitive counts for the preview limit.
const PRIMITIVE_NODES: usize = 8;

/// What the chip, the readout and the canvas show (criteria 5, 12, 15, 18, 56).
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
    /// A drag is in flight and has left the dead zone around the press.
    pub dragging: bool,
    /// The signed distance shown (the drag's, or the remembered one with its Direction).
    pub distance_mm: f64,
    /// The corner style in use: the drag's, else the remembered one (criteria 15, 56).
    pub join: OffsetJoin,
    /// Both sides (Shift during a drag, or Direction Both).
    pub both: bool,
    /// The release or the chip's Enter changes the original.
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
                ObjectSnapshot::Clone(clone) => clone.resolved.all_anchors().count(),
                ObjectSnapshot::Text(_) => {
                    object.drawn_path().map_or(0, |p| p.all_anchors().count())
                }
                ObjectSnapshot::Primitive(_) => PRIMITIVE_NODES,
                ObjectSnapshot::Sketch(_) | ObjectSnapshot::Image(_) => 0,
            })
            .sum();
        nodes > PREVIEW_NODE_LIMIT
    }

    /// The request the canvas previews now (criterion 52): a handle drag's once it has left the
    /// dead zone, or the open chip's values; otherwise none.
    pub(super) fn offset_preview_request(&self) -> Option<OffsetRequest> {
        if self.tool != Tool::Offset {
            return None;
        }
        match self.offset.drag {
            Some(drag) if drag.moved => {
                (drag.d != 0.0).then(|| self.offset_request(Some((drag.d, self.held))))
            }
            Some(_) => None,
            None => self.offset_chip_open().then(|| self.offset_request(None)),
        }
    }

    /// What the chip, readout and canvas show.
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
            dragging: drag.is_some_and(|drag| drag.moved),
            distance_mm,
            join: self.offset_join(),
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
        if let Some(center) = self.offset_center() {
            list.extend(curvyo_render_core::build_offset_center_handle(
                center,
                curvyo_render_core::OffsetCenterState {
                    hovered: self.offset.center_hovered,
                    open: self.offset_chip_open(),
                },
                view,
            ));
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

impl Session {
    /// The selection boxes of the selected objects that are not operands (open paths): they
    /// stay drawn as selected (criterion 55).
    pub(super) fn offset_ignored_boxes(
        &self,
        objects: &[ObjectSnapshot],
    ) -> Vec<(curvyo_document_core::NodeId, [Point; 4])> {
        let selection = self.offset_selection();
        let operands: Vec<_> = offset_operands(objects, &selection)
            .iter()
            .map(|object| object.id())
            .collect();
        objects
            .iter()
            .filter(|object| selection.contains(object.id()) && !operands.contains(&object.id()))
            .map(|object| {
                (
                    object.id(),
                    curvyo_ui_core::oriented_bounds(object).document_corners(),
                )
            })
            .collect()
    }

    /// Where the Offset entry chip is placed from, in document millimetres: the center handle
    /// and the axis-aligned box around the operand box and the standing preview (`ux-rework.md`
    /// 2.4). `None` without operands.
    #[must_use]
    pub fn offset_chip_anchor(&self) -> Option<(Point, Point, Point)> {
        let center = self.offset_center()?;
        let mut points: Vec<Point> = self
            .offset_box_now()
            .map(|bounds| bounds.document_corners().to_vec())
            .unwrap_or_default();
        if !self.offset_selection_is_large()
            && let Some(request) = self.offset_preview_request()
            && let Ok(plan) = self.offset_plan_for(request)
        {
            points.extend(
                plan.preview_outlines()
                    .iter()
                    .flatten()
                    .map(|anchor| anchor.0),
            );
        }
        let min = points
            .iter()
            .fold(center, |m, p| Point::new(m.x.min(p.x), m.y.min(p.y)));
        let max = points
            .iter()
            .fold(center, |m, p| Point::new(m.x.max(p.x), m.y.max(p.y)));
        Some((center, min, max))
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
