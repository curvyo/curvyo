//! Planning Weld on the selected objects (`specs/0068-laserable-lettering`, `adrs.md` decision
//! 6): every leaf of the selection is an operand, at any group depth, and a text one operand per
//! glyph by the one conversion of `0074`; the result takes the lowest leaf's style as line art
//! and the lowest selected object's place.
//!
//! Pure: it reads snapshots and the selected units and returns a plan or a refusal. The write is
//! the session's; the sentences are the host's.

use curvyo_document_core::{NewAnchor, NodeId, ObjectSnapshot, Point, Style};
use curvyo_geometry_core::{BooleanError, Outline, OutlineTriple, Welded, outline_has_area, weld};

use crate::anchor_id_minter::AnchorIdMinter;
use crate::boolean::{BOOLEAN_TOLERANCE, corner_anchors, open_counts, outlines_of};
use crate::text_convert::{TextRefusal, text_path, texts_refusal};
use crate::units::SelectedUnit;

/// The commit label of Weld; the undo slice maps it to "Weld".
pub const WELD_COMMIT_LABEL: &str = "weld";

/// Why Weld was refused. Nothing was changed in any of these cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeldRefusal {
    /// Texts of the selection use a missing face or are not laid out yet (criterion 23).
    Texts(TextRefusal),
    /// No operand encloses an area (criterion 24); every leaf is outlined.
    NoArea {
        /// The leaves, to outline.
        offenders: Vec<NodeId>,
        /// How many objects are selected.
        of: usize,
    },
    /// Operands have a coordinate the kernel cannot work with (criterion 24).
    OutOfRange {
        /// The offending leaves, to outline.
        offenders: Vec<NodeId>,
        /// How many selected objects hold an offender.
        objects: usize,
        /// How many objects are selected.
        of: usize,
    },
}

/// What the Weld button shows (UX notes, "States" and "Tooltip").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeldAvailability {
    /// Nothing is selected (or the caller passed nothing because the tool is not Select).
    NeedsOne,
    /// A press closes this many open outlines with a straight line (`0054` criterion 21).
    ClosesOpen {
        /// Open outlines whose closed form encloses an area.
        outlines: usize,
    },
    /// One or more objects are selected and nothing is closed.
    Ready,
}

/// What [`plan_weld`] decided, ready for `Document::replace_with_path_at`.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // one flag per sentence of the notice, read by name
pub struct WeldPlan {
    /// Every leaf of the selection, bottom-most first.
    pub removed: Vec<NodeId>,
    /// The result's style: the lowest leaf's, as line art (criterion 18, 14 A).
    pub style: Style,
    /// The lowest selected object (a leaf or a group): the result takes its place.
    pub place: NodeId,
    /// The result: corner-only outlines with fresh anchor ids, `(anchors, closed)`.
    pub outlines: Vec<(Vec<NewAnchor>, bool)>,
    /// How many objects were selected (a group is one).
    pub objects: usize,
    /// Every selected object is a text.
    pub texts_only: bool,
    /// The result's total line length in millimetres.
    pub length_mm: f64,
    /// How many operands (leaves or glyphs) were covered completely and removed.
    pub hidden: usize,
    /// Open outlines closed with a straight line (`0054` criterion 21).
    pub closed_open: usize,
    /// Open outlines that enclosed no area and were removed (`0054` criterion 22).
    pub removed_open: usize,
    /// The leaves' styles differ, so the result shows the lowest one's.
    pub mixed_styles: bool,
    /// The lowest leaf had Fill on, which the result switches off.
    pub fill_switched_off: bool,
    /// The lowest leaf had Stroke off, which the result switches on.
    pub stroke_switched_on: bool,
}

/// The outlines of one operand as the kernel reads them.
type Owned = Vec<(Vec<OutlineTriple>, bool)>;

/// The selected leaves of `objects` in stacking order.
fn leaves_in_order<'a>(
    objects: &'a [ObjectSnapshot],
    units: &[SelectedUnit],
) -> Vec<&'a ObjectSnapshot> {
    objects
        .iter()
        .filter(|object| units.iter().any(|unit| unit.leaves.contains(&object.id())))
        .collect()
}

