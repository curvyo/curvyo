//! Pick mode of text on path (`specs/0080-text-on-path` slice 1): the On path press, the carrier
//! under the pointer, the put of a text on a carrier.

use std::sync::Arc;

use curvyo_document_core::{
    CarrierCurve, CarrierRef, Length, NodeId, ObjectSnapshot, OnPathValues, Point, TextSnapshot,
};
use curvyo_ui_core::measure_of;

use super::{Session, Tool};

/// Pick mode: the text to put, the carriers picked with Shift (slice 2) and where edit mode
/// resumes after it.
#[derive(Debug, Clone)]
pub(super) struct Pick {
    pub(super) text: NodeId,
    /// The caret of the edit mode pick mode was started from (UX notes §0.4).
    pub(super) resume: Option<usize>,
    /// The pick set in the order picked; the first gets the text, the others text clones
    /// (criterion 37).
    pub(super) set: Vec<CarrierRef>,
}

/// What a press of On path did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextPathPress {
    /// Nothing: no single text is the subject.
    Ignored,
    /// Pick mode started.
    Picking,
    /// Pick mode ended without a change (a second press).
    Ended,
    /// The text was put on the selected carrier at once (criterion 3).
    Put,
    /// The text is locked: the Pen's lock notice is posted (criterion 7).
    Locked,
}

impl Session {
    /// Whether pick mode runs.
    #[must_use]
    pub fn text_path_picking(&self) -> bool {
        self.text_path.pick.is_some()
    }

