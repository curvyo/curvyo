//! Armed placement of a job fixture and centre snapping (`specs/0113-fixture-sets-and-pins`
//! criteria 13 and 30, `adrs.md` finding 1): one owned struct holds the armed kind, the hanging
//! centre, its snap state and the page picked for Apply; [`Session::end_fixture_placement`] is
//! the one way it ends.

use curvyo_document_core::{Affine, NodeId, Point, Vec2};
use curvyo_laser_core::FixtureKind;
use curvyo_render_core::DrawList;
use curvyo_ui_core::{SnapExclusions, SnapProbe, SnapResult, SnapSettings, SnapState, SnapTargets};

use super::super::Session;
use super::super::keys::{KeyInput, KeyOutcome};
use super::commands::{centred_at, new_fixture};
use super::params::new_params;
use super::placed::PlacedFixture;

/// The kinds the Job fixtures add bar offers, in its order (criterion 12): no grid.
const JOB_FIXTURE_KINDS: [FixtureKind; 5] = [
    FixtureKind::Pin,
    FixtureKind::Screw,
    FixtureKind::Holder,
    FixtureKind::Clamp,
    FixtureKind::Stop,
];

/// The centres of the drawing a job fixture snaps to, gathered once, and the hold of the last
/// frame (`0060` capture, release and hysteresis).
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct CentreSnap {
    targets: SnapTargets,
    state: SnapState,
    settings: Option<SnapSettings>,
    result: Option<SnapResult>,
}

impl CentreSnap {
    /// The offset that moves the nearest of `centres` onto a centre of the drawing; zero when
    /// none is within capture or snapping is off.
    pub(super) fn resolve(&mut self, centres: &[Point]) -> Vec2 {
        let Some(settings) = self.settings else {
            return Vec2::ZERO;
        };
        let result = curvyo_ui_core::resolve(
            &settings,
            &self.targets,
            &SnapProbe::points(centres),
            &mut self.state,
        );
        let offset = result.offset;
        self.result = (!result.is_empty()).then_some(result);
        offset
    }

    /// The last result, for the `0060` marker.
    pub(super) const fn result(&self) -> Option<&SnapResult> {
        self.result.as_ref()
    }
}

/// An armed kind and its hanging centre.
#[derive(Debug, Clone)]
struct Armed {
    kind: FixtureKind,
    /// The raw centre in document space; `None` while the pointer is off the canvas.
    raw: Option<Point>,
    /// The snapped centre the drop uses.
    at: Option<Point>,
    snap: CentreSnap,
}

/// What the session keeps for placement and the Apply preview. View state only.
#[derive(Debug, Clone, Default)]
pub(in crate::session) struct Placement {
    armed: Option<Armed>,
    /// The manufacturing whose job fixtures Apply previews (criterion 34).
    pub(super) apply: Option<NodeId>,
}

impl Session {
    /// The centre snap of a job fixture for a drag or a placement starting now: the centres of
    /// the drawing, and of the objects the laser pages show through placed pages, while snapping
    /// and its Centres target are on (criterion 30, amended 2026-11-16).
    pub(super) fn begin_centre_snap(&self) -> CentreSnap {
        let stored = self.document.snapping();
        let read = self.document_read();
        let mut settings = SnapSettings::from_document(stored, 1.0 / self.view().scale());
        // The centres are gathered only while Centres is on; the Objects switch does not gate
        // them (their kind is the box kind the resolver checks against `objects`).
        settings.objects = true;
        if !self.snapping_effective_enabled() {
            settings = settings.suspended();
        }
        let mut targets =
            SnapTargets::gather_centres(&read.nodes, stored.centres, SnapExclusions::default());
        if stored.centres {
            for manufacturing in self.document.manufacturings() {
                let Some(page) = manufacturing.page.as_ref() else {
                    continue;
                };
                let to_plane = Affine::translation(Vec2::new(page.origin.x, page.origin.y));
                for (_, map, members) in self.placements_of(&manufacturing) {
                    let objects = members.iter().map(|&index| &read.drawn[index]);
                    targets.add_placed_centres(objects, to_plane.compose(map));
                }
            }
        }
        CentreSnap {
            targets,
            settings: Some(settings),
            ..CentreSnap::default()
        }
    }

