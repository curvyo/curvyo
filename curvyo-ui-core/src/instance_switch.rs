//! Switching a code from one symbology to another in place (ADR 0092 §2, `0144` criterion 20): the
//! same object, the same frame, the parameters of equal meaning carried over.

use curvyo_document_core::{
    CHANGE_PARAMETER_COMMIT_LABEL, Document, GeneratedRecord, KEY_SUBJECT_KIND, NodeId,
};
use curvyo_extension_core::{Param, ParamValue, StoredParams};

use crate::extensions::ExtensionSettings;
use crate::instance::{InstanceError, KindRef, find_kind, generate, record_of, stored_kind};

/// The key that is derived, so it never crosses (the generation recomputes it).
const DERIVED: &str = "width";

/// Whether `value` is what a new object of the kind has for `param` (untouched).
fn is_untouched(param: &Param, value: &ParamValue) -> bool {
    *value == param.default || param.initial.as_ref() == Some(value)
}

/// The stored values of `target` after a switch from `from` with the values `old`: a value of an
/// equal key is kept, clamped by the target's schema, unless it is still the old kind's default,
/// then it takes the target's; keys the target lacks are dropped (Undo restores them).
fn carried(from: &KindRef, target: &KindRef, old: &StoredParams) -> StoredParams {
    let old_schema = from.kind.schema();
    let mut next = Param::defaults(&target.kind.schema());
    for param in target.kind.params().filter(|p| p.key != DERIVED) {
        let Some(before) = old_schema.iter().find(|p| p.key == param.key) else {
            continue;
        };
        if std::mem::discriminant(&before.kind) != std::mem::discriminant(&param.kind) {
            continue;
        }
        let value = before.read(old);
        if is_untouched(before, &value) {
            continue;
        }
        if let Some(kept) = param.accept(value) {
            next.insert(param.key.to_owned(), kept.to_stored());
        }
    }
    next
}

/// What a switch wrote.
#[derive(Debug, Clone, PartialEq)]
pub struct Switched {
    /// The kind the object has now (`<extension>/<kind>`).
    pub kind: String,
    /// Whether the new kind cannot encode the carried content: the old outline stays, in the No
    /// solution look, under the new kind.
    pub refused: bool,
}

/// Switches the generated object `id` to the kind `target` of the same family in one commit, the
/// frame and every other register untouched.
///
/// # Errors
/// [`InstanceError::NoSuchObject`], [`InstanceError::UnknownKind`], [`InstanceError::WrongValue`]
/// for a target of another family, or [`InstanceError::Refused`] when the document refuses.
pub fn switch_kind(
    document: &Document,
    settings: &ExtensionSettings,
    id: NodeId,
    target: &str,
) -> Result<Switched, InstanceError> {
    let snapshot = document.generated(id).ok_or(InstanceError::NoSuchObject)?;
    let from = find_kind(settings, &snapshot.record.kind).ok_or(InstanceError::UnknownKind)?;
    let to = find_kind(settings, target).ok_or(InstanceError::UnknownKind)?;
    match (from.kind.family, to.kind.family) {
        (Some(a), Some(b)) if a == b => {}
        _ => return Err(InstanceError::WrongValue),
    }
    let kind = stored_kind(to.manifest.id, to.kind.id);
    if kind == snapshot.record.kind {
        return Ok(Switched {
            kind,
            refused: false,
        });
    }
    let params = carried(&from, &to, &snapshot.record.params);
    let generated =
        generate(&to, &params, snapshot.record.seed, None).ok_or(InstanceError::UnknownKind)?;
    let refused = generated.parts.is_empty();
    // A plain generation: the stored truth wins. Content the target cannot encode keeps the stored
    // outline with the new parameters (ADR 0091 §3).
    let record = if refused {
        GeneratedRecord {
            kind: kind.clone(),
            seed: snapshot.record.seed,
            params: generated.params,
            parts: snapshot.record.parts.clone(),
            part_keys: snapshot.record.part_keys.clone(),
        }
    } else {
        record_of(&kind, snapshot.record.seed, &generated)
    };
    document
        .change_generated(
            id,
            &record,
            snapshot.frame,
            CHANGE_PARAMETER_COMMIT_LABEL,
            &[(KEY_SUBJECT_KIND, kind.clone())],
        )
        .map_err(|_| InstanceError::Refused)?;
    Ok(Switched { kind, refused })
}
