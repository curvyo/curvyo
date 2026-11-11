//! The look of the fixture jobs' parts on the canvas under "View: Operations"
//! (`specs/0103-fixture-jobs` criterion 18; UX notes section 6): the looks and symbols of the
//! work (`0101`), per ticked part of each job with Show on, in job order, the later on top. A
//! part of an Output-off job is in the Not assigned look; an unticked part, a Show-off or an
//! Unknown job keeps the fixture ink. An engraved board takes the canvas ground (J6).

use curvyo_document_core::{
    AnchorId, Angle, NewAnchor, NodeId, ObjectSnapshot, PathSnapshot, Point, ViewTransform,
};
use curvyo_laser_core::operations::OperationKind;
use curvyo_laser_core::{FixturePart, GridSpec};
use curvyo_render_core::{
    ChipItem, DrawList, Glyph, OperationLook, Vertex, build_artwork, chips_list, hatch_list,
    restyled,
};

use super::super::super::Session;
use super::super::draw::band;
use super::super::placed::{PageFixtures, PlacedFixture, Shape, circle, slot_outline};

/// The outlines of one part in document space and whether they enclose an area.
pub(super) struct PartGeometry {
    /// Each outline as a polyline.
    pub outlines: Vec<Vec<Point>>,
    /// Whether the outlines are closed: an area the hatch covers.
    pub closed: bool,
}

/// The look of a kind on a part (`0101` ux-notes section 3).
fn kind_look(kind: OperationKind) -> OperationLook {
    match kind {
        OperationKind::Cut => OperationLook::SolidLine,
        OperationKind::Score => OperationLook::DashedLine,
        OperationKind::Engrave => OperationLook::AreaFill,
        OperationKind::EngraveOutline => OperationLook::AreaWithLine,
        OperationKind::CutOut => OperationLook::OuterLine,
    }
}

/// The symbol of a look; the Not assigned look has none.
fn glyph(look: OperationLook) -> Option<Glyph> {
    match look {
        OperationLook::SolidLine => Some(Glyph::Cut),
        OperationLook::OuterLine => Some(Glyph::CutOut),
        OperationLook::DashedLine => Some(Glyph::Score),
        OperationLook::AreaFill => Some(Glyph::Engrave),
        OperationLook::AreaWithLine => Some(Glyph::EngraveOutline),
        OperationLook::MutedDashed => None,
    }
}

fn hatched(look: OperationLook) -> bool {
    matches!(look, OperationLook::AreaFill | OperationLook::AreaWithLine)
}

/// The looks of each ticked part of `fixture`'s jobs with Show on and a known kind, in job
/// order: the kind's look while Output is on, the Not assigned look while it is off.
pub(super) fn part_looks(fixture: &PlacedFixture) -> Vec<(FixturePart, Vec<OperationLook>)> {
    let mut out: Vec<(FixturePart, Vec<OperationLook>)> = Vec::new();
    for job in fixture.jobs.iter().filter(|job| job.show) {
        let Some(kind) = job.kind.known() else {
            continue;
        };
        let look = if job.output {
            kind_look(kind)
        } else {
            OperationLook::MutedDashed
        };
        for &part in &job.parts {
            match out.iter_mut().find(|(p, _)| *p == part) {
                Some((_, looks)) => looks.push(look),
                None => out.push((part, vec![look])),
            }
        }
    }
    out
}

/// The outline of `segments` as a polygon: each segment's start.
fn polygon(segments: &[(Point, Point)], at: impl Fn(Point) -> Point) -> Vec<Point> {
    segments.iter().map(|&(a, _)| at(a)).collect()
}

impl Session {
    /// The outlines of `part` of `fixture` on its page.
    pub(super) fn part_geometry(
        &self,
        fixtures: &PageFixtures,
        fixture: &PlacedFixture,
        part: FixturePart,
    ) -> PartGeometry {
        let page = &fixtures.page;
        let to = |p: Point| fixture.global.apply_point(p);
        let closed = |outlines| PartGeometry {
            outlines,
            closed: true,
        };
        match (part, &fixture.shape) {
            (FixturePart::Slot(index), _) => closed(
                fixture
                    .slots
                    .get(index)
                    .map(|s| vec![polygon(&slot_outline(&s.slot), to)])
                    .unwrap_or_default(),
            ),
            (
                FixturePart::Stop,
                Shape::Stop {
                    leg_a,
                    leg_b,
                    thickness,
                },
            ) => {
                let (a, b, t) = (leg_a.as_mm(), leg_b.as_mm(), thickness.as_mm());
                let l = [(0.0, 0.0), (a, 0.0), (a, t), (t, t), (t, b), (0.0, b)];
                closed(vec![l.iter().map(|&(x, y)| to(Point::new(x, y))).collect()])
            }
            (FixturePart::ScrewHead | FixturePart::ScrewThread, Shape::Screw { thread, head }) => {
                let d = if part == FixturePart::ScrewHead {
                    head
                } else {
                    thread
                };
                let ring = circle(Point::new(0.0, 0.0), d.as_mm() / 2.0);
                closed(vec![polygon(&ring, to)])
            }
            (FixturePart::GridLines | FixturePart::GridTapes, Shape::Grid(spec)) => {
                self.grid_geometry(fixtures, *spec, part)
            }
            (FixturePart::Board | FixturePart::Clamp, _) => {
                closed(vec![fixture.corners(page).to_vec()])
            }
            _ => closed(Vec::new()),
        }
    }

