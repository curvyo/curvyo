//! The box a generator builds from one selected closed outline (ADR 0083 §2, spec `0149`).

use curvyo_document_core::{Affine, Document, NodeId, ObjectSnapshot, Point};
use curvyo_extension_core::{ClosedPath, Cubic, Generated, Param, ParamValue};

use crate::boolean::{is_open, outlines_of};
use crate::extensions::ExtensionSettings;
use crate::instance::{find_kind, generate, record_of, stored_kind};
use crate::object_bounds::try_object_outline_bounds;

/// Why a box was not built from the selection; nothing was changed in any case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SourceRefusal {
    /// Nothing, several objects, a group, an open path or any other object.
    #[error("Select one closed outline. Nothing was changed.")]
    NotOneOutline,
    /// The outline crosses itself.
    #[error("The outline crosses itself. Nothing was changed.")]
    Crosses,
    /// The area is below `(4 t)^2` for the default wall thickness.
    #[error("The outline is too small for a box with 3 mm walls. Nothing was changed.")]
    TooSmall,
    /// Slice 1 builds straight outlines only.
    #[error(
        "This outline has curves. Box from path builds straight outlines only so far. Nothing was changed."
    )]
    Curved,
    /// The kind is not available or has no source.
    #[error("Select one closed outline. Nothing was changed.")]
    UnknownKind,
    /// The document refused the write.
    #[error("The document refused the change.")]
    Refused,
}

/// The one closed outline of `object`, in page mm, or `None` when it is anything else.
#[must_use]
pub fn outline_of(object: &ObjectSnapshot) -> Option<ClosedPath> {
    if is_open(object) {
        return None;
    }
    let mut outlines = outlines_of(object);
    let (anchors, closed) = outlines.pop().filter(|_| outlines.is_empty())?;
    if !closed || anchors.len() < 2 {
        return None;
    }
    let count = anchors.len();
    let segments = (0..count)
        .map(|i| {
            let (a, b) = (anchors[i], anchors[(i + 1) % count]);
            Cubic {
                c1: Point::new(a.0.x + a.2.x, a.0.y + a.2.y),
                c2: Point::new(b.0.x + b.1.x, b.0.y + b.1.y),
                to: b.0,
            }
        })
        .collect();
    Some(ClosedPath {
        start: anchors[0].0,
        segments,
    })
}

fn translated(path: &ClosedPath, dx: f64, dy: f64) -> ClosedPath {
    let by = |p: Point| Point::new(p.x + dx, p.y + dy);
    ClosedPath {
        start: by(path.start),
        segments: path
            .segments
            .iter()
            .map(|c| Cubic {
                c1: by(c.c1),
                c2: by(c.c2),
                to: by(c.to),
            })
            .collect(),
    }
}

/// The top-left of the box round every point and control point of the parts (the control hull
/// holds every curve, so the gap to the source holds).
fn layout_origin(generated: &Generated) -> (f64, f64) {
    let mut min = (f64::MAX, f64::MAX);
    for path in generated.parts.iter().flat_map(|part| &part.paths) {
        let points = std::iter::once(path.start)
            .chain(path.segments.iter().flat_map(|c| [c.c1, c.c2, c.to]));
        for p in points {
            min = (min.0.min(p.x), min.1.min(p.y));
        }
    }
    min
}

/// What building from `source` would write: the kind, the record, the frame and what the
/// generator returned for it (its marks and warnings).
struct Plan {
    kind: String,
    generated: Generated,
    frame: Affine,
}

fn plan(
    document: &Document,
    settings: &ExtensionSettings,
    kind: &str,
    source: NodeId,
) -> Result<Plan, SourceRefusal> {
    let found = find_kind(settings, kind).ok_or(SourceRefusal::UnknownKind)?;
    let target = found.kind.source.ok_or(SourceRefusal::UnknownKind)?;
    let object = document
        .object(source)
        .ok_or(SourceRefusal::NotOneOutline)?;
    let page = outline_of(&object).ok_or(SourceRefusal::NotOneOutline)?;
    let bounds = try_object_outline_bounds(&object).ok_or(SourceRefusal::NotOneOutline)?;
    // The copy lives round the origin (ADR 0083 §2), the frame carries it back.
    let centre = (
        f64::midpoint(bounds.0.x, bounds.1.x),
        f64::midpoint(bounds.0.y, bounds.1.y),
    );
    let mut params = Param::defaults(&found.kind.schema());
    params.insert(
        target.param.to_owned(),
        ParamValue::Path(translated(&page, -centre.0, -centre.1)).to_stored(),
    );
    let generated = generate(&found, &params, 0, None).ok_or(SourceRefusal::UnknownKind)?;
    refuse_unbuildable(&generated)?;
    let gap = generated
        .params
        .get("gap")
        .and_then(ParamValue::from_stored)
        .and_then(|value| value.length())
        .or_else(|| {
            found
                .kind
                .params()
                .find(|param| param.key == "gap")
                .and_then(|param| param.default.length())
        })
        .map_or(0.0, curvyo_document_core::Length::as_mm);
    let origin = layout_origin(&generated);
    // Gap right of the source's box, the tops level (ADR 0083 §2).
    let frame = Affine::new(
        1.0,
        0.0,
        0.0,
        1.0,
        bounds.1.x + gap - origin.0,
        bounds.0.y - origin.1,
    );
    Ok(Plan {
        kind: stored_kind(found.manifest.id, found.kind.id),
        generated,
        frame,
    })
}

