//! Planning Combine on the selected objects: which outlines go into the one compound path and how
//! each must run so that inner shapes become holes (`specs/0035-combine-and-break-apart`,
//! `specs/0054-combine-open-paths`).
//!
//! Pure: it reads snapshots and the selection and returns a plan or a refusal. The one write
//! (`Document::replace_with_path`) is the session's. Nothing is flattened or moved: the plan holds
//! the operands' own nodes, only reversed where the winding needs it.

use curvyo_document_core::{
    AnchorId, MarkerShape, NewAnchor, NodeId, ObjectSnapshot, outline_of_rotated, reversed_anchors,
};
use curvyo_geometry_core::{
    MAX_COORDINATE_MM, Outline, OutlineTriple, outline_area_mm2, outline_has_area, outline_nesting,
    touching_outlines,
};

use crate::anchor_id_minter::AnchorIdMinter;
use crate::boolean::{BooleanAvailability, boolean_availability, operands_in_order};
use crate::object_selection::ObjectSelection;

/// The commit label of Combine; the undo slice maps it to display text.
pub const COMBINE_COMMIT_LABEL: &str = "combine_paths";

/// What the buttons of the Path card show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathAvailability {
    /// Combine is enabled with two or more selected objects, open ones included
    /// (`specs/0054-combine-open-paths` criterion 13): `NeedsTwo` or `Ready`.
    pub combine: BooleanAvailability,
    /// Split and Break apart are enabled when the selection holds a compound path: one rule, so
    /// the two are never dimmed differently (`specs/0048-split-compound-path` criterion 1).
    pub compound: bool,
    /// Cut is enabled when anything is selected: one path can cross itself
    /// (`specs/0036-cut-at-crossings` criterion 1).
    pub cut: bool,
}

/// What the Path card shows for `selection`, counting only selected ids the document still
/// holds. The one rule the buttons and the plans share. The caller passes an empty selection when
/// the Select tool is not active.
#[must_use]
pub fn path_availability(
    objects: &[ObjectSnapshot],
    selection: &ObjectSelection,
) -> PathAvailability {
    let operands = operands_in_order(objects, selection);
    PathAvailability {
        combine: match boolean_availability(objects, selection) {
            BooleanAvailability::NeedsTwo => BooleanAvailability::NeedsTwo,
            _ => BooleanAvailability::Ready,
        },
        compound: operands.iter().any(|object| is_compound(object)),
        cut: !operands.is_empty(),
    }
}

/// Whether `object` is a path with more than one outline.
pub(crate) fn is_compound(object: &ObjectSnapshot) -> bool {
    matches!(object, ObjectSnapshot::Path(path) if path.is_compound())
}

/// Why Combine was refused. Nothing was changed in any of these cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CombineRefusal {
    /// Fewer than two objects are selected.
    NeedsTwo,
    /// The selection holds a group; Combine does not look through groups
    /// (`specs/0023-groups` criterion 28). `offenders` are the leaves of the groups, to outline.
    Groups {
        /// The leaves of the selected groups, to outline.
        offenders: Vec<NodeId>,
    },
    /// `offenders` of the `of` selected objects have a coordinate that is not finite or beyond
    /// [`MAX_COORDINATE_MM`].
    OutOfRange {
        /// The objects out of range, to outline.
        offenders: Vec<NodeId>,
        /// How many objects are selected.
        of: usize,
    },
    /// Every outline of every selected object has a single node (`0054` criterion 10); nothing is
    /// outlined.
    SingleNodesOnly,
}

/// What [`plan_combine`] decided, ready for `Document::replace_with_path`.
#[derive(Debug, Clone, PartialEq)]
pub struct CombinePlan {
    /// The operands, bottom-most first.
    pub operands: Vec<NodeId>,
    /// The operand the result takes its style and place from: the bottom-most.
    pub base: NodeId,
    /// Every outline of two or more nodes of every operand with fresh anchor ids, in stacking
    /// order and each operand's own order, closed ones reversed where the winding needs it.
    pub outlines: Vec<(Vec<NewAnchor>, bool)>,
    /// How many outlines are holes (closed, odd depth).
    pub holes: usize,
    /// How many outlines are open.
    pub open: usize,
    /// How many closed outlines touch or cross another closed outline or themselves.
    pub touching: usize,
    /// How many holes touch or cross another hole.
    pub touching_holes: usize,
    /// How many outlines with a single node were left out.
    pub single_nodes_removed: usize,
    /// Whether an operand has a start, middle or end marker on (the result draws none).
    pub markers: bool,
    /// Whether the operands' complete styles are not all equal.
    pub styles_differ: bool,
}

