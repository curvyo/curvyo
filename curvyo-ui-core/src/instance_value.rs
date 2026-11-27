//! The value arm of the generated-object mechanism (ADR 0080 §6): a parameter typed or stepped
//! into a row, planned (a live preview) and written (one commit).

use curvyo_document_core::{
    CHANGE_PARAMETER_COMMIT_LABEL, Document, GeneratedRecord, GeneratedSnapshot, KEY_SUBJECT_KIND,
    KEY_SUBJECT_PARAM, NodeId, ObjectSnapshot,
};
use curvyo_extension_core::{Edit, ParamValue};

use crate::extensions::ExtensionSettings;
use crate::instance::{InstanceError, find_kind, generate, record_keeping};
use crate::instance_notice::notices;

/// The object an instance command acts on: the clone of a sketch with its own values (`0099`), or
/// a generated object (`0141`). The two arms share the field, the clamp notice and the preview;
/// each has its own storage (`adrs.md` A7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceSource {
    /// A linked clone of a sketch.
    Sketch {
        /// The clone.
        clone: NodeId,
    },
    /// A generated object.
    Generated {
        /// The object.
        node: NodeId,
    },
}

/// Sets the parameter `key` of `source` to `value` in one commit "Change parameter": for a
/// generated object the key of its schema, for a sketch clone the exposed key (`p:...` or `d:...`)
/// with a length in millimetres or an angle in degrees.
///
/// # Errors
/// As [`change_value`] for a generated object; for a sketch clone
/// [`InstanceError::NoSuchObject`], [`InstanceError::WrongValue`] or [`InstanceError::Refused`].
pub fn change_parameter(
    document: &Document,
    settings: &ExtensionSettings,
    source: InstanceSource,
    key: &str,
    value: &ParamValue,
) -> Result<Changed, InstanceError> {
    match source {
        InstanceSource::Generated { node } => change_value(document, settings, node, key, value),
        InstanceSource::Sketch { clone } => {
            let number = match value {
                ParamValue::Length(length) => length.as_mm(),
                ParamValue::Angle(degrees) => degrees.0,
                _ => return Err(InstanceError::WrongValue),
            };
            crate::instance_sketch::change(document, &[clone], key, number, false, &[])
        }
    }
}

/// What a parameter edit changed.
#[derive(Debug, Clone, PartialEq)]
pub struct Changed {
    /// The notice sentences of the values the generator clamped, the edited field's first.
    pub notices: Vec<String>,
    /// Whether a commit was written (a value equal to the stored one writes nothing).
    pub written: bool,
    /// Whether the generator answered no parts for the new values: the stored outline is the
    /// last valid one (`0144` criterion 13).
    pub refused: bool,
}

/// What setting one parameter would write: the object as stored, the rebuilt record and the
/// notices of the values the generator clamped.
#[derive(Debug, Clone, PartialEq)]
pub struct ValuePlan {
    /// The object as it is stored now.
    pub snapshot: GeneratedSnapshot,
    /// The record the generator rebuilt (the stored one when nothing changes).
    pub record: GeneratedRecord,
    /// The notice sentences of the clamps, the edited field's first.
    pub notices: Vec<String>,
    /// The words the generator gave to values at a limit that is a shape ("full round"), by
    /// parameter.
    pub states: Vec<(&'static str, &'static str)>,
    /// Whether the generator answered no parts: the record keeps the stored outline.
    pub refused: bool,
}

/// What setting the parameter `key` of the generated object `id` to `value` would give, without
/// writing: the generator clamps and derives, the outline is rebuilt. A live preview and
/// [`change_value`] share it.
///
/// # Errors
/// [`InstanceError::NoSuchObject`], [`InstanceError::UnknownKind`] or [`InstanceError::WrongValue`].
pub fn plan_value(
    document: &Document,
    settings: &ExtensionSettings,
    id: NodeId,
    key: &str,
    value: &ParamValue,
) -> Result<ValuePlan, InstanceError> {
    let snapshot = document.generated(id).ok_or(InstanceError::NoSuchObject)?;
    let found = find_kind(settings, &snapshot.record.kind).ok_or(InstanceError::UnknownKind)?;
    let schema = found.kind.schema();
    let param = schema
        .iter()
        .find(|param| param.key == key)
        .ok_or(InstanceError::WrongValue)?;
    if std::mem::discriminant(&param.default) != std::mem::discriminant(value) {
        return Err(InstanceError::WrongValue);
    }
    let mut params = snapshot.record.params.clone();
    params.insert(key.to_owned(), value.to_stored());
    // The Finder colour starts as the fill's colour (UX notes of `0144` §3): switching it on while
    // its colour is still the schema's black writes the fill's colour in the same step.
    if key == "finder_colour_on"
        && matches!(value, ParamValue::Switch(true))
        && params.get("finder_colour").is_none_or(|stored| {
            ParamValue::from_stored(stored)
                == Some(ParamValue::Colour(curvyo_document_core::Color::BLACK))
        })
        && let Some(ObjectSnapshot::Path(path)) = document.object(id)
    {
        let fill = &path.style.fill;
        let colour = fill
            .painted_gradient()
            .and_then(|gradient| gradient.stops.as_slice().first().map(|stop| stop.colour))
            .unwrap_or(fill.color);
        params.insert(
            "finder_colour".to_owned(),
            ParamValue::Colour(colour).to_stored(),
        );
    }
    let generated = generate(
        &found,
        &params,
        snapshot.record.seed,
        Some(Edit::Param(key)),
    )
    .ok_or(InstanceError::UnknownKind)?;
    let record = record_keeping(&snapshot.record, &generated);
    let mut notices = notices(&found.kind, &generated.clamps);
    // The reason content cannot be drawn stays under its field (`InstanceView`), not a notice.
    let drew_nothing = generated.parts.is_empty();
    let reported: Vec<_> = generated
        .warnings
        .iter()
        .filter(|warning| !crate::instance_warnings::is_row_problem(warning, drew_nothing))
        .cloned()
        .collect();
    notices.extend(crate::instance_warnings::sentences_in(
        &reported,
        crate::instance_warnings::Context::of(&generated, found.kind.label),
    ));
    Ok(ValuePlan {
        snapshot,
        record,
        notices,
        states: generated.states,
        refused: generated.parts.is_empty(),
    })
}

/// Sets the parameter `key` of the generated object `id` to `value`: the generator clamps and
/// derives, the outline is rebuilt and one commit writes record and frame.
///
/// # Errors
/// [`InstanceError::NoSuchObject`], [`InstanceError::UnknownKind`], [`InstanceError::WrongValue`]
/// or [`InstanceError::Refused`].
pub fn change_value(
    document: &Document,
    settings: &ExtensionSettings,
    id: NodeId,
    key: &str,
    value: &ParamValue,
) -> Result<Changed, InstanceError> {
    let ValuePlan {
        snapshot,
        record,
        notices,
        refused,
        ..
    } = plan_value(document, settings, id, key, value)?;
    if record == snapshot.record {
        return Ok(Changed {
            notices,
            written: false,
            refused,
        });
    }
    document
        .change_generated(
            id,
            &record,
            snapshot.frame,
            CHANGE_PARAMETER_COMMIT_LABEL,
            &[
                (KEY_SUBJECT_KIND, snapshot.record.kind.clone()),
                (KEY_SUBJECT_PARAM, key.to_owned()),
            ],
        )
        .map_err(|_| InstanceError::Refused)?;
    Ok(Changed {
        notices,
        written: true,
        refused,
    })
}