    /// The kind whose placement is armed, for its pressed button.
    #[must_use]
    pub fn armed_job_fixture(&self) -> Option<FixtureKind> {
        self.fixtures.placement.armed.as_ref().map(|a| a.kind)
    }

    /// A kind of the Job fixtures add bar (criterion 13): arms its placement, or with the kind
    /// already armed ends it (a second press). Returns whether placement is armed now. `false`
    /// off a laser page, for a kind the bar does not offer, or at 64 job fixtures.
    pub fn arm_job_fixture(&mut self, kind: &str) -> bool {
        let Some(kind) =
            super::commands::kind_named(kind).filter(|k| JOB_FIXTURE_KINDS.contains(k))
        else {
            return false;
        };
        if self.armed_job_fixture() == Some(kind) {
            self.end_fixture_placement();
            return false;
        }
        let Some(fixtures) = self.active_fixtures() else {
            return false;
        };
        if fixtures.job_fixtures().count() >= curvyo_document_core::MAX_JOB_FIXTURES {
            return false;
        }
        self.clear_fixture_selection();
        let raw = self.pointer_position;
        let mut armed = Armed {
            kind,
            raw,
            at: raw,
            snap: self.begin_centre_snap(),
        };
        if let Some(raw) = raw {
            armed.at = Some(raw.translated(armed.snap.resolve(&[raw])));
        }
        self.fixtures.placement.armed = Some(armed);
        true
    }

    /// Ends placement without adding (Escape, a right click, a second press, another tool, a page
    /// activation). `true` when it was armed.
    pub fn end_fixture_placement(&mut self) -> bool {
        self.fixtures.placement.armed.take().is_some()
    }

    /// The pointer over the canvas while placement is armed: the fixture hangs there by its
    /// centre, snapped. `true` when placement took the move.
    pub(in crate::session) fn fixture_placement_hover(&mut self, point: Point) -> bool {
        let Some(armed) = self.fixtures.placement.armed.as_mut() else {
            return false;
        };
        armed.raw = Some(point);
        armed.at = Some(point.translated(armed.snap.resolve(&[point])));
        true
    }

    /// The pointer left the canvas: the hanging fixture is not shown.
    pub(in crate::session) fn fixture_placement_leave(&mut self) {
        if let Some(armed) = self.fixtures.placement.armed.as_mut() {
            armed.raw = None;
            armed.at = None;
        }
    }

    /// A press while placement is armed drops the fixture there and selects nothing else; it
    /// stays armed (UX notes F4). `true` when placement took the press.
    pub(in crate::session) fn fixture_placement_press(&mut self, point: Point) -> bool {
        if self.fixtures.placement.armed.is_none() {
            return false;
        }
        self.fixture_placement_hover(point);
        self.drop_fixture_placement();
        true
    }

    /// Moves the hanging fixture by `dx`, `dy` mm from where it hangs, or from the work area's
    /// centre without a pointer (keyboard placement, UX notes F5), with centre snapping.
    pub fn nudge_fixture_placement(&mut self, dx: f64, dy: f64) -> bool {
        let Some(fixtures) = self.active_fixtures() else {
            return false;
        };
        let page = fixtures.page;
        let Some(armed) = self.fixtures.placement.armed.as_mut() else {
            return false;
        };
        let from = armed.at.unwrap_or_else(|| {
            Point::new(
                page.origin.x + page.size.width.as_mm() / 2.0,
                page.origin.y + page.size.height.as_mm() / 2.0,
            )
        });
        let raw = from.translated(Vec2::new(dx, dy));
        armed.raw = Some(raw);
        armed.at = Some(raw.translated(armed.snap.resolve(&[raw])));
        true
    }

