//! One text put on several carriers (`specs/0080-text-on-path` slice 2, criterion 37; UX notes
//! §9): the pick set, its look on the canvas with the "1" badge on the first carrier, and the put
//! that gives the text the first carrier and a text clone each further one.

use std::sync::Arc;

use curvyo_document_core::{
    CarrierCurve, CarrierRef, NodeId, ObjectSnapshot, OnPathValues, TextOnPath, ViewTransform,
};
use curvyo_render_core::DrawList;

use super::Session;

impl Session {
    /// The outline `carrier` gives a text, read from the frame's document read.
    fn picked_curve(&self, carrier: CarrierRef) -> Option<CarrierCurve> {
        self.document_read()
            .leaves
            .iter()
            .find(|o| o.id() == carrier.node)
            .and_then(|o| curvyo_document_core::carrier_curve(o, carrier.outline))
    }

    /// Adds `carrier` to the pick set, or takes it out when it is in it (Shift+click, UX notes
    /// §9).
    pub(super) fn toggle_picked(&mut self, carrier: CarrierRef) {
        let Some(pick) = self.text_path.pick.as_mut() else {
            return;
        };
        if let Some(at) = pick.set.iter().position(|c| *c == carrier) {
            pick.set.remove(at);
        } else {
            pick.set.push(carrier);
        }
    }

    /// Enter in pick mode (criterion 2): puts the text on the pick set and ends pick mode; with
    /// an empty set pick mode ends without a change. Returns whether pick mode ran.
    pub fn confirm_text_path_pick(&mut self) -> bool {
        self.recording(|this| {
            let Some(pick) = this.text_path.pick.take() else {
                return false;
            };
            this.text_path.hover = None;
            match pick.set.as_slice() {
                [] => {}
                [one] => this.put_on(pick.text, *one),
                set => this.put_on_set(pick.text, set),
            }
            this.resume_after_pick(Some(&pick));
            true
        })
    }

    /// The carriers selected with text `text`, lowest in stacking order first (criterion 37).
    pub(super) fn selected_carriers(&self, text: NodeId) -> Vec<CarrierRef> {
        let read = self.object_read();
        let mut ids: Vec<NodeId> = self
            .selection
            .ids()
            .iter()
            .copied()
            .filter(|id| *id != text && self.is_carrier(*id))
            .collect();
        ids.sort_by_key(|id| read.index.get(id).copied());
        ids.into_iter()
            .map(|node| CarrierRef { node, outline: 0 })
            .collect()
    }

    /// The values the text gets on each carrier: on the first as [`Session::put_values`], on each
    /// further one the defaults of criterion 5 (a new text clone).
    fn set_values(&self, id: NodeId, carriers: &[CarrierRef]) -> Vec<(CarrierRef, OnPathValues)> {
        let Some(text) = self.read_text(id) else {
            return Vec::new();
        };
        let mut straight = text.clone();
        straight.on_path = None;
        carriers
            .iter()
            .enumerate()
            .filter_map(|(i, carrier)| {
                let curve = self.picked_curve(*carrier)?;
                let base = if i == 0 { &text } else { &straight };
                Some((*carrier, self.put_values(base, &curve)))
            })
            .collect()
    }

    /// Puts text `id` on `carriers` as one step "Put text on paths" (criterion 37): the text on
    /// the first, a text clone on each further one; selects the text and posts the notice.
    pub(super) fn put_on_set(&mut self, id: NodeId, carriers: &[CarrierRef]) {
        let pairs = self.set_values(id, carriers);
        if pairs.is_empty() {
            return;
        }
        if self.document.put_text_on_paths(id, &pairs, true).is_ok() {
            self.selection.select_single(id);
            self.text_path.announcement = Some("Put on paths");
            self.text_path.notice = Some(format!("Put text on {} paths.", pairs.len()));
        }
    }

    /// Takes the canvas notice of the last put on several carriers ("Put text on 3 paths."), or
    /// the empty string.
    pub fn take_text_path_notice(&mut self) -> String {
        self.text_path.notice.take().unwrap_or_default()
    }

    /// The pick set on the canvas (UX notes §9): each picked carrier in the selected look with
    /// the text previewed on it at the values it would get.
    pub(super) fn pick_set_list(&self, view: ViewTransform) -> DrawList {
        let mut list = DrawList::default();
        let Some(pick) = self.text_path.pick.as_ref() else {
            return list;
        };
        let Some(text) = self.read_text(pick.text) else {
            return list;
        };
        let mut previews = Vec::new();
        for (carrier, values) in self.set_values(pick.text, &pick.set) {
            let Some(curve) = self.picked_curve(carrier) else {
                continue;
            };
            list.extend(curvyo_render_core::build_carrier_picked(&curve, view));
            let mut shown = text.clone();
            shown.on_path = Some(TextOnPath {
                carrier,
                values,
                curve: Some(Arc::new(curve)),
                kept: false,
                kept_curve: None,
            });
            shown.outline = self.fonts.lay_out(&shown).map(Arc::new);
            previews.push(ObjectSnapshot::Text(Box::new(shown)));
        }
        list.extend(curvyo_render_core::build_live_edit_preview(&previews, view));
        list
    }

    /// Where the "1" badge of the pick set sits (UX notes §9): the start of the first picked
    /// carrier, canvas px; `None` outside pick mode or with an empty set.
    #[must_use]
    pub fn text_path_pick_badge(&self) -> Option<(f64, f64)> {
        let first = *self.text_path.pick.as_ref()?.set.first()?;
        let start = self.picked_curve(first)?.anchors.first()?.point;
        Some(self.view().document_to_screen(start))
    }
}
