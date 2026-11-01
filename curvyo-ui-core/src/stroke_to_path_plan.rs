//! Planning Stroke to path on the selected objects (`specs/0085-stroke-to-path` criteria 13 to
//! 18, 22, 25 to 27; `adrs.md` decision 3): the subjects through groups at any depth, texts one
//! subject per glyph by the one conversion of `0074`, clones as they show, the skips and counts of
//! the notice, the refusals, and the kernel's stroke outline per subject with fresh anchor ids.
//!
//! Pure: it reads snapshots and returns a plan or a refusal. The write is the session's; the
//! sentences are the host's.

use std::collections::HashSet;

use curvyo_document_core::{
    AnchorKind, CopySource, NewAnchor, NodeId, ObjectSnapshot, PathSnapshot, PieceOutlines,
    StrokeOutlineWrite, StrokeSubject, StrokeToPath, Style, TextConversion,
    drawing_outline_of_rotated,
};
use curvyo_geometry_core::{
    OffsetAnchor, Outline, OutlineTriple, StrokeOutlineError, StrokeShape, stroke_outline,
};

use crate::anchor_id_minter::AnchorIdMinter;
use crate::boolean::BOOLEAN_TOLERANCE;
use crate::text_convert::{TextRefusal, build_text_conversions};

/// A stroke this wide or narrower paints nothing (`specs/0085-stroke-to-path`, "Subject").
const MIN_STROKE_WIDTH_MM: f64 = 0.002;

/// Why Stroke to path was refused. Nothing was changed in any of these cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrokeToPathRefusal {
    /// Texts of the selection use a missing face or are not laid out yet (criterion 25).
    Texts(TextRefusal),
    /// Subjects have a coordinate the kernel cannot work with (criterion 26).
    OutOfRange {
        /// The offending objects, to outline.
        offenders: Vec<NodeId>,
    },
}

/// What [`plan_stroke_to_path`] decided, ready for `Document::stroke_to_path`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StrokeToPathPlan {
    /// The write; empty outlines when no stroke paints anything.
    pub write: StrokeToPath,
    /// Strokes that became filled outlines (a text counts each glyph's stroke once per text).
    pub strokes: usize,
    /// Selected objects with no stroke (or a stroke without area) left as they are.
    pub unstroked: usize,
    /// Texts converted to paths.
    pub texts: usize,
    /// Clones unlinked.
    pub unlinked: usize,
}

/// Whether `style` paints a stroke: on, wider than 0.002 mm, not transparent.
#[must_use]
pub fn draws_stroke(style: &Style) -> bool {
    let stroke = &style.stroke;
    stroke.enabled && stroke.width.as_mm() > MIN_STROKE_WIDTH_MM && stroke.opacity.get() > 0.0
}

/// The selected leaves of `objects` that may be subjects, in stacking order: in `selected`, not
/// skipped (hidden, locked or a guide).
fn candidates<'a>(
    objects: &'a [ObjectSnapshot],
    selected: &[NodeId],
    skipped: &dyn Fn(NodeId) -> bool,
) -> Vec<&'a ObjectSnapshot> {
    let wanted: HashSet<NodeId> = selected.iter().copied().collect();
    objects
        .iter()
        .filter(|object| wanted.contains(&object.id()) && !skipped(object.id()))
        .collect()
}

/// Whether the button is enabled: some selected leaf draws a stroke (criterion 1).
#[must_use]
pub fn stroke_to_path_available(
    objects: &[ObjectSnapshot],
    selected: &[NodeId],
    skipped: &dyn Fn(NodeId) -> bool,
) -> bool {
    candidates(objects, selected, skipped)
        .iter()
        .any(|object| draws_stroke(object.style()))
}

/// The outlines of a leaf as the canvas strokes them, and whether its markers are drawn.
fn stroked_outlines(object: &ObjectSnapshot) -> (Vec<(Vec<OutlineTriple>, bool)>, bool) {
    let triples = |path: &PathSnapshot| -> Vec<(Vec<OutlineTriple>, bool)> {
        path.subpaths()
            .map(|subpath| {
                let anchors = subpath
                    .anchors
                    .iter()
                    .map(|a| (a.point, a.handle_in, a.handle_out))
                    .collect();
                (anchors, subpath.closed)
            })
            .collect()
    };
    match object {
        ObjectSnapshot::Primitive(primitive) => {
            let outline = drawing_outline_of_rotated(&primitive.shape, primitive.rotation);
            let anchors = outline
                .iter()
                .map(|a| (a.point, a.handle_in, a.handle_out))
                .collect();
            (vec![(anchors, outline.closed)], false)
        }
        ObjectSnapshot::Path(_) | ObjectSnapshot::Clone(_) => {
            object.drawn_path().map_or((Vec::new(), false), |path| {
                let outlines = triples(path);
                // A compound path draws no markers (`0016` criterion 38b).
                let markers = outlines.len() == 1;
                (outlines, markers)
            })
        }
        ObjectSnapshot::Sketch(_)
        | ObjectSnapshot::Text(_)
        | ObjectSnapshot::Image(_)
        | ObjectSnapshot::Dimension(_) => (Vec::new(), false),
    }
}

