//! The generated-object arm of the shared parametric-instance mechanism (ADR 0080 §6): the schema
//! view, a parameter edit, a proportional resize and Convert to paths of a generated object. The
//! sketch instances of `0099` add their arm to the same functions.
//!
//! Nothing here writes more than one commit per call, and nothing is written for a kind the
//! registry does not know or the maker switched off: that object is shown from its stored outline
//! and refuses a change by number or a resize (ADR 0080 §5).

use curvyo_document_core::{
    Affine, CHANGE_PARAMETER_COMMIT_LABEL, Document, GeneratedRecord, GeneratedSnapshot,
    KEY_SUBJECT_KIND, KEY_SUBJECT_PARAM, NodeId, ObjectSnapshot, Point, orthonormalised,
};
use curvyo_extension_core::{
    Edit, GenerateRequest, Generated, GeneratorKind, Manifest, Param, ParamKind, ParamValue, Part,
    ScaleFactor, Seed, ShownWhen, StoredParams,
};
use serde::Serialize;
use serde_json::Value;

use crate::anchor_id_minter::AnchorIdMinter;
use crate::extensions::{ExtensionSettings, generate_object, manifests};
use crate::instance_notice::notices;
pub use crate::instance_notice::{clamp_notice, shown};
pub use crate::instance_resize::{ResizeView, resize_view};

/// Why an instance command did nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InstanceError {
    /// The object is gone or is not a generated object.
    #[error("no such generated object")]
    NoSuchObject,
    /// Its generator is not compiled in, or the maker switched its extension off: the object is
    /// kept and drawn as stored but cannot be changed by number or resized.
    #[error("the generator of this object is not available")]
    UnknownKind,
    /// The value is of the wrong kind for the parameter.
    #[error("the value does not fit the parameter")]
    WrongValue,
    /// The document refused the write.
    #[error("the document refused the change")]
    Refused,
}

/// A generator kind found in the registry with the extension that owns it.
#[derive(Debug, Clone)]
pub struct KindRef {
    /// The extension's manifest.
    pub manifest: Manifest,
    /// The kind.
    pub kind: GeneratorKind,
}

/// `<extension id>/<kind>` as stored.
pub(crate) fn stored_kind(extension: &str, kind: &str) -> String {
    format!("{extension}/{kind}")
}

/// The generator kind a stored `<extension id>/<kind>` names, `None` for an unknown kind or an
/// extension the maker switched off.
#[must_use]
pub fn find_kind(settings: &ExtensionSettings, stored: &str) -> Option<KindRef> {
    let (extension, kind) = stored.split_once('/')?;
    if !settings.is_enabled(extension) {
        return None;
    }
    manifests()
        .into_iter()
        .filter(|manifest| manifest.id == extension)
        .find_map(|manifest| {
            let found = manifest.generators().find(|k| k.id == kind)?.clone();
            Some(KindRef {
                manifest,
                kind: found,
            })
        })
}

