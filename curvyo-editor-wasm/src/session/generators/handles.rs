//! The drag knobs of the one selected generated object (`specs/0141-parametric-generators` UX
//! notes §9): which show, a press on one, the drag with its blue preview and the one commit on
//! release.

use std::cell::RefCell;

use curvyo_document_core::{Affine, GeneratedRecord, Length, NodeId, Point};
use curvyo_extension_core::ParamValue;
use curvyo_render_core::{
    DrawList, TransformDecorationInput, TransformGlyphKind, TransformHandleGlyph,
    build_transform_draw_list,
};
use curvyo_ui_core::SelectTool;
use curvyo_ui_core::instance::{self, change_value, plan_value};
use curvyo_ui_core::instance_handles::{Knob, dragged_mm, knobs};

use super::{
    super::{Session, Tool},
    Preview,
};

/// A shorter box side on screen under which a generated object shows no knob (UX notes §9).
const MIN_SIDE_PX: f64 = 72.0;
/// Two knobs, or a knob and a transform handle, closer than this on screen: the later one is not
/// drawn.
const COLLISION_PX: f64 = 16.0;

/// The knobs of an object as the generator gave them, kept until the object changes.
#[derive(Debug)]
pub(super) struct KnobMemo {
    id: NodeId,
    record: GeneratedRecord,
    frame: Affine,
    /// Whether the generator was available (known and switched on) when the knobs were made.
    available: bool,
    knobs: Vec<Knob>,
    /// The corners of the outline's box in document millimetres.
    bounds: (Point, Point),
}

/// A knob press in flight.
#[derive(Debug)]
pub(super) struct KnobDrag {
    id: NodeId,
    knob: Knob,
    /// Where the press happened: the drag measures from it.
    from: Point,
    /// What the pointer asks for now, in millimetres, and what the generator allows.
    asked: f64,
    shown: f64,
}

/// The state of the knobs: the memo of the generator's answer and the drag.
#[derive(Debug, Default)]
pub(super) struct KnobState {
    memo: RefCell<Option<KnobMemo>>,
    drag: Option<KnobDrag>,
}

impl Session {
    /// The knobs of the object and the corners of its outline, from the memo while the record and
    /// frame are the ones it was made from.
    fn raw_knobs(&self) -> Option<(NodeId, Vec<Knob>, (Point, Point))> {
        let [id] = self.selected_generated()[..] else {
            return None;
        };
        if self.selection.ids().len() != 1 || self.tool != Tool::Select {
            return None;
        }
        let snapshot = self.document.generated(id)?;
        let available =
            instance::find_kind(&self.manufacturing.extensions, &snapshot.record.kind).is_some();
        let mut memo = self.generators.knobs.memo.borrow_mut();
        let stale = memo.as_ref().is_none_or(|held| {
            held.id != id
                || held.record != snapshot.record
                || held.frame != snapshot.frame
                || held.available != available
        });
        if stale {
            let made = knobs(&self.manufacturing.extensions, &snapshot);
            let mut bounds = (
                Point::new(f64::MAX, f64::MAX),
                Point::new(f64::MIN, f64::MIN),
            );
            for subpath in snapshot.record.parts.iter().flatten() {
                for pair in subpath.as_chunks::<2>().0 {
                    let at = snapshot.frame.apply_point(Point::new(pair[0], pair[1]));
                    bounds.0 = Point::new(bounds.0.x.min(at.x), bounds.0.y.min(at.y));
                    bounds.1 = Point::new(bounds.1.x.max(at.x), bounds.1.y.max(at.y));
                }
            }
            *memo = Some(KnobMemo {
                id,
                record: snapshot.record,
                frame: snapshot.frame,
                available,
                knobs: made,
                bounds,
            });
        }
        let held = memo.as_ref()?;
        Some((id, held.knobs.clone(), held.bounds))
    }

    /// The knobs drawn and hit right now: the 72 px rule, then the collision rule in the order of
    /// the table of UX notes §9.
    fn visible_knobs(&self) -> Vec<Knob> {
        if self.generators.knobs.drag.is_none()
            && (self.select.drag_in_flight()
                || self.edit_scope() == curvyo_ui_core::Scope::MoveOnly)
        {
            return Vec::new();
        }
        let Some((_, raw, (low, high))) = self.raw_knobs() else {
            return Vec::new();
        };
        let scale = self.view().scale();
        if (high.x - low.x).min(high.y - low.y) * scale < MIN_SIDE_PX {
            return Vec::new();
        }
        let transform: Vec<Point> = SelectTool::transform_handles(
            &self.objects(),
            &self.selection,
            self.transform_handle_tolerances(),
            false,
        )
        .into_iter()
        .map(|(_, at)| at)
        .collect();
        let near = |a: Point, b: Point| a.vector_to(b).length() * scale < COLLISION_PX;
        let mut kept: Vec<Knob> = Vec::new();
        for knob in raw {
            if kept.iter().any(|earlier| near(earlier.at, knob.at))
                || transform.iter().any(|at| near(*at, knob.at))
            {
                continue;
            }
            kept.push(knob);
        }
        kept
    }

