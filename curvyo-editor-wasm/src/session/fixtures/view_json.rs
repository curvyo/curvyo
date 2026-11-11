//! The Fixtures block's view (`specs/0102-laser-fixtures` criteria 1 to 6, 15, 19, 20, 32, 33;
//! feature `adrs.md` §6a.7 and §7): one serde type, the only place that builds this JSON. The shared
//! fixtures in `curvyo-editor-wasm/tests/fixtures/fixtures_view/` pin its shape.

use curvyo_document_core::{Length, MAX_SLOTS_PER_FIXTURE, NodeId};
use curvyo_laser_core::{FixtureKind, SlotShape, TapeEdge};
use serde::Serialize;

use super::super::Session;
use super::draw::slot_label_fits;
use super::placed::{PageFixtures, PlacedFixture, Shape};

use super::slots::slot_outside;

/// The whole Fixtures block of the active laser page.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FixturesView {
    /// One row per fixture in creation order.
    pub rows: Vec<FixtureRow>,
    /// Whether the Grid button shows: hidden while the page has a grid (criterion 2).
    pub can_add_grid: bool,
    /// The selected fixture's id.
    pub selected: Option<String>,
    /// The selected slot's id.
    pub selected_slot: Option<String>,
    /// The fixture whose trash a Delete on the canvas armed.
    pub armed: Option<String>,
    /// One notice per fixture with damaged values (criterion 33).
    pub damage_notices: Vec<String>,
    /// The Fixtures snap switch (criterion 24).
    pub snapping_fixtures: bool,
    /// What a screen reader says of the selected fixture, with its sizes ("Holder 1 selected,
    /// 100.0 by 60.0 mm."; UX notes §11); `None` without a selection.
    pub selection_announcement: Option<String>,
}

/// One fixture's row and its fields.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FixtureRow {
    /// The node id as its full id text.
    pub id: String,
    /// The name.
    pub name: String,
    /// The kind; `unknown` for a kind this build does not know (criterion 32).
    pub kind: FixtureKindView,
    /// The Active check box (criterion 5).
    pub active: bool,
    /// The eye (criterion 6).
    pub visible: bool,
    /// The grid's fields; holder, clamp, stop and screw have none (D27).
    pub grid: Option<GridValues>,
    /// A holder's readable slots in creation order.
    pub slots: Vec<SlotRow>,
    /// Whether Add slot shows: a holder under 50 slots (criterion 19).
    pub can_add_slot: bool,
}

/// The kinds of the rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureKindView {
    /// The engraved grid.
    Grid,
    /// A holder.
    Holder,
    /// A clamp.
    Clamp,
    /// A stop.
    Stop,
    /// A screw.
    Screw,
    /// A kind this build does not know.
    Unknown,
}

/// The grid's fields (criterion 15).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GridValues {
    /// Spacing in mm.
    pub spacing_mm: f64,
    /// Every n-th line is major.
    pub major_every: u32,
    /// The tape switches.
    pub tapes: TapesView,
    /// The origin offset X in mm.
    pub origin_x_mm: f64,
    /// The origin offset Y in mm.
    pub origin_y_mm: f64,
}

/// The four tape switches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[allow(clippy::struct_excessive_bools)] // one switch per edge
pub struct TapesView {
    /// Bottom edge.
    pub bottom: bool,
    /// Left edge.
    pub left: bool,
    /// Top edge.
    pub top: bool,
    /// Right edge.
    pub right: bool,
}

/// One slot's row (criteria 19, 20).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SlotRow {
    /// The node id as its full id text.
    pub id: String,
    /// The name.
    pub name: String,
    /// `rect` or `circle`.
    pub shape: &'static str,
    /// The centre in mm from the board's corner.
    pub cx_mm: f64,
    /// The centre in mm from the board's corner.
    pub cy_mm: f64,
    /// A rectangle's width; a circle's diameter.
    pub w_mm: f64,
    /// A rectangle's height; a circle's diameter.
    pub h_mm: f64,
    /// Whether it lies past the board's edge ("Outside the board").
    pub outside: bool,
}

/// A text over the canvas the host places in the DOM: a tape number, a fixture's or a slot's
/// name, or the tag of the selected fixture, at screen coordinates in CSS px.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FixtureLabel {
    /// `tape`, `fixture`, `slot` or `tag` (the selected fixture's name chip).
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

pub(super) fn kind_view(kind: FixtureKind) -> FixtureKindView {
    match kind {
        FixtureKind::EngravedGrid => FixtureKindView::Grid,
        FixtureKind::Holder => FixtureKindView::Holder,
        FixtureKind::Clamp => FixtureKindView::Clamp,
        FixtureKind::Stop => FixtureKindView::Stop,
        FixtureKind::Screw => FixtureKindView::Screw,
        FixtureKind::Unknown => FixtureKindView::Unknown,
    }
}