/// The operands of one leaf: one per drawn glyph for a text (criterion 11, `0074` criterion 22),
/// else the leaf's outlines as one operand.
fn operands_of(object: &ObjectSnapshot) -> Vec<Owned> {
    let ObjectSnapshot::Text(text) = object else {
        return vec![outlines_of(object)];
    };
    let (Some(outline), Some(path)) = (text.outline.as_deref(), text_path(text)) else {
        return Vec::new();
    };
    let mut subpaths = path.subpaths();
    outline
        .glyph_outlines
        .iter()
        .map(|&count| {
            subpaths
                .by_ref()
                .take(count)
                .map(|subpath| {
                    let anchors = subpath
                        .anchors
                        .iter()
                        .map(|a| (a.point, a.handle_in, a.handle_out))
                        .collect();
                    (anchors, subpath.closed)
                })
                .collect::<Owned>()
        })
        .filter(|glyph| !glyph.is_empty())
        .collect()
}

/// Whether some outline of `operand` encloses an area once closed.
fn has_area(operand: &Owned) -> bool {
    operand
        .iter()
        .any(|(anchors, closed)| outline_has_area(&Outline::new(anchors, *closed)))
}

/// What the Weld button shows for the selected `units` of `objects` (the caller passes no units
/// when the Select tool is not active).
#[must_use]
pub fn weld_availability(objects: &[ObjectSnapshot], units: &[SelectedUnit]) -> WeldAvailability {
    let leaves = leaves_in_order(objects, units);
    if leaves.is_empty() {
        return WeldAvailability::NeedsOne;
    }
    let owned: Vec<Owned> = leaves.iter().flat_map(|leaf| operands_of(leaf)).collect();
    match open_counts(&owned) {
        (0, _) => WeldAvailability::Ready,
        (outlines, _) => WeldAvailability::ClosesOpen { outlines },
    }
}

/// The kernel's Weld over `operands` (owner leaf ids `owners`, bottom first). An operand whose
/// region the kernel finds empty is left out and the rest welded again.
fn run_weld(operands: &[Owned], owners: &[NodeId]) -> Result<Welded, (BooleanError, Vec<NodeId>)> {
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
        match weld(&slices, BOOLEAN_TOLERANCE) {
            Err(BooleanError::EmptyOperands(empty)) if empty.len() < active.len() => {
                let mut keep = 0;
                active.retain(|_| {
                    keep += 1;
                    !empty.contains(&(keep - 1))
                });
            }
            Err(error) => {
                let blamed = match &error {
                    BooleanError::OutOfRange(indices) => {
                        indices.iter().map(|&k| owners[active[k]]).collect()
                    }
                    _ => owners.to_vec(),
                };
                return Err((error, blamed));
            }
            Ok(welded) => {
                let hidden = welded.hidden.iter().map(|&k| active[k]).collect();
                return Ok(Welded { hidden, ..welded });
            }
        }
    }
}

/// The base's style as line art (criterion 18, 14 A): Fill off; with Stroke off, Stroke on in
/// the fill colour at the default width of a new path.
fn line_art(base: &Style) -> Style {
    let mut style = base.clone();
    style.fill.enabled = false;
    if !style.stroke.enabled {
        style.stroke.enabled = true;
        style.stroke.color = base.fill.color;
        style.stroke.width = Style::default().stroke.width;
    }
    style
}

/// The total length of `outlines` in millimetres, closing segments of closed ones included.
fn line_length(outlines: &[(Vec<Point>, bool)]) -> f64 {
    outlines
        .iter()
        .map(|(points, closed)| {
            let closing = closed.then(|| (points[points.len() - 1], points[0]));
            points
                .windows(2)
                .map(|pair| (pair[0], pair[1]))
                .chain(closing)
                .map(|(a, b)| ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt())
                .sum::<f64>()
        })
        .sum()
}

