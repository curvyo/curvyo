//! The texts over the canvas the host places in the DOM (`specs/0102-laser-fixtures` UX notes
//! §5.1, §5.2; `specs/0113-fixture-sets-and-pins` criterion 32): tape numbers, names, the tag of
//! the selected fixture and the lock badge, at screen coordinates in CSS px.

use curvyo_document_core::{Page, Point, Vec2};
use curvyo_laser_core::{GridSpec, TapeEdge};
use serde::Serialize;

use super::super::Session;
use super::draw::slot_label_fits;
use super::placed::PlacedFixture;
use super::shape::Shape;

/// A text over the canvas the host places in the DOM: a tape number, a fixture's or a slot's
/// name, or the tag of the selected fixture, at screen coordinates in CSS px.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FixtureLabel {
    /// `tape`, `fixture`, `slot`, `tag` (the selected fixture's name chip) or `lock` (the lock
    /// badge of a locked job fixture, at its box's upper left, `0113` criterion 32).
    pub kind: &'static str,
    /// The text.
    pub text: String,
    /// Screen x.
    pub x: f64,
    /// Screen y.
    pub y: f64,
    /// For a tape number, the edge its band lies on: `bottom`, `left`, `top` or `right`. Absent
    /// for every other label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge: Option<&'static str>,
}

/// The stored name of a tape's edge (the tape switches' names).
const fn edge_name(edge: TapeEdge) -> &'static str {
    match edge {
        TapeEdge::Bottom => "bottom",
        TapeEdge::Left => "left",
        TapeEdge::Top => "top",
        TapeEdge::Right => "right",
    }
}

/// The step between two tape numbers at `device_px_per_mm`: 50 mm, or the next of 100, 200,
/// 500, 1,000 mm ... that leaves 40 device px between them (UX notes D16).
pub(super) fn number_step(device_px_per_mm: f64) -> i64 {
    let mut step = 50;
    let mut k = 0;
    #[allow(clippy::cast_precision_loss)] // steps stay far below 2^52
    while (step as f64) * device_px_per_mm < 40.0 && step < 1_000_000 {
        step = [100, 200, 500][k % 3] * 10_i64.pow(u32::try_from(k / 3).unwrap_or(0));
        k += 1;
    }
    step
}

impl Session {
    /// The texts over the canvas: the tape numbers of the active laser page's shown grid at
    /// their 1-2-5 step, the names of boards and clamps 80 device px wide and of slots 40 device
    /// px across (UX notes §5.1, §5.2).
    #[must_use]
    pub fn fixture_labels(&self) -> Vec<FixtureLabel> {
        let Some(fixtures) = self.active_fixtures() else {
            return Vec::new();
        };
        let view = self.view();
        let device = view.scale() * self.device_pixel_ratio.max(f64::MIN_POSITIVE);
        let step = number_step(device);
        let origin = fixtures.page.origin;
        let screen = |p: curvyo_document_core::Point| view.document_to_screen(p);
        let mut out = Vec::new();
        let selected = self.selected_fixture();
        // A filled slot hides its name: the label would show through the artwork (`0154` V16).
        let filled = self.filled_slots();
        for fixture in fixtures.fixtures.iter().filter(|f| f.shown()) {
            let is_selected = selected == Some(fixture.id);
            if let Shape::Grid(spec) = fixture.shape {
                out.extend(self.tape_labels(spec, &fixtures.page, step));
                if is_selected {
                    let (x, y) = screen(origin);
                    out.push(tag(fixture, x, y));
                }
                continue;
            }
            // The lock badge of a locked job fixture, whether it is selected or not (`0113`
            // criterion 32, UX notes 7). It yields the upper right to the job chips (`0103`)
            // and stands outside the box's upper left corner, left of the tag (Proposal).
            if fixture.locked() {
                out.push(self.lock_badge(fixture, &fixtures.page));
            }
            let (w, _) = fixture.local_size(&fixtures.page);
            let named = matches!(fixture.shape, Shape::Board { .. } | Shape::Clamp { .. })
                && w * device >= 80.0;
            if is_selected || named {
                let (x, y) = screen(fixture.corners(&fixtures.page)[0]);
                out.push(if is_selected {
                    tag(fixture, x, y)
                } else {
                    FixtureLabel {
                        kind: "fixture",
                        text: fixture.name.clone(),
                        x,
                        y,
                        edge: None,
                    }
                });
            }
            for slot in fixture
                .slots
                .iter()
                .filter(|s| slot_label_fits(&s.slot, device) && !filled.contains(&s.id))
            {
                let (x, y) = screen(fixture.global.apply_point(slot.slot.centre));
                out.push(FixtureLabel {
                    kind: "slot",
                    text: slot.name.clone(),
                    x,
                    y,
                    edge: None,
                });
            }
        }
        out
    }

    /// The tape numbers of the grid `spec` on `page` at every `step` mm, each on its edge.
    fn tape_labels(&self, spec: GridSpec, page: &Page, step: i64) -> Vec<FixtureLabel> {
        let view = self.view();
        let to_document = Vec2::new(page.origin.x, page.origin.y);
        let grid = self.fixtures.grid_cache.layout(spec, page.size);
        let mut out = Vec::new();
        for tape in &grid.tapes {
            for number in tape.numbers.iter().filter(|n| n.mm.rem_euclid(step) == 0) {
                let (x, y) = view.document_to_screen(number.at.translated(to_document));
                out.push(FixtureLabel {
                    kind: "tape",
                    text: number.mm.to_string(),
                    x,
                    y,
                    edge: Some(edge_name(tape.edge)),
                });
            }
        }
        out
    }

    /// The lock badge of the locked job fixture `fixture`, outside its box's upper left corner.
    fn lock_badge(&self, fixture: &PlacedFixture, page: &Page) -> FixtureLabel {
        let corners = fixture.corners(page);
        let left = corners.iter().map(|c| c.x).fold(f64::INFINITY, f64::min);
        let top = corners.iter().map(|c| c.y).fold(f64::INFINITY, f64::min);
        let (x, y) = self.view().document_to_screen(Point::new(left, top));
        FixtureLabel {
            kind: "lock",
            text: String::new(),
            x,
            y,
            edge: None,
        }
    }
}

/// The tag chip of the selected fixture: its name at its box's corner (criterion 8).
fn tag(fixture: &PlacedFixture, x: f64, y: f64) -> FixtureLabel {
    FixtureLabel {
        kind: "tag",
        text: fixture.name.clone(),
        x,
        y,
        edge: None,
    }
}

#[cfg(test)]
mod tests {
    use super::number_step;

    #[test]
    fn tape_numbers_thin_out_in_one_two_five_steps() {
        assert_eq!(number_step(4.0), 50);
        assert_eq!(number_step(0.5), 100);
        assert_eq!(number_step(0.3), 200);
        assert_eq!(number_step(0.1), 500);
        assert_eq!(number_step(0.05), 1000);
    }
}