/// One outline of an operand, with the index of the operand it belongs to.
struct Contour {
    owner: usize,
    anchors: Vec<NewAnchor>,
    closed: bool,
}

/// The outlines of `object` with their own nodes. A primitive gives the outline "Object to path"
/// would write; its anchor ids are placeholders, replaced when the plan is made.
fn contours_of(owner: usize, object: &ObjectSnapshot) -> Vec<Contour> {
    match object {
        ObjectSnapshot::Path(path)
        | ObjectSnapshot::Clone(curvyo_document_core::CloneSnapshot { resolved: path, .. }) => path
            .subpaths()
            .map(|subpath| Contour {
                owner,
                anchors: subpath.anchors.to_vec(),
                closed: subpath.closed,
            })
            .collect(),
        ObjectSnapshot::Primitive(primitive) => {
            let placeholder = AnchorId::new(0, 0);
            vec![Contour {
                owner,
                anchors: outline_of_rotated(&primitive.shape, primitive.rotation)
                    .into_iter()
                    .map(|a| NewAnchor {
                        id: placeholder,
                        point: a.point,
                        handle_in: a.handle_in,
                        handle_out: a.handle_out,
                        kind: a.kind,
                    })
                    .collect(),
                closed: true,
            }]
        }
    }
}

fn triples(anchors: &[NewAnchor]) -> Vec<OutlineTriple> {
    anchors
        .iter()
        .map(|a| (a.point, a.handle_in, a.handle_out))
        .collect()
}

/// Whether a node or the end of one of its handles is not finite or lies beyond the kernel's
/// range.
pub(crate) fn out_of_range(anchors: &[NewAnchor]) -> bool {
    anchors.iter().any(|a| {
        [
            a.point.x,
            a.point.y,
            a.point.x + a.handle_in.x,
            a.point.y + a.handle_in.y,
            a.point.x + a.handle_out.x,
            a.point.y + a.handle_out.y,
        ]
        .iter()
        .any(|value| !value.is_finite() || value.abs() > MAX_COORDINATE_MM)
    })
}

/// The ids of the operands whose flag is set, in stacking order.
fn ids_flagged(operands: &[&ObjectSnapshot], flags: &[bool]) -> Vec<NodeId> {
    let mut ids = Vec::new();
    for (object, &flag) in operands.iter().zip(flags) {
        if flag {
            ids.push(object.id());
        }
    }
    ids
}

/// The ids of the operands that own one of the `contours` picked by `bad`, in stacking order.
fn owners_where(
    operands: &[&ObjectSnapshot],
    contours: &[Contour],
    mut bad: impl FnMut(usize, &Contour) -> bool,
) -> Vec<NodeId> {
    let mut flagged = vec![false; operands.len()];
    for (index, contour) in contours.iter().enumerate() {
        if bad(index, contour) {
            flagged[contour.owner] = true;
        }
    }
    ids_flagged(operands, &flagged)
}

/// A closed outline whose enclosed area is at most this (mm²) keeps its stored direction
/// (`0054` criterion 7: a closed path of two nodes, a figure eight whose lobes cancel).
const NO_AREA_MM2: f64 = 0.001;

