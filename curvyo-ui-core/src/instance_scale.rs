//! The resize arm of the generated-object mechanism (ADR 0080 §6): the generator scales its size
//! parameters by the factor the outline grew by, and the frame follows the map.

use curvyo_document_core::{
    Affine, Document, GeneratedRecord, NodeId, ObjectSnapshot, orthonormalised,
};
use curvyo_document_core::{GeneratedSnapshot, Vec2};
use curvyo_extension_core::{Edit, Resize, ScaleFactor};

use crate::extensions::ExtensionSettings;
use crate::instance::{InstanceError, find_kind, generate, record_of};
use curvyo_document_core::KEY_SUBJECT_KIND;

/// A resize worked out but not yet written: the record and frame the generator gave.
#[derive(Debug)]
pub struct ResizePlan {
    id: NodeId,
    pub(crate) record: GeneratedRecord,
    pub(crate) frame: Affine,
    kind: String,
    /// The factor the outline grew by.
    pub(crate) factor: f64,
    /// What the generator clamped (the gear's teeth at 6 or 200).
    pub(crate) clamps: Vec<curvyo_extension_core::Clamp>,
}

/// Works out the resize of the generated object `result.id()` to the outline of `result` (a
/// proportional resize of the Select tool, ADR 0080 §6): the generator scales its size parameters
/// by the factor the outline grew by, and the frame follows the map. Writes nothing.
///
/// # Errors
/// As [`crate::instance::change_value`]; [`InstanceError::Refused`] also for a result that is no affine image of
/// the stored outline.
pub fn plan_resize(
    document: &Document,
    settings: &ExtensionSettings,
    result: &ObjectSnapshot,
) -> Result<ResizePlan, InstanceError> {
    let id = result.id();
    let snapshot = document.generated(id).ok_or(InstanceError::NoSuchObject)?;
    let found = find_kind(settings, &snapshot.record.kind).ok_or(InstanceError::UnknownKind)?;
    let map = document
        .generated_map(id, result)
        .ok_or(InstanceError::Refused)?;
    if found.kind.resize == Resize::Free {
        return plan_free(&snapshot, &found, map);
    }
    // Proportional only: the geometric mean of the two factors, whatever else the map holds.
    // Snapped to a billionth: the fitted map carries rounding noise the numbers must not.
    let factor = (map.determinant().abs().sqrt() * 1e9).round() / 1e9;
    if !factor.is_finite() || factor < 1e-6 {
        return Err(InstanceError::Refused);
    }
    let generated = generate(
        &found,
        &snapshot.record.params,
        snapshot.record.seed,
        Some(Edit::Scale(ScaleFactor(factor))),
    )
    .ok_or(InstanceError::UnknownKind)?;
    // A code whose content cannot be drawn has nothing to scale: the resize is refused.
    if generated.parts.is_empty() {
        return Err(InstanceError::Refused);
    }
    Ok(ResizePlan {
        id,
        record: record_of(&snapshot.record.kind, snapshot.record.seed, &generated),
        frame: orthonormalised(map.compose(snapshot.frame)),
        kind: snapshot.record.kind,
        factor,
        clamps: generated.clamps,
    })
}

/// Writes a [`ResizePlan`] as one commit of record and frame under `label`; called inside the
/// gesture's batch.
///
/// # Errors
/// [`InstanceError::Refused`] when the document refuses the write.
pub fn apply_resize(
    document: &Document,
    plan: &ResizePlan,
    label: &str,
) -> Result<(), InstanceError> {
    document
        .change_generated(
            plan.id,
            &plan.record,
            plan.frame,
            label,
            &[(KEY_SUBJECT_KIND, plan.kind.clone())],
        )
        .map_err(|_| InstanceError::Refused)
}

/// [`plan_resize`] then [`apply_resize`] for one object.
///
/// # Errors
/// As [`plan_resize`] and [`apply_resize`].
pub fn resize(
    document: &Document,
    settings: &ExtensionSettings,
    result: &ObjectSnapshot,
    label: &str,
) -> Result<(), InstanceError> {
    apply_resize(document, &plan_resize(document, settings, result)?, label)
}

