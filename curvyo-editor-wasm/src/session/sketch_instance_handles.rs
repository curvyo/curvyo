//! The drag knobs of the one selected clone of a sketch (`specs/0099-sketch-pages` criteria 25,
//! 31, 33, 34, UX notes 12.3 and 12.9.4): which show, a press on one, the drag with its blue
//! preview and its partners following, the readout, and the one commit on release. The anchors
//! are `curvyo_ui_core::instance_sketch_handles`; the rules of what shows are shared with the
//! generator knobs (`generators/handles.rs`).

use std::cell::RefCell;

use curvyo_document_core::{Affine, NodeId, ObjectSnapshot, Point, ValueSet};
use curvyo_render_core::DrawList;
use curvyo_ui_core::instance_handles::{Knob, KnobMotion, dragged};
use curvyo_ui_core::{DimensionMarks, instance_sketch, instance_sketch_handles};

use super::{Session, Tool};

/// The knobs of a clone as its committed values place them, kept until the clone or the zoom
/// changes.
#[derive(Debug)]
struct KnobMemo {
    id: NodeId,
    outline: usize,
    composed: Affine,
    own: ValueSet,
    scale: f64,
    knobs: Vec<Knob>,
    /// Each knob's dimension marks on the clone, by key: the guides of hover and drag.
    guides: Vec<(String, DimensionMarks)>,
    /// The corners of the clone's box in document millimetres.
    bounds: (Point, Point),
}

/// A knob press in flight.
#[derive(Debug)]
struct KnobDrag {
    id: NodeId,
    knob: Knob,
    /// Where the press happened: the drag measures from it.
    from: Point,
    /// Shift at the press: only this value changes and it unlinks (criterion 34).
    only_this: bool,
    /// The other members of its link group that move with it, by key; empty when it changes alone.
    partners: Vec<String>,
    /// Whether it is in a link group at all (the chip's glyph).
    grouped: bool,
    /// What the pointer asks for and what the limits allow, in millimetres or degrees.
    asked: f64,
    shown: f64,
    /// Whether the last step had no solution (the preview holds the last solvable one).
    no_solution: bool,
    /// The knobs at the values of the last solvable step: the held one and its partners move.
    moved: Vec<Knob>,
    /// Their dimension marks, by key.
    moved_guides: Vec<(String, DimensionMarks)>,
}

/// The state of the sketch-clone knobs.
#[derive(Debug, Default)]
pub(super) struct SketchKnobs {
    memo: RefCell<Option<KnobMemo>>,
    drag: Option<KnobDrag>,
    /// The numeric entry chip a double-click on a knob opened (`sketch_knob_entry.rs`).
    pub(super) entry: Option<super::sketch_knob_entry::KnobEntry>,
}

/// The knobs and their guides by key, split.
fn split(pairs: Vec<(Knob, DimensionMarks)>) -> (Vec<Knob>, Vec<(String, DimensionMarks)>) {
    pairs
        .into_iter()
        .map(|(knob, marks)| {
            let key = knob.key.clone();
            (knob, (key, marks))
        })
        .unzip()
}

fn bounds_of(clone: &curvyo_document_core::CloneSnapshot) -> (Point, Point) {
    let mut bounds = (
        Point::new(f64::MAX, f64::MAX),
        Point::new(f64::MIN, f64::MIN),
    );
    for anchor in clone.resolved.all_anchors() {
        let at = anchor.point;
        bounds.0 = Point::new(bounds.0.x.min(at.x), bounds.0.y.min(at.y));
        bounds.1 = Point::new(bounds.1.x.max(at.x), bounds.1.y.max(at.y));
    }
    bounds
}

impl Session {
    /// The one selected clone of a sketch that exposes a value, with Select active.
    fn knob_clone(&self) -> Option<curvyo_document_core::CloneSnapshot> {
        let [id] = self.selection.ids()[..] else {
            return None;
        };
        if self.tool != Tool::Select {
            return None;
        }
        match self.object_read().get(id)? {
            ObjectSnapshot::Clone(clone) => Some(clone.clone()),
            _ => None,
        }
    }