    /// Drops the hanging fixture (a click, or Enter): one step "Add job fixture", the new
    /// fixture "`<Kind>` N", Active, selected; placement stays armed. Without a pointer the fixture
    /// drops at the work area's centre. Returns the new fixture.
    pub fn drop_fixture_placement(&mut self) -> Option<NodeId> {
        let fixtures = self.active_fixtures()?;
        let armed = self.fixtures.placement.armed.clone()?;
        let page = &fixtures.page;
        let at = armed.at.unwrap_or_else(|| {
            Point::new(
                page.origin.x + page.size.width.as_mm() / 2.0,
                page.origin.y + page.size.height.as_mm() / 2.0,
            )
        });
        let local = Point::new(at.x - page.origin.x, at.y - page.origin.y);
        let stored = armed.kind.stored()?;
        let params = new_params(armed.kind);
        let new = new_fixture(
            armed.kind,
            stored,
            &params,
            Some(centred_at(armed.kind, local)),
        );
        let manufacturing = fixtures.manufacturing;
        let id = self
            .recording_unguarded(|this| this.document.add_job_fixture(manufacturing, &new).ok())?;
        self.select_fixture(id);
        // At 64 job fixtures the bar hides, so placement ends with it (criterion 17).
        if self
            .active_fixtures()
            .is_some_and(|f| f.job_fixtures().count() >= curvyo_document_core::MAX_JOB_FIXTURES)
        {
            self.end_fixture_placement();
        }
        Some(id)
    }

    /// The release of a press placement took: nothing more happens. `true` while armed.
    pub(in crate::session) fn fixture_placement_release(&self) -> bool {
        self.fixtures.placement.armed.is_some()
    }

    /// The arrows and Enter while placement is armed (UX notes 5.1): 1 mm, 10 mm with Shift, with
    /// centre snapping; Enter drops.
    pub(super) fn fixture_placement_key(&mut self, input: &KeyInput<'_>) -> Option<KeyOutcome> {
        self.fixtures.placement.armed.as_ref()?;
        if input.dom_blocked || input.ctrl || input.alt {
            return None;
        }
        if input.key == "Enter" {
            self.drop_fixture_placement();
            return Some(KeyOutcome::Consumed);
        }
        let step = if input.shift { 10.0 } else { 1.0 };
        let (dx, dy) = match input.key {
            "ArrowLeft" => (-step, 0.0),
            "ArrowRight" => (step, 0.0),
            "ArrowUp" => (0.0, -step),
            "ArrowDown" => (0.0, step),
            _ => return None,
        };
        self.nudge_fixture_placement(dx, dy);
        Some(KeyOutcome::Consumed)
    }

    /// The hanging fixture as it would drop, for the dashed preview.
    fn hanging_fixture(&self) -> Option<PlacedFixture> {
        let armed = self.fixtures.placement.armed.as_ref()?;
        let at = armed.at?;
        let fixtures = self.active_fixtures()?;
        let page = &fixtures.page;
        let local = Point::new(at.x - page.origin.x, at.y - page.origin.y);
        Some(super::placed::preview_fixture(
            fixtures.manufacturing,
            page,
            armed.kind,
            &new_params(armed.kind),
            centred_at(armed.kind, local),
        ))
    }

    /// Over everything: the hanging fixture dashed in the live preview colour with the `0060`
    /// marker of its snap (UX notes F2).
    pub(super) fn fixture_placement_overlay(&self) -> DrawList {
        let mut list = DrawList::default();
        let Some(fixture) = self.hanging_fixture() else {
            return list;
        };
        let Some(fixtures) = self.active_fixtures() else {
            return list;
        };
        list.extend(curvyo_render_core::build_fixture_preview(
            &fixture.outline(&fixtures.page),
            self.view(),
        ));
        if let Some(result) = self
            .fixtures
            .placement
            .armed
            .as_ref()
            .and_then(|a| a.snap.result())
        {
            list.extend(self.snap_marker_of(result));
        }
        list
    }
}
