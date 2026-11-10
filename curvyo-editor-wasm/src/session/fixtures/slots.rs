//! The slot commands of a holder (`specs/0102-laser-fixtures` criteria 19 and 20): add, remove,
//! rename, shape and the typed centre and size, each one step; a slot past its board is allowed
//! and named in a notice.

use curvyo_document_core::{
    FixtureParams, Length, MAX_SLOTS_PER_FIXTURE, NodeId, RENAME_SLOT_COMMIT_LABEL,
    SET_SLOT_POSITION_COMMIT_LABEL, SET_SLOT_SHAPE_COMMIT_LABEL, SET_SLOT_SIZE_COMMIT_LABEL,
};
use curvyo_laser_core::{FixtureKind, SLOT_CIRCLE, SLOT_RECT, Slot, SlotShape, read_slot};
use serde_json::{Value, json};

use super::super::Session;
use super::params::{slot_field, with_number};
use super::placed::{PlacedFixture, Shape};

/// The side of a new slot, in mm (criterion 19, Proposal).
const NEW_SLOT_MM: f64 = 20.0;

/// Whether `slot` lies past the edge of a board of `width` by `height` (criterion 20).
pub(super) fn slot_outside(slot: &Slot, width: Length, height: Length) -> bool {
    let (w, h) = slot.extent();
    let (hw, hh) = (w.as_mm() / 2.0, h.as_mm() / 2.0);
    let (cx, cy) = (slot.centre.x, slot.centre.y);
    // A slot touching the edge is on the board: 1e-6 mm of rounding is no "outside".
    let eps = 1e-6;
    cx - hw < -eps
        || cy - hh < -eps
        || cx + hw > width.as_mm() + eps
        || cy + hh > height.as_mm() + eps
}

impl Session {
    /// Add slot (criterion 19): "Slot N", a 20 x 20 mm rectangle centred on the board, selected
    /// in its row; one step "Add slot". `None` off a holder or at 50 slots.
    pub fn add_slot(&mut self, holder: NodeId) -> Option<NodeId> {
        let (_, fixture) = self.page_fixture(holder)?;
        let Shape::Board { width, height } = fixture.shape else {
            return None;
        };
        let stored = self.document.fixture(holder)?.slots.len();
        if stored >= MAX_SLOTS_PER_FIXTURE {
            return None;
        }
        let params = rect_params(
            width.as_mm() / 2.0,
            height.as_mm() / 2.0,
            NEW_SLOT_MM,
            NEW_SLOT_MM,
        );
        let slot =
            self.recording_unguarded(|this| this.document.add_slot(holder, "Slot", &params).ok())?;
        self.select_slot(slot);
        Some(slot)
    }

    /// Removes the slot `slot` (criterion 19): one step "Remove slot".
    pub fn remove_slot(&mut self, slot: NodeId) -> bool {
        if self.slot_holder(slot).is_none() {
            return false;
        }
        self.recording_unguarded(|this| this.document.remove_slot(slot).is_ok())
    }

    /// Renames the slot `slot`, up to 40 characters; one step "Rename slot".
    pub fn rename_slot(&mut self, slot: NodeId, name: &str) -> bool {
        let Some(params) = self.slot_params(slot) else {
            return false;
        };
        self.recording_unguarded(|this| {
            this.document
                .set_slot_labelled(slot, name, &params, RENAME_SLOT_COMMIT_LABEL)
                .unwrap_or(false)
        })
    }

    /// The Shape switch (`rect` or `circle`; UX notes §4.2): the size is kept, a circle's
    /// diameter the smaller side, a rectangle's sides the diameter.
    pub fn set_slot_shape(&mut self, slot: NodeId, shape: &str) -> bool {
        let Some(read) = self.slot_params(slot).as_ref().and_then(read_slot) else {
            return false;
        };
        let (cx, cy) = (read.centre.x, read.centre.y);
        let params = match (shape, read.shape) {
            (SLOT_CIRCLE, SlotShape::Rectangle) => {
                let d = read.width.as_mm().min(read.height.as_mm());
                json!({"shape": SLOT_CIRCLE, "cx_mm": cx, "cy_mm": cy, "d_mm": d})
            }
            (SLOT_RECT, SlotShape::Circle) => {
                let d = read.diameter.as_mm();
                Value::Object(rect_params(cx, cy, d, d))
            }
            _ => return false,
        };
        let Value::Object(params) = params else {
            return false;
        };
        self.write_slot(slot, &params, SET_SLOT_SHAPE_COMMIT_LABEL)
    }

    /// Sets the slot field `key` (`cx_mm`, `cy_mm`, `w_mm`, `h_mm`, `d_mm`), set to its nearest
    /// limit with a notice; a slot that ends past its board's edge is kept and named in the
    /// notice "Slot `name` lies outside `holder`." (criterion 20).
    pub fn set_slot_length(&mut self, slot: NodeId, key: &str, value: Length) -> bool {
        let (Some(field), Some(params)) = (slot_field(key), self.slot_params(slot)) else {
            return false;
        };
        let (kept, notice) = field.accept(value);
        self.fixtures.notice = notice;
        let params = with_number(&params, field.key, kept);
        let label = if matches!(field.key, "cx_mm" | "cy_mm") {
            SET_SLOT_POSITION_COMMIT_LABEL
        } else {
            SET_SLOT_SIZE_COMMIT_LABEL
        };
        self.write_slot(slot, &params, label)
    }

    fn write_slot(&mut self, slot: NodeId, params: &FixtureParams, label: &str) -> bool {
        let Some((holder, name)) = self.slot_holder(slot) else {
            return false;
        };
        let written = self.recording_unguarded(|this| {
            this.document
                .set_slot_labelled(slot, &name, params, label)
                .unwrap_or(false)
        });
        if let (true, Some(read), Shape::Board { width, height }) =
            (written, read_slot(params), &holder.shape)
            && slot_outside(&read, *width, *height)
        {
            self.fixtures.notice = Some(format!("Slot {name} lies outside {}.", holder.name));
        }
        written
    }

    /// The holder of `slot` on the active laser page and the slot's name.
    fn slot_holder(&self, slot: NodeId) -> Option<(PlacedFixture, String)> {
        let fixtures = self.active_fixtures()?;
        let holder = fixtures
            .fixtures
            .into_iter()
            .find(|f| f.kind == FixtureKind::Holder && self.slot_of(f.id, slot).is_some())?;
        let name = self.slot_of(holder.id, slot)?.name;
        Some((holder, name))
    }

    fn slot_of(&self, holder: NodeId, slot: NodeId) -> Option<curvyo_document_core::SlotSnapshot> {
        self.document
            .fixture(holder)?
            .slots
            .into_iter()
            .find(|s| s.id == slot)
    }

    fn slot_params(&self, slot: NodeId) -> Option<FixtureParams> {
        let (holder, _) = self.slot_holder(slot)?;
        Some(self.slot_of(holder.id, slot)?.params)
    }
}

/// The stored values of a rectangle slot.
fn rect_params(cx: f64, cy: f64, w: f64, h: f64) -> FixtureParams {
    let Value::Object(map) =
        json!({"shape": SLOT_RECT, "cx_mm": cx, "cy_mm": cy, "w_mm": w, "h_mm": h})
    else {
        return FixtureParams::new();
    };
    map
}
