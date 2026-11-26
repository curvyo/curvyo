//! The operands of the kernel commands that read every leaf of the selection (Weld,
//! `specs/0068-laserable-lettering/adrs.md` decision 6; Bridge, `specs/0156-fused-lettering`):
//! leaves at any group depth in stacking order, a text one operand per glyph.
//!
//! Pure: it reads snapshots and the selected units.

use curvyo_document_core::{NodeId, ObjectSnapshot, Style};
use curvyo_geometry_core::{Outline, OutlineTriple, outline_has_area};

use crate::boolean::outlines_of;
use crate::text_convert::text_path;
use crate::units::SelectedUnit;

/// The outlines of one operand as the kernel reads them.
pub(crate) type Owned = Vec<(Vec<OutlineTriple>, bool)>;

/// The selected leaves of `objects` in stacking order.
pub(crate) fn leaves_in_order<'a>(
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
pub(crate) fn operands_of(object: &ObjectSnapshot) -> Vec<Owned> {
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
pub(crate) fn operand_has_area(operand: &Owned) -> bool {
    operand
        .iter()
        .any(|(anchors, closed)| outline_has_area(&Outline::new(anchors, *closed)))
}

/// The base's style as line art (criterion 18, 14 A): Fill off; with Stroke off, Stroke on in
/// the fill colour at the default width of a new path.
pub(crate) fn line_art(base: &Style) -> Style {
    let mut style = base.clone();
    style.fill.enabled = false;
    if !style.stroke.enabled {
        style.stroke.enabled = true;
        style.stroke.color = base.fill.color;
        style.stroke.width = Style::default().stroke.width;
    }
    style
}

/// Every operand of `leaves` (a text one per glyph) with the leaf it comes from, bottom first.
pub(crate) fn collect_operands(leaves: &[&ObjectSnapshot]) -> (Vec<Owned>, Vec<NodeId>) {
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

/// The leaves whose geometry needs an explicit conversion first: paths of another curve kind or
/// with a width profile.
pub(crate) fn extended_path_offenders(leaves: &[&ObjectSnapshot]) -> Vec<NodeId> {
    leaves
        .iter()
        .filter(|object| {
            object.drawn_path().is_some_and(|path| {
                path.curve_kind != curvyo_document_core::CurveKind::Bezier
                    || !path.width_points.is_empty()
            })
        })
        .map(|object| object.id())
        .collect()
}