/// Every generator kind of the enabled extensions, in registry order: what the Generators bar
/// offers.
#[must_use]
pub fn enabled_kinds(settings: &ExtensionSettings) -> Vec<KindRef> {
    manifests()
        .into_iter()
        .filter(|manifest| settings.is_enabled(manifest.id))
        .flat_map(|manifest| {
            manifest
                .generators()
                .cloned()
                .map(|kind| KindRef {
                    manifest: manifest.clone(),
                    kind,
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The stored record of what a generator returned.
pub(crate) fn record_of(kind: &str, seed: u32, generated: &Generated) -> GeneratedRecord {
    GeneratedRecord {
        kind: kind.to_owned(),
        seed,
        params: generated.params.clone(),
        parts: generated
            .parts
            .iter()
            .map(|part: &Part| {
                part.paths
                    .iter()
                    .map(curvyo_extension_core::ClosedPath::flat)
                    .collect()
            })
            .collect(),
    }
}

pub(crate) fn generate(
    found: &KindRef,
    params: &StoredParams,
    seed: u32,
    edit: Option<Edit<'_>>,
) -> Option<Generated> {
    generate_object(
        found.manifest.id,
        &GenerateRequest {
            kind: found.kind.id,
            params,
            seed: Seed(seed),
            edit,
        },
    )
}

/// The warning sentences of the generated object `snapshot`, empty for an unknown kind.
#[must_use]
pub fn warning_notices(settings: &ExtensionSettings, snapshot: &GeneratedSnapshot) -> Vec<String> {
    find_kind(settings, &snapshot.record.kind)
        .and_then(|found| generate(&found, &snapshot.record.params, snapshot.record.seed, None))
        .map(|g| crate::instance_warnings::sentences(&g.warnings))
        .unwrap_or_default()
}

/// The overlay marks of the generated object `snapshot` (ADR 0083 §11), `None` for an unknown kind.
#[must_use]
pub fn marks_of(
    settings: &ExtensionSettings,
    snapshot: &GeneratedSnapshot,
) -> Option<curvyo_extension_core::Marks> {
    let found = find_kind(settings, &snapshot.record.kind)?;
    generate(&found, &snapshot.record.params, snapshot.record.seed, None).map(|g| g.marks)
}

/// The record of a new object of `found` at its defaults as a ghost draws it: the outer
/// outlines only (`Edit::Draft`).
#[must_use]
pub fn new_ghost_record(found: &KindRef) -> Option<GeneratedRecord> {
    let defaults = Param::defaults(&found.kind.schema());
    let generated = generate(found, &defaults, 0, Some(Edit::Draft))?;
    Some(record_of(
        &stored_kind(found.manifest.id, found.kind.id),
        0,
        &generated,
    ))
}

/// The record of a new object of `found` at its defaults, and what the generator returned.
#[must_use]
pub fn new_record(found: &KindRef) -> Option<(GeneratedRecord, Generated)> {
    let defaults = Param::defaults(&found.kind.schema());
    let generated = generate(found, &defaults, 0, None)?;
    let kind = stored_kind(found.manifest.id, found.kind.id);
    Some((record_of(&kind, 0, &generated), generated))
}

/// Places a new object of `kind` (`<extension>/<kind>`) with its centre at `centre`, on top of the
/// active layer, in one commit. The seed comes from the new node id.
///
/// # Errors
/// [`InstanceError::UnknownKind`] for a kind that is not available, [`InstanceError::Refused`]
/// when the document refuses.
pub fn place(
    document: &Document,
    settings: &ExtensionSettings,
    kind: &str,
    centre: Point,
) -> Result<NodeId, InstanceError> {
    let found = find_kind(settings, kind).ok_or(InstanceError::UnknownKind)?;
    let (record, _) = new_record(&found).ok_or(InstanceError::UnknownKind)?;
    let frame = Affine::new(1.0, 0.0, 0.0, 1.0, centre.x, centre.y);
    document
        .create_generated_seeded(&record, frame)
        .map_err(|_| InstanceError::Refused)
}

/// What a parameter edit changed.
#[derive(Debug, Clone, PartialEq)]
pub struct Changed {
    /// The notice sentences of the values the generator clamped, the edited field's first.
    pub notices: Vec<String>,
    /// Whether a commit was written (a value equal to the stored one writes nothing).
    pub written: bool,
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
    let generated = generate(
        &found,
        &params,
        snapshot.record.seed,
        Some(Edit::Param(key)),
    )
    .ok_or(InstanceError::UnknownKind)?;
    let record = record_of(&snapshot.record.kind, snapshot.record.seed, &generated);
    let mut notices = notices(&found.kind, &generated.clamps);
    notices.extend(crate::instance_warnings::sentences(&generated.warnings));
    Ok(ValuePlan {
        snapshot,
        record,
        notices,
        states: generated.states,
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
        ..
    } = plan_value(document, settings, id, key, value)?;
    if record == snapshot.record {
        return Ok(Changed {
            notices,
            written: false,
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
    })
}

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
/// As [`change_value`]; [`InstanceError::Refused`] also for a result that is no affine image of
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

/// Converts the generated object `id` into plain paths in one commit (a gear becomes one path, a
/// box a group of its plates, under the same id), with fresh anchor ids from `minter`. Works for an
/// unknown kind too: it only reads the stored outline.
///
/// # Errors
/// [`InstanceError::NoSuchObject`] or [`InstanceError::Refused`].
pub fn convert(
    document: &Document,
    minter: &mut AnchorIdMinter,
    id: NodeId,
) -> Result<(), InstanceError> {
    let snapshot = document.generated(id).ok_or(InstanceError::NoSuchObject)?;
    let parts: Vec<Vec<(Vec<curvyo_document_core::NewAnchor>, bool)>> = snapshot
        .outlines()
        .into_iter()
        .map(|part| {
            part.into_iter()
                .map(|subpath| {
                    let anchors = subpath
                        .anchors
                        .into_iter()
                        .map(|mut anchor| {
                            anchor.id = minter.mint();
                            anchor
                        })
                        .collect();
                    (anchors, subpath.closed)
                })
                .collect()
        })
        .collect();
    document
        .convert_generated(id, &parts)
        .map_err(|_| InstanceError::Refused)
}

/// One row of the Generator block as the shell draws it.
#[derive(Debug, Clone, Serialize)]
pub struct RowView {
    /// The group heading it sits under.
    pub group: &'static str,
    /// The parameter, with its kind, bounds, step and decimals.
    pub param: Param,
    /// The stored value (the default for a missing or invalid one).
    pub value: ParamValue,
    /// For an automatic parameter stored as 0: the value the generator used, drawn muted.
    pub used: Option<ParamValue>,
    /// A word for a limit that is a shape ("full round"), reported by the generator.
    pub state: Option<String>,
    /// The largest value a bound that depends on other values allows now, for the slider's end.
    pub limit: Option<ParamValue>,
}

/// What the Properties panel shows for one generated object.
#[derive(Debug, Clone, Serialize)]
pub struct InstanceView {
    /// The object.
    pub id: String,
    /// The kind's name ("Gear"), or the stored `<extension>/<kind>` for an unknown one.
    pub label: String,
    /// Whether the generator is available: an unknown or disabled kind shows no block.
    pub known: bool,
    /// The rows whose condition holds, group by group.
    pub rows: Vec<RowView>,
}

/// The block of `snapshot`: its rows with their stored values, the hidden ones left out.
#[must_use]
pub fn view(settings: &ExtensionSettings, snapshot: &GeneratedSnapshot) -> InstanceView {
    let id = snapshot.id.full_id();
    let Some(found) = find_kind(settings, &snapshot.record.kind) else {
        return InstanceView {
            id,
            label: snapshot.record.kind.clone(),
            known: false,
            rows: Vec::new(),
        };
    };
    let schema = found.kind.schema();
    // The generator tells what it used for automatic rows, the words of shapes and the limits.
    let generated = generate(&found, &snapshot.record.params, snapshot.record.seed, None);
    let corners = generated.as_ref().map_or(0, |generated| {
        generated
            .marks
            .on_footprint
            .iter()
            .filter(|mark| matches!(mark, curvyo_extension_core::Mark::Corner { .. }))
            .count()
    });
    let (used, states, limits) = generated
        .map(|generated| (generated.automatic, generated.states, generated.limits))
        .unwrap_or_default();
    let rows = found
        .kind
        .groups
        .iter()
        .flat_map(|group| group.params.iter().map(|param| (group.label, param)))
        // The outline has no row (ADR 0083 §2).
        .filter(|(_, param)| !matches!(param.kind, ParamKind::Path))
        .filter(|(_, param)| {
            param.shown_when.is_none_or(|condition: ShownWhen| {
                condition.holds(&schema, &snapshot.record.params)
            })
        })
        .map(|(group, param)| RowView {
            group,
            param: param.clone(),
            value: param.read(&snapshot.record.params),
            used: used
                .iter()
                .find(|(key, _)| *key == param.key)
                .map(|(_, value)| value.clone()),
            state: states
                .iter()
                .find(|(key, _)| *key == param.key)
                .map(|(_, word)| (*word).to_owned())
                // The Corner angle says how many corners it makes (`0149` UX notes §5).
                .or_else(|| {
                    (param.key == "corner_angle")
                        .then(|| format!("{corners} corner{}", if corners == 1 { "" } else { "s" }))
                }),
            limit: limits
                .iter()
                .find(|(key, _)| *key == param.key)
                .map(|(_, value)| value.clone()),
        })
        .collect();
    InstanceView {
        id,
        label: found.kind.label.to_owned(),
        known: true,
        rows,
    }
}

/// The `JSON` value of a typed field text for `param`: a number for a length, count or angle.
/// A fraction typed into a count rounds half up (UX notes §4).
#[must_use]
pub fn value_from_number(param: &Param, number: f64) -> Option<ParamValue> {
    if !number.is_finite() {
        return None;
    }
    match param.kind {
        ParamKind::Length { .. } => Some(ParamValue::Length(
            curvyo_document_core::Length::from_mm(number),
        )),
        ParamKind::Angle { .. } => Some(ParamValue::Angle(curvyo_extension_core::Degrees(number))),
        ParamKind::Count { .. } => {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let count = (number + 0.5).floor().clamp(0.0, f64::from(u32::MAX)) as u32;
            Some(ParamValue::Count(count))
        }
        ParamKind::Switch | ParamKind::Choice { .. } | ParamKind::Path => None,
    }
}

/// The value a frontend sent as JSON (`true`, `"id"` or a number) for `param`.
#[must_use]
pub fn value_from_json(param: &Param, value: &Value) -> Option<ParamValue> {
    match (param.kind, value) {
        (ParamKind::Switch, Value::Bool(on)) => Some(ParamValue::Switch(*on)),
        (ParamKind::Choice { .. }, Value::String(id)) => Some(ParamValue::Choice(id.clone())),
        (_, Value::Number(number)) => value_from_number(param, number.as_f64()?),
        _ => None,
    }
}
