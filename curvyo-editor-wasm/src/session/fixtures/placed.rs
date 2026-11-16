//! A laser page's fixtures read once into typed values placed on the canvas: kind, sizes, slots
//! and the map from each fixture's frame into document space (ADR 0049 §2 and §3).

use curvyo_document_core::{
    Affine, FixtureSnapshot, ManufacturingSnapshot, NodeId, Page, Point, Vec2,
};
use curvyo_extension_core::StoredParams;
use curvyo_laser_core::{
    FixtureHome, FixtureKind, Slot, is_locked, on_bed, read_slot, translation_part,
};

use super::shape::{Shape, shape_of};

/// One readable slot of a holder.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct PlacedSlot {
    pub id: NodeId,
    pub name: String,
    pub slot: Slot,
}

/// One fixture of a laser page.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // its checks, eye and locks, one each
pub(super) struct PlacedFixture {
    pub id: NodeId,
    pub name: String,
    pub kind: FixtureKind,
    /// Its own On the bed check (`0102` criterion 5).
    pub active: bool,
    pub visible: bool,
    /// Its set, `None` for a job fixture (`0113` criterion 14).
    pub set: Option<NodeId>,
    /// Where it lives: a set on or off this page, or the page's job fixtures.
    pub home: FixtureHome,
    /// Its own stored Lock ([`Self::locked`]).
    pub stored_locked: bool,
    /// Its set's Lock; off for a job fixture (`0113` criterion 32, amended 2026-11-16).
    pub set_locked: bool,
    /// A set fixture whose profile is not editable here: it keeps its place (criterion 46).
    pub read_only: bool,
    /// The stored matrix, local frame to page-local mm (identity for the grid).
    pub matrix: Affine,
    /// The local frame to document space.
    pub global: Affine,
    pub shape: Shape,
    /// The readable slots; a damaged one is left out (criterion 33).
    pub slots: Vec<PlacedSlot>,
    /// How many slots did not read.
    pub damaged_slots: usize,
    /// How many values of the fixture did not read: the stored shape and its jobs' values
    /// (document-core), the value ranges and the jobs' ticks of no part of the kind (laser-core).
    pub damaged: usize,
    /// How many values of its fixture jobs did not read (`specs/0103` criterion 22): the
    /// document's job values and the ticks of laser-core. The job notice counts these.
    pub job_damaged: usize,
    /// Its fixture jobs in their order (`specs/0103` criterion 3).
    pub jobs: Vec<super::jobs::PlacedJob>,
}

/// The fixtures of one laser page.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct PageFixtures {
    pub manufacturing: NodeId,
    pub page: Page,
    /// The sets in their order (`0113` criterion 6).
    pub sets: Vec<curvyo_document_core::FixtureSetSnapshot>,
    /// Every fixture in listing order: the sets' fixtures, then the job fixtures (criterion 16).
    pub fixtures: Vec<PlacedFixture>,
}

impl PageFixtures {
    /// The fixtures of `manufacturing`'s page; `None` without a page.
    pub(super) fn read(manufacturing: &ManufacturingSnapshot) -> Option<Self> {
        let page = manufacturing.page.clone()?;
        let fixtures = manufacturing
            .fixtures
            .iter()
            .map(|f| {
                let home = manufacturing
                    .set_on_page(f)
                    .map_or(FixtureHome::Job, |on_page| FixtureHome::Set { on_page });
                let mut placed = PlacedFixture::read(&page, f, home);
                placed.set_locked = f
                    .set
                    .and_then(|set| manufacturing.sets.iter().find(|s| s.id == set))
                    .is_some_and(|set| set.locked);
                placed
            })
            .collect();
        Some(Self {
            manufacturing: manufacturing.id,
            page,
            sets: manufacturing.sets.clone(),
            fixtures,
        })
    }

    /// The fixture `id`.
    pub(super) fn get(&self, id: NodeId) -> Option<&PlacedFixture> {
        self.fixtures.iter().find(|f| f.id == id)
    }

    /// Whether the set `set` holds a grid (one per set, `0113` criterion 7).
    pub(super) fn set_has_grid(&self, set: NodeId) -> bool {
        self.fixtures
            .iter()
            .any(|f| f.set == Some(set) && f.kind == FixtureKind::EngravedGrid)
    }

    /// The job fixtures in creation order (`0113` criterion 16).
    pub(super) fn job_fixtures(&self) -> impl Iterator<Item = &PlacedFixture> {
        self.fixtures.iter().filter(|f| f.set.is_none())
    }
}

/// A fixture of `kind` that is not in the document yet, at `matrix` on `page`: the hanging job
/// fixture of a placement (`0113` criterion 13). `id` stands in for its node (it has none).
pub(super) fn preview_fixture(
    id: NodeId,
    page: &Page,
    kind: FixtureKind,
    params: &StoredParams,
    matrix: Affine,
) -> PlacedFixture {
    let to_document = Affine::translation(Vec2::new(page.origin.x, page.origin.y));
    PlacedFixture {
        id,
        name: String::new(),
        kind,
        active: true,
        visible: true,
        set: None,
        home: FixtureHome::Job,
        stored_locked: false,
        set_locked: false,
        read_only: false,
        matrix,
        global: to_document.compose(matrix),
        shape: shape_of(kind, params, page.grid.origin),
        slots: Vec::new(),
        damaged_slots: 0,
        damaged: 0,
        job_damaged: 0,
        jobs: Vec::new(),
    }
}