/// Whether a box can be built from `source`, and the marks and warnings it would carry: what the
/// pick mode hovers and the button's press tests, without writing (ADR 0083 §11 option A).
///
/// # Errors
/// A [`SourceRefusal`] naming why not.
pub fn preview(
    document: &Document,
    settings: &ExtensionSettings,
    kind: &str,
    source: NodeId,
) -> Result<Generated, SourceRefusal> {
    plan(document, settings, kind, source).map(|plan| plan.generated)
}

/// Builds the box of the kind `kind` (`<extension>/<kind>`, one with a source) from `source` and
/// writes it directly above the source in one step; the source is not touched. Returns the new
/// object.
///
/// # Errors
/// A [`SourceRefusal`]: the notice is its `Display`; nothing was changed.
pub fn place_from_source(
    document: &Document,
    settings: &ExtensionSettings,
    kind: &str,
    source: NodeId,
) -> Result<NodeId, SourceRefusal> {
    let plan = plan(document, settings, kind, source)?;
    let record = record_of(&plan.kind, 0, &plan.generated);
    document
        .create_generated_above(&record, plan.frame, source)
        .map_err(|_| SourceRefusal::Refused)
}

/// What the generator says about the outline at the default values: a crossing or curved outline
/// or a thickness the size rule moved is refused (ADR 0083 §2).
fn refuse_unbuildable(generated: &Generated) -> Result<(), SourceRefusal> {
    let has = |id: &str| generated.warnings.iter().any(|w| w.id == id);
    if has("outline_crosses") {
        return Err(SourceRefusal::Crosses);
    }
    if has("outline_curved") {
        return Err(SourceRefusal::Curved);
    }
    if generated
        .clamps
        .iter()
        .any(|clamp| clamp.key == "thickness")
    {
        return Err(SourceRefusal::TooSmall);
    }
    Ok(())
}

/// The marks of the box that `source` would give, with the translation that carries them from the
/// outline's own coordinates to the page (the box centre of the source): what the pick mode draws
/// over the hovered outline.
///
/// # Errors
/// A [`SourceRefusal`] naming why not; nothing is drawn then.
pub fn hover_marks(
    document: &Document,
    settings: &ExtensionSettings,
    kind: &str,
    source: NodeId,
) -> Result<(curvyo_extension_core::Marks, Point), SourceRefusal> {
    let generated = preview(document, settings, kind, source)?;
    let object = document
        .object(source)
        .ok_or(SourceRefusal::NotOneOutline)?;
    let bounds = try_object_outline_bounds(&object).ok_or(SourceRefusal::NotOneOutline)?;
    let centre = Point::new(
        f64::midpoint(bounds.0.x, bounds.1.x),
        f64::midpoint(bounds.0.y, bounds.1.y),
    );
    Ok((generated.marks, centre))
}

/// Replaces the stored outline of the box `target` by a copy of `source` (ADR 0083 §2), in one
/// step; the source is not touched. The refusals are those of building a box.
///
/// # Errors
/// A [`SourceRefusal`]; nothing was changed then.
pub fn replace_outline(
    document: &Document,
    settings: &ExtensionSettings,
    target: NodeId,
    source: NodeId,
) -> Result<(), SourceRefusal> {
    let snapshot = document
        .generated(target)
        .ok_or(SourceRefusal::UnknownKind)?;
    let found = find_kind(settings, &snapshot.record.kind).ok_or(SourceRefusal::UnknownKind)?;
    let param = found.kind.source.ok_or(SourceRefusal::UnknownKind)?.param;
    if source == target {
        return Err(SourceRefusal::NotOneOutline);
    }
    let object = document
        .object(source)
        .ok_or(SourceRefusal::NotOneOutline)?;
    let page = outline_of(&object).ok_or(SourceRefusal::NotOneOutline)?;
    let bounds = try_object_outline_bounds(&object).ok_or(SourceRefusal::NotOneOutline)?;
    let centred = translated(
        &page,
        -f64::midpoint(bounds.0.x, bounds.1.x),
        -f64::midpoint(bounds.0.y, bounds.1.y),
    );
    let value = ParamValue::Path(centred);
    let mut params = snapshot.record.params.clone();
    params.insert(param.to_owned(), value.to_stored());
    let generated = generate(
        &found,
        &params,
        snapshot.record.seed,
        Some(curvyo_extension_core::Edit::Param(param)),
    )
    .ok_or(SourceRefusal::UnknownKind)?;
    refuse_unbuildable(&generated)?;
    crate::instance::change_value(document, settings, target, param, &value)
        .map(|_| ())
        .map_err(|_| SourceRefusal::Refused)
}