/// Plans Combine over the selected objects.
///
/// The stacking order decides the outline order and the base operand, never the order of the
/// selection (criterion 8). Open, touching and crossing outlines are all allowed
/// (`specs/0054-combine-open-paths`); outlines with a single node are left out and counted.
///
/// # Errors
/// A [`CombineRefusal`]: fewer than two objects, a coordinate out of range (the objects to
/// outline), or nothing but single-node outlines.
pub fn plan_combine(
    objects: &[ObjectSnapshot],
    selection: &ObjectSelection,
    minter: &mut AnchorIdMinter,
) -> Result<CombinePlan, CombineRefusal> {
    let operands = operands_in_order(objects, selection);
    let of = operands.len();
    if of < 2 {
        return Err(CombineRefusal::NeedsTwo);
    }
    let all: Vec<Contour> = operands
        .iter()
        .enumerate()
        .flat_map(|(owner, object)| contours_of(owner, object))
        .collect();
    let offenders = owners_where(&operands, &all, |_, c| out_of_range(&c.anchors));
    if !offenders.is_empty() {
        return Err(CombineRefusal::OutOfRange { offenders, of });
    }
    let (contours, single): (Vec<Contour>, Vec<Contour>) =
        all.into_iter().partition(|c| c.anchors.len() >= 2);
    if contours.is_empty() {
        return Err(CombineRefusal::SingleNodesOnly);
    }
    let windings = closed_windings(&contours);
    let outlines: Vec<(Vec<NewAnchor>, bool)> = contours
        .iter()
        .zip(&windings.reverse)
        .map(|(contour, &reverse)| {
            let anchors = if reverse {
                reversed_anchors(&contour.anchors)
            } else {
                contour.anchors.clone()
            };
            (fresh_ids(anchors, minter), contour.closed)
        })
        .collect();
    let first_style = operands[0].style();
    Ok(CombinePlan {
        operands: operands.iter().map(|object| object.id()).collect(),
        base: operands[0].id(),
        outlines,
        holes: windings.holes,
        open: contours.iter().filter(|c| !c.closed).count(),
        touching: windings.touching,
        touching_holes: windings.touching_holes,
        single_nodes_removed: single.len(),
        markers: operands.iter().any(|object| {
            let markers = object.style().stroke.markers;
            [markers.start, markers.mid, markers.end]
                .iter()
                .any(|shape| *shape != MarkerShape::None)
        }),
        styles_differ: operands.iter().any(|object| object.style() != first_style),
    })
}

/// What the closed outlines of a Combine decide: which outline to reverse and the counts of the
/// notice. Open outlines are never reversed and take part in none of it.
struct Windings {
    /// Per contour: whether it is reversed.
    reverse: Vec<bool>,
    holes: usize,
    touching: usize,
    touching_holes: usize,
}

/// Windings by depth (`0054` criterion 7): even depth runs as a shape, odd depth as a hole, by
/// the sign of the area it encloses; a closed outline with no area keeps its direction.
fn closed_windings(contours: &[Contour]) -> Windings {
    let closed: Vec<usize> = (0..contours.len())
        .filter(|&index| contours[index].closed)
        .collect();
    let owned: Vec<Vec<OutlineTriple>> = closed
        .iter()
        .map(|&index| triples(&contours[index].anchors))
        .collect();
    let outlines: Vec<Outline<'_>> = owned.iter().map(|t| Outline::new(t, true)).collect();
    let nesting = outline_nesting(&outlines);
    let is_hole: Vec<bool> = nesting.iter().map(|n| n.depth % 2 == 1).collect();
    let mut reverse = vec![false; contours.len()];
    for (k, &index) in closed.iter().enumerate() {
        let area = outline_area_mm2(&outlines[k]);
        if !outline_has_area(&outlines[k]) || area.abs() <= NO_AREA_MM2 {
            continue;
        }
        reverse[index] = (area > 0.0) == is_hole[k];
    }
    let pairs = touching_outlines(&outlines);
    let mut touches = vec![false; outlines.len()];
    let mut hole_touches = vec![false; outlines.len()];
    for &(a, b) in &pairs {
        touches[a] = true;
        touches[b] = true;
        if a != b && is_hole[a] && is_hole[b] {
            hole_touches[a] = true;
            hole_touches[b] = true;
        }
    }
    Windings {
        reverse,
        holes: is_hole.iter().filter(|&&hole| hole).count(),
        touching: touches.iter().filter(|&&t| t).count(),
        touching_holes: hole_touches.iter().filter(|&&t| t).count(),
    }
}

/// `anchors` with a fresh id on each.
pub(crate) fn fresh_ids(anchors: Vec<NewAnchor>, minter: &mut AnchorIdMinter) -> Vec<NewAnchor> {
    anchors
        .into_iter()
        .map(|anchor| NewAnchor {
            id: minter.mint(),
            ..anchor
        })
        .collect()
}
