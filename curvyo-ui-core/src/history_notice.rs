//! The words of undo and redo: the touched-objects summary of a row and the one-line notice a key
//! press leaves (`specs/0020-undo-redo` criteria 1, 2, 4, 19, 24 and the UX notes, "Notices").

use curvyo_document_core::{Direction, ObjectShape, RestoreReport, Role};

/// The hint of an undo or redo while the Pen holds an unfinished path (`specs/0020-undo-redo`
/// criterion 6e).
pub const FINISH_PEN_FIRST: &str = "Finish the path (Enter) or cancel it (Esc) first.";

/// The shapes a summary counts, from the objects a step touched with what it did to each: the
/// objects as the maker saw them change. A step that deleted objects and created others (a Boolean,
/// Combine, Break apart) counts the deleted ones, the two squares that were selected and not the
/// one result; a document command counts none (`is_document_level_label`).
#[must_use]
pub fn summary_shapes(label: &str, touched: &[(ObjectShape, Role)]) -> Vec<ObjectShape> {
    if curvyo_document_core::is_document_level_label(label) {
        return Vec::new();
    }
    let replaced = touched.iter().any(|(_, role)| *role == Role::Deleted)
        && touched.iter().any(|(_, role)| *role == Role::Created);
    touched
        .iter()
        .filter(|(_, role)| !(replaced && *role == Role::Created))
        .map(|(shape, _)| *shape)
        .collect()
}

/// "1 rectangle", "3 paths", "5 objects": the summary of the objects a step touched. Objects of
/// one shape are counted by that shape, a mix as "objects". `None` for a step that touched no
/// object.
#[must_use]
pub fn objects_summary(shapes: &[ObjectShape]) -> Option<String> {
    let first = *shapes.first()?;
    let count = shapes.len();
    if shapes.iter().all(|shape| *shape == first) {
        let (one, many) = match first {
            ObjectShape::Path => ("path", "paths"),
            ObjectShape::Rectangle => ("rectangle", "rectangles"),
            ObjectShape::Ellipse => ("ellipse", "ellipses"),
            ObjectShape::Polygon => ("polygon", "polygons"),
            ObjectShape::Star => ("star", "stars"),
        };
        Some(format!("{count} {}", if count == 1 { one } else { many }))
    } else {
        Some(format!("{count} objects"))
    }
}

/// The notice for a press with nothing on the stack.
#[must_use]
pub const fn nothing_notice(direction: Direction) -> &'static str {
    match direction {
        Direction::Undo => "Nothing to undo.",
        Direction::Redo => "Nothing to redo.",
    }
}