fn row(fixture: &PlacedFixture) -> FixtureRow {
    let mut out = FixtureRow {
        id: fixture.id.full_id(),
        name: fixture.name.clone(),
        kind: kind_view(fixture.kind),
        active: fixture.active,
        visible: fixture.visible,
        grid: None,
        slots: Vec::new(),
        can_add_slot: false,
    };
    match &fixture.shape {
        Shape::Grid(spec) => {
            out.grid = Some(GridValues {
                spacing_mm: spec.spacing.as_mm(),
                major_every: spec.major_every,
                tapes: TapesView {
                    bottom: spec.tapes.has(TapeEdge::Bottom),
                    left: spec.tapes.has(TapeEdge::Left),
                    top: spec.tapes.has(TapeEdge::Top),
                    right: spec.tapes.has(TapeEdge::Right),
                },
                origin_x_mm: spec.origin_x.as_mm(),
                origin_y_mm: spec.origin_y.as_mm(),
            });
        }
        Shape::Board { width, height } => {
            out.slots = slot_rows(fixture, *width, *height);
            out.can_add_slot = fixture.slots.len() + fixture.damaged_slots < MAX_SLOTS_PER_FIXTURE;
        }
        // Clamp, stop and screw have no field in the block (D27).
        Shape::Clamp { .. } | Shape::Stop { .. } | Shape::Screw { .. } | Shape::Unknown => {}
    }
    out
}

/// A holder's slot rows (criteria 19, 20).
fn slot_rows(holder: &PlacedFixture, width: Length, height: Length) -> Vec<SlotRow> {
    holder
        .slots
        .iter()
        .map(|s| {
            let (w, h) = s.slot.extent();
            SlotRow {
                id: s.id.full_id(),
                name: s.name.clone(),
                shape: match s.slot.shape {
                    SlotShape::Rectangle => "rect",
                    SlotShape::Circle => "circle",
                },
                cx_mm: s.slot.centre.x,
                cy_mm: s.slot.centre.y,
                w_mm: w.as_mm(),
                h_mm: h.as_mm(),
                outside: slot_outside(&s.slot, width, height),
            }
        })
        .collect()
}

/// "1 value damaged on Holder 1." or "<n> values damaged on <name>." (criterion 33).
fn damage_notice(fixture: &PlacedFixture) -> Option<String> {
    let n = fixture.damaged + fixture.damaged_slots;
    match n {
        0 => None,
        1 => Some(format!("1 value damaged on {}.", fixture.name)),
        _ => Some(format!("{n} values damaged on {}.", fixture.name)),
    }
}

/// A length as the announcer says it: one decimal.
fn spoken(value: Length) -> String {
    format!("{:.1}", value.as_mm())
}

/// The announcement of `fixture` selected, with its sizes (UX notes §11).
pub(super) fn announcement(fixture: &PlacedFixture) -> String {
    let name = &fixture.name;
    match &fixture.shape {
        Shape::Board { width, height } | Shape::Clamp { width, height } => format!(
            "{name} selected, {} by {} mm.",
            spoken(*width),
            spoken(*height)
        ),
        Shape::Stop {
            leg_a,
            leg_b,
            thickness,
        } => format!(
            "{name} selected, legs {} and {} mm, thickness {} mm.",
            spoken(*leg_a),
            spoken(*leg_b),
            spoken(*thickness)
        ),
        Shape::Screw { thread, head } => format!(
            "{name} selected, thread {} mm, head {} mm.",
            spoken(*thread),
            spoken(*head)
        ),
        Shape::Grid(spec) => format!("{name} selected, spacing {} mm.", spoken(spec.spacing)),
        Shape::Unknown => format!("{name} selected."),
    }
}

/// The view of `fixtures` with the session's selection, armed trash and switch.
pub(super) fn view_of(
    fixtures: &PageFixtures,
    selected: Option<NodeId>,
    selected_slot: Option<String>,
    armed: Option<String>,
    snapping_fixtures: bool,
) -> FixturesView {
    FixturesView {
        rows: fixtures.fixtures.iter().map(row).collect(),
        can_add_grid: !fixtures.has_grid(),
        selection_announcement: selected.and_then(|id| fixtures.get(id)).map(announcement),
        selected: selected.map(NodeId::full_id),
        selected_slot,
        armed,
        damage_notices: fixtures.fixtures.iter().filter_map(damage_notice).collect(),
        snapping_fixtures,
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
    /// The Fixtures block of the active laser page; `None` on any other page.
    #[must_use]
    pub fn fixtures_view(&self) -> Option<FixturesView> {
        let fixtures = self.active_fixtures()?;
        Some(view_of(
            &fixtures,
            self.selected_fixture(),
            self.selected_slot().map(NodeId::full_id),
            self.armed_fixture().map(NodeId::full_id),
            self.document.snapping().fixtures,
        ))
    }

    /// [`Session::fixtures_view`] as JSON text, `null` off a laser page.
    #[must_use]
    pub fn fixtures_view_json(&self) -> String {
        serde_json::to_string(&self.fixtures_view()).unwrap_or_else(|_| "null".to_owned())
    }

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
        for fixture in fixtures.fixtures.iter().filter(|f| f.shown()) {
            let is_selected = selected == Some(fixture.id);
            if let Shape::Grid(spec) = fixture.shape {
                let grid = self.fixtures.grid_cache.layout(spec, fixtures.page.size);
                for tape in &grid.tapes {
                    for number in &tape.numbers {
                        if number.mm.rem_euclid(step) == 0 {
                            let (x, y) =
                                screen(number.at.translated(curvyo_document_core::Vec2::new(
                                    origin.x, origin.y,
                                )));
                            out.push(FixtureLabel {
                                kind: "tape",
                                text: number.mm.to_string(),
                                x,
                                y,
                                edge: Some(edge_name(tape.edge)),
                            });
                        }
                    }
                }
                if is_selected {
                    let (x, y) = screen(origin);
                    out.push(tag(fixture, x, y));
                }
                continue;
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
                .filter(|s| slot_label_fits(&s.slot, device))
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
