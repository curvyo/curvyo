//! Which objects the Style panel edits for the active tool, and the line that
//! says so (`specs/0007-stroke-and-fill-styling` criterion 37).

use curvyo_document_core::{FontKind, NodeId, ObjectSnapshot, PrimitiveSnapshot, Shape, StyleEdit};

use crate::object_selection::{ObjectSelection, objects_with_ids};
use crate::selection::NodeSelection;
use crate::units::Units;

/// The tool the panel is scoped by (criterion 37). The creation tools and
/// Select all edit the object selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleTool {
    /// The pen: the path being drawn does not exist yet; nothing to edit.
    Pen,
    /// The node tool: the paths that own the selected nodes.
    Node,
    /// Select and the rectangle, ellipse and polygon/star tools.
    Other,
}

/// The objects the panel edits and the line that says so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleScope {
    /// The objects an edit goes to; empty when the panel has nothing to edit. A linked clone is
    /// among them only with its own style (`specs/0057-clone-overrides` criterion 18).
    pub ids: Vec<NodeId>,
    /// A kind ("Rectangle") or a count ("3 rectangles", "4 objects", "2
    /// paths"); empty when there is nothing to edit.
    pub subject: String,
}

impl StyleScope {
    /// The objects `edit` goes to: all of the scope, or for a marker edit only
    /// its paths, since a primitive has no markers (`specs/0018-stroke-markers`
    /// criterion 22).
    /// A fill edit leaves single-line texts alone (`specs/0076-single-line-fonts` criterion 8).
    #[must_use]
    pub fn targets(&self, objects: &[ObjectSnapshot], edit: &StyleEdit) -> Vec<NodeId> {
        if edit.is_marker_edit() {
            paths_among(&self.ids, objects)
        } else if edit.is_fill_edit() {
            objects_with_ids(objects, &self.ids)
                .into_iter()
                .filter(|object| !is_single_line_text(object))
                .map(ObjectSnapshot::id)
                .collect()
        } else {
            self.ids.clone()
        }
    }
}

/// Whether `object` is a text in a single-line face (by its stored kind), which has no fill.
#[must_use]
pub fn is_single_line_text(object: &ObjectSnapshot) -> bool {
    matches!(object, ObjectSnapshot::Text(text) if text.font.kind == FontKind::SingleLine)
}

/// How many of `ids` are single-line texts a fill edit leaves alone: the count of the style skip
/// notice (`specs/0076-single-line-fonts`, UX notes §4).
#[must_use]
pub fn skipped_single_line_texts(objects: &[ObjectSnapshot], ids: &[NodeId]) -> usize {
    objects_with_ids(objects, ids)
        .into_iter()
        .filter(|object| is_single_line_text(object))
        .count()
}

/// The ids in `ids` that are paths, or clones with their own style (they take markers like a
/// path), in order.
fn paths_among(ids: &[NodeId], objects: &[ObjectSnapshot]) -> Vec<NodeId> {
    objects_with_ids(objects, ids)
        .into_iter()
        .filter(|object| match object {
            ObjectSnapshot::Path(_) => true,
            ObjectSnapshot::Clone(clone) => clone.chain.own_style,
            ObjectSnapshot::Primitive(_) | ObjectSnapshot::Text(_) => false,
        })
        .map(ObjectSnapshot::id)
        .collect()
}

/// How many of `ids` are clones that show their original's style, which a style or opacity edit
/// leaves alone: the count of the notice of `specs/0057-clone-overrides` criterion 18.
#[must_use]
pub fn skipped_clones(objects: &[ObjectSnapshot], ids: &[NodeId]) -> usize {
    objects_with_ids(objects, ids)
        .into_iter()
        .filter(|object| !owns_style(object))
        .count()
}

/// Whether an object owns the style it shows: every object but a clone without own style.
#[must_use]
pub fn owns_style(object: &ObjectSnapshot) -> bool {
    !matches!(object, ObjectSnapshot::Clone(clone) if !clone.chain.own_style)
}

/// The scope of the panel for `tool` (criterion 37). Ids the document no
/// longer holds are dropped.
#[must_use]
pub fn style_scope(
    tool: StyleTool,
    objects: &[ObjectSnapshot],
    selection: &ObjectSelection,
    nodes: &NodeSelection,
) -> StyleScope {
    style_scope_in(tool, objects, selection, nodes, &Units::default())
}

