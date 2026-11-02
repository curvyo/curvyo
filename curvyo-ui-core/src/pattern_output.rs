//! A shape's pattern as output (`specs/0071-pattern-fill` criteria 16, 24 to 26; ADR 0029 §7 to
//! §9): which objects paint one, their pieces through the kernel, the style each piece is painted
//! with, and the plan of Convert to paths. The SVG export and Convert read the pieces here, so
//! both give the same geometry.

use std::collections::HashSet;

use curvyo_document_core::{
    AnchorKind, Angle, Fill, Length, Markers, NewAnchor, NodeId, ObjectSnapshot, PatternConversion,
    PatternFill, PatternMode, Style, Vec2,
};
use curvyo_geometry_core::{
    PatternError, PatternOutline, PatternPiece, Tolerance, pattern_pieces_of,
};

use crate::AnchorIdMinter;
use crate::stroke_to_path_plan::stroked_outlines_of;

/// The kernel tolerance of patterns, as for every boolean in the editor.
pub const PATTERN_TOLERANCE: Tolerance = Tolerance::from_mm(0.01);

/// What the kernel needs of a shape with a pattern fill.
#[derive(Debug, Clone, PartialEq)]
pub struct PatternSubject {
    /// The outlines as drawn, document coordinates.
    pub outlines: Vec<PatternOutline>,
    /// The object's rotation register (the pattern's base angle).
    pub rotation: Angle,
    /// The pattern.
    pub fill: PatternFill,
    /// The stroke width: the strips of Shapes mode are this wide.
    pub line_width: Length,
}

/// The pattern subject of `object`: a path, primitive or linked clone whose fill paints a
/// pattern. Texts, images, groups and the others have none.
#[must_use]
pub fn pattern_subject(object: &ObjectSnapshot) -> Option<PatternSubject> {
    if !matches!(
        object,
        ObjectSnapshot::Path(_) | ObjectSnapshot::Primitive(_) | ObjectSnapshot::Clone(_)
    ) {
        return None;
    }
    let style = object.style();
    let fill = style.fill.painted_pattern()?;
    let (outlines, _) = stroked_outlines_of(object);
    Some(PatternSubject {
        outlines,
        rotation: object.rotation(),
        fill,
        line_width: style.stroke.width,
    })
}

/// The pieces of `object`'s pattern in document coordinates; `None` when it paints none.
#[must_use]
pub fn object_pattern_pieces(
    object: &ObjectSnapshot,
) -> Option<Result<Vec<PatternPiece>, PatternError>> {
    let subject = pattern_subject(object)?;
    Some(pattern_pieces_of(
        &subject.outlines,
        subject.rotation,
        &subject.fill,
        subject.line_width,
        PATTERN_TOLERANCE,
    ))
}

/// The style a piece is painted with (ADR 0029 §7): Lines with the shape's stroke settings (no
/// markers), drawn even while the outline's own stroke is off, so a texture can be engraved
/// without cutting the outline (`adrs.md` decision 3, lead 2026-11-02); Shapes with the fill's
/// colour and opacity and no stroke. A Lines piece is never filled.
#[must_use]
pub fn piece_style(style: &Style, mode: PatternMode) -> Style {
    let mut piece = style.clone();
    piece.stroke.markers = Markers::default();
    match mode {
        PatternMode::Lines => {
            piece.stroke.enabled = true;
            piece.fill = Fill::default();
        }
        PatternMode::Shapes => {
            piece.stroke.enabled = false;
            piece.fill = Fill {
                enabled: true,
                color: style.fill.color,
                opacity: style.fill.opacity,
                ..Fill::default()
            };
        }
    }
    piece
}

/// One piece as new path outlines with fresh anchor ids; a node with a handle is smooth.
fn piece_outlines(
    piece: &PatternPiece,
    minter: &mut AnchorIdMinter,
) -> Vec<(Vec<NewAnchor>, bool)> {
    piece
        .outlines
        .iter()
        .map(|(anchors, closed)| {
            let nodes = anchors
                .iter()
                .map(|&(point, handle_in, handle_out)| NewAnchor {
                    id: minter.mint(),
                    point,
                    handle_in,
                    handle_out,
                    kind: if handle_in == Vec2::ZERO && handle_out == Vec2::ZERO {
                        AnchorKind::Corner
                    } else {
                        AnchorKind::Asymmetric
                    },
                })
                .collect();
            (nodes, *closed)
        })
        .collect()
}

/// A path or primitive (a clone takes its style from its source and is left alone).
const fn convertible(object: &ObjectSnapshot) -> bool {
    matches!(
        object,
        ObjectSnapshot::Path(_) | ObjectSnapshot::Primitive(_)
    )
}

/// The plan of Convert pattern to paths over the selected objects (criterion 24): every selected
/// path or primitive with pieces, in stacking order. Open shapes and shapes over the limit are
/// skipped; they have nothing to convert.
#[must_use]
pub fn pattern_convert_plan(
    objects: &[ObjectSnapshot],
    selected: &[NodeId],
    minter: &mut AnchorIdMinter,
) -> Vec<PatternConversion> {
    let wanted: HashSet<NodeId> = selected.iter().copied().collect();
    objects
        .iter()
        .filter(|object| wanted.contains(&object.id()) && convertible(object))
        .filter_map(|object| {
            let fill = object.style().fill.painted_pattern()?;
            let pieces = object_pattern_pieces(object)?.ok()?;
            if pieces.is_empty() {
                return None;
            }
            let style = piece_style(object.style(), fill.mode);
            Some(PatternConversion {
                id: object.id(),
                pieces: pieces
                    .iter()
                    .map(|piece| (piece_outlines(piece, minter), style.clone()))
                    .collect(),
            })
        })
        .collect()
}
