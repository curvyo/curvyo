//! The sketch-clone arm of the shared parametric-instance mechanism (`0099` criteria 24, 26 to 28,
//! ADR 0080 §6, `adrs.md` A7): the rows of the Parameters override (one per exposed value, with
//! the value a clone gets and where it comes from), a value typed or dragged, Original's, Own, and
//! the plan a drag previews. Nothing here knows a solver; document-core solves.

use curvyo_document_core::{
    CloneSnapshot, Document, ExposedKey, ExposedParam, InstanceKind, InstanceValue, InstanceWrite,
    NodeId, ObjectSnapshot, ValueSet,
};
use serde::Serialize;

use crate::instance::{Changed, InstanceError, shown};
use crate::instance_notice::say_number;

/// One value of the Parameters override as the shell draws it.
#[derive(Debug, Clone, Serialize)]
pub struct SketchRowView {
    /// The exposed key, `p:<id>` or `d:<id>`.
    pub key: String,
    /// The field's label.
    pub label: String,
    /// `mm` or `deg`.
    pub unit: &'static str,
    /// The value the selected clones show, `None` when they differ.
    pub value: Option<f64>,
    /// Whether the selected clones show different values.
    pub mixed: bool,
    /// Where the value comes from: `own`, `inherited` (up a chain), `sketch` or `mixed`.
    pub origin: &'static str,
    /// The sketch's own value.
    pub sketch_value: f64,
    /// The lower limit, `None` for none.
    pub min: Option<f64>,
    /// The upper limit, `None` for none.
    pub max: Option<f64>,
    /// The span of the field's track (`UX notes 12.2`): its start.
    pub track_min: f64,
    /// The end of the track.
    pub track_max: f64,
    /// Whether the track runs logarithmically.
    pub track_log: bool,
    /// Whether a selected clone is a clone of a clone, for "Back to the original's value".
    pub chained: bool,
}

/// What the Parameters override shows for the selected clones of one sketch.
#[derive(Debug, Clone, Serialize)]
pub struct SketchInstanceView {
    /// The sketch.
    pub sketch: String,
    /// The sketch's name for the fields' accessible names ("Bracket clone width"): its own name,
    /// else the name of the page it owns, else "Sketch".
    pub sketch_name: String,
    /// How many clones are selected.
    pub clones: usize,
    /// `original`, `own` or `mixed`: which segment of the pill is pressed.
    pub state: &'static str,
    /// Whether a selected clone follows a value set that has no solution.
    pub no_solution: bool,
    /// One row per exposed value.
    pub rows: Vec<SketchRowView>,
}

fn clone_of(document: &Document, id: NodeId) -> Option<CloneSnapshot> {
    match document.read_objects().get(id)? {
        ObjectSnapshot::Clone(clone) => Some(clone.clone()),
        _ => None,
    }
}

/// The clones `ids`, each as `stepped` has it while a field is being stepped, else as committed.
fn clones_of(document: &Document, ids: &[NodeId], stepped: &[CloneSnapshot]) -> Vec<CloneSnapshot> {
    let read = document.read_objects();
    ids.iter()
        .filter_map(|id| match read.get(*id)? {
            ObjectSnapshot::Clone(clone) => Some(
                stepped
                    .iter()
                    .find(|preview| preview.id() == *id)
                    .unwrap_or(clone)
                    .clone(),
            ),
            _ => None,
        })
        .collect()
}

fn number_of(set: &ValueSet, key: &ExposedKey) -> Option<f64> {
    set.get(key).map(InstanceValue::number)
}

/// The track of a field (`UX notes 12.2`): both limits span them; else a length runs log from the
/// lower limit (or 0) to 4 times the sketch's value (at least 10 mm), an angle linear 0 to 360.
fn track(param: &ExposedParam) -> (f64, f64, bool) {
    match (param.min, param.max, param.kind) {
        (Some(min), Some(max), _) => (min, max, false),
        (min, max, InstanceKind::Length) => (
            min.unwrap_or(0.0),
            max.unwrap_or_else(|| (4.0 * param.default.abs()).max(10.0)),
            true,
        ),
        (min, max, InstanceKind::Angle) => (min.unwrap_or(0.0), max.unwrap_or(360.0), false),
    }
}

