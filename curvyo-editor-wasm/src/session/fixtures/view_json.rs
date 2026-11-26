//! The Fixtures tab's view (`specs/0102-laser-fixtures` criteria 1 to 6, 15, 19, 20, 32, 33;
//! `specs/0113-fixture-sets-and-pins` criteria 1 to 20, 25, 31, 33, 40; `adrs.md` finding 3): one
//! serde type, the only place that builds this JSON. The shared fixtures in
//! `curvyo-editor-wasm/tests/fixtures/fixtures_view/` pin its shape.

use curvyo_document_core::{JobRole, Length, MAX_SLOTS_PER_FIXTURE, NodeId};
use curvyo_laser_core::{FixtureKind, SlotShape, TapeEdge};
use serde::Serialize;

use super::super::Session;
use super::placed::{PageFixtures, PlacedFixture};
use super::row_lines::{line2, role_toggle, set_line2};
use super::shape::Shape;
use super::slots::slot_outside;

/// The Fixtures tab of the active laser page (`specs/0113-fixture-sets-and-pins` criteria 1 to
/// 17, 33; `adrs.md` finding 3): its Machine sets, its Job fixtures and the Apply list.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FixturesView {
    /// The machine sets in their order (criterion 2).
    pub sets: Vec<SetView>,
    /// How many sets are on this page: the heading's "`<n>` of `<m>` on this page".
    pub sets_on_page: usize,
    /// Whether "+ Set" shows: hidden at 16 sets and while the sets are not editable here
    /// (criteria 3, 17, 46).
    pub can_add_set: bool,
    /// The line of a page whose sets are read only here (criterion 46): "From the project file.
    /// `<profile>` is not on this computer."; every content control of the sets is hidden then.
    pub read_only: Option<String>,
    /// The line of a page on Generic laser (criterion 3): sets are kept in a machine profile.
    pub profile_hint: Option<String>,
    /// The job fixtures in creation order (criterion 12).
    pub job_fixtures: Vec<FixtureRow>,
    /// Whether the Job fixtures add bar shows: hidden at 64 job fixtures (criterion 17).
    pub can_add_job_fixture: bool,
    /// The kind whose placement is armed: its button is pressed (criterion 13).
    pub armed_kind: Option<FixtureKindView>,
    /// The other laser pages with job fixtures; empty hides "Apply fixtures from page…"
    /// (criterion 33).
    pub apply_candidates: Vec<super::ApplyCandidate>,
    /// The picked page of the Apply list, as its candidate id (criterion 34).
    pub apply_picked: Option<String>,
    /// The selected fixture's id.
    pub selected: Option<String>,
    /// The selected slot's id.
    pub selected_slot: Option<String>,
    /// The fixture whose trash a Delete on the canvas armed.
    pub armed: Option<String>,
    /// One notice per set and one for the job fixtures with damaged values (criterion 40).
    pub damage_notices: Vec<String>,
    /// The Fixtures snap switch (`0102` criterion 24).
    pub snapping_fixtures: bool,
    /// What a screen reader says of the selected fixture, with its sizes ("Holder 1 selected,
    /// 100.0 by 60.0 mm."; UX notes §11); `None` without a selection.
    pub selection_announcement: Option<String>,
}

impl FixturesView {
    /// Every fixture row in listing order: the sets' fixtures, then the job fixtures (criterion
    /// 16).
    #[must_use]
    pub fn rows(&self) -> Vec<FixtureRow> {
        self.sets
            .iter()
            .flat_map(|set| set.fixtures.iter().cloned())
            .chain(self.job_fixtures.iter().cloned())
            .collect()
    }
}

/// One machine set's row and, unfolded, its body (criteria 2, 7).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[allow(clippy::struct_excessive_bools)] // the row's check, Lock, fold and add bar, one each
pub struct SetView {
    /// The set node as its full id text.
    pub id: String,
    /// Its name.
    pub name: String,
    /// The On this page check.
    pub on_page: bool,
    /// Its Lock (criterion 32): every fixture in it keeps its place.
    pub locked: bool,
    /// Whether its add bar shows: hidden while it is locked, read only, or the profile holds 64
    /// set fixtures (criteria 17, 32).
    pub can_add: bool,
    /// Whether its body shows (session only).
    pub unfolded: bool,
    /// Line 2: "Holder, Screw · 2 fixtures", "... · not on this page", or "No fixtures".
    pub line2: String,
    /// Whether its add bar offers Grid: hidden while the set has one (criterion 7).
    pub can_add_grid: bool,
    /// Its fixtures in creation order.
    pub fixtures: Vec<FixtureRow>,
}

