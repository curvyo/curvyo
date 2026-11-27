//! The tool centre loops of a Profile operation on one item (`specs/0161-cnc-output` criteria 6
//! and 7, `adrs.md` CN3, CN11): the item's filled region offset by the tool radius, outward for
//! Outside and inward for Inside, with round joins, flattened.

use curvyo_document_core::{Length, Point, Tolerance};
use curvyo_geometry_core::{
    OffsetError, OffsetJoin, Outline, OutlineTriple, flatten_outline, offset, outline_nesting,
    signed_area_mm2,
};

use crate::operation::Side;

/// The offset's distance from the exact curve, in mm: half of the 0.01 mm budget (criterion 6).
pub const OFFSET_TOLERANCE_MM: f64 = 0.004;
/// The flattening's distance from the offset curve, in mm: the other half less the rounding.
pub const FLATTEN_TOLERANCE_MM: f64 = 0.004;

/// One outline of an item in page mm.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemOutline {
    /// The anchors `(point, handle in, handle out)` in path order.
    pub anchors: Vec<OutlineTriple>,
    /// Whether it is closed.
    pub closed: bool,
}

/// One closed tool-centre loop in page mm.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolLoop {
    /// The polyline; it does not repeat its first point.
    pub points: Vec<Point>,
    /// Whether it bounds the offset region from outside (else it bounds a hole).
    pub outer: bool,
}

/// What a Profile makes of one item.
#[derive(Debug, Clone, PartialEq)]
pub enum ItemLoops {
    /// The tool-centre loops.
    Loops(Vec<ToolLoop>),
    /// The item has an open outline: not milled (criterion 6).
    OpenPath,
    /// A piece of the region vanishes under the tool (criterion 7).
    TooNarrow,
}

/// `anchors` run the other way round.
fn reversed(anchors: &[OutlineTriple]) -> Vec<OutlineTriple> {
    anchors.iter().rev().map(|&(p, i, o)| (p, o, i)).collect()
}

/// The outlines oriented by nesting depth (CN11, Proposal): shapes positive, holes negative, so
/// the region they paint is the item's even-odd fill.
fn oriented(outlines: &[ItemOutline]) -> Vec<Vec<OutlineTriple>> {
    let views: Vec<Outline<'_>> = outlines
        .iter()
        .map(|o| Outline::new(&o.anchors, true))
        .collect();
    let nesting = outline_nesting(&views);
    let tolerance = Tolerance::from_mm(FLATTEN_TOLERANCE_MM);
    outlines
        .iter()
        .zip(nesting)
        .map(|(o, n)| {
            let area = signed_area_mm2(&flatten_outline(&o.anchors, true, tolerance));
            let want_positive = n.depth.is_multiple_of(2);
            if (area > 0.0) == want_positive {
                o.anchors.clone()
            } else {
                reversed(&o.anchors)
            }
        })
        .collect()
}

/// The tool-centre loops of `outlines` on `side` with a tool of radius `radius_mm`.
#[must_use]
pub fn profile_loops(outlines: &[ItemOutline], side: Side, radius_mm: f64) -> ItemLoops {
    if outlines.iter().any(|o| !o.closed) {
        return ItemLoops::OpenPath;
    }
    let anchors = oriented(outlines);
    let operand: Vec<Outline<'_>> = anchors.iter().map(|a| Outline::new(a, true)).collect();
    let distance = match side {
        Side::Outside => radius_mm,
        Side::Inside => -radius_mm,
    };
    let result = offset(
        &operand,
        Length::from_mm(distance),
        OffsetJoin::Round,
        Tolerance::from_mm(OFFSET_TOLERANCE_MM),
    );
    let outlines_out = match result {
        Ok(result) => result.into_outlines(),
        Err(OffsetError::Empty) => return ItemLoops::TooNarrow,
        Err(_) => return ItemLoops::Loops(Vec::new()),
    };
    if outlines_out.len() < operand.len() {
        return ItemLoops::TooNarrow;
    }
    let tolerance = Tolerance::from_mm(FLATTEN_TOLERANCE_MM);
    let loops = outlines_out
        .iter()
        .map(|outline| {
            let triples: Vec<OutlineTriple> = outline
                .iter()
                .map(|a| (a.point, a.handle_in, a.handle_out))
                .collect();
            let points = flatten_outline(&triples, true, tolerance);
            let outer = signed_area_mm2(&points) > 0.0;
            ToolLoop { points, outer }
        })
        .collect();
    ItemLoops::Loops(loops)
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::Vec2;

    fn polygon(points: &[(f64, f64)]) -> ItemOutline {
        ItemOutline {
            anchors: points
                .iter()
                .map(|&(x, y)| (Point::new(x, y), Vec2::ZERO, Vec2::ZERO))
                .collect(),
            closed: true,
        }
    }

    #[test]
    fn an_open_path_is_not_milled_and_a_narrow_slot_is_too_narrow() {
        let mut open = polygon(&[(0.0, 0.0), (10.0, 0.0)]);
        open.closed = false;
        assert_eq!(
            profile_loops(&[open], Side::Outside, 1.0),
            ItemLoops::OpenPath
        );
        let slot = polygon(&[(0.0, 0.0), (20.0, 0.0), (20.0, 2.0), (0.0, 2.0)]);
        assert_eq!(
            profile_loops(&[slot], Side::Inside, 1.5875),
            ItemLoops::TooNarrow
        );
    }
}
