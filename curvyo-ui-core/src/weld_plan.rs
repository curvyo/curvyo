//! Planning Weld on the selected objects (`specs/0068-laserable-lettering`, `adrs.md` decisions 3
//! and 4): every leaf of the selection is an operand, texts by the one conversion of `0074`, each
//! outline contributing its region or its stroke band; the result takes the lowest leaf's style
//! and the lowest selected object's place.
//!
//! Pure: it reads snapshots and the selected units and returns a plan or a refusal. The write is
//! the session's; the sentences are the host's.

use curvyo_document_core::{Length, NewAnchor, NodeId, ObjectSnapshot};
use curvyo_geometry_core::{
    BooleanError, Outline, OutlineTriple, StrokeBand, Weld, WeldOperand, outline_has_area, weld,
};

use crate::anchor_id_minter::AnchorIdMinter;
use crate::boolean::{BOOLEAN_TOLERANCE, corner_anchors, outlines_of};
use crate::text_convert::{TextRefusal, texts_as_paths, texts_refusal};
use crate::units::SelectedUnit;

/// The commit label of Weld; the undo slice maps it to "Weld".
pub const WELD_COMMIT_LABEL: &str = "weld";

/// Why Weld was refused. Nothing was changed in any of these cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeldRefusal {
    /// Texts of the selection use a missing face or are not laid out yet (criterion 27).
    Texts(TextRefusal),
    /// The contributions enclose no area together (criterion 28); every operand is outlined.
    NoArea {
        /// The operands, to outline.
        offenders: Vec<NodeId>,
        /// How many objects are selected.
        of: usize,
    },
    /// Operands have a coordinate the kernel cannot work with (criterion 28).
    OutOfRange {
        /// The offending leaves, to outline.
        offenders: Vec<NodeId>,
        /// How many selected objects hold an offender.
        objects: usize,
        /// How many objects are selected.
        of: usize,
    },
}

/// What [`plan_weld`] decided, ready for `Document::replace_with_path_at`.
#[derive(Debug, Clone, PartialEq)]
pub struct WeldPlan {
    /// Every leaf of the selection, bottom-most first; a group goes with its last leaf.
    pub removed: Vec<NodeId>,
    /// The base operand: the lowest leaf, whose complete style the result takes.
    pub style_from: NodeId,
    /// The lowest selected object (a leaf or a group): the result takes its place.
    pub place: NodeId,
    /// The name of the result: the base text's first 24 characters, when the base is a text.
    pub name: Option<String>,
    /// The result: corner-only closed outlines with fresh anchor ids.
    pub outlines: Vec<(Vec<NewAnchor>, bool)>,
    /// How many objects were selected (a group is one).
    pub objects: usize,
    /// Every selected object is a text.
    pub texts_only: bool,
    /// How many pieces the result has.
    pub pieces: usize,
    /// A dashed open outline was welded as a solid band (criterion 10).
    pub dashed: bool,
    /// Open outlines without a stroke closed with a straight line (`0054` criterion 21).
    pub closed_open: usize,
    /// Open outlines without a stroke that enclosed no area (`0054` criterion 22).
    pub removed_open: usize,
    /// The operands' styles differ, so the result shows the lowest one's.
    pub mixed_styles: bool,
}

/// One operand's outlines, split into what contributes its region and what its band.
struct Contribution {
    regions: Vec<(Vec<OutlineTriple>, bool)>,
    banded: Vec<(Vec<OutlineTriple>, bool)>,
    band: Option<StrokeBand>,
}

/// Counts of the notice, gathered while the contributions are decided.
#[derive(Default)]
struct OpenCounts {
    dashed: bool,
    closed_open: usize,
    removed_open: usize,
}

/// What `object` contributes (criteria 9 and 11): a closed outline its region; an open one its
/// band with Stroke on, and its fill area closed by a straight line with Fill on or Stroke off.
fn contribution(object: &ObjectSnapshot, counts: &mut OpenCounts) -> Contribution {
    let style = object.style();
    let stroke = &style.stroke;
    let mut regions = Vec::new();
    let mut banded = Vec::new();
    for (anchors, closed) in outlines_of(object) {
        if closed {
            regions.push((anchors, true));
            continue;
        }
        if anchors.len() < 2 {
            continue;
        }
        if stroke.enabled {
            counts.dashed |= !stroke.dash.is_solid();
            if style.fill.enabled {
                regions.push((anchors.clone(), false));
            }
            banded.push((anchors, false));
        } else {
            if outline_has_area(&Outline::new(&anchors, false)) {
                counts.closed_open += 1;
            } else {
                counts.removed_open += 1;
            }
            regions.push((anchors, false));
        }
    }
    let band = (stroke.enabled && !banded.is_empty()).then_some(StrokeBand {
        width: stroke.width,
        join: stroke.join,
        cap: stroke.cap,
    });
    Contribution {
        regions,
        banded,
        band,
    }
}

/// The kernel's view of owned outlines.
fn views_of(list: &[(Vec<OutlineTriple>, bool)]) -> Vec<Outline<'_>> {
    list.iter()
        .map(|(anchors, closed)| Outline::new(anchors, *closed))
        .collect()
}