/// The notice for a press that ran: "Undid: Move (3 paths)." and its variants when parts were
/// skipped. `off_screen` replaces the full stop of the plain notice by ", off screen.".
#[must_use]
pub fn restore_notice(
    name: &str,
    report: &RestoreReport,
    summary: Option<&str>,
    off_screen: bool,
) -> String {
    let (done, verb) = match report.direction {
        Direction::Undo => ("Undid", "undo"),
        Direction::Redo => ("Redid", "redo"),
    };
    if !report.wrote {
        return if report.created_object_kept() && report.direction == Direction::Undo {
            "Not undone: someone else changed this object after you drew it.".to_owned()
        } else {
            format!(
                "Could not {verb} {name}: everything it changed was changed or deleted by someone else since."
            )
        };
    }
    let skipped = report.skipped();
    if skipped > 0 {
        return format!(
            "{done}: {name}. {skipped} of {} objects kept later changes by someone else.",
            report.total()
        );
    }
    let objects = summary.map(|text| format!(" ({text})")).unwrap_or_default();
    let tail = if off_screen { ", off screen." } else { "." };
    format!("{done}: {name}{objects}{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{
        Document, NodeId, ObjectReport, ObjectResult, Point, RectBounds, Role, SkipReason,
    };

    fn node_ids(count: usize) -> Vec<NodeId> {
        let document = Document::new(1);
        (0..count)
            .map(|n| {
                let x = f64::from(u32::try_from(n).unwrap()) * 20.0;
                document.create_rect(RectBounds::from_corners(
                    Point::new(x, 0.0),
                    Point::new(x + 10.0, 10.0),
                ))
            })
            .collect()
    }

    fn report(direction: Direction, results: &[ObjectResult], wrote: bool) -> RestoreReport {
        let ids = node_ids(results.len());
        let objects = results
            .iter()
            .zip(ids)
            .map(|(result, id)| ObjectReport {
                id,
                shape: ObjectShape::Path,
                role: Role::Changed,
                result: *result,
            })
            .collect();
        RestoreReport {
            direction,
            objects,
            root: None,
            wrote,
        }
    }

    #[test]
    fn a_boolean_counts_its_operands_and_a_resize_of_the_document_counts_none() {
        use ObjectShape::{Path, Rectangle};
        let union = [
            (Rectangle, Role::Deleted),
            (Rectangle, Role::Deleted),
            (Path, Role::Created),
        ];
        assert_eq!(
            summary_shapes("boolean_union", &union),
            [Rectangle, Rectangle]
        );
        let draw = [(Rectangle, Role::Created)];
        assert_eq!(summary_shapes("create_rect", &draw), [Rectangle]);
        let moved = [(Rectangle, Role::Changed), (Path, Role::Changed)];
        assert_eq!(summary_shapes("resize_document", &moved), []);
        assert_eq!(summary_shapes("translate_objects", &moved).len(), 2);
    }

    #[test]
    fn summaries_count_by_shape_and_fall_back_to_objects() {
        use ObjectShape::{Path, Rectangle};
        assert_eq!(objects_summary(&[]), None);
        assert_eq!(
            objects_summary(&[Rectangle]).as_deref(),
            Some("1 rectangle")
        );
        assert_eq!(objects_summary(&[Path; 3]).as_deref(), Some("3 paths"));
        assert_eq!(
            objects_summary(&[Path, Rectangle]).as_deref(),
            Some("2 objects")
        );
    }

    #[test]
    fn the_notices_read_as_the_specification_words_them() {
        let ok = report(Direction::Undo, &[ObjectResult::Restored; 3], true);
        assert_eq!(
            restore_notice("Move", &ok, Some("3 paths"), false),
            "Undid: Move (3 paths)."
        );
        assert_eq!(
            restore_notice("Delete", &ok, Some("2 rectangles"), true),
            "Undid: Delete (2 rectangles), off screen."
        );
        let redo = report(Direction::Redo, &[ObjectResult::Restored], true);
        assert_eq!(restore_notice("Move", &redo, None, false), "Redid: Move.");
        let partly = report(
            Direction::Undo,
            &[
                ObjectResult::Restored,
                ObjectResult::Restored,
                ObjectResult::Skipped(SkipReason::ChangedLater),
            ],
            true,
        );
        assert_eq!(
            restore_notice("Move", &partly, Some("3 paths"), false),
            "Undid: Move. 1 of 3 objects kept later changes by someone else."
        );
        let none = report(
            Direction::Undo,
            &[ObjectResult::Skipped(SkipReason::DeletedLater)],
            false,
        );
        assert_eq!(
            restore_notice("Move", &none, None, false),
            "Could not undo Move: everything it changed was changed or deleted by someone else since."
        );
        assert_eq!(nothing_notice(Direction::Undo), "Nothing to undo.");
        assert_eq!(nothing_notice(Direction::Redo), "Nothing to redo.");
    }

    #[test]
    fn a_created_object_a_peer_changed_keeps_its_own_notice() {
        let mut kept = report(
            Direction::Undo,
            &[ObjectResult::Skipped(SkipReason::ChangedLater)],
            false,
        );
        kept.objects[0].role = Role::Created;
        assert_eq!(
            restore_notice("Draw rectangle", &kept, Some("1 rectangle"), false),
            "Not undone: someone else changed this object after you drew it."
        );
    }
}