/// The kernel's stroke outline of `outlines` (point, handle in, handle out; closed) in `style`,
/// markers drawn when `markers`: what Stroke to path makes of them, for the SVG export's
/// Strokes as paths (`specs/0024-svg-import-export` criterion 146). `None` when the stroke paints
/// nothing or is out of range.
pub(crate) fn stroke_outline_anchors(
    outlines: &[(Vec<OutlineTriple>, bool)],
    style: &Style,
    markers: bool,
) -> Option<Vec<Vec<OffsetAnchor>>> {
    let stroke = &style.stroke;
    let shape = StrokeShape {
        width: stroke.width,
        join: stroke.join,
        cap: stroke.cap,
        dash: stroke.dash.clone(),
        markers: if markers {
            stroke.markers
        } else {
            curvyo_document_core::Markers::default()
        },
    };
    let borrowed: Vec<Outline<'_>> = outlines
        .iter()
        .map(|(anchors, closed)| Outline::new(anchors, *closed))
        .collect();
    let result = stroke_outline(&borrowed, &shape, BOOLEAN_TOLERANCE).ok()?;
    Some(result.outlines().to_vec())
}

/// The outlines of `object` as the canvas strokes them and whether its markers are drawn.
pub(crate) fn stroked_outlines_of(
    object: &ObjectSnapshot,
) -> (Vec<(Vec<OutlineTriple>, bool)>, bool) {
    stroked_outlines(object)
}

/// The kernel's stroke outline of `outlines` in `style`, with fresh anchor ids.
fn outline_of(
    outlines: &[(Vec<OutlineTriple>, bool)],
    style: &Style,
    markers: bool,
    minter: &mut AnchorIdMinter,
) -> Result<PieceOutlines, StrokeOutlineError> {
    let stroke = &style.stroke;
    let shape = StrokeShape {
        width: stroke.width,
        join: stroke.join,
        cap: stroke.cap,
        dash: stroke.dash.clone(),
        markers: if markers {
            stroke.markers
        } else {
            curvyo_document_core::Markers::default()
        },
    };
    let borrowed: Vec<Outline<'_>> = outlines
        .iter()
        .map(|(anchors, closed)| Outline::new(anchors, *closed))
        .collect();
    let result = stroke_outline(&borrowed, &shape, BOOLEAN_TOLERANCE)?;
    Ok(result
        .outlines()
        .iter()
        .map(|outline| (fresh(outline, minter), true))
        .collect())
}

/// One outline as new anchors; a smooth node is asymmetric (tangent, own lengths), as Offset's.
fn fresh(outline: &[OffsetAnchor], minter: &mut AnchorIdMinter) -> Vec<NewAnchor> {
    outline
        .iter()
        .map(|anchor| NewAnchor {
            id: minter.mint(),
            point: anchor.point,
            handle_in: anchor.handle_in,
            handle_out: anchor.handle_out,
            kind: if anchor.smooth {
                AnchorKind::Asymmetric
            } else {
                AnchorKind::Corner
            },
        })
        .collect()
}

/// What planning collects while it walks the subjects.
#[derive(Default)]
struct Walk {
    plan: StrokeToPathPlan,
    offenders: Vec<NodeId>,
    /// Objects (not glyphs) whose stroke is converted: their clones are unlinked.
    converted: HashSet<NodeId>,
}

impl Walk {
    /// Adds the outline of one subject, or counts it as without stroke (criterion 27), or as an
    /// offender (criterion 26). Returns whether an outline was added.
    fn add(
        &mut self,
        subject: StrokeSubject,
        owner: NodeId,
        result: Result<PieceOutlines, StrokeOutlineError>,
    ) -> bool {
        match result {
            Ok(outline) => {
                self.plan
                    .write
                    .outlines
                    .push(StrokeOutlineWrite { subject, outline });
                true
            }
            Err(StrokeOutlineError::Empty) => false,
            Err(_) => {
                if !self.offenders.contains(&owner) {
                    self.offenders.push(owner);
                }
                false
            }
        }
    }
}

