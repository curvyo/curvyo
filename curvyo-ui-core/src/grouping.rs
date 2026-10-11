//! The Group and Ungroup commands as the maker meets them: when the rail buttons and the chords
//! act, and the words of their notices (`specs/0023-groups` criteria 2, 3, 5, 6 and 28, the UX notes
//! "Group and Ungroup" and "Selecting, hovering, entering, leaving"). The writes are
//! `Document::group_objects` and `Document::ungroup`; the session calls them.

use crate::object_selection::ObjectSelection;
use crate::units::Units;

/// What the Grouping card of the rail shows (criteria 2 and 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupingAvailability {
    /// Two or more objects of the context are selected: Group acts.
    pub group: bool,
    /// At least one group is selected: Ungroup acts.
    pub ungroup: bool,
}

/// Group acts on two or more selected objects of the context, Ungroup on a selection that holds a
/// group. A group counts as one object however many leaves it has.
#[must_use]
pub fn grouping_availability(units: &Units, selection: &ObjectSelection) -> GroupingAvailability {
    let selected = units.selected(selection.ids());
    GroupingAvailability {
        group: selected.len() >= 2,
        ungroup: selected.iter().any(|unit| unit.is_group),
    }
}

/// The refusal of Group that would nest groups too deep (criterion 5).
pub const TOO_DEEP_NOTICE: &str = "Groups can be nested 32 levels deep. Nothing was changed.";

/// "Grouped 3 objects." (criterion 6).
#[must_use]
pub fn grouped_notice(objects: usize) -> String {
    format!(
        "Grouped {objects} {}.",
        plural(objects, "object", "objects")
    )
}

/// "Ungrouped 2 groups into 7 objects." (criterion 6).
#[must_use]
pub fn ungrouped_notice(groups: usize, objects: usize) -> String {
    format!(
        "Ungrouped {groups} {} into {objects} {}.",
        plural(groups, "group", "groups"),
        plural(objects, "object", "objects"),
    )
}

/// "Union does not work on groups. Ungroup first. Nothing was changed." (criterion 28, the refusal
/// style of `0016` criterion 15).
#[must_use]
pub fn groups_refusal_notice(operation: &str) -> String {
    format!("{operation} does not work on groups. Ungroup first. Nothing was changed.")
}

/// What a screen reader hears when the context changes (UX notes, "Announcements"): "Inside group,
/// 1 level. Escape leaves." on entering, "Inside group, 2 levels." when a deeper level is left,
/// "Back at the document." at the root.
#[must_use]
pub fn context_announcement(depth: usize, entered: bool) -> String {
    match (depth, entered) {
        (0, _) => "Back at the document.".to_owned(),
        (levels, true) => format!(
            "Inside group, {levels} {}. Escape leaves.",
            plural(levels, "level", "levels")
        ),
        (levels, false) => format!(
            "Inside group, {levels} {}.",
            plural(levels, "level", "levels")
        ),
    }
}

/// How a selected group reads to a screen reader: "Group of 5 objects selected, 46.2 by 18.7 mm";
/// several objects of the context: "3 objects selected, ...".
#[must_use]
pub fn selection_announcement(
    selected: &[crate::units::SelectedUnit],
    width_mm: f64,
    height_mm: f64,
) -> String {
    match selected {
        [unit] if unit.is_group => format!(
            "Group of {} {} selected, {width_mm:.1} by {height_mm:.1} mm",
            unit.leaves.len(),
            plural(unit.leaves.len(), "object", "objects"),
        ),
        _ => format!(
            "{} objects selected, {width_mm:.1} by {height_mm:.1} mm",
            selected.len()
        ),
    }
}

const fn plural(count: usize, one: &'static str, many: &'static str) -> &'static str {
    if count == 1 { one } else { many }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{Document, Length, Point, RectBounds};

    use super::*;

    fn rect(document: &Document, x: f64) -> curvyo_document_core::NodeId {
        document.create_rect(RectBounds {
            origin: Point::new(x, 0.0),
            width: Length::from_mm(10.0),
            height: Length::from_mm(10.0),
        })
    }

    #[test]
    fn group_needs_two_objects_and_ungroup_needs_a_group() {
        let document = Document::new(1);
        let (a, b, c) = (
            rect(&document, 0.0),
            rect(&document, 20.0),
            rect(&document, 40.0),
        );
        let group = document.group_objects(&[b, c]).unwrap();
        let _ = group;
        let units = Units::build(&document, None);
        let mut selection = ObjectSelection::new();
        let none = grouping_availability(&units, &selection);
        assert!(!none.group && !none.ungroup);
        selection.set(&[a]);
        assert!(
            !grouping_availability(&units, &selection).group,
            "one object"
        );
        selection.set(&[b]);
        let one_group = grouping_availability(&units, &selection);
        assert!(!one_group.group, "a group alone is one object");
        assert!(one_group.ungroup);
        selection.set(&[a, b, c]);
        let both = grouping_availability(&units, &selection);
        assert!(both.group && both.ungroup);
    }

    #[test]
    fn the_notices_are_the_sentences_of_the_spec() {
        assert_eq!(grouped_notice(3), "Grouped 3 objects.");
        assert_eq!(ungrouped_notice(2, 7), "Ungrouped 2 groups into 7 objects.");
        assert_eq!(ungrouped_notice(1, 1), "Ungrouped 1 group into 1 object.");
        assert_eq!(
            groups_refusal_notice("Union"),
            "Union does not work on groups. Ungroup first. Nothing was changed."
        );
        assert_eq!(
            context_announcement(1, true),
            "Inside group, 1 level. Escape leaves."
        );
        assert_eq!(context_announcement(0, false), "Back at the document.");
        assert_eq!(context_announcement(2, false), "Inside group, 2 levels.");
    }
}