    /// Takes the polite announcement of the last put ("Put on path").
    pub fn take_text_path_announcement(&mut self) -> Option<&'static str> {
        self.text_path.announcement.take()
    }

    /// The On path button (criteria 1 to 3, 7; UX notes §2): with exactly one text as subject and
    /// carriers selected with it (Select tool), the text goes on the first of them at once;
    /// otherwise pick mode starts (a second press ends it). From edit mode, edit mode commits
    /// first and resumes after the put.
    pub fn text_path_press(&mut self) -> TextPathPress {
        self.recording(|this| {
            if let Some(pick) = this.text_path.pick.take() {
                this.text_path.hover = None;
                // A second press puts the text on the pick set (criterion 2).
                if !pick.set.is_empty() {
                    this.put_on_set(pick.text, &pick.set);
                    this.resume_after_pick(Some(&pick));
                    return TextPathPress::Put;
                }
                this.resume_after_pick(None);
                return TextPathPress::Ended;
            }
            // From edit mode, edit mode commits first (a new text is created) and resumes after
            // the put (UX notes §0.4).
            let editing = this.text.edit.as_ref().map(curvyo_ui_core::TextEdit::caret);
            if let Some(edit) = this.text.edit.as_ref()
                && (edit.string().is_empty() || this.bar_texts().len() != 1)
            {
                return TextPathPress::Ignored;
            }
            if editing.is_some() {
                this.finish_text_edit_inner();
            }
            let texts = this.bar_texts();
            let [text] = texts.as_slice() else {
                return TextPathPress::Ignored;
            };
            if this.text_locked(text.id) {
                let pick = editing.map(|caret| Pick {
                    text: text.id,
                    resume: Some(caret),
                    set: Vec::new(),
                });
                this.resume_after_pick(pick.as_ref());
                return TextPathPress::Locked;
            }
            let carriers = this.selected_carriers(text.id);
            if this.tool == Tool::Select && editing.is_none() && !carriers.is_empty() {
                let id = text.id;
                if let [carrier] = carriers.as_slice() {
                    this.put_on(id, *carrier);
                } else {
                    this.put_on_set(id, &carriers);
                }
                return TextPathPress::Put;
            }
            let id = text.id;
            let resume = editing;
            this.text_path.pick = Some(Pick {
                text: id,
                resume,
                set: Vec::new(),
            });
            TextPathPress::Picking
        })
    }

    /// Ends pick mode without a change (Escape, a right click, a tool switch). Returns whether it
    /// ran.
    pub fn cancel_text_path_pick(&mut self) -> bool {
        let Some(pick) = self.text_path.pick.take() else {
            return false;
        };
        self.text_path.hover = None;
        self.resume_after_pick(Some(&pick));
        true
    }

    pub(super) fn resume_after_pick(&mut self, pick: Option<&Pick>) {
        if let Some(&Pick {
            text,
            resume: Some(caret),
            ..
        }) = pick
        {
            self.edit_text_with_caret(text, caret);
        }
    }

    /// Whether the text `id` is locked, by its own flag or an ancestor's; the lock notice is
    /// posted when it is (criterion 7).
    fn text_locked(&mut self, id: NodeId) -> bool {
        let read = self.document_read();
        let Some(node) = read.nodes.iter().find(|n| n.id == id) else {
            return false;
        };
        if !node.locked {
            return false;
        }
        let holder = node.locked_by.unwrap_or(id);
        let name = read
            .nodes
            .iter()
            .find(|n| n.id == holder)
            .and_then(|n| n.registers.name.clone())
            .unwrap_or_default();
        self.layers.notice = Some((curvyo_ui_core::draw_refused_notice(&name, true), true));
        true
    }

    /// Whether `id` is an object a text can be put on: a path, a primitive or a resolved clone.
    pub(super) fn is_carrier(&self, id: NodeId) -> bool {
        self.document_read()
            .leaves
            .iter()
            .any(|o| o.id() == id && !matches!(o, ObjectSnapshot::Text(_)))
    }

    /// The values a text gets on `carrier`: a text already on a path keeps Side, Line, Distance
    /// and Mirror letters (UX notes §0.5); Position is the default of criterion 5.
    pub(super) fn put_values(&self, text: &TextSnapshot, curve: &CarrierCurve) -> OnPathValues {
        let mut values = text
            .on_path
            .as_ref()
            .map_or(OnPathValues::at(Length::from_mm(0.0)), |p| p.values);
        let mut placed = text.clone();
        placed.on_path = Some(curvyo_document_core::TextOnPath {
            carrier: CarrierRef {
                node: text.id,
                outline: 0,
            },
            values,
            curve: Some(Arc::new(curve.clone())),
            kept: false,
            kept_curve: None,
        });
        if let Some(face) = self.face_of(&text.font)
            && let Some(laid) = curvyo_ui_core::text_layout(&placed, face)
        {
            values.position = curvyo_ui_core::default_position(&laid, &measure_of(curve));
        }
        values
    }

    /// Puts text `id` on `carrier` as one step, selects the text and ends pick mode.
    pub(super) fn put_on(&mut self, id: NodeId, carrier: CarrierRef) {
        let Some(text) = self.read_text(id) else {
            return;
        };
        let Some(curve) = self
            .document_read()
            .leaves
            .iter()
            .find(|o| o.id() == carrier.node)
            .and_then(|o| curvyo_document_core::carrier_curve(o, carrier.outline))
        else {
            return;
        };
        let values = self.put_values(&text, &curve);
        if self
            .document
            .put_text_on_path(id, carrier, &values, true)
            .is_ok()
        {
            self.selection.select_single(id);
            self.text_path.announcement = Some("Put on path");
        }
    }

    /// The carrier under `point` in pick mode: the deepest visible path, shape or clone, a guide
    /// by its line only, a compound path's outline nearest the point (UX notes §2).
    pub(super) fn carrier_at(&self, point: Point, exclude: NodeId) -> Option<CarrierRef> {
        let read = self.document_read();
        let candidates: Vec<ObjectSnapshot> = read
            .visible
            .iter()
            .filter(|o| !matches!(o, ObjectSnapshot::Text(_)) && o.id() != exclude)
            .map(|o| {
                let mut o = o.clone();
                if read.guides.contains(&o.id()) {
                    o.style_mut().fill.enabled = false;
                }
                o
            })
            .collect();
        let id = curvyo_ui_core::hit_test_object(&candidates, point, self.object_tolerance())?;
        let object = candidates.iter().find(|o| o.id() == id)?;
        let outline = match object.drawn_path() {
            Some(path) if path.is_compound() => curvyo_ui_core::nearest_outline(path, point),
            _ => 0,
        };
        Some(CarrierRef { node: id, outline })
    }

    /// A press in pick mode: with Shift it adds the carrier under the pointer to the pick set or
    /// takes it out (criterion 2); without, it puts the text on the pick set and that carrier
    /// (one step); elsewhere nothing.
    pub(super) fn pick_press(&mut self, point: Point, shift: bool) {
        let Some(pick) = self.text_path.pick.as_mut() else {
            return;
        };
        let text = pick.text;
        let Some(carrier) = self.carrier_at(point, text) else {
            return;
        };
        if shift {
            self.toggle_picked(carrier);
            return;
        }
        let Some(mut pick) = self.text_path.pick.take() else {
            return;
        };
        self.text_path.hover = None;
        if !pick.set.contains(&carrier) {
            pick.set.push(carrier);
        }
        if let [one] = pick.set.as_slice() {
            self.put_on(text, *one);
        } else {
            self.put_on_set(text, &pick.set);
        }
        self.resume_after_pick(Some(&pick));
    }

    /// Edit mode on text `id` with the caret at byte `caret`.
    fn edit_text_with_caret(&mut self, id: NodeId, caret: usize) {
        self.edit_text_at(id, None);
        if let Some(edit) = self.text.edit.as_mut() {
            let caret = caret.min(edit.string().len());
            let _ = edit.apply(
                curvyo_ui_core::TextInput::Select {
                    anchor: caret,
                    caret,
                },
                None,
            );
        }
    }
}
