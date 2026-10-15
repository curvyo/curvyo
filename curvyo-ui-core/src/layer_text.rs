//! The words of the Layers panel and of the layer refusals on the canvas
//! (`specs/0050-layers-and-objects-panel` criteria 4, 14, 23, 26, 32, 36 and 41, the UX notes).
//! Every name is plain text; the host never reads one as markup (criterion 17).

/// The layer as a sentence names it: "Layer Cut", or the name alone when it already starts with
/// "Layer" ("Layer 1", "Initial Layer" is kept as it is too), so a default name never reads
/// "Layer Layer 1" (UX review of the customer feedback 2026-10-14).
fn layer_words(layer: &str, noun: &str) -> String {
    if layer.starts_with("Layer ") || layer.ends_with(" Layer") || layer == "Layer" {
        layer.to_owned()
    } else {
        format!("{noun} {layer}")
    }
}

/// "Layer Cut is locked. Unlock it or choose another layer." or the hidden counterpart: drawing
/// into the active layer is refused (criterion 23).
#[must_use]
pub fn draw_refused_notice(layer: &str, locked: bool) -> String {
    let layer = layer_words(layer, "Layer");
    if locked {
        format!("{layer} is locked. Unlock it or choose another layer.")
    } else {
        format!("{layer} is hidden. Show it or choose another layer.")
    }
}

use crate::layer_rows::{Cause, LayerRow, percent};

/// "Hidden by layer Cut": the state an ancestor gives a row.
fn by(cause: &Cause) -> String {
    if cause.kind == "layer" {
        format!("by {}", layer_words(&cause.name, "layer"))
    } else {
        format!("by {} {}", cause.kind, cause.name)
    }
}

/// The state words of a row, each when it applies: "hidden" or "hidden by layer Cut",
/// "locked" or "locked by group Tabs", and "opacity 60 percent".
fn state_words(row: &LayerRow) -> Vec<String> {
    let mut words = Vec::new();
    if !row.own.visible {
        words.push("hidden".to_owned());
    } else if let Some(cause) = &row.hidden_by {
        words.push(format!("hidden {}", by(cause)));
    }
    if row.own.locked {
        words.push("locked".to_owned());
    } else if let Some(cause) = &row.locked_by {
        words.push(format!("locked {}", by(cause)));
    }
    let opacity = percent(row.opacity);
    if opacity < 100 {
        words.push(format!("opacity {opacity} percent"));
    }
    words
}

/// The inherited states of a row as the eye and the lock describe them: ("Hidden by layer Cut",
/// "Locked by group Tabs"), each empty when no ancestor gives it (criterion 22).
#[must_use]
pub fn inherited_lines(row: &LayerRow) -> (String, String) {
    let line = |cause: &Option<Cause>, word: &str| {
        cause
            .as_ref()
            .map_or_else(String::new, |cause| format!("{word} {}", by(cause)))
    };
    (
        line(&row.hidden_by, "Hidden"),
        line(&row.locked_by, "Locked"),
    )
}

/// A row's accessible name: "Rectangle 3f9a1c, rectangle, hidden, locked, opacity 60 percent"
/// (criterion 41); a collapsed container with a selected descendant adds "contains the selection".
#[must_use]
pub fn row_accessible_name(row: &LayerRow) -> String {
    let kind = match row.kind {
        crate::layer_rows::RowKind::Clone => "linked clone".to_owned(),
        other => other.word().to_lowercase(),
    };
    let mut parts = vec![row.name.clone(), kind];
    if !row.overrides_line.is_empty() {
        // `specs/0057-clone-overrides` criterion 28: the name appends the overrides.
        parts.push(row.overrides_line.to_lowercase());
    }
    parts.extend(state_words(row));
    if row.container.is_some_and(|c| c.contains_selection) {
        parts.push("contains the selection".to_owned());
    }
    parts.join(", ")
}

/// The second, muted line of a row's tooltip: "Hidden", "Locked", "Hidden by layer Cut", or
/// empty when the row is neither hidden nor locked.
#[must_use]
pub fn row_state_line(row: &LayerRow) -> String {
    let mut words: Vec<String> = state_words(row)
        .into_iter()
        .filter(|word| !word.starts_with("opacity"))
        .collect();
    if let Some(first) = words.first_mut() {
        let mut chars = first.chars();
        if let Some(c) = chars.next() {
            *first = c.to_uppercase().chain(chars).collect();
        }
    }
    words.join(", ")
}