    /// The knob under `point`, within the parameter handle's hit radius.
    fn knob_at(&self, knobs: &[Knob], point: Point) -> Option<usize> {
        let radius = self.transform_handle_tolerances().param_hit.as_mm();
        knobs
            .iter()
            .enumerate()
            .map(|(index, knob)| (index, knob.at.vector_to(point).length()))
            .filter(|(_, distance)| *distance <= radius)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(index, _)| index)
    }

    /// A press at `point` on a knob starts its drag: `true` when it took the press.
    pub(in crate::session) fn generator_knob_press(&mut self, point: Point) -> bool {
        let knobs = self.visible_knobs();
        let Some(index) = self.knob_at(&knobs, point) else {
            return false;
        };
        let Some((id, _, _)) = self.raw_knobs() else {
            return false;
        };
        let knob = knobs[index].clone();
        self.generators.knobs.drag = Some(KnobDrag {
            id,
            asked: knob.value_mm,
            shown: knob.value_mm,
            knob,
            from: point,
        });
        true
    }

    /// The pointer moved to `point` with a knob held: previews the regenerated object in blue.
    pub(in crate::session) fn generator_knob_drag_move(&mut self, point: Point) -> bool {
        let Some(drag) = self.generators.knobs.drag.as_ref() else {
            return false;
        };
        let asked = dragged_mm(&drag.knob, drag.from, point, self.held.shift);
        let value = ParamValue::Length(Length::from_mm(asked));
        let plan = plan_value(
            &self.document,
            &self.manufacturing.extensions,
            drag.id,
            drag.knob.key,
            &value,
        );
        let (id, key) = (drag.id, drag.knob.key);
        if let Ok(plan) = plan {
            let shown = instance::find_kind(&self.manufacturing.extensions, &plan.record.kind)
                .and_then(|found| {
                    let param = found.kind.params().find(|param| param.key == key)?;
                    match param.read(&plan.record.params) {
                        ParamValue::Length(length) => Some(length.as_mm()),
                        _ => None,
                    }
                })
                .unwrap_or(asked);
            self.generators.preview = Some(Preview {
                id,
                record: plan.record,
                frame: plan.snapshot.frame,
                overlay: true,
            });
            if let Some(drag) = self.generators.knobs.drag.as_mut() {
                drag.asked = asked;
                drag.shown = shown;
            }
        }
        true
    }

    /// The release of a knob drag at `point`: one step "Gear: Pitch diameter".
    pub(in crate::session) fn generator_knob_drag_end(&mut self, point: Point) -> bool {
        self.generator_knob_drag_move(point);
        let Some(drag) = self.generators.knobs.drag.take() else {
            return false;
        };
        self.generators.preview = None;
        if (drag.shown - drag.knob.value_mm).abs() < 1e-9 {
            return true;
        }
        let name = self
            .document
            .generated(drag.id)
            .and_then(|snapshot| self.kind_label(&snapshot.record.kind))
            .map(|label| format!("{label}: {}", drag.knob.label));
        let value = ParamValue::Length(Length::from_mm(drag.asked));
        let _ = self.recording_named_guarded(name, |this| {
            change_value(
                &this.document,
                &this.manufacturing.extensions,
                drag.id,
                drag.knob.key,
                &value,
            )
            .is_ok()
        });
        true
    }

    /// Escape or a lost pointer: the knob drag ends and nothing is written.
    pub(in crate::session) fn cancel_generator_knob_drag(&mut self) -> bool {
        let was = self.generators.knobs.drag.take().is_some();
        if was {
            self.generators.preview = None;
        }
        was
    }

    /// The readout at the pointer while a knob is held: "Pitch diameter 48.0 mm", with "max" or
    /// "min" at a bound.
    pub(in crate::session) fn generator_knob_readout(
        &self,
    ) -> Option<super::super::shapes::LiveReadout> {
        let drag = self.generators.knobs.drag.as_ref()?;
        let mut text = format!("{} {:.1} mm", drag.knob.label, drag.shown);
        let gap = drag.asked - drag.shown;
        if gap > 5e-4 {
            text.push_str(" max");
        } else if gap < -5e-4 {
            text.push_str(" min");
        }
        Some(super::super::shapes::LiveReadout {
            text,
            anchor: self.pointer_position?,
        })
    }

    /// The knobs drawn over the transform handles: the "Parameter handle" glyph, filled while held.
    pub(in crate::session) fn generator_knobs_draw_list(&self) -> DrawList {
        let knobs = self.visible_knobs();
        if knobs.is_empty() {
            return DrawList::default();
        }
        let hovered = self
            .pointer_position
            .filter(|_| self.generators.knobs.drag.is_none())
            .and_then(|point| self.knob_at(&knobs, point));
        let held = self
            .generators
            .knobs
            .drag
            .as_ref()
            .map(|drag| drag.knob.key);
        let handles = knobs
            .iter()
            .enumerate()
            .map(|(index, knob)| TransformHandleGlyph {
                position: knob.at,
                kind: TransformGlyphKind::Parameter,
                dragging: held == Some(knob.key),
                hovered: hovered == Some(index),
            })
            .collect();
        build_transform_draw_list(
            self.view(),
            &TransformDecorationInput {
                handles,
                device_pixel_ratio: self.device_pixel_ratio,
                ..TransformDecorationInput::default()
            },
        )
    }
}