/// The kernel's Weld over `operands` (ids `ids`, bottom first), with the notice's open-outline
/// counts; a kernel refusal as the refusal of the selected `units`.
fn run_weld(
    operands: &[&ObjectSnapshot],
    ids: &[NodeId],
    units: &[SelectedUnit],
    gap: Length,
) -> Result<(Weld, OpenCounts), WeldRefusal> {
    let mut counts = OpenCounts::default();
    let contributions: Vec<Contribution> = operands
        .iter()
        .map(|object| contribution(object, &mut counts))
        .collect();
    let views: Vec<(Vec<Outline<'_>>, Vec<Outline<'_>>)> = contributions
        .iter()
        .map(|c| (views_of(&c.regions), views_of(&c.banded)))
        .collect();
    let kernel_operands: Vec<WeldOperand<'_>> = views
        .iter()
        .zip(&contributions)
        .map(|((regions, banded), c)| WeldOperand {
            regions,
            banded,
            band: c.band,
        })
        .collect();
    let of = units.len();
    let welded = weld(&kernel_operands, gap, BOOLEAN_TOLERANCE).map_err(|error| match error {
        BooleanError::OutOfRange(indices) => {
            let offenders: Vec<NodeId> = indices.iter().map(|&i| ids[i]).collect();
            let objects = units
                .iter()
                .filter(|unit| unit.leaves.iter().any(|leaf| offenders.contains(leaf)))
                .count();
            WeldRefusal::OutOfRange {
                offenders,
                objects,
                of,
            }
        }
        _ => WeldRefusal::NoArea {
            offenders: ids.to_vec(),
            of,
        },
    })?;
    Ok((welded, counts))
}

/// Whether every selected object is a text.
fn all_texts(units: &[SelectedUnit], originals: &[&ObjectSnapshot]) -> bool {
    units.iter().all(|unit| {
        !unit.is_group
            && originals
                .iter()
                .any(|o| o.id() == unit.id && matches!(o, ObjectSnapshot::Text(_)))
    })
}

/// Plans Weld over the selected `units` (the objects of the context, each with its leaves) at the
/// gap `gap`. `objects` are the context's leaves in stacking order.
///
/// # Errors
/// A [`WeldRefusal`] with the objects to outline.
pub fn plan_weld(
    objects: &[ObjectSnapshot],
    units: &[SelectedUnit],
    gap: Length,
    minter: &mut AnchorIdMinter,
) -> Result<WeldPlan, WeldRefusal> {
    let leaves: Vec<NodeId> = units
        .iter()
        .flat_map(|unit| unit.leaves.iter().copied())
        .collect();
    if let Some(refusal) = texts_refusal(objects, &leaves) {
        return Err(WeldRefusal::Texts(refusal));
    }
    let originals: Vec<&ObjectSnapshot> = objects
        .iter()
        .filter(|o| leaves.contains(&o.id()))
        .collect();
    let converted = texts_as_paths(objects);
    let operands: Vec<&ObjectSnapshot> = converted
        .iter()
        .filter(|o| leaves.contains(&o.id()))
        .collect();
    let of = units.len();
    let (Some(base), Some(original_base)) = (operands.first(), originals.first()) else {
        return Err(WeldRefusal::NoArea {
            offenders: Vec::new(),
            of,
        });
    };
    let ids: Vec<NodeId> = operands.iter().map(|o| o.id()).collect();
    let (welded, counts) = run_weld(&operands, &ids, units, gap)?;
    let place = units
        .iter()
        .find(|unit| unit.leaves.contains(&base.id()))
        .map_or(base.id(), |unit| unit.id);
    let name = match original_base {
        ObjectSnapshot::Text(text) => Some(text.default_name()),
        _ => None,
    };
    let texts_only = all_texts(units, &originals);
    let outlines: Vec<(Vec<NewAnchor>, bool)> = welded
        .result
        .outlines()
        .iter()
        .map(|points| (corner_anchors(points, minter), true))
        .collect();
    Ok(WeldPlan {
        removed: ids,
        style_from: base.id(),
        place,
        name,
        outlines,
        objects: of,
        texts_only,
        pieces: welded.pieces,
        dashed: counts.dashed,
        closed_open: counts.closed_open,
        removed_open: counts.removed_open,
        mixed_styles: operands.iter().any(|o| o.style() != base.style()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{Document, Point, RectBounds};

    use crate::units::Units;

    fn rect(document: &Document, x: f64, y: f64, side: f64) -> NodeId {
        document.create_rect(RectBounds {
            origin: Point::new(x, y),
            width: Length::from_mm(side),
            height: Length::from_mm(side),
        })
    }

    fn objects(document: &Document) -> Vec<ObjectSnapshot> {
        document
            .object_ids()
            .into_iter()
            .filter_map(|id| document.object(id))
            .collect()
    }

    /// Criteria 13, 20 and 29: a group's leaves are operands; the place is the lowest selected
    /// object (the group), the style its lowest leaf; the selection order does not matter.
    #[test]
    fn ac13_a_group_welds_with_its_leaves_and_gives_its_place() {
        let document = Document::new(1);
        let a = rect(&document, 0.0, 0.0, 20.0);
        let b = rect(&document, 10.0, 10.0, 20.0);
        let group = document.group_objects(&[a, b]).unwrap();
        let c = rect(&document, 25.0, 0.0, 10.0);
        let units = Units::build(&document, None);
        let all = objects(&document);
        let forward = units.selected(&[a, b, c]);
        let backward = units.selected(&[c, b, a]);
        let plan = plan_weld(
            &all,
            &forward,
            Length::from_mm(0.1),
            &mut AnchorIdMinter::new(5),
        )
        .unwrap();
        let again = plan_weld(
            &all,
            &backward,
            Length::from_mm(0.1),
            &mut AnchorIdMinter::new(5),
        )
        .unwrap();
        assert_eq!(plan, again);
        assert_eq!(plan.removed, vec![a, b, c]);
        assert_eq!((plan.place, plan.style_from), (group, a));
        assert_eq!((plan.objects, plan.pieces, plan.texts_only), (2, 1, false));
        assert_eq!(plan.outlines.len(), 1);
    }
}
