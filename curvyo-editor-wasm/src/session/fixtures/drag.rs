//! A drag of the selected fixture by one of its handles (`specs/0102-laser-fixtures` criteria 14,
//! 18, 22, 23, 38; feature `adrs.md` §6a.5): a move, a turn with the 22.5 degree snap under Ctrl,
//! a resize or a knob; nothing is written before the release, which is one step.

use curvyo_document_core::{NodeId, Page, Point, Vec2};

use super::super::Session;
use super::edit::{self, FixtureEdit};
use super::handles::{FixtureHandle, fixture_box};
use super::placed::{PageFixtures, PlacedFixture};
use super::placement::CentreSnap;

/// A fixture drag in flight.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct FixtureDrag {
    pub id: NodeId,
    /// The handle the press took.
    pub handle: FixtureHandle,
    /// The press, in document space.
    start: Point,
    /// The fixture at the press, and its page.
    pub fixture: PlacedFixture,
    pub page: Page,
    /// What the drag shows now; `None` until the pointer leaves the dead zone.
    pub live: Option<FixtureEdit>,
    /// The pointer of the last move, in document space: the readout's anchor.
    pub pointer: Point,
    /// Whether the drag snaps the fixture's centres: a moved job fixture (`0113` criterion 30).
    wants_snap: bool,
    /// The centre snap, gathered on the first move out of the dead zone, so a press pays nothing
    /// for it (`0113` criterion 42 a); `None` before that and for every drag that does not snap.
    snap: Option<CentreSnap>,
}

impl Session {
    /// Begins a drag of the fixture `id` by `handle` at `point`.
    pub(super) fn begin_fixture_drag(
        &mut self,
        fixtures: &PageFixtures,
        id: NodeId,
        point: Point,
        handle: FixtureHandle,
    ) {
        let Some(fixture) = fixtures.get(id) else {
            return;
        };
        // A locked job fixture keeps its place: a press on it holds and starts no marquee
        // (`0113` criterion 32).
        let handle = if fixture.keeps_place() {
            FixtureHandle::Hold
        } else {
            handle
        };
        let wants_snap = handle == FixtureHandle::Move && fixture.is_job_fixture();
        self.fixtures.drag = Some(FixtureDrag {
            id,
            handle,
            start: point,
            fixture: fixture.clone(),
            page: fixtures.page.clone(),
            live: None,
            pointer: point,
            wants_snap,
            snap: None,
        });
    }

    /// One pointer move of a fixture drag; `true` when one is in flight.
    pub(in crate::session) fn fixture_drag_move(&mut self, point: Point) -> bool {
        let threshold = self.transform_handle_tolerances().drag_threshold_mm;
        let modifiers = (self.held.shift, self.held.ctrl);
        let Some(drag) = self.fixtures.drag.as_ref() else {
            return false;
        };
        let delta = drag.start.vector_to(point);
        if drag.handle == FixtureHandle::Hold
            || (drag.live.is_none() && delta.length() <= threshold)
        {
            return true;
        }
        if drag.wants_snap && drag.snap.is_none() {
            let snap = self.begin_centre_snap();
            if let Some(drag) = self.fixtures.drag.as_mut() {
                drag.snap = Some(snap);
            }
        }
        let Some(drag) = self.fixtures.drag.as_mut() else {
            return false;
        };
        let mut edit = drag_edit(drag, point, modifiers);
        if let Some(snap) = drag.snap.as_mut() {
            // Each centre of the grid is tried; the nearest match moves the whole fixture.
            let moved = Vec2::new(
                edit.matrix.e - drag.fixture.matrix.e,
                edit.matrix.f - drag.fixture.matrix.f,
            );
            let centres: Vec<Point> = drag
                .fixture
                .centres()
                .into_iter()
                .map(|c| c.translated(moved))
                .collect();
            let offset = snap.resolve(&centres);
            edit = edit::translate(&drag.fixture, moved + offset);
        }
        drag.live = Some(edit);
        drag.pointer = point;
        true
    }

    /// The centre snap of the job fixture being dragged, for the `0060` marker.
    pub(super) fn fixture_drag_snap(&self) -> Option<&curvyo_ui_core::SnapResult> {
        self.fixtures.drag.as_ref()?.snap.as_ref()?.result()
    }

    /// The release of a fixture drag: one step "Move fixture", "Rotate fixture" or "Resize
    /// fixture" when the pointer left the dead zone. `true` when one was in flight.
    pub(in crate::session) fn fixture_drag_release(&mut self, point: Point) -> bool {
        if !self.fixture_drag_move(point) {
            return false;
        }
        let Some(drag) = self.fixtures.drag.take() else {
            return false;
        };
        if let Some(edit) = drag.live {
            self.write_fixture_edit(drag.id, &edit, label_of(drag.handle));
        }
        true
    }

    /// What the fixture `id` shows while a drag of it is in flight.
    pub(super) fn fixture_live_edit(&self, id: NodeId) -> Option<&FixtureEdit> {
        self.fixtures
            .drag
            .as_ref()
            .filter(|d| d.id == id)
            .and_then(|d| d.live.as_ref())
    }

    /// The handle a drag of the fixture holds, for its dragging look.
    pub(super) fn fixture_drag_handle(&self) -> Option<FixtureHandle> {
        self.fixtures.drag.as_ref().map(|d| d.handle)
    }

    /// Ends a fixture drag without writing (Escape); `true` when there was one.
    pub(in crate::session) fn cancel_fixture_drag(&mut self) -> bool {
        self.fixtures.drag.take().is_some()
    }
}

/// The commit label of a release of `handle`.
pub(super) const fn label_of(handle: FixtureHandle) -> &'static str {
    match handle {
        FixtureHandle::Move | FixtureHandle::Hold => {
            curvyo_document_core::MOVE_FIXTURE_COMMIT_LABEL
        }
        FixtureHandle::Rotate(_) => curvyo_document_core::ROTATE_FIXTURE_COMMIT_LABEL,
        _ => curvyo_document_core::RESIZE_FIXTURE_COMMIT_LABEL,
    }
}

/// The edit of `drag` with the pointer at `point`: the box arithmetic in the fixture's own frame.
fn drag_edit(drag: &FixtureDrag, point: Point, (shift, ctrl): (bool, bool)) -> FixtureEdit {
    let box_ = fixture_box(&drag.fixture, &drag.page);
    let local = |p: Point| {
        let q = box_.to_local(p);
        Point::new(q.x - box_.pivot.x, q.y - box_.pivot.y)
    };
    let moved = local(drag.start).vector_to(local(point));
    match drag.handle {
        FixtureHandle::Move | FixtureHandle::Hold => {
            edit::translate(&drag.fixture, drag.start.vector_to(point))
        }
        FixtureHandle::Rotate(_) => {
            let centre = box_.to_document(box_.local_center());
            let angle = curvyo_ui_core::rotate_delta_angle(centre, drag.start, point, ctrl);
            let (min, max) = drag.fixture.local_box(&drag.page);
            let pivot = Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y));
            edit::rotate(&drag.fixture, pivot, angle)
        }
        FixtureHandle::Resize(direction) => {
            edit::resize(&drag.fixture, direction, moved, (shift, ctrl))
        }
        handle => edit::knob(&drag.fixture, handle, local(point), moved),
    }
}
