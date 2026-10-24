//! Drags of a selected text on path (`specs/0080-text-on-path` slice 1): the position handle, the
//! body with its Ctrl copy, the corners, the rotate handles, their live preview and readout.

use std::sync::Arc;

use curvyo_document_core::{
    EllipseCarrier, Length, LineOnPath, NodeId, OnPathLabel, OnPathValues, PathSide, Point,
    TextSnapshot,
};

use super::shapes::LiveReadout;
use super::text_on_path::OnCarrier;
use super::{Session, Tool};

/// How far beyond the carrier (screen px) a handle drag switches the side (criterion 16).
const CROSS_PX: f64 = 8.0;
/// The position handle's hit radius, screen px ("Parameter handle").
const HANDLE_HIT_PX: f64 = 12.0;

/// What a drag of a selected text on path does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum DragKind {
    /// The position handle or the text's body: along the carrier (criteria 15, 16, 21).
    Slide,
    /// A corner of the selection box: Size about the text's middle (criterion 21).
    Size {
        /// The pointer's distance from the text's middle at the press, mm.
        from: f64,
        /// The size at the press.
        size: Length,
    },
    /// A rotate handle on an ellipse carrier: about its centre (criterion 20).
    Rotate {
        /// The pointer's angle about the centre at the press, radians.
        from: f64,
    },
}

/// A drag of a selected text on path in flight.
#[derive(Debug, Clone, Copy)]
pub(super) struct PathDrag {
    pub(super) text: NodeId,
    pub(super) kind: DragKind,
    pub(super) start: OnPathValues,
    pub(super) values: OnPathValues,
    pub(super) size: Length,
    /// The press point: a body press moves nothing until the pointer leaves the dead zone.
    pub(super) press: Point,
    /// How far along the carrier the press lay from the text's middle: kept during the drag.
    pub(super) grab: Length,
    /// A press on the text's body, not on the position handle: Ctrl at the release copies
    /// (criterion 31) instead of snapping.
    pub(super) body: bool,
    pub(super) moved: bool,
}