impl PlacedFixture {
    fn read(page: &Page, snapshot: &FixtureSnapshot, home: FixtureHome) -> Self {
        let kind = FixtureKind::from_stored(&snapshot.kind);
        let params = &snapshot.params;
        let shape = shape_of(kind, params, page.grid.origin);
        let stored = snapshot.matrix.unwrap_or(Affine::IDENTITY);
        // A pin or a screw never turns: its matrix reads as its translation part (ADR 0059 §4).
        let (matrix, turned) = if shape.centre_grid().is_some() {
            translation_part(stored)
        } else {
            (stored, 0)
        };
        let to_document = Affine::translation(Vec2::new(page.origin.x, page.origin.y));
        let mut slots = Vec::new();
        let mut damaged_slots = 0;
        if kind == FixtureKind::Holder {
            for slot in &snapshot.slots {
                match read_slot(&slot.params) {
                    Some(read) => slots.push(PlacedSlot {
                        id: slot.id,
                        name: slot.name.clone(),
                        slot: read,
                    }),
                    None => damaged_slots += 1,
                }
            }
        }
        let (jobs, job_damage) = super::jobs::read_jobs(kind, snapshot.id, &slots, &snapshot.jobs);
        let snapshot_job_damage: usize = snapshot.jobs.iter().map(|job| job.damaged).sum();
        Self {
            id: snapshot.id,
            name: snapshot.name.clone(),
            kind,
            active: snapshot.active,
            visible: snapshot.visible,
            set: snapshot.set,
            home,
            stored_locked: snapshot.locked,
            set_locked: false,
            read_only: false,
            matrix,
            global: to_document.compose(matrix),
            shape,
            slots,
            damaged_slots,
            damaged: snapshot.damaged
                + curvyo_laser_core::params_damage(kind, params)
                + job_damage
                + turned,
            job_damaged: snapshot_job_damage + job_damage,
            jobs,
        }
    }

    /// This fixture with the shape, matrix and slot centres of `edit` (a drag in flight shows
    /// it there, `adrs.md` §6a.5).
    pub(super) fn with_edit(&self, page: &Page, edit: &super::edit::FixtureEdit) -> Self {
        let to_document = Affine::translation(Vec2::new(page.origin.x, page.origin.y));
        let mut slots = self.slots.clone();
        for (id, centre) in &edit.slots {
            if let Some(slot) = slots.iter_mut().find(|s| s.id == *id) {
                slot.slot.centre = *centre;
            }
        }
        Self {
            matrix: edit.matrix,
            global: to_document.compose(edit.matrix),
            shape: edit.shape.clone(),
            slots,
            ..self.clone()
        }
    }

    /// Whether it lies on the bed: its own check and, for a set fixture, its set on this page
    /// (`0113` criterion 8). Only then is it a snap or fit target and runs its jobs.
    pub(super) fn on_bed(&self) -> bool {
        on_bed(self.home, self.active)
    }

    /// Whether it lies on the bed and is drawn: only then is it hit (criteria 5, 6).
    pub(super) fn shown(&self) -> bool {
        self.on_bed() && self.visible && self.kind != FixtureKind::Unknown
    }

    /// Whether it keeps its place: its own Lock or its set's is on (`0113` criterion 32).
    pub(super) fn locked(&self) -> bool {
        is_locked(self.stored_locked, self.set_locked)
    }

    /// Whether no handle, knob, drag, nudge or chip changes its place or size: it is locked, or
    /// it is a set fixture that is read only here (criteria 32, 46).
    pub(super) fn keeps_place(&self) -> bool {
        self.locked() || self.read_only
    }

    /// Whether it is a job fixture, in no set (`0113` criterion 14).
    pub(super) fn is_job_fixture(&self) -> bool {
        matches!(self.home, FixtureHome::Job)
    }

    /// The centres of a pin or screw grid in document space; the frame origin for any other kind.
    pub(super) fn centres(&self) -> Vec<Point> {
        self.shape
            .local_centres()
            .into_iter()
            .map(|p| self.global.apply_point(p))
            .collect()
    }

    /// Whether `p` lies within `tolerance` of a ring of a pin or screw grid, or inside a pin;
    /// `None` for any other shape. Each centre is tried by distance, not by chords (budget `0113`
    /// criterion 42a).
    pub(super) fn near_rings(&self, p: Point, tolerance: f64) -> Option<bool> {
        self.shape.centre_grid()?;
        let radii: Vec<f64> = self
            .shape
            .centre_diameters()
            .iter()
            .map(|d| d.as_mm() / 2.0)
            .collect();
        // A pin is a solid dowel: inside its circle hits too (UX review 2026-11-16).
        let solid = matches!(self.shape, Shape::Pin { .. });
        Some(self.centres().into_iter().any(|c| {
            let d = c.vector_to(p).length();
            radii
                .iter()
                .any(|r| (d - r).abs() <= tolerance || (solid && d <= *r))
        }))
    }

