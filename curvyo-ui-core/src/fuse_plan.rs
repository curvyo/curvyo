//! Planning Bridge on the selected objects (`specs/0156-fused-lettering`, `adrs.md` decision 5):
//! the operands as Weld reads them, the field values and their limits, and the result as one
//! group of an Outline and its Bridges with the lowest leaf's style as line art at the top-most
//! selected object's place.
//!
//! Pure: it reads snapshots and the selected units and returns a plan or a refusal. The write is
//! the session's; the sentences are the host's.

use curvyo_document_core::{Length, NodeId, ObjectSnapshot, PieceOutlines, Point, Style};
use curvyo_geometry_core::{BooleanError, FuseError, Fused, Outline, fuse_cut_out, fuse_stencil};

use crate::anchor_id_minter::AnchorIdMinter;
use crate::boolean::{BOOLEAN_TOLERANCE, corner_anchors};
use crate::kernel_operands::{
    Owned, collect_operands, extended_path_offenders, leaves_in_order, line_art, operand_has_area,
};
use crate::text_convert::{TextRefusal, area_refusal};
use crate::units::SelectedUnit;

/// The commit label of Bridge; the undo slice maps it to "Bridge".
pub const FUSE_COMMIT_LABEL: &str = "fuse";

/// The most bridges one press makes (criterion 17).
pub const FUSE_MAX_BRIDGES: usize = 2000;

/// The Width field: 0.2 to 10 mm, 1.0 by default (criterion 5).
pub const FUSE_WIDTH_MM: FieldRange = FieldRange {
    min: 0.2,
    max: 10.0,
    default: 1.0,
};

/// The Reach field: 0.5 to 50 mm, 5.0 by default (criterion 5).
pub const FUSE_REACH_MM: FieldRange = FieldRange {
    min: 0.5,
    max: 50.0,
    default: 5.0,
};

/// The Bridges per island field: 1 to 4, 2 by default (criterion 21, Frage 397 A).
pub const FUSE_BRIDGES_PER_ISLAND: FieldRange = FieldRange {
    min: 1.0,
    max: 4.0,
    default: 2.0,
};

/// The limits and default of one field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldRange {
    /// The smallest value.
    pub min: f64,
    /// The largest value.
    pub max: f64,
    /// The first-use value.
    pub default: f64,
}

impl FieldRange {
    /// `value` within the limits; a value that is not a number reads as the default.
    #[must_use]
    pub fn clamp(self, value: f64) -> f64 {
        if value.is_nan() {
            self.default
        } else {
            value.clamp(self.min, self.max)
        }
    }
}

/// Which bridges Bridge makes (criterion 21).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuseMode {
    /// The lettering is the part: pieces are joined (slice 1).
    Lettering,
    /// The lettering is the hole: islands are held to the sheet (slice 2).
    Stencil,
}

/// The values of the Bridge fields, as the host remembers them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FuseSettings {
    /// Lettering or Stencil.
    pub mode: FuseMode,
    /// The neck width in millimetres.
    pub width_mm: f64,
    /// The largest gap bridged in Lettering mode, in millimetres.
    pub reach_mm: f64,
    /// Join all: every piece is joined whatever its gap (Frage 398 C).
    pub join_all: bool,
    /// Bridges per island in Stencil mode.
    pub bridges_per_island: f64,
}

impl FuseSettings {
    /// The settings with every number within its field's limits (`adrs.md` decision 6).
    #[must_use]
    pub fn clamped(self) -> Self {
        Self {
            width_mm: FUSE_WIDTH_MM.clamp(self.width_mm),
            reach_mm: FUSE_REACH_MM.clamp(self.reach_mm),
            bridges_per_island: FUSE_BRIDGES_PER_ISLAND
                .clamp(self.bridges_per_island)
                .round(),
            ..self
        }
    }
}