/// Why a press on a hidden or locked row selects nothing (criterion 14): "Hidden" or "Locked";
/// `None` for a row that can be selected.
#[must_use]
pub fn row_refusal(row: &LayerRow) -> Option<&'static str> {
    if !row.effective.visible {
        Some("Hidden")
    } else if row.effective.locked {
        Some("Locked")
    } else {
        None
    }
}

/// "1 object", "40 objects": the subject line of the Layers tab (criterion 4).
#[must_use]
pub fn objects_count_text(count: usize) -> String {
    if count == 1 {
        "1 object".to_owned()
    } else {
        format!("{count} objects")
    }
}

/// "Deleted layer Cut and 12 objects." (criterion 36).
#[must_use]
pub fn deleted_layer_notice(layer: &str, objects: usize) -> String {
    let layer = layer_words(layer, "layer");
    match objects {
        0 => format!("Deleted {layer}."),
        1 => format!("Deleted {layer} and 1 object."),
        n => format!("Deleted {layer} and {n} objects."),
    }
}

/// "Layer Cut is locked. Unlock it first." (criterion 36).
#[must_use]
pub fn delete_locked_layer_notice(layer: &str) -> String {
    format!(
        "{} is locked. Unlock it first.",
        layer_words(layer, "Layer")
    )
}

/// "Could not duplicate. Nothing was changed.": Duplicate of the Layers toolbar was refused
/// (customer feedback 2026-10-14, criterion 57).
pub const DUPLICATE_FAILED_NOTICE: &str = "Could not duplicate. Nothing was changed.";

/// "Rectangle 3f9a1c is locked. Unlock it first." (criterion 37, amended 2026-10-12).
#[must_use]
pub fn locked_object_notice(name: &str) -> String {
    format!("{name} is locked. Unlock it first.")
}

/// Why Select contents found nothing to select (UX notes): the layer is empty, hidden or locked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NothingToSelect {
    /// The layer holds nothing.
    Empty,
    /// The layer is hidden.
    Hidden,
    /// The layer is locked.
    Locked,
}

/// "Layer Cut is empty." / "Layer Cut is hidden." / "Layer Cut is locked."
#[must_use]
pub fn nothing_to_select_notice(layer: &str, why: NothingToSelect) -> String {
    let word = match why {
        NothingToSelect::Empty => "empty",
        NothingToSelect::Hidden => "hidden",
        NothingToSelect::Locked => "locked",
    };
    format!("{} is {word}.", layer_words(layer, "Layer"))
}

/// "Moved Rectangle 3f9a1c to position 2 of 5" (criterion 32): what a screen reader hears after a
/// move by key. Positions count from the top of the list.
#[must_use]
pub fn moved_announcement(name: &str, position: usize, of: usize) -> String {
    format!("Moved {name} to position {position} of {of}")
}

/// The caption under the Opacity field (criterion 26): what it acts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpacityTarget {
    /// Nothing is targeted.
    Nothing,
    /// The target is locked.
    Locked,
    /// Selected objects, by count.
    Objects(usize),
    /// A layer, by name.
    Layer(String),
    /// Only linked clones, which show their original's opacity (`0050` criterion 26).
    OnlyClones,
}

