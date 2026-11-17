//! The running fixture jobs of the active laser page as the job reads them
//! (`specs/0029-laser-job-preview-and-output` criteria 2, 20; ADR 0064 §1): one run per kind, in
//! run order, each with its jobs and the ticked parts' exact outlines in document space (circles
//! as cubic arcs, criterion 17: the core flattens them as every other curve).

use curvyo_document_core::{Affine, NodeId, Point, Vec2};
use curvyo_geometry_core::OutlineTriple;
use curvyo_laser_core::operations::OperationKind;
use curvyo_laser_core::{FixturePart, SlotShape};

use super::super::super::Session;
use super::super::placed::{PageFixtures, PlacedFixture};
use super::super::shape::Shape;
use super::{page_jobs, running};

/// One outline of a fixture part in document space.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::session) struct PartOutline {
    /// Its anchors, handles relative to them.
    pub anchors: Vec<OutlineTriple>,
    /// Whether it is closed.
    pub closed: bool,
}

/// Cubic arcs per circle: 22.5 degrees each, far inside 0.01 mm of the true circle.
const CIRCLE_ARCS: u32 = 16;

/// The circle of radius `r` about `centre` (local frame) as cubic arcs, mapped by `map`.
pub(in crate::session) fn circle_outline(centre: Point, r: f64, map: Affine) -> PartOutline {
    let step = std::f64::consts::TAU / f64::from(CIRCLE_ARCS);
    let k = 4.0 / 3.0 * (step / 4.0).tan() * r;
    let anchors = (0..CIRCLE_ARCS)
        .map(|i| {
            let a = step * f64::from(i);
            let (sin, cos) = a.sin_cos();
            let tangent = Vec2::new(-sin * k, cos * k);
            (
                map.apply_point(Point::new(centre.x + r * cos, centre.y + r * sin)),
                map.apply_vector(tangent.negated()),
                map.apply_vector(tangent),
            )
        })
        .collect();
    PartOutline {
        anchors,
        closed: true,
    }
}

/// A polyline as straight anchors.
fn straight(points: Vec<Point>, closed: bool) -> PartOutline {
    PartOutline {
        anchors: points
            .into_iter()
            .map(|p| (p, Vec2::ZERO, Vec2::ZERO))
            .collect(),
        closed,
    }
}

/// The running fixture jobs of one kind (UX notes P3: one run per kind).
#[derive(Debug, Clone, PartialEq)]
pub(in crate::session) struct FixtureRun {
    /// The kind they run.
    pub kind: OperationKind,
    /// The jobs, in run order.
    pub jobs: Vec<NodeId>,
    /// Each ticked part as one leaf: its fixture and its outlines.
    pub parts: Vec<(NodeId, Vec<PartOutline>)>,
}

impl Session {
    /// The running fixture jobs of the active laser page, grouped by kind in run order; empty
    /// without fixtures.
    pub(in crate::session) fn running_fixture_runs(&self) -> Vec<FixtureRun> {
        let Some(fixtures) = self.active_fixtures() else {
            return Vec::new();
        };
        let jobs = page_jobs(&fixtures);
        let mut runs: Vec<FixtureRun> = Vec::new();
        for at in running(&jobs, self.fixture_cuts_last(&fixtures)) {
            let (fixture, job) = jobs[at];
            // invariant: the run rule only runs jobs of a known kind.
            let Some(kind) = job.kind.known() else {
                continue;
            };
            let parts = job
                .parts
                .iter()
                .map(|&part| (fixture.id, self.exact_part(&fixtures, fixture, part)));
            match runs.iter_mut().find(|run| run.kind == kind) {
                Some(run) => {
                    run.jobs.push(job.id);
                    run.parts.extend(parts);
                }
                None => runs.push(FixtureRun {
                    kind,
                    jobs: vec![job.id],
                    parts: parts.collect(),
                }),
            }
        }
        runs
    }
}

impl Session {
    /// The outlines of `part` of `fixture` as the job runs them: circles exact, everything else
    /// the straight outline the canvas draws.
    fn exact_part(
        &self,
        fixtures: &PageFixtures,
        fixture: &PlacedFixture,
        part: FixturePart,
    ) -> Vec<PartOutline> {
        let map = fixture.global;
        let rings = |d: f64| {
            fixture
                .shape
                .local_centres()
                .into_iter()
                .map(|c| circle_outline(c, d / 2.0, map))
                .collect::<Vec<_>>()
        };
        match (part, &fixture.shape) {
            (FixturePart::PinCircle, Shape::Pin { diameter, .. }) => rings(diameter.as_mm()),
            (FixturePart::ScrewHead, Shape::Screw { head, .. }) => rings(head.as_mm()),
            (FixturePart::ScrewThread, Shape::Screw { thread, .. }) => rings(thread.as_mm()),
            (FixturePart::PinMarks | FixturePart::ScrewMarks, shape) => {
                let Some(grid) = shape.centre_grid() else {
                    return Vec::new();
                };
                let marks = curvyo_laser_core::centre_marks(&shape.local_centres(), grid.mark);
                let mut out: Vec<PartOutline> = marks
                    .circles
                    .iter()
                    .map(|c| circle_outline(c.centre, c.diameter.as_mm() / 2.0, map))
                    .collect();
                out.extend(marks.lines.iter().map(|l| {
                    straight(vec![map.apply_point(l.from), map.apply_point(l.to)], false)
                }));
                out
            }
            (FixturePart::Slot(index), _)
                if fixture
                    .slots
                    .get(index)
                    .is_some_and(|s| s.slot.shape != SlotShape::Rectangle) =>
            {
                let slot = &fixture.slots[index].slot;
                vec![circle_outline(
                    slot.centre,
                    slot.diameter.as_mm() / 2.0,
                    map,
                )]
            }
            _ => {
                let geometry = self.part_geometry(fixtures, fixture, part);
                geometry
                    .outlines
                    .into_iter()
                    .map(|points| straight(points, geometry.closed))
                    .collect()
            }
        }
    }
}

#[cfg(test)]
impl Session {
    /// The document-space centres of the pin or screw grid `id`, for the output tests.
    pub(in crate::session) fn fixture_centres_for_tests(&self, id: NodeId) -> Vec<Point> {
        self.page_fixture(id)
            .map(|(_, fixture)| fixture.centres())
            .unwrap_or_default()
    }
}
