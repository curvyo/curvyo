//! The sketch-clone arm of the shared parametric-instance mechanism (`0099` criteria 24, 26 to 28,
//! 33, 34, ADR 0080 §6, `adrs.md` A7, A14): a value typed or dragged, coupled through the sketch's
//! link groups or changed alone, Original's, Own, Back, and the plan a drag previews. Nothing here
//! knows a solver; document-core solves. The rows live in `instance_sketch_view.rs`.

use curvyo_document_core::{
    CloneSnapshot, Document, ExposedKey, ExposedParam, Held, InstanceKind, InstanceWrite, KeySet,
    LinkedChange, NodeId, ObjectSnapshot, ValueSet, linked_change, linked_reset,
};

use crate::instance::{Changed, InstanceError, shown};
use crate::instance_notice::say_number;
pub use crate::instance_sketch_view::{SketchInstanceView, SketchRowView, group_letter, view};

fn clone_of(document: &Document, id: NodeId) -> Option<CloneSnapshot> {
    match document.read_objects().get(id)? {
        ObjectSnapshot::Clone(clone) => Some(clone.clone()),
        _ => None,
    }
}

/// What a value would write for one clone, without writing: the notices, the clone as it would
/// read (`None` when the values have no solution), the whole set of own values and the marks.
#[derive(Debug, Clone)]
pub struct SketchPlan {
    /// The clone with the value in place; `None` for a value set without solution.
    pub preview: Option<CloneSnapshot>,
    /// The clone's whole new set of own values.
    pub own: ValueSet,
    /// The keys it would hold unlinked.
    pub unlinked: KeySet,
    /// The notice sentences of a value held at a limit.
    pub notices: Vec<String>,
}

/// The notice of a value held at a limit: "Width set to 120 mm, the maximum.", or naming the
/// partner whose limit held: "Radius 1 set to 30 mm, the maximum of Radius 2." (UX notes 12.9.3).
fn notice(param: &ExposedParam, held: &Held) -> String {
    let bound = if held.minimum { "minimum" } else { "maximum" };
    let said = say_number(param.kind == InstanceKind::Angle, held.number);
    if held.by == param.label {
        format!("{} set to {said}, the {bound}.", param.label)
    } else {
        format!("{} set to {said}, the {bound} of {}.", param.label, held.by)
    }
}

/// The change of `key` to `number` on the committed `read`, through the link groups of
/// `exposed`, and its notice.
fn coupled(
    exposed: &[ExposedParam],
    read: &CloneSnapshot,
    key: &str,
    number: f64,
    only_this: bool,
) -> Result<(LinkedChange, Option<String>), InstanceError> {
    let key = ExposedKey::parse(key).ok_or(InstanceError::WrongValue)?;
    let param = exposed
        .iter()
        .find(|param| param.key == key)
        .ok_or(InstanceError::WrongValue)?;
    if !number.is_finite() {
        return Err(InstanceError::WrongValue);
    }
    let change = linked_change(
        exposed,
        &read.chain.own_values,
        &read.chain.inherited_values,
        &read.chain.unlinked,
        &key,
        number,
        only_this,
    );
    let said = change.held.as_ref().map(|held| notice(param, held));
    Ok((change, said))
}

/// Works out setting the exposed value `key` of `clone` to `number` (millimetres or degrees)
/// without writing: coupled to its link group unless `only_this` (Shift at the gesture's start),
/// clamped into the limits, the clone read with it (one solve for the whole group).
///
/// # Errors
/// [`InstanceError::NoSuchObject`] for a clone that follows no sketch, [`InstanceError::WrongValue`]
/// for a key that is not exposed or a number that is not finite.
pub fn plan(
    document: &Document,
    clone: NodeId,
    key: &str,
    number: f64,
    only_this: bool,
) -> Result<SketchPlan, InstanceError> {
    let read = clone_of(document, clone).ok_or(InstanceError::NoSuchObject)?;
    let exposed = document.exposed_params(read.chain.root);
    let (change, said) = coupled(&exposed, &read, key, number, only_this)?;
    Ok(SketchPlan {
        preview: document.instance_preview(clone, &change.own),
        own: change.own,
        unlinked: change.unlinked,
        notices: said.into_iter().collect(),
    })
}

