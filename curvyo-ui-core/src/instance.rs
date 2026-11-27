//! The generated-object arm of the shared parametric-instance mechanism (ADR 0080 §6): the schema
//! view, a parameter edit, a proportional resize and Convert to paths of a generated object. The
//! sketch instances of `0099` add their arm to the same functions.
//!
//! Nothing here writes more than one commit per call, and nothing is written for a kind the
//! registry does not know or the maker switched off: that object is shown from its stored outline
//! and refuses a change by number or a resize (ADR 0080 §5).

use curvyo_document_core::{Affine, Document, GeneratedRecord, GeneratedSnapshot, NodeId, Point};
use curvyo_extension_core::{
    Edit, GenerateRequest, Generated, GeneratorKind, Manifest, Param, ParamKind, ParamValue, Part,
    Seed, StoredParams,
};

use crate::anchor_id_minter::AnchorIdMinter;
use crate::extensions::{ExtensionSettings, generate_object, manifests};
pub use crate::instance_notice::{clamp_notice, noun, shown};
pub use crate::instance_resize::{ResizeView, resize_view};
pub use crate::instance_view::{
    InstanceView, PartView, RowView, part_name, value_from_json, value_from_number, view,
};

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
    let mut kinds: Vec<KindRef> = manifests()
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
        .collect();
    // The built-ins by the text of their segment (`0144` UX notes §1); stable, so one extension's
    // own order holds for equal texts.
    kinds.sort_by_key(|found| found.kind.segment());
    kinds
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
        part_keys: generated.parts.iter().map(|p| p.key.to_string()).collect(),
    }
}

/// The record of an edit of the object whose record is `stored`: what a generator returned, or,
/// when it answered no parts (the content cannot be drawn, ADR 0091 §3), the stored parts with
/// the new parameters, so the object keeps its last valid geometry.
pub(crate) fn record_keeping(stored: &GeneratedRecord, generated: &Generated) -> GeneratedRecord {
    if generated.parts.is_empty() {
        return GeneratedRecord {
            kind: stored.kind.clone(),
            seed: stored.seed,
            params: generated.params.clone(),
            parts: stored.parts.clone(),
            part_keys: stored.part_keys.clone(),
        };
    }
    record_of(&stored.kind, stored.seed, generated)
}

pub(crate) fn generate(
    found: &KindRef,
    params: &StoredParams,
    seed: u32,
    edit: Option<Edit<'_>>,
) -> Option<Generated> {
    // The one funnel every writer goes through (ADR 0092 §3.3): a kind with a Font parameter gets
    // the outlines of the face that parameter names, from the registry the session set.
    let table = found
        .kind
        .params()
        .find(|param| matches!(param.kind, ParamKind::Font))
        .and_then(|param| match param.read(params) {
            ParamValue::Font(font) => crate::code_glyphs::glyphs_for(font.as_ref()),
            _ => None,
        });
    generate_object(
        found.manifest.id,
        &GenerateRequest {
            kind: found.kind.id,
            params,
            seed: Seed(seed),
            edit,
            glyphs: table.as_deref(),
        },
    )
}

/// The part keys a fresh generation of `snapshot` has, in part order: what a record written before
/// format 65 gets in the Detach part step (ADR 0082 §10). Empty for an unknown kind.
#[must_use]
pub fn part_keys(settings: &ExtensionSettings, snapshot: &GeneratedSnapshot) -> Vec<String> {
    find_kind(settings, &snapshot.record.kind)
        .and_then(|found| generate(&found, &snapshot.record.params, snapshot.record.seed, None))
        .map(|generated| generated.parts.iter().map(|p| p.key.to_string()).collect())
        .unwrap_or_default()
}

/// The warning sentences of the generated object `snapshot`, empty for an unknown kind.
#[must_use]
pub fn warning_notices(settings: &ExtensionSettings, snapshot: &GeneratedSnapshot) -> Vec<String> {
    let Some(found) = find_kind(settings, &snapshot.record.kind) else {
        return Vec::new();
    };
    generate(&found, &snapshot.record.params, snapshot.record.seed, None)
        .map(|g| {
            crate::instance_warnings::sentences_in(
                &g.warnings,
                crate::instance_warnings::Context::of(&g, found.kind.label),
            )
        })
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

/// Whether the selected generated object draws overlay marks: a path box always, a rounded box
/// with the curved top (bend bands on the end plates' arcs, part numbers; ux-notes of `0147`
/// §10.8, §10.9); Flat draws none.
#[must_use]
pub fn draws_marks(settings: &ExtensionSettings, snapshot: &GeneratedSnapshot) -> bool {
    let Some(found) = find_kind(settings, &snapshot.record.kind) else {
        return false;
    };
    found.kind.source.is_some()
        || (snapshot.record.kind
            == stored_kind(
                curvyo_generators_core::PROCESS_ID,
                curvyo_generators_core::ROUNDED_BOX,
            )
            && snapshot
                .record
                .params
                .get("top_shape")
                .and_then(ParamValue::from_stored)
                .is_some_and(|value| value.choice() == Some("curved")))
}

/// Whether a generator of `kind` can answer no parts at all: only one with a text parameter can
/// (ADR 0091 §3: content that cannot be clamped), so a box or a gear is never asked.
#[must_use]
pub fn can_refuse(settings: &ExtensionSettings, kind: &str) -> bool {
    find_kind(settings, kind).is_some_and(|found| {
        found
            .kind
            .params()
            .any(|param| matches!(param.kind, curvyo_extension_core::ParamKind::Text { .. }))
    })
}

/// Whether the generator of `kind` answers no parts for `params`: the stored outline is then the last
/// valid one and is drawn in the No solution look (ADR 0091 §3). `false` for an unknown kind.
#[must_use]
pub fn is_refused(
    settings: &ExtensionSettings,
    kind: &str,
    params: &StoredParams,
    seed: u32,
) -> bool {
    find_kind(settings, kind)
        .and_then(|found| generate(&found, params, seed, None))
        .is_some_and(|generated| generated.parts.is_empty())
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

pub use crate::instance_scale::{ResizePlan, apply_resize, is_resize, plan_resize, resize};
pub use crate::instance_switch::{Switched, switch_kind};
pub use crate::instance_value::{
    Changed, InstanceSource, ValuePlan, change_parameter, change_value, plan_value,
};

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
