//! Derives a filled stroke outline from a path's relative width profile.

use curvyo_document_core::{
    AnchorId, AnchorKind, CurveKind, FillPaint, NewAnchor, ObjectSnapshot, PathSnapshot,
    SubpathSnapshot, Tolerance,
};
use curvyo_geometry_core::{
    Outline, StrokeShape, VariableStrokeError, WidthPoint, WidthProfile, variable_stroke_outline,
};

/// Supplies already-computed variable artwork for the exact source snapshot.
pub type VariableArtworkLookup<'a> =
    dyn Fn(&PathSnapshot) -> Option<std::sync::Arc<(PathSnapshot, Option<PathSnapshot>)>> + 'a;

/// Every path with width points that `object` draws: its own outline, or each profiled leaf of a
/// clone of a group, text or image (nested picture clones included, ADR 0045 §3).
#[must_use]
pub fn profiled_paths(object: &ObjectSnapshot) -> Vec<&PathSnapshot> {
    let mut found = Vec::new();
    collect_profiled(object, &mut found);
    found
}

fn collect_profiled<'a>(object: &'a ObjectSnapshot, found: &mut Vec<&'a PathSnapshot>) {
    if let Some(picture) = object.picture() {
        for leaf in picture {
            collect_profiled(leaf, found);
        }
    } else if let Some(path) = object
        .drawn_path()
        .filter(|path| !path.width_points.is_empty())
    {
        found.push(path);
    }
}

pub(crate) fn synchronous_artwork(
    path: &PathSnapshot,
) -> Option<std::sync::Arc<(PathSnapshot, Option<PathSnapshot>)>> {
    verified_variable_stroke_artwork(path)
        .ok()
        .map(std::sync::Arc::new)
}

/// The profiled artwork could not be derived safely.
#[derive(Debug, thiserror::Error)]
pub enum VariableArtworkError {
    /// The underlying editable curve could not be evaluated.
    #[error(transparent)]
    Curve(#[from] crate::EvaluatePathError),
    /// The width profile or geometry is unsupported.
    #[error(transparent)]
    Stroke(#[from] VariableStrokeError),
}

/// Produces the original fill and a separate filled stroke outline without modifying the source.
///
/// # Errors
/// Returns an error when the curve or its variable stroke cannot be represented accurately.
pub fn variable_stroke_artwork(
    source: &PathSnapshot,
) -> Result<(PathSnapshot, Option<PathSnapshot>), VariableArtworkError> {
    variable_stroke_artwork_with_tolerance(source, Tolerance::from_mm(0.01))
}

/// Produces variable artwork with the requested source-space tolerance.
///
/// # Errors
/// Returns an error when the geometry cannot meet the requested tolerance.
pub fn variable_stroke_artwork_with_tolerance(
    source: &PathSnapshot,
    tolerance: Tolerance,
) -> Result<(PathSnapshot, Option<PathSnapshot>), VariableArtworkError> {
    let mut center = crate::evaluate_path(source)?;
    if center.is_compound() {
        return Err(VariableStrokeError::Unsupported.into());
    }
    let controls = center
        .width_points
        .iter()
        .map(|p| WidthPoint::new(p.id.as_u128(), p.position, p.multiplier, center.closed))
        .collect::<Option<Vec<_>>>()
        .ok_or(VariableStrokeError::Unsupported)?;
    let profile =
        WidthProfile::new(controls, center.closed).ok_or(VariableStrokeError::Unsupported)?;
    let triples = center
        .anchors
        .iter()
        .map(|a| (a.point, a.handle_in, a.handle_out))
        .collect::<Vec<_>>();
    let stroke = &center.style.stroke;
    let style = StrokeShape {
        width: stroke.width,
        cap: stroke.cap,
        join: stroke.join,
        dash: stroke.dash.clone(),
        markers: stroke.markers,
    };
    let outlines = match variable_stroke_outline(
        Outline::new(&triples, center.closed),
        &style,
        &profile,
        tolerance,
    ) {
        Ok(outlines) => outlines,
        Err(VariableStrokeError::Outline(curvyo_geometry_core::StrokeOutlineError::Empty)) => {
            center.style.stroke.enabled = false;
            center.width_points.clear();
            return Ok((center, None));
        }
        Err(error) => return Err(error.into()),
    };
    let mut next_id = 0_u64;
    let mut paths = outlines.outlines().iter().map(|ring| {
        let anchors = ring
            .iter()
            .map(|a| {
                let id = AnchorId::new(0, next_id);
                next_id = next_id.saturating_add(1);
                NewAnchor {
                    id,
                    point: a.point,
                    handle_in: a.handle_in,
                    handle_out: a.handle_out,
                    kind: if a.smooth {
                        AnchorKind::Asymmetric
                    } else {
                        AnchorKind::Corner
                    },
                }
            })
            .collect();
        SubpathSnapshot {
            closed: true,
            anchors,
        }
    });
    let first = paths.next().ok_or(VariableStrokeError::Unsupported)?;
    let mut outline = center.clone();
    outline.anchors = first.anchors;
    outline.closed = true;
    outline.extra_subpaths = paths.collect();
    outline.curve_kind = CurveKind::Bezier;
    outline.width_points.clear();
    outline.style.fill.enabled = true;
    outline.style.fill.paint = FillPaint::Colour;
    outline.style.fill.color = stroke.color;
    outline.style.fill.opacity = stroke.opacity;
    outline.style.fill.pattern = None;
    outline.style.stroke.enabled = false;
    center.style.stroke.enabled = false;
    center.width_points.clear();
    Ok((center, Some(outline)))
}

/// Produces artwork only when the geometry kernel accepts the path and profile.
///
/// # Errors
/// Returns an error when the kernel cannot safely construct the contour.
pub fn verified_variable_stroke_artwork(
    source: &PathSnapshot,
) -> Result<(PathSnapshot, Option<PathSnapshot>), VariableArtworkError> {
    if !source.style.stroke.enabled {
        let mut center = crate::evaluate_path(source)?;
        center.width_points.clear();
        return Ok((center, None));
    }
    variable_stroke_artwork(source)
}

/// Produces verified artwork with the requested source-space tolerance.
///
/// # Errors
/// Returns an error when the geometry cannot meet the requested tolerance.
pub fn verified_variable_stroke_artwork_with_tolerance(
    source: &PathSnapshot,
    tolerance: Tolerance,
) -> Result<(PathSnapshot, Option<PathSnapshot>), VariableArtworkError> {
    if !source.style.stroke.enabled {
        let mut center = crate::evaluate_path(source)?;
        center.width_points.clear();
        return Ok((center, None));
    }
    variable_stroke_artwork_with_tolerance(source, tolerance)
}
