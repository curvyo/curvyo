//! Pick mask paths, the Trace tool's pick mode (`specs/0094-masked-composite-trace` slice 1b,
//! criteria 34 to 43, `ux-notes.md` §11): while it is armed, a press adds or removes the topmost
//! mask-eligible object under it by the Shift+click route of the tool, the traced image never hit;
//! a text, a sketch or a second image with nothing eligible under it is refused; the mode stays.

use curvyo_document_core::{NodeId, ObjectSnapshot, Point};
use curvyo_ui_core::{hit_test_objects, mask_operand, object_outline_bounds};
use kurbo::{BezPath, Shape};

use super::trace_region::is_mask_path;
use super::{Session, Tool};

/// What the pick mode holds; part of the Trace tool's state, so leaving the tool disarms it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct MaskPick {
    armed: bool,
    /// The last press hit an object that cannot be a mask path: the bar's notice row says so
    /// until the next accepted pick or the end of the mode.
    refused: bool,
}

/// What an armed press on an object does.
enum PickTarget {
    /// A path-like object or a group of them: added or removed.
    Mask(NodeId),
    /// A text, a sketch, a second image or a group holding one: not added, the row shows.
    Refused,
}

impl Session {
    /// Arms (`true`) or disarms the pick mode; only the Trace tool arms it, and arming it
    /// disarms the pipette (plan, Proposal).
    pub fn set_trace_mask_pick(&mut self, armed: bool) {
        let armed = armed && self.tool == Tool::Trace;
        if armed {
            self.set_trace_pick(None);
        }
        self.trace.mask_pick = MaskPick {
            armed,
            refused: false,
        };
        self.hovered_object = None;
    }

    /// Whether the pick mode is armed.
    #[must_use]
    pub const fn trace_mask_pick(&self) -> bool {
        self.trace.mask_pick.armed
    }

    /// Whether the last armed press was refused (criterion 38).
    #[must_use]
    pub const fn trace_mask_pick_refused(&self) -> bool {
        self.trace.mask_pick.refused
    }

    /// Clear mask (criterion 40): only the traced image stays selected; the mask paths stay in
    /// the document, nothing is written and the mode is unchanged.
    pub fn clear_trace_mask(&mut self) {
        if self.tool != Tool::Trace {
            return;
        }
        let images = self.trace_view().images;
        if let [image] = images.as_slice() {
            self.selection.set(&[*image]);
        }
    }

    /// The topmost mask-eligible object under `point`, the traced `images` never hit, above or
    /// below the image: a closed outline by its interior, filled or not, an open one by its stroke
    /// (criteria 35 and 36). Checked cheapest first, as the hover asks on every move.
    pub(super) fn trace_mask_pick_eligible(
        &self,
        point: Point,
        images: &[NodeId],
    ) -> Option<NodeId> {
        let tolerance = self.object_tolerance();
        let units = self.units();
        let objects = self.objects();
        let eligible = |hit: NodeId| {
            units.leaves_with(&hit).iter().all(|member| {
                objects
                    .iter()
                    .find(|object| object.id() == *member)
                    .is_some_and(is_mask_path)
            })
        };
        self.pickable_objects()
            .iter()
            .rev()
            .filter(|object| !images.contains(&object.id()) && is_mask_path(object))
            .find(|object| {
                (!hit_test_objects(std::slice::from_ref(*object), point, tolerance).is_empty()
                    || encloses(object, point))
                    && eligible(object.id())
            })
            .map(ObjectSnapshot::id)
    }

    /// What a press at `point` picks: the eligible object under it; without one, a refusal when
    /// the topmost other object hit there, the traced `images` left out, is a text, a sketch, a
    /// second image or a group holding one; `None` over the image alone or empty canvas
    /// (criterion 38).
    fn trace_mask_pick_target(&self, point: Point, images: &[NodeId]) -> Option<PickTarget> {
        if let Some(hit) = self.trace_mask_pick_eligible(point, images) {
            return Some(PickTarget::Mask(hit));
        }
        // A page's sketch is not pickable (ADR 0043 §2) but is still refused where it is drawn.
        let fixed = self
            .objects()
            .iter()
            .filter(|o| {
                matches!(o, ObjectSnapshot::Sketch(_)) && self.document.is_fixed_sketch(o.id())
            })
            .cloned()
            .collect::<Vec<_>>();
        let others: Vec<ObjectSnapshot> = self
            .pickable_objects()
            .iter()
            .filter(|object| !images.contains(&object.id()))
            .cloned()
            .chain(fixed)
            .collect();
        (!hit_test_objects(&others, point, self.object_tolerance()).is_empty())
            .then_some(PickTarget::Refused)
    }

    /// A press while armed: adds or removes the mask path under it (the Shift+click route),
    /// refuses what cannot be one, ignores the image and empty canvas. Returns whether the mode
    /// took the press; without a traced image it does not.
    pub(super) fn trace_mask_pick_press(&mut self, point: Point) -> bool {
        if !self.trace.mask_pick.armed {
            return false;
        }
        let images = self.trace_view().images;
        if images.is_empty() {
            return false;
        }
        match self.trace_mask_pick_target(point, &images) {
            Some(PickTarget::Mask(hit)) => {
                self.trace_select_hit(hit, true);
                self.trace.mask_pick.refused = false;
            }
            Some(PickTarget::Refused) => self.trace.mask_pick.refused = true,
            None => {}
        }
        true
    }

    /// The pointer moved while armed: the object a press would add or remove gets the hover box
    /// (UX notes §11.4).
    pub(super) fn trace_mask_pick_hover(&mut self, point: Point) {
        if !self.trace.mask_pick.armed {
            return;
        }
        let images = self.trace_view().images;
        self.hovered_object = self.trace_mask_pick_eligible(point, &images);
    }
}

/// Whether a closed outline of `object` holds `point` by the nonzero rule, filled or not: a mask
/// path is a region, so an outline-only circle is picked inside (a hole of a compound path is not).
fn encloses(object: &ObjectSnapshot, point: Point) -> bool {
    let (low, high) = object_outline_bounds(object);
    if point.x < low.x || point.x > high.x || point.y < low.y || point.y > high.y {
        return false;
    }
    let at = |p: Point| kurbo::Point::new(p.x, p.y);
    let mut path = BezPath::new();
    let Ok(operand) = mask_operand(object) else {
        return false;
    };
    for (anchors, _) in operand
        .iter()
        .filter(|(anchors, closed)| *closed && anchors.len() >= 2)
    {
        path.move_to(at(anchors[0].0));
        for (i, &(from, _, out)) in anchors.iter().enumerate() {
            let (to, into, _) = anchors[(i + 1) % anchors.len()];
            path.curve_to(at(from.translated(out)), at(to.translated(into)), at(to));
        }
        path.close_path();
    }
    path.winding(at(point)) != 0
}