impl Session {
    /// The cursor of text on path: "pick-carrier" in pick mode, "pointer" over or dragging the
    /// position handle, else `None`.
    pub(super) fn text_path_cursor(&self) -> Option<&'static str> {
        if self.text_path.pick.is_some() {
            return Some("pick-carrier");
        }
        if let Some(drag) = self.text_path.drag {
            return (drag.kind == DragKind::Slide).then_some("pointer");
        }
        let point = self.pointer_position?;
        let text = self.sole_text_on_path()?;
        let on = self.on_carrier(&text)?;
        let (handle, _) = curvyo_ui_core::handle_points(&on.placement);
        (handle.vector_to(point).length() <= HANDLE_HIT_PX / self.view().scale())
            .then_some("pointer")
    }

    /// The pointer went down: pick mode takes the press; with one text on path selected its
    /// position handle, a corner, a rotate handle on an ellipse carrier or its body start a drag
    /// (criteria 15, 20, 21). Returns whether the press was taken.
    pub(super) fn text_path_pointer_down(&mut self, point: Point, shift: bool) -> bool {
        if self.text_path.pick.is_some() {
            self.pick_press(point, shift);
            return true;
        }
        if shift || !matches!(self.tool, Tool::Select | Tool::Text) {
            return false;
        }
        let Some(text) = self.sole_text_on_path() else {
            return false;
        };
        let Some(on) = self.on_carrier(&text) else {
            return false;
        };
        let Some(on_path) = text.on_path.as_ref() else {
            return false;
        };
        let scale = self.view().scale();
        let (handle, _) = curvyo_ui_core::handle_points(&on.placement);
        let on_handle = handle.vector_to(point).length() <= HANDLE_HIT_PX / scale;
        let kind = if on_handle {
            Some(DragKind::Slide)
        } else if self.tool == Tool::Select && self.text.edit.is_none() {
            self.box_handle_at(&text, &on, point).or_else(|| {
                curvyo_ui_core::line_box_contains(&text, point, 0.0).then_some(DragKind::Slide)
            })
        } else {
            None
        };
        let Some(kind) = kind else {
            return false;
        };
        let measure = on.placement.measure();
        self.text_path.drag = Some(PathDrag {
            text: text.id,
            kind,
            start: on_path.values,
            values: on_path.values,
            size: text.size,
            press: point,
            grab: curvyo_ui_core::shift_along(
                measure,
                on_path.values.position,
                measure.nearest(point),
            ),
            body: !on_handle,
            moved: false,
        });
        true
    }

    /// The selection-box handle of `text` under `point`: a corner (Size) or, on an ellipse
    /// carrier, a rotate handle outside a corner.
    fn box_handle_at(&self, text: &TextSnapshot, on: &OnCarrier, point: Point) -> Option<DragKind> {
        let tolerance = self.handle_tolerance().as_mm();
        let reference = on
            .placement
            .measure()
            .at(on.placement.values().position)
            .point;
        for (corner, rotate) in self.box_handles(text) {
            if corner.vector_to(point).length() > tolerance {
                continue;
            }
            return Some(if rotate {
                let centre = text.on_path.as_ref()?.curve.as_ref()?.ellipse?.centre;
                let v = centre.vector_to(point);
                DragKind::Rotate {
                    from: v.y.atan2(v.x),
                }
            } else {
                DragKind::Size {
                    from: reference.vector_to(point).length().max(1e-6),
                    size: text.size,
                }
            });
        }
        None
    }

    /// The handles of the selection box of `text`: its four corners (Size) and, on an ellipse
    /// carrier, a rotate handle off each corner (criteria 20, 21).
    pub(super) fn box_handles(&self, text: &TextSnapshot) -> Vec<(Point, bool)> {
        let [a, _, c, _] = curvyo_ui_core::line_box_corners(text);
        let corners = [a, Point::new(c.x, a.y), c, Point::new(a.x, c.y)];
        let mut handles: Vec<(Point, bool)> = corners.iter().map(|p| (*p, false)).collect();
        let ellipse = text
            .on_path
            .as_ref()
            .and_then(|p| p.curve.as_ref())
            .is_some_and(|c| c.ellipse.is_some());
        if ellipse {
            let offset = self.handle_tolerance().as_mm() * 1.5;
            let signs = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
            for (corner, (sx, sy)) in corners.iter().zip(signs) {
                handles.push((
                    Point::new(corner.x + sx * offset, corner.y + sy * offset),
                    true,
                ));
            }
        }
        handles
    }

    /// The pointer moved: pick mode tracks the carrier under it; a drag follows the pointer.
    /// Returns whether the move was taken.
    pub(super) fn text_path_pointer_move(&mut self, point: Point) -> bool {
        if let Some(text) = self.text_path.pick.as_ref().map(|pick| pick.text) {
            self.text_path.hover = self.carrier_at(point, text);
            return true;
        }
        let Some(mut drag) = self.text_path.drag else {
            return false;
        };
        if !drag.moved && drag.press.vector_to(point).length() * self.view().scale() < 3.0 {
            return true;
        }
        drag.moved = true;
        let Some(text) = self.read_text(drag.text) else {
            return true;
        };
        let Some(on) = self.on_carrier(&text) else {
            return true;
        };
        let measure = on.placement.measure();
        let ellipse = text
            .on_path
            .as_ref()
            .and_then(|p| p.curve.as_ref())
            .and_then(|c| c.ellipse);
        match drag.kind {
            DragKind::Slide => drag.values = self.slid_values(&drag, &on, ellipse.as_ref(), point),
            DragKind::Size { from, size } => {
                let reference = measure.at(drag.start.position).point;
                let factor = reference.vector_to(point).length() / from;
                drag.size =
                    curvyo_document_core::clamp_text_size(Length::from_mm(size.as_mm() * factor));
            }
            DragKind::Rotate { from } => {
                if let Some(ellipse) = ellipse {
                    let v = ellipse.centre.vector_to(point);
                    let mut degrees = (v.y.atan2(v.x) - from).to_degrees();
                    if self.held.ctrl {
                        degrees = (degrees / 22.5).round() * 22.5;
                    }
                    drag.values.position =
                        curvyo_ui_core::rotated_position(measure, &ellipse, &drag.start, degrees);
                }
            }
        }
        self.text_path.drag = Some(drag);
        true
    }

    /// The values of a Slide drag at `point`: along the carrier with the side switch and, with
    /// Ctrl on the position handle, the snap stops; a Ctrl body drag projects on the carrier.
    fn slid_values(
        &self,
        drag: &PathDrag,
        on: &OnCarrier,
        ellipse: Option<&EllipseCarrier>,
        point: Point,
    ) -> OnPathValues {
        let measure = on.placement.measure();
        if drag.body && self.held.ctrl {
            // A Ctrl-copy move: the drag projected on the carrier (criterion 31).
            return curvyo_ui_core::projected_values(measure, drag.start, drag.press, point);
        }
        let stops = (self.held.ctrl && !drag.body).then(|| {
            let default = curvyo_ui_core::default_position(&on.laid, measure);
            curvyo_ui_core::snap_stops(measure, ellipse, default)
        });
        curvyo_ui_core::slid_values(
            measure,
            drag.start,
            drag.grab,
            point,
            CROSS_PX / self.view().scale(),
            stops.as_deref(),
        )
    }

    /// The release ends a drag with its one step ("Move text on path", "Mirror text on path"
    /// after a crossing, "Change text size"). Returns whether it was taken.
    pub(super) fn text_path_pointer_up(&mut self) -> bool {
        let Some(drag) = self.text_path.drag.take() else {
            return false;
        };
        if !drag.moved {
            return true;
        }
        match drag.kind {
            DragKind::Size { .. } => {
                let _ = self.document.set_text_size(&[drag.text], drag.size);
            }
            DragKind::Slide if drag.body && self.held.ctrl => self.copy_text_on_path(&drag),
            DragKind::Slide | DragKind::Rotate { .. } => {
                let label = if drag.values.side == drag.start.side {
                    OnPathLabel::Move
                } else {
                    OnPathLabel::Mirror
                };
                let _ = self
                    .document
                    .set_text_path(&[(drag.text, drag.values)], label);
            }
        }
        true
    }

    /// The release of a Ctrl body drag (criterion 31): a copy of the text on the same carrier,
    /// Position shifted by the drag along the carrier, the original kept; the copy is selected.
    fn copy_text_on_path(&mut self, drag: &PathDrag) {
        let Some(text) = self.read_text(drag.text) else {
            return;
        };
        let Some(on) = self.on_carrier(&text) else {
            return;
        };
        let shift = curvyo_ui_core::shift_along(
            on.placement.measure(),
            drag.start.position,
            drag.values.position,
        );
        let source = curvyo_document_core::CopySource {
            id: drag.text,
            anchor_ids: Vec::new(),
        };
        let shifts = std::collections::HashMap::from([(drag.text, shift)]);
        if let Ok(copies) = self.document.duplicate_objects_shifting(
            &[source],
            curvyo_document_core::Vec2::ZERO,
            &shifts,
        ) {
            self.selection.set(&copies);
        }
    }

    /// Escape during a drag cancels it: the release writes nothing. Returns whether one ran.
    pub(super) fn cancel_text_path_drag(&mut self) -> bool {
        self.text_path.drag.take().is_some()
    }

    /// Whether a text-on-path drag is in flight.
    pub(super) fn text_path_dragging(&self) -> bool {
        self.text_path.drag.is_some_and(|d| d.moved)
    }

    /// The texts on path as drawn this frame: the dragged one at its live values (criterion 15),
    /// or the one of the handle's entry box at its pending values.
    pub(super) fn text_path_live(&self) -> Option<TextSnapshot> {
        let (mut text, values) = if let Some(drag) = self.text_path.drag.filter(|d| d.moved) {
            let mut text = self.read_text(drag.text)?;
            text.size = drag.size;
            (text, drag.values)
        } else {
            let (id, values) = self.text_path_entry_values()?;
            (self.read_text(id)?, values)
        };
        text.on_path.as_mut()?.values = values;
        text.outline = self.fonts.lay_out(&text).map(Arc::new);
        Some(text)
    }

    /// The readout chip of a drag (UX notes §3.2): "42.3 mm" or "90.0°", a snap point and the
    /// state after a crossing.
    pub(super) fn text_path_readout(&self) -> Option<LiveReadout> {
        let drag = self.text_path.drag.filter(|d| d.moved)?;
        let text = self.text_path_live()?;
        let on = self.on_carrier(&text)?;
        let measure = on.placement.measure();
        let (handle, _) = curvyo_ui_core::handle_points(&on.placement);
        let ellipse = text
            .on_path
            .as_ref()
            .and_then(|p| p.curve.as_ref())
            .and_then(|c| c.ellipse);
        let mut readout = match (drag.kind, ellipse) {
            (DragKind::Size { .. }, _) => format!("{:.1} mm", drag.size.as_mm()),
            (_, Some(ellipse)) => {
                format!("{:.1}°", curvyo_ui_core::ellipse_degrees(&ellipse, handle))
            }
            _ => format!("{:.1} mm", drag.values.position.as_mm()),
        };
        if self.held.ctrl && drag.kind == DragKind::Slide && !drag.body && !measure.is_closed() {
            let length = measure.length().as_mm();
            let s = drag.values.position.as_mm();
            for (at, name) in [(0.0, "start"), (length / 2.0, "middle"), (length, "end")] {
                if (s - at).abs() < 1e-6 {
                    readout.push_str(" · ");
                    readout.push_str(name);
                }
            }
        }
        if drag.values.side != drag.start.side {
            readout.push_str(match (drag.values.side, drag.values.line) {
                (PathSide::Above, LineOnPath::Baseline) => " · Above, baseline",
                (PathSide::Above, LineOnPath::TopEdge) => " · Above, top edge",
                (PathSide::Below, LineOnPath::Baseline) => " · Below, baseline",
                (PathSide::Below, LineOnPath::TopEdge) => " · Below, top edge",
            });
        }
        Some(LiveReadout {
            text: readout,
            anchor: handle,
        })
    }
}