    /// The knobs of the selected clone at its committed values and the corners of its box, from
    /// the memo while the clone and the zoom are the ones it was made for.
    fn raw_sketch_knobs(&self) -> Option<(NodeId, Vec<Knob>, (Point, Point))> {
        let clone = self.knob_clone()?;
        let sketch = self.document.sketch(clone.chain.root)?;
        if sketch.exposed.is_empty() {
            return None;
        }
        let view = self.view();
        let outline = std::sync::Arc::as_ptr(&clone.chain.root_outline) as usize;
        let mut memo = self.sketch_knobs.memo.borrow_mut();
        let stale = memo.as_ref().is_none_or(|held| {
            held.id != clone.id()
                || held.outline != outline
                || held.composed != clone.chain.composed
                || held.own != clone.chain.own_values
                || held.scale.to_bits() != view.scale().to_bits()
        });
        if stale {
            let solved = self.document.instance_solution(clone.id(), None)?;
            let (knobs, guides) = split(instance_sketch_handles::knobs_with_guides(
                &sketch, &solved, &clone, view,
            ));
            *memo = Some(KnobMemo {
                id: clone.id(),
                outline,
                composed: clone.chain.composed,
                own: clone.chain.own_values.clone(),
                scale: view.scale(),
                knobs,
                guides,
                bounds: bounds_of(&clone),
            });
        }
        let held = memo.as_ref()?;
        Some((held.id, held.knobs.clone(), held.bounds))
    }

    /// The guides of the committed knobs, by key.
    fn sketch_knob_guides(&self) -> Vec<(String, DimensionMarks)> {
        self.raw_sketch_knobs();
        self.sketch_knobs
            .memo
            .borrow()
            .as_ref()
            .map(|memo| memo.guides.clone())
            .unwrap_or_default()
    }

    /// The sketch-clone knobs drawn and hit right now.
    pub(super) fn visible_sketch_knobs(&self) -> Vec<Knob> {
        let Some((_, raw, bounds)) = self.raw_sketch_knobs() else {
            return Vec::new();
        };
        self.shown_knobs(raw, bounds, self.sketch_knobs.drag.is_some())
    }

    /// The other members of `key`'s link group on the selected clone that a coupled change moves:
    /// none when it is in no group or the clone holds it unlinked.
    pub(super) fn knob_partners(&self, id: NodeId, key: &str) -> (bool, Vec<String>) {
        let Some(ObjectSnapshot::Clone(clone)) = self.object_read().get(id).cloned() else {
            return (false, Vec::new());
        };
        let exposed = self.document.exposed_params(clone.chain.root);
        let Some(param) = exposed.iter().find(|param| param.key.as_str() == key) else {
            return (false, Vec::new());
        };
        if param.group.is_none() {
            return (false, Vec::new());
        }
        if clone.chain.unlinked.contains(&param.key) {
            return (true, Vec::new());
        }
        let partners = exposed
            .iter()
            .filter(|other| {
                other.group == param.group
                    && other.key != param.key
                    && !clone.chain.unlinked.contains(&other.key)
            })
            .map(|other| other.key.as_str().to_owned())
            .collect();
        (true, partners)
    }

    /// A press at `point` on a knob starts its drag; Shift held at the press (`shift`) changes only
    /// this value. `true` when it took the press.
    pub(super) fn sketch_knob_press(&mut self, point: Point, shift: bool) -> bool {
        let knobs = self.visible_sketch_knobs();
        let Some(index) = self.knob_at(&knobs, point) else {
            return false;
        };
        let Some((id, _, _)) = self.raw_sketch_knobs() else {
            return false;
        };
        let knob = knobs[index].clone();
        let (grouped, partners) = self.knob_partners(id, &knob.key);
        let moved_guides = self.sketch_knob_guides();
        self.instances = super::sketch_instance::InstanceState::default();
        self.sketch_knobs.drag = Some(KnobDrag {
            id,
            asked: knob.value,
            shown: knob.value,
            from: point,
            only_this: shift,
            partners: if shift { Vec::new() } else { partners },
            grouped,
            no_solution: false,
            moved: knobs,
            moved_guides,
            knob,
        });
        true
    }

