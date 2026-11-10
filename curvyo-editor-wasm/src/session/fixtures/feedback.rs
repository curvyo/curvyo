//! The pointer feedback of the selected fixture's handles, as the rectangle gives it
//! (`specs/0102-laser-fixtures` criteria 14, 18, 22, 23, 38; UX notes §0, §4.4, §7.1): the
//! cursor over a handle or the box, the live readout at the pointer, and the drag ghost.

use curvyo_document_core::{Angle, Vec2};
use curvyo_laser_core::StopPart;
use curvyo_render_core::DrawList;
use curvyo_ui_core::{ResizeDirection, format_degrees, resize_cursor_angle_degrees};

use super::super::Session;
use super::super::move_indicators::move_readout_text;
use super::super::shapes::LiveReadout;
use super::handles::{FixtureHandle, fixture_box, hit_fixture_handle};
use super::hit::inside;
use super::placed::Shape;

/// The cursor of `handle` on a box turned by `angle`: the rectangle's names.
fn cursor_of(handle: FixtureHandle, angle: Angle) -> Option<String> {
    let resize = |direction| {
        format!(
            "resize:{:.1}",
            resize_cursor_angle_degrees(direction, angle)
        )
    };
    Some(match handle {
        FixtureHandle::Resize(direction) => resize(direction),
        FixtureHandle::Leg(StopPart::LegA) => resize(ResizeDirection::E),
        FixtureHandle::Leg(_) => resize(ResizeDirection::S),
        FixtureHandle::Rotate(_) => "rotate".to_owned(),
        FixtureHandle::Move => "move".to_owned(),
        FixtureHandle::Thickness | FixtureHandle::Head | FixtureHandle::Thread => {
            "pointer".to_owned()
        }
        FixtureHandle::Hold => return None,
    })
}

impl Session {
    /// The cursor over the selected fixture: its dragged handle, the handle under the pointer, or
    /// "move" inside its box (D13); `None` elsewhere, for the objects' own cursor.
    pub(in crate::session) fn fixture_cursor(&self) -> Option<String> {
        if let Some(drag) = self.fixtures.drag.as_ref() {
            let angle = fixture_box(&drag.fixture, &drag.page).angle;
            return cursor_of(drag.handle, angle);
        }
        let pointer = self.pointer_position?;
        let (fixtures, fixture) = self
            .selected_fixture()
            .and_then(|id| self.page_fixture(id))?;
        if !super::commands::movable(&fixture) {
            return None;
        }
        let box_ = fixture_box(&fixture, &fixtures.page);
        let tolerances = self.transform_handle_tolerances();
        if let Some(handle) = hit_fixture_handle(
            &fixture,
            &fixtures.page,
            pointer,
            &tolerances,
            self.held.shift,
        ) {
            return cursor_of(handle, box_.angle);
        }
        inside(&fixture.corners(&fixtures.page), pointer).then(|| "move".to_owned())
    }

    /// The live readout of a fixture drag at the pointer: "Δ x, y mm" for a move, the angle for a
    /// turn, "W × H mm" for a resize or a leg, "t 5.0 mm" for the thickness, "⌀ 6.0 mm" for a
    /// screw knob ("max" for a thread at its head, "min" for a head at its thread, D29).
    pub(in crate::session) fn fixture_readout(&self) -> Option<LiveReadout> {
        let drag = self.fixtures.drag.as_ref()?;
        let edit = drag.live.as_ref()?;
        let text = match (drag.handle, &edit.shape) {
            (FixtureHandle::Move, _) => {
                let (from, to) = (drag.fixture.matrix, edit.matrix);
                move_readout_text(Vec2::new(to.e - from.e, to.f - from.f), false)
            }
            (FixtureHandle::Rotate(_), _) => {
                format_degrees(edit.matrix.b.atan2(edit.matrix.a).to_degrees())
            }
            (FixtureHandle::Thickness, Shape::Stop { thickness, .. }) => {
                format!("t {:.1} mm", thickness.as_mm())
            }
            (_, Shape::Board { width, height } | Shape::Clamp { width, height }) => {
                format!("{:.1} × {:.1} mm", width.as_mm(), height.as_mm())
            }
            (_, Shape::Stop { leg_a, leg_b, .. }) => {
                format!("{:.1} × {:.1} mm", leg_a.as_mm(), leg_b.as_mm())
            }
            (handle, Shape::Screw { thread, head }) => {
                let at_limit = (thread.as_mm() - head.as_mm()).abs() < 1e-9;
                let (value, limit) = match handle {
                    FixtureHandle::Thread => (thread, " max"),
                    FixtureHandle::Head => (head, " min"),
                    _ => return None,
                };
                let limit = if at_limit { limit } else { "" };
                format!("⌀ {:.1} mm{limit}", value.as_mm())
            }
            _ => return None,
        };
        Some(LiveReadout {
            text,
            anchor: drag.pointer,
        })
    }

    /// The outline the dragged fixture had at the press, as a ghost under its live place.
    pub(super) fn fixture_ghost(&self) -> Option<DrawList> {
        let drag = self.fixtures.drag.as_ref()?;
        drag.live.as_ref()?;
        let outline = drag.fixture.outline(&drag.page);
        Some(curvyo_render_core::build_fixture_ghost(
            &outline,
            self.view(),
        ))
    }
}