/// One fixture's row and its fields.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[allow(clippy::struct_excessive_bools)] // the row's checks and states, one each
pub struct FixtureRow {
    /// The node id as its full id text.
    pub id: String,
    /// The name.
    pub name: String,
    /// The kind; `unknown` for a kind this build does not know (criterion 32).
    pub kind: FixtureKindView,
    /// Its own On the bed check (`0102` criterion 5).
    pub active: bool,
    /// Whether it lies on the bed: its own check and its set on this page (criterion 8).
    pub on_bed: bool,
    /// The eye (criterion 6).
    pub visible: bool,
    /// Its own Lock (criteria 31, 32; a set fixture's is the profile's).
    pub locked: bool,
    /// Whether its set's Lock is on: the fixture is locked and its own Lock is not pressable
    /// (criterion 32); `false` for a job fixture.
    pub locked_by_set: bool,
    /// Line 2 of the row (UX notes 4.3, 5.2).
    pub line2: String,
    /// The grid's fields; holder, clamp, stop and screw have none (D27).
    pub grid: Option<GridValues>,
    /// The fields and Jobs toggles of a pin or a screw (criteria 18 to 20, 25).
    pub round: Option<RoundValues>,
    /// A holder's readable slots in creation order.
    pub slots: Vec<SlotRow>,
    /// Whether Add slot shows: a holder under 50 slots (criterion 19).
    pub can_add_slot: bool,
}

/// The unfolded body of a pin or a screw (criteria 18 to 20, 25; UX notes 6.1).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RoundValues {
    /// A pin's Diameter; `None` for a screw, which keeps its knobs (criterion 20).
    pub diameter_mm: Option<f64>,
    /// The columns.
    pub columns: u32,
    /// The rows.
    pub rows: u32,
    /// The spacing of the columns; its field hides while `columns` is 1.
    pub column_spacing_mm: f64,
    /// The spacing of the rows; its field hides while `rows` is 1.
    pub row_spacing_mm: f64,
    /// The Centre mark's Size.
    pub mark_mm: f64,
    /// The Centre mark toggle: its job's Output; `None` hides the toggles (an old screw, no role
    /// jobs, criterion 39).
    pub centre_mark: Option<RoleToggle>,
    /// The Diameters toggle: its job's Output.
    pub diameters: Option<RoleToggle>,
}

/// One Jobs toggle: the role job it switches and whether its Output is on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoleToggle {
    /// The role job's full id text, for the Output switch.
    pub job: String,
    /// Its Output.
    pub on: bool,
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
    /// A pin (`0113` criterion 18).
    Pin,
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

pub(super) fn kind_view(kind: FixtureKind) -> FixtureKindView {
    match kind {
        FixtureKind::EngravedGrid => FixtureKindView::Grid,
        FixtureKind::Holder => FixtureKindView::Holder,
        FixtureKind::Clamp => FixtureKindView::Clamp,
        FixtureKind::Stop => FixtureKindView::Stop,
        FixtureKind::Screw => FixtureKindView::Screw,
        FixtureKind::Pin => FixtureKindView::Pin,
        FixtureKind::Unknown => FixtureKindView::Unknown,
    }
}