/// "Locked", "3 objects", "1 object", "Cut" (after the `Layers` glyph); nothing when nothing is targeted (the hint
/// "Select an object or a layer." was removed, customer feedback 2026-10-14).
#[must_use]
pub fn opacity_caption(target: &OpacityTarget) -> String {
    match target {
        OpacityTarget::Nothing => String::new(),
        OpacityTarget::Locked => "Locked".to_owned(),
        OpacityTarget::Objects(count) => objects_count_text(*count),
        // The caption's `Layers` glyph says layer: the name alone.
        OpacityTarget::Layer(name) => name.clone(),
        OpacityTarget::OnlyClones => "Linked clones show the opacity of their original.".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use curvyo_document_core::{Document, MovePlan, MoveTarget, Opacity, Point, RectBounds};
    use std::collections::HashSet;

    use crate::layer_rows::{Expansion, LayerRows, RowFacts};

    fn row_of(document: &Document, id: curvyo_document_core::NodeId) -> LayerRow {
        let selected = HashSet::new();
        LayerRows::build(
            &document.read_nodes(),
            &Expansion::default(),
            RowFacts {
                selected: &selected,
                active_layer: None,
                entered: None,
            },
        )
        .row(id)
        .unwrap()
        .clone()
    }

    /// UX review should-fix 14: a clone row says "linked clone".
    #[test]
    fn a_clone_row_is_named_a_linked_clone() {
        use curvyo_document_core::{AnchorId, NewAnchor};
        let document = Document::new(1);
        let corner = |k: u64, x: f64| NewAnchor::corner(AnchorId::new(1, k), Point::new(x, x));
        let a = document.create_path(&[corner(0, 0.0), corner(1, 5.0), corner(2, 9.0)], true);
        let c = document.linked_clone(&[a], &[]).unwrap()[0];
        assert!(row_accessible_name(&row_of(&document, c)).ends_with(", linked clone"));
    }

    #[test]
    fn a_row_names_its_kind_and_every_state_that_applies() {
        let document = Document::new(1);
        let a = document.create_rect(RectBounds::from_corners(
            Point::new(0.0, 0.0),
            Point::new(10.0, 10.0),
        ));
        let plain = row_of(&document, a);
        assert_eq!(
            row_accessible_name(&plain),
            format!("Rectangle {}, rectangle", a.short_id())
        );
        assert_eq!(row_state_line(&plain), "");
        assert_eq!(row_refusal(&plain), None);
        document.set_node_visible(&[a], false).unwrap();
        document
            .set_node_opacity(&[a], Opacity::new(0.6).unwrap())
            .unwrap();
        document.set_node_locked(&[a], true).unwrap();
        let row = row_of(&document, a);
        assert_eq!(
            row_accessible_name(&row),
            format!(
                "Rectangle {}, rectangle, hidden, locked, opacity 60 percent",
                a.short_id()
            )
        );
        assert_eq!(row_state_line(&row), "Hidden, locked");
        assert_eq!(row_refusal(&row), Some("Hidden"));
    }

    #[test]
    fn an_inherited_state_is_named_by_its_layer() {
        let document = Document::new(1);
        let a = document.create_rect(RectBounds::from_corners(
            Point::new(0.0, 0.0),
            Point::new(10.0, 10.0),
        ));
        let layer = document.new_layer(None);
        document.rename_node(layer, "Cut").unwrap();
        document
            .move_nodes(&MovePlan {
                nodes: vec![a],
                target: MoveTarget::FrontOf(Some(layer)),
            })
            .unwrap();
        document.set_node_visible(&[layer], false).unwrap();
        let row = row_of(&document, a);
        assert_eq!(row_state_line(&row), "Hidden by layer Cut");
        assert_eq!(
            inherited_lines(&row),
            ("Hidden by layer Cut".to_owned(), String::new())
        );
        assert!(row_accessible_name(&row).ends_with("hidden by layer Cut"));
    }

    #[test]
    fn the_notices_and_captions() {
        assert_eq!(objects_count_text(1), "1 object");
        assert_eq!(objects_count_text(40), "40 objects");
        assert_eq!(
            deleted_layer_notice("Cut", 12),
            "Deleted layer Cut and 12 objects."
        );
        assert_eq!(deleted_layer_notice("Cut", 0), "Deleted layer Cut.");
        assert_eq!(
            delete_locked_layer_notice("Cut"),
            "Layer Cut is locked. Unlock it first."
        );
        assert_eq!(
            nothing_to_select_notice("Cut", NothingToSelect::Hidden),
            "Layer Cut is hidden."
        );
        assert_eq!(
            moved_announcement("Rectangle 3f9a1c", 2, 5),
            "Moved Rectangle 3f9a1c to position 2 of 5"
        );
        assert_eq!(opacity_caption(&OpacityTarget::Nothing), "");
        assert_eq!(opacity_caption(&OpacityTarget::Objects(3)), "3 objects");
    }

    #[test]
    fn a_default_layer_name_is_not_preceded_by_the_word_layer_again() {
        // UX review of the customer feedback 2026-10-14, should-fix 5.
        assert_eq!(deleted_layer_notice("Layer 1", 0), "Deleted Layer 1.");
        assert_eq!(
            delete_locked_layer_notice("Initial Layer"),
            "Initial Layer is locked. Unlock it first."
        );
        assert_eq!(
            draw_refused_notice("Layer 2", true),
            "Layer 2 is locked. Unlock it or choose another layer."
        );
        assert_eq!(
            nothing_to_select_notice("Layer 1", NothingToSelect::Empty),
            "Layer 1 is empty."
        );
        assert_eq!(
            opacity_caption(&OpacityTarget::Layer("Layer 1".into())),
            "Layer 1",
            "the caption's Layers glyph says layer"
        );
        assert_eq!(opacity_caption(&OpacityTarget::Layer("Cut".into())), "Cut");
    }

    #[test]
    fn the_draw_refusals_name_the_layer_and_the_way_out() {
        assert_eq!(
            draw_refused_notice("Cut", true),
            "Layer Cut is locked. Unlock it or choose another layer."
        );
        assert_eq!(
            draw_refused_notice("Cut", false),
            "Layer Cut is hidden. Show it or choose another layer."
        );
    }
}