/// [`style_scope`] with groups (`specs/0023-groups` criteria 11 and 26): a selected group is
/// edited through its leaves, and the subject line names it: "Group of 5 objects", "2 groups" or,
/// for a mix of objects of the context, "4 objects".
#[must_use]
pub fn style_scope_in(
    tool: StyleTool,
    objects: &[ObjectSnapshot],
    selection: &ObjectSelection,
    nodes: &NodeSelection,
    units: &Units,
) -> StyleScope {
    if tool == StyleTool::Pen {
        return StyleScope {
            ids: Vec::new(),
            subject: String::new(),
        };
    }
    let wanted: Vec<NodeId> = if tool == StyleTool::Node {
        let owners = node_owners(nodes);
        if owners.is_empty() {
            // The path being edited: the paths of the object selection.
            paths_among(selection.ids(), objects)
        } else {
            owners
        }
    } else {
        selection.ids().to_vec()
    };
    let held = objects_with_ids(objects, &wanted);
    let all: Vec<NodeId> = held.iter().map(|object| object.id()).collect();
    let subject = group_subject(tool, &all, units).unwrap_or_else(|| match held.as_slice() {
        [ObjectSnapshot::Clone(clone)] => clone_subject(objects, clone),
        _ => subject_line(&held),
    });
    // A linked clone without its own style shows its original's: an edit never goes to it, and
    // "Mixed" is read from the other objects (`specs/0049-linked-clone` criterion 24 as extended
    // by `specs/0057-clone-overrides` criterion 18).
    let ids = held
        .iter()
        .filter(|object| owns_style(object))
        .map(|object| object.id())
        .collect();
    StyleScope { ids, subject }
}

/// The subject line of a selection that holds a group, `None` when it holds none (or the panel is
/// scoped by nodes): one group is "Group of 5 objects", several groups "2 groups", groups and
/// leaves together a count of the objects of the context ("4 objects").
fn group_subject(tool: StyleTool, ids: &[NodeId], units: &Units) -> Option<String> {
    if tool == StyleTool::Node {
        return None;
    }
    let selected = units.selected(ids);
    if !selected.iter().any(|unit| unit.is_group) {
        return None;
    }
    let all_groups = selected.iter().all(|unit| unit.is_group);
    Some(match selected.as_slice() {
        [unit] => format!(
            "Group of {} {}",
            unit.leaves.len(),
            if unit.leaves.len() == 1 {
                "object"
            } else {
                "objects"
            }
        ),
        _ if all_groups => format!("{} groups", selected.len()),
        _ => format!("{} objects", selected.len()),
    })
}

/// The paths owning the selected nodes or the selected segment, in selection
/// order, once each.
fn node_owners(nodes: &NodeSelection) -> Vec<NodeId> {
    let mut owners: Vec<NodeId> = Vec::new();
    let paths = nodes.node_pairs().iter().map(|&(path, _)| path);
    let segment = nodes.segment_with_path().map(|(path, _, _)| path);
    for path in paths.chain(segment) {
        if !owners.contains(&path) {
            owners.push(path);
        }
    }
    owners
}

/// The subject line of one selected clone: "Clone of " and its direct original's name
/// (`specs/0049-linked-clone` criterion 21), each step of the chain found by id.
fn clone_subject(
    objects: &[ObjectSnapshot],
    clone: &curvyo_document_core::CloneSnapshot,
) -> String {
    let mut prefix = String::from("Clone of ");
    let mut current = clone.chain.origin;
    for _ in 0..curvyo_document_core::MAX_CLONE_DEPTH {
        match objects.iter().find(|object| object.id() == current) {
            Some(ObjectSnapshot::Clone(next)) => {
                prefix.push_str("Clone of ");
                current = next.chain.origin;
            }
            Some(object) => {
                return format!("{prefix}{} {}", kind_name(object).0, current.short_id());
            }
            None => return format!("{prefix}Object {}", current.short_id()),
        }
    }
    format!("{prefix}…")
}

fn kind_name(object: &ObjectSnapshot) -> (&'static str, &'static str) {
    match object {
        ObjectSnapshot::Path(path) if path.is_compound() => ("Compound path", "compound paths"),
        ObjectSnapshot::Path(_) => ("Path", "paths"),
        ObjectSnapshot::Clone(_) => ("Clone", "clones"),
        ObjectSnapshot::Text(_) => ("Text", "texts"),
        ObjectSnapshot::Primitive(PrimitiveSnapshot { shape, .. }) => match shape {
            Shape::Rect { .. } => ("Rectangle", "rectangles"),
            Shape::Ellipse { .. } => ("Ellipse", "ellipses"),
            Shape::Polygon { .. } => ("Polygon", "polygons"),
            Shape::Star { .. } => ("Star", "stars"),
        },
    }
}

/// What one object is called in a subject line: "Rectangle", "Path", "Compound path".
#[must_use]
pub fn object_kind_name(object: &ObjectSnapshot) -> &'static str {
    kind_name(object).0
}

fn subject_line(objects: &[&ObjectSnapshot]) -> String {
    let Some(first) = objects.first() else {
        return String::new();
    };
    let (singular, plural) = kind_name(first);
    let same_kind = objects.iter().all(|object| kind_name(object).0 == singular);
    // Compound paths are paths: a mix of them and ordinary paths is "N paths".
    let all_paths = objects
        .iter()
        .all(|object| matches!(object, ObjectSnapshot::Path(_)));
    match (objects.len(), same_kind) {
        (1, _) => singular.to_string(),
        (count, true) => format!("{count} {plural}"),
        (count, false) if all_paths => format!("{count} paths"),
        (count, false) => format!("{count} objects"),
    }
}