    /// The grid's lines (open) or its switched-on tapes' bands (closed) on its page.
    fn grid_geometry(
        &self,
        fixtures: &PageFixtures,
        spec: GridSpec,
        part: FixturePart,
    ) -> PartGeometry {
        let page = &fixtures.page;
        let grid = self.fixtures.grid_cache.layout(spec, page.size);
        if part == FixturePart::GridTapes {
            let (w, h) = (page.size.width.as_mm(), page.size.height.as_mm());
            return PartGeometry {
                outlines: grid
                    .tapes
                    .iter()
                    .map(|tape| band(tape.edge, page.origin, w, h).0.to_vec())
                    .collect(),
                closed: true,
            };
        }
        let o = page.origin;
        let shift = |p: Point| Point::new(o.x + p.x, o.y + p.y);
        PartGeometry {
            outlines: grid
                .minor
                .iter()
                .chain(&grid.major)
                .map(|s| vec![shift(s.start), shift(s.end)])
                .collect(),
            closed: false,
        }
    }

    /// The boards of `fixtures` that a running Engrave kind hatches: their ground is the canvas
    /// ground (UX notes J6). None under "View: Drawn" or off the active laser page.
    pub(in crate::session::fixtures) fn engraved_boards(
        &self,
        fixtures: &PageFixtures,
    ) -> Vec<NodeId> {
        if !self.job_looks_drawn(fixtures) {
            return Vec::new();
        }
        fixtures
            .fixtures
            .iter()
            .filter(|f| f.shown())
            .filter(|f| {
                part_looks(f).iter().any(|(part, looks)| {
                    *part == FixturePart::Board && looks.iter().any(|&l| hatched(l))
                })
            })
            .map(|f| f.id)
            .collect()
    }

    /// Whether the parts' looks are drawn: the active laser page under "View: Operations".
    fn job_looks_drawn(&self, fixtures: &PageFixtures) -> bool {
        self.operations_view_on()
            && self
                .active_manufacturing()
                .is_some_and(|m| m.id == fixtures.manufacturing)
    }

    /// The triangles of the parts' looks of `fixtures`, for the fixture layer, which keeps the
    /// first fragment: the painter's order reversed, so a later look lies on top.
    pub(in crate::session::fixtures) fn fixture_job_triangles(
        &self,
        fixtures: &PageFixtures,
    ) -> Vec<Vertex> {
        if !self.job_looks_drawn(fixtures) {
            return Vec::new();
        }
        let view = self.view();
        let mut painted = DrawList::default();
        let mut chips = Vec::new();
        for fixture in fixtures.fixtures.iter().filter(|f| f.shown()) {
            let fixture = self.fixture_live_edit(fixture.id).map_or_else(
                || fixture.clone(),
                |edit| fixture.with_edit(&fixtures.page, edit),
            );
            for (part, looks) in part_looks(&fixture) {
                let geometry = self.part_geometry(fixtures, &fixture, part);
                painted.extend(part_list(&fixture, &geometry, part, &looks, view));
                if let Some(chip) = part_chip(&geometry, &looks) {
                    chips.push(chip);
                }
            }
        }
        painted.extend(chips_list(&chips, self.viewport.canvas_size(), view));
        first_fragment_order(&painted.triangles)
    }
}

/// The hatch (an engraved area, a board without its slot holes) and the lines of `looks` on one
/// part, in the painter's order.
fn part_list(
    fixture: &PlacedFixture,
    geometry: &PartGeometry,
    part: FixturePart,
    looks: &[OperationLook],
    view: ViewTransform,
) -> DrawList {
    let mut list = DrawList::default();
    for &look in looks {
        if geometry.closed && hatched(look) {
            let mut area = geometry.outlines.clone();
            if part == FixturePart::Board {
                for slot in &fixture.slots {
                    area.push(polygon(&slot_outline(&slot.slot), |p| {
                        fixture.global.apply_point(p)
                    }));
                }
            }
            list.extend(hatch_list(&area, view));
        }
        if look == OperationLook::AreaFill {
            continue;
        }
        let objects: Vec<ObjectSnapshot> = geometry
            .outlines
            .iter()
            .filter_map(|outline| {
                let path = ObjectSnapshot::Path(outline_path(fixture.id, outline, geometry.closed));
                restyled(&path, look, view)
            })
            .collect();
        list.extend(build_artwork(&objects, view));
    }
    list
}

/// The symbol chip of a part: the symbols of its Output-on looks.
fn part_chip(geometry: &PartGeometry, looks: &[OperationLook]) -> Option<ChipItem> {
    let mut glyphs: Vec<Glyph> = looks.iter().filter_map(|&l| glyph(l)).collect();
    glyphs.sort_by_key(|g| *g as u8);
    glyphs.dedup();
    let mut points = geometry.outlines.iter().flatten();
    let first = *points.next()?;
    let (min, max) = points.fold((first, first), |(lo, hi), p| {
        (
            Point::new(lo.x.min(p.x), lo.y.min(p.y)),
            Point::new(hi.x.max(p.x), hi.y.max(p.y)),
        )
    });
    (!glyphs.is_empty()).then_some(ChipItem {
        bounds: (min, max),
        glyphs,
    })
}

/// A path through `points`, the carrier of a look; never written to the document.
fn outline_path(id: NodeId, points: &[Point], closed: bool) -> PathSnapshot {
    PathSnapshot {
        id,
        closed,
        style: curvyo_document_core::Style::default(),
        anchors: points
            .iter()
            .enumerate()
            .map(|(i, &p)| NewAnchor::corner(AnchorId::new(0, i as u64), p))
            .collect(),
        extra_subpaths: Vec::new(),
        rotation: Angle::from_radians(0.0),
    }
}

/// The triangles of `painted` in reverse order: in a layer that keeps the first fragment, the
/// last painted lies on top.
fn first_fragment_order(painted: &[Vertex]) -> Vec<Vertex> {
    painted
        .as_chunks::<3>()
        .0
        .iter()
        .rev()
        .flatten()
        .copied()
        .collect()
}