/// Why Bridge was refused by the plan. Nothing was changed in any of these cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FuseRefusal {
    /// Editable geometry requires explicit conversion first.
    ExtendedPath {
        /// Objects requiring conversion.
        offenders: Vec<NodeId>,
        /// Number of selected objects.
        of: usize,
    },
    /// Texts of the selection use a missing face, are not laid out yet or have no area.
    Texts(TextRefusal),
    /// No operand encloses an area (`0016` criterion 17a).
    NoArea {
        /// The leaves, to outline.
        offenders: Vec<NodeId>,
        /// How many objects are selected.
        of: usize,
    },
    /// Operands have a coordinate the kernel cannot work with (`0016` criterion 16).
    OutOfRange {
        /// The offending leaves, to outline.
        offenders: Vec<NodeId>,
        /// How many selected objects hold an offender.
        objects: usize,
        /// How many objects are selected.
        of: usize,
    },
    /// No two pieces lie within Reach, or there is one piece (criterion 17).
    NothingToJoin,
    /// More than [`FUSE_MAX_BRIDGES`] bridges (criterion 17).
    TooManyBridges,
    /// Stencil: no islands (criterion 22).
    NoIslands,
}

/// What [`plan_fuse`] decided, ready for `Document::replace_with_group`.
#[derive(Debug, Clone, PartialEq)]
pub struct FusePlan {
    /// Every leaf of the selection, bottom-most first.
    pub removed: Vec<NodeId>,
    /// The bottom-most leaf: its effective assignment goes to the Outline (criterion 14).
    pub bottom: NodeId,
    /// The top-most selected object (a leaf or a group): the result takes its place.
    pub place: NodeId,
    /// The group's name: "Bridged lettering" or "Bridged stencil".
    pub group_name: &'static str,
    /// The style of the Outline and the Bridges: the lowest leaf's, as line art.
    pub style: Style,
    /// The Outline's outlines with fresh anchor ids.
    pub outline: PieceOutlines,
    /// The Bridges, "Bridge 1" first, with fresh anchor ids.
    pub bridges: Vec<(String, PieceOutlines)>,
}