    /// The size of its box in its own frame; the stop's is the box of its two legs, the
    /// screw's the square around its head.
    pub(super) fn local_size(&self, page: &Page) -> (f64, f64) {
        let (min, max) = self.local_box(page);
        (max.x - min.x, max.y - min.y)
    }

    /// Its box in its own frame, `(min, max)`: from the origin for a board, a clamp and a stop;
    /// about the origin for a screw, whose origin is its centre (ADR 0049 §2).
    pub(super) fn local_box(&self, page: &Page) -> (Point, Point) {
        let origin = Point::new(0.0, 0.0);
        let to = |w: f64, h: f64| (origin, Point::new(w, h));
        match &self.shape {
            Shape::Grid(_) => to(page.size.width.as_mm(), page.size.height.as_mm()),
            Shape::Board { width, height } | Shape::Clamp { width, height } => {
                to(width.as_mm(), height.as_mm())
            }
            Shape::Stop { leg_a, leg_b, .. } => to(leg_a.as_mm(), leg_b.as_mm()),
            Shape::Screw { grid, .. } | Shape::Pin { grid, .. } => {
                let r = self.shape.centre_radius();
                let w = f64::from(grid.counts.columns() - 1) * grid.column_spacing.as_mm();
                let h = f64::from(grid.counts.rows() - 1) * grid.row_spacing.as_mm();
                (Point::new(-r, -r), Point::new(w + r, h + r))
            }
            Shape::Unknown => (origin, origin),
        }
    }

    /// The corners of its box in document space, clockwise from its frame's minimum corner.
    pub(super) fn corners(&self, page: &Page) -> [Point; 4] {
        let (min, max) = self.local_box(page);
        [
            (min.x, min.y),
            (max.x, min.y),
            (max.x, max.y),
            (min.x, max.y),
        ]
        .map(|(x, y)| self.global.apply_point(Point::new(x, y)))
    }

    /// The segments a press hits (criteria 8, 9): the rectangle's outline, the stop's L, every
    /// slot's outline; the grid's work-area edge.
    pub(super) fn outline(&self, page: &Page) -> Vec<(Point, Point)> {
        let mut local: Vec<(Point, Point)> = match &self.shape {
            Shape::Stop {
                leg_a,
                leg_b,
                thickness,
            } => {
                let (a, b, t) = (leg_a.as_mm(), leg_b.as_mm(), thickness.as_mm());
                closed(&[(0.0, 0.0), (a, 0.0), (a, t), (t, t), (t, b), (0.0, b)])
            }
            Shape::Screw { .. } | Shape::Pin { .. } => {
                let mut rings = Vec::new();
                for centre in self.shape.local_centres() {
                    for d in self.shape.centre_diameters() {
                        rings.extend(circle(centre, d.as_mm() / 2.0));
                    }
                }
                rings
            }
            Shape::Unknown => Vec::new(),
            _ => {
                let (w, h) = self.local_size(page);
                closed(&[(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)])
            }
        };
        for slot in &self.slots {
            local.extend(slot_outline(&slot.slot));
        }
        local
            .into_iter()
            .map(|(a, b)| (self.global.apply_point(a), self.global.apply_point(b)))
            .collect()
    }
}

/// The edges of a closed polygon.
fn closed(points: &[(f64, f64)]) -> Vec<(Point, Point)> {
    (0..points.len())
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % points.len()]);
            (Point::new(a.0, a.1), Point::new(b.0, b.1))
        })
        .collect()
}

/// A slot's outline in the board's frame: its rectangle, or a circle as 48 chords.
pub(super) fn slot_outline(slot: &Slot) -> Vec<(Point, Point)> {
    let (w, h) = slot.extent();
    let (cx, cy) = (slot.centre.x, slot.centre.y);
    if slot.shape == curvyo_laser_core::SlotShape::Rectangle {
        let (hw, hh) = (w.as_mm() / 2.0, h.as_mm() / 2.0);
        return closed(&[
            (cx - hw, cy - hh),
            (cx + hw, cy - hh),
            (cx + hw, cy + hh),
            (cx - hw, cy + hh),
        ]);
    }
    circle(slot.centre, w.as_mm() / 2.0)
}

/// A circle of radius `r` about `centre` as 48 chords.
pub(super) fn circle(centre: Point, r: f64) -> Vec<(Point, Point)> {
    let points: Vec<(f64, f64)> = (0..48)
        .map(|k| {
            let a = std::f64::consts::TAU * f64::from(k) / 48.0;
            (centre.x + r * a.cos(), centre.y + r * a.sin())
        })
        .collect();
    closed(&points)
}
