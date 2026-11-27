//! The schema view of a generated object for the Properties panel: its rows with their stored
//! values, and the text-to-value conversions of a field.

use curvyo_document_core::GeneratedSnapshot;
use curvyo_extension_core::{Param, ParamKind, ParamValue, ShownWhen};
use serde::Serialize;
use serde_json::Value;

use crate::extensions::ExtensionSettings;
use crate::instance::{find_kind, generate};

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
    /// Why the stored value cannot be drawn ("Enter a text to encode."): a lasting line under the
    /// field while the object keeps its last geometry (`0144` criterion 13).
    pub problem: Option<String>,
}

/// One part of a generated object the Parts group lists (ADR 0082 §10, `0147` UX notes §9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PartView {
    /// The part's stable key.
    pub key: String,
    /// What the row calls it ("Inner base").
    pub name: &'static str,
}

/// The name of the part `key` of a rounded box (`0147` UX notes §9); `None` for a key the
/// Parts group does not list.
#[must_use]
pub fn part_name(key: &str) -> Option<&'static str> {
    Some(match key {
        "strip" => "Strip",
        "base" => "Inner base",
        "cover" => "Cover",
        "ring" => "Ring",
        "lid" => "Lid",
        "left" => "Left end plate",
        "right" => "Right end plate",
        "front" => "Front",
        _ => return None,
    })
}

/// The kind whose parts can be detached in this slice (`0147`).
const DETACHABLE_KIND: &str = "curvyo.generators/rounded-box";

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
    /// The parts the Parts group lists, in layout order; empty for a kind that has none.
    pub parts: Vec<PartView>,
}

/// The row the generator names for its reason it drew nothing, with the sentence of that reason
/// (the content never appears in it).
fn row_problem(
    generated: &curvyo_extension_core::Generated,
    label: &'static str,
) -> Option<(&'static str, String)> {
    let drew_nothing = generated.parts.is_empty();
    let warnings: Vec<_> = generated
        .warnings
        .iter()
        .filter(|warning| crate::instance_warnings::is_row_problem(warning, drew_nothing))
        .cloned()
        .collect();
    let row = warnings.first()?.param?;
    crate::instance_warnings::sentences_in(
        &warnings,
        crate::instance_warnings::Context::of(generated, label),
    )
    .into_iter()
    .next()
    .map(|sentence| (row, sentence))
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
            parts: Vec::new(),
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
    let parts = generated
        .as_ref()
        .filter(|_| snapshot.record.kind == DETACHABLE_KIND)
        .map(|generated| {
            generated
                .parts
                .iter()
                .filter_map(|part| {
                    Some(PartView {
                        key: part.key.to_string(),
                        name: part_name(&part.key)?,
                    })
                })
                .collect::<Vec<_>>()
        })
        .filter(|parts| parts.len() > 1)
        .unwrap_or_default();
    let problem = generated
        .as_ref()
        .and_then(|generated| row_problem(generated, found.kind.label));
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
                    (param.key == "corner_angle").then(|| match corners {
                        0 => "no corners".to_owned(),
                        1 => "1 corner".to_owned(),
                        n => format!("{n} corners"),
                    })
                }),
            limit: limits
                .iter()
                .find(|(key, _)| *key == param.key)
                .map(|(_, value)| value.clone()),
            problem: problem
                .as_ref()
                .filter(|(row, _)| *row == param.key)
                .map(|(_, sentence)| sentence.clone()),
        })
        .collect();
    InstanceView {
        id,
        label: found.kind.label.to_owned(),
        known: true,
        rows,
        parts,
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
        ParamKind::Percent { .. } => {
            #[allow(clippy::cast_possible_truncation)]
            let percent = (number + 0.5)
                .floor()
                .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32;
            Some(ParamValue::Percent(percent))
        }
        ParamKind::Ratio { .. } => Some(ParamValue::Ratio(number)),
        ParamKind::Switch
        | ParamKind::Choice { .. }
        | ParamKind::Path
        | ParamKind::Text { .. }
        | ParamKind::Font => None,
    }
}

/// The value a frontend sent as JSON (`true`, `"id"` or a number) for `param`.
#[must_use]
pub fn value_from_json(param: &Param, value: &Value) -> Option<ParamValue> {
    match (param.kind, value) {
        (ParamKind::Switch, Value::Bool(on)) => Some(ParamValue::Switch(*on)),
        (ParamKind::Choice { .. }, Value::String(id)) => Some(ParamValue::Choice(id.clone())),
        (ParamKind::Text { .. }, Value::String(text)) => Some(ParamValue::Text(text.clone())),
        (ParamKind::Font, Value::Null) => Some(ParamValue::Font(None)),
        (ParamKind::Font, Value::Object(_)) => {
            ParamValue::from_stored(&serde_json::json!({ "font": value }))
        }
        (_, Value::Number(number)) => value_from_number(param, number.as_f64()?),
        _ => None,
    }
}