/// The kernel's Bridge over `operands` (owner leaf ids `owners`). An operand whose region the
/// kernel finds empty is left out and the rest bridged again.
fn run_fuse(
    operands: &[Owned],
    owners: &[NodeId],
    settings: FuseSettings,
) -> Result<Fused, (FuseError, Vec<NodeId>)> {
    let mut active: Vec<usize> = (0..operands.len()).collect();
    loop {
        let views: Vec<Vec<Outline<'_>>> = active
            .iter()
            .map(|&i| {
                operands[i]
                    .iter()
                    .map(|(anchors, closed)| Outline::new(anchors, *closed))
                    .collect()
            })
            .collect();
        let slices: Vec<&[Outline<'_>]> = views.iter().map(Vec::as_slice).collect();
        let width = Length::from_mm(settings.width_mm);
        let result = match settings.mode {
            FuseMode::Lettering => fuse_cut_out(
                &slices,
                width,
                (!settings.join_all).then(|| Length::from_mm(settings.reach_mm)),
                FUSE_MAX_BRIDGES,
                BOOLEAN_TOLERANCE,
            ),
            FuseMode::Stencil => {
                // invariant: clamped to 1..=4 above.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let per_island = settings.bridges_per_island as u8;
                fuse_stencil(
                    &slices,
                    width,
                    per_island,
                    FUSE_MAX_BRIDGES,
                    BOOLEAN_TOLERANCE,
                )
            }
        };
        match result {
            Err(FuseError::Operands(BooleanError::EmptyOperands(empty)))
                if empty.len() < active.len() =>
            {
                let mut index = 0;
                active.retain(|_| {
                    index += 1;
                    !empty.contains(&(index - 1))
                });
            }
            Err(error) => {
                let blamed = match &error {
                    FuseError::Operands(BooleanError::OutOfRange(indices)) => {
                        indices.iter().map(|&k| owners[active[k]]).collect()
                    }
                    _ => owners.to_vec(),
                };
                return Err((error, blamed));
            }
            Ok(fused) => return Ok(fused),
        }
    }
}

/// `outlines` as closed corner outlines with fresh anchor ids.
fn closed_outlines(outlines: &[Vec<Point>], minter: &mut AnchorIdMinter) -> PieceOutlines {
    outlines
        .iter()
        .map(|points| (corner_anchors(points, minter), true))
        .collect()
}

/// The refusal for a kernel `error` that blames the leaves `blamed`; `ids` are every leaf.
fn refusal_of(
    error: &FuseError,
    blamed: Vec<NodeId>,
    units: &[SelectedUnit],
    ids: &[NodeId],
) -> FuseRefusal {
    let of = units.len();
    match error {
        FuseError::Operands(BooleanError::OutOfRange(_)) => FuseRefusal::OutOfRange {
            objects: units
                .iter()
                .filter(|unit| unit.leaves.iter().any(|leaf| blamed.contains(leaf)))
                .count(),
            offenders: blamed,
            of,
        },
        FuseError::NothingToJoin => FuseRefusal::NothingToJoin,
        FuseError::TooManyBridges => FuseRefusal::TooManyBridges,
        FuseError::NoIslands => FuseRefusal::NoIslands,
        FuseError::Operands(_) => FuseRefusal::NoArea {
            offenders: ids.to_vec(),
            of,
        },
    }
}

/// Plans Bridge over the selected `units` (the objects of the context, each with its leaves)
/// with `settings` (clamped here). `objects` are the context's leaves in stacking order.
///
/// # Errors
/// A [`FuseRefusal`].
pub fn plan_fuse(
    objects: &[ObjectSnapshot],
    units: &[SelectedUnit],
    settings: FuseSettings,
    minter: &mut AnchorIdMinter,
) -> Result<FusePlan, FuseRefusal> {
    let settings = settings.clamped();
    let leaves = leaves_in_order(objects, units);
    let ids: Vec<NodeId> = leaves.iter().map(|leaf| leaf.id()).collect();
    let of = units.len();
    let offenders = extended_path_offenders(&leaves);
    if !offenders.is_empty() {
        return Err(FuseRefusal::ExtendedPath { offenders, of });
    }
    if let Some(refusal) = area_refusal(objects, &ids) {
        return Err(FuseRefusal::Texts(refusal));
    }
    let no_area = || FuseRefusal::NoArea {
        offenders: ids.clone(),
        of,
    };
    let (Some(base), Some(top)) = (leaves.first(), leaves.last()) else {
        return Err(no_area());
    };
    let (operands, owners) = collect_operands(&leaves);
    let with_area: Vec<usize> = (0..operands.len())
        .filter(|&i| operand_has_area(&operands[i]))
        .collect();
    if with_area.is_empty() {
        return Err(no_area());
    }
    let kept: Vec<Owned> = with_area.iter().map(|&i| operands[i].clone()).collect();
    let kept_owners: Vec<NodeId> = with_area.iter().map(|&i| owners[i]).collect();
    let fused = run_fuse(&kept, &kept_owners, settings)
        .map_err(|(error, blamed)| refusal_of(&error, blamed, units, &ids))?;
    let place = units
        .iter()
        .find(|unit| unit.leaves.contains(&top.id()))
        .map_or(top.id(), |unit| unit.id);
    let outline = closed_outlines(&fused.outline, minter);
    let bridges = fused
        .connectors
        .iter()
        .zip(1..)
        .map(|(connector, n)| (format!("Bridge {n}"), closed_outlines(connector, minter)))
        .collect();
    Ok(FusePlan {
        removed: ids,
        bottom: base.id(),
        place,
        group_name: match settings.mode {
            FuseMode::Lettering => "Bridged lettering",
            FuseMode::Stencil => "Bridged stencil",
        },
        style: line_art(base.style()),
        outline,
        bridges,
    })
}

#[cfg(test)]
#[path = "fuse_plan_tests.rs"]
mod tests;