/// Whether `result` resizes the generated object it belongs to (its outline is not the stored
/// one moved, turned or flipped): that is the generator's business, not a transform.
#[must_use]
pub fn is_resize(document: &Document, result: &ObjectSnapshot) -> bool {
    let Some(map) = document.generated_map(result.id(), result) else {
        return document.generated(result.id()).is_some();
    };
    let unit = |x: f64, y: f64| (x * x + y * y - 1.0).abs() <= 1e-6;
    !(unit(map.a, map.b) && unit(map.c, map.d) && (map.a * map.c + map.b * map.d).abs() <= 1e-6)
}

/// The box of stored parts in frame-local mm: `(x0, y0, x1, y1)`, from the on-curve points.
fn local_box(parts: &[Vec<Vec<f64>>]) -> Option<(f64, f64, f64, f64)> {
    let mut found: Option<(f64, f64, f64, f64)> = None;
    for numbers in parts.iter().flatten() {
        let points = std::iter::once((numbers.first()?, numbers.get(1)?)).chain(
            numbers
                .get(2..)
                .into_iter()
                .flat_map(|rest| rest.as_chunks::<6>().0.iter())
                .map(|chunk| (&chunk[4], &chunk[5])),
        );
        for (x, y) in points {
            found = Some(found.map_or((*x, *y, *x, *y), |(x0, y0, x1, y1)| {
                (x0.min(*x), y0.min(*y), x1.max(*x), y1.max(*y))
            }));
        }
    }
    found
}

/// The shift along one axis that keeps the point of the old box the resize left fixed in the same
/// relative place of the new box (`a` is the axis' factor, `e` its offset in frame-local mm).
fn keep_fixed(a: f64, e: f64, old: (f64, f64), new: (f64, f64)) -> f64 {
    if (a - 1.0).abs() < 1e-9 {
        return 0.0;
    }
    let fixed = e / (1.0 - a);
    let width = old.1 - old.0;
    let relative = if width > 1e-9 {
        ((fixed - old.0) / width).clamp(0.0, 1.0)
    } else {
        0.5
    };
    fixed - (new.0 + relative * (new.1 - new.0))
}

/// The resize of a kind that resizes freely (ADR 0092 §4): the factors along the frame's axes go
/// to the generator, and the frame is placed so the point the drag left fixed (the edge or corner
/// opposite the handle) stays where it was in the regenerated box.
fn plan_free(
    snapshot: &GeneratedSnapshot,
    found: &crate::instance::KindRef,
    map: Affine,
) -> Result<ResizePlan, InstanceError> {
    let frame = snapshot.frame;
    let inverse = frame.inverse().ok_or(InstanceError::Refused)?;
    // The map in frame-local axes: frame, then the map, then back.
    let local = inverse.compose(map.compose(frame));
    let skewed = local.b.abs() > 1e-6 * local.a.abs().max(1.0)
        || local.c.abs() > 1e-6 * local.d.abs().max(1.0);
    let snap = |n: f64| (n.abs() * 1e9).round() / 1e9;
    let (x, y) = (snap(local.a), snap(local.d));
    if skewed || !x.is_finite() || !y.is_finite() || x < 1e-6 || y < 1e-6 {
        return Err(InstanceError::Refused);
    }
    // One factor along both axes is the proportional resize (Ctrl, or a corner of a selection with
    // a proportional kind in it).
    let edit = if (x - y).abs() < 1e-9 {
        Edit::Scale(ScaleFactor(x))
    } else {
        Edit::Stretch {
            x: ScaleFactor(x),
            y: ScaleFactor(y),
        }
    };
    let generated = generate(
        found,
        &snapshot.record.params,
        snapshot.record.seed,
        Some(edit),
    )
    .ok_or(InstanceError::UnknownKind)?;
    if generated.parts.is_empty() {
        return Err(InstanceError::Refused);
    }
    let record = record_of(&snapshot.record.kind, snapshot.record.seed, &generated);
    let (old, new) = (local_box(&snapshot.record.parts), local_box(&record.parts));
    let shift = match (old, new) {
        (Some(old), Some(new)) => Vec2::new(
            keep_fixed(local.a, local.e, (old.0, old.2), (new.0, new.2)),
            keep_fixed(local.d, local.f, (old.1, old.3), (new.1, new.3)),
        ),
        _ => Vec2::ZERO,
    };
    Ok(ResizePlan {
        id: snapshot.id,
        record,
        frame: frame.compose(Affine::translation(shift)),
        kind: snapshot.record.kind.clone(),
        factor: (x * y).sqrt(),
        clamps: generated.clamps,
    })
}
