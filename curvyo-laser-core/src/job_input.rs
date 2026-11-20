//! The input the session passes to the job builder (ADR 0064 §1): types only, every leaf already
//! in page space, every block already in run order.

use std::sync::Arc;

use curvyo_document_core::{AssetHash, DocumentSize, Length, NodeId, PlacedGradient, Point};
use curvyo_geometry_core::OutlineTriple;
use curvyo_image_core::WorkingImage;

use crate::image_adjust_resolve::AdjustRegisters;
use crate::machine_profile::ProfileValues;
use crate::operation_kind::KindRead;
use crate::operation_options::{OperationOptions, OptionsDamage};
use crate::operation_settings::SettingsRead;
use crate::output_settings::Start;

/// A point in whole micrometres of the page (Y grows down), the unit of every move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PointUm {
    /// X in micrometres from the page's left edge.
    pub x: i64,
    /// Y in micrometres from the page's top edge.
    pub y: i64,
}

impl PointUm {
    /// The point at `(x, y)` micrometres.
    #[must_use]
    pub const fn new(x: i64, y: i64) -> Self {
        Self { x, y }
    }

    /// The page point `point` (mm) rounded to whole micrometres (ADR 0064 §2).
    #[must_use]
    pub fn of_mm(point: Point) -> Self {
        Self::new(um_of_mm(point.x), um_of_mm(point.y))
    }

    /// The point in mm.
    #[must_use]
    pub fn to_mm(self) -> Point {
        Point::new(mm_of_um(self.x), mm_of_um(self.y))
    }

    /// The squared distance to `other` in µm², exact.
    #[must_use]
    pub const fn distance_squared(self, other: Self) -> i128 {
        let (dx, dy) = ((other.x - self.x) as i128, (other.y - self.y) as i128);
        dx * dx + dy * dy
    }

    /// The distance to `other` in mm. Uses `sqrt`, not `f64::hypot`: `sqrt` is correctly rounded
    /// by IEEE 754 on every target while `hypot` comes from the platform's libm, and job bytes
    /// must not differ between native and wasm32.
    #[must_use]
    pub fn distance_mm(self, other: Self) -> f64 {
        let (dx, dy) = (mm_of_um(self.x - other.x), mm_of_um(self.y - other.y));
        (dx * dx + dy * dy).sqrt()
    }
}

/// `mm` rounded to whole micrometres; a value beyond the range of `i64` saturates.
#[must_use]
pub fn um_of_mm(mm: f64) -> i64 {
    #[allow(clippy::cast_possible_truncation)]
    // invariant: `as` saturates; page coordinates are far inside the range (10 m beds)
    let um = (mm * 1000.0).round() as i64;
    um
}

/// `um` micrometres in mm.
#[must_use]
pub fn mm_of_um(um: i64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    // exact: page coordinates in µm are far below 2^53
    let mm = um as f64 / 1000.0;
    mm
}

/// One outline of a leaf in page mm, handles relative to their anchor.
#[derive(Debug, Clone, PartialEq)]
pub struct LeafOutline {
    /// The anchors in path order.
    pub anchors: Vec<OutlineTriple>,
    /// Whether the outline is closed.
    pub closed: bool,
}

/// The geometry of one leaf as it stands on the laser page.
#[derive(Debug, Clone, PartialEq)]
pub enum LeafGeometry {
    /// A path, text, primitive, clone or placed page member, resolved to page-space outlines.
    Outlines(Vec<LeafOutline>),
    /// An image: its frame's four corners in drawing order (the picture's top-left, top-right,
    /// bottom-right, bottom-left) and its working image.
    Image {
        /// The frame corners.
        frame: [Point; 4],
        /// The asset the pixels come from (the key of its grey pyramid).
        asset: AssetHash,
        /// The decoded working image; `None` when its bytes cannot be read (skipped, a warn).
        pixels: Option<Arc<WorkingImage>>,
        /// The adjustments the picture shows (`specs/0135` criteria 19, 61: a clone's effective
        /// ones); the default is no adjustment.
        adjust: AdjustRegisters,
    },
    /// A path, text or primitive whose fill paint is a gradient (`specs/0123` criterion 3): its
    /// outlines and the gradient placed on the page.
    Gradient {
        /// The outlines in page mm.
        outlines: Vec<LeafOutline>,
        /// The gradient in page mm.
        gradient: PlacedGradient,
    },
}