/// Every operand of `leaves` (a text one per glyph) with the leaf it comes from, bottom first.
fn collect_operands(leaves: &[&ObjectSnapshot]) -> (Vec<Owned>, Vec<NodeId>) {
    let mut operands: Vec<Owned> = Vec::new();
    let mut owners: Vec<NodeId> = Vec::new();
    for leaf in leaves {
        for operand in operands_of(leaf) {
            operands.push(operand);
            owners.push(leaf.id());
        }
    }
    (operands, owners)
}

/// Welds the operands that enclose an area. `Err(Some(leaves))` names the leaves out of range,
/// `Err(None)` means nothing with area is left.
fn weld_with_area(operands: &[Owned], owners: &[NodeId]) -> Result<Welded, Option<Vec<NodeId>>> {
    let with_area: Vec<usize> = (0..operands.len())
        .filter(|&i| has_area(&operands[i]))
        .collect();
    if with_area.is_empty() {
        return Err(None);
    }
    let kept: Vec<Owned> = with_area.iter().map(|&i| operands[i].clone()).collect();
    let kept_owners: Vec<NodeId> = with_area.iter().map(|&i| owners[i]).collect();
    run_weld(&kept, &kept_owners).map_err(|(error, blamed)| match error {
        BooleanError::OutOfRange(_) => Some(blamed),
        _ => None,
    })
}

/// Whether every selected object is a text.
fn texts_only(units: &[SelectedUnit], leaves: &[&ObjectSnapshot]) -> bool {
    units.iter().all(|unit| {
        !unit.is_group
            && leaves
                .iter()
                .any(|leaf| leaf.id() == unit.id && matches!(leaf, ObjectSnapshot::Text(_)))
    })
}

/// Plans Weld over the selected `units` (the objects of the context, each with its leaves).
/// `objects` are the context's leaves in stacking order.
///
/// # Errors
/// A [`WeldRefusal`] with the leaves to outline.
pub fn plan_weld(
    objects: &[ObjectSnapshot],
    units: &[SelectedUnit],
    minter: &mut AnchorIdMinter,
) -> Result<WeldPlan, WeldRefusal> {
    let leaves = leaves_in_order(objects, units);
    let ids: Vec<NodeId> = leaves.iter().map(|leaf| leaf.id()).collect();
    if let Some(refusal) = texts_refusal(objects, &ids) {
        return Err(WeldRefusal::Texts(refusal));
    }
    let of = units.len();
    let no_area = || WeldRefusal::NoArea {
        offenders: ids.clone(),
        of,
    };
    let Some(base) = leaves.first() else {
        return Err(no_area());
    };
    let (operands, owners) = collect_operands(&leaves);
    let (closed_open, removed_open) = open_counts(&operands);
    let welded = weld_with_area(&operands, &owners).map_err(|failure| match failure {
        Some(blamed) => WeldRefusal::OutOfRange {
            objects: units
                .iter()
                .filter(|unit| unit.leaves.iter().any(|leaf| blamed.contains(leaf)))
                .count(),
            offenders: blamed,
            of,
        },
        None => no_area(),
    })?;
    let base_style = base.style();
    let place = units
        .iter()
        .find(|unit| unit.leaves.contains(&base.id()))
        .map_or(base.id(), |unit| unit.id);
    Ok(WeldPlan {
        removed: ids,
        style: line_art(base_style),
        place,
        outlines: welded
            .outlines
            .iter()
            .map(|(points, closed)| (corner_anchors(points, minter), *closed))
            .collect(),
        objects: of,
        texts_only: texts_only(units, &leaves),
        length_mm: line_length(&welded.outlines),
        hidden: welded.hidden.len(),
        closed_open,
        removed_open,
        mixed_styles: leaves.iter().any(|leaf| leaf.style() != base_style),
        fill_switched_off: base_style.fill.enabled,
        stroke_switched_on: !base_style.stroke.enabled,
    })
}

#[cfg(test)]
#[path = "weld_plan_tests.rs"]
mod tests;