/// Sets the exposed value `key` of every clone of `ids` to `number` (millimetres or degrees), each
/// coupled inside itself unless `only_this`, in one commit "Change parameter"; a typed value is
/// clamped with a notice. `last_solvable` gives, per clone, the last value set a drag step solved,
/// kept as the saved fallback when this one has no solution.
///
/// # Errors
/// [`InstanceError::NoSuchObject`], [`InstanceError::WrongValue`] or [`InstanceError::Refused`].
pub fn change(
    document: &Document,
    ids: &[NodeId],
    key: &str,
    number: f64,
    only_this: bool,
    last_solvable: &[(NodeId, ValueSet)],
) -> Result<Changed, InstanceError> {
    let mut writes = Vec::new();
    let mut notices = Vec::new();
    for &id in ids {
        let read = clone_of(document, id).ok_or(InstanceError::NoSuchObject)?;
        let exposed = document.exposed_params(read.chain.root);
        let (change, said) = coupled(&exposed, &read, key, number, only_this)?;
        if notices.is_empty() {
            notices.extend(said);
        }
        if change.own != read.chain.own_values || change.unlinked != read.chain.unlinked {
            writes.push(InstanceWrite {
                clone: id,
                own: change.own,
                last_solvable: last_solvable
                    .iter()
                    .find(|(clone, _)| *clone == id)
                    .map(|(_, set)| set.clone()),
                unlinked: change.unlinked,
            });
        }
    }
    if writes.is_empty() {
        return Ok(Changed {
            notices,
            written: false,
        });
    }
    let subject = ExposedKey::parse(key);
    document
        .write_instance_values(
            &writes,
            curvyo_document_core::CHANGE_PARAMETER_COMMIT_LABEL,
            subject.as_ref(),
        )
        .map_err(|_| InstanceError::Refused)?;
    Ok(Changed {
        notices,
        written: true,
    })
}

/// Drops the own value `key` of every clone of `ids` and its unlinked mark ("Back to the sketch's
/// value", which links it again; the partners keep theirs), or with `None` every own value and
/// mark of each ("Original's": "Back to the sketch's values").
///
/// # Errors
/// [`InstanceError::NoSuchObject`] or [`InstanceError::Refused`].
pub fn reset(
    document: &Document,
    ids: &[NodeId],
    key: Option<&str>,
) -> Result<bool, InstanceError> {
    let parsed = key.and_then(ExposedKey::parse);
    let mut writes = Vec::new();
    for &id in ids {
        let read = clone_of(document, id).ok_or(InstanceError::NoSuchObject)?;
        let (own, unlinked) = match (&parsed, key) {
            (Some(key), _) => linked_reset(&read.chain.own_values, &read.chain.unlinked, Some(key)),
            (None, Some(_)) => (read.chain.own_values.clone(), read.chain.unlinked.clone()),
            (None, None) => linked_reset(&read.chain.own_values, &read.chain.unlinked, None),
        };
        if own != read.chain.own_values || unlinked != read.chain.unlinked {
            writes.push(InstanceWrite {
                clone: id,
                own,
                last_solvable: None,
                unlinked,
            });
        }
    }
    if writes.is_empty() {
        return Ok(false);
    }
    let label = if key.is_some() {
        curvyo_document_core::RESET_PARAMETER_COMMIT_LABEL
    } else {
        curvyo_document_core::RESET_PARAMETERS_COMMIT_LABEL
    };
    document
        .write_instance_values(&writes, label, parsed.as_ref())
        .map_err(|_| InstanceError::Refused)?;
    Ok(true)
}

/// Writes every value each clone of `ids` shows as its own ("Own parameter values"), so nothing
/// moves; the unlinked marks stay.
///
/// # Errors
/// [`InstanceError::NoSuchObject`] or [`InstanceError::Refused`].
pub fn own_all(document: &Document, ids: &[NodeId]) -> Result<bool, InstanceError> {
    let mut writes = Vec::new();
    for &id in ids {
        let read = clone_of(document, id).ok_or(InstanceError::NoSuchObject)?;
        let mut own = read.chain.own_values.clone();
        for param in document.exposed_params(read.chain.root) {
            if own.get(&param.key).is_none() {
                let shown_value = read
                    .chain
                    .inherited_values
                    .get(&param.key)
                    .unwrap_or_else(|| param.default_value());
                own.insert(param.key.clone(), shown_value);
            }
        }
        if own != read.chain.own_values {
            writes.push(InstanceWrite {
                clone: id,
                own,
                last_solvable: None,
                unlinked: read.chain.unlinked.clone(),
            });
        }
    }
    if writes.is_empty() {
        return Ok(false);
    }
    document
        .write_instance_values(
            &writes,
            curvyo_document_core::OWN_PARAMETERS_COMMIT_LABEL,
            None,
        )
        .map_err(|_| InstanceError::Refused)?;
    Ok(true)
}

/// `number` as the Parameters field shows it, for tooltips.
#[must_use]
pub fn shown_number(angle: bool, number: f64) -> String {
    shown(number, if angle { 1 } else { 2 })
}