/// Plans Stroke to path for the `selected` leaves of `objects` (every leaf of the document in
/// stacking order, so the clones of a subject are found wherever they are). `skipped` names the
/// leaves that are never subjects and are not counted: hidden, locked, guides. `names` gives a
/// text's stored name for its group.
///
/// # Errors
/// [`StrokeToPathRefusal`]: texts first (criterion 25), then out of range (criterion 26).
pub fn plan_stroke_to_path(
    objects: &[ObjectSnapshot],
    selected: &[NodeId],
    skipped: &dyn Fn(NodeId) -> bool,
    names: &dyn Fn(NodeId) -> Option<String>,
    minter: &mut AnchorIdMinter,
) -> Result<StrokeToPathPlan, StrokeToPathRefusal> {
    let leaves = candidates(objects, selected, skipped);
    let text_ids: Vec<NodeId> = leaves
        .iter()
        .filter(|object| {
            matches!(object, ObjectSnapshot::Sketch(_) | ObjectSnapshot::Text(_))
                && draws_stroke(object.style())
        })
        .map(|object| object.id())
        .collect();
    let conversions = if text_ids.is_empty() {
        Vec::new()
    } else {
        build_text_conversions(objects, &text_ids, names, minter)
            .map_err(StrokeToPathRefusal::Texts)?
    };
    let mut walk = Walk::default();
    for object in &leaves {
        if !draws_stroke(object.style()) {
            walk.plan.unstroked += 1;
            continue;
        }
        if let ObjectSnapshot::Text(text) = object {
            let conversion = conversions.iter().find(|c| c.id == text.id);
            walk_text(&mut walk, conversion, &text.style, minter);
            continue;
        }
        let (outlines, markers) = stroked_outlines(object);
        let result = outline_of(&outlines, object.style(), markers, minter);
        if walk.add(StrokeSubject::Object(object.id()), object.id(), result) {
            walk.plan.strokes += 1;
            walk.converted.insert(object.id());
        } else {
            walk.plan.unstroked += 1;
        }
    }
    if !walk.offenders.is_empty() {
        return Err(StrokeToPathRefusal::OutOfRange {
            offenders: walk.offenders,
        });
    }
    let mut plan = walk.plan;
    if plan.write.outlines.is_empty() {
        return Ok(plan);
    }
    plan.write.unlinks = unlinks(objects, &walk.converted, minter);
    plan.unlinked = plan.write.unlinks.len();
    plan.write.texts = conversions;
    plan.texts = plan.write.texts.len();
    Ok(plan)
}

/// The glyph outlines of one text (criterion 15); the text counts as one stroke.
fn walk_text(
    walk: &mut Walk,
    conversion: Option<&TextConversion>,
    style: &Style,
    minter: &mut AnchorIdMinter,
) {
    let Some(conversion) = conversion else {
        walk.plan.unstroked += 1;
        return;
    };
    let mut any = false;
    for (glyph, outlines) in conversion.glyphs.iter().enumerate() {
        let triples: Vec<(Vec<OutlineTriple>, bool)> = outlines
            .iter()
            .map(|(anchors, closed)| {
                let anchors = anchors
                    .iter()
                    .map(|a| (a.point, a.handle_in, a.handle_out))
                    .collect();
                (anchors, *closed)
            })
            .collect();
        let result = outline_of(&triples, style, false, minter);
        let subject = StrokeSubject::Glyph {
            text: conversion.id,
            glyph,
        };
        any |= walk.add(subject, conversion.id, result);
    }
    if any {
        walk.plan.strokes += 1;
    } else {
        walk.plan.unstroked += 1;
    }
}

/// The clones to unlink before the write (criterion 17): every converted clone and every clone
/// that follows a converted object, with fresh anchor ids for the outline it shows.
fn unlinks(
    objects: &[ObjectSnapshot],
    converted: &HashSet<NodeId>,
    minter: &mut AnchorIdMinter,
) -> Vec<CopySource> {
    objects
        .iter()
        .filter_map(|object| match object {
            ObjectSnapshot::Clone(clone)
                if converted.contains(&clone.id()) || converted.contains(&clone.chain.origin) =>
            {
                Some(CopySource {
                    id: clone.id(),
                    anchor_ids: clone
                        .resolved
                        .all_anchors()
                        .map(|_| minter.mint())
                        .collect(),
                })
            }
            _ => None,
        })
        .collect()
}