/// One leaf that runs in a block by the nearest rule.
#[derive(Debug, Clone, PartialEq)]
pub struct LeafInput {
    /// The leaf's node.
    pub node: NodeId,
    /// Its stacking order on the page, lower first; breaks ties of the order (criterion 22).
    pub stacking: u32,
    /// Its geometry.
    pub geometry: LeafGeometry,
}

/// One item of a Cut-out block: the leaves whose union's outer contour runs (criterion 16).
#[derive(Debug, Clone, PartialEq)]
pub struct CutOutItem {
    /// The item's node.
    pub item: NodeId,
    /// Its stacking order on the page.
    pub stacking: u32,
    /// The outlines of each of its leaves.
    pub leaves: Vec<Vec<LeafOutline>>,
}

/// Where a block's numbers come from.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockSource {
    /// A work operation: its stored settings and options.
    Operation {
        /// The read `op_settings`.
        settings: SettingsRead,
        /// The read `op_options`.
        options: Result<OperationOptions, OptionsDamage>,
        /// The Cut-out margin (0 for the other kinds).
        margin: Length,
        /// The test grid whose derived operation this is (ADR 0057 §5), `None` for any other.
        grid: Option<NodeId>,
    },
    /// The running fixture jobs of one kind: the page's fixture material pick (Frage 221 B).
    Fixture {
        /// The jobs, in run order (pins' role jobs included).
        jobs: Vec<NodeId>,
    },
}

/// What a run's ring on the page and in its row shows (UX P26): the operation row's digit, or
/// the fixture glyph for the fixture jobs of one kind. The core counts no run numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ring {
    /// The operation row's digit in the Operations tab (1 based), as the session passes it.
    Operation(u32),
    /// The fixture jobs of one kind.
    Fixture,
}

/// One run of the job: an operation, or the running fixture jobs of one kind.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockInput {
    /// The operation's node, or the first fixture job's node.
    pub id: NodeId,
    /// Its ring.
    pub ring: Ring,
    /// Its name ("Cut 1"; "Fixture jobs · Score").
    pub name: String,
    /// Its kind; Unknown runs nothing.
    pub kind: KindRead,
    /// Where its numbers come from.
    pub source: BlockSource,
    /// The leaves that run in it (all kinds but Cut-out).
    pub leaves: Vec<LeafInput>,
    /// The items of a Cut-out block.
    pub cut_out: Vec<CutOutItem>,
}

impl BlockInput {
    /// Whether the block has anything assigned.
    #[must_use]
    pub fn has_items(&self) -> bool {
        !self.leaves.is_empty() || !self.cut_out.is_empty()
    }
}

/// The page's machine: its profile copy.
#[derive(Debug, Clone, PartialEq)]
pub struct MachineInput {
    /// The copy's profile id (`generic-laser` for the built-in).
    pub profile_id: String,
    /// The copied values.
    pub values: ProfileValues,
    /// The work area, which is the page.
    pub bed: DocumentSize,
    /// Whether the copy is damaged or unreadable (block 4 c).
    pub damaged: bool,
}

/// Everything the job of one laser page is built from.
#[derive(Debug, Clone, PartialEq)]
pub struct JobInput {
    /// The job's name for the file header (the page name).
    pub name: String,
    /// The machine.
    pub machine: MachineInput,
    /// How the job starts (Machine origin when the register is damaged).
    pub start: Start,
    /// Whether the page's Start register is damaged (block `StartDamaged`).
    pub start_damaged: bool,
    /// The material id the fixture jobs run with, `None` without a pick.
    pub fixture_material: Option<String>,
    /// The runs, in run order: the fixture blocks, then the operations.
    pub blocks: Vec<BlockInput>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Determinism note (ADR 0064): job bytes must match on native and wasm32, so distances use
    /// `sqrt` (correctly rounded everywhere) instead of libm's `hypot`. This pins the bits of a
    /// non-Pythagorean distance; the GRBL goldens pin the written bytes on every CI target.
    #[test]
    fn the_distance_is_the_correctly_rounded_square_root() {
        let d = PointUm::new(0, 0).distance_mm(PointUm::new(1_000, 1_000));
        assert_eq!(d.to_bits(), 2_f64.sqrt().to_bits());
        let d = PointUm::new(1_234, -5_678).distance_mm(PointUm::new(-9_876, 5_432));
        let (dx, dy) = (11.11_f64, 11.11_f64);
        assert_eq!(d.to_bits(), (dx * dx + dy * dy).sqrt().to_bits());
        assert!((PointUm::new(0, 0).distance_mm(PointUm::new(3_000, 4_000)) - 5.0).abs() < 1e-12);
    }
}