    /// The pointer moved to `point` with a knob held: one plan for the clone (one solve for the
    /// whole link group), drawn blue over the committed clone; Ctrl or Cmd snaps whole units.
    pub(super) fn sketch_knob_drag_move(&mut self, point: Point) -> bool {
        let Some(drag) = self.sketch_knobs.drag.as_ref() else {
            return false;
        };
        let asked = dragged(&drag.knob, drag.from, point, self.held.ctrl);
        let (id, key, only_this) = (drag.id, drag.knob.key.clone(), drag.only_this);
        let Ok(plan) = instance_sketch::plan(&self.document, id, &key, asked, only_this) else {
            return true;
        };
        let parsed = curvyo_document_core::ExposedKey::parse(&key);
        let shown = parsed
            .as_ref()
            .and_then(|key| plan.own.get(key))
            .map_or(asked, curvyo_document_core::InstanceValue::number);
        let moved = plan.preview.as_ref().and_then(|preview| {
            let sketch = self.document.sketch(preview.chain.root)?;
            let solved = self.document.instance_solution(id, Some(&plan.own))?;
            Some(split(instance_sketch_handles::knobs_with_guides(
                &sketch,
                &solved,
                preview,
                self.view(),
            )))
        });
        let solved = plan.preview.is_some();
        if let Some(preview) = plan.preview {
            self.instances.last_solvable.insert(id, plan.own);
            self.instances.previews = vec![preview];
            self.instances.overlay = true;
        }
        if let Some(drag) = self.sketch_knobs.drag.as_mut() {
            drag.asked = asked;
            drag.shown = shown;
            drag.no_solution = !solved;
            if let Some((moved, guides)) = moved {
                drag.moved = moved;
                drag.moved_guides = guides;
            }
        }
        true
    }

    /// The release of a knob drag at `point`: one step "Change parameter", the row turns Own.
    pub(super) fn sketch_knob_drag_end(&mut self, point: Point) -> bool {
        if self.sketch_knobs.drag.is_none() {
            return false;
        }
        self.sketch_knob_drag_move(point);
        let Some(drag) = self.sketch_knobs.drag.take() else {
            return false;
        };
        let last: Vec<(NodeId, ValueSet)> = std::mem::take(&mut self.instances.last_solvable)
            .into_iter()
            .collect();
        self.instances = super::sketch_instance::InstanceState::default();
        if (drag.shown - drag.knob.value).abs() < 1e-9 {
            return true;
        }
        let _ = self.recording(|this| {
            instance_sketch::change(
                &this.document,
                &[drag.id],
                &drag.knob.key,
                drag.asked,
                drag.only_this,
                &last,
            )
            .is_ok_and(|changed| changed.written)
        });
        true
    }

    /// Escape or a lost pointer: the knob drag ends and nothing is written.
    pub(super) fn cancel_sketch_knob_drag(&mut self) -> bool {
        let was = self.sketch_knobs.drag.take().is_some();
        if was {
            self.instances = super::sketch_instance::InstanceState::default();
        }
        was
    }

    /// The readout chip at the pointer while a knob is held: "Width 82.0 mm", a coupled drag
    /// "Radius 1 = Radius 2 40.0 mm" (three or more: "Leg 1 = Leg 2 + 1 more 40.0 mm"), with
    /// " max" or " min" at a limit and " · no solution".
    pub(super) fn sketch_knob_readout(&self) -> Option<super::shapes::LiveReadout> {
        let drag = self.sketch_knobs.drag.as_ref()?;
        let angle = matches!(drag.knob.motion, KnobMotion::Around { .. });
        let labels: Vec<&str> = drag
            .moved
            .iter()
            .filter(|knob| drag.partners.contains(&knob.key))
            .map(|knob| knob.label.as_str())
            .collect();
        let names = match &labels[..] {
            [] => drag.knob.label.clone(),
            [one] => format!("{} = {one}", drag.knob.label),
            [one, rest @ ..] => format!("{} = {one} + {} more", drag.knob.label, rest.len()),
        };
        let mut text = if angle {
            format!("{names} {:.1}°", drag.shown)
        } else {
            format!("{names} {:.1} mm", drag.shown)
        };
        let gap = drag.asked - drag.shown;
        if gap > 5e-4 {
            text.push_str(" max");
        } else if gap < -5e-4 {
            text.push_str(" min");
        }
        if drag.no_solution {
            text.push_str(" · no solution");
        }
        Some(super::shapes::LiveReadout {
            text,
            anchor: self.pointer_position?,
        })
    }