fn sketch_name(document: &Document, sketch: NodeId) -> String {
    document
        .node_registers(sketch)
        .and_then(|registers| registers.name)
        .or_else(|| {
            document
                .pages()
                .into_iter()
                .find(|page| document.page_sketch(&page.id) == Some(sketch))
                .map(|page| page.name)
        })
        .unwrap_or_else(|| "Sketch".to_owned())
}

/// The Parameters override of the selected `ids`, `None` when they are no clones of one sketch
/// that exposes a value (the row is not in the tree). `stepped` are the clones of a field being
/// stepped: the fields follow the preview as the canvas does.
#[must_use]
pub fn view(
    document: &Document,
    ids: &[NodeId],
    stepped: &[CloneSnapshot],
) -> Option<SketchInstanceView> {
    let clones = clones_of(document, ids, stepped);
    let first = clones.first()?;
    let root = first.chain.root;
    if clones.iter().any(|clone| clone.chain.root != root) {
        // Clones of different sketches: the mixed pill and no fields (UX notes 12.2), while one
        // of the sketches exposes a value.
        let exposes = clones
            .iter()
            .any(|clone| !document.exposed_params(clone.chain.root).is_empty());
        return exposes.then(|| SketchInstanceView {
            sketch: String::new(),
            sketch_name: String::new(),
            clones: clones.len(),
            state: "mixed",
            no_solution: clones.iter().any(|clone| clone.chain.no_solution),
            rows: Vec::new(),
        });
    }
    let sketch = document.sketch(root)?;
    if sketch.exposed.is_empty() {
        return None;
    }
    let owning = clones.iter().filter(|clone| clone.chain.own_params).count();
    let state = match owning {
        0 => "original",
        n if n == clones.len() => "own",
        _ => "mixed",
    };
    let rows = sketch
        .exposed
        .iter()
        .map(|param| {
            let shows: Vec<(f64, &'static str)> = clones
                .iter()
                .map(|clone| {
                    if let Some(own) = number_of(&clone.chain.own_values, &param.key) {
                        (own, "own")
                    } else if let Some(up) = number_of(&clone.chain.inherited_values, &param.key) {
                        (up, "inherited")
                    } else {
                        (param.default, "sketch")
                    }
                })
                .collect();
            let (first_value, first_origin) = shows[0];
            let mixed = shows
                .iter()
                .any(|(value, _)| (value - first_value).abs() > 1e-9);
            let origin = if shows.iter().all(|(_, origin)| *origin == first_origin) {
                first_origin
            } else {
                "mixed"
            };
            let (track_min, track_max, track_log) = track(param);
            SketchRowView {
                key: param.key.as_str().to_owned(),
                label: param.label.clone(),
                unit: match param.kind {
                    InstanceKind::Length => "mm",
                    InstanceKind::Angle => "deg",
                },
                value: (!mixed).then_some(first_value),
                mixed,
                origin,
                sketch_value: param.default,
                min: param.min,
                max: param.max,
                track_min,
                track_max,
                track_log,
                chained: clones.iter().any(|clone| clone.chain.depth > 1),
            }
        })
        .collect();
    Some(SketchInstanceView {
        sketch: root.full_id(),
        sketch_name: sketch_name(document, root),
        clones: clones.len(),
        state,
        no_solution: clones.iter().any(|clone| clone.chain.no_solution),
        rows,
    })
}

/// What a value would write for one clone, without writing: the notices, the clone as it would
/// read (`None` when the values have no solution) and the whole set of own values.
#[derive(Debug, Clone)]
pub struct SketchPlan {
    /// The clone with the value in place; `None` for a value set without solution.
    pub preview: Option<CloneSnapshot>,
    /// The clone's whole new set of own values.
    pub own: ValueSet,
    /// The notice sentences of a value held at a limit.
    pub notices: Vec<String>,
}

fn value_of(param: &ExposedParam, number: f64) -> (InstanceValue, Option<String>) {
    let (clamped, was) = param.clamp(number);
    let value = match param.kind {
        InstanceKind::Length => InstanceValue::from_mm(clamped),
        InstanceKind::Angle => InstanceValue::from_deg(clamped),
    };
    let notice = was.then(|| {
        let bound = if param.min.is_some_and(|min| clamped <= min) && number < clamped {
            "minimum"
        } else {
            "maximum"
        };
        let said = say_number(param.kind == InstanceKind::Angle, clamped);
        format!("{} set to {said}, the {bound}.", param.label)
    });
    (value, notice)
}

fn exposed_param(document: &Document, clone: &CloneSnapshot, key: &str) -> Option<ExposedParam> {
    let key = ExposedKey::parse(key)?;
    document
        .exposed_params(clone.chain.root)
        .into_iter()
        .find(|param| param.key == key)
}

/// Works out setting the exposed value `key` of `clone` to `number` (millimetres or degrees)
/// without writing: the value is clamped into the limits, the clone is read with it.
///
/// # Errors
/// [`InstanceError::NoSuchObject`] for a clone that follows no sketch, [`InstanceError::WrongValue`]
/// for a key that is not exposed or a number that is not finite.
pub fn plan(
    document: &Document,
    clone: NodeId,
    key: &str,
    number: f64,
) -> Result<SketchPlan, InstanceError> {
    let read = clone_of(document, clone).ok_or(InstanceError::NoSuchObject)?;
    let param = exposed_param(document, &read, key).ok_or(InstanceError::WrongValue)?;
    if !number.is_finite() {
        return Err(InstanceError::WrongValue);
    }
    let (value, notice) = value_of(&param, number);
    let mut own = read.chain.own_values.clone();
    own.insert(param.key.clone(), value);
    Ok(SketchPlan {
        preview: document.instance_preview(clone, &own),
        own,
        notices: notice.into_iter().collect(),
    })
}

/// Sets the exposed value `key` of every clone of `ids` to `number` (millimetres or degrees), in
/// one commit "Change parameter"; a typed value is clamped with a notice. `last_solvable` gives,
/// per clone, the last value set a drag step solved, kept as the saved fallback when this one has
/// no solution.
///
/// # Errors
/// [`InstanceError::NoSuchObject`], [`InstanceError::WrongValue`] or [`InstanceError::Refused`].
pub fn change(
    document: &Document,
    ids: &[NodeId],
    key: &str,
    number: f64,
    last_solvable: &[(NodeId, ValueSet)],
) -> Result<Changed, InstanceError> {
    let mut writes = Vec::new();
    let mut notices = Vec::new();
    for &id in ids {
        let read = clone_of(document, id).ok_or(InstanceError::NoSuchObject)?;
        let param = exposed_param(document, &read, key).ok_or(InstanceError::WrongValue)?;
        if !number.is_finite() {
            return Err(InstanceError::WrongValue);
        }
        let (value, notice) = value_of(&param, number);
        let mut own = read.chain.own_values.clone();
        own.insert(param.key.clone(), value);
        if notices.is_empty() {
            notices.extend(notice);
        }
        if own != read.chain.own_values {
            writes.push(InstanceWrite {
                clone: id,
                own,
                last_solvable: last_solvable
                    .iter()
                    .find(|(clone, _)| *clone == id)
                    .map(|(_, set)| set.clone()),
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

/// Drops the own value `key` of every clone of `ids` ("Back to the sketch's value"), or with
/// `None` every own value of each ("Original's": "Back to the sketch's values").
///
/// # Errors
/// [`InstanceError::NoSuchObject`] or [`InstanceError::Refused`].
pub fn reset(
    document: &Document,
    ids: &[NodeId],
    key: Option<&str>,
) -> Result<bool, InstanceError> {
    let mut writes = Vec::new();
    for &id in ids {
        let read = clone_of(document, id).ok_or(InstanceError::NoSuchObject)?;
        let mut own = read.chain.own_values.clone();
        match key.and_then(ExposedKey::parse) {
            Some(key) => own.remove(&key),
            None => own = ValueSet::default(),
        }
        if own != read.chain.own_values {
            writes.push(InstanceWrite {
                clone: id,
                own,
                last_solvable: None,
            });
        }
    }
    if writes.is_empty() {
        return Ok(false);
    }
    let (label, subject) = match key {
        Some(key) => (
            curvyo_document_core::RESET_PARAMETER_COMMIT_LABEL,
            ExposedKey::parse(key),
        ),
        None => (curvyo_document_core::RESET_PARAMETERS_COMMIT_LABEL, None),
    };
    document
        .write_instance_values(&writes, label, subject.as_ref())
        .map_err(|_| InstanceError::Refused)?;
    Ok(true)
}

/// Writes every value each clone of `ids` shows as its own ("Own parameter values"), so nothing
/// moves.
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
