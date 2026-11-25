//! The Parameters override of selected sketch clones as the shell draws it (`0099` criteria 24, 28,
//! 32 to 36, UX notes 12.2 and 12.9.2): one row per exposed value with the value shown, where it
//! comes from and its link state, read from the committed clones except the stepped number.

use curvyo_document_core::{
    CloneSnapshot, Document, ExposedKey, ExposedParam, InstanceKind, InstanceValue, NodeId,
    ObjectSnapshot, ValueSet,
};
use serde::Serialize;

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
    /// The letter of its link group ("A"), `None` when it is in none (UX notes 12.9).
    pub group: Option<String>,
    /// The labels of the other members of its group, in exposed order.
    pub partners: Vec<String>,
    /// `none` (no group), `linked`, `unlinked` (the clones hold it unlinked) or `mixed`.
    pub link: &'static str,
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

/// The clones `ids`, each as `stepped` has it while a field is being stepped, else as committed.
pub(crate) fn clones_of(
    document: &Document,
    ids: &[NodeId],
    stepped: &[CloneSnapshot],
) -> Vec<CloneSnapshot> {
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

/// The letter of link group `group` (A, B, C, ...; display only, UX notes 12.9).
#[must_use]
pub fn group_letter(group: usize) -> String {
    let mut text = String::new();
    let mut n = group;
    loop {
        text.insert(0, char::from(b'A' + u8::try_from(n % 26).unwrap_or(0)));
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    text
}

/// One value per clone and where it comes from.
fn shown(clone: &CloneSnapshot, param: &ExposedParam) -> (f64, &'static str) {
    if let Some(own) = number_of(&clone.chain.own_values, &param.key) {
        (own, "own")
    } else if let Some(up) = number_of(&clone.chain.inherited_values, &param.key) {
        (up, "inherited")
    } else {
        (param.default, "sketch")
    }
}

fn row(
    param: &ExposedParam,
    exposed: &[ExposedParam],
    committed: &[CloneSnapshot],
    stepped: &[CloneSnapshot],
) -> SketchRowView {
    let values: Vec<f64> = stepped.iter().map(|clone| shown(clone, param).0).collect();
    let origins: Vec<&'static str> = committed
        .iter()
        .map(|clone| shown(clone, param).1)
        .collect();
    let first_value = values[0];
    let mixed = values
        .iter()
        .any(|value| (value - first_value).abs() > 1e-9);
    let origin = if origins.iter().all(|origin| *origin == origins[0]) {
        origins[0]
    } else {
        "mixed"
    };
    let unlinked = committed
        .iter()
        .filter(|clone| clone.chain.unlinked.contains(&param.key))
        .count();
    let link = if param.group.is_none() {
        "none"
    } else if unlinked == 0 {
        "linked"
    } else if unlinked == committed.len() {
        "unlinked"
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
        chained: committed.iter().any(|clone| clone.chain.depth > 1),
        group: param.group.map(group_letter),
        partners: exposed
            .iter()
            .filter(|other| {
                other.group.is_some() && other.group == param.group && other.key != param.key
            })
            .map(|other| other.label.clone())
            .collect(),
        link,
    }
}

/// The Parameters override of the selected `ids`, `None` when they are no clones of one sketch
/// that exposes a value (the row is not in the tree). `stepped` are the clones of a gesture in
/// flight: only the numbers follow them; the pill, the origins and the link marks keep the
/// committed state until the commit (`0099` criterion 24, `adrs.md` A15 (b)).
#[must_use]
pub fn view(
    document: &Document,
    ids: &[NodeId],
    stepped: &[CloneSnapshot],
) -> Option<SketchInstanceView> {
    let committed = clones_of(document, ids, &[]);
    let first = committed.first()?;
    let root = first.chain.root;
    if committed.iter().any(|clone| clone.chain.root != root) {
        // Clones of different sketches: the mixed pill and no fields (UX notes 12.2), while one
        // of the sketches exposes a value.
        let exposes = committed
            .iter()
            .any(|clone| !document.exposed_params(clone.chain.root).is_empty());
        return exposes.then(|| SketchInstanceView {
            sketch: String::new(),
            sketch_name: String::new(),
            clones: committed.len(),
            state: "mixed",
            no_solution: committed.iter().any(|clone| clone.chain.no_solution),
            rows: Vec::new(),
        });
    }
    let sketch = document.sketch(root)?;
    if sketch.exposed.is_empty() {
        return None;
    }
    let shown_clones = clones_of(document, ids, stepped);
    let owning = committed
        .iter()
        .filter(|clone| clone.chain.own_params)
        .count();
    let state = match owning {
        0 => "original",
        n if n == committed.len() => "own",
        _ => "mixed",
    };
    let rows = sketch
        .exposed
        .iter()
        .map(|param| row(param, &sketch.exposed, &committed, &shown_clones))
        .collect();
    Some(SketchInstanceView {
        sketch: root.full_id(),
        sketch_name: sketch_name(document, root),
        clones: committed.len(),
        state,
        no_solution: committed.iter().any(|clone| clone.chain.no_solution),
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::group_letter;

    #[test]
    fn groups_are_lettered_in_order() {
        assert_eq!(
            [0, 1, 25, 26].map(group_letter),
            ["A", "B", "Z", "AA"].map(String::from)
        );
    }
}
