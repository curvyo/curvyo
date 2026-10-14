//! Planning Cut on the selected objects: which operands are divided where they cross or touch,
//! what the open pieces are, and why the command was refused (`specs/0036-cut-at-crossings`,
//! criteria 2, 3, 8 to 10).
//!
//! Pure: it reads snapshots and the selection and returns a plan or a refusal. The curve work is
//! `curvyo_geometry_core::cut_parameters`, the split is `cut_pieces`, the one write
//! (`Document::replace_with_pieces`) is the session's.

use curvyo_document_core::{
    AnchorId, NewAnchor, NodeId, ObjectSnapshot, PieceOutlines, outline_of_rotated,
};
use curvyo_geometry_core::{Outline, OutlineTriple, cut_parameters};

use crate::anchor_id_minter::AnchorIdMinter;
use crate::boolean::operands_in_order;
use crate::combine::{is_compound, out_of_range};
use crate::cut_pieces::{piece_count, split_contour};
use crate::object_selection::ObjectSelection;

/// The commit label of Cut; the undo slice maps it to display text.
pub const CUT_COMMIT_LABEL: &str = "cut_at_crossings";

/// A result of more pieces than this is refused (`0036` criterion 10).
pub const MAX_CUT_PIECES: usize = 10_000;

/// Why Cut was refused. Nothing was changed in any of these cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CutRefusal {
    /// Nothing is selected (the button is dimmed then; reachable only through the session).
    NoneSelected,
    /// The selection holds a group; Cut does not look through groups (`0023` criterion 28).
    /// `offenders` are the leaves of the groups, to outline.
    Groups {
        /// The leaves of the selected groups, to outline.
        offenders: Vec<NodeId>,
    },
    /// The selection holds compound paths, which Split or Break apart must take apart first.
    Compound {
        /// The compound paths, to outline.
        offenders: Vec<NodeId>,
    },
    /// `offenders` of the `of` selected objects have a coordinate that is not finite or beyond
    /// the kernel's range.
    OutOfRange {
        /// The objects out of range, to outline.
        offenders: Vec<NodeId>,
        /// How many objects are selected.
        of: usize,
    },
    /// No selected path crosses, touches or overlaps another or itself.
    NoCrossing,
    /// The result would have `pieces` pieces, more than [`MAX_CUT_PIECES`].
    TooMany {
        /// How many pieces the cut would make.
        pieces: usize,
    },
}

/// What [`plan_cut`] decided, ready for `Document::replace_with_pieces` with `PieceFill::None`.
#[derive(Debug, Clone, PartialEq)]
pub struct CutPlan {
    /// The operands that are divided, bottom-most first.
    pub removed: Vec<NodeId>,
    /// The open pieces, each with the operand it takes its place and style from: the pieces of
    /// one operand are consecutive, in the order along its contour, with fresh anchor ids.
    pub pieces: Vec<(NodeId, PieceOutlines)>,
    /// The selected operands that are not divided and stay as they are.
    pub kept: Vec<NodeId>,
}

/// One operand's contour: a path's own, a primitive's outline as "Object to path" writes it.
struct Contour {
    id: NodeId,
    anchors: Vec<NewAnchor>,
    closed: bool,
}

fn contour_of(object: &ObjectSnapshot) -> Option<Contour> {
    match object {
        ObjectSnapshot::Path(path)
        | ObjectSnapshot::Clone(curvyo_document_core::CloneSnapshot { resolved: path, .. })
            if !path.is_compound() =>
        {
            Some(Contour {
                id: path.id,
                anchors: path.anchors.clone(),
                closed: path.closed,
            })
        }
        ObjectSnapshot::Path(_) | ObjectSnapshot::Clone(_) => None,
        ObjectSnapshot::Primitive(primitive) => {
            let placeholder = AnchorId::new(0, 0);
            let anchors = outline_of_rotated(&primitive.shape, primitive.rotation)
                .into_iter()
                .map(|a| NewAnchor {
                    id: placeholder,
                    point: a.point,
                    handle_in: a.handle_in,
                    handle_out: a.handle_out,
                    kind: a.kind,
                })
                .collect();
            Some(Contour {
                id: primitive.id,
                anchors,
                closed: true,
            })
        }
    }
}

/// Plans Cut over the selected objects.
///
/// # Errors
/// A [`CutRefusal`] with the objects to outline, if any.
pub fn plan_cut(
    objects: &[ObjectSnapshot],
    selection: &ObjectSelection,
    minter: &mut AnchorIdMinter,
) -> Result<CutPlan, CutRefusal> {
    let operands = operands_in_order(objects, selection);
    if operands.is_empty() {
        return Err(CutRefusal::NoneSelected);
    }
    let compounds: Vec<NodeId> = operands
        .iter()
        .filter(|object| is_compound(object))
        .map(|object| object.id())
        .collect();
    if !compounds.is_empty() {
        return Err(CutRefusal::Compound {
            offenders: compounds,
        });
    }
    let contours: Vec<Contour> = operands.iter().filter_map(|o| contour_of(o)).collect();
    let offenders: Vec<NodeId> = contours
        .iter()
        .filter(|contour| out_of_range(&contour.anchors))
        .map(|contour| contour.id)
        .collect();
    if !offenders.is_empty() {
        return Err(CutRefusal::OutOfRange {
            offenders,
            of: contours.len(),
        });
    }
    let owned: Vec<Vec<OutlineTriple>> = contours
        .iter()
        .map(|c| {
            c.anchors
                .iter()
                .map(|a| (a.point, a.handle_in, a.handle_out))
                .collect()
        })
        .collect();
    let outlines: Vec<Outline<'_>> = owned
        .iter()
        .zip(&contours)
        .map(|(triples, contour)| Outline::new(triples, contour.closed))
        .collect();
    let cuts = cut_parameters(&outlines);

    let total: usize = contours
        .iter()
        .zip(&cuts)
        .filter(|(_, list)| !list.is_empty())
        .map(|(contour, list)| piece_count(contour.closed, list.len()))
        .sum();
    if total == 0 {
        return Err(CutRefusal::NoCrossing);
    }
    if total > MAX_CUT_PIECES {
        return Err(CutRefusal::TooMany { pieces: total });
    }
    let mut plan = CutPlan {
        removed: Vec::new(),
        pieces: Vec::with_capacity(total),
        kept: Vec::new(),
    };
    for (contour, list) in contours.iter().zip(&cuts) {
        if list.is_empty() {
            plan.kept.push(contour.id);
            continue;
        }
        plan.removed.push(contour.id);
        for piece in split_contour(&contour.anchors, contour.closed, list, minter) {
            plan.pieces.push((contour.id, vec![(piece, false)]));
        }
    }
    Ok(plan)
}