fn row(fixture: &PlacedFixture) -> FixtureRow {
    let mut out = FixtureRow {
        id: fixture.id.full_id(),
        name: fixture.name.clone(),
        kind: kind_view(fixture.kind),
        active: fixture.active,
        on_bed: fixture.on_bed(),
        visible: fixture.visible,
        locked: fixture.stored_locked,
        locked_by_set: fixture.set_locked,
        line2: line2(fixture),
        grid: None,
        round: None,
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
        Shape::Pin { grid, .. } | Shape::Screw { grid, .. } => {
            let diameter_mm = match &fixture.shape {
                Shape::Pin { diameter, .. } => Some(diameter.as_mm()),
                _ => None,
            };
            out.round = Some(RoundValues {
                diameter_mm,
                columns: grid.counts.columns(),
                rows: grid.counts.rows(),
                column_spacing_mm: grid.column_spacing.as_mm(),
                row_spacing_mm: grid.row_spacing.as_mm(),
                mark_mm: grid.mark.as_mm(),
                centre_mark: role_toggle(fixture, JobRole::CentreMark),
                diameters: role_toggle(fixture, JobRole::Diameters),
            });
        }
        // Clamp and stop have no field in the block (D27).
        Shape::Clamp { .. } | Shape::Stop { .. } | Shape::Unknown => {}
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

/// "1 value damaged in Set 1." or "`<n>` values damaged in `<where>`." (criterion 40): the damaged
/// values of one set (`own`, its name or check) and its fixtures, or of the job fixtures.
fn damage_notice<'a>(
    place: &str,
    own: usize,
    fixtures: impl Iterator<Item = &'a PlacedFixture>,
) -> Option<String> {
    let n: usize = own + fixtures.map(|f| f.damaged + f.damaged_slots).sum::<usize>();
    match n {
        0 => None,
        1 => Some(format!("1 value damaged in {place}.")),
        _ => Some(format!("{n} values damaged in {place}.")),
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
        Shape::Screw { thread, head, .. } => format!(
            "{name} selected, thread {} mm, head {} mm.",
            spoken(*thread),
            spoken(*head)
        ),
        Shape::Pin { diameter, .. } => {
            format!("{name} selected, diameter {} mm.", spoken(*diameter))
        }
        Shape::Grid(spec) => format!("{name} selected, spacing {} mm.", spoken(spec.spacing)),
        Shape::Unknown => format!("{name} selected."),
    }
}

/// The notice line of a page on Generic laser (criterion 3, UX notes 4.5).
pub(super) const PROFILE_HINT: &str =
    "Fixture sets are kept in a machine profile. Save this machine as a profile to add sets.";

/// The line of a page whose sets are read only here (criterion 46, UX notes 4.5).
pub(super) fn read_only_line(profile: &str) -> String {
    format!(
        "From the project file. {} is not on this computer.",
        crate::session::library_journal::short_name(profile)
    )
}

/// What the session adds to a view of the page's fixtures.
pub(super) struct ViewState {
    /// Where the page's sets live on this computer.
    pub home: super::profile_sets::SetsHome,
    pub selected: Option<NodeId>,
    pub selected_slot: Option<String>,
    pub armed: Option<String>,
    pub snapping_fixtures: bool,
    pub armed_kind: Option<FixtureKind>,
    pub apply_candidates: Vec<super::ApplyCandidate>,
    pub apply_picked: Option<String>,
}

/// The view of `fixtures` with the session's state; `unfolded` says which sets show their body.
pub(super) fn view_of(
    fixtures: &PageFixtures,
    unfolded: impl Fn(NodeId) -> bool,
    state: ViewState,
) -> FixturesView {
    let editable = matches!(state.home, super::profile_sets::SetsHome::Editable(_));
    let set_fixtures = fixtures.fixtures.iter().filter(|f| f.set.is_some()).count();
    let sets: Vec<SetView> = fixtures
        .sets
        .iter()
        .map(|set| {
            let members: Vec<&PlacedFixture> = fixtures
                .fixtures
                .iter()
                .filter(|f| f.set == Some(set.id))
                .collect();
            SetView {
                id: set.id.full_id(),
                name: set.name.clone(),
                on_page: set.on_page,
                locked: set.locked,
                can_add: editable
                    && !set.locked
                    && set_fixtures < curvyo_laser_core::MAX_RECORD_FIXTURES,
                unfolded: unfolded(set.id),
                line2: set_line2(set.on_page, set.locked, &members),
                can_add_grid: !fixtures.set_has_grid(set.id),
                fixtures: members.into_iter().map(row).collect(),
            }
        })
        .collect();
    let job_fixtures: Vec<FixtureRow> = fixtures.job_fixtures().map(row).collect();
    let mut damage_notices: Vec<String> = fixtures
        .sets
        .iter()
        .filter_map(|set| {
            damage_notice(
                &set.name,
                set.damaged,
                fixtures.fixtures.iter().filter(|f| f.set == Some(set.id)),
            )
        })
        .collect();
    damage_notices.extend(damage_notice("Job fixtures", 0, fixtures.job_fixtures()));
    FixturesView {
        sets_on_page: fixtures.sets.iter().filter(|s| s.on_page).count(),
        can_add_set: editable && fixtures.sets.len() < curvyo_document_core::MAX_FIXTURE_SETS,
        read_only: match &state.home {
            super::profile_sets::SetsHome::ReadOnly(name) => Some(read_only_line(name)),
            _ => None,
        },
        profile_hint: matches!(state.home, super::profile_sets::SetsHome::Generic)
            .then(|| PROFILE_HINT.to_owned()),
        can_add_job_fixture: job_fixtures.len() < curvyo_document_core::MAX_JOB_FIXTURES,
        sets,
        job_fixtures,
        armed_kind: state.armed_kind.map(kind_view),
        apply_candidates: state.apply_candidates,
        apply_picked: state.apply_picked,
        selection_announcement: state
            .selected
            .and_then(|id| fixtures.get(id))
            .map(announcement),
        selected: state.selected.map(NodeId::full_id),
        selected_slot: state.selected_slot,
        armed: state.armed,
        damage_notices,
        snapping_fixtures: state.snapping_fixtures,
    }
}

impl Session {
    /// The Fixtures tab of the active laser page; `None` on any other page.
    #[must_use]
    pub fn fixtures_view(&self) -> Option<FixturesView> {
        let fixtures = self.active_fixtures()?;
        let home = self.sets_home(&self.active_manufacturing()?);
        let state = ViewState {
            home,
            selected: self.selected_fixture(),
            selected_slot: self.selected_slot().map(NodeId::full_id),
            armed: self.armed_fixture().map(NodeId::full_id),
            snapping_fixtures: self.document.snapping().fixtures,
            armed_kind: self.armed_job_fixture(),
            apply_candidates: self.apply_fixture_candidates(),
            apply_picked: self.apply_preview_source().map(NodeId::full_id),
        };
        let view = view_of(&fixtures, |id| self.fixtures.sets.is_unfolded(id), state);
        Some(self.with_filled_holders(view))
    }

    /// [`Session::fixtures_view`] as JSON text, `null` off a laser page.
    #[must_use]
    pub fn fixtures_view_json(&self) -> String {
        serde_json::to_string(&self.fixtures_view()).unwrap_or_else(|_| "null".to_owned())
    }
}