    /// The glyph of the readout chip (UX notes 12.9.4): `link` for a coupled drag, `unlink` for a
    /// value in a group that changes alone, empty otherwise or with no knob held.
    #[must_use]
    pub fn sketch_knob_chip_glyph(&self) -> &'static str {
        match self.sketch_knobs.drag.as_ref() {
            Some(drag) if drag.grouped && drag.partners.is_empty() => "unlink",
            Some(drag) if !drag.partners.is_empty() => "link",
            _ => "",
        }
    }

    /// Whether a sketch-clone knob is held: the shell re-reads the Parameters row each move, so the
    /// dragged field and its partners follow the knob (criterion 33).
    #[must_use]
    pub fn sketch_knob_dragging(&self) -> bool {
        self.sketch_knobs.drag.is_some()
    }

    /// The sketch-clone knobs over the transform handles: the held one filled, its partners (and,
    /// on hover without Shift, the hovered knob's partners) on the follower ground.
    pub(super) fn sketch_knobs_draw_list(&self) -> DrawList {
        let shown = self.sketch_knob_guide_keys();
        let mut list = match self.sketch_knobs.drag.as_ref() {
            Some(drag) => self.knob_guides_list(&drag.moved_guides, &shown),
            None => self.knob_guides_list(&self.sketch_knob_guides(), &shown),
        };
        if let Some(drag) = self.sketch_knobs.drag.as_ref() {
            list.extend(
                self.knobs_draw_list(&drag.moved, Some(&drag.knob.key), |knob| {
                    drag.partners.contains(&knob.key)
                }),
            );
            return list;
        }
        let knobs = self.visible_sketch_knobs();
        list.extend(self.knobs_draw_list(&knobs, None, |knob| {
            shown.first() != Some(&knob.key) && shown.contains(&knob.key)
        }));
        list
    }

    /// The keys whose dimension guides show now, the held or hovered knob first: during a drag
    /// it and its moving partners; on hover it and, without Shift, its partners (UX notes 12.3,
    /// 12.9.4). A diagnostic for the session test as well.
    #[doc(hidden)]
    #[must_use]
    pub fn sketch_knob_guide_keys(&self) -> Vec<String> {
        if let Some(drag) = self.sketch_knobs.drag.as_ref() {
            return std::iter::once(drag.knob.key.clone())
                .chain(drag.partners.iter().cloned())
                .collect();
        }
        let knobs = self.visible_sketch_knobs();
        let Some(index) = self
            .pointer_position
            .and_then(|point| self.knob_at(&knobs, point))
        else {
            return Vec::new();
        };
        let key = knobs[index].key.clone();
        let partners = match self.selection.ids().first() {
            Some(id) if !self.held.shift => self.knob_partners(*id, &key).1,
            _ => Vec::new(),
        };
        std::iter::once(key).chain(partners).collect()
    }

    /// The extension lines and arrowheads of the dimensions whose key `shown` names, in
    /// `--dim-guide`, without text (UX notes 12.3, 12.9.4).
    fn knob_guides_list(&self, guides: &[(String, DimensionMarks)], shown: &[String]) -> DrawList {
        let mut list = DrawList::default();
        for (_, marks) in guides.iter().filter(|(key, _)| shown.contains(key)) {
            let arrows: Vec<(Point, curvyo_document_core::Vec2)> = marks
                .arrows
                .iter()
                .map(|arrow| (arrow.tip, arrow.back))
                .collect();
            list.extend(curvyo_render_core::build_dimension_marks(
                &curvyo_render_core::DimensionMarkDrawing {
                    lines: &marks.lines,
                    arrows: &arrows,
                    look: curvyo_render_core::DimensionLook::Guide,
                    dashed: false,
                    hovered: false,
                    selected: false,
                },
                self.view(),
            ));
        }
        list
    }

    /// The keys and places of the sketch-clone knobs drawn now: a diagnostic for the session and
    /// budget tests, which press them.
    #[doc(hidden)]
    #[must_use]
    pub fn sketch_knob_points(&self) -> Vec<(String, Point)> {
        self.visible_sketch_knobs()
            .into_iter()
            .map(|knob| (knob.key, knob.at))
            .collect()
    }
}
