//! A laser page's fixtures read once into typed values placed on the canvas: kind, sizes, slots
//! and the map from each fixture's frame into document space (ADR 0049 §2 and §3).

use curvyo_document_core::{
    Affine, FixtureSnapshot, Length, ManufacturingSnapshot, NodeId, OriginCorner, Page, Point, Vec2,
};
use curvyo_extension_core::StoredParams;
use curvyo_laser_core::{
    FixtureKind, GridSpec, MmRange, Slot, board_side_range, grid_spec, read_slot, screw_spec,
    stop_leg_range, stop_thickness_range,
};

/// The default width and height of a new holder (criterion 2, Proposal).
pub(super) const HOLDER_SIZE_MM: (f64, f64) = (100.0, 60.0);
/// The default width and height of a new clamp (criterion 2, Proposal).
pub(super) const CLAMP_SIZE_MM: (f64, f64) = (40.0, 20.0);

/// The shape and the typed values of one fixture.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Shape {
    /// The engraved grid over the work area.
    Grid(GridSpec),
    /// A holder's board.
    Board { width: Length, height: Length },
    /// A clamp's keep-out rectangle.
    Clamp { width: Length, height: Length },
    /// An L-stop: two legs from the outer corner along +x and +y, and the thickness.
    Stop {
        leg_a: Length,
        leg_b: Length,
        thickness: Length,
    },
    /// A screw: two concentric circles about the frame's origin (criterion 37).
    Screw { thread: Length, head: Length },
    /// A kind this build does not know: drawn never, hit never (criterion 32).
    Unknown,
}

impl Shape {
    /// The stored parameter keys and values of this shape (ADR 0049 §2); none for the grid and an
    /// unknown kind, whose values are written by their own fields.
    pub(super) fn params(&self) -> Vec<(&'static str, f64)> {
        match self {
            Self::Board { width, height } | Self::Clamp { width, height } => {
                vec![("width_mm", width.as_mm()), ("height_mm", height.as_mm())]
            }
            Self::Stop {
                leg_a,
                leg_b,
                thickness,
            } => vec![
                ("leg_a_mm", leg_a.as_mm()),
                ("leg_b_mm", leg_b.as_mm()),
                ("thickness_mm", thickness.as_mm()),
            ],
            Self::Screw { thread, head } => vec![
                ("thread_diameter_mm", thread.as_mm()),
                ("head_diameter_mm", head.as_mm()),
            ],
            Self::Grid(_) | Self::Unknown => Vec::new(),
        }
    }
}

/// One readable slot of a holder.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct PlacedSlot {
    pub id: NodeId,
    pub name: String,
    pub slot: Slot,
}

/// One fixture of a laser page.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct PlacedFixture {
    pub id: NodeId,
    pub name: String,
    pub kind: FixtureKind,
    pub active: bool,
    pub visible: bool,
    /// The stored matrix, local frame to page-local mm (identity for the grid).
    pub matrix: Affine,
    /// The local frame to document space.
    pub global: Affine,
    pub shape: Shape,
    /// The readable slots; a damaged one is left out (criterion 33).
    pub slots: Vec<PlacedSlot>,
    /// How many slots did not read.
    pub damaged_slots: usize,
    /// How many values of the fixture did not read: the stored shape (document-core) and the
    /// value ranges (laser-core).
    pub damaged: usize,
}

/// The fixtures of one laser page.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct PageFixtures {
    pub manufacturing: NodeId,
    pub page: Page,
    pub fixtures: Vec<PlacedFixture>,
}

impl PageFixtures {
    /// The fixtures of `manufacturing`'s page; `None` without a page.
    pub(super) fn read(manufacturing: &ManufacturingSnapshot) -> Option<Self> {
        let page = manufacturing.page.clone()?;
        let fixtures = manufacturing
            .fixtures
            .iter()
            .map(|f| PlacedFixture::read(&page, f))
            .collect();
        Some(Self {
            manufacturing: manufacturing.id,
            page,
            fixtures,
        })
    }

    /// The fixture `id`.
    pub(super) fn get(&self, id: NodeId) -> Option<&PlacedFixture> {
        self.fixtures.iter().find(|f| f.id == id)
    }

    /// Whether the page holds a grid (one per page, criterion 2).
    pub(super) fn has_grid(&self) -> bool {
        self.fixtures
            .iter()
            .any(|f| f.kind == FixtureKind::EngravedGrid)
    }
}

/// A length parameter inside its range, or the range's default (criterion 33).
fn mm(params: &StoredParams, key: &str, range: MmRange) -> Length {
    params
        .get(key)
        .and_then(serde_json::Value::as_f64)
        .filter(|v| (range.min.as_mm()..=range.max.as_mm()).contains(v))
        .map_or(range.default, Length::from_mm)
}

/// The typed shape of a fixture of `kind` with the stored `params`; the grid counts from the
/// page grid's origin corner `corner` (Frage 174 B).
fn shape_of(kind: FixtureKind, params: &StoredParams, corner: OriginCorner) -> Shape {
    let side = |key, default| mm(params, key, board_side_range(default));
    match kind {
        FixtureKind::EngravedGrid => Shape::Grid(grid_spec(params, corner).0),
        FixtureKind::Holder => Shape::Board {
            width: side("width_mm", HOLDER_SIZE_MM.0),
            height: side("height_mm", HOLDER_SIZE_MM.1),
        },
        FixtureKind::Clamp => Shape::Clamp {
            width: side("width_mm", CLAMP_SIZE_MM.0),
            height: side("height_mm", CLAMP_SIZE_MM.1),
        },
        FixtureKind::Stop => Shape::Stop {
            leg_a: mm(params, "leg_a_mm", stop_leg_range()),
            leg_b: mm(params, "leg_b_mm", stop_leg_range()),
            thickness: mm(params, "thickness_mm", stop_thickness_range()),
        },
        FixtureKind::Screw => {
            let (thread, head, _) = screw_spec(params);
            Shape::Screw { thread, head }
        }
        FixtureKind::Unknown => Shape::Unknown,
    }
}

impl PlacedFixture {
    fn read(page: &Page, snapshot: &FixtureSnapshot) -> Self {
        let kind = FixtureKind::from_stored(&snapshot.kind);
        let params = &snapshot.params;
        let shape = shape_of(kind, params, page.grid.origin);
        let matrix = snapshot.matrix.unwrap_or(Affine::IDENTITY);
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
        Self {
            id: snapshot.id,
            name: snapshot.name.clone(),
            kind,
            active: snapshot.active,
            visible: snapshot.visible,
            matrix,
            global: to_document.compose(matrix),
            shape,
            slots,
            damaged_slots,
            damaged: snapshot.damaged + curvyo_laser_core::params_damage(kind, params),
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

    /// Whether it lies on the bed and is drawn: only then is it hit (criteria 5, 6).
    pub(super) fn shown(&self) -> bool {
        self.active && self.visible && self.kind != FixtureKind::Unknown
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
            Shape::Screw { head, .. } => {
                let r = head.as_mm() / 2.0;
                (Point::new(-r, -r), Point::new(r, r))
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
            Shape::Screw { thread, head } => {
                let mut rings = circle(Point::new(0.0, 0.0), head.as_mm() / 2.0);
                rings.extend(circle(Point::new(0.0, 0.0), thread.as_mm() / 2.0));
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
